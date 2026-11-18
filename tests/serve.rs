//! `affordance serve`: an app behind HTTP. The memory server demo (views/demo/memory_server) answers
//! OpenAI-compatible chat requests through a stand-in model server; older exchanges fold into a tree
//! of summaries the model sees; a streamed request gets server-sent events; a restart with --resume
//! keeps the conversation.
use serde_json::{Value as J, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// A stand-in OpenAI-compatible model server: a summary request (its system prompt starts with
/// "Summarize") gets "summary N"; any other gets "reply N". Every request body is kept.
fn model_server() -> (String, Arc<Mutex<Vec<J>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    let seen = Arc::new(Mutex::new(vec![]));
    let log = seen.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let stream = stream.unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let (mut length, mut line) = (0, String::new());
            while reader.read_line(&mut line).unwrap() > 0 && line != "\r\n" {
                if let Some(n) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    length = n.trim().parse().unwrap();
                }
                line.clear();
            }
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();
            let body: J = serde_json::from_slice(&body).unwrap();
            let summary = body["messages"][0]["content"]
                .as_str()
                .is_some_and(|s| s.starts_with("Summarize"));
            let mut seen = log.lock().unwrap();
            seen.push(body);
            let content = format!("{} {}", if summary { "summary" } else { "reply" }, seen.len());
            // Asked to stream (a piped reply): the same content in two pieces, as server-sent events.
            if seen.last().unwrap()["stream"] == true {
                let (first, second) = content.split_at(content.find(' ').unwrap());
                let piece = |text: &str| {
                    format!(
                        "data: {}\n\n",
                        json!({"id": "c", "model": "m", "choices": [{"index": 0, "delta": {"content": text}}]})
                    )
                };
                let events = format!(
                    "{}{}data: {}\n\ndata: [DONE]\n\n",
                    piece(first),
                    piece(second),
                    json!({"id": "c", "model": "m", "choices": [{"index": 0, "delta": {}, "finish_reason": "stop"}], "usage": {"prompt_tokens": 10, "completion_tokens": 2}})
                );
                let _ = write!(
                    &stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{events}",
                    events.len()
                );
                continue;
            }
            let reply =
                json!({"choices": [{"message": {"role": "assistant", "content": content}, "finish_reason": "stop"}],
                "usage": {"prompt_tokens": 10, "completion_tokens": 2}})
                .to_string();
            let _ = write!(
                &stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
                reply.len()
            );
        }
    });
    (url, seen)
}

/// The port a started server says it listens on (`serving on http://127.0.0.1:PORT`, on its
/// standard error). The server picks the port itself (`:0`): a port a test picked and released
/// can be taken by another test before the server binds it.
fn listening(child: &mut Child) -> u16 {
    use std::io::BufRead;
    let mut lines = std::io::BufReader::new(child.stderr.take().unwrap()).lines();
    let port = lines
        .by_ref()
        .map_while(Result::ok)
        .find_map(|line| line.split_once("serving on http://127.0.0.1:")?.1.trim().parse().ok())
        .expect("the server did not start");
    // The rest of what it prints is read and dropped, so the server never waits on a full pipe.
    std::thread::spawn(move || lines.for_each(drop));
    port
}

fn serve(model: &str, dir: &std::path::Path, resume: bool) -> (Child, u16) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_affordance"));
    cmd.arg("serve")
        .arg(root().join("views/demo/memory_server"))
        .args(["--set", &format!("model.url={model}"), "--listen", "127.0.0.1:0"])
        .arg("--dir")
        .arg(dir)
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    if resume {
        cmd.arg("--resume");
    }
    let mut child = cmd.spawn().unwrap();
    let port = listening(&mut child);
    (child, port)
}

