//! App-library lifecycle regressions, using deterministic commands rather than real servers.
use crate::tokenizer;
use affordance::{
    View,
    effects::Outcome,
    runtime::{Agent, AgentConfig, Command, Input},
};
use serde_json::{Value as J, json};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

struct Rig {
    config: AgentConfig,
    agent: Agent,
    view: View,
    enc: affordance::emit::Encoder,
    clock: i64,
    dir: PathBuf,
    journal: Vec<Input>,
}
impl Rig {
    fn new(source: Option<&str>, props: J) -> (Self, Vec<Command>) {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "aff-lifecycle-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = if let Some(source) = source {
            let path = dir.join("app.tsx");
            std::fs::write(&path, source.replace("@ROOT@", env!("CARGO_MANIFEST_DIR"))).unwrap();
            path
        } else {
            PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/views/demo/mcp/app.tsx"))
        };
        let enc = tokenizer::encoder();
        let view = View::load(&path, "Main", &enc).unwrap();
        let config = AgentConfig {
            context: None,
            retry: Default::default(),
            template: None,
            id: "a".into(),
            props,
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
        let commands = agent.start(&view, &enc).unwrap();
        (
            Self {
                config,
                agent,
                view,
                enc,
                clock: 0,
                dir,
                journal: vec![],
            },
            commands,
        )
    }
    fn input(&mut self, input: Input) -> Vec<Command> {
        self.clock = input.clock_ms();
        self.journal.push(input.clone());
        self.agent.handle(&self.view, &self.enc, input).unwrap()
    }
    fn event(&mut self, name: &str) -> Vec<Command> {
        self.clock += 1;
        self.input(Input::Event {
            name: name.into(),
            payload: json!({}),
            id: None,
            clock_ms: self.clock,
        })
    }
    fn reply(&mut self, commands: &[Command]) -> Vec<Command> {
        let task = commands
            .iter()
            .find_map(|c| {
                if let Command::Generate { task, .. } = c {
                    Some(*task)
                } else {
                    None
                }
            })
            .unwrap();
        let mut ids = self.enc.tok.encode("</think>answer", false).unwrap().get_ids().to_vec();
        ids.push(self.enc.token_id("<|user|>").unwrap());
        self.clock += 1;
        self.input(Input::Reply {
            server: None,
            task,
            ids,
            cut_off: false,
            clock_ms: self.clock,
            prompt: None,
            aborted: false,
        })
    }
    fn check_restore(&self) {
        let (restored, _) = Agent::restore(self.config.clone(), self.journal.clone(), &self.view, &self.enc).unwrap();
        assert_eq!(restored.state(), self.agent.state());
        assert_eq!(restored.metrics.handler_errors, 0);
        assert_eq!(self.agent.metrics.handler_errors, 0);
    }
    fn props(&mut self, url: &str, alias: &str) -> Vec<Command> {
        self.clock += 1;
        self.input(Input::Props {
            props: json!({"url":url,"expose":{"search":alias}}),
            clock_ms: self.clock,
        })
    }
    fn http(&mut self, command: &Command, result: J) -> Vec<Command> {
        let Command::Call { task, args, .. } = command else {
            panic!("expected HTTP call")
        };
        let body: J = serde_json::from_str(args["body"].as_str().unwrap()).unwrap();
        self.clock += 1;
        self.input(Input::Result{task:*task,outcome:Outcome::Ok(json!({"status":200,"headers":{"content-type":["application/json"]},"body":json!({"jsonrpc":"2.0","id":body["id"],"result":result}).to_string()})),clock_ms:self.clock})
    }
    fn message(&mut self) -> Vec<Command> {
        self.clock += 1;
        self.input(Input::Event {
            name: "message".into(),
            payload: json!({"text":"find it"}),
            id: None,
            clock_ms: self.clock,
        })
    }
    fn catalogue(&self) -> J {
        self.agent.state()["__ui"]["@mcp.catalogue"].clone()
    }
}
impl Drop for Rig {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}
fn call(commands: &[Command]) -> Command {
    commands
        .iter()
        .find(|c| matches!(c, Command::Call { .. }))
        .expect("HTTP request")
        .clone()
}
fn discovery() -> J {
    json!({"supportedVersions":["2026-07-28"],"capabilities":{"tools":{}}})
}
fn tools(description: &str) -> J {
    json!({"tools":[{"name":"search","description":description,"inputSchema":{"type":"object"}}]})
}
fn generates(commands: &[Command]) -> usize {
    commands
        .iter()
        .filter(|c| matches!(c, Command::Generate { .. }))
        .count()
}

#[test]
fn a_log_rejects_overlapping_calls_without_closing_the_owners_turn() {
    let source = r#"import {useLog} from '@ROOT@/views/lib/log.tsx';
import {GLM53App} from '@ROOT@/views/template_apps/glm53.tsx';
import {TurnStrategy} from '@ROOT@/views/strategies/turn_strategy.tsx';
export default function Main(){const log=useLog();const [error,setError]=useState('','error');
useEffect(()=>{(async()=>{const out=await log.call();log.close(out);})();(async()=>{try{await log.call();}catch(e){setError(e.message);}})();},[]);
return <GLM53App><TurnStrategy strategy='unrolled' log={log.records}>hello</TurnStrategy></GLM53App>;}"#;
    let (mut rig, commands) = Rig::new(Some(source), json!({}));
    assert_eq!(generates(&commands), 1);
    let state = rig.agent.state();
    assert_eq!(state["__ui"]["~log~turn"]["phase"], "calling");
    assert!(state["__ui"]["@error"].as_str().unwrap().contains("open turn"));
    let Command::Generate { task, .. } = commands.iter().find(|c| matches!(c, Command::Generate { .. })).unwrap()
    else {
        unreachable!()
    };
    let mut ids = rig.enc.tok.encode("</think>done", false).unwrap().get_ids().to_vec();
    ids.push(rig.enc.token_id("<|user|>").unwrap());
    rig.input(Input::Reply {
        server: None,
        task: *task,
        ids,
        cut_off: false,
        clock_ms: 1,
        prompt: None,
        aborted: false,
    });
    let trace = rig.agent.take_trace();
    assert_eq!(
        rig.agent.state()["__ui"]["~log~turn"]["phase"],
        "closed",
        "{:?} {:?}",
        trace,
        rig.agent.state()
    );
    assert_eq!(
        rig.agent.state()["__ui"]["@log"]["records"].as_array().unwrap().len(),
        2
    );
}

