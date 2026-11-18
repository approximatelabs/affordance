//! A conformance run must cover its declared inventory, including on a fresh CI checkout.
use crate::tokenizer;
use serde_json::{Value as J, json};
use std::{
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};
struct Fixture {
    root: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "aff-inventory-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        for directory in ["views", "ref/tokenizers", "ref/out/dev"] {
            std::fs::create_dir_all(root.join(directory)).unwrap();
        }
        let fixture = Self { root };
        fixture.write(
            "views/index.json",
            json!({"modelA":{"view":"app.tsx","entry":"Main"},"modelB":{"view":"app.tsx","entry":"Main"}}),
        );
        fixture.write(
            "ref/cases_dev.json",
            json!([{"id":"case","messages":[{"role":"user","content":"hello"}],"tools":[],"kwargs":{}}]),
        );
        std::fs::write(
            fixture.root.join("views/app.tsx"),
            "export function Main(){return 'ok';}",
        )
        .unwrap();
        for model in ["modelA", "modelB"] {
            std::fs::copy(
                tokenizer::path(),
                fixture.root.join(format!("ref/tokenizers/{model}.json")),
            )
            .unwrap();
            fixture.write(&format!("ref/tokenizers/{model}.special.json"), json!({}));
        }
        fixture.reference("modelA");
        fixture
    }
    fn write(&self, path: &str, value: J) {
        std::fs::write(self.root.join(path), value.to_string()).unwrap();
    }
    fn reference(&self, model: &str) {
        self.write(
            &format!("ref/out/dev/{model}.json"),
            json!({"case":{"ids":tokenizer::encoder().encode_run("ok",true,false)}}),
        );
    }
    fn run(&self, extra: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_affordance"))
            .env("AFFORDANCE_ROOT", &self.root)
            .args(["check", "--set", "dev"])
            .args(extra)
            .output()
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
#[test]
fn partial_model_inventory_is_an_error_but_explicit_selection_is_supported() {
    let f = Fixture::new();
    let partial = f.run(&[]);
    assert!(!partial.status.success());
    assert!(String::from_utf8_lossy(&partial.stderr).contains("modelB"));
    let selected = f.run(&["--only", "modelA"]);
    assert!(
        selected.status.success(),
        "{}",
        format!(
            "{}\n{}",
            String::from_utf8_lossy(&selected.stdout),
            String::from_utf8_lossy(&selected.stderr)
        )
    );
    assert!(String::from_utf8_lossy(&selected.stdout).contains("Reference coverage: 1 selected models"));
    f.reference("modelB");
    assert!(f.run(&[]).status.success());
}
#[test]
fn every_selected_model_must_contain_every_expected_case() {
    let f = Fixture::new();
    f.write("ref/out/dev/modelA.json", json!({"other":{"ids":[]}}));
    let result = f.run(&["--only", "modelA"]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("modelA / case"));
}
#[test]
fn malformed_reference_records_fail_before_comparison() {
    let f = Fixture::new();
    for record in [
        json!({"ids":["x"]}),
        json!({"ids":[-1]}),
        json!({"ids":[4294967296u64]}),
        json!({"error":null}),
        json!({"ids":[],"error":"x"}),
        json!({"ids":[],"ids_raw":[null]}),
    ] {
        f.write("ref/out/dev/modelA.json", json!({"case":record}));
        let result = f.run(&["--only", "modelA"]);
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains("INVALID REFERENCE"));
    }
    std::fs::write(f.root.join("ref/out/dev/modelA.json"), "{broken").unwrap();
    assert!(!f.run(&["--only", "modelA"]).status.success());
}
#[test]
fn duplicate_cases_and_empty_model_selections_are_errors() {
    let f = Fixture::new();
    assert!(!f.run(&["--only", "unknown"]).status.success());
    f.write("ref/cases_dev.json", json!([{"id":"case"},{"id":"case"}]));
    let result = f.run(&[]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("duplicate id"));
}
