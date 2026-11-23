//! Template apps mark the protocol turns they write:
//! `<turn>` spans around each unit a drop would remove whole (a message; consecutive tool
//! results, which share one opener and closer), `<history>` around the run of history units,
//! the system header and the current screen apart. The spans are transparent: the ids are
//! the template's, byte for byte (the conformance sets check that).
use affordance::View;
use affordance::emit::{Encoder, RunCache};
use serde_json::{Value as J, json};
use std::path::PathBuf;
use std::sync::Arc;

use crate::shared;
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn spans(app: &str, tokenizer: &str, log: J) -> Vec<(String, J, String)> {
    let (enc, r) = render_app(app, tokenizer, log, "");
    // In document order (a span is recorded when it closes, so an outer one after its inner ones).
    let mut found: Vec<_> = r
        .spans
        .iter()
        .filter(|s| s.label == "turn" || s.label == "history")
        .filter(|s| s.kind == "label")
        .collect();
    found.sort_by_key(|s| (s.start, std::cmp::Reverse(s.end)));
    found
        .into_iter()
        .map(|s| {
            (
                s.label.clone(),
                s.meta.clone(),
                enc.tok.decode(&r.ids[s.start..s.end], false).unwrap(),
            )
        })
        .collect()
}

/// Render a template app over a log, with a style sheet.
fn render_app(app: &str, tokenizer: &str, log: J, tss: &str) -> (Arc<Encoder>, affordance::Rendered) {
    try_render_app(app, tokenizer, log, tss)
        .map(|(e, r)| (e, r.unwrap()))
        .unwrap()
}

fn try_render_app(
    app: &str,
    tokenizer: &str,
    log: J,
    tss: &str,
) -> Result<(Arc<Encoder>, Result<affordance::Rendered, String>), String> {
    try_render(app, tokenizer, log, tss, "")
}

/// `try_render_app` with `prelude` (directives) at the top of the app file.
fn try_render(
    app: &str,
    tokenizer: &str,
    log: J,
    tss: &str,
    prelude: &str,
) -> Result<(Arc<Encoder>, Result<affordance::Rendered, String>), String> {
    let path = if tokenizer.ends_with(".json") {
        root().join("ref").join(tokenizer)
    } else {
        root().join("ref/tokenizers").join(format!("{tokenizer}.json"))
    };
    let enc = shared::tokenizer(&path);
    let dir = std::env::temp_dir().join(format!("aff-turn-spans-{}-{app}", std::process::id()));
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = dir.join(N.fetch_add(1, std::sync::atomic::Ordering::SeqCst).to_string());
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("s.tss"), tss).unwrap();
    std::fs::write(
        dir.join("app.tsx"),
        format!(
            r#"{prelude}import "./s.tss";
import {{ {app} }} from '{root}/views/template_apps/{file}.tsx';
import {{ TurnStrategy }} from '{root}/views/strategies/turn_strategy.tsx';
export default function Main({{ log }}) {{
  return <{app}><TurnStrategy strategy="unrolled" log={{log}}>the screen</TurnStrategy></{app}>;
}}"#,
            root = root().display(),
            file = match app {
                "Qwen3App" => "qwen3",
                "Qwen3BaseApp" => "qwen3_base",
                "Qwen36App" => "qwen36",
                "Llama31App" => "llama31",
                "MiMo25App" => "mimo25",
                "Nemotron3App" => "nemotron3",
                "GLM53App" => "glm53",
                "DeepSeekV41App" => "deepseek_v41",
                "MessagesApp" => "messages",
                other => other,
            }
        ),
    )
    .unwrap();
    let view = View::load(&dir.join("app.tsx"), "Main", &enc).unwrap();
    let state = affordance::value::state(&json!({"log": log}));
    let now = chrono::NaiveDate::from_ymd_opt(2026, 11, 3)
        .unwrap()
        .and_hms_opt(9, 0, 0)
        .unwrap();
    let r = view.render(&enc, state, &mut RunCache::new(true), now);
    Ok((enc, r))
}

