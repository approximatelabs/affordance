//! The instance as a page: the runtime never calls the model
//! on its own. An app fires its own view (`harness.fire({log: "main"})`) and runs the reply's
//! calls (`reply.act()`) from effects; listeners run when their event arrives, even while a tool
//! waits; pushes made while a fire is open are written after its records; a reply not acted on
//! closes with `not run` results; effects clean up when their deps change.
use affordance::View;
use affordance::effects::Outcome;
use affordance::emit::Encoder;
use affordance::runtime::{Agent, AgentConfig, Command, Input};
use serde_json::{Value as J, json};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
const T0: i64 = 1_791_000_000_000;

fn encoder() -> Encoder {
    Encoder::from_file(root().join("ref/tokenizers/zai-org__GLM-5.3.json").to_str().unwrap()).unwrap()
}

fn app(enc: &Encoder, name: &str, body: &str) -> View {
    let src = format!(
        r#"import {{ TurnStrategy }} from "{root}/views/strategies/turn_strategy.tsx";
import {{ useConversation, useLoop }} from "{root}/views/lib/loop.tsx";
import {{ useLog }} from "{root}/views/lib/log.tsx";
import {{ GLM53App }} from "{root}/views/template_apps/glm53.tsx";
{body}"#,
        root = root().display()
    );
    let dir = std::env::temp_dir().join(format!("aff-page-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{name}.tsx"));
    std::fs::write(&path, src).unwrap();
    View::load(&path, "Main", enc).unwrap()
}

/// A chat with one slow tool; a message listener pushes the text and counts what it heard.
const CHAT: &str = r#"
export default function Main() {
  const log = useLog();
  useConversation(log);
  const [heard, setHeard] = useState(0, "heard");
  const app = <>
    <aff event="message" onCallback={({ text }) => { setHeard(h => h + 1); log.append({kind: "input", content: text}); }}/>
    <aff tool_name="lookup" description="Look something up." args_schema={{type: "object", properties: {q: {type: "string"}}, required: ["q"]}}
         onRun={async ({ q }) => (await harness.machine.call({cmd: q})).output}>{"[lookup]\n"}</aff>
  </>;
  return <GLM53App><TurnStrategy strategy="unrolled" log={log.records}>{app}</TurnStrategy></GLM53App>;
}"#;

struct Rig {
    enc: Encoder,
    view: View,
    agent: Agent,
    clock: i64,
}

impl Rig {
    fn new(body: &str, name: &str) -> (Rig, Vec<Command>) {
        let enc = encoder();
        let view = app(&enc, name, body);
        let mut agent = Agent::new(AgentConfig {
            context: None,
            retry: Default::default(),
            template: None,
            id: "a".into(),
            props: json!({}),
            env: BTreeMap::new(),
            max_new: 256,
            max_turns: Some(20),
            max_prompt: None,
            start_ms: T0,
            seed: 7,
            machines: vec!["home".into()],
            model: Some("default".into()),
        })
        .unwrap();
        let out = agent.start(&view, &enc).unwrap();
        (
            Rig {
                enc,
                view,
                agent,
                clock: T0,
            },
            out,
        )
    }
    fn feed(&mut self, input: Input) -> Vec<Command> {
        self.clock += 10;
        self.agent.handle(&self.view, &self.enc, input).unwrap()
    }
    fn message(&mut self, text: &str) -> Vec<Command> {
        let clock = self.clock + 10;
        self.feed(Input::Event {
            name: "message".into(),
            payload: json!({"text": text}),
            id: None,
            clock_ms: clock,
        })
    }
    fn reply(&mut self, task: u64, text: &str) -> Vec<Command> {
        let stop = if text.contains("<tool_call>") {
            "<|observation|>"
        } else {
            "<|user|>"
        };
        let mut ids = self.enc.tok.encode(text, false).unwrap().get_ids().to_vec();
        ids.push(self.enc.token_id(stop).unwrap());
        let clock = self.clock + 10;
        self.feed(Input::Reply {
            server: None,
            task,
            ids,
            cut_off: false,
            clock_ms: clock,
            prompt: None,
            aborted: false,
        })
    }
    fn kinds(&self) -> Vec<String> {
        affordance::scenario::log_records(&self.agent.state())
            .iter()
            .map(|r| r["kind"].as_str().unwrap().to_string())
            .collect()
    }
}