#[test]
fn changing_mcp_endpoint_or_selection_hides_old_tools_until_discovered() {
    let (mut rig, commands) = Rig::new(None, json!({"url":"/original","expose":{"search":"search_docs"}}));
    let list = rig.http(&call(&commands), discovery());
    rig.http(&call(&list), tools("old catalogue"));
    assert_eq!(rig.catalogue()["ready"], true);
    let reload = rig.props("/changed", "lookup");
    assert_eq!(rig.catalogue()["ready"], false);
    assert_eq!(rig.catalogue()["tools"], json!([]));
    assert_eq!(
        generates(&rig.message()),
        0,
        "conversation ran with undiscovered endpoint"
    );
    let list = rig.http(&call(&reload), discovery());
    let answer = rig.http(&call(&list), tools("new catalogue"));
    assert_eq!(generates(&answer), 1);
    let Command::Generate { task, ids, .. } = answer.iter().find(|c| matches!(c, Command::Generate { .. })).unwrap()
    else {
        unreachable!()
    };
    let prompt = rig.enc.tok.decode(ids, false).unwrap();
    assert!(prompt.contains("lookup"));
    assert!(!prompt.contains("old catalogue"));
    let mut ids = rig
        .enc
        .tok
        .encode("</think><tool_call>lookup</tool_call>", false)
        .unwrap()
        .get_ids()
        .to_vec();
    ids.push(rig.enc.token_id("<|observation|>").unwrap());
    rig.clock += 1;
    let dispatched = rig.input(Input::Reply {
        server: None,
        task: *task,
        ids,
        cut_off: false,
        clock_ms: rig.clock,
        prompt: None,
        aborted: false,
    });
    let Command::Call { args, .. } = call(&dispatched) else {
        unreachable!()
    };
    assert_eq!(args["path"], "/changed", "{args}");
    let body: J = serde_json::from_str(args["body"].as_str().unwrap()).unwrap();
    assert_eq!(body["method"], "tools/call");
    assert_eq!(body["params"]["name"], "search");
    rig.props("/changed", "new_alias");
    assert_eq!(rig.catalogue()["ready"], false);
}

#[test]
fn late_mcp_discovery_cannot_replace_a_new_generation_even_at_the_same_url() {
    for stale_failure in [false, true] {
        let (mut rig, commands) = Rig::new(None, json!({"url":"/same","expose":{"search":"search_docs"}}));
        let old_list = rig.http(&call(&commands), discovery());
        rig.props("/other", "search_docs");
        let latest = rig.props("/same", "search_docs");
        let list = rig.http(&call(&latest), discovery());
        rig.http(&call(&list), tools("latest catalogue"));
        let expected = rig.catalogue();
        if stale_failure {
            let Command::Call { task, .. } = call(&old_list) else {
                unreachable!()
            };
            rig.clock += 1;
            rig.input(Input::Result {
                task,
                outcome: Outcome::Error("old endpoint failed".into()),
                clock_ms: rig.clock,
            });
        } else {
            rig.http(&call(&old_list), tools("stale catalogue"));
        }
        let trace = rig.agent.take_trace();
        assert_eq!(rig.catalogue(), expected);
        assert!(rig.agent.stopped().is_none(), "{:?} {:?}", rig.agent.stopped(), trace);
        assert_eq!(
            generates(&rig.message()),
            1,
            "stale completion disabled the current app"
        );
    }
}

#[test]
fn a_failed_updater_rethrows_its_value_after_suspension_and_journal_restore() {
    let source = r#"export default function Main(){const [n,setN]=useState(0,'n');const [seen,setSeen]=useState(0,'seen');
useEffect(()=>{(async()=>{let caught;try{setN(()=>{throw {code:41};});}catch(e){caught=e.code;}await harness.wait({ms:10});setSeen(caught);})();},[]);return String(n);}"#;
    let (mut rig, commands) = Rig::new(Some(source), json!({}));
    assert!(
        commands.iter().all(|c| matches!(c, Command::Timer { .. })),
        "unexpected host call: {commands:?}"
    );
    let input = Input::Tick { clock_ms: 20 };
    rig.input(input.clone());
    assert_eq!(rig.agent.state()["__ui"]["@n"], 0);
    assert_eq!(rig.agent.state()["__ui"]["@seen"], 41);
    assert_eq!(rig.agent.metrics.handler_errors, 0);
    let (restored, _) = Agent::restore(rig.config.clone(), vec![input], &rig.view, &rig.enc).unwrap();
    assert_eq!(restored.state(), rig.agent.state());
    assert_eq!(restored.metrics.handler_errors, 0);
}

#[test]
fn an_updater_can_resume_its_clock_read_without_issuing_an_internal_host_call() {
    let source = "export default function Main(){const [n,setN]=useState(0,'n');useEffect(()=>{setN(n=>{harness.now();return n+1;});},[]);return String(n);}";
    let (rig, commands) = Rig::new(Some(source), json!({}));
    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(rig.agent.state()["__ui"]["@n"], 1);
    assert_eq!(rig.agent.metrics.handler_errors, 0);
}

#[test]
fn caught_mutable_exceptions_are_reconstructed_across_multiple_awaits() {
    for thrown in [
        "throw {code:1};",
        "const e=Object.assign(new Error('x'),{code:1});throw e;",
        "const e={code:1};e.self=e;throw e;",
    ] {
        let source = format!(
            "export default function Main(){{const [n,setN]=useState(0,'n');const [done,setDone]=useState(0,'done');useEvent('update',async()=>{{let code;try{{setN(()=>{{{thrown}}});}}catch(e){{code=e.code;e.code++;}}await harness.wait({{ms:10}});await harness.store.put({{text:String(code)}});await harness.wait({{ms:10}});setDone(code);}});return String(n);}}"
        );
        let (mut rig, _) = Rig::new(Some(&source), json!({}));
        rig.event("update");
        let calls = rig.input(Input::Tick { clock_ms: 20 });
        let Command::Call { task, args, .. } = call(&calls) else {
            unreachable!()
        };
        assert_eq!(args["text"], "1");
        rig.input(Input::Result {
            task,
            outcome: Outcome::Ok(json!({"key":"b1"})),
            clock_ms: 30,
        });
        rig.input(Input::Tick { clock_ms: 100 });
        assert_eq!(rig.agent.state()["__ui"]["@done"], 1);
        rig.check_restore();
    }
}

