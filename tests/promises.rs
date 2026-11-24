//! Observable Promise behavior, compared with Node and exercised through both execution APIs.
//! No downloaded tokenizer, model, server, allocation address or effect key is required.
use crate::tokenizer;
use affordance::{
    View,
    effects::{Effect, Outcome, Request},
    runtime::{Command, Input, Instance, InstanceConfig},
};
use serde_json::{Value as J, json};
use std::{collections::BTreeMap, path::PathBuf, process::Command as Process, rc::Rc};

#[derive(Default)]
struct Case {
    name: String,
    body: String,
}
fn corpus() -> Vec<Case> {
    let mut cases: Vec<Case> = vec![];
    for line in include_str!("promise_corpus.js").lines() {
        if let Some(name) = line.strip_prefix("// case: ") {
            cases.push(Case {
                name: name.into(),
                ..Default::default()
            });
        } else if let Some(case) = cases.last_mut() {
            case.body.push_str(line);
            case.body.push('\n');
        }
    }
    cases
}
fn hosts() -> J {
    json!({
        "object": {"rank": 0, "outcome": Outcome::Ok(json!({"n": 2}))},
        "slow": {"rank": 2, "outcome": Outcome::Ok(json!("SLOW"))},
        "fast": {"rank": 1, "outcome": Outcome::Ok(json!("FAST"))},
        "failure": {"rank": 0, "outcome": Outcome::Error("no such file".into())},
        "abort": {"rank": 0, "outcome": Outcome::Error("AbortError: aborted".into())},
        "work": {"rank": 0, "outcome": Outcome::Ok(J::Null)},
        "cleanup": {"rank": 0, "outcome": Outcome::Ok(J::Null)}
    })
}
fn host(label: &J) -> (u64, Outcome) {
    let record = &hosts()[label.as_str().expect("host label")];
    (
        record["rank"].as_u64().expect("known host label"),
        serde_json::from_value(record["outcome"].clone()).unwrap(),
    )
}
struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn node(cases: &[Case], dir: &Scratch) -> Vec<J> {
    let input = dir.0.join("cases.json");
    std::fs::write(
        &input,
        json!({"bodies": cases.iter().map(|c| &c.body).collect::<Vec<_>>(), "hosts": hosts()}).to_string(),
    )
    .unwrap();
    let script = dir.0.join("oracle.cjs");
    std::fs::write(
        &script,
        r#"
const {bodies, hosts} = JSON.parse(require('fs').readFileSync(process.argv[2], 'utf8'));
const AsyncFunction = Object.getPrototypeOf(async function(){}).constructor;
(async () => {
  const outputs = [];
  for (const body of bodies) {
    const pending = [];
    const host = q => new Promise((resolve, reject) => pending.push({q, resolve, reject}));
    let done = false, result, error;
    new AsyncFunction('host', body)(host).then(v => {done = true; result = v;}, e => {done = true; error = e;});
    for (let step = 0; step < 100; step++) {
      await new Promise(setImmediate);
      if (!pending.length) { if (done) break; throw new Error('no progress'); }
      pending.sort((a,b) => hosts[a.q].rank - hosts[b.q].rank);
      const {q, resolve, reject} = pending.shift();
      const outcome = hosts[q].outcome;
      if (outcome.status === 'ok') resolve(outcome.value);
      else {
        const text = outcome.value, typed = /^([A-Za-z]+Error): (.*)$/.exec(text);
        const e = new Error(typed ? typed[2] : text);
        if (typed) e.name = typed[1];
        reject(e);
      }
    }
    if (!done || error) throw error || new Error('step limit');
    outputs.push(result);
  }
  process.stdout.write(JSON.stringify(outputs));
})().catch(e => {console.error(e); process.exitCode = 1;});
"#,
    )
    .unwrap();
    let out = Process::new("node")
        .arg(&script)
        .arg(&input)
        .output()
        .expect("Node is required for Promise conformance");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    serde_json::from_slice(&out.stdout).unwrap()
}
fn config() -> InstanceConfig {
    InstanceConfig {
        context: None,
        retry: Default::default(),
        template: None,
        id: "promise".into(),
        props: json!({}),
        env: BTreeMap::new(),
        max_new: 100,
        max_turns: None,
        max_prompt: None,
        start_ms: 0,
        seed: 7,
        machines: vec!["http".into()],
        model: None,
    }
}
fn agent(view: &View, enc: &affordance::emit::Encoder) -> J {
    let mut agent = Instance::new(config()).unwrap();
    assert!(agent.start(view, enc).unwrap().is_empty());
    let mut journal = vec![Input::Event {
        name: "go".into(),
        payload: json!({}),
        id: None,
        clock_ms: 1,
    }];
    let mut commands = agent.handle(view, enc, journal[0].clone()).unwrap();
    for step in 0..100 {
        if commands.is_empty() {
            break;
        }
        commands.sort_by_key(|c| match c {
            Command::Call { args, .. } => host(&args["q"]).0,
            other => panic!("unexpected {other:?}"),
        });
        let Command::Call { task, args, .. } = commands.remove(0) else {
            unreachable!()
        };
        let input = Input::Result {
            task,
            outcome: host(&args["q"]).1,
            clock_ms: step + 2,
        };
        journal.push(input.clone());
        commands.extend(agent.handle(view, enc, input).unwrap());
    }
    assert!(commands.is_empty(), "agent step limit");
    assert!(agent.quiescent(), "settled work must release its owner");
    let errors = agent.metrics.handler_errors;
    assert_eq!(errors, 0, "{:?}", agent.take_trace());
    let (restored, _) = Instance::restore(config(), journal, view, enc).unwrap();
    assert_eq!(restored.metrics.handler_errors, 0);
    assert!(restored.quiescent(), "restored settled work must release its owner");
    assert_eq!(restored.state(), agent.state(), "restore must preserve observations");
    agent.state()["__ui"]["@result"].clone()
}
fn transcript(view: &View, enc: &affordance::emit::Encoder) -> J {
    let mut log: Vec<Effect> = vec![];
    let now = chrono::NaiveDate::from_ymd_opt(2026, 10, 26)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    for at in 1..100 {
        let run = view
            .act_effects(
                enc,
                Rc::new(Default::default()),
                now,
                "check",
                0,
                &json!({}),
                &json!({}),
                log,
            )
            .unwrap();
        log = run.effects;
        let next = log
            .iter()
            .enumerate()
            .filter(|(_, e)| e.outcome.is_none())
            .min_by_key(|(_, e)| {
                let Request::Call { args, .. } = &e.request else {
                    panic!("unexpected request")
                };
                host(&args["q"]).0
            })
            .map(|(i, _)| i);
        if let Some(i) = next {
            let Request::Call { args, .. } = &log[i].request else {
                unreachable!()
            };
            log[i].outcome = Some(host(&args["q"]).1);
            log[i].settled = Some(at);
        } else {
            assert!(run.done, "handler made no progress");
            return run.writes["result"].clone();
        }
    }
    panic!("transcript step limit")
}
fn compare() {
    let dir = Scratch(std::env::temp_dir().join(format!("aff-promise-behavior-{}", std::process::id())));
    std::fs::create_dir_all(&dir.0).unwrap();
    let cases = corpus();
    let want = node(&cases, &dir);
    let enc = tokenizer::encoder();
    for (i, (case, want)) in cases.iter().zip(want).enumerate() {
        let path = dir.0.join(format!("case{i}.tsx"));
        std::fs::write(
            &path,
            format!(
                r#"export default function Main() {{
          const [result, setResult] = useState(null, 'result');
          const check = async () => {{
            const host = q => harness.machine.call({{q}});
            const run = async () => {{ {} }};
            return {{result: await run()}};
          }};
          useEvent('go', async () => {{ const out = await check(); setResult(out.result); }});
          return <aff name="check" onCheck={{check}}/>;
        }}"#,
                case.body
            ),
        )
        .unwrap();
        let view = View::load(&path, "Main", &enc).unwrap_or_else(|e| panic!("{}: {e}", case.name));
        eprintln!("checking {}", case.name);
        let live = agent(&view, &enc);
        let replay = transcript(&view, &enc);
        assert_eq!(live, replay, "{}: agent and transcript disagree", case.name);
        assert_eq!(live, want, "{}", case.name);
    }
    eprintln!(
        "{} Promise behaviors checked against Node, transcript replay and journal restore",
        cases.len()
    );
}
#[test]
fn supported_promise_behavior_matches_node_and_survives_restore() {
    compare();
}
