//! A machine call's `stdin`: the bytes reach the command as its standard input exactly, a command
//! that does not read them never runs them as script, and from an app a large input travels as a
//! blob reference and is written to disk once (the journal holds only the result), as `fetch` sends a Blob body.
use affordance::emit::Encoder;
use affordance::runtime::{
    AgentSpec, CallContext, Exec, Executor, FnModel, GenReply, GenRequest, Runtime, RuntimeConfig, Target,
};
use serde_json::{Value as J, json};
use std::path::PathBuf;
use std::sync::Arc;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("aff-stdin-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}
fn ctx() -> CallContext {
    CallContext {
        agent: "a".into(),
        task: 0,
        env: Default::default(),
        cancel: Default::default(),
        blobs: None,
        output: None,
    }
}
fn exec() -> Exec {
    Exec {
        target: Target::Local { home: None },
        default_timeout_s: 30,
        max_capture: 16 << 20,
    }
}
fn run(args: J) -> J {
    exec().call(&ctx(), &args).unwrap()
}

#[test]
fn the_command_reads_its_input_exactly_and_the_input_is_never_script() {
    let input = "line one\nexit 7\n'quoted' \"too\" $HOME `date`\n✓ wörld\nno newline at the end";
    let r = run(json!({"cmd": "cat", "stdin": input}));
    assert_eq!(r["output"], input);
    assert_eq!(r["exit"], 0);
    // A command that ignores its input: the input is not run.
    let r = run(json!({"cmd": "echo ignored", "stdin": "echo RAN; exit 9\n"}));
    assert_eq!(r["output"], "ignored\n");
    assert_eq!(r["exit"], 0);
    // A command the shell cannot parse, or one that contains what looks like the end of the
    // script: the input is still data, never run.
    let r = run(json!({"cmd": "echo 'unclosed", "stdin": "echo RAN; exit 9\n"}));
    assert!(
        !r["output"].as_str().unwrap().contains("RAN") && r["exit"] != 9 && r["exit"] != 0,
        "{r}"
    );
    let r = run(json!({"cmd": "cat <<X\n__AFF_COMMAND_0\nX\ncat", "stdin": "data\n"}));
    assert_eq!(r["output"], "__AFF_COMMAND_0\ndata\n");
    // A command over several lines, which reads part of its input, with env and an exit status.
    let r = run(
        json!({"cmd": "read first\necho \"$first/$GREETING\"\nwc -l | tr -d ' '\nexit 3", "stdin": "a\nb\nc\n", "env": {"GREETING": "hi"}}),
    );
    assert_eq!(r["output"], "a/hi\n2\n");
    assert_eq!(r["exit"], 3);
    // An empty input is an empty stream, not a missing one.
    let r = run(json!({"cmd": "wc -c | tr -d ' '", "stdin": ""}));
    assert_eq!(r["output"], "0\n");
    // Without stdin, a command reads nothing.
    let r = run(json!({"cmd": "cat; echo end"}));
    assert_eq!(r["output"], "end\n");
    assert!(
        exec()
            .call(&ctx(), &json!({"cmd": "cat", "stdin": 5}))
            .unwrap_err()
            .contains("stdin must be a string")
    );
}

#[test]
fn a_large_input_streams_through() {
    let input: String = (0..400_000).map(|i| format!("{i} é\n")).collect();
    let r = run(json!({"cmd": "wc -c | tr -d ' '; echo done", "stdin": input}));
    assert_eq!(r["output"], format!("{}\ndone\n", input.len()));
}

/// From an app: a large input is one blob on disk, never in the journal; the command gets its bytes.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs the test tokenizers: tools/fetch-test-tokenizers.sh")]
fn an_apps_large_input_is_a_blob_reference_in_the_journal() {
    let dir = tmp("app");
    let app = dir.join("app.tsx");
    std::fs::write(
        &app,
        r#"export default function Main() {
  useEffect(() => {
    (async () => {
      const lines = [];
      for (let i = 0; i < 20000; i++) lines.push(`{"agent": "agent-${i}", "turns": ${i % 7}}`);
      const r = await harness.machine.call({cmd: "wc -l | tr -d ' '", stdin: lines.join("\n") + "\n"});
      harness.stop(r.output.trim());
    })();
  }, []);
  return <text>{"waiting"}</text>;
}"#,
    )
    .unwrap();
    let enc =
        Arc::new(Encoder::from_file(root().join("ref/tokenizers/Qwen__Qwen3-8B.json").to_str().unwrap()).unwrap());
    let mut cfg = RuntimeConfig::new(&app);
    cfg.dir = Some(dir.join("run"));
    cfg.spill_bytes = 64 << 10;
    let model = Arc::new(FnModel(|_: &GenRequest| Ok(GenReply::default())));
    let mut rt = Runtime::new(cfg, enc, model);
    rt.bind_machine("home", Arc::new(exec()));
    let mut spec = AgentSpec::new("a", json!({}));
    spec.model = None;
    rt.add_agent(spec).unwrap();
    let report = rt.run().unwrap();
    assert_eq!(
        report.agents[0].stopped,
        Some(json!("20000")),
        "{:?}",
        report.agents[0].error
    );
    // The journal records the call's result (its arguments are recomputed on replay); the input
    // crossed as a blob reference, so it sat on disk once, not on the heap.
    let journal = std::fs::read_to_string(dir.join("run/a/journal.jsonl")).unwrap();
    assert!(journal.len() < 1024, "{journal}");
    let blobs: Vec<_> = std::fs::read_dir(dir.join("run/blobs"))
        .unwrap()
        .map(|e| std::fs::read_to_string(e.unwrap().path()).unwrap())
        .collect();
    assert_eq!(blobs.len(), 1);
    assert!(blobs[0].starts_with("{\"agent\": \"agent-0\"") && blobs[0].lines().count() == 20000);
}

/// A command's output reaches the call's `output` sink while it runs, before the command exits;
/// the result is the same.
#[test]
fn output_is_handed_out_as_the_command_writes_it() {
    let chunks = Arc::new(std::sync::Mutex::new(Vec::<(std::time::Instant, Vec<u8>)>::new()));
    let sink = chunks.clone();
    let mut context = ctx();
    context.output = Some(affordance::runtime::Output(Arc::new(move |bytes: &[u8]| {
        sink.lock().unwrap().push((std::time::Instant::now(), bytes.to_vec()));
    })));
    let started = std::time::Instant::now();
    let result = exec()
        .call(&context, &json!({"cmd": "echo first; sleep 0.5; echo second"}))
        .unwrap();
    let ended = std::time::Instant::now();
    assert_eq!(result["output"], "first\nsecond\n");
    let chunks = chunks.lock().unwrap();
    let all: Vec<u8> = chunks.iter().flat_map(|(_, b)| b.clone()).collect();
    assert_eq!(all, b"first\nsecond\n");
    let (at, first) = &chunks[0];
    assert_eq!(first, b"first\n");
    assert!(*at - started < ended - started - std::time::Duration::from_millis(300));
}
