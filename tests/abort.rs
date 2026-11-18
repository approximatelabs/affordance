//! `AbortController`: a model call stopped by its signal keeps what it generated; a machine call or
//! `fetch` rejects with `AbortError`; the journal decides a race; replay and resume agree.
use crate::tokenizer;
use affordance::{
    View,
    effects::Outcome,
    runtime::{Agent, AgentConfig, Command, Input},
};
use serde_json::{Value as J, json};
use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicUsize, Ordering},
};

const APP: &str = r#"
import {GLM53App} from '@ROOT@/views/template_apps/glm53.tsx';
import {TurnStrategy} from '@ROOT@/views/strategies/turn_strategy.tsx';
function Ask(){
  return <GLM53App><TurnStrategy strategy='unrolled' log={[{kind:'input',content:'Say something long.'}]}/></GLM53App>;
}
export default function Main(){
  const [out,setOut]=useState(null,'out');
  const [turn,setTurn]=useRef(null,'turn');
  useEvent('ask',async()=>{
    const c=new AbortController();
    setTurn(c);
    const o=await model(<Ask/>,{signal:c.signal});
    setOut({text:o.text,aborted:!!o.aborted,cut_off:o.cut_off,signal:c.signal.aborted});
  });
  useEvent('page',async()=>{
    const c=new AbortController();
    setTurn(c);
    const o=await model({signal:c.signal});
    setOut({text:o.text,aborted:!!o.aborted,calls:o.calls.length});
  });
  useEvent('call',async()=>{
    const c=new AbortController();
    setTurn(c);
    try{await harness.machine.get('http').call({path:'/slow'},{signal:c.signal});setOut('answered');}
    catch(e){setOut([e.name,c.signal.aborted]);}
  });
  useEvent('stop',()=>{turn().abort();});
  useEvent('early',async()=>{
    const c=new AbortController();
    c.abort('enough');
    const seen=[c.signal.aborted,c.signal.reason];
    try{c.signal.throwIfAborted();}catch(e){seen.push(e);}
    try{await model(<Ask/>,{signal:c.signal});}catch(e){seen.push(e.name);}
    try{await fetch('https://example.com/',{signal:AbortSignal.abort()});}catch(e){seen.push(e.name);}
    const fresh=new AbortController();
    seen.push(fresh.signal.aborted,fresh.signal.reason===undefined);
    fresh.abort();
    seen.push(fresh.signal.reason.name);
    setOut(seen);
  });
  return <GLM53App><TurnStrategy strategy='unrolled' log={[]}/></GLM53App>;
}"#;

struct Rig {
    config: AgentConfig,
    agent: Agent,
    view: View,
    enc: affordance::emit::Encoder,
    clock: i64,
    journal: Vec<Input>,
}

impl Rig {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "aff-abort-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("app.tsx");
        std::fs::write(&path, APP.replace("@ROOT@", env!("CARGO_MANIFEST_DIR"))).unwrap();
        let enc = tokenizer::encoder();
        let view = View::load(&path, "Main", &enc).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        let config = AgentConfig {
            context: None,
            retry: Default::default(),
            template: None,
            id: "a".into(),
            props: json!({}),
            env: BTreeMap::new(),
            max_new: 100,
            max_turns: None,
            max_prompt: None,
            start_ms: 0,
            seed: 7,
            machines: vec!["http".into()],
            model: Some("default".into()),
        };
        let mut agent = Agent::new(config.clone()).unwrap();
        // The page's own first turn is not this test's: the app has no loop.
        agent.start(&view, &enc).unwrap();
        Self {
            config,
            agent,
            view,
            enc,
            clock: 0,
            journal: vec![],
        }
    }
    fn input(&mut self, input: Input) -> Vec<Command> {
        self.clock = input.clock_ms();
        self.journal.push(input.clone());
        let out = self.agent.handle(&self.view, &self.enc, input).unwrap();
        for t in self.agent.take_trace() {
            assert_ne!(t["t"], "handler_error", "{t}");
        }
        out
    }
    fn event(&mut self, name: &str) -> Vec<Command> {
        self.clock += 1;
        self.input(Input::Event {
            name: name.into(),
            payload: json!({}),
            op: None,
            clock_ms: self.clock,
        })
    }
    fn reply(&mut self, task: u64, text: &str, stop: bool, aborted: bool) -> Vec<Command> {
        let mut ids = self.enc.tok.encode(text, false).unwrap().get_ids().to_vec();
        if stop {
            ids.push(self.enc.token_id("<|user|>").unwrap());
        }
        self.clock += 1;
        self.input(Input::Reply {
            server: None,
            task,
            ids,
            cut_off: false,
            clock_ms: self.clock,
            prompt: None,
            aborted,
        })
    }
    fn out(&self) -> J {
        self.agent.state()["__ui"]["@out"].clone()
    }
    fn restored(&self) -> (Agent, Vec<Command>) {
        Agent::restore(self.config.clone(), self.journal.clone(), &self.view, &self.enc).unwrap()
    }
}

