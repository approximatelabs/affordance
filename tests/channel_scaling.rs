//! Observable render-frame, delivery and recovery behavior, using the offline tokenizer.
use crate::tokenizer;
use affordance::{
    View,
    runtime::{Agent, AgentConfig, Command, Input},
};
use serde_json::{Value as J, json};
use std::{collections::BTreeMap, path::PathBuf};

struct Files(PathBuf);
impl Files {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "aff-channel-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn view(&self, source: &str, enc: &affordance::emit::Encoder) -> View {
        let path = self.0.join("app.tsx");
        std::fs::write(&path, source).unwrap();
        View::load(&path, "Main", enc).unwrap()
    }
}
impl Drop for Files {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn config() -> AgentConfig {
    AgentConfig {
        context: None,
        retry: Default::default(),
        id: "a".into(),
        props: json!({}),
        env: BTreeMap::new(),
        max_new: 32,
        max_turns: None,
        max_prompt: None,
        start_ms: 0,
        seed: 1,
        machines: vec!["home".into()],
        model: None,
        template: None,
    }
}
fn event(name: &str, clock_ms: i64) -> Input {
    Input::Event {
        name: name.into(),
        payload: json!({}),
        id: None,
        clock_ms,
    }
}

#[test]
fn render_reuse_observes_fields_escapes_dates_and_fresh_handler_globals() {
    let enc = tokenizer::encoder();
    for (body, observing) in [
        ("return 'static';", false),
        ("return String(harness.clock_ms);", true),
        ("return String(harness['clock_ms']);", true),
        ("const context=harness;return String(context.clock_ms);", true),
        ("return JSON.stringify(harness);", true),
        ("return formatNow('%S');", true),
        ("return String(harness.agent);", false),
    ] {
        let files = Files::new();
        let view=files.view(&format!("export default function Main(){{useEvent('finish',()=>harness.stop({{clock:harness.clock_ms}}));{body}}}"),&enc);
        let mut agent = Agent::new(config()).unwrap();
        agent.start(&view, &enc).unwrap();
        let evaluations = agent.render_stats.evaluations;
        agent.handle(&view, &enc, Input::Tick { clock_ms: 10_000 }).unwrap();
        agent.handle(&view, &enc, event("finish", 20_000)).unwrap();
        assert_eq!(agent.stopped(), Some(&json!({"clock":20_000})), "{body}");
        assert_eq!(
            agent.render_stats.evaluations - evaluations,
            if observing { 2 } else { 0 },
            "{body}"
        );
    }
    let files = Files::new();
    let view=files.view("export default function Main(){const context=harness;useEvent('finish',()=>harness.stop(context.clock_ms));return 'capture';}",&enc);
    let mut agent = Agent::new(config()).unwrap();
    agent.start(&view, &enc).unwrap();
    agent.handle(&view, &enc, event("finish", 20_000)).unwrap();
    assert_eq!(agent.stopped(), Some(&json!(20_000)));
    let view=files.view("export default function Main(){const roles=harness.machines;useEvent('finish',()=>harness.stop(Object.is(roles,harness.machines)));return 'captured field';}",&enc);
    let mut agent = Agent::new(config()).unwrap();
    agent.start(&view, &enc).unwrap();
    agent.handle(&view, &enc, event("finish", 20_000)).unwrap();
    assert_eq!(agent.stopped(), Some(&json!(true)));
}

#[test]
fn an_input_frame_drains_member_microtasks_and_commits_effects_once_and_replays() {
    let files = Files::new();
    let enc = tokenizer::encoder();
    let view = files.view(
        r#"export default function Main(){
      const [n,setN]=useState(0,'n');const [direct,setDirect]=useState(0,'direct');
      useEvent('inc',async()=>{setN(n=>n+1);setDirect(direct+1);await Promise.resolve();setN(n=>n+1);});
      useEffect(()=>{harness.machine.call({n});},[n]);return String(n);
    }"#,
        &enc,
    );
    let mut agent = Agent::new(config()).unwrap();
    let initial = agent.start(&view, &enc).unwrap();
    assert!(
        initial
            .iter()
            .any(|c| matches!(c,Command::Call {args,..} if args["n"]==0))
    );
    let input = Input::Batch {
        inputs: vec![event("inc", 10), event("inc", 20)],
    };
    let next = agent.handle(&view, &enc, input.clone()).unwrap();
    let state = agent.state();
    // How inputs are batched never changes what the app computes: the batch gives what the same
    // two events give one at a time (a captured `direct` too), with each one's effects pass.
    let mut alone = Agent::new(config()).unwrap();
    alone.start(&view, &enc).unwrap();
    let mut each = alone.handle(&view, &enc, event("inc", 10)).unwrap();
    each.extend(alone.handle(&view, &enc, event("inc", 20)).unwrap());
    assert_eq!(state, alone.state());
    assert_eq!(state["__ui"]["@n"], 4);
    assert_eq!(state["__ui"]["@direct"], 2);
    let calls = |cs: &[Command]| -> Vec<J> {
        cs.iter()
            .filter_map(|c| match c {
                Command::Call { args, .. } => Some(args["n"].clone()),
                _ => None,
            })
            .collect()
    };
    assert_eq!(calls(&next), calls(&each));
    assert_eq!(calls(&next), vec![json!(2), json!(4)]);
    assert_eq!(agent.journal_len(), 2);
    let (restored, _) = Agent::restore(config(), [input], &view, &enc).unwrap();
    assert_eq!(restored.state(), state);
    for bad in [
        json!({"input":"batch","inputs":[]}),
        json!({"input":"batch","inputs":[{"input":"tick","clock_ms":1}]}),
        json!({"input":"batch","inputs":[{"input":"batch","inputs":[]}]}),
    ] {
        assert!(serde_json::from_value::<Input>(bad).is_err());
    }
}