fn generates(cmds: &[Command]) -> Vec<(u64, Vec<u32>)> {
    cmds.iter()
        .filter_map(|c| match c {
            Command::Generate { task, ids, .. } => Some((*task, ids.to_vec())),
            _ => None,
        })
        .collect()
}
fn calls(cmds: &[Command]) -> Vec<u64> {
    cmds.iter()
        .filter_map(|c| match c {
            Command::Call { task, .. } => Some(*task),
            _ => None,
        })
        .collect()
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn a_chat_is_quiet_until_spoken_to_and_after_it_answers() {
    let (mut rig, start) = Rig::new(CHAT, "chat_idle");
    assert!(generates(&start).is_empty(), "nothing to answer: no model call");
    let out = rig.message("hello");
    let g = generates(&out);
    assert_eq!(g.len(), 1);
    let first_prompt = g[0].1.clone();
    let out = rig.reply(g[0].0, "Greeting.</think>Hi there.");
    assert!(
        generates(&out).is_empty(),
        "a bare answer ends the turn: no further model call"
    );
    assert!(rig.agent.quiescent(), "idle");
    let out = rig.message("how are you?");
    let g = generates(&out);
    assert_eq!(g.len(), 1);
    assert!(g[0].1.starts_with(&first_prompt), "an exact append of the conversation");
    assert_eq!(rig.agent.metrics.rewrites, 0);
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn messages_during_an_answer_are_written_after_it_and_answered_together() {
    let (mut rig, _) = Rig::new(CHAT, "chat_three");
    let g = generates(&rig.message("m1"));
    // m2 and m3 arrive while the model answers m1: no second model call, nothing written yet.
    assert!(generates(&rig.message("m2")).is_empty());
    assert!(generates(&rig.message("m3")).is_empty());
    assert_eq!(rig.kinds(), vec!["input"]);
    // The answer to m1 lands; m2 and m3 follow it, and one model call answers both.
    let out = rig.reply(g[0].0, "Answering.</think>About m1.");
    assert_eq!(rig.kinds(), vec!["input", "screen", "model", "input", "input"]);
    let g2 = generates(&out);
    assert_eq!(g2.len(), 1);
    let text = rig.enc.tok.decode(&g2[0].1, false).unwrap();
    assert!(
        text.contains("About m1.") && text.find("m2").unwrap() > text.find("About m1.").unwrap(),
        "{text}"
    );
    assert_eq!(rig.agent.metrics.rewrites, 0);
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn a_listener_runs_while_a_tool_waits() {
    let (mut rig, _) = Rig::new(CHAT, "chat_tool");
    let g = generates(&rig.message("look it up"));
    let out = rig.reply(
        g[0].0,
        "Lookup.</think><tool_call>lookup<arg_key>q</arg_key><arg_value>weather</arg_value></tool_call>",
    );
    let call = calls(&out)[0];
    // A message while the tool's call is out: its listener runs now (the hook moves), its push waits.
    rig.message("also: hurry");
    assert_eq!(rig.agent.state()["__ui"]["@heard"], 2);
    assert_eq!(rig.kinds(), vec!["input", "screen", "model"]);
    // The tool's result closes the fire; the push follows the action, and the model is called again.
    let clock = rig.clock + 10;
    let out = rig.feed(Input::Result {
        task: call,
        outcome: Outcome::Ok(json!({"output": "sunny"})),
        clock_ms: clock,
    });
    assert_eq!(rig.kinds(), vec!["input", "screen", "model", "action", "input"]);
    assert_eq!(generates(&out).len(), 1);
}

/// An output that is never dispatched runs none of its calls; the log closes the turn with
/// `not run` results (`log.close`), so the model sees what happened.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn an_output_not_dispatched_runs_nothing_and_closes_with_not_run_results() {
    let body = r#"
export default function Main() {
  const log = useLog();
  const [done, setDone] = useState(false, "done");
  useEffect(() => {
    if (done) return;
    const once = async () => { const out = await log.call(); log.close(out, "the app chose not to act"); setDone(true); };
    once();
  }, [done]);
  const app = <aff tool_name="lookup" description="Look." args_schema={{type: "object", properties: {q: {type: "string"}}, required: ["q"]}}
                   onRun={({ q }) => "x"}>{"[lookup]\n"}</aff>;
  return <GLM53App><TurnStrategy strategy="unrolled" log={[{kind: "input", content: "go"}, ...log.records]}>{app}</TurnStrategy></GLM53App>;
}"#;
    let (mut rig, start) = Rig::new(body, "auto_close");
    let g = generates(&start);
    assert_eq!(g.len(), 1);
    let out = rig.reply(
        g[0].0,
        "Calling.</think><tool_call>lookup<arg_key>q</arg_key><arg_value>a</arg_value></tool_call>",
    );
    assert!(calls(&out).is_empty(), "the call did not run");
    let t = affordance::scenario::log_records(&rig.agent.state());
    let action = t.iter().find(|r| r["kind"] == "action").unwrap();
    assert!(
        action["result"]
            .as_str()
            .unwrap()
            .contains("not run (the app chose not to act)"),
        "{action}"
    );
    assert!(rig.agent.quiescent());
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn an_effect_cleans_up_when_its_deps_change() {
    let body = r#"
export default function Main() {
  const [round, setRound] = useState(0, "round");
  const [cleaned, setCleaned] = useState(0, "cleaned");
  useEffect(() => () => setCleaned(c => c + 1), [round]);
  return <aff event="next" onCallback={() => setRound(r => r + 1)}/>;
}"#;
    let (mut rig, _) = Rig::new(body, "cleanup");
    let clock = rig.clock + 10;
    rig.feed(Input::Event {
        name: "next".into(),
        payload: json!({}),
        id: None,
        clock_ms: clock,
    });
    let clock = rig.clock + 10;
    rig.feed(Input::Event {
        name: "next".into(),
        payload: json!({}),
        id: None,
        clock_ms: clock,
    });
    assert_eq!(rig.agent.state()["__ui"]["@round"], 2);
    assert_eq!(
        rig.agent.state()["__ui"]["@cleaned"],
        2,
        "the cleanup of round 0, then of round 1"
    );
}

/// Replay: a journal of the three-messages chat restores to the same transcript and state.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn the_chat_replays_from_its_journal() {
    let (mut rig, _) = Rig::new(CHAT, "chat_replay");
    let mut journal = vec![];
    let mut feed = |rig: &mut Rig, input: Input| {
        journal.push(input.clone());
        rig.agent.handle(&rig.view, &rig.enc, input).unwrap()
    };
    let ev = |t: &str, c: i64| Input::Event {
        name: "message".into(),
        payload: json!({"text": t}),
        id: None,
        clock_ms: c,
    };
    let g = generates(&feed(&mut rig, ev("m1", T0 + 10)));
    feed(&mut rig, ev("m2", T0 + 20));
    let mut ids = rig
        .enc
        .tok
        .encode("Ok.</think>Fine.", false)
        .unwrap()
        .get_ids()
        .to_vec();
    ids.push(rig.enc.token_id("<|user|>").unwrap());
    feed(
        &mut rig,
        Input::Reply {
            server: None,
            task: g[0].0,
            ids,
            cut_off: false,
            clock_ms: T0 + 30,
            prompt: None,
            aborted: false,
        },
    );
    let cfg = AgentConfig {
        context: None,
        retry: Default::default(),
        template: None,
        id: "a".into(),
        props: json!({}),
        env: BTreeMap::new(),
        max_new: 256,
        max_turns: Some(20),
        max_prompt: None,
        start_ms: T0,
        seed: 7,
        machines: vec!["home".into()],
        model: Some("default".into()),
    };
    let (b, outstanding) = Agent::restore(cfg, journal, &rig.view, &rig.enc).unwrap();
    assert_eq!(
        affordance::scenario::log_records(&b.state()),
        affordance::scenario::log_records(&rig.agent.state())
    );
    assert_eq!(b.state(), rig.agent.state());
    assert_eq!(
        generates(&outstanding).len(),
        1,
        "the pending answer to m2 is re-issued"
    );
}

/// A tool answers when its body returns (a Service Worker's `respondWith`): the model is called
/// again at once, and the work it started without awaiting goes on and lands later.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn a_tool_answers_when_it_returns_and_its_unawaited_work_goes_on() {
    let body = r#"
export default function Main() {
  const log = useLog();
  useConversation(log);
  const [got, setGot] = useState("", "got");
  const app = <>
    <aff event="message" onCallback={({ text }) => log.append({kind: "input", content: text})}/>
    <aff tool_name="start" description="Start a job." args_schema={{type: "object", properties: {}}}
         onStart={() => { harness.machine.call({cmd: "job"}).then(r => setGot(r.output)); return "started"; }}>{"[start]\n"}</aff>
  </>;
  return <GLM53App><TurnStrategy strategy="unrolled" log={log.records}>{app}</TurnStrategy></GLM53App>;
}"#;
    let (mut rig, _) = Rig::new(body, "respond");
    let g = generates(&rig.message("go"));
    let out = rig.reply(g[0].0, "Start.</think><tool_call>start</tool_call>");
    let job = calls(&out)[0];
    assert_eq!(
        generates(&out).len(),
        1,
        "the tool answered `started`: the model is called again while the job runs"
    );
    assert_eq!(rig.kinds(), vec!["input", "screen", "model", "action"]);
    let clock = rig.clock + 10;
    rig.feed(Input::Result {
        task: job,
        outcome: Outcome::Ok(json!({"output": "done"})),
        clock_ms: clock,
    });
    assert_eq!(rig.agent.state()["__ui"]["@got"], "done");
}

/// `harness.wait({ms})` reads milliseconds as `setTimeout` does: a fraction is truncated, not "no deadline".
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn a_wait_with_fractional_ms_times_out() {
    let body = r#"
export default function Main() {
  const [woke, setWoke] = useState("", "woke");
  return <aff event="go" onCallback={async () => { const r = await harness.wait({ms: 1500.5}); setWoke(r.woke); }}/>;
}"#;
    let (mut rig, _) = Rig::new(body, "wait_fraction");
    let clock = rig.clock + 10;
    rig.feed(Input::Event {
        name: "go".into(),
        payload: json!({}),
        id: None,
        clock_ms: clock,
    });
    let clock = rig.clock + 2000;
    rig.feed(Input::Tick { clock_ms: clock });
    assert_eq!(rig.agent.state()["__ui"]["@woke"], "timeout");
}

/// A declared function is a handler like an arrow (`onRun={lookup}`): it used to be dropped
/// silently, leaving no tool and no error.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn a_declared_function_is_a_tool_handler() {
    let body = r#"
function lookup({ q }) { return "found " + q; }
export default function Main() {
  const log = useLog();
  useConversation(log);
  const app = <>
    <aff event="message" onCallback={({ text }) => log.append({kind: "input", content: text})}/>
    <aff tool_name="lookup" description="Look." args_schema={{type: "object", properties: {q: {type: "string"}}, required: ["q"]}} onRun={lookup}>{"[lookup]\n"}</aff>
  </>;
  return <GLM53App><TurnStrategy strategy="unrolled" log={log.records}>{app}</TurnStrategy></GLM53App>;
}"#;
    let (mut rig, _) = Rig::new(body, "declared");
    let g = generates(&rig.message("hi"));
    rig.reply(
        g[0].0,
        "Look.</think><tool_call>lookup<arg_key>q</arg_key><arg_value>x</arg_value></tool_call>",
    );
    let t = affordance::scenario::log_records(&rig.agent.state());
    assert_eq!(t.iter().find(|r| r["kind"] == "action").unwrap()["result"], "found x");
}

