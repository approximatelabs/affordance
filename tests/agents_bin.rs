//! `affordance run` end to end: a mock SGLang `/generate`
//! server over real HTTP, three agents of `views/demo/runtime/agent.tsx`, `exec` on local bash.
use crate::tokenizer;
use affordance::emit::Encoder;
use serde_json::{Value as J, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Arc;

use crate::shared;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// A GLM-5.3 SGLang stand-in: the reply depends on how many model turns the prompt holds.
/// Like some servers, it leaves the matched stop token out of `output_ids`.
fn mock_sglang(enc: Arc<Encoder>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let enc = enc.clone();
            std::thread::spawn(move || {
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let (mut line, mut len) = (String::new(), 0usize);
                reader.read_line(&mut line).unwrap();
                // The model's context length, as SGLang reports it.
                if line.starts_with("GET /v1/models ") {
                    let models =
                        json!({"object": "list", "data": [{"id": "glm", "object": "model", "max_model_len": 131072}]})
                            .to_string();
                    let _ = write!(
                        &stream,
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{models}",
                        models.len()
                    );
                    return;
                }
                loop {
                    line.clear();
                    reader.read_line(&mut line).unwrap();
                    if line.trim().is_empty() {
                        break;
                    }
                    if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                        len = v.trim().parse().unwrap();
                    }
                }
                let mut body = vec![0; len];
                reader.read_exact(&mut body).unwrap();
                let req: J = serde_json::from_slice(&body).unwrap();
                if req.get("prompt").is_some() {
                    return serve_completions(stream, &req, &enc);
                }
                let ids: Vec<u32> = req["input_ids"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_u64().unwrap() as u32)
                    .collect();
                let n = ids
                    .iter()
                    .filter(|&&t| t == enc.token_id("<|assistant|>").unwrap())
                    .count();
                let text = match n {
                    1 => {
                        "Look.</think><tool_call>shell<arg_key>command</arg_key><arg_value>echo hello; echo $DEPLOY_ENV; echo $HOSTSECRET</arg_value></tool_call>"
                    }
                    2 => {
                        "Note.</think><tool_call>set_notes<arg_key>text</arg_key><arg_value>hello seen</arg_value></tool_call>"
                    }
                    3 => {
                        "Ping.</think><tool_call>send_message<arg_key>to</arg_key><arg_value>agent-0</arg_value><arg_key>text</arg_key><arg_value>ping</arg_value></tool_call>"
                    }
                    _ => "Rest.</think><tool_call>wait<arg_key>seconds</arg_key><arg_value>1</arg_value></tool_call>",
                };
                let out = enc.tok.encode(text, false).unwrap().get_ids().to_vec();
                let finish = json!({"type": "stop", "matched": enc.token_id("<|observation|>").unwrap()});
                serve(stream, &req, &out, finish);
            });
        }
    });
    format!("http://{addr}")
}