fn generate(commands: &[Command]) -> (u64, bool) {
    commands
        .iter()
        .find_map(|c| match c {
            Command::Generate { task, opts, .. } => Some((*task, opts.abortable)),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no model call in {commands:?}"))
}

#[test]
fn an_aborted_model_call_resolves_with_what_it_generated() {
    let mut rig = Rig::new();
    let (task, abortable) = generate(&rig.event("ask"));
    assert!(abortable, "a call with a signal streams, so it can be stopped");
    assert_eq!(rig.event("stop"), vec![Command::Abort { task }]);
    assert!(rig.reply(task, "</think>Once upon a", false, true).is_empty());
    assert_eq!(
        rig.out(),
        json!({"text": "Once upon a", "aborted": true, "cut_off": true, "signal": true})
    );
    // Replay reads the same journal to the same state.
    let (restored, outstanding) = rig.restored();
    assert_eq!(restored.state(), rig.agent.state());
    assert!(outstanding.is_empty());
}

#[test]
fn an_aborted_page_call_keeps_its_text_and_makes_no_call() {
    let mut rig = Rig::new();
    let (task, _) = generate(&rig.event("page"));
    assert_eq!(rig.event("stop"), vec![Command::Abort { task }]);
    rig.reply(
        task,
        "</think>Let me check.<tool_call>shell<arg_key>cmd</arg_key>",
        false,
        true,
    );
    assert_eq!(rig.out()["aborted"], true);
    assert_eq!(rig.out()["calls"], 0);
    assert!(rig.out()["text"].as_str().unwrap().starts_with("Let me check."));
}

#[test]
fn a_reply_that_wins_the_race_is_whole() {
    let mut rig = Rig::new();
    let (task, _) = generate(&rig.event("ask"));
    rig.event("stop");
    // The whole reply was on its way when the abort reached the model: the journal says so.
    rig.reply(task, "</think>Done.", true, false);
    assert_eq!(
        rig.out(),
        json!({"text": "Done.", "aborted": false, "cut_off": false, "signal": true})
    );
}

#[test]
fn an_aborted_call_is_stopped_again_on_resume_not_reissued_whole() {
    let mut rig = Rig::new();
    let (task, _) = generate(&rig.event("ask"));
    rig.event("stop");
    // A crash before the reply: the restored agent asks for the call and its stop together.
    let (mut restored, outstanding) = rig.restored();
    let at = |c: &Command| outstanding.iter().position(|o| o == c);
    let issued = outstanding
        .iter()
        .position(|c| matches!(c, Command::Generate { task: t, .. } if *t == task))
        .unwrap();
    assert!(at(&Command::Abort { task }).unwrap() > issued, "{outstanding:?}");
    // The host answers it at once with nothing generated.
    let reply = Input::Reply {
        server: None,
        task,
        ids: vec![],
        cut_off: false,
        clock_ms: rig.clock + 1,
        prompt: None,
        aborted: true,
    };
    restored.handle(&rig.view, &rig.enc, reply).unwrap();
    assert_eq!(restored.state()["__ui"]["@out"]["aborted"], true);
    assert_eq!(restored.state()["__ui"]["@out"]["text"], "");
}

#[test]
fn an_aborted_model_call_that_fails_ends_as_stopped_not_retried() {
    let mut rig = Rig::new();
    let (task, _) = generate(&rig.event("ask"));
    rig.event("stop");
    rig.clock += 1;
    let commands = rig.input(Input::ModelError {
        task,
        error: "HTTP 503".into(),
        transient: true,
        kind: None,
        retry_after_ms: None,
        clock_ms: rig.clock,
    });
    assert!(
        !commands.iter().any(|c| matches!(c, Command::Timer { .. })),
        "{commands:?}"
    );
    assert_eq!(rig.out()["aborted"], true);
}

#[test]
fn an_aborted_machine_call_is_cancelled_and_rejects() {
    let mut rig = Rig::new();
    let commands = rig.event("call");
    let task = commands
        .iter()
        .find_map(|c| match c {
            Command::Call { task, .. } => Some(*task),
            _ => None,
        })
        .unwrap();
    assert_eq!(rig.event("stop"), vec![Command::Cancel { task }]);
    assert_eq!(rig.out(), json!(["AbortError", true]));
    // Its late result changes nothing.
    rig.clock += 1;
    rig.input(Input::Result {
        task,
        outcome: Outcome::Ok(json!({"status": 200})),
        clock_ms: rig.clock,
    });
    assert_eq!(rig.out(), json!(["AbortError", true]));
    assert_eq!(rig.restored().0.state(), rig.agent.state());
}

#[test]
fn an_already_aborted_signal_rejects_at_once_and_issues_nothing() {
    let mut rig = Rig::new();
    let commands = rig.event("early");
    assert!(
        !commands
            .iter()
            .any(|c| matches!(c, Command::Generate { .. } | Command::Call { .. })),
        "{commands:?}"
    );
    assert_eq!(
        rig.out(),
        json!([
            true,
            "enough",
            "enough",
            "AbortError",
            "AbortError",
            false,
            true,
            "AbortError"
        ])
    );
    assert_eq!(rig.restored().0.state(), rig.agent.state());
}

/// A model that streams one token every 10 ms, for up to ten seconds, and stops when told. After
/// five tokens it posts the user's Escape (the event `stop`) to the agent.
struct Streaming {
    opener: Vec<u32>,
    word: u32,
    inbox: std::sync::Mutex<Option<affordance::runtime::Inbox>>,
}
impl affordance::runtime::Model for Streaming {
    fn generate(
        &self,
        _: &affordance::runtime::GenRequest,
    ) -> Result<affordance::runtime::GenReply, affordance::runtime::model::ModelError> {
        panic!("a call with a signal streams")
    }
    fn generate_stream(
        &self,
        req: &affordance::runtime::GenRequest,
        on_tokens: &mut dyn FnMut(affordance::runtime::Delta<'_>) -> bool,
    ) -> Result<affordance::runtime::GenReply, affordance::runtime::model::ModelError> {
        let mut ids = self.opener.clone();
        on_tokens(affordance::runtime::Delta::Ids(&self.opener));
        for n in 0..1000 {
            std::thread::sleep(std::time::Duration::from_millis(10));
            ids.push(self.word);
            if !on_tokens(affordance::runtime::Delta::Ids(&[self.word])) {
                break;
            }
            if n == 4 {
                let inbox = self.inbox.lock().unwrap();
                inbox.as_ref().unwrap().post(&req.agent, "stop", json!({})).unwrap();
            }
        }
        Ok(affordance::runtime::GenReply {
            ids,
            ..Default::default()
        })
    }
}

#[test]
fn escape_stops_a_streaming_reply_and_the_journal_keeps_it() {
    use affordance::runtime::{AgentSpec, Runtime, RuntimeConfig};
    use std::sync::Arc;
    let dir = std::env::temp_dir().join(format!("aff-abort-run-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let app = dir.join("app.tsx");
    std::fs::write(&app, APP.replace("@ROOT@", env!("CARGO_MANIFEST_DIR"))).unwrap();
    let enc = Arc::new(tokenizer::encoder());
    let word = enc.tok.encode("a", false).unwrap().get_ids()[0];
    let model = Arc::new(Streaming {
        opener: enc.tok.encode("</think>", false).unwrap().get_ids().to_vec(),
        word,
        inbox: Default::default(),
    });
    let mut cfg = RuntimeConfig::new(app.clone());
    cfg.dir = Some(dir.join("run"));
    cfg.library = Some(std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/views")));
    cfg.max_wall = Some(std::time::Duration::from_secs(30));
    let mut rt = Runtime::new(cfg, enc, model.clone());
    let inbox = rt.inbox();
    *model.inbox.lock().unwrap() = Some(inbox.clone());
    inbox.post("a", "ask", json!({})).unwrap();
    let mut spec = AgentSpec::new("a", json!({}));
    spec.machines = Some(vec![]);
    rt.add_agent(spec).unwrap();
    let report = rt.run().unwrap();
    let a = &report.agents[0];
    assert_eq!(a.error, None);
    let out = &a.state["__ui"]["@out"];
    assert_eq!(out["aborted"], true, "{out}");
    assert_eq!(out["signal"], true);
    let text = out["text"].as_str().unwrap();
    let words = text.matches('a').count();
    assert!((5..50).contains(&words), "stopped soon after the Escape: {text:?}");
    let journal = std::fs::read_to_string(dir.join("run/a/journal.jsonl")).unwrap();
    assert!(journal.contains(r#""aborted":true"#), "{journal}");
    let _ = std::fs::remove_dir_all(&dir);
}
