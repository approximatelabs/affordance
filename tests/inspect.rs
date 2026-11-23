//! `harness.inspect`: an instance granted `inspect` reads a copy of other
//! instances' committed state. The copy is tagged JSON (a Map stays a Map), `since` sends only what
//! changed, a granted prefix bounds what may be read, an instance can refuse to be read, the copy
//! can be one string of JSON lines, and the reader's journal replays it exactly.
use crate::tokenizer;
use affordance::runtime::{AgentSpec, FnModel, GenReply, GenRequest, Runtime, RuntimeConfig};
use serde_json::{Value as J, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

const TARGET: &str = r#"export default function Main() {
  const [count, setCount] = useState(0, "count");
  const [items, setItems] = useState([], "items");
  const [m, setM] = useState(new Map(), "m");
  const [secret] = useState("s3cret", "secret");
  const [data] = useState({"a.b": [1, 2], a: {b: [3]}, "": 4}, "data");
  useEvent("bump", () => {
    setCount(c => c + 1);
    setItems(xs => [...xs, {n: xs.length, at: harness.agent}]);
    setM(prev => new Map(prev).set(`k${prev.size}`, prev.size));
  });
  return <text>{`count=${count}`}</text>;
}"#;

const OVERSEER: &str = r#"export default function Main() {
  useEffect(() => {
    (async () => {
      const select = ["count", "m", {key: "items", tail: 2}];
      const r1 = await harness.inspect("*", {select});
      for (let i = 0; i < 3; i++) await harness.send("agent-0", {}, {event: "bump"});
      const r2 = await harness.inspect("*", {select, since: r1.since});
      await harness.send("agent-0", {}, {event: "bump"});
      const r3 = await harness.inspect("*", {select, since: r2.since});
      const r4 = await harness.inspect("*", {select, since: r3.since});
      const more = await harness.inspect("agent-0", {select: [{key: "items", tail: 4}], since: r4.since});
      await harness.send("agent-0", {}, {event: "bump"});
      const countOnly = await harness.inspect("agent-0", {select: ["count"], since: more.since});
      const expanded = await harness.inspect("agent-0", {select: ["count", "m", "secret", {key: "items", tail: 4}], since: countOnly.since});
      const whole = await harness.inspect("agent-0", {select: ["items"], since: expanded.since});
      let denied = null;
      try { await harness.inspect("other", {select}); } catch (e) { denied = e.message; }
      const named = await harness.inspect(["agent-1", "agent-quiet", "agent-none"], {select: ["count"]});
      const t = await harness.inspect(["agent-0"], {select: ["m"], as: "text"});
      const paths = await harness.inspect("agent-0", {select: [
        {key: "data", path: ["a.b"], tail: 1}, {key: "data", path: ["a", "b"]}, {key: "data", path: [""]}
      ]});
      const shape = (r) => Object.fromEntries(Object.entries(r.agents).map(([id, a]) => [id, a.changed === false ? "unchanged" : a.error || a.changes]));
      harness.stop({r1: shape(r1), r2: shape(r2), r3: shape(r3), r4: shape(r4), more: shape(more), expanded: shape(expanded), whole: shape(whole), paths: shape(paths), phase: r1.agents["agent-1"].status.phase, denied, named: shape(named), text: t.text});
    })();
  }, []);
  return <text>{"overseeing"}</text>;
}"#;

fn run(dir: &Path, restore: bool) -> affordance::runtime::Report {
    std::fs::write(dir.join("target.tsx"), TARGET).unwrap();
    std::fs::write(dir.join("overseer.tsx"), OVERSEER).unwrap();
    let enc = Arc::new(tokenizer::encoder());
    let mut cfg = RuntimeConfig::new(dir.join("target.tsx"));
    cfg.dir = Some(dir.join("run"));
    cfg.workers = 3;
    let model = Arc::new(FnModel(|_: &GenRequest| Ok(GenReply::default())));
    let mut rt = if restore {
        Runtime::restore(cfg, enc, model).unwrap()
    } else {
        Runtime::new(cfg, enc, model)
    };
    if !restore {
        for id in ["agent-0", "agent-1", "agent-quiet", "other"] {
            let mut s = AgentSpec::new(id, json!({"name": id}));
            s.model = None;
            s.inspectable = id != "agent-quiet";
            rt.add_agent(s).unwrap();
        }
        let mut s = AgentSpec::new("overseer", json!({}));
        s.model = None;
        s.app = Some(dir.join("overseer.tsx").display().to_string());
        s.inspect = vec!["agent-*".into()];
        rt.add_agent(s).unwrap();
    }
    rt.run().unwrap()
}