#[test]
fn successful_and_nested_updaters_replay_computation_without_reapplying_writes() {
    let source = "export default function Main(){const [n,setN]=useState(0,'n');const [other,setOther]=useState(0,'other');const [seen,setSeen]=useState(0,'seen');useEvent('start',async()=>{let next;setN(n=>{harness.now();setOther(x=>x+1);next=n+1;return next;});await harness.wait({ms:10});setSeen(next);});useEvent('bump',()=>{setN(n=>n+10);setOther(n=>n+10);});return String(n);}";
    let (mut rig, _) = Rig::new(Some(source), json!({}));
    rig.event("start");
    rig.event("bump");
    rig.input(Input::Tick { clock_ms: 100 });
    let state = rig.agent.state();
    assert_eq!(state["__ui"]["@n"], 11);
    assert_eq!(state["__ui"]["@other"], 11);
    assert_eq!(state["__ui"]["@seen"], 1);
    rig.check_restore();
}

const LOG_APP: &str = r#"import {useLog} from '@ROOT@/views/lib/log.tsx';
import {GLM53App} from '@ROOT@/views/template_apps/glm53.tsx';
import {TurnStrategy} from '@ROOT@/views/strategies/turn_strategy.tsx';
export default function Main(){const log=useLog();const [previous,setPrevious]=useState(null,'previous');const [error,setError]=useState('','error');
useEffect(()=>{(async()=>{const out=await log.call();setPrevious(out);log.close(out);})();},[]);
useEvent('close',()=>{try{log.close();}catch(e){setError(e.message);}});
useEvent('input',()=>log.append({kind:'input',content:'held input',id:'forged'}));
useEvent('start',()=>{(async()=>{const out=await log.call();log.close(out);})();});
useEvent('old-close',()=>log.close(previous));
useEvent('old-dispatch',async()=>{try{await log.dispatch(previous);}catch(e){setError(e.message);}});
return <GLM53App><TurnStrategy strategy='unrolled' log={log.records}>hello</TurnStrategy></GLM53App>;}"#;

#[test]
fn log_cannot_close_an_unresolved_call_and_reopen_under_its_completion() {
    let (mut rig, first) = Rig::new(Some(LOG_APP), json!({}));
    rig.event("input");
    rig.event("close");
    assert!(!rig.agent.state()["__ui"]["@error"].as_str().unwrap().is_empty());
    assert_eq!(rig.agent.state()["__ui"]["~log~turn"]["phase"], "calling");
    rig.reply(&first);
    assert_eq!(rig.agent.state()["__ui"]["~log~turn"]["phase"], "closed");
    let state = rig.agent.state();
    let records = state["__ui"]["@log"]["records"].as_array().unwrap();
    assert_eq!(records.last().unwrap()["content"], "held input");
    for (i, record) in records.iter().enumerate() {
        assert_eq!(record["id"], format!("e{i}"));
    }
    rig.check_restore();
}

#[test]
fn old_log_outputs_cannot_close_or_dispatch_a_new_turn() {
    let (mut rig, first) = Rig::new(Some(LOG_APP), json!({}));
    rig.reply(&first);
    let second = rig.event("start");
    assert_eq!(generates(&second), 1);
    rig.event("old-close");
    rig.event("old-dispatch");
    assert_eq!(rig.agent.state()["__ui"]["~log~turn"]["phase"], "calling");
    rig.reply(&second);
    assert_eq!(rig.agent.state()["__ui"]["~log~turn"]["phase"], "closed");
    assert_eq!(
        rig.agent.state()["__ui"]["@log"]["records"].as_array().unwrap().len(),
        4
    );
    rig.check_restore();
}

#[test]
fn log_dispatch_owns_its_turn_until_the_tools_settle() {
    let source = r#"import {useLog} from '@ROOT@/views/lib/log.tsx';import {GLM53App} from '@ROOT@/views/template_apps/glm53.tsx';import {TurnStrategy} from '@ROOT@/views/strategies/turn_strategy.tsx';
export default function Main(){const log=useLog();const [out,setOut]=useState(null,'out');const [error,setError]=useState('','error');useEffect(()=>{(async()=>{const result=await log.call();setOut(result);await log.dispatch(result);})();},[]);useEvent('close',()=>{try{log.close(out);}catch(e){setError(e.message);}});return <GLM53App><TurnStrategy strategy='unrolled' log={log.records}><aff tool_name='wait' args_schema={{type:'object',properties:{}}} onRun={async()=>{await harness.wait({ms:10});return 'done';}}>wait</aff></TurnStrategy></GLM53App>;}"#;
    let (mut rig, commands) = Rig::new(Some(source), json!({}));
    let task = commands
        .iter()
        .find_map(|c| {
            if let Command::Generate { task, .. } = c {
                Some(*task)
            } else {
                None
            }
        })
        .unwrap();
    let mut ids = rig
        .enc
        .tok
        .encode("</think><tool_call>wait</tool_call>", false)
        .unwrap()
        .get_ids()
        .to_vec();
    ids.push(rig.enc.token_id("<|observation|>").unwrap());
    rig.input(Input::Reply {
        server: None,
        task,
        ids,
        cut_off: false,
        clock_ms: 1,
        prompt: None,
        aborted: false,
    });
    assert_eq!(rig.agent.state()["__ui"]["~log~turn"]["phase"], "dispatching");
    rig.event("close");
    assert!(!rig.agent.state()["__ui"]["@error"].as_str().unwrap().is_empty());
    rig.input(Input::Tick { clock_ms: 100 });
    assert_eq!(rig.agent.state()["__ui"]["~log~turn"]["phase"], "closed");
    rig.check_restore();
}

#[test]
fn a_setter_ignores_its_call_receiver_when_it_invokes_the_updater() {
    let source = "export default function Main(){const [n,setN]=useState(0,'n');useEvent('update',()=>{const o={setN};o.setN(function(n){return this===undefined?n+1:99;});});return String(n);}";
    let (mut rig, _) = Rig::new(Some(source), json!({}));
    rig.event("update");
    assert_eq!(rig.agent.state()["__ui"]["@n"], 1);
    rig.check_restore();
}

