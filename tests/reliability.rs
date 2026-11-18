//! Regression cases for the runtime's boundaries and reliability.
use crate::tokenizer;
use affordance::{
    View,
    emit::{Encoder, RunCache},
    runtime::{AgentSpec, FnModel, GenReply, GenRequest, Runtime, RuntimeConfig, store::Store},
};
use serde_json::{Value as J, json};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "aff-reliability-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn app(&self, source: &str) -> PathBuf {
        let p = self.0.join("app.tsx");
        std::fs::write(&p, source).unwrap();
        p
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn encoder() -> Arc<Encoder> {
    Arc::new(tokenizer::encoder())
}

fn config(app: &std::path::Path) -> RuntimeConfig {
    let mut c = RuntimeConfig::new(app);
    c.workers = 1;
    c.model_threads = 1;
    c.exec_threads = 1;
    c.max_wall = Some(Duration::from_secs(5));
    // The apps are written to a temp directory and import the repository's views.
    c.library = Some(env!("CARGO_MANIFEST_DIR").into());
    c
}
fn model() -> Arc<dyn affordance::runtime::Model> {
    Arc::new(FnModel(|_: &GenRequest| Ok(GenReply::default())))
}
fn render(source: &str) -> String {
    let dir = Scratch::new();
    let app = dir.app(source);
    let enc = encoder();
    let v = View::load(&app, "Main", &enc).unwrap();
    let r = v
        .render(
            &enc,
            Default::default(),
            &mut RunCache::new(false),
            "2026-10-23T00:00:00".parse().unwrap(),
        )
        .unwrap();
    enc.tok.decode(&r.ids, false).unwrap()
}

#[test]
fn log_release_reads_current_state_and_commits_only_persisted_prefixes() {
    let dir = Scratch::new();
    let app = dir.app(&format!(
        r#"
import {{useLog,recordText}} from '{}/views/lib/log.tsx';
export default function Main({{denied,reverse,name}}){{
  const log=useLog(name);
  useEffect(()=>{{(async()=>{{
    log.append({{kind:'input',content:'first',id:'forged'}});
    let failed=false, boundary=null;
    try {{boundary=await log.release(100);}} catch(e) {{failed=true;}}
    if(!denied){{
      log.append({{kind:'input',content:'second'}},{{kind:'input',content:'third'}});
      await Promise.all(reverse ? [log.release(3),log.release(2)] : [log.release(2),log.release(3)]);
    }}
    const records=await log.read(0,3);
    const first=await log.get(records[0].id);
    harness.stop({{failed,boundary,records,first:recordText(first),missing:await log.get('e9007199254740991')}});
  }})();}},[]);
  return 'history';
}}
"#,
        env!("CARGO_MANIFEST_DIR")
    ));
    let mut cfg = config(&app);
    cfg.dir = Some(dir.0.join("run"));
    let run = |resume| {
        let mut rt = if resume {
            Runtime::resume(cfg.clone(), encoder(), model()).unwrap()
        } else {
            Runtime::new(cfg.clone(), encoder(), model())
        };
        if !resume {
            for (id, denied, reverse) in [
                ("saved", false, false),
                ("reversed", false, true),
                ("named", false, false),
                ("denied", true, false),
            ] {
                let mut spec = AgentSpec::new(
                    id,
                    json!({"denied":denied,"reverse":reverse,"name":if id=="named" {"archive"} else {"log"}}),
                );
                spec.model = None;
                if denied {
                    spec.capabilities = Some(vec![]);
                }
                rt.add_agent(spec).unwrap();
            }
        }
        rt.run().unwrap()
    };
    let report = run(false);
    for agent in &report.agents {
        assert!(agent.error.is_none(), "{:?}", agent.error);
        let denied = agent.id == "denied";
        let records = if denied {
            vec!["first"]
        } else {
            vec!["first", "second", "third"]
        };
        assert_eq!(
            agent.stopped,
            Some(
                json!({"failed":denied,"boundary":if denied {J::Null} else {json!(1)}, "first":"first", "missing":null,
            "records":records.into_iter().enumerate().map(|(i,content)|json!({"kind":"input","content":content,"id":format!("{}e{i}",if agent.id=="named" {"archive:"} else {""})})).collect::<Vec<_>>()})
            )
        );
        // Exercise complete journal replay as well as the normal cache-based stop path.
        let _ = std::fs::remove_file(dir.0.join("run").join(&agent.id).join("checkpoint.json"));
    }
    let resumed = run(true);
    for (before, after) in report.agents.iter().zip(&resumed.agents) {
        assert!(after.error.is_none(), "{:?}", after.error);
        assert_eq!(after.stopped, before.stopped);
        assert_eq!(after.state, before.state);
    }
}