#[test]
fn an_overseer_reads_what_changed_and_only_what_it_was_granted() {
    let dir = std::env::temp_dir().join(format!("aff-inspect-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let report = run(&dir, false);
    let o = report.agents.iter().find(|a| a.id == "overseer").unwrap();
    assert!(o.error.is_none(), "{:?}", o.error);
    let s = o.stopped.clone().expect("the overseer stopped");
    let map =
        |n: usize| json!({"$": "map", "entries": (0..n).map(|i| json!([format!("k{i}"), i])).collect::<Vec<_>>()});
    let item = |n: usize| json!({"n": n, "at": "agent-0"});
    // The first read: every granted, inspectable instance; the selected hooks whole, a list's tail.
    let first = json!([{"key": "count", "set": 0}, {"key": "m", "set": map(0)}, {"key": "items", "from": 0, "items": [], "length": 0}]);
    assert_eq!(s["r1"], json!({"agent-0": first, "agent-1": first}), "{s}");
    // Since r1: agent-0 changed (its list sends what it gained); agent-1 did not.
    assert_eq!(
        s["r2"]["agent-0"],
        json!([{"key": "count", "set": 3}, {"key": "m", "set": map(3)}, {"key": "items", "from": 0, "items": [item(0), item(1), item(2)], "length": 3}]),
        "{s}"
    );
    assert_eq!(s["r2"]["agent-1"], "unchanged");
    assert_eq!(
        s["r3"]["agent-0"],
        json!([{"key": "count", "set": 4}, {"key": "m", "set": map(4)}, {"key": "items", "from": 3, "items": [item(3)], "length": 4}]),
        "{s}"
    );
    assert_eq!(s["r4"], json!({"agent-0": "unchanged", "agent-1": "unchanged"}));
    assert_eq!(
        s["more"]["agent-0"],
        json!([{"key":"items","from":0,"items":[item(0),item(1),item(2),item(3)],"length":4}])
    );
    assert_eq!(
        s["expanded"]["agent-0"],
        json!([{"key":"m","set":map(5)},{"key":"secret","set":"s3cret"},
        {"key":"items","from":1,"items":[item(1),item(2),item(3),item(4)],"length":5}])
    );
    assert_eq!(
        s["whole"]["agent-0"],
        json!([{"key":"items","set":[item(0),item(1),item(2),item(3),item(4)]}])
    );
    assert_eq!(
        s["paths"]["agent-0"],
        json!([
            {"key":"data","path":["a.b"],"from":1,"items":[2],"length":2},
            {"key":"data","path":["a","b"],"from":0,"items":[3],"length":1},
            {"key":"data","path":[""],"set":4}
        ])
    );
    assert_eq!(s["phase"], "idle");
    assert!(s["denied"].as_str().unwrap().contains("may not inspect other"), "{s}");
    assert_eq!(
        s["named"],
        json!({"agent-1": [{"key": "count", "set": 0}], "agent-quiet": "agent-quiet is not inspectable", "agent-none": "no instance agent-none"})
    );
    let line: J = serde_json::from_str(s["text"].as_str().unwrap().trim_end()).unwrap();
    assert_eq!(line["id"], "agent-0");
    assert_eq!(line["changes"], json!([{"key": "m", "set": map(5)}]));
    // The targets' journals hold only their own inputs: being read wrote nothing to them.
    let journal = std::fs::read_to_string(crate::shared::inst(&dir, "run/agent-0/journal.jsonl")).unwrap();
    assert_eq!(journal.lines().count(), 5, "{journal}");
    assert!(
        !std::fs::read_to_string(crate::shared::inst(&dir, "run/agent-1/journal.jsonl"))
            .unwrap_or_default()
            .contains("inspect")
    );
    // The overseer replays its reads from its journal.
    let again = run(&dir, true);
    let o2 = again.agents.iter().find(|a| a.id == "overseer").unwrap();
    assert_eq!(o2.stopped, Some(s));
    assert_eq!(o2.diverged_at, None);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// No grant, no read: `capabilities` absent ("all") does not include `inspect`.
#[test]
fn inspect_needs_an_explicit_grant() {
    let dir = std::env::temp_dir().join(format!("aff-inspect-grant-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("app.tsx"),
        r#"export default function Main({ name }) {
  useEffect(() => { if (name == "reader") (async () => {
    try { await harness.inspect("*"); harness.stop("read"); } catch (e) { harness.stop(e.message); }
  })(); }, []);
  return <text>{name}</text>;
}"#,
    )
    .unwrap();
    let enc = Arc::new(tokenizer::encoder());
    let mut rt = Runtime::new(
        RuntimeConfig::new(dir.join("app.tsx")),
        enc,
        Arc::new(FnModel(|_: &GenRequest| Ok(GenReply::default()))),
    );
    for id in ["reader", "b"] {
        let mut s = AgentSpec::new(id, json!({"name": id}));
        s.model = None;
        rt.add_agent(s).unwrap();
    }
    let report = rt.run().unwrap();
    let r = report.agents.iter().find(|a| a.id == "reader").unwrap();
    assert!(
        r.stopped
            .as_ref()
            .unwrap()
            .as_str()
            .unwrap()
            .contains("this agent may not call inspect"),
        "{:?}",
        r.stopped
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The overseer pattern: inspect the fleet as text, pipe it into a mirror on the overseer's
/// machine, and query the mirror there as ordinary data. No byte of it passes through a model.
#[test]
fn an_overseer_mirrors_the_fleet_into_its_machine() {
    use affordance::runtime::{Exec, Target};
    let dir = std::env::temp_dir().join(format!("aff-inspect-mirror-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("target.tsx"), TARGET).unwrap();
    std::fs::write(dir.join("overseer.tsx"), r#"export default function Main({ lib }) {
  useEffect(() => {
    (async () => {
      const select = ["count", "m", {key: "items", tail: 2},
        {key: "data", path: ["a.b"], tail: 1}, {key: "data", path: ["a", "b"]}, {key: "data", path: [""]}];
      const env = {LIB: lib};
      let since = {};
      for (let round = 0; round < 3; round++) {
        const r = await harness.inspect("*", {select, since, as: "text"});
        await harness.machine.call({cmd: 'python3 "$LIB/affordance_mirror.py" apply mirror', stdin: r.text, env});
        since = r.since;
        for (let i = 0; i < 2; i++) await harness.send("agent-0", {}, {event: "bump"});
      }
      const q = await harness.machine.call({env, cmd: `PYTHONPATH="$LIB" python3 -c '
import json
from affordance_mirror import Mirror
a = Mirror("mirror")
z = a["agent-0"]
print(json.dumps({"ids": a.ids(), "count": z["count"], "items": [i["n"] for i in z["items"]], "m": sorted(z["m"].items()), "data": z["data"], "version": a.version("agent-0"), "phase": a.status("agent-1")["phase"]}))'`});
      harness.stop(JSON.parse(q.output));
    })();
  }, []);
  return <text>{"overseeing"}</text>;
}"#).unwrap();
    let enc = Arc::new(tokenizer::encoder());
    let mut cfg = RuntimeConfig::new(dir.join("target.tsx"));
    cfg.dir = Some(dir.join("run"));
    cfg.workers = 2;
    let mut rt = Runtime::new(cfg, enc, Arc::new(FnModel(|_: &GenRequest| Ok(GenReply::default()))));
    rt.bind_machine(
        "home",
        Arc::new(Exec {
            target: Target::Local {
                home: Some(dir.join("home").display().to_string()),
            },
            default_timeout_s: 30,
            max_capture: 1 << 20,
        }),
    );
    for id in ["agent-0", "agent-1"] {
        let mut s = AgentSpec::new(id, json!({}));
        s.model = None;
        s.machines = Some(vec![]);
        rt.add_agent(s).unwrap();
    }
    let mut s = AgentSpec::new(
        "overseer",
        json!({"lib": root().join("tests/support").display().to_string()}),
    );
    s.model = None;
    s.app = Some(dir.join("overseer.tsx").display().to_string());
    s.inspect = vec!["*".into()];
    rt.add_agent(s).unwrap();
    let report = rt.run().unwrap();
    let o = report.agents.iter().find(|a| a.id == "overseer").unwrap();
    assert!(o.error.is_none(), "{:?}", o.error);
    // Mirrored after the third read: four bumps (the last two came after it).
    assert_eq!(
        o.stopped,
        Some(json!({"ids": ["agent-0", "agent-1"], "count": 4, "items": [0, 1, 2, 3],
        "m": [["k0", 0], ["k1", 1], ["k2", 2], ["k3", 3]], "data": {"a.b": [2], "a": {"b": [3]}, "": 4}, "version": 4, "phase": "idle"})),
        "{:?}",
        o.stopped
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn python_interop_mirror_preserves_instance_identity() {
    let out = std::process::Command::new("python3")
        .arg(root().join("tests/support/check_mirror.py"))
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
}