#[test]
fn multicast_has_one_durable_result_and_recovery_preserves_batched_delivery() {
    use affordance::runtime::{AgentSpec, FnModel, Runtime, RuntimeConfig};
    use std::sync::Arc;
    let files = Files::new();
    let path = files.0.join("fleet.tsx");
    std::fs::write(&path,r#"export default function Main({hub,bad}){
      if(bad)throw new Error('deliberate startup failure');
      const [seen,setSeen]=useState([],'seen');
      useEvent('message',({i})=>setSeen(items=>[...items,i]));
      useEffect(()=>{if(hub)(async()=>{
        await harness.wait({ms:20});
        const empty=await harness.send([],{});
        const results=await Promise.all(Array.from({length:16},(_,i)=>harness.send(['good','bad','missing','hub','good'],{i})));
        harness.stop({empty,results});
      })();},[]);
      return 'long observation '.repeat(2000)+JSON.stringify(seen);
    }"#).unwrap();
    let mut cfg = RuntimeConfig::new(&path);
    cfg.dir = Some(files.0.join("run"));
    cfg.workers = 1;
    cfg.input_batch = 64;
    cfg.model_threads = 1;
    cfg.exec_threads = 1;
    cfg.strict_replay = true;
    let enc = Arc::new(tokenizer::encoder());
    let model = Arc::new(FnModel(|_: &affordance::runtime::GenRequest| {
        Err("no inference expected".into())
    }));
    let mut runtime = Runtime::new(cfg.clone(), enc.clone(), model.clone());
    for (id, props) in [
        ("good", json!({})),
        ("bad", json!({"bad":true})),
        ("hub", json!({"hub":true})),
    ] {
        let mut spec = AgentSpec::new(id, props);
        spec.model = None;
        spec.dedicated_worker = id == "hub";
        runtime.add_agent(spec).unwrap();
    }
    let first = runtime.run().unwrap();
    let good = first.agents.iter().find(|a| a.id == "good").unwrap();
    let hub = first.agents.iter().find(|a| a.id == "hub").unwrap();
    assert!(good.error.is_none(), "{:?}", good.error);
    assert!(hub.error.is_none(), "{:?}", hub.error);
    assert_ne!(good.worker, hub.worker);
    assert_eq!(good.state["__ui"]["@seen"], json!((0..16).collect::<Vec<_>>()));
    let stop = hub.stopped.as_ref().unwrap();
    assert_eq!(stop["empty"], json!({"delivered":0,"failed":0,"undelivered":[]}));
    for result in stop["results"].as_array().unwrap() {
        assert_eq!(
            result,
            &json!({"delivered":1,"failed":3,"undelivered":["bad","missing","hub"]})
        );
    }
    let journal = std::fs::read_to_string(crate::shared::inst(&files.0, "run/good/journal.jsonl")).unwrap();
    let frames: Vec<Input> = journal
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert!(
        frames
            .iter()
            .any(|input| matches!(input,Input::Batch {inputs} if inputs.len()>1)),
        "the burst must share a journal frame"
    );
    let cached = Runtime::restore(cfg.clone(), enc.clone(), model.clone())
        .unwrap()
        .run()
        .unwrap();
    for id in ["good", "hub"] {
        assert_eq!(
            first.agents.iter().find(|a| a.id == id).unwrap().state,
            cached.agents.iter().find(|a| a.id == id).unwrap().state
        );
    }
    for id in ["good", "hub"] {
        let _ = std::fs::remove_file(crate::shared::instance_dir(&files.0.join("run"), id).join("checkpoint.json"));
    }
    let replay = Runtime::restore(cfg, enc, model).unwrap().run().unwrap();
    for id in ["good", "hub"] {
        let before = first.agents.iter().find(|a| a.id == id).unwrap();
        let after = replay.agents.iter().find(|a| a.id == id).unwrap();
        assert!(after.error.is_none(), "{:?}", after.error);
        assert_eq!(before.state, after.state);
        assert_eq!(before.stopped, after.stopped);
    }
}

/// Starting an agent costs the same whether it is the first on its thread or the 400th: one
/// agent's work does not walk the other agents' values. The app keeps mutable state in closures,
/// which is what the cycle collector tracks.
#[test]
fn starting_an_agent_does_not_slow_down_with_the_agents_already_running() {
    let enc = tokenizer::encoder();
    let files = Files::new();
    let view = files.view(
        "export default function Main(){const [items,setItems]=useState([],'items');\
         useEffect(()=>{const held=[];for(let i=0;i<200;i++)held.push({i,next:held});setItems(held.map(h=>h.i));},[]);\
         return String(items.length);}",
        &enc,
    );
    let mut agents = vec![];
    let mut times = vec![];
    for i in 0..400 {
        let mut cfg = config();
        cfg.id = format!("a{i}");
        let started = std::time::Instant::now();
        let mut agent = Agent::new(cfg).unwrap();
        agent.start(&view, &enc).unwrap();
        times.push(started.elapsed());
        agents.push(agent);
    }
    // The fastest start in each window: other work on the machine only ever adds time, so the
    // fastest is the work itself. If starting walked the other agents, even it would grow.
    let fastest = |slice: &[std::time::Duration]| *slice.iter().min().unwrap();
    let (first, last) = (fastest(&times[10..60]), fastest(&times[350..]));
    assert!(
        last < first * 5,
        "the first agents start in {first:?}, the last in {last:?}"
    );
}