#[test]
fn busy_local_calls_allow_timers_inspection_pause_and_shutdown() {
    const CHILD: &str = "AFFORDANCE_FAIR_WORKER_CHILD";
    if std::env::var_os(CHILD).is_none() {
        // The scenario runs in a child with a 15 s limit: a runtime that cannot shut down would
        // otherwise hang the test.
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "reliability::busy_local_calls_allow_timers_inspection_pause_and_shutdown",
                "--test-threads=1",
            ])
            .env(CHILD, "1")
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(15);
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break Some(status);
            }
            if std::time::Instant::now() > deadline {
                let _ = child.kill();
                break None;
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        assert!(
            status.is_some_and(|s| s.success()),
            "busy work must not prevent runtime shutdown"
        );
        return;
    }
    for pause in [false, true] {
        let dir = Scratch::new();
        let app = dir.app(
            r#"
export default function Main(){
  const [n,setN]=useState(0,'n');
  useEffect(()=>{(async()=>{
    if(harness.agent==='busy'){
      for(;;){await harness.store.put({key:'same',text:'value'});setN(x=>x+1);}
    }else{
      await harness.wait({ms:0});
      const seen=await harness.inspect('busy',{select:['n']});
      await harness.machine.call({});
      harness.stop(seen.agents.busy.version);
    }
  })();},[]);
  return String(n);
}"#,
        );
        let mut cfg = config(&app);
        cfg.dir = Some(dir.0.join("run"));
        cfg.max_wall = if pause { None } else { Some(Duration::from_secs(1)) };
        let control = dir.0.join("run/control");
        std::fs::create_dir_all(&control).unwrap();
        let mut rt = Runtime::new(cfg.clone(), encoder(), model());
        rt.bind_machine(
            "control",
            Arc::new(move |_: &affordance::runtime::CallContext, _: &J| {
                if pause {
                    std::fs::write(control.join("pause.json"), r#"{"mode":"immediate"}"#).unwrap();
                }
                Ok(json!({}))
            }),
        );
        for id in ["busy", "watcher"] {
            let mut spec = AgentSpec::new(id, json!({}));
            spec.model = None;
            if id == "watcher" {
                spec.inspect = vec!["busy".into()];
            }
            rt.add_agent(spec).unwrap();
        }
        let report = rt.run().unwrap();
        assert!(!report.complete);
        let watcher = report.agents.iter().find(|a| a.id == "watcher").unwrap();
        assert!(watcher.error.is_none(), "{:?}", watcher.error);
        assert!(watcher.stopped.as_ref().and_then(J::as_u64).is_some_and(|n| n > 0));
        if pause {
            let before = report.agents.iter().find(|a| a.id == "busy").unwrap();
            let pause_file = dir.0.join("run/control/pause.json");
            std::fs::remove_file(&pause_file).unwrap();
            // The resumed run ends as soon as the busy agent's journal has grown (a graceful
            // pause from the controller); the wall limit only bounds a failure, so a slow
            // restore under load cannot end the run before the held work continues.
            cfg.max_wall = Some(Duration::from_secs(2));
            let journal = dir.0.join("run/busy/journal.jsonl");
            let lines = |p: &std::path::Path| std::fs::read(p).map_or(0, |b| b.iter().filter(|&&c| c == b'\n').count());
            let start = lines(&journal).max(before.journal_len);
            let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let watch = {
                let done = done.clone();
                std::thread::spawn(move || {
                    while !done.load(Ordering::SeqCst) {
                        if lines(&journal) > start {
                            std::fs::write(&pause_file, r#"{"mode":"graceful"}"#).unwrap();
                            return;
                        }
                        std::thread::sleep(Duration::from_millis(5));
                    }
                })
            };
            let mut rt = Runtime::resume(cfg, encoder(), model()).unwrap();
            rt.bind_machine(
                "control",
                Arc::new(|_: &affordance::runtime::CallContext, _: &J| Ok(json!({}))),
            );
            let resumed = rt.run().unwrap();
            done.store(true, Ordering::SeqCst);
            watch.join().unwrap();
            let after = resumed.agents.iter().find(|a| a.id == "busy").unwrap();
            assert!(after.error.is_none(), "{:?}", after.error);
            assert!(
                after.journal_len > before.journal_len,
                "held work must continue after recovery"
            );
        }
    }
}

#[test]
fn child_keys_cannot_alias_persisted_instance_paths() {
    for key in [".", "..", "a/b", "a\\b"] {
        let dir = Scratch::new();
        let app=dir.app(&format!("function Kid(){{return 'child';}} export default function Main(){{return <Mount key={{{}}}><Kid/></Mount>;}}",json!(key)));
        let mut cfg = config(&app);
        cfg.dir = Some(dir.0.join("run"));
        let mut rt = Runtime::new(cfg, encoder(), model());
        rt.add_agent(AgentSpec::new("parent", json!({}))).unwrap();
        let report = rt.run().unwrap();
        assert!(report.agents[0].error.as_deref().unwrap().contains("key"));
        let saved: J = serde_json::from_slice(&std::fs::read(dir.0.join("run/parent/agent.json")).unwrap()).unwrap();
        assert_eq!(saved["id"], "parent");
        assert_eq!(report.agents.len(), 1);
    }
}

#[test]
fn a_remount_is_fresh_and_a_restart_restores_that_incarnation() {
    let dir = Scratch::new();
    let app = dir.app(
        r#"
function Kid({initial}) { const [n,setN]=useState(initial,'n'); useEffect(()=>{setN(initial+1)},[]); return String(n); }
export default function Main() { const [phase,setPhase]=useState(0,'phase'); useEffect(()=>{(async()=>{
 await harness.wait({ms:30});setPhase(1);await harness.wait({ms:30});setPhase(2);
})()},[]);return phase===1 ? 'off' : <Mount key="kid"><Kid initial={phase===0?10:100}/></Mount>; }
"#,
    );
    let mut cfg = config(&app);
    cfg.dir = Some(dir.0.join("run"));
    let enc = encoder();
    let mut rt = Runtime::new(cfg.clone(), enc.clone(), model());
    rt.add_agent(AgentSpec::new("a", json!({}))).unwrap();
    for report in [
        rt.run().unwrap(),
        Runtime::resume(cfg, enc, model()).unwrap().run().unwrap(),
    ] {
        let kid = report.agents.iter().find(|a| a.id == "a/kid").unwrap();
        assert!(kid.error.is_none(), "{:?}", kid.error);
        assert_eq!(kid.state["__ui"]["@n"], 101);
    }
    assert!(dir.0.join("run/.retired").is_dir());
}

#[test]
fn old_model_results_cannot_resolve_a_new_childs_task() {
    let dir = Scratch::new();
    let app=dir.app(r#"
function Kid() { const [value,setValue]=useState('waiting','value'); useEffect(()=>{(async()=>{const out=await model();setValue(out.text);})()},[]);return <><span label="protocol" syntax="hermes"/>{value}</>; }
export default function Main(){const [phase,setPhase]=useState(0,'phase');useEffect(()=>{(async()=>{
 await harness.wait({ms:50});setPhase(1);await harness.wait({ms:20});setPhase(2);
})()},[]);return phase===1?'off':<Mount key="kid"><Kid/></Mount>;}
"#);
    let enc = encoder();
    let output_enc = enc.clone();
    let count = Arc::new(AtomicUsize::new(0));
    let calls = count.clone();
    let model = Arc::new(FnModel(move |_: &GenRequest| {
        let first = count.fetch_add(1, Ordering::SeqCst) == 0;
        if first {
            std::thread::sleep(Duration::from_millis(250));
        }
        Ok(GenReply {
            ids: output_enc
                .tok
                .encode(if first { "old" } else { "new" }, false)
                .unwrap()
                .get_ids()
                .to_vec(),
            ..Default::default()
        })
    }));
    let mut cfg = config(&app);
    cfg.model_threads = 2;
    let mut rt = Runtime::new(cfg, enc, model);
    rt.add_agent(AgentSpec::new("a", json!({}))).unwrap();
    let report = rt.run().unwrap();
    let kid = report.agents.iter().find(|a| a.id == "a/kid").unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(kid.state["__ui"]["@value"], "new");
}

#[test]
fn state_updates_preserve_object_order_and_class_identity() {
    let dir = Scratch::new();
    let app = dir.app(
        r#"
class A { constructor(){this.x=1;} } class B { constructor(){this.x=1;} }
export default function Main(){const [o,setO]=useState({a:1,b:2},'o');const [v,setV]=useState(new A(),'v');
 const [kind,setKind]=useState(false,'kind');useEffect(()=>{setO({b:2,a:1});setV(new B());},[]);
 useEffect(()=>{setKind(v instanceof B);},[v]);return Object.keys(o).join(',');}
"#,
    );
    let mut rt = Runtime::new(config(&app), encoder(), model());
    rt.add_agent(AgentSpec::new("a", json!({}))).unwrap();
    let report = rt.run().unwrap();
    let state = &report.agents[0].state["__ui"];
    assert_eq!(
        state["@o"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["b", "a"]
    );
    assert_eq!(state["@kind"], true);
}

#[test]
fn nullable_literal_types_accept_null_in_validation() {
    let schema: J = serde_json::from_str(&render(
        "function f({unit}:{unit:'c'|'f'|null}){} export default function Main(){return JSON.stringify(schemaOf(f));}",
    ))
    .unwrap();
    affordance::reply::validate(&schema, &json!({"unit":null})).unwrap();
}

#[test]
fn source_identity_includes_file_roles_but_is_relocation_stable() {
    let a = Scratch::new();
    let b = Scratch::new();
    let enc = encoder();
    let setup = |dir: &Scratch| {
        dir.app("import './a.tsx';import {helper} from './b.tsx';export default function Main(){return helper();}");
        std::fs::write(dir.0.join("a.tsx"), "export function helper(){return 'A';}").unwrap();
        std::fs::write(dir.0.join("b.tsx"), "export function helper(){return 'B';}").unwrap();
    };
    setup(&a);
    setup(&b);
    let first = View::load(&a.0.join("app.tsx"), "Main", &enc).unwrap().source_hash;
    assert_eq!(
        first,
        View::load(&b.0.join("app.tsx"), "Main", &enc).unwrap().source_hash
    );
    let x = std::fs::read(a.0.join("a.tsx")).unwrap();
    let y = std::fs::read(a.0.join("b.tsx")).unwrap();
    std::fs::write(a.0.join("a.tsx"), y).unwrap();
    std::fs::write(a.0.join("b.tsx"), x).unwrap();
    assert_ne!(
        first,
        View::load(&a.0.join("app.tsx"), "Main", &enc).unwrap().source_hash
    );
}

#[test]
fn store_recovers_partial_utf8_tail_and_preserves_committed_records() {
    let dir = Scratch::new();
    let store = Store::new(Some(dir.0.clone()));
    store.call("a", "store.put", &json!({"text":"saved"})).unwrap();
    let file = dir.0.join("a/store.jsonl");
    use std::io::Write;
    std::fs::OpenOptions::new()
        .append(true)
        .open(&file)
        .unwrap()
        .write_all(b"{\"key\":\"b2\",\"text\":\"\xf0\x9f")
        .unwrap();
    let recovered = Store::new(Some(dir.0.clone()));
    recovered.load("a").unwrap();
    assert_eq!(
        recovered.call("a", "store.get", &json!({"key":"b1"})).unwrap()["text"],
        "saved"
    );
    assert_eq!(
        recovered.call("a", "store.put", &json!({"text":"next"})).unwrap()["key"],
        "b2"
    );
    let again = Store::new(Some(dir.0.clone()));
    again.load("a").unwrap();
    assert_eq!(
        again.call("a", "store.get", &json!({"key":"b2"})).unwrap()["text"],
        "next"
    );
    assert!(
        std::fs::read_dir(dir.0.join("a"))
            .unwrap()
            .any(|e| { e.unwrap().file_name().to_string_lossy().starts_with("torn-store-") })
    );
}

#[test]
fn checkpoints_resume_parent_and_child_suffixes_and_keep_the_complete_history() {
    use std::io::Write;
    let dir = Scratch::new();
    let app = dir.app(
        r#"
function Child({label}) {
  const [n,setN]=useState(10,'n');useEffect(()=>setN(n=>n+1),[]);
  useEvent('bump',()=>setN(n=>n+1));return label+n;
}
export default function Main(){
  const [label,setLabel]=useState('first','label');const [runs,setRuns]=useState(0,'runs');
  const [initial]=useState(harness.env.DEMO,'initial');
  useEffect(()=>setRuns(n=>n+1),[]);
  useEvent('advance',({label})=>setLabel(label));
  return <Mount key='kid'><Child label={label}/></Mount>;
}"#,
    );
    let run = dir.0.join("run");
    let mut cfg = config(&app);
    cfg.dir = Some(run.clone());
    cfg.env.insert("DEMO".into(), "left".into());
    let enc = encoder();
    let mut rt = Runtime::new(cfg.clone(), enc.clone(), model());
    rt.add_agent(AgentSpec::new("a", json!({}))).unwrap();
    let first = rt.run().unwrap();
    assert!(first.agents.iter().all(|a| a.error.is_none()));
    let read = |path: &std::path::Path| -> J { serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap() };
    let append = |id: &str, name: &str, payload: J, clock: i64| {
        let input = affordance::runtime::Input::Event {
            name: name.into(),
            payload,
            op: None,
            clock_ms: clock,
        };
        writeln!(
            std::fs::OpenOptions::new()
                .append(true)
                .open(run.join(id).join("journal.jsonl"))
                .unwrap(),
            "{}",
            serde_json::to_string(&input).unwrap()
        )
        .unwrap();
    };
    let checkpoint = run.join("a/checkpoint.json");
    let child_checkpoint = run.join("a/kid/checkpoint.json");
    assert!(checkpoint.exists() && child_checkpoint.exists());
    let clock = read(&run.join("clock.json"))["clock_ms"].as_i64().unwrap() + 10;
    append("a", "advance", json!({"label":"second"}), clock);
    append("a/kid", "bump", json!({}), clock + 1);
    let second = Runtime::resume(cfg.clone(), enc.clone(), model())
        .unwrap()
        .run()
        .unwrap();
    assert!(second.agents.iter().all(|a| a.error.is_none()), "{second:?}");
    assert_eq!(
        second.agents.iter().find(|a| a.id == "a/kid").unwrap().state["__ui"]["@n"],
        12
    );
    let saved = read(&checkpoint);
    let child_saved = read(&child_checkpoint);
    let from = saved["data"]["image"]["journaled"].as_u64().unwrap();
    let child_from = child_saved["data"]["image"]["journaled"].as_u64().unwrap();
    assert!(from > 0 && child_from > 0);
    let root_prefix = std::fs::read(run.join("a/journal.jsonl")).unwrap();
    let child_prefix = std::fs::read(run.join("a/kid/journal.jsonl")).unwrap();
    let clock = read(&run.join("clock.json"))["clock_ms"].as_i64().unwrap() + 10;
    append("a", "advance", json!({"label":"third"}), clock);
    append("a/kid", "bump", json!({}), clock + 1);
    let child_journal = run.join("a/kid/journal.jsonl");
    let boundary = std::fs::metadata(&child_journal).unwrap().len();
    std::fs::OpenOptions::new()
        .append(true)
        .open(&child_journal)
        .unwrap()
        .write_all(b"{\"input\":\"event\",\"name\":\"bad\",\"payload\":\"\xf0\x9f")
        .unwrap();
    let third = Runtime::resume(cfg.clone(), enc.clone(), model())
        .unwrap()
        .run()
        .unwrap();
    assert!(third.agents.iter().all(|a| a.error.is_none()), "{third:?}");
    let parent = third.agents.iter().find(|a| a.id == "a").unwrap();
    let child = third.agents.iter().find(|a| a.id == "a/kid").unwrap();
    assert_eq!(parent.state["__ui"]["@label"], "third");
    assert_eq!(parent.state["__ui"]["@runs"], 1);
    assert_eq!(child.state["__ui"]["@n"], 13);
    assert_eq!(child.state["label"], "third");
    for (id, from, prefix) in [("a", from, root_prefix), ("a/kid", child_from, child_prefix)] {
        let records = std::fs::read_to_string(run.join(id).join("trace.jsonl"))
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str::<J>(line).unwrap())
            .collect::<Vec<_>>();
        let restored = records.iter().rev().find(|v| v["t"] == "checkpoint_restore").unwrap();
        assert_eq!(restored["from_input"], from);
        assert_eq!(restored["replayed_inputs"], 1);
        assert!(
            std::fs::read(run.join(id).join("journal.jsonl"))
                .unwrap()
                .starts_with(&prefix)
        );
    }
    assert!(run.join("a/kid").join(format!("torn-journal-{boundary}.bin")).exists());
    // A valid edit of the same byte length must invalidate the certified prefix.
    let root_journal = run.join("a/journal.jsonl");
    let before = std::fs::read_to_string(&root_journal).unwrap();
    let edited = before.replace("\"label\":\"third\"", "\"label\":\"other\"");
    assert_ne!(before, edited);
    assert_eq!(before.len(), edited.len());
    std::fs::write(&root_journal, edited).unwrap();
    let edited = Runtime::resume(cfg.clone(), enc.clone(), model())
        .unwrap()
        .run()
        .unwrap();
    let expected = edited.agents.iter().find(|a| a.id == "a").unwrap().state.clone();
    assert_eq!(expected["__ui"]["@label"], "other");
    // A broken cache is disposable; recovery still has the entire original journal.
    std::fs::write(&checkpoint, b"{broken checkpoint").unwrap();
    let restored = Runtime::resume(cfg.clone(), enc.clone(), model())
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(restored.agents.iter().find(|a| a.id == "a").unwrap().state, expected);
    // Configuration changes must rerun initialization under the requested environment.
    cfg.env.insert("DEMO".into(), "right".into());
    let changed = Runtime::resume(cfg, enc, model()).unwrap().run().unwrap();
    assert!(changed.agents.iter().all(|a| a.error.is_none()), "{changed:?}");
    assert_eq!(
        changed.agents.iter().find(|a| a.id == "a").unwrap().state["__ui"]["@initial"],
        "right"
    );
}

/// A deploy installs the same app in a new release directory and repoints a symlink to it, and
/// the node changes its operational settings (pools, admission, restore turns, timeouts): the
/// images stay usable. A change to what the agent computes (the app's code, its environment, the
/// binding's template) still refuses them. (A release switch alone once made every agent of four
/// fleets replay its whole journal.)
#[test]
fn checkpoints_survive_a_moved_app_and_operational_changes_but_not_semantic_ones() {
    use std::io::Write;
    let dir = Scratch::new();
    let source = "export default function Main(){const [n,setN]=useState(0,'n');const [env]=useState(harness.env.DEMO,'env');\
        useEffect(()=>setN(n=>n+1),[]);useEvent('bump',()=>setN(n=>n+1));return env+n;}";
    for release in ["one", "two"] {
        let views = dir.0.join("releases").join(release).join("views");
        std::fs::create_dir_all(&views).unwrap();
        std::fs::write(views.join("app.tsx"), source).unwrap();
    }
    let current = dir.0.join("app");
    std::os::unix::fs::symlink(dir.0.join("releases/one"), &current).unwrap();
    let app = current.join("views/app.tsx");
    let run = dir.0.join("run");
    let mut cfg = config(&app);
    cfg.dir = Some(run.clone());
    cfg.env.insert("DEMO".into(), "left".into());
    let enc = encoder();
    let template = |options: J| {
        affordance::runtime::model::with_template(
            model(),
            affordance::runtime::model::ModelTemplate::new("glm53", options).unwrap(),
        )
        .unwrap()
    };
    let mut rt = Runtime::new(cfg.clone(), enc.clone(), template(json!({"reasoning_effort":"high"})));
    rt.add_agent(AgentSpec::new("a", json!({}))).unwrap();
    assert!(rt.run().unwrap().agents[0].error.is_none());
    assert!(run.join("a/checkpoint.json").exists());
    let restores = || {
        std::fs::read_to_string(run.join("a/trace.jsonl"))
            .unwrap()
            .lines()
            .filter(|l| serde_json::from_str::<J>(l).unwrap()["t"] == "checkpoint_restore")
            .count()
    };
    let mut bumps = 1;
    // One more input each time, so each resume has a suffix to replay after the image.
    let mut resume = |cfg: &RuntimeConfig, options: J| {
        let clock = serde_json::from_slice::<J>(&std::fs::read(run.join("clock.json")).unwrap()).unwrap()["clock_ms"]
            .as_i64()
            .unwrap()
            + 10;
        let input = affordance::runtime::Input::Event {
            name: "bump".into(),
            payload: json!({}),
            op: None,
            clock_ms: clock,
        };
        writeln!(
            std::fs::OpenOptions::new()
                .append(true)
                .open(run.join("a/journal.jsonl"))
                .unwrap(),
            "{}",
            serde_json::to_string(&input).unwrap()
        )
        .unwrap();
        bumps += 1;
        let report = Runtime::resume(cfg.clone(), enc.clone(), template(options))
            .unwrap()
            .run()
            .unwrap();
        assert!(report.agents[0].error.is_none(), "{report:?}");
        assert_eq!(report.agents[0].state["__ui"]["@n"], bumps);
        report.agents[0].state["__ui"]["@env"].clone()
    };
    let before = restores();
    // The release switch (the same files under another real path) and every operational knob.
    std::fs::remove_file(&current).unwrap();
    std::os::unix::fs::symlink(dir.0.join("releases/two"), &current).unwrap();
    let mut operational = cfg.clone();
    operational.workers = 3;
    operational.model_threads = 5;
    operational.exec_threads = 7;
    operational.model_inflight = Some(2);
    operational.restore_concurrency = Some(1);
    operational.call_timeout = Duration::from_secs(17);
    operational.max_wall = Some(Duration::from_secs(6));
    resume(&operational, json!({"reasoning_effort":"high"}));
    assert_eq!(
        restores(),
        before + 1,
        "a moved app and operational settings keep the image"
    );
    // Semantic changes each refuse the image (a full replay, no restore), then image again.
    let refused = |after: usize| assert_eq!(restores(), after, "a semantic change refuses the image");
    let n = restores();
    std::fs::write(
        dir.0.join("releases/two/views/app.tsx"),
        format!("{source}\n// changed code\n"),
    )
    .unwrap();
    resume(&operational, json!({"reasoning_effort":"high"}));
    refused(n);
    resume(&operational, json!({"reasoning_effort":"high"}));
    assert_eq!(restores(), n + 1);
    resume(&operational, json!({"reasoning_effort":"low"}));
    refused(n + 1);
    let mut environment = operational.clone();
    environment.env.insert("DEMO".into(), "right".into());
    assert_eq!(resume(&environment, json!({"reasoning_effort":"low"})), "right");
    refused(n + 1);
}

#[test]
fn recovery_caches_skip_secret_environments_and_live_source_values() {
    for source_values in [false, true] {
        let dir = Scratch::new();
        let app=dir.app(if source_values {
            "import {loadModule} from 'affordance:compiler';export default function Main(){const [m]=useState(()=>loadModule(\"export default function Panel(){return 'child';}\"));return <m.default/>;}"
        } else {"export default function Main(){return 'idle';}"});
        let mut cfg = config(&app);
        let run = dir.0.join("run");
        cfg.dir = Some(run.clone());
        if !source_values {
            cfg.env.insert("SECRET".into(), "pin-0042".into());
            cfg.secrets.insert("SECRET".into());
        }
        let enc = encoder();
        let mut rt = Runtime::new(cfg.clone(), enc.clone(), model());
        rt.add_agent(AgentSpec::new("a", json!({}))).unwrap();
        let first = rt.run().unwrap();
        assert!(first.agents[0].error.is_none());
        assert!(!run.join("a/checkpoint.json").exists());
        let second = Runtime::resume(cfg, enc, model()).unwrap().run().unwrap();
        assert!(second.agents[0].error.is_none());
        assert_eq!(first.agents[0].state, second.agents[0].state);
    }
}

#[test]
fn corrupt_stores_fail_only_their_instance_and_fresh_starts_archive_them() {
    let dir = Scratch::new();
    let app=dir.app("export default function Main(){const [n,setN]=useState(0,'n');useEffect(()=>setN(n=>n+1),[]);return String(n);}");
    let mut cfg = config(&app);
    let run = dir.0.join("run");
    cfg.dir = Some(run.clone());
    let enc = encoder();
    let fresh = |cfg: RuntimeConfig| {
        let mut rt = Runtime::new(cfg, enc.clone(), model());
        for id in ["a", "b"] {
            rt.add_agent(AgentSpec::new(id, json!({}))).unwrap();
        }
        rt.run().unwrap()
    };
    fresh(cfg.clone());
    let path = run.join("a/store.jsonl");
    std::fs::write(&path, b"{broken record}\n").unwrap();
    let resumed = Runtime::resume(cfg.clone(), enc.clone(), model())
        .unwrap()
        .run()
        .unwrap();
    assert!(
        resumed
            .agents
            .iter()
            .find(|a| a.id == "a")
            .unwrap()
            .error
            .as_deref()
            .unwrap()
            .contains("store recovery failed")
    );
    let b = resumed.agents.iter().find(|a| a.id == "b").unwrap();
    assert!(b.error.is_none());
    assert_eq!(b.state["__ui"]["@n"], 1);
    assert_eq!(std::fs::read(&path).unwrap(), b"{broken record}\n");
    let restarted = fresh(cfg);
    assert!(restarted.agents.iter().all(|a| a.error.is_none()));
    assert!(!path.exists());
    assert!(run.join(".retired").is_dir());
}

#[test]
fn late_messages_after_stop_do_not_force_a_checkpoint_for_every_input() {
    let dir = Scratch::new();
    let app = dir.app(
        r#"
export function Halt(){useEffect(()=>harness.stop('done'),[]);return 'stopped';}
export function Sender(){const [seen,setSeen]=useState([],'seen');
  useEffect(()=>{(async()=>{for(let n=0;n<3;n++){
    await harness.send('halt',{text:'late'});
    const out=await harness.machine.call({cmd:'checkpoint'});setSeen(v=>[...v,out.inputs]);
  }})();},[]);return String(seen.length);}
"#,
    );
    let mut cfg = config(&app);
    let run = dir.0.join("run");
    cfg.dir = Some(run.clone());
    let enc = encoder();
    let mut rt = Runtime::new(cfg, enc, model());
    let checkpoint = run.join("halt/checkpoint.json");
    let observed = checkpoint.clone();
    rt.bind_machine(
        "home",
        Arc::new(move |_: &affordance::runtime::CallContext, _: &J| {
            let image: J = serde_json::from_slice(&std::fs::read(&observed).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            Ok(json!({"inputs":image["data"]["image"]["journaled"]}))
        }),
    );
    for (id, entry) in [("halt", "Halt"), ("sender", "Sender")] {
        let mut spec = AgentSpec::new(id, json!({}));
        spec.entry = Some(entry.into());
        rt.add_agent(spec).unwrap();
    }
    let report = rt.run().unwrap();
    assert!(report.agents.iter().all(|a| a.error.is_none()), "{report:?}");
    let sender = report.agents.iter().find(|a| a.id == "sender").unwrap();
    assert_eq!(sender.state["__ui"]["@seen"], json!([0, 0, 0]));
    assert_eq!(sender.metrics.handler_errors, 0);
    let saved: J = serde_json::from_slice(&std::fs::read(checkpoint).unwrap()).unwrap();
    assert_eq!(saved["data"]["image"]["journaled"], 3);
}

#[test]
fn root_and_child_journals_recover_repeatedly_before_new_appends() {
    use std::io::Write;
    let dir = Scratch::new();
    let app=dir.app("function Child(){const [n,setN]=useState(0,'n');useEffect(()=>{(async()=>{while(true){await harness.wait({ms:5});setN(x=>x+1);}})();},[]);return String(n);}export default function Main(){return <Mount key='child'><Child/></Mount>;}");
    let mut cfg = config(&app);
    cfg.dir = Some(dir.0.join("run"));
    cfg.max_wall = Some(Duration::from_millis(100));
    let enc = encoder();
    let mut first = Runtime::new(cfg.clone(), enc.clone(), model());
    first.add_agent(AgentSpec::new("a", json!({}))).unwrap();
    let first_report = first.run().unwrap();
    assert!(
        first_report.agents.iter().all(|a| a.error.is_none()),
        "{first_report:?}"
    );
    let child = cfg.dir.as_ref().unwrap().join("a/child/journal.jsonl");
    let root = cfg.dir.as_ref().unwrap().join("a/journal.jsonl");
    let mut last = first_report.agents.iter().find(|a| a.id == "a/child").unwrap().state["__ui"]["@n"]
        .as_u64()
        .unwrap();
    for attempt in 0..3 {
        let mut prefixes = vec![];
        for path in [&root, &child] {
            let prefix = std::fs::read(path).unwrap();
            prefixes.push(prefix);
            let mut f = std::fs::OpenOptions::new().append(true).open(path).unwrap();
            f.write_all(if attempt % 2 == 0 {
                b"{\"input\":\"event\",\"name\":\"\xf0\x9f"
            } else {
                b"{\"input\":"
            })
            .unwrap();
            f.sync_all().unwrap();
        }
        let report = Runtime::resume(cfg.clone(), enc.clone(), model())
            .unwrap()
            .run()
            .unwrap();
        assert!(report.agents.iter().all(|a| a.error.is_none()), "{report:?}");
        let n = report.agents.iter().find(|a| a.id == "a/child").unwrap().state["__ui"]["@n"]
            .as_u64()
            .unwrap();
        assert!(
            n > last,
            "child failed to progress after recovery {attempt}: {report:?}"
        );
        last = n;
        for (path, prefix) in [&root, &child].into_iter().zip(prefixes) {
            let bytes = std::fs::read(path).unwrap();
            assert!(bytes.starts_with(&prefix));
            for line in bytes.split(|b| *b == b'\n').filter(|l| !l.is_empty()) {
                serde_json::from_slice::<affordance::runtime::Input>(line).unwrap();
            }
        }
    }
    let mut bytes = std::fs::read(&root).unwrap();
    bytes.extend_from_slice(b"{\"input\":\"result\"}");
    std::fs::write(&root, &bytes).unwrap();
    // A bad record is not torn: the agent's own restore refuses it, and its journal is kept.
    let report = Runtime::resume(cfg, enc, model()).unwrap().run().unwrap();
    let a = report.agents.iter().find(|a| a.id == "a").unwrap();
    assert!(
        a.error.as_deref().is_some_and(|e| e.contains("resume failed")),
        "{report:?}"
    );
    assert_eq!(std::fs::read(&root).unwrap(), bytes);
}

/// A journal record this binary cannot read (written by a newer one) fails that agent's
/// restore alone: the rest of the fleet resumes.
#[test]
fn one_unreadable_journal_does_not_stop_the_fleet() {
    use std::io::Write;
    let dir = Scratch::new();
    let app = dir.app(
        "export default function Main(){const [n,setN]=useState(0,'n');useEffect(()=>{setN(1);},[]);return String(n);}",
    );
    let mut cfg = config(&app);
    cfg.dir = Some(dir.0.join("run"));
    let enc = encoder();
    let mut first = Runtime::new(cfg.clone(), enc.clone(), model());
    for id in ["a", "b", "c"] {
        first.add_agent(AgentSpec::new(id, json!({}))).unwrap();
    }
    assert!(first.run().unwrap().agents.iter().all(|a| a.error.is_none()));
    let journal = dir.0.join("run/b/journal.jsonl");
    std::fs::OpenOptions::new()
        .append(true)
        .open(&journal)
        .unwrap()
        .write_all(b"{\"input\":\"unknown-variant\",\"clock_ms\":1}\n")
        .unwrap();
    let bytes = std::fs::read(&journal).unwrap();
    let report = Runtime::resume(cfg, enc, model()).unwrap().run().unwrap();
    for agent in &report.agents {
        if agent.id == "b" {
            assert!(
                agent.error.as_deref().is_some_and(|e| e.contains("resume failed")),
                "{report:?}"
            );
        } else {
            assert!(agent.error.is_none(), "{report:?}");
            assert_eq!(agent.state["__ui"]["@n"], json!(1));
        }
    }
    assert_eq!(std::fs::read(&journal).unwrap(), bytes);
}

/// One runtime owns a run directory: a second one over it (a resume, or a fresh run) is refused
/// before it recovers anything, and the first runs on unaffected. The lock goes with its owner.
#[test]
fn a_second_runtime_cannot_take_a_live_run_directory() {
    let dir = Scratch::new();
    let app=dir.app("export default function Main(){const [n,setN]=useState(0,'n');useEffect(()=>{(async()=>{while(true){await harness.wait({ms:5});setN(x=>x+1);}})();},[]);return String(n);}");
    let mut cfg = config(&app);
    cfg.dir = Some(dir.0.join("run"));
    cfg.max_wall = Some(Duration::from_millis(600));
    let enc = encoder();
    let mut first = Runtime::new(cfg.clone(), enc.clone(), model());
    first.add_agent(AgentSpec::new("a", json!({}))).unwrap();
    let running = std::thread::spawn(move || first.run());
    let journal = dir.0.join("run/a/journal.jsonl");
    while std::fs::metadata(&journal).map_or(true, |m| m.len() == 0) {
        std::thread::sleep(Duration::from_millis(5));
    }
    let err = Runtime::resume(cfg.clone(), enc.clone(), model())
        .err()
        .expect("a resume over a live directory must fail");
    assert!(
        err.contains("in use by another runtime") && err.contains(&cfg.dir.as_ref().unwrap().display().to_string()),
        "{err}"
    );
    let mut fresh = Runtime::new(cfg.clone(), enc.clone(), model());
    fresh.add_agent(AgentSpec::new("a", json!({}))).unwrap();
    assert!(
        fresh
            .run()
            .err()
            .is_some_and(|e| e.contains("in use by another runtime"))
    );
    let report = running.join().unwrap().unwrap();
    assert!(report.agents.iter().all(|a| a.error.is_none()), "{report:?}");
    let n = report.agents[0].state["__ui"]["@n"].as_u64().unwrap();
    let resumed = Runtime::resume(cfg, enc, model()).unwrap().run().unwrap();
    assert!(
        resumed.agents[0].error.is_none() && resumed.agents[0].state["__ui"]["@n"].as_u64().unwrap() > n,
        "{resumed:?}"
    );
}

#[test]
fn inspection_grants_are_independent_of_other_api_capabilities() {
    let dir = Scratch::new();
    let app=dir.app("export default function Main(){useEffect(()=>{(async()=>{try{await harness.inspect('*');harness.stop('allowed');}catch(e){harness.stop(e.message);}})()},[]);return 'reader';}");
    let mut rt = Runtime::new(config(&app), encoder(), model());
    let mut spec = AgentSpec::new("reader", json!({}));
    spec.inspect = vec!["*".into()];
    spec.capabilities = Some(vec!["store.*".into()]);
    rt.add_agent(spec).unwrap();
    assert_eq!(rt.run().unwrap().agents[0].stopped, Some(json!("allowed")));
}

// Counts this process's threads in `/proc`.
#[cfg(target_os = "linux")]
#[test]
fn completed_runtimes_release_their_background_threads() {
    const CHILD: &str = "AFFORDANCE_RELIABILITY_THREAD_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "reliability::completed_runtimes_release_their_background_threads",
                "--test-threads=1",
            ])
            .env(CHILD, "1")
            .status()
            .unwrap();
        assert!(status.success());
        return;
    }
    let threads = || std::fs::read_dir("/proc/self/task").unwrap().count();
    let before = threads();
    let enc = encoder();
    for queued in [false, true, false] {
        let dir = Scratch::new();
        let app=dir.app(if queued {"export default function Main(){useEffect(()=>{model();},[]);return <span label=\"protocol\" syntax=\"hermes\">waiting</span>;}"} else {"export default function Main(){return 'idle';}"});
        let mut cfg = config(&app);
        cfg.dir = Some(dir.0.join("run"));
        if queued {
            cfg.model_inflight = Some(0);
            cfg.max_wall = Some(Duration::from_millis(30));
        }
        let mut rt = Runtime::new(cfg, enc.clone(), model());
        rt.add_agent(AgentSpec::new("a", json!({}))).unwrap();
        rt.run().unwrap();
    }
    for _ in 0..100 {
        if threads() <= before {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(threads() <= before, "threads before={before}, after={}", threads());
}

#[test]
fn resume_rejects_a_saved_identity_that_escapes_its_directory() {
    let dir = Scratch::new();
    let app = dir.app("export default function Main(){return 'idle';}");
    let mut cfg = config(&app);
    cfg.dir = Some(dir.0.join("run"));
    let enc = encoder();
    let mut rt = Runtime::new(cfg.clone(), enc.clone(), model());
    rt.add_agent(AgentSpec::new("a", json!({}))).unwrap();
    rt.run().unwrap();
    let path = dir.0.join("run/a/agent.json");
    let mut saved: J = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    saved["id"] = json!("../outside");
    std::fs::write(&path, saved.to_string()).unwrap();
    let err = Runtime::resume(cfg, enc, model())
        .err()
        .expect("invalid saved identity must fail");
    assert!(err.contains("does not match its directory"), "{err}");
}

#[test]
fn an_inspection_cursor_refreshes_after_a_child_is_remounted() {
    let dir = Scratch::new();
    let app = dir.app(
        r#"
function Kid({initial}) { const [n]=useState(initial,'n'); return String(n); }
export default function Main() {
 const [phase,setPhase]=useState(0,'phase'); const [seen,setSeen]=useState(null,'seen');
 useEffect(()=>{(async()=>{
  await harness.wait({ms:30}); const first=await harness.inspect('a/kid',{select:['n']});
  setPhase(1); await harness.wait({ms:30}); setPhase(2); await harness.wait({ms:30});
  const second=await harness.inspect('a/kid',{select:['n'],since:first.since}); setSeen(second.agents['a/kid']);
 })()},[]);
 return phase===1?'off':<Mount key="kid"><Kid initial={phase===0?1:2}/></Mount>;
}
"#,
    );
    let mut cfg = config(&app);
    cfg.dir = Some(dir.0.join("run"));
    let mut rt = Runtime::new(cfg, encoder(), model());
    let mut spec = AgentSpec::new("a", json!({}));
    spec.inspect = vec!["a/kid".into()];
    rt.add_agent(spec).unwrap();
    let report = rt.run().unwrap();
    let parent = report.agents.iter().find(|a| a.id == "a").unwrap();
    let seen = &parent.state["__ui"]["@seen"];
    assert_eq!(seen["version"], 0, "both incarnations have empty journals");
    assert_eq!(seen["changed"], true);
    assert_eq!(seen["changes"], json!([{"key":"n","set":2}]));
}