fn qwen_log() -> J {
    json!([
        {"kind": "input", "content": "Build it."},
        {"kind": "model", "content": "", "calls": [
            {"id": "c1", "name": "shell", "arguments": {"command": "make"}},
            {"id": "c2", "name": "shell", "arguments": {"command": "make test"}}]},
        {"kind": "action", "name": "shell", "tool_call_id": "c1", "result": "built"},
        {"kind": "action", "name": "shell", "tool_call_id": "c2", "result": "passed"},
        {"kind": "input", "content": "Ship it."},
    ])
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn qwen3_marks_each_protocol_turn() {
    for (app, tokenizer) in [
        ("Qwen3App", "Qwen__Qwen3-8B"),
        ("Qwen3BaseApp", "Qwen__Qwen3-8B"),
        ("Qwen36App", "Qwen__Qwen3-8B"),
        ("MiMo25App", "XiaomiMiMo__MiMo-V2.5"),
        (
            "Nemotron3App",
            "middle_models/nvidia__NVIDIA-Nemotron-3-Nano-30B-A3B-BF16/tokenizer.json",
        ),
    ] {
        check_im_turns(app, tokenizer);
    }
}

// The ChatML families: <|im_start|> opens every turn; consecutive tool results are one turn.
fn check_im_turns(app: &str, tokenizer: &str) {
    let s = spans(app, tokenizer, qwen_log());
    let roles: Vec<String> = s
        .iter()
        .map(|(label, meta, _)| {
            if label == "history" {
                "history".into()
            } else {
                meta["role"].as_str().unwrap_or("").to_string()
            }
        })
        .collect();
    assert_eq!(
        roles,
        ["system", "history", "user", "assistant", "tool", "user", "user"],
        "{app}: {s:#?}"
    );
    // One tool turn holds both results: one opener, one closer.
    let tool = &s.iter().find(|(_, m, _)| m["role"] == "tool").unwrap().2;
    assert!(
        tool.starts_with("<|im_start|>")
            && tool.trim_end().ends_with("<|im_end|>")
            && tool.contains("built")
            && tool.contains("passed"),
        "{app}: {tool:?}"
    );
    assert_eq!(tool.matches("<|im_start|>").count(), 1);
    // Every turn starts at a special token, so dropping one leaves the rest encoded as it was.
    for (label, _, text) in &s {
        if label == "turn" {
            assert!(text.starts_with("<|im_start|>"), "{text:?}");
        }
    }
    // The current screen is its own turn, after the history.
    let current = s.iter().find(|(_, m, _)| m["current"] == true).unwrap();
    assert!(current.2.contains("the screen"), "{current:?}");
}

fn roles(s: &[(String, J, String)]) -> Vec<String> {
    s.iter()
        .map(|(label, meta, _)| {
            if label == "history" {
                "history".into()
            } else {
                meta["role"].as_str().unwrap_or("").to_string()
            }
        })
        .collect()
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn llama31_marks_each_message() {
    let s = spans(
        "Llama31App",
        "meta-llama__Llama-3.1-8B-Instruct",
        json!([
            {"kind": "input", "content": "Build it."},
            // The HF template reads `tool_calls[i].function` (and raises without it).
            {"kind": "model", "content": "", "calls": [{"id": "c1", "type": "function", "function": {"name": "shell", "arguments": {"command": "make"}}}]},
            {"kind": "action", "name": "shell", "tool_call_id": "c1", "result": "built"},
            {"kind": "input", "content": "Ship it."},
        ]),
    );
    assert_eq!(
        roles(&s),
        ["system", "history", "user", "assistant", "tool", "user", "user"],
        "{s:#?}"
    );
    assert!(
        s[0].2.starts_with("<|begin_of_text|><|start_header_id|>system"),
        "{:?}",
        s[0].2
    );
    for (label, _, text) in &s[1..] {
        if label == "turn" {
            assert!(
                text.starts_with("<|start_header_id|>") && text.ends_with("<|eot_id|>"),
                "{text:?}"
            );
        }
    }
    assert!(s.last().unwrap().2.contains("the screen"));
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs the test tokenizers: tools/fetch-test-tokenizers.sh")]
fn glm53_keeps_an_assistant_with_its_observation_block() {
    let s = spans("GLM53App", "zai-org__GLM-5.3", qwen_log());
    assert_eq!(
        roles(&s),
        ["system", "history", "user", "assistant", "user", "user"],
        "{s:#?}"
    );
    // The observation block (both results, written before the next user turn) ends the assistant's turn.
    let assistant = &s.iter().find(|(_, m, _)| m["role"] == "assistant").unwrap().2;
    assert!(
        assistant.starts_with("<|assistant|>")
            && assistant.contains("<|observation|>")
            && assistant.contains("built")
            && assistant.contains("passed"),
        "{assistant:?}"
    );
    assert_eq!(assistant.matches("<|observation|>").count(), 1);
    for (label, m, text) in &s[1..] {
        if label == "turn" && m["role"] == "user" {
            assert!(text.starts_with("<|user|>"), "{text:?}");
        }
    }
    assert!(s.last().unwrap().2.contains("the screen"));
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs the test tokenizers: tools/fetch-test-tokenizers.sh")]
fn deepseek_v41_marks_each_exchange() {
    let s = spans(
        "DeepSeekV41App",
        "deepseek-ai__DeepSeek-V4.1-Flash",
        json!([
            {"kind": "input", "content": "Build it."},
            {"kind": "model", "content": "", "calls": [{"id": "c1", "type": "function", "function": {"name": "shell", "arguments": {"command": "make"}}}]},
            {"kind": "action", "name": "shell", "tool_call_id": "c1", "result": "built"},
            {"kind": "model", "content": "Done."},
            {"kind": "input", "content": "Ship it."},
        ]),
    );
    // The first exchange is the user's request with the assistant's call; the tool result opens
    // the next one (merged into a user message) with the answer; then the screen's.
    assert_eq!(roles(&s), ["system", "history", "user", "user", "user"], "{s:#?}");
    assert!(
        s[0].2.starts_with("<｜begin▁of▁sentence｜><｜System｜>"),
        "{:?}",
        s[0].2
    );
    for (label, _, text) in &s[1..] {
        if label == "turn" {
            assert!(text.starts_with("<｜User｜>"), "{text:?}");
        }
    }
    let first = &s[2].2;
    assert!(
        first.contains("Build it.") && first.contains("<｜Assistant｜>") && first.ends_with("<｜end▁of▁sentence｜>"),
        "{first:?}"
    );
    assert!(s.last().unwrap().2.contains("the screen"));
}

/// A log's text-only `model` record carries its calls as `{name, arguments}`; the app hands
/// Llama 3.1's template the chat-API shape it reads (`{function: {name, arguments}}`).
#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn llama31_app_renders_text_only_call_records() {
    let s = spans(
        "Llama31App",
        "meta-llama__Llama-3.1-8B-Instruct",
        json!([
            {"kind": "input", "content": "Build it."},
            {"kind": "model", "content": "", "calls": [{"id": "c1", "name": "shell", "arguments": {"command": "make"}}]},
            {"kind": "action", "name": "shell", "tool_call_id": "c1", "result": "built"},
            {"kind": "input", "content": "Ship it."},
        ]),
    );
    let assistant = &s.iter().find(|(_, m, _)| m["role"] == "assistant").unwrap().2;
    assert!(
        assistant.contains(r#"{"name": "shell", "parameters": {"command": "make"}}"#),
        "{assistant:?}"
    );
}

/// A text-only reply with no calls (`useLog` writes `calls: []`) is an assistant message with no
/// `tool_calls`: Llama 3.1's template rejects a list that does not hold exactly one call.
#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn a_reply_without_calls_renders_on_every_template() {
    let log = json!([
        {"kind": "input", "content": "Hi."},
        {"kind": "model", "ids": null, "content": "Hello.", "calls": []},
        {"kind": "input", "content": "Bye."},
    ]);
    for (app, tokenizer) in [
        ("Llama31App", "meta-llama__Llama-3.1-8B-Instruct"),
        ("Qwen3App", "Qwen__Qwen3-8B"),
        ("GLM53App", "zai-org__GLM-5.3"),
    ] {
        let s = spans(app, tokenizer, log.clone());
        assert!(
            s.iter()
                .any(|(_, m, text)| m["role"] == "assistant" && text.contains("Hello.")),
            "{app}: {s:?}"
        );
    }
}

fn long_log(n: usize) -> J {
    let mut log = vec![];
    for i in 0..n {
        log.push(
            json!({"kind": "input", "content": format!("Request {i}: please build the next part of the project.")}),
        );
        log.push(json!({"kind": "model", "content": format!("Working on part {i}."), "calls": [{"id": format!("c{i}"), "type": "function", "function": {"name": "shell", "arguments": {"command": format!("make part{i}")}}}]}));
        log.push(json!({"kind": "action", "name": "shell", "tool_call_id": format!("c{i}"), "result": format!("part {i} built: 12 files, 0 warnings")}));
    }
    log.push(json!({"kind": "input", "content": "Ship it."}));
    J::Array(log)
}

/// A sheet caps the history: whole protocol turns drop from the start, the system turn and the
/// current screen stay whole, and every kept turn is exactly as the template wrote it.
#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn a_sheet_drops_whole_history_turns() {
    for (app, tokenizer, opener) in [
        ("Qwen3App", "Qwen__Qwen3-8B", "<|im_start|>"),
        ("DeepSeekV41App", "deepseek-ai__DeepSeek-V4.1-Flash", "<｜User｜>"),
        ("Qwen3BaseApp", "Qwen__Qwen3-8B", "<|im_start|>"),
        ("Qwen36App", "Qwen__Qwen3-8B", "<|im_start|>"),
        ("MiMo25App", "XiaomiMiMo__MiMo-V2.5", "<|im_start|>"),
        (
            "Nemotron3App",
            "middle_models/nvidia__NVIDIA-Nemotron-3-Nano-30B-A3B-BF16/tokenizer.json",
            "<|im_start|>",
        ),
        ("GLM53App", "zai-org__GLM-5.3", "<|"),
        ("Llama31App", "meta-llama__Llama-3.1-8B-Instruct", "<|start_header_id|>"),
    ] {
        let (enc, full) = render_app(app, tokenizer, long_log(60), "");
        let (_, cut) = render_app(
            app,
            tokenizer,
            long_log(60),
            "history { max-tokens: 600tok; overflow: drop-start; drop-step: 25%; }",
        );
        let bx = cut
            .spans
            .iter()
            .find(|s| s.kind == "box" && s.label == "history")
            .expect("the history is a box");
        assert!(
            bx.end - bx.start <= 600 && bx.meta["dropped_items"].as_u64().unwrap() > 0,
            "{app}: {:?}",
            bx.meta
        );
        let kept = enc.tok.decode(&cut.ids[bx.start..bx.end], false).unwrap();
        assert!(
            kept.starts_with(opener),
            "{app}: the first kept turn is whole: {:?}",
            &kept[..80.min(kept.len())]
        );
        // The rest of the prompt is unchanged: the same ids before and after the history.
        let full_box = full
            .spans
            .iter()
            .find(|s| s.kind == "label" && s.label == "history")
            .unwrap();
        assert_eq!(
            cut.ids[..bx.start],
            full.ids[..full_box.start],
            "{app}: the system turn"
        );
        assert_eq!(cut.ids[bx.end..], full.ids[full_box.end..], "{app}: the current screen");
        // The kept turns are a suffix of the full history, token for token.
        assert!(
            full.ids[full_box.start..full_box.end].ends_with(&cut.ids[bx.start..bx.end]),
            "{app}"
        );
    }
}

/// `clip-middle` keeps the head and the tail; `more` names what was hidden.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs the test tokenizers: tools/fetch-test-tokenizers.sh")]
fn clip_middle_keeps_head_and_tail() {
    let enc = shared::tokenizer(&root().join("ref/tokenizers/Qwen__Qwen3-8B.json"));
    let dir = std::env::temp_dir().join(format!("aff-clip-middle-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("v.tsx"), r#"export default function Main({ lines }) {
  return <box id="out" max_tokens={60} overflow="clip-middle" more={"\n[... {hidden} tokens hidden, read {id}]\n"}>{lines.join("\n")}</box>;
}"#).unwrap();
    let view = View::load(&dir.join("v.tsx"), "Main", &enc).unwrap();
    let lines: Vec<String> = (0..200).map(|i| format!("line {i} of the build log")).collect();
    let now = chrono::NaiveDate::from_ymd_opt(2026, 11, 3)
        .unwrap()
        .and_hms_opt(9, 0, 0)
        .unwrap();
    let r = view
        .render(
            &enc,
            affordance::value::state(&json!({"lines": lines})),
            &mut RunCache::new(true),
            now,
        )
        .unwrap();
    let text = enc.tok.decode(&r.ids, false).unwrap();
    assert!(r.ids.len() <= 60, "{}", r.ids.len());
    assert!(
        text.starts_with("line 0 of the build log") && text.ends_with("line 199 of the build log"),
        "{text:?}"
    );
    let bx = r.spans.iter().find(|s| s.kind == "box").unwrap();
    let hidden = bx.meta["hidden_tokens"].as_u64().unwrap();
    assert!(
        text.contains(&format!("[... {hidden} tokens hidden, read out]")),
        "{text:?}"
    );
}

/// `max-tokens: 80win` is a share of the prompt's window, which the model binding's `context`
/// sets; with no window (no `context`) it caps nothing, like a page with no viewport.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs the test tokenizers: tools/fetch-test-tokenizers.sh")]
fn a_window_share_without_a_window_caps_nothing() {
    let (_, plain) = render_app("Qwen3App", "Qwen__Qwen3-8B", long_log(30), "");
    let (_, r) = render_app(
        "Qwen3App",
        "Qwen__Qwen3-8B",
        long_log(30),
        "history { max-tokens: 1win; overflow: drop-start; }",
    );
    assert_eq!(r.ids, plain.ids);
}

/// Tool output is capped by layout, in tokens: a `result` rule boxes each tool result's content
/// inside its turn (the protocol tokens around it stay), and `{id}` names the record.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs the test tokenizers: tools/fetch-test-tokenizers.sh")]
fn a_sheet_caps_tool_results_in_tokens() {
    let long: String = (0..400).map(|i| format!("line {i} of the build output\n")).collect();
    let log = json!([
        {"kind": "input", "content": "Build it."},
        {"kind": "model", "content": "", "calls": [{"id": "c1", "type": "function", "function": {"name": "shell", "arguments": {"command": "make"}}}]},
        {"kind": "action", "name": "shell", "tool_call_id": "c1", "result": long, "id": "e7"},
        {"kind": "input", "content": "Ship it."},
    ]);
    let tss = r#"result[name="shell"] { max-tokens: 80tok; overflow: clip-middle; more: "\n[... {hidden} tokens hidden, read_event {id}]\n"; }"#;
    for (app, tokenizer, open, close) in [
        ("Qwen3App", "Qwen__Qwen3-8B", "<tool_response>\n", "\n</tool_response>"),
        (
            "DeepSeekV41App",
            "deepseek-ai__DeepSeek-V4.1-Flash",
            "<tool_result>",
            "</tool_result>",
        ),
    ] {
        let (enc, full) = render_app(app, tokenizer, log.clone(), "");
        let (_, capped) = render_app(app, tokenizer, log.clone(), tss);
        let bx = capped
            .spans
            .iter()
            .find(|s| s.kind == "box" && s.label == "e7")
            .expect("the result is a box");
        assert!(bx.end - bx.start <= 80, "{app}: {}", bx.end - bx.start);
        let text = enc.tok.decode(&capped.ids, false).unwrap();
        let hidden = bx.meta["hidden_tokens"].as_u64().unwrap();
        assert!(
            text.contains(&format!("{open}line 0 of the build output")),
            "{app}: {text}"
        );
        assert!(
            text.contains(&format!("line 399 of the build output\n{close}")),
            "{app}: {text}"
        );
        assert!(
            text.contains(&format!("[... {hidden} tokens hidden, read_event e7]")),
            "{app}: {text}"
        );
        assert!(capped.ids.len() + 100 < full.ids.len(), "{app}");
        // Everything outside the tool output is as the template wrote it.
        let full_text = enc.tok.decode(&full.ids, false).unwrap();
        let (a, b) = (
            text.split("line 0 of").next().unwrap(),
            full_text.split("line 0 of").next().unwrap(),
        );
        assert_eq!(a, b, "{app}");
    }
}

/// Descendant selectors: an app styles the history its template writes by its own props
/// (`Policy[mode="trunc"] history { ... }`), as CSS selects an element inside a classed one.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs the test tokenizers: tools/fetch-test-tokenizers.sh")]
fn a_descendant_selector_styles_the_history_inside_an_app_component() {
    let enc = shared::tokenizer(&root().join("ref/tokenizers/Qwen__Qwen3-8B.json"));
    let dir = std::env::temp_dir().join(format!("aff-descendant-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("s.tss"),
        r#"Policy[mode="trunc"] history { max-tokens: 400tok; overflow: drop-start; }"#,
    )
    .unwrap();
    std::fs::write(
        dir.join("app.tsx"),
        format!(
            r#"import "./s.tss";
import {{ Qwen3App }} from '{root}/views/template_apps/qwen3.tsx';
import {{ TurnStrategy }} from '{root}/views/strategies/turn_strategy.tsx';
function Policy({{ mode, log }}) {{
  return <Qwen3App><TurnStrategy strategy="unrolled" log={{log}}>the screen</TurnStrategy></Qwen3App>;
}}
export default function Main({{ log, mode }}) {{ return <Policy mode={{mode}} log={{log}}/>; }}"#,
            root = root().display()
        ),
    )
    .unwrap();
    let view = View::load(&dir.join("app.tsx"), "Main", &enc).unwrap();
    let now = chrono::NaiveDate::from_ymd_opt(2026, 11, 3)
        .unwrap()
        .and_hms_opt(9, 0, 0)
        .unwrap();
    let render = |mode: &str| {
        view.render(
            &enc,
            affordance::value::state(&json!({"log": long_log(30), "mode": mode})),
            &mut RunCache::new(true),
            now,
        )
        .unwrap()
    };
    let (trunc, none) = (render("trunc"), render("none"));
    let bx = trunc
        .spans
        .iter()
        .find(|s| s.kind == "box" && s.label == "history")
        .expect("capped under mode=trunc");
    assert!(bx.end - bx.start <= 400);
    assert!(
        none.spans.iter().all(|s| s.kind != "box"),
        "no rule applies under another mode"
    );
    assert!(trunc.ids.len() < none.ids.len());
}

/// Hosted (Messages): layout runs before serialization. A `history` rule drops whole units (a
/// message, or a call with all its results) from the start; the system message and the screen
/// stay, and a call never loses its results.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs the test tokenizers: tools/fetch-test-tokenizers.sh")]
fn a_hosted_history_drops_whole_units_before_serialization() {
    let messages = |tss: &str| {
        let (enc, r) = render_app("MessagesApp", "Qwen__Qwen3-8B", long_log(40), tss);
        let text = enc.tok.decode(&r.ids, false).unwrap();
        let doc: J = serde_json::from_str(text.lines().next().unwrap()).unwrap();
        doc["messages"].as_array().unwrap().clone()
    };
    let (full, cut) = (
        messages(""),
        messages("history { max-tokens: 1500tok; overflow: drop-start; drop-step: 25%; }"),
    );
    assert!(
        cut.len() < full.len() && cut.len() > 2,
        "{} of {}",
        cut.len(),
        full.len()
    );
    assert_eq!(cut[0], full[0], "the system message stays");
    assert_eq!(cut.last(), full.last(), "the screen stays");
    // The kept history is a suffix of the full one, starting at a unit: never a tool result alone.
    // When that unit is not the user's, a user message says earlier messages were dropped.
    let marked = cut[1]["content"] == "[Earlier messages were dropped to fit the context window.]";
    let kept = &cut[1 + usize::from(marked)..];
    assert_eq!(kept, &full[full.len() - kept.len()..]);
    assert_ne!(kept[0]["role"], "tool");
    assert_eq!(cut[1]["role"], "user", "the history starts with a user message");
    assert_eq!(marked, kept[0]["role"] != "user");
    // An agent's history has one user message, the oldest: once it drops, the marker stands in,
    // with the rule's own text when it gives one.
    let (enc, r) = render_app(
        "MessagesApp",
        "Qwen__Qwen3-8B",
        graded_log(40),
        "history { max-tokens: 600tok; overflow: drop-start; more: \"[older steps omitted]\"; }",
    );
    let text = enc.tok.decode(&r.ids, false).unwrap();
    let doc: J = serde_json::from_str(text.lines().next().unwrap()).unwrap();
    let agent = doc["messages"].as_array().unwrap();
    assert_eq!(agent[1], json!({"role": "user", "content": "[older steps omitted]"}));
    assert_eq!(agent[2]["role"], "assistant");
}

fn graded_log(n: usize) -> J {
    let mut log = vec![];
    for i in 0..n {
        log.push(json!({"kind": "model", "content": "", "calls": [{"id": format!("c{i}"), "type": "function", "function": {"name": "shell", "arguments": {"command": "make"}}}]}));
        log.push(json!({"kind": "action", "name": "shell", "tool_call_id": format!("c{i}"), "result": format!("output {i}: the build wrote twelve files and printed no warnings at all"), "id": format!("r{i}")}));
    }
    log.insert(0, json!({"kind": "input", "content": "Build."}));
    log.push(json!({"kind": "input", "content": "Ship it."}));
    J::Array(log)
}

/// Graded overflow: `result:nth-last(n+K / S)` elides results at
/// least K from the end among the history's, the boundary moving S at a time, natively and hosted.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs the test tokenizers: tools/fetch-test-tokenizers.sh")]
fn old_results_elide_in_steps() {
    let tss = r#"result:nth-last(n+5 / 3) { max-tokens: 6tok; overflow: elide; more: "[elided]"; }"#;
    for (app, tokenizer) in [
        ("Qwen3App", "Qwen__Qwen3-8B"),
        ("DeepSeekV41App", "deepseek-ai__DeepSeek-V4.1-Flash"),
        ("MessagesApp", "Qwen__Qwen3-8B"),
    ] {
        let elided = |n: usize| {
            let (enc, r) = render_app(app, tokenizer, graded_log(n), tss);
            let text = enc.tok.decode(&r.ids, false).unwrap();
            let text = if app == "MessagesApp" {
                text.lines().next().unwrap().to_string()
            } else {
                text
            };
            let kept = (0..n).filter(|i| text.contains(&format!("output {i}:"))).count();
            assert_eq!(text.matches("[elided]").count() + kept, n, "{app} n={n}");
            n - kept
        };
        // Of n results, those in the history count (DeepSeek merges the last one into the current
        // exchange, which is not history): index i elides iff i <= 3 * floor((h - 5) / 3).
        let in_current = usize::from(app == "DeepSeekV41App");
        let want: Vec<usize> = (40..45)
            .map(|n: usize| {
                let h = n - in_current;
                3 * ((h - 5) / 3) + 1
            })
            .collect();
        assert_eq!((40..45).map(elided).collect::<Vec<_>>(), want, "{app}");
        // Without the rule nothing changes.
        let (_, plain) = render_app(app, tokenizer, graded_log(10), "");
        let (_, other) = render_app(
            app,
            tokenizer,
            graded_log(10),
            r#"result[name="nothing"] { max-tokens: 6tok; }"#,
        );
        assert_eq!(plain.ids, other.ids, "{app}");
    }
}

/// Grading never moves a history drop's cut: the drop measures results at their static length.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs the test tokenizers: tools/fetch-test-tokenizers.sh")]
fn grading_does_not_move_the_history_cut() {
    let drop = "history { max-tokens: 900tok; overflow: drop-start; drop-step: 25%; }";
    let graded =
        format!("{drop}\nresult:nth-last(n+5 / 3) {{ max-tokens: 6tok; overflow: elide; more: \"[elided]\"; }}");
    for n in [40, 41, 50] {
        let first_kept = |tss: &str| {
            let (enc, r) = render_app("Qwen3App", "Qwen__Qwen3-8B", graded_log(n), tss);
            let text = enc.tok.decode(&r.ids, false).unwrap();
            (0..n)
                .find(|i| text.contains(&format!("\"id\": \"c{i}\"")) || text.contains(&format!("output {i}:")))
                .unwrap_or(n)
        };
        let (a, b) = (first_kept(drop), first_kept(&graded));
        assert!(a > 0, "the history overflowed (n={n})");
        // The model calls carry their ids; the first kept call is the same either way.
        let call = |tss: &str| {
            let (enc, r) = render_app("Qwen3App", "Qwen__Qwen3-8B", graded_log(n), tss);
            let text = enc.tok.decode(&r.ids, false).unwrap();
            text.matches("<tool_call>").count()
        };
        assert_eq!(call(drop), call(&graded), "n={n}: the same turns are kept ({a} vs {b})");
    }
}

/// Every template app writes its control tokens as tokens, never as text that spells them: on
/// clean data a render is the same with data text kept as text (`"use safe_text"`) as without.
/// (A template that passed `tok.x.text` where the token belongs differed: Llama 3.1's
/// `bos_token`, Nemotron 3's think tags.)
#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn template_apps_emit_tokens_not_text_that_spells_them() {
    for (app, tokenizer) in [
        ("Qwen3App", "Qwen__Qwen3-8B"),
        ("DeepSeekV41App", "deepseek-ai__DeepSeek-V4.1-Flash"),
        ("Qwen3BaseApp", "Qwen__Qwen3-8B"),
        ("Qwen36App", "Qwen__Qwen3.6-27B"),
        ("MiMo25App", "XiaomiMiMo__MiMo-V2.5"),
        (
            "Nemotron3App",
            "middle_models/nvidia__NVIDIA-Nemotron-3-Nano-30B-A3B-BF16/tokenizer.json",
        ),
        ("GLM53App", "zai-org__GLM-5.3"),
        ("Llama31App", "meta-llama__Llama-3.1-8B-Instruct"),
        ("MessagesApp", "Qwen__Qwen3-8B"),
    ] {
        let render = |prelude: &str| {
            try_render(app, tokenizer, long_log(6), "", prelude)
                .unwrap()
                .1
                .unwrap()
                .ids
        };
        assert_eq!(render("\"use safe_text\";\n"), render(""), "{app}");
    }
}

/// A template app's options are declared with it (views/template_apps/index.json): GLM-5.3's
/// `reasoning_effort` reaches its prompt, and an option the app does not take (GLM-5.3 has no
/// thinking switch) is refused, not silently ignored.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs the test tokenizers: tools/fetch-test-tokenizers.sh")]
fn template_options_are_the_apps_own() {
    use affordance::runtime::model::ModelTemplate;
    let error = ModelTemplate::new("glm53", json!({"enable_thinking": true})).unwrap_err();
    assert!(
        error.contains("takes no option \"enable_thinking\"") && error.contains("reasoning_effort"),
        "{error}"
    );
    assert!(ModelTemplate::new("glm53", json!({"reasoning_effort": "high"})).is_ok());
    assert!(
        ModelTemplate::new("qwen3", json!({"enable_thinking": false}))
            .unwrap_err()
            .contains("its options: keep_replies")
    );
    let enc = shared::tokenizer(&root().join("ref/tokenizers/zai-org__GLM-5.3.json"));
    let dir = std::env::temp_dir().join(format!("aff-turn-spans-effort-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("app.tsx"), format!(r#"import {{ GLM53App }} from '{root}/views/template_apps/glm53.tsx';
import {{ TurnStrategy }} from '{root}/views/strategies/turn_strategy.tsx';
export default function Main({{ effort }}) {{
  return <GLM53App reasoning_effort={{effort}}><TurnStrategy strategy="unrolled" log={{[{{kind: "input", content: "Go."}}]}}>the screen</TurnStrategy></GLM53App>;
}}"#, root = root().display())).unwrap();
    let view = View::load(&dir.join("app.tsx"), "Main", &enc).unwrap();
    let render = |effort: J| {
        let r = view
            .render(
                &enc,
                affordance::value::state(&json!({"effort": effort})),
                &mut RunCache::new(true),
                chrono::NaiveDate::from_ymd_opt(2026, 11, 3)
                    .unwrap()
                    .and_hms_opt(9, 0, 0)
                    .unwrap(),
            )
            .unwrap();
        enc.tok.decode(&r.ids, false).unwrap()
    };
    assert!(
        render(json!("high")).contains("Reasoning Effort: High") && render(J::Null).contains("Reasoning Effort: Max")
    );
}

/// A box's limit holds also when a graded rule makes old results larger than their static
/// style (so the history's measured length is under its limit while its content is over):
/// the box is then treated as over, not laid out whole. Found by the proof in `src/fit.rs`,
/// whose claim had this as its one exception.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs the test tokenizers: tools/fetch-test-tokenizers.sh")]
fn a_history_cap_holds_when_old_results_grow() {
    let tss = "history { max-tokens: 1545tok; overflow: drop-start; }\n\
               result { max-tokens: 6tok; }\n\
               result:nth-last(n+5 / 3) { max-tokens: 400tok; }";
    let (_, r) = render_app("Qwen3App", "Qwen__Qwen3-8B", graded_log(40), tss);
    let history = r
        .spans
        .iter()
        .filter(|s| s.label == "history")
        .map(|s| s.end - s.start)
        .max()
        .expect("a history span");
    assert!(
        history <= 1545,
        "the history box emitted {history} tokens over its 1545 limit"
    );
    assert!(history > 1000, "the test must fill the box ({history})");
}
