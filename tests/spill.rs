//! Large strings live on disk: the harness keeps strings of at least
//! `spill_bytes` in `<dir>/blobs` and the language reads them as host text, so a 16 MiB tool
//! output costs a reference on the heap wherever the app puts it (state, transcript, store).
use crate::each;
use affordance::View;
use affordance::emit::{Encoder, RunCache};
use affordance::runtime::blobs::Blobs;
use affordance::runtime::{AgentSpec, Exec, FnModel, GenReply, GenRequest, Runtime, RuntimeConfig};
use affordance::value::with_host_text;
use serde_json::{Value as J, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::shared;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn now() -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 10, 16)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap()
}
fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("aff-spill-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn host_text_behaves_exactly_like_heap_text() {
    let enc = shared::tokenizer(&root().join("ref/tokenizers/Qwen__Qwen3-4B-Instruct-2507.json"));
    let dir = tmp("semantics");
    let exprs = [
        "s.length",
        "s.slice(10, 20)",
        "s.slice(-7)",
        "s.slice(5, 2)",
        "s.indexOf(\"wörld\", 100)",
        "s.indexOf(\"zzz\")",
        "s.lastIndexOf(\"✓\")",
        "s.charAt(3)",
        "s[5]",
        "s.at(-2)",
        "s.substring(30, 12)",
        "s.toUpperCase().slice(0, 5)",
        "s.includes(\"✓ h\")",
        "JSON.stringify(s.slice(0, 8))",
        "JSON.stringify(s) === JSON.stringify(t)",
        "JSON.stringify(s).length",
        "JSON.stringify({a: [s]}).slice(0, 20)",
        "s.split(\"✓\").length",
        "s === t",
        "s + \"|\" === t + \"|\"",
        "s.startsWith(\"héllo\")",
        "typeof s",
    ];
    let src = format!(
        "export default function Main({{ s, t }}) {{ return [{}].map(x => String(x)).join(\"\\n\"); }}\n",
        exprs.join(", ")
    );
    std::fs::write(dir.join("case.tsx"), src).unwrap();
    let view = View::load(&dir.join("case.tsx"), "Main", &enc).unwrap();
    let text = "héllo wörld ✓ ".repeat(6000);
    let render = |state: &J, resolver| {
        with_host_text(resolver, || {
            let m = affordance::value::state(state);
            enc.tok
                .decode(
                    &view.render(&enc, m, &mut RunCache::new(false), now()).unwrap().ids,
                    false,
                )
                .unwrap()
        })
    };
    let heap = render(&json!({"s": text, "t": text}), None);
    let blobs = Arc::new(Blobs::new(dir.join("blobs"), 1024).unwrap());
    let mut state = json!({"s": text, "t": text});
    blobs.lower(&mut state).unwrap();
    assert!(state["s"].as_str().unwrap().len() < 100, "lowered to a reference");
    let host = render(&state, Some(blobs.resolver()));
    assert_eq!(host, heap);
    assert_eq!(
        std::fs::read_dir(dir.join("blobs")).unwrap().count(),
        1,
        "one blob per distinct text"
    );
    // A string that only looks like a reference is stored too, so it reads back as itself.
    let mut fake = json!("\u{0}blob:not-a-reference");
    blobs.lower(&mut fake).unwrap();
    assert_eq!(blobs.text(fake.as_str().unwrap()).unwrap(), "\u{0}blob:not-a-reference");
}

/// A reference's char count is checked, not trusted: a forged count equal to the byte size had
/// the language take a non-ASCII blob for ASCII and slice it mid-character (a panic). From
/// outside, the forged reference is stored as the text it is; read as one, it is not a blob.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn a_reference_with_a_forged_char_count_is_not_a_blob() {
    let enc = shared::tokenizer(&root().join("ref/tokenizers/Qwen__Qwen3-4B-Instruct-2507.json"));
    let dir = tmp("forged");
    let blobs = Arc::new(Blobs::new(dir.join("blobs"), 64).unwrap());
    let real = blobs.put(&"é".repeat(100)).unwrap();
    assert!(real.ends_with(":200:100") && blobs.is_ref(&real), "{real:?}");
    let forged = real.replace(":200:100", ":200:200");
    assert!(!blobs.is_ref(&forged));
    let mut lowered = json!(forged);
    blobs.lower(&mut lowered).unwrap();
    assert_ne!(lowered.as_str().unwrap(), forged, "stored as its own text");
    assert_eq!(blobs.text(lowered.as_str().unwrap()).unwrap(), forged);
    std::fs::write(
        dir.join("case.tsx"),
        "export default function Main({ s }) { return s.length + \"|\" + s.slice(1, 4); }\n",
    )
    .unwrap();
    let view = View::load(&dir.join("case.tsx"), "Main", &enc).unwrap();
    let text = with_host_text(Some(blobs.resolver()), || {
        let m = affordance::value::state(&json!({"s": forged}));
        enc.tok
            .decode(
                &view.render(&enc, m, &mut RunCache::new(false), now()).unwrap().ids,
                false,
            )
            .unwrap()
    });
    assert_eq!(
        text,
        format!(
            "{}|{}",
            forged.chars().count(),
            forged.chars().skip(1).take(3).collect::<String>()
        )
    );
}