#[allow(dead_code)]
fn unused(_: J) {}

/// A listener is a fact of the evaluated document, not of its layout: an `<aff event>` inside a
/// box cropped to nothing, or inside a `display: none` element, still hears its event (the spec
/// keeps listeners mounted when hidden). Event delivery reads the evaluated document's listener
/// table and lays nothing out; this pins that it finds the same listeners as before.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn a_listener_in_a_cropped_or_hidden_box_hears_its_event() {
    const APP: &str = r#"
export default function Main() {
  const log = useLog();
  const [heard, setHeard] = useState([], "heard");
  const app = <>
    <box max_tokens={2}>{"a long line that the box crops away entirely ........ "}<aff event="ping" onCallback={() => setHeard(h => [...h, "cropped"])}/></box>
    <box style={{display: "none"}}><aff event="ping" onCallback={() => setHeard(h => [...h, "hidden"])}/></box>
  </>;
  return <GLM53App><TurnStrategy strategy="unrolled" log={log.records}>{app}</TurnStrategy></GLM53App>;
}"#;
    let (mut rig, _) = Rig::new(APP, "listener_cropped");
    let clock = rig.clock + 10;
    rig.feed(Input::Event {
        name: "ping".into(),
        payload: json!({}),
        id: None,
        clock_ms: clock,
    });
    let heard = rig.agent.state()["__ui"]["@heard"].clone();
    assert_eq!(heard, json!(["cropped", "hidden"]), "{}", rig.agent.state());
}

