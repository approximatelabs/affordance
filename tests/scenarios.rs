//! Every app scenario in views/ (`X.scenario.json` next to `X.tsx`) passes under `afr test`'s runner.
use std::path::{Path, PathBuf};

use crate::shared;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn scenarios(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            scenarios(&p, out)
        } else if p.to_string_lossy().ends_with(".scenario.json") {
            out.push(p)
        }
    }
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn every_app_scenario_passes() {
    let mut found = vec![];
    scenarios(&root().join("views"), &mut found);
    found.sort();
    assert!(!found.is_empty());
    let mut failures = vec![];
    for sc in &found {
        let app = PathBuf::from(sc.to_string_lossy().replace(".scenario.json", ".tsx"));
        let s: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(sc).unwrap()).unwrap();
        let tok = s["tokenizer"].as_str().unwrap_or("zai-org__GLM-5.3");
        let enc = shared::tokenizer(&root().join(format!("ref/tokenizers/{tok}.json")));
        match affordance::scenario::run(&app, s["entry"].as_str().unwrap_or("Main"), &enc, &s) {
            Ok(r) if r.ok => {}
            Ok(r) => failures.push(format!("{}: {}", sc.display(), r.failure.unwrap())),
            Err(e) => failures.push(format!("{}: {e}", sc.display())),
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} scenarios failed:\n{}",
        failures.len(),
        found.len(),
        failures.join("\n\n")
    );
}

/// "My scenario can script a reply the app stopped, and one that ran out of tokens." The app
/// stops its first model call with a signal and reads `aborted`; its second reply is `cut_off`.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn a_scenario_scripts_an_aborted_reply_and_a_cut_off_one() {
    let dir = std::env::temp_dir().join(format!("aff-scenario-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let app = dir.join("app.tsx");
    std::fs::write(
        &app,
        r#"export default function Main(){
      const [controller, setController] = useRef(null, "controller");
      useEvent("halt", () => controller().abort());
      useEffect(()=>{(async()=>{
        const made = new AbortController();
        setController(made);
        const first = await model(undefined, {signal: made.signal});
        const second = await model();
        harness.stop({aborted: !!first.aborted, first: first.text, cut_off: !!second.cut_off});
      })();},[]);
      return <>hello<span label="protocol" syntax="glm" stop={[tok.user.id]} in_think={false}/></>;
    }"#,
    )
    .unwrap();
    let scenario = serde_json::json!({"steps": [
        {"event": "halt"},
        {"model": {"text": "part"}, "aborted": true},
        {"model": {"text": "the rest"}, "cut_off": true},
        {"expect": {"stopped": {"aborted": true, "first": "part", "cut_off": true}}}
    ]});
    let enc = shared::tokenizer(&root().join("ref/tokenizers/zai-org__GLM-5.3.json"));
    let report = affordance::scenario::run(&app, "Main", &enc, &scenario).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(report.ok, "{:?}", report.failure);
}