/// One HTTP request; the response's status line and body (a chunked body is decoded).
fn request(port: u16, method: &str, path: &str, body: &str) -> (String, String) {
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(
        s,
        "{method} {path} HTTP/1.1\r\nHost: x\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .unwrap();
    let mut raw = String::new();
    s.read_to_string(&mut raw).unwrap();
    let (head, rest) = raw.split_once("\r\n\r\n").unwrap();
    let status = head.lines().next().unwrap().to_string();
    let body = if head.to_ascii_lowercase().contains("transfer-encoding: chunked") {
        let (mut out, mut rest) = (String::new(), rest);
        while let Some((size, after)) = rest.split_once("\r\n") {
            let n = usize::from_str_radix(size.trim(), 16).unwrap();
            if n == 0 {
                break;
            }
            out.push_str(&after[..n]);
            rest = &after[n + 2..];
        }
        out
    } else {
        rest.to_string()
    };
    (status, body)
}

fn chat(port: u16, text: &str) -> String {
    let body = json!({"model": "m", "messages": [{"role": "user", "content": text}]}).to_string();
    let (status, body) = request(port, "POST", "/v1/chat/completions", &body);
    assert!(status.contains("200"), "{status}: {body}");
    let reply: J = serde_json::from_str(&body).unwrap();
    reply["choices"][0]["message"]["content"].as_str().unwrap().to_string()
}

#[test]
fn a_served_app_remembers_past_its_context_and_across_a_restart() {
    let (model, seen) = model_server();
    let dir = std::env::temp_dir().join(format!("aff-serve-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let (mut server, port) = serve(&model, &dir, false);

    let (status, body) = request(port, "GET", "/v1/models", "");
    assert!(
        status.contains("200") && body.contains("affordance-memory"),
        "{status} {body}"
    );
    let (status, _) = request(port, "GET", "/nowhere", "");
    assert!(status.contains("404"), "{status}");

    // Twelve exchanges, the client sending only its newest message each time.
    for n in 0..12 {
        assert!(chat(port, &format!("fact {n}")).starts_with("reply"));
    }
    // Exchanges past KEEP + LEAF were folded: a later prompt carries summaries, not every message.
    let last = seen
        .lock()
        .unwrap()
        .iter()
        .rev()
        .find(|b| {
            !b["messages"][0]["content"]
                .as_str()
                .unwrap_or("")
                .starts_with("Summarize")
        })
        .cloned()
        .unwrap();
    let system = last["messages"][0]["content"].as_str().unwrap().to_string();
    assert!(
        system.contains("What you remember") && system.contains("summary"),
        "{system}"
    );
    assert!(
        !last.to_string().contains("fact 0"),
        "the oldest exchange is folded, not verbatim: {last}"
    );
    assert!(last.to_string().contains("fact 11"), "the newest is there: {last}");

    // A streamed request gets server-sent events, ending with [DONE].
    let body =
        json!({"model": "m", "stream": true, "messages": [{"role": "user", "content": "stream please"}]}).to_string();
    let (status, events) = request(port, "POST", "/v1/chat/completions", &body);
    assert!(
        status.contains("200")
            && events.contains("chat.completion.chunk")
            && events.trim_end().ends_with("data: [DONE]"),
        "{events}"
    );
    // The reply is piped: the model's two pieces reach the client as two chunks, in order, and
    // the whole text is not sent again after them.
    let deltas: Vec<String> = events
        .lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .filter_map(|data| serde_json::from_str::<J>(data).ok())
        .filter_map(|chunk| chunk["choices"][0]["delta"]["content"].as_str().map(str::to_owned))
        .filter(|text| !text.is_empty())
        .collect();
    assert_eq!(deltas.len(), 2, "{events}");
    assert_eq!(deltas[0], "reply", "{events}");
    assert!(
        deltas[1].starts_with(' ') && deltas[1].trim().parse::<u32>().is_ok(),
        "{events}"
    );

    // A restart resumes the same memory: the next prompt still carries the summaries.
    server.kill().unwrap();
    server.wait().unwrap();
    let before = seen.lock().unwrap().len();
    let (mut server, port) = serve(&model, &dir, true);
    assert!(chat(port, "what do you remember?").starts_with("reply"));
    let after: Vec<J> = seen.lock().unwrap()[before..].to_vec();
    let prompt = after
        .iter()
        .find(|b| b.to_string().contains("what do you remember?"))
        .unwrap();
    assert!(
        prompt["messages"][0]["content"]
            .as_str()
            .unwrap()
            .contains("What you remember"),
        "{prompt}"
    );
    server.kill().unwrap();
    server.wait().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
}

/// A served request ends in one of four ways, and each one answers its client: the app answers;
/// the app never answers (504 when `timeout_s` is up); the app leaves a stream open (it is cut at
/// `timeout_s`); the app answers with a status that cannot be sent (the request still waits, and
/// the app's corrected answer reaches the client).
#[test]
fn a_served_request_always_ends() {
    let dir = std::env::temp_dir().join(format!("aff-serve-ends-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("app.tsx"),
        r#"import { useRequests } from "@affordance/lib/http_server.tsx";
export default function Main() {
  useRequests(async (req, res) => {
    if (req.path === "/hang") return;
    if (req.path === "/open") { const out = await res.stream(200, {"content-type": "text/plain"}); await out.write("first\n"); return; }
    if (req.path === "/bad") { try { await res.json({a: 1}, 99); } catch (e) { return await res.json({recovered: true}, 500); } }
    return await res.json({ok: true});
  });
  return "serving";
}"#,
    )
    .unwrap();
    std::fs::write(
        dir.join("affordance.yaml"),
        "root: {app: app.tsx}\nmodel: {kind: chat, url: \"http://127.0.0.1:9/v1\", model: x, context: 1000}\nserve: {listen: \"127.0.0.1:0\", timeout_s: 2}\n",
    )
    .unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_affordance"))
        .arg("serve")
        .current_dir(&dir)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let port = listening(&mut child);
    let (status, body) = request(port, "GET", "/ok", "");
    assert!(
        status.contains("200") && body.contains("\"ok\":true"),
        "{status} {body}"
    );
    let started = Instant::now();
    let (status, _) = request(port, "GET", "/hang", "");
    assert!(status.contains("504"), "{status}");
    assert!(
        started.elapsed() < Duration::from_secs(6),
        "a request that is never answered ends at timeout_s"
    );
    let started = Instant::now();
    let (status, body) = request(port, "GET", "/open", "");
    assert!(status.contains("200") && body.contains("first"), "{status} {body}");
    assert!(
        started.elapsed() < Duration::from_secs(6),
        "a stream left open is cut at timeout_s"
    );
    let (status, body) = request(port, "GET", "/bad", "");
    assert!(status.contains("500") && body.contains("recovered"), "{status} {body}");
    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&dir);
}