#[test]
fn replay_preserves_frozen_reference_identity_and_preexisting_collection_keys() {
    for expression in [
        "({n:1})",
        "[1,2]",
        "new Map([['a',1]])",
        "new Set([1,2])",
        "/a/g",
        "(()=>7)",
        "((x)=>x).bind(null,7)",
        "new Error('same')",
        "Promise.resolve(7)",
        "Promise.resolve({n:1})",
        "Promise.reject({code:7})",
        "Promise.all([{n:1},{n:2}])",
        "Promise.allSettled([Promise.resolve({n:1}),Promise.reject({code:7})])",
    ] {
        for observe_first in [false, true] {
            let source = format!(
                r#"export default function Main(){{
          const [x,setX]=useState(null,'x');const [done,setDone]=useState(false,'done');
          const [checked,setChecked]=useState(false,'checked');
          useEvent('start',async()=>{{
            harness.now();
            const shared={expression};const distinct={expression};
            let completion;
            if({observe_first}){{try{{completion=await shared;}}catch(e){{completion=e;}}}}
            const keyed=new Map([[shared,'found']]);const members=new Set([shared]);
            const check=cur=>{{
              if(cur.a!==shared || cur.a!==cur.b || cur.a===distinct || !keyed.has(cur.a) || !members.has(cur.a)) throw new Error('identity');
              keyed.set(cur.a,'updated');if(keyed.size!==1)throw new Error('duplicate key');
              return cur;
            }};
            setX({{a:shared,b:shared}});setX(check);
            if(!{observe_first}){{try{{completion=await shared;}}catch(e){{completion=e;}}}}
            if(Array.isArray(completion)){{
              let frozen=false;try{{completion.push(3);}}catch(e){{frozen=true;}}
              if(!frozen)throw new Error('retained completion is mutable');
            }}
            const completions=new Map([[completion,'found']]);
            await harness.wait({{ms:10}});setX(check);
            let repeated;try{{repeated=await shared;}}catch(e){{repeated=e;}}
            if(completion!==repeated || !completions.has(repeated))throw new Error('completion identity');
            setDone(true);
          }});
          useEvent('later',async()=>{{
            let selected;try{{selected=await Promise.race([x.a,Promise.resolve('other')]);}}catch(e){{selected=e;}}
            let original;try{{original=await x.a;}}catch(e){{original=e;}}
            if(selected!==original)throw new Error('completed promise was treated as future work');
            setChecked(true);
          }});return '';
        }}"#
            );
            let (mut rig, _) = Rig::new(Some(&source), json!({}));
            rig.event("start");
            rig.input(Input::Tick { clock_ms: 100 });
            assert_eq!(rig.agent.state()["__ui"]["@done"], true, "{expression}");
            rig.event("later");
            assert_eq!(rig.agent.state()["__ui"]["@checked"], true, "{expression}");
            rig.check_restore();
        }
    }
}

#[test]
fn replay_preserves_evaluated_class_brands_and_private_members() {
    let source = r#"export default function Main(){
      const [x,setX]=useState(null,'x');const [done,setDone]=useState(false,'done');
      useEvent('start',async()=>{
        class Box { #n=9; static #s=11; read(){return this.#n;} static read(){return this.#s;} }
        const box=new Box();const method=box.read;const keyed=new Map([[Box,'class'],[method,'method']]);
        const check=cur=>{
          if(cur.Box!==Box || cur.box!==box || !(cur.box instanceof Box) || !(box instanceof cur.Box))throw new Error('class identity');
          if(method.call(cur.box)!==9 || cur.box.read.call(box)!==9 || Box.read.call(cur.Box)!==11)throw new Error('brand');
          if(keyed.get(cur.Box)!=='class' || keyed.get(cur.box.read)!=='method')throw new Error('method key');
          return cur;
        };
        setX({Box,box});setX(check);await harness.wait({ms:10});setX(check);setDone(true);
      });return '';
    }"#;
    let (mut rig, _) = Rig::new(Some(source), json!({}));
    rig.event("start");
    rig.input(Input::Tick { clock_ms: 100 });
    assert_eq!(rig.agent.state()["__ui"]["@done"], true);
    rig.check_restore();
}

#[test]
fn replay_reference_identity_survives_interleaved_inputs_and_new_writes() {
    let source = r#"export default function Main(){
      const [x,setX]=useState(null,'x');const [saved,setSaved]=useState(null,'saved');const [done,setDone]=useState(false,'done');
      useEvent('start',async()=>{
        const item={n:1};setX({item});
        await harness.wait({ms:10});
        setX(cur=>{if(cur.item!==item || cur.other!==1)throw new Error('first continuation');return {...cur,item};});
        await harness.wait({ms:10});
        setSaved(cur=>{if(cur!==item)throw new Error('interleaved reference');return cur;});setDone(true);
      });
      useEvent('bump',()=>{setSaved(x.item);setX(cur=>({...cur,other:1}));});
      return '';
    }"#;
    let (mut rig, _) = Rig::new(Some(source), json!({}));
    rig.event("start");
    rig.event("bump");
    rig.input(Input::Tick { clock_ms: 100 });
    rig.event("bump");
    rig.input(Input::Tick { clock_ms: 200 });
    assert_eq!(rig.agent.state()["__ui"]["@done"], true);
    assert_eq!(rig.agent.state()["__ui"]["@x"]["other"], 1);
    rig.check_restore();
}

#[test]
fn shared_pending_promises_resume_their_owners_and_restore() {
    let source = r#"
    function Joined({p, setSeen}) {
      const [joined] = useState(() => Promise.all([p, p]));
      useEvent('watch', async () => { const [a, b] = await joined; setSeen(a === b ? a.n : -1); });
      return '';
    }
    export default function Main() {
      const [p, setP] = useState(null, 'pending');
      const [own, setOwn] = useState(0, 'own');
      const [seen, setSeen] = useState(0, 'seen');
      useEvent('begin', async () => {
        const p = harness.machine.get(await Promise.resolve('http')).call(await Promise.resolve({q: 'object'}));
        setP(p);
        setOwn((await p).n);
      });
      return p ? <Joined p={p} setSeen={setSeen}/> : '';
    }"#;
    let (mut rig, initial) = Rig::new(Some(source), json!({}));
    assert!(initial.is_empty());
    let commands = rig.event("begin");
    assert!(rig.event("watch").is_empty());
    rig.check_restore();
    let task = commands
        .iter()
        .find_map(|c| match c {
            Command::Call { task, .. } => Some(*task),
            _ => None,
        })
        .unwrap();
    rig.input(Input::Result {
        task,
        outcome: Outcome::Ok(json!({"n":7})),
        clock_ms: 3,
    });
    assert_eq!(rig.agent.state()["__ui"]["@own"], 7);
    assert_eq!(rig.agent.state()["__ui"]["@seen"], 7);
    rig.check_restore();
}

