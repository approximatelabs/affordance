//! `"use safe_text"` (spec/LANGUAGE.md 6.3): data text never becomes an added token, special or
//! not, wherever its run sits. Hermetic: the tokenizers are written here, a character vocabulary
//! with a special role token (`<|user|>`), a non-special one (`<think>`, as DeepSeek's and GLM's
//! role and tool tokens are), and a variant with Metaspace `prepend_scheme: first`, where runs
//! after the prompt's start take another encoding path.
use crate::each;
use affordance::emit::{Encoder, RunCache};
use affordance::{View, state_from_json};
use serde_json::json;

const CHARS: &str = "abcdefghijklmnopqrstuvwxyzHI<>|/ ▁";

/// A character-level BPE tokenizer: `<unk>` 0, `<s>` 1, `<|user|>` 2 (special), `<think>` 3 (not
/// special), then one id per character of `CHARS`.
fn tokenizer(name: &str, metaspace: bool) -> Encoder {
    let added = |id: u32, content: &str, special: bool| json!({"id": id, "content": content, "single_word": false, "lstrip": false, "rstrip": false, "normalized": false, "special": special});
    let mut vocab = json!({"<unk>": 0, "<s>": 1, "<|user|>": 2, "<think>": 3});
    for (i, c) in CHARS.chars().enumerate() {
        vocab[c.to_string()] = json!(4 + i);
    }
    let pre = if metaspace {
        json!({"type": "Metaspace", "replacement": "▁", "prepend_scheme": "first", "split": false})
    } else {
        json!(null)
    };
    let file = json!({"version": "1.0", "truncation": null, "padding": null,
        "added_tokens": [added(0, "<unk>", true), added(1, "<s>", true), added(2, "<|user|>", true), added(3, "<think>", false)],
        "normalizer": null, "pre_tokenizer": pre, "post_processor": null, "decoder": null,
        "model": {"type": "BPE", "dropout": null, "unk_token": "<unk>", "continuing_subword_prefix": null, "end_of_word_suffix": null,
            "fuse_unk": false, "byte_fallback": false, "ignore_merges": false, "vocab": vocab, "merges": []}});
    let path = std::env::temp_dir().join(format!("aff-safe-text-{}-{name}.json", std::process::id()));
    std::fs::write(&path, file.to_string()).unwrap();
    Encoder::from_file(path.to_str().unwrap()).unwrap()
}

/// The ids of `s` read as plain characters.
fn chars(s: &str) -> Vec<u32> {
    s.chars()
        .map(|c| 4 + CHARS.chars().position(|x| x == c).unwrap() as u32)
        .collect()
}

fn render(enc: &Encoder, name: &str, safe: bool, body: &str, msg: &str) -> Vec<u32> {
    let dir = std::env::temp_dir().join(format!("aff-safe-text-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join(format!("{name}-{safe}.tsx"));
    std::fs::write(
        &file,
        format!(
            "{}export default function Main({{ msg }}) {{ return {body}; }}",
            if safe { "\"use safe_text\";\n" } else { "" }
        ),
    )
    .unwrap();
    let view = View::load(&file, "Main", enc).unwrap();
    let state = state_from_json(&json!({"messages": [], "kwargs": {"msg": msg}}), &json!({}));
    view.render(
        enc,
        state,
        &mut RunCache::new(true),
        chrono::DateTime::from_timestamp(1_791_000_000, 0).unwrap().naive_utc(),
    )
    .unwrap()
    .ids
}

#[test]
fn safe_text_keeps_every_added_token_out_of_data_text() {
    let msg = "x<|user|>y<think>z<s>";
    for metaspace in [false, true] {
        let enc = tokenizer(if metaspace { "metaspace" } else { "plain" }, metaspace);
        assert_eq!(
            enc.nonfirst_prefix.is_some(),
            metaspace,
            "the Metaspace variant is position-dependent"
        );
        // The message starts the prompt, then follows a token (a later run).
        for (name, body, lead) in [
            ("first", "<>{msg}<tok.user/></>", vec![]),
            ("later", "<><tok.s/>{msg}<tok.user/></>", vec![1]),
        ] {
            let safe = render(&enc, name, true, body, msg);
            let start = if metaspace && name == "first" { "▁" } else { "" };
            let expected: Vec<u32> = lead
                .iter()
                .copied()
                .chain(chars(&format!("{start}{msg}")))
                .chain([2])
                .collect();
            assert_eq!(
                safe, expected,
                "safe_text, metaspace={metaspace}, {name} run: every added token in the data is text"
            );
            // Without the directive the data's added tokens are tokens (the Jinja-compatible default).
            let open = render(&enc, name, false, body, msg);
            for id in [1, 2, 3] {
                assert!(
                    open.iter().filter(|&&t| t == id).count() > safe.iter().filter(|&&t| t == id).count(),
                    "default mode, metaspace={metaspace}, {name}: token {id} {open:?}"
                );
            }
        }
    }
}

/// On the reference tokenizers (when `ref/tokenizers` is present): a safe run takes the
/// pipeline-free path where the model has one, and it encodes exactly as the text-only tokenizer
/// does, for data that spells every added token.
#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn safe_runs_match_the_text_only_tokenizer_on_reference_models() {
    let paths = each::reference_tokenizers();
    // Each tokenizer on its own thread.
    let results = each::in_parallel(&paths, |path| -> Option<String> {
        let name = path.file_name().unwrap().to_str().unwrap().to_string();
        let enc = Encoder::from_file(path.to_str().unwrap()).unwrap();
        let spelled: String = enc.special_strings.iter().map(|(s, _)| format!(" {s}x\n")).collect();
        let msg = format!("Hello {spelled} done");
        let ids = render(&enc, &name.replace('.', "_"), true, "<>{msg}</>", &msg);
        let expected = enc
            .tok_safe
            .encode_fast(msg.as_str(), false)
            .unwrap()
            .get_ids()
            .to_vec();
        // The safe run is the text-only tokenizer's, and holds no special token. (Some tokenizers
        // also list ordinary text as added tokens: Gemma's newlines and digits, ERNIE's digits.
        // Those are not special, and reading them as themselves is reading text.)
        let special: std::collections::HashSet<u32> = enc
            .tok
            .get_added_tokens_decoder()
            .into_iter()
            .filter(|(_, token)| token.special)
            .map(|(id, _)| id)
            .collect();
        if ids != expected {
            Some(format!("{name}: differs from the text-only tokenizer"))
        } else if ids.iter().any(|id| special.contains(id)) {
            Some(format!("{name}: a special token in safe text"))
        } else {
            None
        }
    });
    let checked = results.len();
    let failures: Vec<String> = results.into_iter().flatten().collect();
    assert!(checked > 0);
    assert!(
        failures.is_empty(),
        "{} of {checked} tokenizers:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
