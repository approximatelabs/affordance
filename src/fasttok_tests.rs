//! Equality proof for the fast pre-tokenizer: for every model with a fast plan and
//! every case whose reference ids are available, the `regex`-based split must produce
//! exactly the same pretokenizer pieces as the original onig `Split`, and the full
//! `encode_fast` must give the same ids. Models are checked on their own thread.

use super::{FastTokenizer, split_pieces};
use serde_json::json;
use std::path::{Path, PathBuf};
use tokenizers::Tokenizer;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn check_model(path: &Path, sets: &[&str]) -> Result<usize, String> {
    let name = path.file_stem().unwrap().to_str().unwrap().to_string();
    let fast = FastTokenizer::from_file(path).map_err(|e| e.to_string())?;
    if !fast.get_pre_tokenizer().map(|p| p.fast()).unwrap_or(false) {
        return Ok(0);
    }
    let orig = Tokenizer::from_file(path).map_err(|e| e.to_string())?;
    let mut compared = 0usize;
    for set in sets {
        let refp = root().join(format!("ref/out/{}/{}.json", set, name));
        if !refp.exists() {
            continue;
        }
        let refj: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&refp).unwrap()).unwrap();
        for (case, v) in refj.as_object().unwrap() {
            let ids: Vec<u32> = match v.get("ids").and_then(|x| x.as_array()) {
                Some(a) => a.iter().map(|x| x.as_u64().unwrap() as u32).collect(),
                None => continue,
            };
            let text = v["text"]
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| orig.decode(&ids, false).unwrap_or_default());
            if text.is_empty() {
                continue;
            }
            let want = split_pieces(orig.get_pre_tokenizer().unwrap(), &text);
            let got = split_pieces(fast.get_pre_tokenizer().unwrap(), &text);
            if want != got {
                return Err(format!("{} {}/{}: split mismatch", name, set, case));
            }
            let a = orig.encode_fast(text.as_str(), false).unwrap().get_ids().to_vec();
            let b = fast.encode_fast(text.as_str(), false).unwrap().get_ids().to_vec();
            if a != b {
                return Err(format!("{} {}/{}: id mismatch", name, set, case));
            }
            compared += 1;
        }
    }
    Ok(compared)
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn fast_splits_match_onig_on_case_texts() {
    let sets = ["dev", "heldout1", "heldout2", "heldout3"];
    let mut paths = vec![];
    for entry in std::fs::read_dir(root().join("ref/tokenizers")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if path.file_stem().unwrap().to_str().unwrap().ends_with(".special") {
            continue;
        }
        paths.push(path);
    }
    // The tokenizers of the models with hand-written views; under tools/check-models.sh
    // (AFFORDANCE_ALL_MODELS), every one.
    if std::env::var("AFFORDANCE_ALL_MODELS").is_err() {
        let index: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(root().join("views/index.json")).unwrap()).unwrap();
        paths.retain(|path| {
            let model = path.file_stem().unwrap().to_str().unwrap();
            index
                .get(model)
                .is_none_or(|entry| entry["converted"] != true && !entry["shared"].is_string())
        });
    }
    // A thread for each model, at most 64 at a time.
    let mut results = vec![];
    for batch in paths.chunks(64) {
        results.extend(std::thread::scope(|scope| {
            let handles: Vec<_> = paths
                .iter()
                .filter(|p| batch.contains(p))
                .map(|p| scope.spawn(move || check_model(p, &sets)))
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect::<Vec<_>>()
        }));
    }
    let mut compared = 0usize;
    for r in results {
        compared += r.unwrap();
    }
    assert!(compared > 1000, "compared only {} texts", compared);
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn unknown_split_patterns_keep_original_behavior() {
    let mut j: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(root().join("ref/tokenizers/Qwen__Qwen3-8B.json")).unwrap())
            .unwrap();
    j["pre_tokenizer"]["pretokenizers"][0]["pattern"]["Regex"] = json!(r"\s+");
    let s = j.to_string();
    let fast = FastTokenizer::from_bytes(s.as_bytes()).unwrap();
    let orig = tokenizers::Tokenizer::from_bytes(s.as_bytes()).unwrap();
    assert_eq!(
        split_pieces(fast.get_pre_tokenizer().unwrap(), "a   b"),
        split_pieces(orig.get_pre_tokenizer().unwrap(), "a   b")
    );
    // Nullable regexes must fall back, avoiding a non-progressing manual scanner.
    j["pre_tokenizer"]["pretokenizers"][0]["pattern"]["Regex"] = json!("x*");
    let fast = FastTokenizer::from_bytes(j.to_string().as_bytes()).unwrap();
    assert!(!fast.get_pre_tokenizer().unwrap().fast());
}