/// Answer a `/generate` request: plain JSON, or (with `stream`) server-sent events whose
/// `output_ids` are cumulative (SGLang's default) or incremental (`"incremental": true` in the
/// sampling params of this mock). `"stall_ms"` sleeps mid-stream; `"cut"` ends without a finish.
fn serve(mut s: std::net::TcpStream, req: &J, out: &[u32], finish: J) {
    if req["stream"] != true {
        let resp = json!({"output_ids": out, "meta_info": {"finish_reason": finish, "cached_tokens": 0}}).to_string();
        let _ = write!(
            s,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            resp.len(),
            resp
        );
        return;
    }
    let sp = &req["sampling_params"];
    let _ = write!(s, "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\r\n");
    for (i, chunk) in out.chunks(3).enumerate() {
        if i == 1
            && let Some(ms) = sp["stall_ms"].as_u64()
        {
            std::thread::sleep(std::time::Duration::from_millis(ms));
        }
        let upto = (i * 3 + chunk.len()).min(out.len());
        let ids: Vec<u32> = if sp["incremental"] == true {
            chunk.to_vec()
        } else {
            out[..upto].to_vec()
        };
        let last = upto == out.len();
        if last && sp["cut"] == true {
            return;
        }
        let ev = json!({"output_ids": ids, "meta_info": {"completion_tokens": upto, "cached_tokens": 5, "finish_reason": if last { finish.clone() } else { J::Null }}});
        if write!(s, "data: {ev}\n\n").and_then(|_| s.flush()).is_err() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let _ = write!(s, "data: [DONE]\n\n");
}

/// A vLLM-style `/v1/completions` stand-in: incremental `token_ids` per chunk, the stop token
/// reported in `stop_reason` (not in the ids), usage in a final chunk.
fn serve_completions(mut s: std::net::TcpStream, req: &J, enc: &Encoder) {
    assert_eq!(
        (req["return_token_ids"].clone(), req["stream"].clone()),
        (json!(true), json!(true))
    );
    let out = enc
        .tok
        .encode("Done.</think>All good.", false)
        .unwrap()
        .get_ids()
        .to_vec();
    let _ = write!(
        s,
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n"
    );
    let mut send = |ev: String| {
        let line = format!("data: {ev}\n\n");
        let _ = write!(s, "{:x}\r\n{line}\r\n", line.len());
    };
    for (i, chunk) in out.chunks(2).enumerate() {
        let last = (i + 1) * 2 >= out.len();
        send(json!({"choices": [{"token_ids": chunk, "finish_reason": if last { json!("stop") } else { J::Null }, "stop_reason": if last { json!(enc.token_id("<|user|>").unwrap()) } else { J::Null }}]}).to_string());
    }
    send(json!({"choices": [], "usage": {"prompt_tokens_details": {"cached_tokens": 7}}}).to_string());
    send("[DONE]".into());
    let _ = write!(s, "0\r\n\r\n");
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn completions_streaming_with_token_ids() {
    use affordance::runtime::{CancelToken, Completions, GenRequest, Model};
    let tokenizer = root().join("ref/tokenizers/zai-org__GLM-5.3.json");
    let enc = shared::tokenizer(&tokenizer);
    let url = mock_sglang(enc.clone());
    let m = Completions::new(&url, "glm");
    let r = GenRequest::new("a", vec![1, 2], 64, vec![], None, CancelToken::new());
    let reply = m.generate(&r).unwrap();
    let mut want = enc
        .tok
        .encode("Done.</think>All good.", false)
        .unwrap()
        .get_ids()
        .to_vec();
    want.push(enc.token_id("<|user|>").unwrap());
    assert_eq!((reply.ids, reply.cached_tokens, reply.cut_off), (want, Some(7), false));
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn sglang_streaming_cumulative_incremental_stop_stall_and_cut() {
    use affordance::runtime::{CancelToken, GenRequest, Model, Sglang};
    let tokenizer = root().join("ref/tokenizers/zai-org__GLM-5.3.json");
    let enc = shared::tokenizer(&tokenizer);
    let url = mock_sglang(enc.clone());
    let obs = enc.token_id("<|observation|>").unwrap();
    let want = {
        let mut v = enc.tok.encode("Look.</think><tool_call>shell<arg_key>command</arg_key><arg_value>echo hello; echo $DEPLOY_ENV; echo $HOSTSECRET</arg_value></tool_call>", false).unwrap().get_ids().to_vec();
        v.push(obs);
        v
    };
    let prompt = vec![enc.token_id("<|assistant|>").unwrap()];
    let req = |sampling: J| -> (Sglang, GenRequest) {
        let mut m = Sglang::new(&url, 1);
        m.sampling = sampling;
        m.stall = std::time::Duration::from_millis(300);
        (
            m,
            GenRequest::new("a", prompt.clone(), 64, vec![obs], None, CancelToken::new()),
        )
    };
    for sampling in [json!({}), json!({"incremental": true})] {
        let (m, r) = req(sampling.clone());
        let mut seen = vec![];
        let reply = m
            .generate_stream(&r, &mut |ids| {
                let affordance::runtime::Delta::Ids(ids) = ids else {
                    panic!("{ids:?}")
                };
                seen.extend_from_slice(ids);
                true
            })
            .unwrap();
        assert_eq!(reply.ids, want, "{sampling}");
        assert_eq!(
            seen,
            want[..want.len() - 1],
            "the observer saw every generated id once ({sampling})"
        );
        assert_eq!(reply.cached_tokens, Some(5));
        assert!(reply.first_token_ms.is_some() && !reply.cut_off);
    }
    // The client stops reading: generation is cut short (the server sees the connection close).
    let (m, r) = req(json!({}));
    let reply = m.generate_stream(&r, &mut |_| false).unwrap();
    assert_eq!(reply.ids.len(), 3);
    // A stalled stream and a stream cut without a finish reason are errors (and would be retried).
    let (m, r) = req(json!({"stall_ms": 2000}));
    let e = m.generate(&r).unwrap_err();
    assert!(
        e.message.contains("stalled") && e.kind.may_pass(),
        "a stall is an outage, not the agent's end: {e:?}"
    );
    let (m, r) = req(json!({"cut": true}));
    let e = m.generate(&r).unwrap_err();
    assert!(
        e.message.contains("without a finish reason") && e.kind.may_pass(),
        "{e:?}"
    );
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn generic_runtime_binary_runs_an_app_from_config() {
    let tokenizer = root().join("ref/tokenizers/zai-org__GLM-5.3.json");
    let enc = shared::tokenizer(&tokenizer);
    let url = mock_sglang(enc);
    let dir = std::env::temp_dir().join(format!("aff-agents-bin-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let config = json!({
        "app": {"view": root().join("views/demo/runtime/agent.tsx"), "tokenizer": tokenizer},
        "model": {"kind": "sglang", "url": url, "ranks": 2},
        "machines": {"home": {"kind": "local"}},
        "instances": [{"id": "agent", "count": 3, "props": {"task": "Say hello."}, "max_new": 256, "max_turns": 4}],
        "env": {"DEPLOY_ENV": "staging"},
        "dir": dir, "workers": 2, "max_wall_s": 60
    });
    let cfg_path = dir.join("config.json");
    std::fs::write(&cfg_path, config.to_string()).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_affordance"))
        .arg("run")
        .arg(&cfg_path)
        .env("HOSTSECRET", "runtime-only")
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let report: J = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["complete"], true, "{report}");
    assert_eq!(
        (
            report["agents"].as_u64(),
            report["turns"].as_u64(),
            report["rewrites"].as_u64()
        ),
        (Some(3), Some(12), Some(0)),
        "{report}"
    );
    assert_eq!(report["exact_appends"], 9);
    // The journal of agent-0 holds the messages from its two peers.
    let journal = std::fs::read_to_string(crate::shared::inst(&dir, "agent-0/journal.jsonl")).unwrap();
    assert_eq!(journal.matches("\"name\":\"message\"").count(), 2, "{journal}");
    // A local command sees the agent's environ (DEPLOY_ENV) but not the runtime's (HOSTSECRET).
    assert!(journal.contains("hello\\nstaging\\n\\n"), "{journal}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn the_binary_runs_a_root_app_that_mounts_a_fleet_on_a_machine_pool() {
    let tokenizer = root().join("ref/tokenizers/zai-org__GLM-5.3.json");
    let enc = shared::tokenizer(&tokenizer);
    let url = mock_sglang(enc);
    let dir = std::env::temp_dir().join(format!("aff-agents-root-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let config = json!({
        "instances": [{"id": "root", "app": root().join("views/demo/tree/fleet.tsx"), "props": {"task": "Say hello."}, "max_turns": 4}],
        "model": {"kind": "sglang", "url": url, "tokenizer": tokenizer},
        "machines": {"vm-{n:02}": {"kind": "local", "count": 3}},
        "env": {"DEPLOY_ENV": "staging"},
        "dir": dir, "workers": 1, "max_wall_s": 60
    });
    let cfg_path = dir.join("config.json");
    std::fs::write(&cfg_path, config.to_string()).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_affordance"))
        .arg("run")
        .arg(&cfg_path)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let report: J = serde_json::from_slice(&out.stdout).unwrap();
    let ids: Vec<&str> = report["per_agent"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids,
        ["root", "root/agent-0", "root/agent-1", "root/agent-2"],
        "{report}"
    );
    assert_eq!(report["turns"], 12, "three agents, four turns each: {report}");
    // Each agent ran its shell on the machine the root handed it (one of the pool).
    for n in 0..3 {
        let spec: J = serde_json::from_slice(
            &std::fs::read(crate::shared::inst(&dir, &format!("root/agent-{n}/agent.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(spec["machines"], json!([format!("vm-{n:02}")]));
        let journal =
            std::fs::read_to_string(crate::shared::inst(&dir, &format!("root/agent-{n}/journal.jsonl"))).unwrap();
        assert!(journal.contains("hello\\nstaging"), "{journal}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn obsolete_memory_limit_and_unknown_fields_are_rejected() {
    let root = std::env::temp_dir().join(format!("aff-invalid-config-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    for key in ["resident_bytes", "model_threds"] {
        let path = root.join(format!("{key}.json"));
        std::fs::write(&path, format!("{{\"{key}\":1024}}")).unwrap();
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_affordance"))
            .arg("run")
            .arg(path)
            .output()
            .unwrap();
        assert!(!output.status.success());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("unknown field") && error.contains(key), "{error}");
    }
    std::fs::remove_dir_all(root).unwrap();
}

/// A per-agent machine (`{id}` in its spec) serves the agent's mounted children too: a child
/// (`a/kid`) runs on its root's machine, as `<Mount>` hands a child its parent's machines.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn a_mounted_child_gets_its_own_place_on_a_per_instance_machine() {
    let tokenizer = root().join("ref/tokenizers/zai-org__GLM-5.3.json");
    let url = mock_sglang(shared::tokenizer(&tokenizer));
    let dir = std::env::temp_dir().join(format!("aff-agents-child-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let app = dir.join("app.tsx");
    std::fs::write(&app, r#"export function Child({ onDone }) {
  useEffect(() => { (async () => { const r = await harness.machine.call({cmd: "pwd"}); onDone(r.output.trim()); })(); }, []);
  return <>child</>;
}
export default function Main() {
  const [where, setWhere] = useState(null, "where");
  useEffect(() => { if (where !== null) harness.stop({reason: "done", where}); }, [where]);
  return <Mount key="kid" machine="home"><Child onDone={w => setWhere(w)}/></Mount>;
}"#).unwrap();
    let config = json!({
        "app": {"view": app, "tokenizer": tokenizer},
        "model": {"kind": "sglang", "url": url},
        "machines": {"home": {"kind": "local", "home": dir.join("homes"), "per_instance": true}},
        "instances": [{"id": "a"}],
        "dir": dir.join("run"), "workers": 1, "max_wall_s": 20
    });
    let cfg_path = dir.join("config.json");
    std::fs::write(&cfg_path, config.to_string()).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_affordance"))
        .arg("run")
        .arg(&cfg_path)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    // The child's own place, named by the id the runtime gave it (never by its name).
    let kid = crate::shared::instance_dir(&dir.join("run"), "a/kid");
    let place = dir.join("homes").join(kid.file_name().unwrap());
    let state = std::fs::read_to_string(crate::shared::inst(&dir, "run/a/journal.jsonl")).unwrap();
    assert!(state.contains(&format!("{}", place.display())), "{state}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// `affordance run --help` explains itself; a run where an agent fails (here its app does not load)
/// still prints the report, then exits nonzero naming the agent and why.
#[test]
fn help_and_a_failed_agent_fail_the_run() {
    let help = std::process::Command::new(env!("CARGO_BIN_EXE_affordance"))
        .arg("run")
        .arg("--help")
        .output()
        .unwrap();
    assert!(help.status.success() && String::from_utf8_lossy(&help.stdout).contains("affordance run [PATH]"));
    let dir = std::env::temp_dir().join(format!("aff-agents-failed-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("app.tsx"),
        "export default function Main() { return <>unclosed; }",
    )
    .unwrap();
    let tokenizer = affordance_support_tokenizer(&dir);
    let config = json!({"app": {"view": dir.join("app.tsx"), "tokenizer": tokenizer},
        "model": {"kind": "sglang", "url": "http://127.0.0.1:1", "context": "none"}, "instances": [{"id": "a"}], "max_wall_s": 5});
    std::fs::write(dir.join("config.json"), config.to_string()).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_affordance"))
        .arg("run")
        .arg(dir.join("config.json"))
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !out.status.success() && stderr.contains("1 of 1 agents failed") && stderr.contains("a: "),
        "{stderr}"
    );
    let report: J = serde_json::from_slice(&out.stdout).unwrap();
    assert!(report["per_agent"][0]["error"].is_string(), "{report}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A tokenizer file for a test deployment: the synthetic test tokenizer, written to `dir`.
fn affordance_support_tokenizer(dir: &std::path::Path) -> PathBuf {
    let path = dir.join("tokenizer.json");
    std::fs::copy(tokenizer::path(), &path).unwrap();
    path
}
