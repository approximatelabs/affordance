//! Resume from a recovery image of a loop agent: `views/agents/session.tsx`
//! is imaged at its step boundaries, a run stopped in the middle of a step leaves an image of
//! the last boundary, and restoring from it replays only that step's inputs and reaches exactly the
//! state a full replay of the same journal reaches.
use affordance::runtime::{FnModel, GenReply, GenRequest, InstanceSpec, Runtime, RuntimeConfig};
use serde_json::{Value as J, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use crate::shared;
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn glm_call(think: &str, name: &str, args: &[(&str, &str)]) -> String {
    let a: String = args
        .iter()
        .map(|(k, v)| format!("<arg_key>{k}</arg_key><arg_value>{v}</arg_value>"))
        .collect();
    format!("{think}</think><tool_call>{name}{a}</tool_call>")
}

/// Run session.tsx in `dir`. `hang_at`: the model call that never answers (the run then stops at
/// `max_wall`, in the middle of a step).
fn run(dir: &Path, restore: bool, hang_at: Option<usize>) -> affordance::runtime::Report {
    let enc = shared::tokenizer(&root().join("ref/tokenizers/zai-org__GLM-5.3.json"));
    let mut cfg = RuntimeConfig::new(root().join("views/agents/session.tsx"));
    cfg.workers = 1;
    cfg.dir = Some(dir.to_path_buf());
    // A hung run stops at this limit; the steps before the hang take milliseconds.
    cfg.max_wall = Some(if hang_at.is_some() {
        Duration::from_millis(1500)
    } else {
        Duration::from_secs(60)
    });
    let model_enc = enc.clone();
    let model = Arc::new(FnModel(move |req: &GenRequest| {
        let asst = model_enc.token_id("<|assistant|>").unwrap();
        let n = req.ids.iter().filter(|&&t| t == asst).count();
        if hang_at == Some(n) {
            std::thread::sleep(Duration::from_secs(30));
        }
        let text = if n % 3 == 0 {
            "Thinking.</think>Still working.".to_string()
        } else {
            glm_call("Run.", "shell", &[("command", &format!("echo step {n}"))])
        };
        let mut ids = model_enc.tok.encode(text.as_str(), false).unwrap().get_ids().to_vec();
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
    // A machine whose results do not depend on the wall clock, so two resumes can be compared.
    rt.bind_machine("home", Arc::new(|_: &affordance::runtime::CallContext, args: &J| {
        let cmd = args["cmd"].as_str().unwrap_or("");
        Ok(json!({"output": format!("{}\n", cmd.trim_start_matches("echo ")), "exit": 0, "seconds": 0.01, "timed_out": false, "truncated": false}))
    }));
    if !restore {
        let mut spec = InstanceSpec::new("agent-0", json!({"id": "agent-0", "task": "Keep going."}));
        spec.max_turns = Some(14);
        spec.max_new = 256;
        rt.add_instance(spec).unwrap();
    }
    rt.run().unwrap()
}

fn copy(from: &Path, to: &Path) {
    let _ = std::fs::remove_dir_all(to);
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        if e.file_type().unwrap().is_dir() {
            copy(&e.path(), &to.join(e.file_name()))
        } else {
            std::fs::copy(e.path(), to.join(e.file_name())).unwrap();
        }
    }
}

fn restores(dir: &Path) -> Vec<J> {
    std::fs::read_to_string(crate::shared::inst(dir, "agent-0/trace.jsonl"))
        .unwrap()
        .lines()
        .filter_map(|l| serde_json::from_str::<J>(l).ok())
        .filter(|t| t["t"] == "checkpoint_restore")
        .collect()
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn a_loop_agent_stopped_mid_step_resumes_from_its_last_step_boundary() {
    let base = std::env::temp_dir().join(format!("aff-restore-image-{}", std::process::id()));
    let (a, b) = (base.join("image"), base.join("full"));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&a).unwrap();
    // The model hangs on its 8th call: the run stops at max_wall in the middle of that step.
    let first = run(&a, false, Some(8));
    assert!(
        !first.complete,
        "{:?} {:?}",
        first.instances[0].stopped, first.instances[0].metrics
    );
    assert!(first.instances[0].error.is_none(), "{:?}", first.instances[0].error);
    let journal = first.instances[0].journal_len;
    let image: J = serde_json::from_slice(
        &std::fs::read(crate::shared::inst(&a, "agent-0/checkpoint.json")).expect("a loop agent is imaged"),
    )
    .unwrap();
    let inputs = image["data"]["image"]["journaled"].as_u64().unwrap() as usize;
    assert!(
        inputs > 0 && inputs <= journal && journal - inputs <= 2,
        "the image is the last step boundary: {inputs} of {journal}"
    );
    // A copy without the image resumes by full replay.
    copy(&a, &b);
    std::fs::remove_file(crate::shared::inst(&b, "agent-0/checkpoint.json")).unwrap();
    // Each resumes, runs, and stops in the middle of a later step; the image path writes a new
    // image from a boundary its replay found, and the next restore uses it.
    // (Side by side: each waits out its own hang.)
    std::thread::scope(|scope| {
        for d in [&a, &b] {
            scope.spawn(move || assert!(!run(d, true, Some(11)).complete));
        }
    });
    let second: J =
        serde_json::from_slice(&std::fs::read(crate::shared::inst(&a, "agent-0/checkpoint.json")).unwrap()).unwrap();
    let inputs_2 = second["data"]["image"]["journaled"].as_u64().unwrap() as usize;
    assert!(inputs_2 > inputs, "a newer image: {inputs_2} after {inputs}");
    let from_image = run(&a, true, None);
    let full = run(&b, true, None);
    for r in [&from_image, &full] {
        assert!(r.complete, "{:?}", r.instances[0].error);
    }
    let restore = restores(&a);
    assert_eq!(restore.len(), 2, "both resumes used an image: {restore:?}");
    assert_eq!(restore[0]["from_input"], json!(inputs));
    assert_eq!(restore[1]["from_input"], json!(inputs_2));
    assert!(
        restore.iter().all(|r| r["replayed_inputs"].as_u64().unwrap() <= 2),
        "{restore:?}"
    );
    // The copy replayed in full the first time; its second restore used an image from that replay.
    let copy_restores = restores(&b);
    assert_eq!(copy_restores.len(), 1, "{copy_restores:?}");
    assert_eq!(
        from_image.instances[0].state, full.instances[0].state,
        "image restore equals full replay"
    );
    assert_eq!(from_image.instances[0].metrics, full.instances[0].metrics);
    assert_eq!(from_image.instances[0].journal_len, full.instances[0].journal_len);
    std::fs::remove_dir_all(&base).unwrap();
}

/// A Promise used during render (settled before the render ends) leaves an agent as imageable as
/// one that uses none: each app is imaged when its run ends, and a restore from that image reaches
/// the state a full replay of the same journal reaches.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn a_promise_used_during_render_does_not_stop_recovery_images() {
    let base = std::env::temp_dir().join(format!("aff-render-promise-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    for (name, line) in [
        ("plain", ""),
        ("resolve", "Promise.resolve(1);"),
        ("all", "useState(() => { Promise.all([1, 2]); return 0; }, 'p');"),
    ] {
        let dir = base.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        let app = dir.join("app.tsx");
        std::fs::write(
            &app,
            format!(
                "export default function Main() {{
  {line}
  const [n, setN] = useState(0, 'n');
  useEffect(() => {{ if (n < 3) harness.wait({{ms: 10}}).then(() => setN(n + 1)); }}, [n]);
  return <>{{`n=${{n}}`}}</>;
}}"
            ),
        )
        .unwrap();
        let run = |dir: &Path, restore: bool| {
            let mut cfg = RuntimeConfig::new(&app);
            cfg.workers = 1;
            cfg.dir = Some(dir.to_path_buf());
            let enc = shared::tokenizer(&root().join("ref/tokenizers/zai-org__GLM-5.3.json"));
            let model = Arc::new(FnModel(|_: &GenRequest| panic!("no model call")));
            let mut rt = if restore {
                Runtime::restore(cfg, enc, model).unwrap()
            } else {
                Runtime::new(cfg, enc, model)
            };
            if !restore {
                let mut spec = InstanceSpec::new("agent-0", json!({}));
                spec.model = None;
                rt.add_instance(spec).unwrap();
            }
            rt.run().unwrap()
        };
        let (image, full) = (dir.join("image"), dir.join("full"));
        let first = run(&image, false);
        assert!(
            first.instances[0].error.is_none(),
            "{name}: {:?}",
            first.instances[0].error
        );
        assert_eq!(first.instances[0].state["__ui"]["@n"], json!(3), "{name}");
        let saved = std::fs::read(crate::shared::inst(&image, "agent-0/checkpoint.json"))
            .unwrap_or_else(|_| panic!("{name}: the agent was not imaged"));
        let saved: J = serde_json::from_slice(&saved).unwrap();
        assert_eq!(
            saved["data"]["image"]["journaled"].as_u64(),
            Some(first.instances[0].journal_len as u64),
            "{name}"
        );
        copy(&image, &full);
        std::fs::remove_file(crate::shared::inst(&full, "agent-0/checkpoint.json")).unwrap();
        let (from_image, replayed) = (run(&image, true), run(&full, true));
        assert_eq!(restores(&image).len(), 1, "{name}: the restore used the image");
        assert!(restores(&full).is_empty(), "{name}");
        assert_eq!(
            from_image.instances[0].state, replayed.instances[0].state,
            "{name}: image restore equals full replay"
        );
        assert_eq!(from_image.instances[0].metrics, replayed.instances[0].metrics, "{name}");
        assert_eq!(
            from_image.instances[0].journal_len, replayed.instances[0].journal_len,
            "{name}"
        );
    }
    std::fs::remove_dir_all(&base).unwrap();
}
