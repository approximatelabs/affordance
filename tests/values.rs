//! Values at the runtime's boundaries: objects are references inside a
//! run, and what the runtime keeps is frozen. These run in an agent, where state, kept closures and
//! the host are real: a Map in state stays a Map; state is changed by copying it, and writing to it
//! in place is a TypeError; a kept closure cannot assign the variable it captured; a value that
//! contains itself cannot go into state or to the host.
use affordance::View;
use affordance::emit::Encoder;
use affordance::runtime::{Agent, AgentConfig, Input};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
const T0: i64 = 1_791_000_000_000;

const APP: &str = r#"
export default function Main() {
  const [m, setM] = useState(new Map(), "m");
  const [items, setItems] = useState([], "items");
  const [out, setOut] = useState("", "out");
  const [c, setC] = useState(null, "counter");
  let local = 0;
  useEvent("add", ({ k, v }) => setM(prev => new Map(prev).set(k, v)));
  useEvent("read", () => setOut(`${m instanceof Map}:${m.size}:${m.get("a")}:${[...m.keys()].join("+")}`));
  useEvent("copy", () => setItems(xs => [...xs, xs.length]));
  useEvent("mutate-map", () => { try { m.set("x", 1); setOut("no error"); } catch (e) { setOut(`${e.name}: ${e.message}`); } });
  useEvent("mutate-array", () => { try { items.push(9); setOut("no error"); } catch (e) { setOut(`${e.name}: ${e.message}`); } });
  useEvent("assign-captured", () => { try { local = 5; setOut("no error"); } catch (e) { setOut(`${e.name}: ${e.message}`); } });
  useEvent("cycle-into-state", () => { try { const o = {n: 1}; o.self = o; setItems([o]); setOut("no error"); } catch (e) { setOut(`${e.name}: ${e.message}`); } });
  useEvent("cycle-to-host", async () => { try { const o = {}; o.self = o; await harness.store.put({text: "t", meta: o}); setOut("sent"); } catch (e) { setOut(`${e.name}: ${e.message}`); } });
  useEvent("counter-new", () => setC(new Counter(1)));
  useEvent("counter-bump", () => { try { c.bump(); setOut("no error"); } catch (e) { setOut(`${e.name}: ${e.message.slice(0, 40)}`); } });
  useEvent("counter-next", () => setC(prev => prev.next()));
  useEvent("counter-read", () => setOut(`${c instanceof Counter}:${c.n}:${c.double}`));
  const [deep, setDeep] = useState(null, "deep");
  const nest = (v, n) => { for (let i = 0; i < n; i++) v = {v}; return v; };
  useEvent("deep-base", () => setDeep(nest(1, 120)));
  useEvent("deep-wrap", ({ n }) => { try { setDeep(d => nest(d, n)); setOut("ok"); } catch (e) { setOut(`${e.name}`); } });
  useEvent("deep-fresh", ({ n }) => { try { setDeep(nest(1, n)); setOut("ok"); } catch (e) { setOut(`${e.name}`); } });
  return <text>{`items=${items.length}`}</text>;
}
class Counter {
  constructor(n) { this.n = n; }
  get double() { return this.n * 2; }
  bump() { this.n += 1; }
  next() { return new Counter(this.n + 1); }
}"#;

struct Rig {
    enc: Encoder,
    view: View,
    agent: Agent,
    clock: i64,
}