#[test]
fn a_runtime_component_value_renders_and_registers_actions_inside_its_parent() {
    let source = r#"
    function makeChild(label) {
      return ({report}) => {
        const [count,setCount]=useState(0);
        useEvent('increment',()=>setCount(n=>n+1));
        useEvent('observe',()=>report(`${label}:${count}`));
        return <span>{label}{count}</span>;
      };
    }
    export default function Main() {
      const [current,setCurrent]=useState(null);
      const [seen,setSeen]=useState(null,'seen');
      useEvent('install',()=>setCurrent({View:makeChild('child')}));
      useEvent('remove',()=>setCurrent(null));
      useEvent('parent',()=>setSeen('parent'));
      return <>{'parent:'}{current && <current.View report={setSeen}/>}</>;
    }"#;
    let (mut rig, _) = Rig::new(Some(source), json!({}));
    let screen = |rig: &Rig| {
        let rendered = rig
            .view
            .render_with_harness(
                &rig.enc,
                rig.agent.kept_state().clone(),
                &mut affordance::emit::RunCache::new(false),
                chrono::DateTime::from_timestamp_millis(rig.clock).unwrap().naive_utc(),
                affordance::value::Value::from_json(&json!({"owner":"agent"})),
            )
            .unwrap();
        rig.enc.tok.decode(&rendered.ids, false).unwrap()
    };
    assert_eq!(screen(&rig), "parent:");
    rig.event("install");
    let trace = rig.agent.take_trace();
    assert_eq!(rig.agent.metrics.handler_errors, 0, "{trace:?}");
    assert_eq!(
        screen(&rig),
        "parent:child0",
        "state: {:?}, trace: {trace:?}",
        rig.agent.state()
    );
    rig.event("increment");
    rig.event("observe");
    assert_eq!(rig.agent.state()["__ui"]["@seen"], "child:1");
    assert_eq!(screen(&rig), "parent:child1");
    rig.check_restore();
    rig.event("remove");
    assert_eq!(screen(&rig), "parent:");
    rig.event("parent");
    rig.event("observe");
    assert_eq!(rig.agent.state()["__ui"]["@seen"], "parent");
    rig.check_restore();
}

#[test]
fn source_components_install_replace_undo_remove_and_recover_from_recorded_source() {
    let source = "import LiveComponents from '@ROOT@/views/demo/live_components.tsx'; export default function Main(){return <LiveComponents/>;}";
    let (mut rig, _) = Rig::new(Some(source), json!({}));
    let screen = |view: &View, agent: &Agent, rig: &Rig| {
        let r = view
            .render_with_harness(
                &rig.enc,
                agent.kept_state().clone(),
                &mut affordance::emit::RunCache::new(false),
                chrono::DateTime::from_timestamp_millis(rig.clock).unwrap().naive_utc(),
                affordance::value::Value::from_json(&json!({"owner":"agent"})),
            )
            .unwrap();
        rig.enc.tok.decode(&r.ids, false).unwrap()
    };
    let install = |rig: &mut Rig, source: &str| {
        rig.input(Input::Event {
            name: "install".into(),
            payload: json!({"source":source}),
            id: None,
            clock_ms: rig.clock + 1,
        });
        let trace = rig.agent.take_trace();
        assert_eq!(rig.agent.metrics.handler_errors, 0, "{trace:?}");
    };
    let revision = |label: &str| {
        format!(
            "const label={}; function helper(){{return label;}} export default function Panel(){{const [n,setN]=useState(0);return <aff event='increment' onCallback={{()=>setN(n=>n+1)}}>{{helper()}}{{n}}</aff>;}}",
            json!(label)
        )
    };
    let controls = "[install][remove][undo]";
    assert_eq!(screen(&rig.view, &rig.agent, &rig), controls);
    install(&mut rig, &revision("first"));
    rig.event("increment");
    assert_eq!(screen(&rig.view, &rig.agent, &rig), format!("{controls}first1"));
    install(&mut rig, "export default function Broken(");
    assert_eq!(rig.agent.state()["__ui"]["@source"]["selected"], 0);
    assert!(screen(&rig.view, &rig.agent, &rig).ends_with("first1"));
    install(&mut rig, &revision("second"));
    assert_eq!(screen(&rig.view, &rig.agent, &rig), format!("{controls}second0"));
    rig.event("undo");
    assert_eq!(screen(&rig.view, &rig.agent, &rig), format!("{controls}first0"));
    install(
        &mut rig,
        "export default function Bad(){useEvent('increment',()=>{throw new Error('abandoned listener');});throw new Error('bad render');}",
    );
    assert!(screen(&rig.view, &rig.agent, &rig).starts_with(&format!("{controls}Component failed: bad render")));
    rig.event("increment"); // The abandoned child's listener was rolled back.
    assert_eq!(rig.agent.metrics.handler_errors, 0);
    rig.event("undo");
    assert_eq!(screen(&rig.view, &rig.agent, &rig), format!("{controls}second0"));
    rig.event("remove");
    rig.event("increment");
    assert_eq!(screen(&rig.view, &rig.agent, &rig), controls);
    rig.event("undo");
    assert!(screen(&rig.view, &rig.agent, &rig).contains("Component failed: bad render"));
    rig.event("undo");
    rig.event("increment");
    let fresh = View::load(&rig.dir.join("app.tsx"), "Main", &rig.enc).unwrap();
    let (restored, _) = Agent::restore(rig.config.clone(), rig.journal.clone(), &fresh, &rig.enc).unwrap();
    assert_eq!(screen(&fresh, &restored, &rig), format!("{controls}second1"));
    assert_eq!(restored.state(), rig.agent.state());
    assert_eq!(restored.metrics.handler_errors, 0);
}

#[test]
fn source_components_can_declare_file_backed_child_instances() {
    let source = r#"
import {loadModule} from 'affordance:compiler';
import Hello from '@ROOT@/views/demo/hello.tsx';
export default function Main(){
  const [module]=useState(()=>loadModule("import Hello from './hello.tsx'; export default function Panel(){return <Mount key='child'><Hello name='live'/></Mount>;}",
    {imports:{'./hello.tsx':{default:Hello}}}));
  return <module.default/>;
}"#;
    let (rig, commands) = Rig::new(Some(source), json!({}));
    assert!(commands.iter().any(
        |c| matches!(c,Command::Mount{key,entry,props,..} if key=="child" && entry=="Hello" && props["name"]=="live")
    ));
    rig.check_restore();
}

