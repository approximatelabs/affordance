//! Stock binding configuration is tested at offline HTTP endpoints, including CLI recovery.
use crate::tokenizer;
use affordance::runtime::{
    CancelToken, GenRequest, Model, Sessions, Sglang,
    model::{Credentials, from_config},
};
use serde_json::{Value as J, json};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};

#[derive(Clone, Debug)]
struct Wire {
    path: String,
    headers: String,
    body: J,
}
struct Peer {
    url: String,
    seen: Arc<Mutex<Vec<Wire>>>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Peer {
    fn new(handle: impl Fn(&Wire, &mut TcpStream) + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let seen = Arc::new(Mutex::new(vec![]));
        let stop = Arc::new(AtomicBool::new(false));
        let (records, done, handle) = (seen.clone(), stop.clone(), Arc::new(handle));
        let thread = std::thread::spawn(move || {
            let mut threads = vec![];
            while !done.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        // On macOS an accepted socket inherits the listener's non-blocking mode.
                        stream.set_nonblocking(false).unwrap();
                        let handle = handle.clone();
                        let records = records.clone();
                        threads.push(std::thread::spawn(move || {
                            stream.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
                            let mut header = vec![];
                            let mut byte = [0];
                            while !header.ends_with(b"\r\n\r\n") {
                                if stream.read_exact(&mut byte).is_err() {
                                    return;
                                }
                                header.push(byte[0]);
                            }
                            // Header names are case-insensitive: compare them in lowercase.
                            let headers: String = String::from_utf8(header)
                                .unwrap()
                                .split("\r\n")
                                .enumerate()
                                .map(|(i, l)| match l.split_once(':') {
                                    Some((k, v)) if i > 0 => {
                                        let v = v.trim();
                                        if v.is_empty() {
                                            format!("{}:\r\n", k.to_ascii_lowercase())
                                        } else {
                                            format!("{}: {v}\r\n", k.to_ascii_lowercase())
                                        }
                                    }
                                    _ => format!("{l}\r\n"),
                                })
                                .collect();
                            let length: usize = headers
                                .lines()
                                .find_map(|l| {
                                    l.to_ascii_lowercase()
                                        .strip_prefix("content-length:")
                                        .map(|s| s.trim().parse().unwrap())
                                })
                                .unwrap_or(0);
                            let mut body = vec![0; length];
                            stream.read_exact(&mut body).unwrap();
                            let path = headers.split_whitespace().nth(1).unwrap().to_string();
                            let wire = Wire {
                                path,
                                headers,
                                // A GET has no body.
                                body: if body.is_empty() {
                                    J::Null
                                } else {
                                    serde_json::from_slice(&body).unwrap()
                                },
                            };
                            records.lock().unwrap().push(wire.clone());
                            handle(&wire, &mut stream);
                        }));
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(2))
                    }
                    Err(e) => panic!("{e}"),
                }
            }
            for thread in threads {
                thread.join().unwrap();
            }
        });
        Self {
            url,
            seen,
            stop,
            thread: Some(thread),
        }
    }
    fn stock() -> Self {
        Self::new(answer)
    }
    /// The requests the clients sent, but the window query a binding makes when it is validated.
    fn requests(&self) -> Vec<Wire> {
        self.seen
            .lock()
            .unwrap()
            .iter()
            .filter(|w| !w.headers.starts_with("GET "))
            .cloned()
            .collect()
    }
    /// The window queries (`GET /v1/models`, `GET /get_server_info`).
    fn gets(&self) -> Vec<String> {
        self.seen
            .lock()
            .unwrap()
            .iter()
            .filter(|w| w.headers.starts_with("GET "))
            .map(|w| w.path.clone())
            .collect()
    }
}
impl Drop for Peer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.thread.take().unwrap().join().unwrap();
    }
}
fn reply(stream: &mut TcpStream, body: J) {
    let text = body.to_string();
    let _ = write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}",
        text.len()
    );
}
fn answer(w: &Wire, stream: &mut TcpStream) {
    let mut body = if w.path == "/v1/models" {
        // The served model and its context length, as SGLang and vLLM report them.
        json!({"object":"list","data":[{"id":"test","object":"model","max_model_len":131072}]})
    } else if w.path.ends_with("/generate") {
        json!({"output_ids":[41,42],"meta_info":{"completion_tokens":2,"prompt_tokens":w.body["input_ids"].as_array().unwrap().len()+w.body["session_params"]["offset"].as_u64().unwrap_or(0) as usize,"finish_reason":{"type":"stop","matched":99}}})
    } else if w.path.ends_with("/v1/messages") {
        json!({"id":"msg_1","model":"claude-test","content":[{"type":"text","text":"okay"}],"stop_reason":"end_turn","usage":{"input_tokens":5,"cache_read_input_tokens":7,"output_tokens":1}})
    } else if w.path.ends_with("/chat/completions") {
        json!({"choices":[{"message":{"content":"okay"},"finish_reason":"stop"}],"usage":{"cost":0.001}})
    } else if w.path.ends_with("/completions") {
        json!({"choices":[{"token_ids":[41,42],"finish_reason":"stop","stop_reason":99}],"usage":{}})
    } else {
        json!({})
    };
    // Real providers sometimes echo authentication in diagnostic metadata.
    if let Some(auth) = w
        .headers
        .lines()
        .find(|l| l.starts_with("authorization:") || l.starts_with("x-api-key:"))
    {
        let metadata = if body.get("meta_info").is_some() {
            "meta_info"
        } else {
            "usage"
        };
        if body.get(metadata).is_some() {
            body[metadata]["credential_diagnostic"] = json!({auth:[auth]});
        }
    }
    if w.body["stream"] == true {
        let text = format!("data: {body}\n\ndata: [DONE]\n\n");
        let _ = write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}",
            text.len()
        );
    } else {
        reply(stream, body)
    }
}
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let p = std::env::temp_dir().join(format!(
            "aff-bindings-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&p).unwrap();
        Self(p)
    }
    fn credentials(&self, name: &str) -> PathBuf {
        let p = self.0.join(format!("{name}.key"));
        std::fs::write(&p, format!("KEY=fixture-secret-{name}\n")).unwrap();
        p
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn request() -> GenRequest {
    GenRequest::new("same-agent", vec![7, 8], 16, vec![99], Some(3), CancelToken::new())
}
fn has_key(w: &Wire, key: &str) -> bool {
    w.headers
        .lines()
        .any(|l| l == format!("authorization: Bearer fixture-secret-{key}"))
}

#[test]
fn public_factory_routes_native_and_chat_options_without_sharing_defaults_or_auth() {
    let files = Scratch::new();
    let native = Peer::stock();
    let completions = Peer::stock();
    let chat = Peer::stock();
    let enc = Arc::new(tokenizer::encoder());
    for stream in [false, true] {
        for (name, url, kind) in [
            ("a", &native.url, "sglang"),
            ("b", &native.url, "sglang"),
            ("c", &completions.url, "completions"),
            ("d", &chat.url, "chat"),
        ] {
            let mut cfg = json!({"kind":kind,"url":url,"key_file":files.credentials(name),"key_name":"KEY","sampling":{"temperature":0.6,"top_p":0.9},"provider_options":{"vendor_future":{"binding":name}},"stream":stream&&kind!="chat"});
            if kind == "sglang" {
                cfg["provider_options"]["lora_path"] = json!(format!("variant-{name}"));
            }
            if kind == "completions" {
                cfg["name"] = json!("provider-model:adapter-c");
            }
            if kind == "chat" {
                cfg["model"] = json!("provider-chat-d");
                cfg["context"] = json!(200000);
            }
            let model = from_config(&cfg, enc.clone()).unwrap();
            let mut req = request();
            req.sampling = json!({"temperature":0.2});
            if kind == "chat" {
                req.ids = enc
                    .tok
                    .encode(r#"{"messages":[{"role":"user","content":"hello"}]}"#, false)
                    .unwrap()
                    .get_ids()
                    .to_vec()
                    .into();
            }
            let result = model.generate(&req).unwrap();
            assert_eq!(result.attempts, 1);
            assert!(
                !result
                    .model_meta
                    .as_ref()
                    .unwrap()
                    .to_string()
                    .contains("fixture-secret-")
            );
            let records = match kind {
                "sglang" => native.requests(),
                "completions" => completions.requests(),
                _ => chat.requests(),
            };
            let w = records.last().unwrap();
            assert!(has_key(w, name));
            assert_eq!(w.body["vendor_future"]["binding"], name);
            let sampling = if kind == "sglang" {
                &w.body["sampling_params"]
            } else {
                &w.body
            };
            assert_eq!(sampling["temperature"], 0.2);
            assert_eq!(sampling["top_p"], 0.9);
            if kind == "sglang" {
                assert_eq!(w.body["lora_path"], format!("variant-{name}"));
                assert!(sampling.get("lora_path").is_none());
                assert_eq!(w.body["input_ids"], json!([7, 8]));
            }
            if kind == "completions" {
                assert_eq!(w.body["model"], "provider-model:adapter-c");
                assert_eq!(w.body["prompt"], json!([7, 8]));
                assert!(w.body.get("lora_path").is_none());
            }
            if kind == "chat" {
                assert_eq!(w.body["model"], "provider-chat-d");
                assert!(w.body["messages"].is_array());
            }
        }
    }
}

#[test]
fn invalid_configuration_and_per_call_conflicts_fail_before_inference() {
    let enc = Arc::new(tokenizer::encoder());
    let base = json!({"kind":"sglang","url":"http://127.0.0.1:1"});
    for change in [
        json!({"lora_path":"ignored"}),
        json!({"samplign":{}}),
        json!({"ranks":0}),
        json!({"ranks":1.5}),
        json!({"stream":"true"}),
        json!({"retries":-1}),
        json!({"sampling":null}),
        json!({"provider_options":[]}),
        json!({"request_timeout_s":0}),
        json!({"sessions":{"turnz":2}}),
        json!({"sessions":{"turns":0}}),
        json!({"key_name":"KEY"}),
        json!({"url":"ftp://unsupported-native"}),
        json!({"url":"http://secret@example.com"}),
        json!({"provider_options":{"rid":"bad"}}),
        json!({"provider_options":{"model":"bad"}}),
        json!({"provider_options":{"lora_path":[]}}),
        json!({"sampling":{"lora_path":"bad"}}),
        json!({"provider_options":{"future":1},"sampling":{"future":2}}),
    ] {
        let mut cfg = base.clone();
        for (k, v) in change.as_object().unwrap() {
            cfg[k] = v.clone();
        }
        assert!(from_config(&cfg, enc.clone()).is_err(), "{change}");
    }
    let mut model = Sglang::new("http://127.0.0.1:1", 1);
    model.provider_options = json!({"vendor_option":1});
    for sampling in [
        json!({"lora_path":"another"}),
        json!({"model":"another"}),
        json!({"vendor_option":2}),
        json!({"input_ids":[0]}),
    ] {
        let mut req = request();
        req.sampling = sampling;
        let error = model.generate(&req).unwrap_err();
        assert!(!error.kind.may_pass(), "{error:?}");
    }
    model.provider_options = json!({"input_ids":[0]});
    assert!(!model.generate(&request()).unwrap_err().kind.may_pass());
}

#[test]
fn sessions_keep_lora_identity_and_authenticate_all_control_requests() {
    let files = Scratch::new();
    let peer = Peer::stock();
    let mut model = Sglang::new(&peer.url, 8);
    model.stream = false;
    model.sessions = Some(Sessions::new(4, Duration::from_secs(30)));
    model.auth = Some(Credentials::new(files.credentials("sessions"), "KEY"));
    model.provider_options = json!({"lora_path":"variant-a"});
    model.generate(&request()).unwrap();
    let mut req = request();
    req.ids = vec![7, 8, 9].into();
    req.sampling = json!({"temperature":0.1});
    model.generate(&req).unwrap();
    req.ids = vec![11].into();
    model.generate(&req).unwrap();
    let seen = peer.requests();
    assert!(seen.iter().all(|w| has_key(w, "sessions")));
    assert_eq!(seen.iter().filter(|w| w.path == "/open_session").count(), 2);
    assert!(seen.iter().any(|w| w.path == "/close_session"));
    let generates: Vec<_> = seen.iter().filter(|w| w.path == "/generate").collect();
    assert_eq!(generates[1].body["input_ids"], json!([9]));
    assert_eq!(generates[1].body["session_params"]["offset"], 2);
    assert!(generates.iter().all(|w| w.body["lora_path"] == "variant-a"));
    model.provider_options = json!({"lora_path":"variant-b"});
    let error = model.generate(&req).unwrap_err();
    assert!(error.message.contains("identity changed"));
    assert_eq!(peer.requests().len(), seen.len());
}

#[test]
fn a_failed_attempt_is_aborted_with_the_key_and_the_next_keeps_the_provider_options() {
    let files = Scratch::new();
    let count = AtomicUsize::new(0);
    let peer = Peer::new(move |w, s| {
        if w.path == "/generate" && count.fetch_add(1, Ordering::SeqCst) == 0 {
            return;
        }
        answer(w, s)
    });
    let mut model = Sglang::new(&peer.url, 1);
    model.stream = true;
    model.auth = Some(Credentials::new(files.credentials("retry"), "KEY"));
    model.provider_options = json!({"lora_path":"variant-retry","vendor_switch":true});
    // The client makes one attempt: the first call fails, and the caller's next one succeeds.
    let error = model.generate(&request()).unwrap_err();
    assert!(error.kind.may_pass(), "{error:?}");
    model.generate(&request()).unwrap();
    let seen = peer.requests();
    assert!(seen.iter().all(|w| has_key(w, "retry")));
    let generations: Vec<_> = seen.iter().filter(|w| w.path == "/generate").collect();
    assert_eq!(generations.len(), 2);
    assert!(
        generations
            .iter()
            .all(|w| w.body["lora_path"] == "variant-retry" && w.body["vendor_switch"] == true)
    );
    let abort = seen.iter().find(|w| w.path == "/abort_request").unwrap();
    assert_eq!(abort.body["rid"], generations[0].body["rid"]);
    assert_ne!(generations[0].body["rid"], generations[1].body["rid"]);
}

fn cli(config: &J, file: &std::path::Path, capture: &std::path::Path, resume: bool) -> std::process::Output {
    // The apps are written to a scratch directory and import the repository's views.
    let mut config = config.clone();
    config["library"] = json!(env!("CARGO_MANIFEST_DIR"));
    std::fs::write(file, config.to_string()).unwrap();
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_affordance"));
    command.arg("run").arg(file).env("AFFORDANCE_CAPTURE_DIR", capture);
    command.arg(if resume { "--resume" } else { "--fresh" });
    command.output().unwrap()
}
fn texts(path: &std::path::Path) -> String {
    let mut result = String::new();
    for entry in std::fs::read_dir(path).unwrap().flatten() {
        let p = entry.path();
        if p.is_dir() {
            result.push_str(&texts(&p));
        } else {
            result.push_str(&std::fs::read_to_string(p).unwrap());
        }
    }
    result
}
#[test]
fn cli_named_bindings_recover_without_reissuing_replies_and_pin_identity() {
    let files = Scratch::new();
    let native = Peer::stock();
    let completions = Peer::stock();
    let app = files.0.join("app.tsx");
    std::fs::write(&app,r#"export default function Main(){useEffect(()=>{(async()=>{await model();harness.stop({reason:'done'});})();},[]);return <>hello<span label='protocol' syntax='glm' stop={[tok.user.id]} in_think={false}/></>;}"#).unwrap();
    let binding = |name: &str, url: &str, kind: &str| json!({"kind":kind,"url":url,"tokenizer":tokenizer::path(),"key_file":files.credentials(name),"key_name":"KEY","sampling":{"temperature":if name=="a"{0.1}else{0.8}},"stream":name=="b","provider_options":{"binding_tag":name}});
    let mut a = binding("a", &native.url, "sglang");
    a["provider_options"]["lora_path"] = json!("variant-a");
    let mut b = binding("b", &native.url, "sglang");
    b["provider_options"]["lora_path"] = json!("variant-b");
    let mut c = binding("c", &completions.url, "completions");
    c["name"] = json!("provider-model-c");
    // The stock server serves "test", so this binding states its window.
    c["context"] = json!(8192);
    let run = files.0.join("run");
    let capture = files.0.join("capture");
    let path = files.0.join("deployment.json");
    let mut config = json!({"app":{"view":app,"tokenizer":tokenizer::path()},"model":a,"models":{"A":a,"B":b,"C":c},"agents":[{"id":"one","model":"A"},{"id":"two","model":"B"},{"id":"three","model":"C"}],"dir":run,"replay":"strict","max_wall_s":5});
    let first = cli(&config, &path, &capture, false);
    assert!(first.status.success(), "{}", String::from_utf8_lossy(&first.stderr));
    let report: J = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(report["turns"], 3);
    assert_eq!(report["model_errors"], 0);
    assert!(
        report["per_agent"]
            .as_array()
            .unwrap()
            .iter()
            .all(|a| a["error"].is_null() && a["stopped"]["reason"] == "done"),
        "{report}"
    );
    let n = native.requests();
    let c = completions.requests();
    assert_eq!(n.len(), 2);
    assert_eq!(c.len(), 1);
    for w in &n {
        let key = w.body["binding_tag"].as_str().unwrap();
        assert!(has_key(w, key));
        assert_eq!(w.body["lora_path"], format!("variant-{key}"));
        assert_eq!(
            w.body["sampling_params"]["temperature"],
            if key == "a" { 0.1 } else { 0.8 }
        );
    }
    assert!(has_key(&c[0], "c"));
    assert_eq!(c[0].body["model"], "provider-model-c");
    assert!(c[0].body.get("lora_path").is_none());
    let restored = cli(&config, &path, &capture, true);
    assert!(restored.status.success());
    let restored: J = serde_json::from_slice(&restored.stdout).unwrap();
    assert!(
        restored["per_agent"]
            .as_array()
            .unwrap()
            .iter()
            .all(|a| a["error"].is_null()),
        "{restored}"
    );
    assert_eq!(native.requests().len(), 2);
    assert_eq!(completions.requests().len(), 1);
    config["models"]["A"]["provider_options"]["lora_path"] = json!("changed");
    let changed = cli(&config, &path, &capture, true);
    let changed: J = serde_json::from_slice(&changed.stdout).unwrap();
    let one = changed["per_agent"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == "one")
        .unwrap();
    assert!(
        one["error"].as_str().unwrap().contains("model binding identity"),
        "{changed}"
    );
    assert_eq!(native.requests().len(), 2);
    assert_eq!(completions.requests().len(), 1);
    let stored = texts(&run) + &texts(&capture) + &String::from_utf8(first.stdout).unwrap();
    assert!(!stored.contains("fixture-secret-"));
    assert!(texts(&capture).contains("[redacted]"));
    let pins: J = serde_json::from_slice(&std::fs::read(run.join("one/agent.json")).unwrap()).unwrap();
    assert_eq!(pins["pins"]["model"].as_str().unwrap().len(), 64);
    // The digest does not depend on credential contents; rotation keeps completed recovery valid.
    config["models"]["A"]["provider_options"]["lora_path"] = json!("variant-a");
    std::fs::write(files.0.join("a.key"), "KEY=rotated-fixture-secret\n").unwrap();
    let rotated = cli(&config, &path, &capture, true);
    let rotated: J = serde_json::from_slice(&rotated.stdout).unwrap();
    assert!(
        rotated["per_agent"]
            .as_array()
            .unwrap()
            .iter()
            .all(|a| a["error"].is_null()),
        "{rotated}"
    );
    assert_eq!(native.requests().len(), 2);
    // How a server is reached is not which model answers: other timeouts, retries and sampling
    // resume strictly (an operational change must not make a fleet unresumable).
    let mut moved = config.clone();
    moved["models"]["A"]["request_timeout_s"] = json!(77);
    moved["models"]["A"]["sampling"]["temperature"] = json!(0.3);
    let moved: J = serde_json::from_slice(&cli(&moved, &path, &capture, true).stdout).unwrap();
    assert!(
        moved["per_agent"]
            .as_array()
            .unwrap()
            .iter()
            .all(|a| a["error"].is_null()),
        "{moved}"
    );
    assert_eq!(native.requests().len(), 2);
    // A crash before the first reply leaves only that agent's generation outstanding.
    let journal = run.join("one/journal.jsonl");
    let recorded = std::fs::read_to_string(&journal).unwrap();
    let prefix = recorded
        .lines()
        .take_while(|line| serde_json::from_str::<J>(line).unwrap()["input"] != "reply")
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&journal, if prefix.is_empty() { prefix } else { prefix + "\n" }).unwrap();
    let outstanding = cli(&config, &path, &capture, true);
    let outstanding: J = serde_json::from_slice(&outstanding.stdout).unwrap();
    assert!(
        outstanding["per_agent"]
            .as_array()
            .unwrap()
            .iter()
            .all(|a| a["error"].is_null()),
        "{outstanding}"
    );
    assert_eq!(native.requests().len(), 3);
    let last = native.requests().last().unwrap().clone();
    assert!(!has_key(&last, "a"));
    assert!(last.headers.contains("Bearer rotated-fixture-secret"));
    assert_eq!(last.body["lora_path"], "variant-a");
    // Warning recovery of an unpinned journal cannot manufacture a strict identity pin.
    let saved = run.join("one/agent.json");
    let mut legacy: J = serde_json::from_slice(&std::fs::read(&saved).unwrap()).unwrap();
    legacy.as_object_mut().unwrap().remove("pins");
    std::fs::write(&saved, legacy.to_string()).unwrap();
    config["replay"] = json!("warn");
    let warning = cli(&config, &path, &capture, true);
    assert!(String::from_utf8_lossy(&warning.stderr).contains("was not recorded"));
    let legacy: J = serde_json::from_slice(&std::fs::read(&saved).unwrap()).unwrap();
    assert!(legacy["pins"]["model"].is_null());
    config["replay"] = json!("strict");
    let strict = cli(&config, &path, &capture, true);
    let strict: J = serde_json::from_slice(&strict.stdout).unwrap();
    assert!(
        strict["per_agent"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["id"] == "one")
            .unwrap()["error"]
            .as_str()
            .unwrap()
            .contains("model binding identity")
    );
    // CLI schema errors fail before launching another request.
    config["models"]["B"]["samplign"] = json!({});
    let invalid = cli(&config, &path, &capture, true);
    assert!(!invalid.status.success());
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("samplign"));
    assert_eq!(native.requests().len(), 3);
}

#[test]
fn cancellation_preserves_binding_and_authenticates_the_native_abort() {
    let files = Scratch::new();
    let (ready_rx, ready_tx) = {
        let (tx, rx) = std::sync::mpsc::channel();
        (rx, tx)
    };
    let peer = Peer::new(move |w, s| {
        if w.path == "/generate" {
            write!(
                s,
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n"
            )
            .unwrap();
            s.flush().unwrap();
            ready_tx.send(()).unwrap();
            let mut byte = [0];
            let _ = s.read(&mut byte);
        } else {
            answer(w, s)
        }
    });
    let mut model = Sglang::new(&peer.url, 1);
    model.auth = Some(Credentials::new(files.credentials("cancel"), "KEY"));
    model.provider_options = json!({"lora_path":"variant-cancel"});
    let req = request();
    let cancel = req.cancel.clone();
    let thread = std::thread::spawn(move || model.generate(&req));
    ready_rx.recv_timeout(Duration::from_secs(3)).unwrap();
    cancel.cancel("fixture cancellation");
    let error = thread.join().unwrap().unwrap_err();
    assert!(error.message.contains("cancel"));
    let seen = peer.requests();
    assert_eq!(seen.iter().filter(|w| w.path == "/generate").count(), 1);
    assert!(seen.iter().all(|w| has_key(w, "cancel")));
    assert!(seen.iter().any(|w| w.path == "/abort_request"));
    assert_eq!(
        seen.iter().find(|w| w.path == "/generate").unwrap().body["lora_path"],
        "variant-cancel"
    );
}

#[test]
fn separate_lora_clients_keep_separate_sessions_and_read_rotated_keys() {
    let files = Scratch::new();
    let peer = Peer::stock();
    let enc = Arc::new(tokenizer::encoder());
    let make = |name: &str| {
        from_config(&json!({"kind":"sglang","url":peer.url,"stream":false,"sessions":true,"key_file":files.credentials(name),"key_name":"KEY","provider_options":{"lora_path":name}}),enc.clone()).unwrap()
    };
    let (a, b) = (make("a"), make("b"));
    a.generate(&request()).unwrap();
    b.generate(&request()).unwrap();
    std::fs::write(files.0.join("a.key"), "KEY=fixture-secret-a-rotated\n").unwrap();
    let mut next = request();
    next.ids = vec![7, 8, 9].into();
    a.generate(&next).unwrap();
    b.generate(&next).unwrap();
    let seen = peer.requests();
    let opens: Vec<_> = seen.iter().filter(|w| w.path == "/open_session").collect();
    assert_eq!(opens.len(), 2);
    assert_ne!(opens[0].body["session_id"], opens[1].body["session_id"]);
    for name in ["a", "b"] {
        let open = opens.iter().find(|w| has_key(w, name)).unwrap();
        let continued = seen
            .iter()
            .find(|w| w.path == "/generate" && w.body["lora_path"] == name && w.body["session_params"]["offset"] == 2)
            .unwrap();
        assert_eq!(continued.body["session_params"]["id"], open.body["session_id"]);
        assert!(has_key(continued, if name == "a" { "a-rotated" } else { "b" }));
    }
}

#[test]
fn quoted_credentials_are_sent_literally_and_invalid_keys_fail_locally() {
    let files = Scratch::new();
    let peer = Peer::stock();
    let enc = Arc::new(tokenizer::encoder());
    let key = files.0.join("quoted.key");
    std::fs::write(&key, "KEY='fixture-quote\"-slash\\-key'\n").unwrap();
    let cfg = json!({"kind":"chat","context":200000,"url":peer.url,"model":"test","key_file":key,"key_name":"KEY"});
    let mut req = request();
    req.ids = enc
        .tok
        .encode(r#"{"messages":[{"role":"user","content":"hello"}]}"#, false)
        .unwrap()
        .get_ids()
        .to_vec()
        .into();
    from_config(&cfg, enc.clone()).unwrap().generate(&req).unwrap();
    assert!(
        peer.requests()[0]
            .headers
            .lines()
            .any(|l| l == "authorization: Bearer fixture-quote\"-slash\\-key")
    );
    for contents in ["KEY=\n", "KEY=has space\n", "KEY=\"unterminated\n"] {
        std::fs::write(&key, contents).unwrap();
        let error = from_config(&cfg, enc.clone()).err().unwrap();
        if !contents.trim_start_matches("KEY=").trim().is_empty() {
            assert!(!error.contains(contents.trim_start_matches("KEY=").trim()));
        }
    }
    assert_eq!(peer.requests().len(), 1);
}

#[test]
fn provider_errors_cannot_echo_credentials_into_model_failures() {
    let files = Scratch::new();
    let enc = Arc::new(tokenizer::encoder());
    let peer = Peer::new(|w, s| {
        let auth = w.headers.lines().find(|l| l.starts_with("authorization:")).unwrap();
        let body = json!({"error":format!("invalid credential: {auth}")}).to_string();
        write!(
            s,
            "HTTP/1.1 401 Unauthorized\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
    });
    for kind in ["sglang", "completions", "chat"] {
        let mut cfg =
            json!({"kind":kind,"url":peer.url,"key_file":files.credentials(kind),"key_name":"KEY","stream":false});
        if kind == "chat" {
            cfg["model"] = json!("m");
            cfg["context"] = json!(200000);
        }
        let mut req = request();
        if kind == "chat" {
            req.ids = enc
                .tok
                .encode(r#"{"messages":[{"role":"user","content":"hello"}]}"#, false)
                .unwrap()
                .get_ids()
                .to_vec()
                .into();
        }
        let error = from_config(&cfg, enc.clone()).unwrap().generate(&req).unwrap_err();
        assert!(!error.kind.may_pass());
        assert!(!error.message.contains("fixture-secret-"), "{error:?}");
        assert!(error.message.contains("[redacted]"), "{error:?}");
    }
}

#[test]
fn fractional_chat_timeout_is_not_rounded_down_to_unlimited() {
    let files = Scratch::new();
    let enc = Arc::new(tokenizer::encoder());
    let peer = Peer::new(|w, s| {
        std::thread::sleep(Duration::from_millis(500));
        answer(w, s)
    });
    for timeout in [0.05, 0.000000001] {
        let cfg = json!({"kind":"chat","context":200000,"url":peer.url,"model":"test","key_file":files.credentials("timeout"),"key_name":"KEY","request_timeout_s":timeout});
        let mut req = request();
        req.ids = enc
            .tok
            .encode(r#"{"messages":[{"role":"user","content":"hello"}]}"#, false)
            .unwrap()
            .get_ids()
            .to_vec()
            .into();
        let error = from_config(&cfg, enc.clone()).unwrap().generate(&req).unwrap_err();
        assert!(error.kind.may_pass());
        assert!(error.message.contains("timed out"), "{}", error.message);
    }
}

#[test]
fn chat_headers_are_binding_local_validated_and_legacy_money_fields_fail() {
    let files = Scratch::new();
    let peer = Peer::stock();
    let enc = Arc::new(tokenizer::encoder());
    let base = json!({"kind":"chat","context":200000,"url":peer.url,"model":"test","key_file":files.credentials("headers"),"key_name":"KEY",
        "headers":{"HTTP-Referer":"https://example.test/","X-Title":"quote\" slash\\","X-Empty":""}});
    let client = from_config(&base, enc.clone()).unwrap();
    let mut req = request();
    req.ids = enc
        .tok
        .encode(r#"{"messages":[{"role":"user","content":"hello"}]}"#, false)
        .unwrap()
        .get_ids()
        .to_vec()
        .into();
    client.generate(&req).unwrap();
    let seen = peer.requests();
    for line in [
        "http-referer: https://example.test/",
        "x-title: quote\" slash\\",
        "x-empty:",
    ] {
        assert!(seen[0].headers.lines().any(|header| header == line), "{:?}", seen[0]);
    }
    for headers in [
        json!([]),
        json!({"X-Title":7}),
        json!({"Bad Name":"a"}),
        json!({"":"a"}),
        json!({"X-Title":"line\r\ninjected: value"}),
        json!({"X-Title":"a\u{0000}b"}),
        json!({"AUTHORIZATION":"forbidden"}),
        json!({"content-TYPE":"forbidden"}),
        json!({"X-Title":"a","x-title":"b"}),
    ] {
        let mut bad = base.clone();
        bad["headers"] = headers;
        assert!(from_config(&bad, enc.clone()).is_err(), "{bad}");
    }
    for field in ["price_in", "price_out", "budget_usd"] {
        let mut bad = base.clone();
        bad[field] = json!(0);
        assert!(from_config(&bad, enc.clone()).err().unwrap().contains(field));
    }
    let mut changed = base.clone();
    changed["headers"]["X-Title"] = json!("new routing profile");
    assert_ne!(
        client.binding_identity(),
        from_config(&changed, enc).unwrap().binding_identity()
    );
    assert_eq!(peer.requests().len(), 1, "invalid configs cannot dispatch");
}

/// "I can say in the deployment or in my TSX what happens when a model call fails." The server
/// here answers 503 (busy) to its first two requests. With the binding's `retry` the call is made
/// again and the app gets the reply; a call that says `retry: false` gets the first failure; a
/// policy that does not list `busy` does not try a busy server again.
#[test]
fn one_retry_policy_is_stated_in_the_deployment_or_on_the_call() {
    let files = Scratch::new();
    let write = |name: &str, options: &str| {
        let app = files.0.join(format!("{name}.tsx"));
        let source = r#"export default function Main(){
      useEffect(()=>{(async()=>{
        try { await model(undefined, OPTIONS); harness.stop('answered'); }
        catch (e) { harness.stop({reason: 'failed', error: String(e)}); }
      })();},[]);
      return <>hello<span label="protocol" syntax="glm" stop={[tok.user.id]} in_think={false}/></>;
    }"#;
        std::fs::write(&app, source.replace("OPTIONS", options)).unwrap();
        app
    };
    for (name, binding, call, stopped, requests) in [
        ("deployment", json!({"wait_s": [0.02, 0.05]}), "{}", "answered", 3),
        (
            "call",
            json!({"on": []}),
            "{retry: {on: ['busy'], wait_s: [0.02, 0.05]}}",
            "answered",
            3,
        ),
        ("off", json!({"wait_s": [0.02, 0.05]}), "{retry: false}", "failed", 1),
        (
            "other-kinds",
            json!({"on": ["unreachable", "interrupted"]}),
            "{}",
            "failed",
            1,
        ),
        (
            "out-of-time",
            json!({"for_s": 0.03, "wait_s": [0.02, 0.05]}),
            "{}",
            "failed",
            2,
        ),
    ] {
        let seen = AtomicUsize::new(0);
        let peer = Peer::new(move |wire, socket| {
            if wire.path == "/v1/completions" && seen.fetch_add(1, Ordering::SeqCst) < 2 {
                let _ = write!(
                    socket,
                    "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 4\r\nConnection: close\r\n\r\nbusy"
                );
            } else {
                answer(wire, socket);
            }
        });
        let config = json!({"app":{"view":write(name, call),"tokenizer":tokenizer::path()},
          "model":{"kind":"completions","url":peer.url,"name":"test","stream":false,"retry":binding},
          "agents":[{"id":"a"}],"dir":files.0.join(format!("run-{name}")),"max_wall_s":20,"replay":"strict"});
        let path = files.0.join(format!("{name}.json"));
        let output = cli(&config, &path, &files.0.join(format!("capture-{name}")), false);
        let report: J = serde_json::from_slice(&output.stdout)
            .unwrap_or_else(|_| panic!("{name}: {}", String::from_utf8_lossy(&output.stderr)));
        let reason = &report["per_agent"][0]["stopped"];
        assert_eq!(reason.get("reason").unwrap_or(reason), stopped, "{name}: {report}");
        let calls = peer.requests().iter().filter(|w| w.path == "/v1/completions").count();
        assert_eq!(calls, requests, "{name}");
        // Each failure is in the trace with its kind, and a resumed run makes no new request.
        let trace = std::fs::read_to_string(files.0.join(format!("run-{name}/a/trace.jsonl"))).unwrap();
        assert_eq!(
            trace.matches(r#""kind":"busy""#).count(),
            requests.min(2),
            "{name}: {trace}"
        );
        cli(&config, &path, &files.0.join(format!("capture-{name}")), true);
        let calls = peer.requests().iter().filter(|w| w.path == "/v1/completions").count();
        assert_eq!(calls, requests, "{name}: a resumed run replays its journaled failures");
    }
    // A policy that cannot be read is refused when the deployment is loaded.
    let peer = Peer::stock();
    let config = json!({"app":{"view":write("bad", "{}"),"tokenizer":tokenizer::path()},
      "model":{"kind":"completions","url":peer.url,"name":"test","retry":{"on":["sometimes"]}},
      "agents":[{"id":"a"}],"dir":files.0.join("run-bad")});
    let output = cli(&config, &files.0.join("bad.json"), &files.0.join("capture-bad"), false);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("retry.on"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// "If the server says when to come back, the agent waits at least that long." A token-id
/// server answers 503 with `Retry-After: 1` once; the next attempt is a second later, though the
/// policy's own wait is 20 ms, streamed or not.
#[test]
fn a_token_id_servers_retry_after_is_the_least_the_next_attempt_waits() {
    let files = Scratch::new();
    let app = files.0.join("app.tsx");
    std::fs::write(
        &app,
        r#"export default function Main(){
      useEffect(()=>{(async()=>{await model();harness.stop('answered');})();},[]);
      return <>hello<span label="protocol" syntax="glm" stop={[tok.user.id]} in_think={false}/></>;
    }"#,
    )
    .unwrap();
    for (name, streaming) in [("plain", false), ("stream", true)] {
        let seen = AtomicUsize::new(0);
        let peer = Peer::new(move |wire, socket| {
            if wire.path == "/v1/completions" && seen.fetch_add(1, Ordering::SeqCst) == 0 {
                let _ = write!(
                    socket,
                    "HTTP/1.1 503 Service Unavailable\r\nRetry-After: 1\r\nContent-Length: 4\r\nConnection: close\r\n\r\nbusy"
                );
            } else {
                answer(wire, socket);
            }
        });
        let config = json!({"app":{"view":app,"tokenizer":tokenizer::path()},
          "model":{"kind":"completions","url":peer.url,"name":"test","stream":streaming,"retry":{"wait_s":[0.02,0.05]}},
          "agents":[{"id":"a"}],"dir":files.0.join(format!("run-{name}")),"max_wall_s":20,"replay":"strict"});
        let output = cli(
            &config,
            &files.0.join(format!("{name}.json")),
            &files.0.join(format!("capture-{name}")),
            false,
        );
        assert!(
            output.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let trace = std::fs::read_to_string(files.0.join(format!("run-{name}/a/trace.jsonl"))).unwrap();
        assert!(trace.contains(r#""retry_in_ms":1000"#), "{name}: {trace}");
    }
}

#[test]
fn completions_capture_each_attempt_and_stream_event_without_credentials() {
    let files = Scratch::new();
    let app = files.0.join("native.tsx");
    std::fs::write(
        &app,
        r#"export default function Main(){
      useEffect(()=>{(async()=>{await model();harness.stop('done');})();},[]);
      return <>hello<span label="protocol" syntax="glm" stop={[tok.user.id]} in_think={false}/></>;
    }"#,
    )
    .unwrap();
    for (name, streaming, retries) in [("plain", false, 0), ("stream", true, 0), ("retry", true, 1)] {
        let seen = AtomicUsize::new(0);
        let peer = Peer::new(move |wire, socket| {
            if wire.path == "/v1/completions" && seen.fetch_add(1, Ordering::SeqCst) == 0 && retries > 0 {
                // A partial stream must be captured even though it cannot complete the call.
                let text = "data: {\"choices\":[{\"token_ids\":[41]}]}\n\n";
                write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}", text.len()).unwrap();
            } else {
                answer(wire, socket);
            }
        });
        let capture = files.0.join(name);
        let config = json!({"app":{"view":app,"tokenizer":tokenizer::path()},
          "model":{"kind":"completions","url":peer.url,"name":"test","stream":streaming,"retry":{"wait_s":[0.05,1]},
            "key_file":files.credentials(name),"key_name":"KEY"},
          "agents":[{"id":"native-agent"}],"dir":files.0.join(format!("run-{name}")),"max_wall_s":5,"replay":"strict"});
        let path = files.0.join(format!("{name}.json"));
        let output = cli(&config, &path, &capture, false);
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        let report: J = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["per_agent"][0]["stopped"], "done", "{report}");
        let mut attempts = std::collections::BTreeMap::<String, std::collections::BTreeMap<String, J>>::new();
        for entry in std::fs::read_dir(capture.join("native-agent")).unwrap() {
            let path = entry.unwrap().path();
            assert_eq!(path.extension().unwrap(), "json", "no temporary capture remains");
            let record: J = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            let rid = record["request_id"].as_str().unwrap();
            let kind = path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .strip_prefix(&format!("{rid}."))
                .unwrap()
                .strip_suffix(".json")
                .unwrap();
            attempts.entry(rid.into()).or_default().insert(kind.into(), record);
        }
        assert_eq!(attempts.len(), retries + 1);
        for (i, captures) in attempts.values().enumerate() {
            let request = &captures["request"]["data"];
            assert_eq!(request["body"], peer.requests()[i].body);
            assert!(
                request["headers"]
                    .as_array()
                    .unwrap()
                    .contains(&json!(["Authorization", "[redacted]"]))
            );
            assert_eq!(captures.contains_key("event-0"), streaming);
            assert!(captures.contains_key("response"));
            if retries > 0 && i == 0 {
                assert!(
                    captures["response"]["data"]["result"]["Err"]
                        .as_str()
                        .unwrap()
                        .contains("finish reason")
                );
            }
        }
        assert!(!texts(&capture).contains("fixture-secret-"));
        assert!(cli(&config, &path, &capture, true).status.success());
        assert_eq!(
            peer.requests().len(),
            retries + 1,
            "replay must reuse the journaled reply"
        );
    }
}

#[test]
fn chat_captures_requests_responses_and_usage_without_credentials() {
    let files = Scratch::new();
    let peer = Peer::stock();
    let app = files.0.join("chat.tsx");
    std::fs::write(
        &app,
        format!(
            r#"import {{ModelContext}} from '{0}/views/lib/model.tsx';
      export function Child({{onDone}}){{
        useEffect(()=>{{(async()=>{{await model();onDone();harness.stop({{reason:'done'}});}})();}},[]);
        return <ModelContext strategy="unrolled">hello</ModelContext>;
      }}
      export default function Main(){{
        return <Mount key="child"><Child onDone={{()=>harness.stop({{reason:'done'}})}}/></Mount>;
      }}"#,
            env!("CARGO_MANIFEST_DIR")
        ),
    )
    .unwrap();
    let capture = files.0.join("captures");
    let run = files.0.join("run");
    let config = json!({"app":{"view":app,"tokenizer":tokenizer::path()},
      "model":{"kind":"chat","context":200000,"url":peer.url,"model":"test","key_file":files.credentials("chat-capture"),"key_name":"KEY",
        "headers":{"HTTP-Referer":"https://example.test/","X-Title":"Affordance"}},
      "agents":[{"id":"chat-agent"}],"dir":run,"max_wall_s":5,"replay":"strict"});
    let path = files.0.join("deployment.json");
    let output = cli(&config, &path, &capture, false);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let report: J = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["model_errors"], 0, "{report}");
    assert!(
        capture.join("chat-agent/child").is_dir(),
        "model call did not capture: {report}; {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let paths: Vec<_> = std::fs::read_dir(capture.join("chat-agent/child"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(paths.len(), 2, "one atomic request and response, no temporary files");
    let read = |kind: &str| -> J {
        let path = paths
            .iter()
            .find(|p| p.to_string_lossy().ends_with(&format!(".{kind}.json")))
            .unwrap();
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
    };
    let request = read("request");
    let response = read("response");
    assert_eq!(request["request_id"], response["request_id"]);
    assert_eq!(request["data"]["body"], peer.requests()[0].body);
    let headers = request["data"]["headers"].as_array().unwrap();
    assert!(headers.contains(&json!(["Authorization", "[redacted]"])));
    assert!(headers.contains(&json!(["X-Title", "Affordance"])));
    assert_eq!(response["data"]["result"]["Ok"][0], 200);
    assert_eq!(response["data"]["result"]["Ok"][1]["usage"]["cost"], 0.001);
    let stored = texts(&run) + &texts(&capture) + &String::from_utf8(output.stdout).unwrap();
    assert!(
        !stored.contains("fixture-secret-"),
        "credentials escaped into diagnostics"
    );
    assert!(stored.contains("credential_diagnostic"));
    assert!(stored.contains("[redacted]"));
    let restored = cli(&config, &path, &capture, true);
    assert!(restored.status.success());
    assert_eq!(peer.requests().len(), 1, "recovery must reuse the journaled reply");
}

#[test]
fn bindings_select_templates_for_children_history_and_strict_recovery() {
    let files = Scratch::new();
    let peer = Peer::stock();
    let app = files.0.join("generic.tsx");
    std::fs::write(&app, format!(r#"
import {{ModelContext}} from '{0}/views/lib/model.tsx';
import {{useLog}} from '{0}/views/lib/log.tsx';
export function Child(){{
  const log=useLog();
  useEffect(()=>{{(async()=>{{
    for(let i=0;i<2;i++){{const out=await log.call();log.close(out);}}
    harness.stop({{binding:harness.model,records:await log.read(0,99)}});
  }})();}},[]);
  return <ModelContext system='shared app' log={{log.records}} strategy='unrolled'>hello</ModelContext>;
}}
export default function Main(){{return <><Mount key='native' model='native'><Child/></Mount><Mount key='hosted' model='hosted'><Child/></Mount></>;}}
"#, env!("CARGO_MANIFEST_DIR"))).unwrap();
    let native =
        json!({"kind":"sglang","url":peer.url,"tokenizer":tokenizer::path(),"stream":false,"template":"glm53"});
    let hosted = json!({"kind":"chat","context":200000,"url":peer.url,"tokenizer":tokenizer::path(),"model":"test","key_file":files.credentials("presentation"),"key_name":"KEY"});
    let mut config = json!({"root":{"app":app},"model":native,"models":{"native":native,"hosted":hosted},
        "dir":files.0.join("run"),"max_wall_s":5,"replay":"strict"});
    let path = files.0.join("deployment.json");
    let capture = files.0.join("capture");
    let first = cli(&config, &path, &capture, false);
    assert!(first.status.success(), "{}", String::from_utf8_lossy(&first.stderr));
    let report: J = serde_json::from_slice(&first.stdout).unwrap();
    for (child, template) in [("native", "glm53"), ("hosted", "messages")] {
        let agent = report["per_agent"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["id"] == format!("root/{child}"))
            .unwrap();
        assert_eq!(agent["error"], J::Null, "{agent}");
        assert_eq!(agent["stopped"]["binding"]["template"], template, "{agent}");
        assert_eq!(agent["stopped"]["binding"]["name"], child);
        let records = agent["stopped"]["records"].as_array().unwrap();
        assert_eq!(records.iter().filter(|r| r["kind"] == "model").count(), 2);
        for (i, r) in records.iter().enumerate() {
            assert_eq!(r["id"], format!("e{i}"));
        }
    }
    assert_eq!(peer.requests().len(), 4);
    assert!(cli(&config, &path, &capture, true).status.success());
    assert_eq!(peer.requests().len(), 4, "replay must not make provider calls");
    config["models"]["native"]["template_options"] = json!({"reasoning_effort":"low"});
    let changed = cli(&config, &path, &capture, true);
    assert!(String::from_utf8_lossy(&changed.stderr).contains("model binding identity"));
    assert_eq!(peer.requests().len(), 4);

    let enc = Arc::new(tokenizer::encoder());
    for (key, value) in [
        ("template", json!("missing")),
        ("template", J::Null),
        ("template_options", json!([])),
        ("template_options", json!({"children":"bad"})),
    ] {
        let mut bad = native.clone();
        bad[key] = value;
        assert!(from_config(&bad, enc.clone()).is_err(), "{bad}");
    }
    let mut bad = hosted.clone();
    bad["template"] = json!("glm53");
    assert!(from_config(&bad, enc).is_err());
    // Presentation must not certify an unidentified custom inference implementation.
    let scripted: Arc<dyn Model> = Arc::new(affordance::runtime::FnModel(|_: &GenRequest| Ok(Default::default())));
    let scripted = affordance::runtime::model::with_template(
        scripted,
        affordance::runtime::model::ModelTemplate::new("glm53", json!({})).unwrap(),
    )
    .unwrap();
    assert!(scripted.binding_identity().is_none());
}

#[test]
fn anthropic_binding_captures_privately_replays_strictly_and_pins_its_settings() {
    let files = Scratch::new();
    let peer = Peer::stock();
    let app = files.0.join("claude.tsx");
    std::fs::write(
        &app,
        format!(
            r#"
import {{ModelContext}} from '{0}/views/lib/model.tsx';
import {{useLog}} from '{0}/views/lib/log.tsx';
export default function Main(){{
  const log=useLog();
  useEffect(()=>{{(async()=>{{
    for(let i=0;i<2;i++){{const out=await log.call();log.close(out);}}
    harness.stop({{binding:harness.model,records:await log.read(0,99)}});
  }})();}},[]);
  return <ModelContext system='shared app' log={{log.records}} strategy='unrolled'>hello</ModelContext>;
}}
"#,
            env!("CARGO_MANIFEST_DIR")
        ),
    )
    .unwrap();
    let capture = files.0.join("captures");
    let mut config = json!({"app":{"view":app,"tokenizer":tokenizer::path()},
      "model":{"kind":"anthropic","context":200000,"url":peer.url,"model":"claude-test","key_file":files.credentials("claude"),"key_name":"KEY",
        "headers":{"anthropic-beta":"thinking-binding-controls-2026-08-01"}},
      "agents":[{"id":"claude"}],"dir":files.0.join("run"),"max_wall_s":5,"replay":"strict"});
    let path = files.0.join("deployment.json");
    let first = cli(&config, &path, &capture, false);
    assert!(first.status.success(), "{}", String::from_utf8_lossy(&first.stderr));
    let report: J = serde_json::from_slice(&first.stdout).unwrap();
    let agent = &report["per_agent"][0];
    assert_eq!(agent["error"], J::Null, "{report}");
    assert_eq!(agent["stopped"]["binding"]["template"], "messages");
    assert_eq!(
        agent["stopped"]["records"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["kind"] == "model" && r["content"] == "okay")
            .count(),
        2,
        "{agent}"
    );
    let requests = peer.requests();
    assert_eq!(requests.len(), 2);
    assert!(
        requests
            .iter()
            .all(|w| w.path == "/v1/messages" && w.body["system"][0]["text"] == "shared app")
    );
    // The second turn's history ends at a cache breakpoint before the current screen.
    let history = requests[1].body["messages"].as_array().unwrap();
    assert_eq!(
        history[history.len() - 2]["content"][0]["cache_control"]["type"],
        "ephemeral"
    );
    let stored = texts(&files.0.join("run")) + &texts(&capture) + &String::from_utf8(first.stdout).unwrap();
    assert!(
        !stored.contains("fixture-secret-"),
        "credentials escaped into diagnostics"
    );
    assert!(stored.contains("credential_diagnostic") && stored.contains(r#"["x-api-key","[redacted]"]"#));
    assert!(cli(&config, &path, &capture, true).status.success());
    assert_eq!(peer.requests().len(), 2, "replay must not make provider calls");
    // How the provider is reached (here its cache TTL) is not the model: a strict resume goes on.
    config["model"]["cache_ttl"] = json!("1h");
    assert!(cli(&config, &path, &capture, true).status.success());
    // Another model is: refused.
    config["model"]["model"] = json!("claude-other");
    let changed = cli(&config, &path, &capture, true);
    assert!(
        String::from_utf8_lossy(&changed.stderr).contains("model binding identity"),
        "{}",
        String::from_utf8_lossy(&changed.stderr)
    );
    assert_eq!(peer.requests().len(), 2);
}

fn chat_app(files: &Scratch) -> PathBuf {
    let chat = files.0.join("chat.tsx");
    std::fs::write(
        &chat,
        format!(
            r#"import {{ModelContext}} from '{0}/views/lib/model.tsx';
      export default function Main(){{
        useEffect(()=>{{(async()=>{{await model();harness.stop('done');}})();}},[]);
        return <ModelContext strategy="unrolled">hello</ModelContext>;
      }}"#,
            env!("CARGO_MANIFEST_DIR")
        ),
    )
    .unwrap();
    chat
}

/// A capture is diagnostics: a capture directory that cannot be written (a full disk) never
/// fails, resends or aborts a call, native or hosted. A streaming call captures its response.
#[test]
fn captures_are_best_effort_and_streams_capture_their_response() {
    let files = Scratch::new();
    let native = files.0.join("native.tsx");
    std::fs::write(&native, r#"export default function Main(){useEffect(()=>{(async()=>{await model();harness.stop('done');})();},[]);return <>hello<span label="protocol" syntax="glm" stop={[tok.user.id]} in_think={false}/></>;}"#).unwrap();
    let chat = chat_app(&files);
    // A file where the capture directory should be: every capture fails.
    let unwritable = files.0.join("unwritable");
    std::fs::write(&unwritable, "").unwrap();
    for (name, kind, streaming) in [
        ("plain", "sglang", false),
        ("stream", "sglang", true),
        ("chat", "chat", false),
        ("captured", "sglang", true),
    ] {
        let peer = Peer::stock();
        let model = if kind == "chat" {
            json!({"kind":"chat","context":200000,"url":peer.url,"model":"test","key_file":files.credentials(name),"key_name":"KEY"})
        } else {
            json!({"kind":"sglang","url":peer.url,"stream":streaming})
        };
        let config = json!({"app":{"view":if kind == "chat" {&chat} else {&native},"tokenizer":tokenizer::path()},"model":model,
          "agents":[{"id":"agent"}],"dir":files.0.join(format!("run-{name}")),"max_wall_s":5,"replay":"strict"});
        let capture = if name == "captured" {
            files.0.join("captures")
        } else {
            unwritable.clone()
        };
        let output = cli(&config, &files.0.join(format!("{name}.json")), &capture, false);
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        let report: J = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            (&report["per_agent"][0]["stopped"], &report["model_errors"]),
            (&json!("done"), &json!(0)),
            "{name}: {report}"
        );
        let paths: Vec<_> = peer.requests().into_iter().map(|w| w.path).collect();
        assert_eq!(paths.len(), 1, "{name}: one request, no resend or abort: {paths:?}");
        if name == "captured" {
            assert_eq!(report["capture_errors"], 0);
            let mut kinds: Vec<_> = std::fs::read_dir(capture.join("agent"))
                .unwrap()
                .map(|e| {
                    e.unwrap()
                        .file_name()
                        .to_string_lossy()
                        .rsplit('.')
                        .nth(1)
                        .unwrap()
                        .to_string()
                })
                .collect();
            kinds.sort();
            assert_eq!(kinds, ["event-0", "request", "response"]);
        } else {
            let failed = report["capture_errors"].as_u64().unwrap_or(0);
            assert!(failed >= 2, "{name}: {report}");
            assert_eq!(
                String::from_utf8_lossy(&output.stderr)
                    .matches("request capture failed")
                    .count() as u32,
                failed.ilog2() + 1,
                "logged at the 1st, 2nd, 4th, ... failure"
            );
        }
    }
}

/// A hosted call that cannot reach its server captures the failure without the address.
#[test]
fn hosted_captures_do_not_name_the_server() {
    let files = Scratch::new();
    // Nothing listens on port 9. (A port picked and released could be taken by another test.)
    let port = 9;
    let config = json!({"app":{"view":chat_app(&files),"tokenizer":tokenizer::path()},
      "model":{"kind":"chat","context":200000,"url":format!("http://127.0.0.1:{port}"),"model":"test","key_file":files.credentials("closed"),"key_name":"KEY"},
      "agents":[{"id":"agent"}],"dir":files.0.join("run"),"max_wall_s":2,"replay":"strict"});
    let capture = files.0.join("captures");
    cli(&config, &files.0.join("closed.json"), &capture, false);
    let captured = texts(&capture);
    assert!(captured.contains("unreachable"), "{captured}");
    for address in ["127.0.0.1".to_string(), format!(":{port}"), format!("port {port}")] {
        assert!(!captured.contains(&address), "{address} in {captured}");
    }
}

/// A server's `abort` finish is a transient failure, never a reply, in both stream modes.
#[test]
fn a_server_abort_is_a_transient_failure_not_a_reply() {
    for streaming in [false, true] {
        let peer = Peer::new(move |w, s| {
            if w.path != "/generate" {
                return answer(w, s);
            }
            let finish = json!({"type":"abort","message":"the scheduler shut down"});
            if w.body["stream"] == true {
                let mut text = String::new();
                for n in 1..=3 {
                    let done = if n == 3 { finish.clone() } else { J::Null };
                    text += &format!(
                        "data: {}\n\n",
                        json!({"output_ids":[40 + n],"meta_info":{"completion_tokens":n,"prompt_tokens":2,"finish_reason":done}})
                    );
                }
                text += "data: [DONE]\n\n";
                let _ = write!(
                    s,
                    "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}",
                    text.len()
                );
            } else {
                reply(
                    s,
                    json!({"output_ids":[41,42,43],"meta_info":{"completion_tokens":3,"prompt_tokens":2,"finish_reason":finish}}),
                );
            }
        });
        let mut model = Sglang::new(&peer.url, 1);
        model.stream = streaming;
        let error = model.generate(&request()).unwrap_err();
        assert!(
            error.kind.may_pass()
                && error.message.contains("aborted")
                && error.message.contains("the scheduler shut down"),
            "{error:?}"
        );
        assert_eq!(peer.requests().len(), 1);
    }
}

/// An HTTP 401 with a plain-text body is answered: one request, no retry, no abort.
#[test]
fn a_plain_text_refusal_is_sent_once_and_never_aborted() {
    for streaming in [false, true] {
        let peer = Peer::new(|w, s| {
            if w.path != "/generate" {
                return answer(w, s);
            }
            let _ = write!(
                s,
                "HTTP/1.1 401 Unauthorized\r\nContent-Type: text/plain\r\nContent-Length: 12\r\nConnection: close\r\n\r\nunauthorized"
            );
        });
        let mut model = Sglang::new(&peer.url, 1);
        model.stream = streaming;
        let error = model.generate(&request()).unwrap_err();
        assert!(
            !error.kind.may_pass() && error.message.contains("401") && error.message.contains("unauthorized"),
            "{error:?}"
        );
        let paths: Vec<_> = peer.requests().into_iter().map(|w| w.path).collect();
        assert_eq!(paths, ["/generate"], "streaming={streaming}");
    }
}

/// A hosted request is its binding's config alone: a `~/.curlrc` (here a proxy that refuses
/// every connection) changes neither where it goes nor how. The test runs in a child whose home
/// is a scratch directory.
#[test]
fn a_hosted_request_ignores_the_users_curlrc() {
    const CHILD: &str = "AFFORDANCE_CURLRC_CHILD";
    let files = Scratch::new();
    if std::env::var_os(CHILD).is_none() {
        let closed = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap();
        std::fs::write(files.0.join(".curlrc"), format!("proxy = \"http://{closed}\"\n")).unwrap();
        let out = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "model_bindings::a_hosted_request_ignores_the_users_curlrc",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .env("HOME", &files.0)
            .env("CURL_HOME", &files.0)
            .env_remove("XDG_CONFIG_HOME")
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        return;
    }
    let enc = Arc::new(tokenizer::encoder());
    let peer = Peer::stock();
    let cfg = json!({"kind":"chat","context":200000,"url":peer.url,"model":"test","key_file":files.credentials("rc"),"key_name":"KEY"});
    let mut req = request();
    req.ids = enc
        .tok
        .encode(r#"{"messages":[{"role":"user","content":"hello"}]}"#, false)
        .unwrap()
        .get_ids()
        .to_vec()
        .into();
    from_config(&cfg, enc).unwrap().generate(&req).unwrap();
    assert_eq!(peer.requests().len(), 1, "the provider saw the request");
}

/// While an agent's model call retries a transient failure (here a serving outage), the run's
/// status (`Control::status`) shows it `retrying`, with its failures so far and their kind, and
/// healthy: an outage is retried for as long as it lasts. Once the server answers, it goes on.
#[test]
fn a_retrying_agent_shows_in_the_status_feed() {
    use affordance::runtime::{AgentSpec, Runtime, RuntimeConfig};
    let files = Scratch::new();
    let up = Arc::new(AtomicBool::new(false));
    let serving = up.clone();
    let peer = Peer::new(move |w, s| {
        if serving.load(Ordering::SeqCst) || w.path == "/v1/models" {
            return answer(w, s);
        }
        let _ = write!(
            s,
            "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 4\r\nConnection: close\r\n\r\nbusy"
        );
    });
    let app = files.0.join("app.tsx");
    std::fs::write(&app, r#"export default function Main(){useEffect(()=>{(async()=>{await model();harness.stop({reason:'done'});})();},[]);return <>hello<span label='protocol' syntax='glm' stop={[tok.user.id]} in_think={false}/></>;}"#).unwrap();
    let mut cfg = RuntimeConfig::new(&app);
    cfg.library = Some(env!("CARGO_MANIFEST_DIR").into());
    cfg.dir = Some(files.0.join("run"));
    cfg.max_wall = Some(Duration::from_secs(60));
    let mut model = Sglang::new(&peer.url, 1);
    model.stream = false;
    let mut rt = Runtime::new(cfg, Arc::new(tokenizer::encoder()), Arc::new(model));
    rt.add_agent(AgentSpec::new("a", json!({}))).unwrap();
    let control = rt.control();
    let run = std::thread::spawn(move || rt.run().unwrap());
    let started = std::time::Instant::now();
    let agent = loop {
        assert!(started.elapsed() < Duration::from_secs(30), "never shown retrying");
        let status = control.status();
        let agent = status["agents"][0].clone();
        if agent["state"] == "retrying" && agent["retrying"]["failures"].as_u64() >= Some(2) {
            break agent;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(
        (&agent["healthy"], &agent["retrying"]["error"]),
        (&json!(true), &json!("busy")),
        "{agent}"
    );
    assert!(!agent.to_string().contains("127.0.0.1"), "{agent}");
    up.store(true, Ordering::SeqCst);
    let report = run.join().unwrap();
    assert_eq!(report.agents[0].stopped, Some(json!({"reason": "done"})));
}

/// A model server that answers `/v1/models` and `/get_server_info` with these bodies (HTTP 404
/// for null), and model calls as the stock peer does, after 300 ms (a run's status is published).
fn reporting(models: J, info: J) -> Peer {
    Peer::new(move |w, s| match w.path.as_str() {
        "/v1/models" | "/get_server_info" => {
            let body = if w.path == "/v1/models" { &models } else { &info };
            if body.is_null() {
                let _ = write!(
                    s,
                    "HTTP/1.1 404 Not Found\r\nContent-Length: 9\r\nConnection: close\r\n\r\nnot found"
                );
            } else {
                reply(s, body.clone())
            }
        }
        _ => {
            std::thread::sleep(Duration::from_millis(300));
            answer(w, s)
        }
    })
}

fn served(window: usize) -> J {
    json!({"object":"list","data":[{"id":"served-name","object":"model","max_model_len":window}]})
}

/// A native binding's window is the server's (a page's viewport is its screen's): asked once,
/// when the binding is validated. The runtime reports it (stderr, `Control::status`), lays prompts
/// out in it, and never sends a prompt over it: the call fails with a ContextError instead.
#[test]
fn a_native_binding_takes_its_window_from_the_server_and_never_sends_a_prompt_over_it() {
    use affordance::runtime::{AgentSpec, Runtime, RuntimeConfig};
    let peer = reporting(served(4096), J::Null);
    let enc = Arc::new(tokenizer::encoder());
    let model = from_config(&json!({"kind":"sglang","url":peer.url,"stream":false}), enc.clone()).unwrap();
    assert_eq!(
        (model.context(), peer.gets().len()),
        (None, 0),
        "construction asks no server"
    );
    model.validate().unwrap();
    model.validate().unwrap();
    assert_eq!(
        (model.context(), model.window_source()),
        (Some(4096), Some("server /v1/models".into()))
    );
    assert_eq!(peer.gets(), ["/v1/models"], "a client asks its server once");
    let files = Scratch::new();
    let app = files.0.join("app.tsx");
    std::fs::write(&app, r#"export default function Main({words}){
      useEffect(()=>{(async()=>{
        try { await model(); harness.stop({sent: true}); } catch (e) { harness.stop({name: e.name, message: e.message}); }
      })();},[]);
      return <>{"word ".repeat(words)}<span label='protocol' syntax='glm' stop={[tok.user.id]} in_think={false}/></>;
    }"#).unwrap();
    let mut cfg = RuntimeConfig::new(&app);
    cfg.library = Some(env!("CARGO_MANIFEST_DIR").into());
    cfg.dir = Some(files.0.join("run"));
    cfg.max_wall = Some(Duration::from_secs(60));
    let mut rt = Runtime::new(cfg, enc, model);
    for (id, words) in [("small", 100), ("large", 6000)] {
        let mut spec = AgentSpec::new(id, json!({"words": words}));
        spec.max_new = 256;
        rt.add_agent(spec).unwrap();
    }
    let control = rt.control();
    let report = rt.run().unwrap();
    let stopped = |id: &str| {
        let a = report.agents.iter().find(|a| a.id == id).unwrap();
        a.stopped
            .clone()
            .unwrap_or_else(|| panic!("{id}: {:?} {}", a.error, a.state))
    };
    assert_eq!(stopped("small"), json!({"sent": true}));
    let large = stopped("large");
    assert_eq!(large["name"], "ContextError", "{large}");
    assert!(
        large["message"]
            .as_str()
            .unwrap()
            .contains("do not fit the model's context (4096)"),
        "{large}"
    );
    assert_eq!(
        peer.requests().iter().filter(|w| w.path == "/generate").count(),
        1,
        "only the prompt in the window was sent"
    );
    assert_eq!(
        control.status()["models"],
        json!({"default": {"context": 4096, "source": "server /v1/models"}})
    );
}

/// A configured `context` only lowers the server's window; a server that cannot be asked leaves
/// it as configured; `"none"` asks nothing and has no window.
#[test]
fn a_configured_context_only_lowers_the_servers_window() {
    let enc = Arc::new(tokenizer::encoder());
    let peer = reporting(served(4096), J::Null);
    let binding = |context: J| {
        let mut b = json!({"kind":"sglang","url":peer.url});
        if !context.is_null() {
            b["context"] = context
        }
        from_config(&b, enc.clone()).unwrap()
    };
    let error = binding(json!(8192)).validate().unwrap_err().message;
    assert!(
        error.contains("8192") && error.contains("4096") && !error.contains("127.0.0.1"),
        "{error}"
    );
    let lower = binding(json!(2048));
    lower.validate().unwrap();
    assert_eq!(
        (lower.context(), lower.window_source()),
        (Some(2048), Some("configured".into()))
    );
    let asked = peer.gets().len();
    let none = binding(json!("none"));
    none.validate().unwrap();
    assert_eq!(
        (none.context(), none.window_source(), peer.gets().len()),
        (None, None, asked),
        "\"none\": no window, no request"
    );
    for bad in [json!(0), json!("unbounded"), json!(-1)] {
        assert!(
            from_config(&json!({"kind":"sglang","url":peer.url,"context":bad}), enc.clone()).is_err(),
            "{bad}"
        )
    }
    // An unreachable server: the configured window stands, and says why it was not checked.
    let closed = 9; // nothing listens there
    let unreachable = from_config(
        &json!({"kind":"completions","url":format!("http://127.0.0.1:{closed}"),"name":"m","context":2048}),
        enc.clone(),
    )
    .unwrap();
    unreachable.validate().unwrap();
    assert_eq!(unreachable.context(), Some(2048));
    let source = unreachable.window_source().unwrap();
    assert!(
        source.starts_with("configured (the server reported none: GET /v1/models: unreachable")
            && !source.contains("127.0.0.1"),
        "{source}"
    );
}

/// No window by omission: a native server that reports none, or a hosted binding (hosted APIs
/// do not report one) without `context`, fails validation naming what was tried and the ways out.
#[test]
fn a_binding_without_a_window_fails_validation() {
    let enc = Arc::new(tokenizer::encoder());
    let peer = reporting(
        json!({"object":"list","data":[{"id":"a"},{"id":"b"}]}),
        json!({"version":"0.5"}),
    );
    let model = from_config(&json!({"kind":"sglang","url":peer.url}), enc.clone()).unwrap();
    let error = model.validate().unwrap_err().message;
    for part in [
        "GET /v1/models: 2 models and no model name",
        "GET /get_server_info: no context_length",
        "\"context\"",
        "\"none\"",
    ] {
        assert!(error.contains(part), "{part}: {error}");
    }
    assert!(!error.contains("127.0.0.1"), "{error}");
    // The runtime names the binding.
    let files = Scratch::new();
    let app = files.0.join("app.tsx");
    std::fs::write(&app, "export default function Main(){return <>hello</>;}").unwrap();
    let rt = affordance::runtime::Runtime::new(affordance::runtime::RuntimeConfig::new(&app), enc.clone(), model);
    let error = rt.run().err().unwrap();
    assert!(
        error.starts_with("model binding \"default\": the model server reports no context length"),
        "{error}"
    );
    for kind in ["chat", "anthropic", "responses"] {
        let mut hosted =
            json!({"kind":kind,"url":peer.url,"model":"m","key_file":files.credentials(kind),"key_name":"KEY"});
        if kind == "chat" {
            // An OpenAI-compatible server is asked, as a native one is; this one reports no window.
            let error = from_config(&hosted, enc.clone())
                .unwrap()
                .validate()
                .unwrap_err()
                .message;
            assert!(
                error.starts_with("chat: the model server reports no context length") && error.contains("\"none\""),
                "{error}"
            );
            assert!(!error.contains("127.0.0.1"), "{error}");
        } else {
            let error = from_config(&hosted, enc.clone()).err().unwrap();
            assert!(
                error.contains("does not report the model's context length") && error.contains("\"none\""),
                "{error}"
            );
        }
        let asked = peer.gets().len();
        hosted["context"] = json!(200000);
        assert_eq!(from_config(&hosted, enc.clone()).unwrap().context(), Some(200000));
        hosted["context"] = json!("none");
        assert_eq!(from_config(&hosted, enc.clone()).unwrap().context(), None);
        assert_eq!(peer.gets().len(), asked, "a configured hosted binding asks nothing");
    }
    // A self-hosted OpenAI-compatible server that reports its window (vLLM's `max_model_len`).
    let vllm = reporting(
        json!({"object":"list","data":[{"id":"m","max_model_len":32768}]}),
        json!({}),
    );
    let chat = from_config(
        &json!({"kind":"chat","url":format!("{}/v1", vllm.url),"model":"m"}),
        enc.clone(),
    )
    .unwrap();
    chat.validate().unwrap();
    assert_eq!(
        (chat.context(), chat.window_source()),
        (Some(32768), Some("server /v1/models".into()))
    );
}

/// `/v1/models` that names another model (an SGLang serving "deepseek-v4.1-flash-1017" to a
/// binding that names none, among others) or gives no length falls through to SGLang's
/// `/get_server_info`. A completions binding finds its own model by name.
#[test]
fn the_window_falls_through_to_get_server_info() {
    let enc = Arc::new(tokenizer::encoder());
    for models in [
        json!({"data":[{"id":"x"},{"id":"y","max_model_len":9}]}),
        json!({"data":[{"id":"deepseek-v4.1-flash-1017"}]}),
    ] {
        let peer = reporting(models, json!({"context_length": 262144}));
        let model = from_config(&json!({"kind":"sglang","url":peer.url}), enc.clone()).unwrap();
        model.validate().unwrap();
        assert_eq!(
            (model.context(), model.window_source()),
            (Some(262144), Some("server /get_server_info".into()))
        );
        assert_eq!(peer.gets(), ["/v1/models", "/get_server_info"]);
    }
    let peer = reporting(
        json!({"data":[{"id":"other","max_model_len":9},{"id":"mine","context_length":32768}]}),
        J::Null,
    );
    let model = from_config(&json!({"kind":"completions","url":peer.url,"name":"mine"}), enc.clone()).unwrap();
    model.validate().unwrap();
    assert_eq!(model.context(), Some(32768));
    let peer = reporting(
        json!({"data":[{"id":"other","max_model_len":9}]}),
        json!({"context_length": 65536}),
    );
    let model = from_config(&json!({"kind":"completions","url":peer.url,"name":"mine"}), enc.clone()).unwrap();
    model.validate().unwrap();
    assert_eq!(model.context(), Some(65536), "another model's length is not this one's");
}

/// The window is part of the binding's identity, as it was when configured: a fleet that drops
/// its hand-set `context` once the server reports the same one keeps its resume pin.
#[test]
fn a_window_the_server_reports_keeps_the_pin_of_the_same_window_configured() {
    use affordance::runtime::model::{ModelTemplate, with_context, with_template};
    let enc = Arc::new(tokenizer::encoder());
    let peer = reporting(served(262144), J::Null);
    for template in [None, Some("glm53")] {
        let binding = |context: Option<usize>| {
            let mut b = json!({"kind":"sglang","url":peer.url,"provider_options":{"lora_path":"v"}});
            if let Some(t) = template {
                b["template"] = json!(t)
            }
            if let Some(c) = context {
                b["context"] = json!(c)
            }
            let model = from_config(&b, enc.clone()).unwrap();
            model.validate().unwrap();
            model
        };
        let (configured, discovered, lower) = (binding(Some(262144)), binding(None), binding(Some(131072)));
        assert_eq!(
            configured.replay_identity(),
            discovered.replay_identity(),
            "{template:?}"
        );
        assert_eq!(
            configured.binding_identity(),
            discovered.binding_identity(),
            "{template:?}"
        );
        assert_ne!(lower.replay_identity(), discovered.replay_identity(), "{template:?}");
        // The identity a configured window had before windows came from servers: the client,
        // its template, the window stated over them.
        let mut client = Sglang::new(&peer.url, 1);
        (client.provider_options, client.window) = (json!({"lora_path":"v"}), affordance::runtime::Window::Unbounded);
        let mut before: Arc<dyn Model> = Arc::new(client);
        if let Some(t) = template {
            before = with_template(before, ModelTemplate::new(t, json!({})).unwrap()).unwrap()
        }
        assert_eq!(
            with_context(before, 262144).replay_identity(),
            discovered.replay_identity(),
            "{template:?}"
        );
    }
}
