//! Adversarial checks of the engine, written independently of the conformance sets.
use affordance::emit::{Encoder, RunCache};
use affordance::{View, state_from_json};
use serde_json::json;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn now() -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 10, 5)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap()
}
fn enc() -> Encoder {
    Encoder::from_file(root().join("ref/tokenizers/Qwen__Qwen3-8B.json").to_str().unwrap()).unwrap()
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn overflow_marker_never_exceeds_budget() {
    let enc = enc();
    let dir = std::env::temp_dir().join(format!("afr_review_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for mode in ["drop-start", "drop-end", "clip-start", "clip-end"] {
        let path = dir.join(format!("marker-{mode}.tsx"));
        std::fs::write(&path, format!("export default function Main({{ limit }}) {{ return <box max_tokens={{limit}} overflow=\"{mode}\" more=\"[this marker is much too long]\">{{\"many words here and more here\"}}</box>; }}")).unwrap();
        let view = View::load(&path, "Main", &enc).unwrap();
        for budget in [0, 1, 2] {
            let mut cache = RunCache::new(false);
            let r = view
                .render(
                    &enc,
                    state_from_json(&json!({"messages":[],"kwargs":{"limit":budget}}), &json!({})),
                    &mut cache,
                    now(),
                )
                .unwrap();
            assert!(r.ids.len() <= budget, "{mode} budget {budget}: {} tokens", r.ids.len());
        }
    }
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn arbitrary_apps_can_have_their_own_tools_state() {
    let enc = enc();
    let dir = std::env::temp_dir().join(format!("afr_review_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("tools.tsx");
    std::fs::write(
        &path,
        "export default function Main({ tools }) { return p`${tools.hammer}`; }",
    )
    .unwrap();
    let view = View::load(&path, "Main", &enc).unwrap();
    let state = state_from_json(&json!({"messages":[],"tools":{"hammer":"ready"}}), &json!({}));
    let r = view.render(&enc, state, &mut RunCache::new(false), now()).unwrap();
    assert_eq!(enc.tok.decode(&r.ids, false).unwrap(), "ready");
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn fast_unicode_and_whitespace_match_official_tokenizer() {
    let mut seen = std::collections::HashSet::new();
    for name in [
        "Qwen__Qwen3-8B",
        "zai-org__GLM-5.3",
        "deepseek-ai__DeepSeek-V4.1-Flash",
        "meta-llama__Llama-3.3-70B-Instruct",
        "Aleph-Alpha__Kolibri-1",
        "openai__gpt-oss-20b",
    ] {
        let path = root().join(format!("ref/tokenizers/{name}.json"));
        let enc = Encoder::from_file(path.to_str().unwrap()).unwrap();
        let orig = tokenizers::Tokenizer::from_file(&path).unwrap();
        let config: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        if !seen.insert(config["pre_tokenizer"].to_string()) {
            continue;
        }
        let alphabet = [
            'a',
            'B',
            ' ',
            '\t',
            '\n',
            '\r',
            '\u{85}',
            '\u{a0}',
            '\u{2003}',
            '\u{2028}',
            '\u{2029}',
            '\u{3000}',
            'é',
            '\u{301}',
            '中',
            '🙂',
            '\u{1c89}',
            '\u{11f02}',
            '\u{1e4d0}',
        ];
        let mut seed = 82191u64;
        for _ in 0..128 {
            let mut text = String::new();
            for _ in 0..40 {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                text.push(alphabet[(seed >> 32) as usize % alphabet.len()]);
            }
            let want = orig.encode(text.as_str(), false).unwrap().get_ids().to_vec();
            let got = enc
                .encode_direct(&text)
                .unwrap_or_else(|| enc.encode_run(&text, true, false));
            assert_eq!(got, want, "{name}: {text:?}");
        }
    }
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn dropping_items_removes_hidden_actions_and_rebases_visible_spans() {
    let enc = enc();
    let dir = std::env::temp_dir().join(format!("afr_review_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("actions.tsx");
    std::fs::write(&path, r#"
const Rows = ({ items }) => items.map(item => <aff label={item.id} onClick={() => ({ selected: item.id })}>{p`${item.text}\n`}</aff>);
export default function Main({ items, budget }) {
  return <>{"prefix\n"}<Rows items={items} style={{ maxTokens: budget, overflow: "drop-start", more: "..." }}/>{"suffix"}</>;
}"#).unwrap();
    let items =
        json!([{"id":"old", "text":"old row with several words"}, {"id":"last", "text":"last row with several words"}]);
    let budget =
        enc.encode_run("last row with several words\n", false, false).len() + enc.encode_run("...", false, false).len();
    let state = state_from_json(
        &json!({"messages":[],"kwargs":{"items":items,"budget":budget}}),
        &json!({}),
    );
    let view = View::load(&path, "Main", &enc).unwrap();
    let r = view
        .render(&enc, state.clone(), &mut RunCache::new(false), now())
        .unwrap();
    let clicks: Vec<_> = r.spans.iter().filter(|s| s.label == "click").collect();
    assert_eq!(clicks.len(), 1, "hidden rows must not expose handlers");
    assert_eq!(
        enc.tok.decode(&r.ids[clicks[0].start..clicks[0].end], false).unwrap(),
        "last row with several words\n"
    );
    assert!(r.spans.iter().all(|s| s.start <= s.end && s.end <= r.ids.len()));
    let next = view.act(&enc, state, now(), "click", 0, &json!({})).unwrap();
    assert_eq!(next.state["selected"], "last");
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn drop_children_never_clips_chat_control_tokens() {
    let enc = enc();
    let dir = std::env::temp_dir().join(format!("afr_review_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("children.tsx");
    std::fs::write(
        &path,
        r#"
"use strict_tokens";
const Turns = ({ messages }) => messages.map(m => p`${tok.im_start}user\n${m}${tok.im_end}\n`);
export default function Main({ messages, budget }) {
  return <Turns messages={messages} style={{ maxTokens: budget, overflow: "drop-children" }}/>;
}"#,
    )
    .unwrap();
    let view = View::load(&path, "Main", &enc).unwrap();
    for budget in [0, 1, 8, 32, 128] {
        let state = state_from_json(
            &json!({"messages":["small", "very long content ".repeat(100)], "kwargs":{"budget":budget}}),
            &json!({}),
        );
        let r = view.render(&enc, state, &mut RunCache::new(false), now()).unwrap();
        assert!(r.ids.len() <= budget);
        // The last item is larger than every tested budget, so all items disappear.
        assert!(r.ids.is_empty(), "whole children, no partial turn at budget {budget}");
        assert_eq!(r.spans[0].meta["dropped_items"], 2);
    }
}