#[test]
fn stable_declarations_and_batched_installs_work_through_model_dispatch_and_restore() {
    let source = r#"
import LiveComponents from '@ROOT@/views/demo/live_components.tsx';
import {GLM53App} from '@ROOT@/views/template_apps/glm53.tsx';
import {TurnStrategy} from '@ROOT@/views/strategies/turn_strategy.tsx';
function marker(){}
export default function Main(){
  const [runs,setRuns]=useState(0,'runs');
  const [saved]=useState(()=>({fn:marker,keys:new Map([[marker,'found']])}));
  useEffect(()=>setRuns(n=>n+1),[marker]);
  useEvent('offer',async()=>{await dispatch(await model());});
  if(saved.fn!==marker || saved.keys.get(marker)!=='found') throw new Error('unstable declaration');
  return <GLM53App><TurnStrategy strategy='unrolled' log={[]}><LiveComponents/></TurnStrategy></GLM53App>;
}"#;
    let (mut rig, _) = Rig::new(Some(source), json!({}));
    let commands = rig.event("offer");
    let task = commands
        .iter()
        .find_map(|c| match c {
            Command::Generate { task, .. } => Some(*task),
            _ => None,
        })
        .unwrap();
    let mut text = String::from("</think>");
    for label in ["first", "second"] {
        let module = format!("export default function Panel(){{return '{}';}}", label);
        text.push_str(&format!(
            "<tool_call>install<arg_key>source</arg_key><arg_value>{module}</arg_value></tool_call>"
        ));
    }
    let mut ids = rig.enc.tok.encode(text, false).unwrap().get_ids().to_vec();
    ids.push(rig.enc.token_id("<|observation|>").unwrap());
    rig.input(Input::Reply {
        server: None,
        task,
        ids,
        cut_off: false,
        clock_ms: rig.clock + 1,
        prompt: None,
        aborted: false,
    });
    let state = rig.agent.state();
    assert_eq!(state["__ui"]["@runs"], 1);
    assert_eq!(state["__ui"]["@source"]["selected"], 1);
    assert_eq!(state["__ui"]["@source"]["revisions"].as_array().unwrap().len(), 2);
    rig.check_restore();
}

#[test]
fn listener_prefixes_precede_microtasks_and_suspended_markup_preserves_order() {
    let source = r#"
    function Mark({onMark, value, children}) { onMark(value); return children; }
    export default function Main() {
      const [order,setOrder]=useState([], 'order');
      const [markup,setMarkup]=useState(null, 'markup');
      useEvent('arm',async()=>{await harness.wait({events:['go']});setOrder(x=>[...x,'waiter']);});
      useEvent('go',async()=>{setOrder(x=>[...x,'first']);await 0;setOrder(x=>[...x,'after']);});
      useEvent('go',()=>{setOrder(x=>[...x,'second']);});
      useEvent('go',async()=>{
        const seen=[];const mark=x=>{seen.push(x);return x;};
        const fragment=<box label="outer"><Mark onMark={mark} value="component">{mark('child')}</Mark>{await Promise.resolve(mark('await'))}<Mark style={{display:'none'}} onMark={mark} value="hidden">{mark('bad')}</Mark>{mark('tail')}</box>;
        const image=<img tok_size={await Promise.resolve(2)} t={tok.user.text}/>;
        let fallback;
        try {const failed=<box key="retry">{await Promise.reject('bad child')}</box>;}
        catch(error){fallback=<box key="retry">recovered</box>;}
        setMarkup([fragment.includes('childawaittail'),seen,image.includes(tok.user.text+tok.user.text),fallback.includes('recovered')]);
      });
      return '';
    }"#;
    let (mut rig, _) = Rig::new(Some(source), json!({}));
    assert!(rig.event("arm").is_empty());
    assert!(rig.event("go").is_empty());
    assert_eq!(
        rig.agent.state()["__ui"]["@order"],
        json!(["first", "second", "waiter", "after"])
    );
    assert_eq!(
        rig.agent.state()["__ui"]["@markup"],
        json!([
            false,
            ["child", "component", "await", "bad", "hidden", "tail"],
            true,
            true
        ])
    );
    rig.check_restore();
}

#[test]
fn failing_task_rejects_its_shared_async_result() {
    let source = r#"
    export default function Main(){
      const [pending,setPending]=useState(null);
      const [reason,setReason]=useState(null,'reason');
      useEvent('begin',async()=>{
        const work=n=>(async()=>{await harness.machine.call({q:'work'+n});return n;})();
        setPending([work(0),work(1)]);
        await harness.wait({events:['cancel']});
        throw new Error('cancel the owner');
      });
      useEvent('watch',async()=>{setReason(await Promise.race(pending.map((p,i)=>p.catch(e=>[i,e.name]))));});
      return '';
    }"#;
    let (mut rig, _) = Rig::new(Some(source), json!({}));
    let commands = rig.event("begin");
    assert!(commands.iter().any(|c| matches!(c, Command::Call { .. })));
    rig.event("watch");
    let cancelled = rig.event("cancel");
    assert!(cancelled.iter().any(|c| matches!(c, Command::Cancel { .. })));
    assert_eq!(rig.agent.state()["__ui"]["@reason"], json!([0, "AbortError"]));
    assert_eq!(rig.agent.metrics.handler_errors, 1);
    let (restored, _) = Agent::restore(rig.config.clone(), rig.journal.clone(), &rig.view, &rig.enc).unwrap();
    assert_eq!(restored.state(), rig.agent.state());
    assert_eq!(restored.metrics.handler_errors, 1);
}