impl Rig {
    fn new() -> Rig {
        let enc = Encoder::from_file(root().join("ref/tokenizers/Qwen__Qwen3-8B.json").to_str().unwrap()).unwrap();
        // One directory per rig: tests run in parallel and would otherwise read each other's half-written file.
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "aff-values-{}-{}",
            std::process::id(),
            N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("app.tsx"), APP).unwrap();
        let view = View::load(&dir.join("app.tsx"), "Main", &enc).unwrap();
        let mut agent = Agent::new(AgentConfig {
            context: None,
            retry: Default::default(),
            template: None,
            id: "a".into(),
            props: json!({}),
            env: BTreeMap::new(),
            max_new: 64,
            max_turns: None,
            max_prompt: None,
            start_ms: T0,
            seed: 7,
            machines: vec![],
            model: None,
        })
        .unwrap();
        agent.start(&view, &enc).unwrap();
        Rig {
            enc,
            view,
            agent,
            clock: T0,
        }
    }
    fn event(&mut self, name: &str, payload: serde_json::Value) {
        self.clock += 10;
        self.agent
            .handle(
                &self.view,
                &self.enc,
                Input::Event {
                    name: name.into(),
                    payload,
                    id: None,
                    clock_ms: self.clock,
                },
            )
            .unwrap();
    }
    fn out(&self) -> String {
        self.agent.state()["__ui"]["@out"].as_str().unwrap_or("").to_string()
    }
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs the test tokenizers: tools/fetch-test-tokenizers.sh")]
fn a_map_in_state_stays_a_map_and_changes_by_copy() {
    let mut r = Rig::new();
    r.event("add", json!({"k": "a", "v": 1}));
    r.event("add", json!({"k": "b", "v": 2}));
    r.event("read", json!({}));
    assert_eq!(r.out(), "true:2:1:a+b");
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs the test tokenizers: tools/fetch-test-tokenizers.sh")]
fn state_is_frozen_and_copying_it_is_how_it_changes() {
    let mut r = Rig::new();
    r.event("copy", json!({}));
    r.event("copy", json!({}));
    assert_eq!(r.agent.state()["__ui"]["@items"], json!([0, 1]));
    r.event("mutate-array", json!({}));
    assert!(
        r.out()
            .starts_with("TypeError: Cannot change this value: it is kept by the runtime"),
        "{}",
        r.out()
    );
    assert_eq!(
        r.agent.state()["__ui"]["@items"],
        json!([0, 1]),
        "the state did not change"
    );
    r.event("mutate-map", json!({}));
    assert!(
        r.out().starts_with("TypeError: this Map is kept by the runtime"),
        "{}",
        r.out()
    );
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs the test tokenizers: tools/fetch-test-tokenizers.sh")]
fn a_kept_closure_cannot_assign_its_captured_variable() {
    let mut r = Rig::new();
    r.event("assign-captured", json!({}));
    assert!(
        r.out()
            .starts_with("TypeError: Assignment to `local`, a variable of a closure kept from an earlier run"),
        "{}",
        r.out()
    );
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs the test tokenizers: tools/fetch-test-tokenizers.sh")]
fn a_value_that_contains_itself_goes_neither_into_state_nor_to_the_host() {
    let mut r = Rig::new();
    r.event("cycle-into-state", json!({}));
    assert!(
        r.out().starts_with("TypeError: Converting circular structure to state"),
        "{}",
        r.out()
    );
    r.event("cycle-to-host", json!({}));
    assert!(
        r.out().starts_with("TypeError: Converting circular structure to JSON"),
        "{}",
        r.out()
    );
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs the test tokenizers: tools/fetch-test-tokenizers.sh")]
fn a_class_instance_in_state_keeps_its_class_and_is_frozen() {
    let mut r = Rig::new();
    r.event("counter-new", json!({}));
    r.event("counter-next", json!({}));
    r.event("counter-read", json!({}));
    assert_eq!(r.out(), "true:2:4");
    r.event("counter-bump", json!({}));
    assert!(
        r.out().starts_with("TypeError: Cannot change this value"),
        "{}",
        r.out()
    );
}

/// State nesting is bounded (128 levels), and the bound is the same when most of the depth is a
/// subtree an earlier state check already passed (its height is kept, not walked again).
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs the test tokenizers: tools/fetch-test-tokenizers.sh")]
fn state_nesting_is_bounded_through_checked_subtrees() {
    let mut r = Rig::new();
    let mut fresh = vec![];
    let mut wrapped = vec![];
    for n in 125..=131 {
        r.event("deep-fresh", json!({"n": n}));
        fresh.push(r.out());
        r.event("deep-base", json!({}));
        r.event("deep-wrap", json!({"n": n - 120}));
        wrapped.push(r.out());
    }
    assert_eq!(fresh, wrapped);
    assert!(
        fresh.contains(&"ok".to_string()) && fresh.contains(&"RangeError".to_string()),
        "{fresh:?}"
    );
}