const MARK: &str = "TAIL-MARKER-7731";

fn glm_call(think: &str, name: &str, args: &[(&str, &str)]) -> String {
    let a: String = args
        .iter()
        .map(|(k, v)| format!("<arg_key>{k}</arg_key><arg_value>{v}</arg_value>"))
        .collect();
    format!("{think}</think><tool_call>{name}{a}</tool_call>")
}

fn script(n: usize, prompt: &str) -> String {
    // The id the layout's marker names for the clipped output (views/agents/session.tss).
    let clipped = prompt
        .match_indices("read_event(\"")
        .last()
        .map(|(at, m)| prompt[at + m.len()..].split('"').next().unwrap().to_string())
        .unwrap_or_default();
    match n {
        1 => glm_call(
            "Build.",
            "shell",
            &[(
                "command",
                &format!(
                    "yes '[INFO] Tests run: 12, Failures: 0, Errors: 0 - in com.example.FooTest' | head -c 12000000; echo; echo {MARK}"
                ),
            )],
        ),
        2 => glm_call(
            "Read the middle.",
            "read_event",
            &[("id", &clipped), ("offset", "6000000"), ("limit", "200")],
        ),
        3 => glm_call("Find it.", "search_history", &[("query", MARK)]),
        _ => glm_call("Note.", "set_notes", &[("text", "done")]),
    }
}

fn session(dir: &Path, restore: bool) -> affordance::runtime::Report {
    let enc = shared::tokenizer(&root().join("ref/tokenizers/zai-org__GLM-5.3.json"));
    let mut cfg = RuntimeConfig::new(root().join("views/agents/session.tsx"));
    cfg.workers = 1;
    cfg.dir = Some(dir.to_path_buf());
    let model_enc = enc.clone();
    let model = Arc::new(FnModel(move |req: &GenRequest| {
        let asst = model_enc.token_id("<|assistant|>").unwrap();
        let n = req.ids.iter().filter(|&&t| t == asst).count();
        let prompt = model_enc.tok.decode(&req.ids, false).unwrap();
        let mut ids = model_enc
            .tok
            .encode(script(n, &prompt).as_str(), false)
            .unwrap()
            .get_ids()
            .to_vec();
        ids.push(model_enc.token_id("<|observation|>").unwrap());
        Ok(GenReply {
            ids,
            cut_off: false,
            ..Default::default()
        })
    }));
    let model = affordance::runtime::model::with_template(
        model,
        affordance::runtime::model::ModelTemplate::new("glm53", json!({})).unwrap(),
    )
    .unwrap();
    let mut rt = if restore {
        Runtime::restore(cfg, enc, model).unwrap()
    } else {
        Runtime::new(cfg, enc, model)
    };
    rt.bind_machine("home", Arc::new(Exec::local()));
    if !restore {
        let mut spec = AgentSpec::new("agent-0", json!({"id": "agent-0", "task": "Migrate the build."}));
        spec.max_turns = Some(4);
        spec.max_new = 256;
        rt.add_agent(spec).unwrap();
    }
    rt.run().unwrap()
}