/// `useRef`: storage that does not render. A handler's writes to it alone keep the last
/// evaluated document current (no new evaluation for them); a render that reads it fails.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn a_ref_does_not_render() {
    const APP: &str = r#"
export default function Main() {
  const log = useLog();
  const [count, setCount] = useRef(0, "pings");
  const app = <aff event="ping" onCallback={() => setCount(n => n + 1)}/>;
  return <GLM53App><TurnStrategy strategy="unrolled" log={log.records}>{app}</TurnStrategy></GLM53App>;
}"#;
    let (mut rig, _) = Rig::new(APP, "ref_quiet");
    let evaluations = rig.agent.render_stats.evaluations;
    for _ in 0..3 {
        let clock = rig.clock + 10;
        rig.feed(Input::Event {
            name: "ping".into(),
            payload: json!({}),
            id: None,
            clock_ms: clock,
        });
    }
    assert_eq!(rig.agent.state()["__ui"]["~pings"], json!(3), "the writes are kept");
    assert_eq!(
        rig.agent.render_stats.evaluations, evaluations,
        "no evaluation for them"
    );
    // Reading a ref during render is an error.
    const BAD: &str = r#"
export default function Main() {
  const log = useLog();
  const [count] = useRef(0, "n");
  return <GLM53App><TurnStrategy strategy="unrolled" log={log.records}>{String(count())}</TurnStrategy></GLM53App>;
}"#;
    let enc = encoder();
    let view = app(&enc, "ref_read", BAD);
    let mut agent = Agent::new(AgentConfig {
        context: None,
        retry: Default::default(),
        template: None,
        id: "a".into(),
        props: json!({}),
        env: BTreeMap::new(),
        max_new: 256,
        max_turns: Some(2),
        max_prompt: None,
        start_ms: T0,
        seed: 7,
        machines: vec![],
        model: Some("default".into()),
    })
    .unwrap();
    let e = agent.start(&view, &enc).expect_err("a render read of a ref fails");
    assert!(e.contains("a ref is read in handlers, not during render"), "{e}");
}