#[test]
fn hidden_and_clipped_components_keep_lifecycle_but_keys_and_removal_reset_locals() {
    let source = r#"
function Child({report,trace}) {
  const [count,setCount]=useState(0);
  useEffect(()=>{trace(x=>[...x,'start']);return ()=>trace(x=>[...x,'stop']);},[]);
  useEvent('increment',()=>setCount(x=>x+1));
  useEvent('observe',()=>report(count));
  return <aff label="child" onRun={()=>count}>child{count}</aff>;
}
export default function Main() {
  const [hidden,setHidden]=useState(false);
  const [mounted,setMounted]=useState(true);
  const [label,setLabel]=useState('first');
  const [revision,setRevision]=useState(0);
  const [seen,setSeen]=useState(null,'seen');
  const [trace,setTrace]=useState([],'trace');
  useEvent('hide',()=>setHidden(x=>!x));
  useEvent('rename',()=>setLabel('other'));
  useEvent('rekey',()=>setRevision(x=>x+1));
  useEvent('remove',()=>setMounted(false));
  useEvent('mount',()=>setMounted(true));
  return <box max_tokens={0} style={{display:hidden?'none':'contents'}}>
    {mounted && <Child key={revision} label={label} report={setSeen} trace={setTrace}/>}
  </box>;
}"#;
    let (mut rig, _) = Rig::new(Some(source), json!({}));
    assert_eq!(rig.agent.state()["__ui"]["@trace"], json!(["start"]));
    rig.event("increment");
    rig.event("hide");
    rig.event("increment");
    rig.event("rename");
    rig.event("observe");
    assert_eq!(rig.agent.state()["__ui"]["@seen"], 2);
    assert_eq!(rig.agent.state()["__ui"]["@trace"], json!(["start"]));
    rig.check_restore();
    rig.event("rekey");
    rig.event("observe");
    assert_eq!(rig.agent.state()["__ui"]["@seen"], 0);
    assert_eq!(rig.agent.state()["__ui"]["@trace"], json!(["start", "stop", "start"]));
    rig.event("increment");
    rig.event("remove");
    rig.event("mount");
    rig.event("observe");
    assert_eq!(rig.agent.state()["__ui"]["@seen"], 0);
    assert_eq!(
        rig.agent.state()["__ui"]["@trace"],
        json!(["start", "stop", "start", "stop", "start"])
    );
    rig.check_restore();
}

#[test]
fn child_instances_and_callbacks_survive_hidden_and_cropped_presentation() {
    let source = r#"
export function Worker(){return 'worker';}
export default function Main(){
  const [hidden,setHidden]=useState(false);
  const [mounted,setMounted]=useState(true);
  const [seen,setSeen]=useState(null,'seen');
  useEvent('hide',()=>setHidden(true));useEvent('remove',()=>setMounted(false));
  return <box max_tokens={0} style={{display:hidden?'none':'contents'}}>
    {mounted && <Mount key="worker"><Worker onDone={value=>setSeen(value)}/></Mount>}
  </box>;
}"#;
    let (mut rig, start) = Rig::new(Some(source), json!({}));
    assert!(
        start
            .iter()
            .any(|c| matches!(c,Command::Mount {key,..} if key=="worker"))
    );
    assert!(!rig.event("hide").iter().any(|c| matches!(c, Command::Unmount { .. })));
    rig.input(Input::Event {
        name: "mount:worker:onDone".into(),
        payload: json!({"$args":["still mounted"]}),
        id: None,
        clock_ms: 2,
    });
    assert_eq!(rig.agent.state()["__ui"]["@seen"], "still mounted");
    rig.check_restore();
    assert!(
        rig.event("remove")
            .iter()
            .any(|c| matches!(c,Command::Unmount {key} if key=="worker"))
    );
    rig.check_restore();
}

/// A page that fails to render rejects its `model()` call, as `fetch` rejects: the app catches it
/// and goes on (the agent failed, on every resume too, since replay throws again).
#[test]
fn a_page_that_fails_to_render_rejects_its_model_call() {
    let source = r#"
import {GLM53App} from '@ROOT@/views/template_apps/glm53.tsx';
import {TurnStrategy} from '@ROOT@/views/strategies/turn_strategy.tsx';
function History(){ throw new TypeError('unexpected history shape'); }
export default function Main(){
  const [broken,setBroken]=useState(false,'broken');
  const [caught,setCaught]=useState(null,'caught');
  useEvent('call',async()=>{
    setBroken(true);
    try {await model();} catch(e){setCaught([e.name,e.message]);}
    setBroken(false);
    await model();
  });
  return <GLM53App><TurnStrategy strategy="unrolled" log={[]}>{broken ? <History/> : 'ok'}</TurnStrategy></GLM53App>;
}"#;
    let (mut rig, _) = Rig::new(Some(source), json!({}));
    let commands = rig.event("call");
    assert_eq!(
        rig.agent.state()["__ui"]["@caught"],
        json!(["TypeError", "unexpected history shape"])
    );
    assert!(
        commands.iter().any(|c| matches!(c, Command::Generate { .. })),
        "the next call is sent"
    );
    rig.check_restore();
}

#[test]
fn projected_scroll_uses_the_captured_viewport_and_restores_its_position() {
    let source = r#"
import {GLM53App} from '@ROOT@/views/template_apps/glm53.tsx';
import {TurnStrategy} from '@ROOT@/views/strategies/turn_strategy.tsx';
export default function Main(){
  const [observed,setObserved]=useState(null,'observed');
  useEvent('read',async()=>{const output=await model();await dispatch(output);});
  return <GLM53App><TurnStrategy strategy="unrolled" log={[]}><box id="viewport" style={{maxTokens:8,overflow:'scroll',more:'...'}}
    onScroll={event=>setObserved(event)}>{'one two three four five six seven eight nine ten eleven twelve'}</box></TurnStrategy></GLM53App>;
}"#;
    let (mut rig, _) = Rig::new(Some(source), json!({}));
    let mut prompts = vec![];
    for by in [3, 10000, -10000] {
        let commands = rig.event("read");
        let (task, ids) = commands
            .iter()
            .find_map(|c| match c {
                Command::Generate { task, ids, .. } => Some((*task, ids.clone())),
                _ => None,
            })
            .unwrap();
        prompts.push(ids.to_vec());
        let text =
            format!("</think><tool_call>scroll__viewport<arg_key>by</arg_key><arg_value>{by}</arg_value></tool_call>");
        let mut reply = rig.enc.tok.encode(text, false).unwrap().get_ids().to_vec();
        reply.push(rig.enc.token_id("<|observation|>").unwrap());
        let clock = rig.clock + 1;
        rig.input(Input::Reply {
            server: None,
            task,
            ids: reply,
            cut_off: false,
            clock_ms: clock,
            prompt: None,
            aborted: false,
        });
        let observed = &rig.agent.state()["__ui"]["@observed"];
        if by == 3 {
            assert_eq!(observed, &json!({"by":3,"offset":3}));
        }
        if by == 10000 {
            assert!(observed["offset"].as_i64().unwrap() > 3);
            assert!(observed["by"].as_i64().unwrap() < 10000);
        }
        if by < 0 {
            assert_eq!(observed["offset"], 0);
        }
        rig.check_restore();
    }
    assert_ne!(prompts[0], prompts[1]);
    assert_eq!(rig.agent.metrics.handler_errors, 0);
}

