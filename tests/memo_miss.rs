//! A `memoScan` that can reuse nothing of its kept variant (its init or deps are new objects every
//! render) recomputes the whole history each time. The runtime reports it once, as a `memo_miss`
//! trace entry naming the cause, instead of an app silently getting slower with every input.
use affordance::View;
use affordance::emit::Encoder;
use affordance::runtime::{Input, Instance, InstanceConfig};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::PathBuf;

fn agent(init: &str, deps: &str) -> Vec<serde_json::Value> {
    scan("xs", init, deps)
}
fn agent_items() -> Vec<serde_json::Value> {
    scan("xs.map(x => ({...x}))", "null", "null")
}

fn scan(items: &str, init: &str, deps: &str) -> Vec<serde_json::Value> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let enc = Encoder::from_file(root.join("ref/tokenizers/Qwen__Qwen3-8B.json").to_str().unwrap()).unwrap();
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "aff-memo-miss-{}-{}",
        std::process::id(),
        N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("app.tsx"),
        format!(
            r#"
export default function Main() {{
  const [xs, setXs] = useState(Array.from({{length: 100}}, (_, i) => ({{i}})), "xs");
  useEvent("add", () => setXs(a => [...a, {{i: a.length}}]));
  const r = memoScan({items}, {init}, (s, x) => [(s ?? 0) + 1, x.i], {deps});
  return <text>{{`${{r.outs.length}}`}}</text>;
}}"#
        ),
    )
    .unwrap();
    let view = View::load(&dir.join("app.tsx"), "Main", &enc).unwrap();
    let mut a = Instance::new(InstanceConfig {
        context: None,
        retry: Default::default(),
        id: "a".into(),
        props: json!({}),
        env: BTreeMap::new(),
        max_new: 64,
        max_turns: None,
        max_prompt: None,
        start_ms: 1_791_000_000_000,
        seed: 7,
        machines: vec![],
        model: None,
        template: None,
    })
    .unwrap();
    a.start(&view, &enc).unwrap();
    for n in 0..3 {
        a.handle(
            &view,
            &enc,
            Input::Event {
                name: "add".into(),
                payload: json!({}),
                id: None,
                clock_ms: 1_791_000_000_100 + n,
            },
        )
        .unwrap();
    }
    a.take_trace().into_iter().filter(|t| t["t"] == "memo_miss").collect()
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn a_scan_that_reuses_nothing_is_reported_once_with_its_cause() {
    let misses = agent("{n: 0}", "null");
    assert_eq!(misses.len(), 1, "{misses:?}");
    assert!(
        misses[0]["reason"].as_str().unwrap().starts_with("init differs"),
        "{misses:?}"
    );
    let misses = agent("null", "{k: 1}");
    assert_eq!(misses.len(), 1, "{misses:?}");
    assert!(
        misses[0]["reason"].as_str().unwrap().starts_with("deps differ"),
        "{misses:?}"
    );
    assert!(
        agent("null", "[1]").is_empty(),
        "a scan that reuses its prefix is not reported"
    );
    // Items rebuilt as new objects every render: only the reused prefix is cheap, so it is reported.
    let misses = agent_items();
    assert_eq!(misses.len(), 1, "{misses:?}");
    assert!(
        misses[0]["reason"].as_str().unwrap().starts_with("an item differs"),
        "{misses:?}"
    );
}
