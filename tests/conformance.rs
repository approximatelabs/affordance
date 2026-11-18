//! Conformance: every view must reproduce its reference token ids (or raise where the
//! reference raises) on every case set present in ref/ (dev, heldout1..3). Skips a set if its
//! references have not been generated (ref/render_ref*.py).

use std::process::Command;

/// The six case sets in one run of the command (each tokenizer is loaded once): the hand-written
/// views, or, under `tools/check-models.sh` (`AFFORDANCE_ALL_MODELS`), every model.
const SETS: &str = "dev,heldout1,heldout2,heldout3,heldout_a2,agent_loops";

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn conformance_cheap_tier_families() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let refs = root.join("ref/new_out");
    assert!(
        refs.exists(),
        "reference data: ref/new_out is missing (docs/DEVELOPING.md)"
    );
    let family_set = |set: &str| {
        let set = format!("heldout_{}", set);
        // Expected family membership comes from the declared reference-model inventory,
        // never from whichever outputs happened to download successfully.
        let family = set.strip_prefix("heldout_").unwrap();
        let family = if family == "openai" { "gptoss" } else { family };
        let models = std::fs::read_to_string(root.join("ref/models.txt"))
            .unwrap()
            .lines()
            .filter_map(|line| {
                let (tag, repo) = line.split_once('|')?;
                (tag.trim() == family).then(|| repo.trim().replace('/', "__"))
            })
            .collect::<Vec<_>>();
        assert!(!models.is_empty());
        let selected = models.join(",");
        let out = Command::new(env!("CARGO_BIN_EXE_affordance"))
            .args([
                "check",
                "--set",
                &set,
                "--only",
                &selected,
                "--ref-dir",
                refs.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}: {}\n{}",
            set,
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let table: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(root.join(format!(
                "target/conformance/check_{}{}.json",
                set,
                if models.len() > 1 {
                    "_selected".to_string()
                } else {
                    format!("_{}", models[0])
                }
            )))
            .unwrap(),
        )
        .unwrap();
        let model_count = models.len();
        assert_eq!(
            table.as_object().unwrap().len(),
            model_count,
            "all declared models checked"
        );
        assert!(
            table
                .as_object()
                .unwrap()
                .values()
                .all(|row| row.as_object().unwrap().len() == 60)
        );
    };
    // The families are independent runs of the command: side by side.
    std::thread::scope(|scope| {
        for set in [
            "deepseek", "glm", "kolibri", "llama", "mimo", "mistral", "muse", "openai", "qwen",
        ] {
            scope.spawn(|| family_set(set));
        }
    });
}

/// Every case set (the standard cases, the held-out sets, the agent loops): the same ids as the
/// official template, or the same kind of error, on every case.
#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn conformance_on_every_case_set() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for set in SETS.split(',') {
        assert!(
            root.join(format!("ref/out/{set}")).exists(),
            "reference data: ref/out/{set} is missing (docs/DEVELOPING.md)"
        );
    }
    let mut command = Command::new(env!("CARGO_BIN_EXE_affordance"));
    command.args(["check", "--set", SETS]);
    if std::env::var("AFFORDANCE_ALL_MODELS").is_ok() {
        command.args(["--threads", "96"]);
    } else {
        command.args(["--tier", "hand"]);
    }
    let out = command.output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "check command failed\n{text}");
    let totals: Vec<&str> = text.lines().filter(|l| l.starts_with("TOTAL")).collect();
    assert_eq!(totals.len(), SETS.split(',').count(), "{text}");
    for total in totals {
        assert!(
            total.contains("mismatch 0, unexpected-error 0, missing-error 0"),
            "{total}\n{text}"
        );
    }
    assert!(!text.contains("LOAD ERROR"), "{text}");
}

/// A dozen converted views, one per kind of template the converter meets (two named templates,
/// a dict with number keys, %-formatting, macros, image parts), on the standard cases. All of
/// them, on every set, are `tools/check-models.sh`.
#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn converted_views_match_their_official_templates() {
    let sample = "google__gemma-4-31B-it,google__gemma-3-4b-it,microsoft__phi-4,NousResearch__Hermes-3-Llama-3.1-8B,ByteDance-Seed__Seed-OSS-36B-Instruct,naver-hyperclovax__HyperCLOVAX-SEED-Think-32B,ibm-granite__granite-4.1-3b,MiniMaxAI__MiniMax-M2.5,LiquidAI__LFM2.5-350M,nvidia__NVIDIA-Nemotron-3-Nano-4B-BF16,tencent__Hunyuan-A13B-Instruct,llava-hf__llava-1.5-7b-hf";
    let out = Command::new(env!("CARGO_BIN_EXE_affordance"))
        .args(["check", "--set", "dev", "--tier", "converted", "--only", sample])
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{text}");
    assert!(text.contains("Reference coverage: 12 selected models"), "{text}");
    assert!(
        text.lines()
            .last()
            .unwrap_or("")
            .contains("mismatch 0, unexpected-error 0, missing-error 0"),
        "{text}"
    );
}
