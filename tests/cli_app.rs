//! CLI transport, batch state isolation and machine-readable failures.
use crate::tokenizer;
use serde_json::{Value as J, json};
use std::{
    io::Write,
    path::PathBuf,
    process::{Command, Output, Stdio},
};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn run(input: &str, extra: &[&str]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_affordance"))
        .args(["render", "--model", "Qwen__Qwen3-4B-Instruct-2507", "--view"])
        .arg(root().join("tests/fixtures/render_app.tsx"))
        .args(["--template", "qwen3", "--state", "-", "--json"])
        .args(extra)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    child.wait_with_output().unwrap()
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn app_json_contains_native_ids_and_rebased_action_spans() {
    let out = run(&json!({"text":"Notes: copper","active":true}).to_string(), &[]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let result: J = serde_json::from_slice(&out.stdout).unwrap();
    let ids: Vec<u32> = serde_json::from_value(result["ids"].clone()).unwrap();
    let span = result["spans"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["kind"] == "affordance" && s["meta"].get("handler").is_some())
        .unwrap();
    assert_eq!(span["label"], "change__notes");
    let enc = affordance::emit::Encoder::from_file(
        root()
            .join("ref/tokenizers/Qwen__Qwen3-4B-Instruct-2507.json")
            .to_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        enc.tok
            .decode(
                &ids[span["start"].as_u64().unwrap() as usize..span["end"].as_u64().unwrap() as usize],
                false
            )
            .unwrap(),
        "Notes: copper"
    );
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn app_batch_caching_preserves_each_states_actions() {
    let out = run(
        &json!([
            {"text":"first","active":true},
            {"text":"second","active":false},
            {"text":"first","active":true},
        ])
        .to_string(),
        &["--batch", "--cache"],
    );
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let result: J = serde_json::from_slice(&out.stdout).unwrap();
    let results = result.as_array().unwrap();
    assert_eq!(results.len(), 3);
    assert_eq!(results[0], results[2]);
    assert_ne!(results[0]["ids"], results[1]["ids"]);
    assert!(
        !results[1]["spans"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["kind"] == "affordance")
    );
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn invalid_app_input_returns_failure() {
    for (input, flags, message) in [
        ("{", vec![], "invalid app state"),
        ("[]", vec![], "JSON object"),
        ("[]", vec!["--batch"], "batch is empty"),
    ] {
        let out = run(input, &flags);
        assert!(!out.status.success());
        assert!(String::from_utf8_lossy(&out.stderr).contains(message));
    }
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn rendering_error_is_machine_readable_and_fails_the_command() {
    let out = run(&json!({"text":{},"active":true}).to_string(), &[]);
    assert!(!out.status.success());
    let result: J = serde_json::from_slice(&out.stdout).unwrap();
    assert!(result["error"].is_string());
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn an_installed_cli_can_render_with_explicit_files_outside_a_checkout() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_affordance"))
        .args(["render", "--view"])
        .arg(root().join("tests/fixtures/render_app.tsx"))
        .arg("--tokenizer")
        .arg(root().join("ref/tokenizers/Qwen__Qwen3-4B-Instruct-2507.json"))
        .args(["--template", "qwen3", "--state", "-", "--json"])
        .env("AFFORDANCE_ROOT", "/nonexistent-source-checkout")
        .current_dir(std::env::temp_dir())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(br#"{"text":"outside checkout","active":true}"#)
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let value: J = serde_json::from_slice(&output.stdout).unwrap();
    assert!(value["ids"].is_array());
}

/// `afr app` prints one readable line per span: the element's span and, for each `on*`
/// handler, an `action` line with its parameters.
#[test]
fn app_output_is_readable() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_affordance"))
        .args(["render", "--tokenizer", tokenizer::path().to_str().unwrap(), "--view"])
        .arg(root().join("views/demo/hello.tsx"))
        .args(["--entry", "Hello", "--state", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(br#"{"name":"world"}"#).unwrap();
    let out = String::from_utf8(child.wait_with_output().unwrap().stdout).unwrap();
    assert!(out.contains("text    Hello, world! [greet]"), "{out}");
    assert!(
        out.lines()
            .any(|l| l.starts_with("span ") && l.ends_with("action     greet(name)")),
        "{out}"
    );
    assert!(!out.contains("Span {"), "{out}");
}