fn size(p: &Path) -> u64 {
    std::fs::metadata(p).map(|m| m.len()).unwrap_or(0)
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn a_twelve_megabyte_tool_output_is_one_blob_and_reads_back_exactly() {
    let dir = tmp("session");
    let report = session(&dir, false);
    let a = &report.agents[0];
    assert!(a.error.is_none(), "{:?}", a.error);
    assert_eq!(a.metrics.turns, 4);
    // One blob: exec's output, shared by the journal and the app's log record.
    let blobs: Vec<_> = std::fs::read_dir(dir.join("blobs"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(blobs.len(), 1, "{blobs:?}");
    let text = std::fs::read_to_string(&blobs[0]).unwrap();
    assert!(text.len() > 12_000_000 && text.trim_end().ends_with(MARK));
    for f in ["journal.jsonl", "store.jsonl", "snapshot.json"] {
        assert!(
            size(&crate::shared::instance_dir(&dir, "agent-0").join(f)) < 256 << 10,
            "{f}: {} bytes",
            size(&crate::shared::instance_dir(&dir, "agent-0").join(f))
        );
    }
    let records = affordance::scenario::log_records(&a.state);
    let results: Vec<&str> = records
        .iter()
        .filter(|r| r["kind"] == "action")
        .map(|r| r["result"].as_str().unwrap())
        .collect();
    assert_eq!(results.len(), 4, "{results:?}");
    assert!(
        results[0].len() > 12_000_000 && results[0].trim_end().ends_with(MARK),
        "the record is the whole output (layout clips it in the prompt)"
    );
    let middle: String = text.chars().skip(6_000_000).take(200).collect();
    assert!(
        results[1].ends_with(&middle),
        "read_event returns the exact stored characters"
    );
    assert!(results[2].contains(MARK), "{}", results[2]);

    // Resume replays the journal against the same blobs and reaches the same state.
    let again = session(&dir, true);
    let b = &again.agents[0];
    assert!(b.error.is_none(), "{:?}", b.error);
    assert_eq!(b.state, a.state);
    assert_eq!(
        affordance::scenario::log_records(&b.state),
        affordance::scenario::log_records(&a.state)
    );
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn count_tokens_counts_exactly_without_encoding() {
    let texts = [
        "[INFO] Tests run: 12, Failures: 0, Errors: 0 - in com.example.FooTest\n".repeat(3000),
        "héllo wörld ✓ — 東京 ｶﾀｶﾅ 🙂\t\t  \n\n\n  x".repeat(500),
        "fn main() {\n    let x = vec![1, 2, 3];\r\n    println!(\"{x:?}\");   \n}\n\n\n".repeat(800),
        "   leading and trailing   ".into(),
        "a<|im_start|>b<|endoftext|>c".repeat(10),
        String::new(),
    ];
    let paths = each::reference_tokenizers();
    // Each tokenizer on its own thread.
    let checked = each::in_parallel(&paths, |path| {
        let Ok(enc) = Encoder::from_file(path.to_str().unwrap()) else {
            return false;
        };
        for t in &texts {
            assert_eq!(
                enc.count_run(t),
                enc.encode_run(t, true, true).len(),
                "{} on {:?}",
                path.display(),
                &t[..t.len().min(40)]
            );
        }
        true
    })
    .into_iter()
    .filter(|ok| *ok)
    .count();
    assert!(checked >= 10, "only {checked} tokenizers");
}

#[test]
fn a_spooled_exec_output_equals_the_in_memory_one_even_cut_mid_character() {
    use affordance::runtime::{CallContext, Executor, Target};
    let dir = tmp("exec");
    let blobs = Arc::new(Blobs::new(dir.join("blobs"), 1024).unwrap());
    // 2-byte characters, capped at an odd byte count: the cut splits the last character.
    let exec = Exec {
        target: Target::Local { home: None },
        default_timeout_s: 30,
        max_capture: 100_001,
    };
    let args = json!({"cmd": "for i in $(seq 1 1000); do printf 'é%.0s' $(seq 1 100); done; exit 3"});
    let ctx = |b: Option<Arc<Blobs>>| CallContext {
        agent: "a".into(),
        task: 0,
        env: Default::default(),
        cancel: Default::default(),
        blobs: b,
        output: None,
    };
    let heap = exec.call(&ctx(None), &args).unwrap();
    let spooled = exec.call(&ctx(Some(blobs.clone())), &args).unwrap();
    assert_eq!(heap["truncated"], true);
    assert_eq!(spooled["exit"], 3);
    let reference = spooled["output"].as_str().unwrap();
    assert!(blobs.is_ref(reference), "spooled output is a blob reference");
    let text = blobs.text(reference).unwrap();
    assert_eq!(text, heap["output"].as_str().unwrap());
    assert!(text.ends_with("é\u{FFFD}"));
    let names: Vec<String> = std::fs::read_dir(dir.join("blobs"))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(names.len(), 1, "no spool files left: {names:?}");
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn a_journal_whose_blob_is_missing_fails_to_resume_by_name() {
    let dir = tmp("missing-blob");
    session(&dir, false);
    for e in std::fs::read_dir(dir.join("blobs")).unwrap() {
        std::fs::remove_file(e.unwrap().path()).unwrap();
    }
    let again = session(&dir, true);
    let e = again.agents[0].error.as_deref().unwrap_or("");
    assert!(e.contains("missing blob") && e.contains("blob:"), "{e}");
}