#[test]
fn failed_element_construction_does_not_publish_lifecycle_or_reserve_its_key() {
    let source = r#"
function Broken({report}) {
  useState(0);
  useEffect(()=>report('leaked effect'),[]);
  useEvent('leaked',()=>report('leaked listener'));
  throw new Error('bad child');
}
function Fallback(){return 'fallback';}
export default function Main(){
  const [seen,setSeen]=useState('parent','seen');
  let child;
  try {child=<Broken key="child" report={setSeen}/>;}
  catch(error){child=<Fallback key="child"/>;}
  return child;
}"#;
    let (mut rig, _) = Rig::new(Some(source), json!({}));
    rig.event("leaked");
    assert_eq!(rig.agent.state()["__ui"]["@seen"], "parent");
    rig.check_restore();
}

/// `Date.now()` in a handler is the journal's clock, so a replay reads the same time, and
/// `console.log` writes to the instance's trace without touching its state.
#[test]
fn date_now_is_the_journals_clock_and_the_console_is_the_trace() {
    let source = r#"
    export default function Main() {
      const [seen, setSeen] = useState(null, 'seen');
      useEvent('go', () => {
        const at = Date.now();
        console.log('went at', at, {ok: true});
        console.error('and this is an error line');
        setSeen(at);
      });
      return '';
    }"#;
    let (mut rig, _) = Rig::new(Some(source), json!({}));
    rig.agent.take_trace();
    rig.clock = 41_000;
    rig.event("go");
    assert_eq!(rig.agent.state()["__ui"]["@seen"], 41_001);
    let console: Vec<J> = rig
        .agent
        .take_trace()
        .into_iter()
        .filter(|t| t["t"] == "console")
        .map(|t| json!([t["level"], t["text"]]))
        .collect();
    assert_eq!(
        console,
        vec![
            json!(["log", r#"went at 41001 {"ok":true}"#]),
            json!(["error", "and this is an error line"])
        ]
    );
    rig.check_restore();
}

#[test]
fn a_rejection_nothing_awaits_is_reported() {
    let source = r#"
    export default function Main(){
      const work=async()=>{throw new Error('boom');};
      // Awaited and caught, or caught with .catch: read, so not reported.
      useEvent('fine',async()=>{try{await work();}catch(e){} work().catch(()=>{}); await Promise.allSettled([work()]);});
      // Called and not awaited: nothing reads the rejection.
      useEvent('lost',()=>{work();});
      // The same after a wait on the host: the rejection comes in a later step.
      useEvent('later',()=>{(async()=>{await harness.machine.call({q:'x'});throw new Error('late');})();});
      return '';
    }"#;
    let (mut rig, _) = Rig::new(Some(source), json!({}));
    rig.event("fine");
    let trace = rig.agent.take_trace();
    assert_eq!(rig.agent.metrics.handler_errors, 0, "{trace:?}");
    rig.event("lost");
    assert_eq!(rig.agent.metrics.handler_errors, 1);
    let trace = rig.agent.take_trace();
    let lost = trace.iter().find(|e| e["t"] == "handler_error").unwrap();
    assert_eq!(lost["origin"], "callback lost");
    assert_eq!(lost["unhandled_rejection"], true);
    assert!(lost["error"].as_str().unwrap().contains("boom"), "{lost}");
    let commands = rig.event("later");
    let Some(Command::Call { task, .. }) = commands.iter().find(|c| matches!(c, Command::Call { .. })) else {
        panic!("{commands:?}")
    };
    rig.clock += 1;
    rig.input(Input::Result {
        task: *task,
        outcome: Outcome::Ok(json!({})),
        clock_ms: rig.clock,
    });
    assert_eq!(rig.agent.metrics.handler_errors, 2);
    let trace = rig.agent.take_trace();
    assert!(
        trace
            .iter()
            .any(|e| e["t"] == "handler_error" && e["error"].as_str().unwrap().contains("late")),
        "{trace:?}"
    );
    let (restored, _) = Agent::restore(rig.config.clone(), rig.journal.clone(), &rig.view, &rig.enc).unwrap();
    assert_eq!(restored.metrics.handler_errors, 2);
}

#[test]
fn what_the_server_reported_reaches_the_app_and_replays_from_the_journal() {
    // A model call's output carries the journaled `server` record (cached tokens, timings, the
    // reply's wall-clock arrival), so an app can render from it and a replay sees the same.
    let source = r#"import {GLM53App} from '@ROOT@/views/template_apps/glm53.tsx';
import {TurnStrategy} from '@ROOT@/views/strategies/turn_strategy.tsx';
export default function Main(){const [seen,setSeen]=useState(null,'seen');
useEffect(()=>{(async()=>{const out=await model();setSeen(out.server);})();},[]);
return <GLM53App><TurnStrategy strategy='unrolled' log={[]}>hello</TurnStrategy></GLM53App>;}"#;
    let (mut rig, commands) = Rig::new(Some(source), json!({}));
    let Command::Generate { task, .. } = commands.iter().find(|c| matches!(c, Command::Generate { .. })).unwrap()
    else {
        unreachable!()
    };
    let mut ids = rig.enc.tok.encode("</think>done", false).unwrap().get_ids().to_vec();
    ids.push(rig.enc.token_id("<|user|>").unwrap());
    let server = json!({"cached_tokens": 12, "ms": 340.5, "first_token_ms": 80.25, "wall_ms": 1794700000000u64});
    rig.input(Input::Reply {
        server: Some(server.clone()),
        task: *task,
        ids,
        cut_off: false,
        clock_ms: 1,
        prompt: None,
        aborted: false,
    });
    assert_eq!(rig.agent.state()["__ui"]["@seen"], server);
    rig.check_restore();
    // A journal written before the field existed still loads; its outputs carry `server: null`.
    let old: Input =
        serde_json::from_value(json!({"input": "reply", "task": 0, "ids": [1], "cut_off": false, "clock_ms": 1}))
            .unwrap();
    assert!(matches!(old, Input::Reply { server: None, .. }));
}
