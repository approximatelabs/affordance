//! `affordance host`: a deployment driven by a frontend over stdio. The demo coding chat
//! (views/demo/host) asks permission before a command, shows its output as it runs, streams its
//! reply, and stops a turn on `abort`, keeping the partial answer in the journal.
use serde_json::{Value as J, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn chunk(delta: J, finish: Option<&str>) -> String {
    format!(
        "data: {}\n\n",
        json!({"id": "c", "model": "local", "choices": [{"index": 0, "delta": delta, "finish_reason": finish}]})
    )
}

/// A stand-in OpenAI-compatible server that streams: "tell a story" gets words slowly, for as long
/// as the client reads; after the command's output, "Done."; "list files" gets a call to `run`.
fn model_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
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
            assert_eq!(body["stream"], true, "a call with a signal streams");
            let said = body["messages"].to_string();
            let events: Vec<String> = if said.contains("tell a story") {
                (0..500)
                    .map(|i| chunk(json!({"content": format!("word{i} ")}), None))
                    .collect()
            } else if said.contains("hi-from-shell\\n") {
                vec![chunk(json!({"content": "Done."}), None), chunk(json!({}), Some("stop"))]
            } else if said.contains("list files") {
                let call = json!({"tool_calls": [{"index": 0, "id": "call_1", "type": "function",
                    "function": {"name": "run", "arguments": "{\"command\":\"echo hi-from-shell\"}"}}]});
                vec![chunk(call, None), chunk(json!({}), Some("tool_calls"))]
            } else {
                panic!("unexpected request: {said}")
            };
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n"
            );
            for event in events {
                if stream.write_all(event.as_bytes()).and_then(|_| stream.flush()).is_err() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            let _ = stream.write_all(b"data: [DONE]\n\n");
        }
    });
    url
}

struct Frontend {
    child: std::process::Child,
    events: Receiver<J>,
    seen: Vec<J>,
}

impl Frontend {
    fn send(&mut self, command: J) {
        let stdin = self.child.stdin.as_mut().unwrap();
        writeln!(stdin, "{command}").unwrap();
        stdin.flush().unwrap();
    }
    /// Events until one matches, or fail after a while.
    fn until(&mut self, what: &str, matches: impl Fn(&J) -> bool) -> J {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.events.recv_timeout(left) {
                Ok(event) => {
                    self.seen.push(event.clone());
                    if matches(&event) {
                        return event;
                    }
                }
                Err(_) => panic!(
                    "no {what}; saw:\n{}",
                    self.seen.iter().map(J::to_string).collect::<Vec<_>>().join("\n")
                ),
            }
        }
    }
}

fn start(url: &str, dir: &std::path::Path, restore: bool) -> Frontend {
    let mut command = Command::new(env!("CARGO_BIN_EXE_affordance"));
    command
        .arg("host")
        .arg(root().join("views/demo/host"))
        .args(["--set", &format!("model.url={url}")])
        .arg("--dir")
        .arg(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if restore {
        command.arg("--restore");
    }
    let mut child = command.spawn().unwrap();
    let (tx, events) = channel();
    let stdout = child.stdout.take().unwrap();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let line = line.unwrap();
            let event: J = serde_json::from_str(&line).unwrap_or_else(|e| panic!("not JSON on stdout ({e}): {line}"));
            if tx.send(event).is_err() {
                break;
            }
        }
    });
    let mut f = Frontend {
        child,
        events,
        seen: vec![],
    };
    f.until("ready", |e| e["type"] == "ready");
    f
}

#[test]
fn a_frontend_drives_the_coding_chat_over_stdio() {
    let url = model_server();
    let dir = std::env::temp_dir().join(format!("aff-host-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut f = start(&url, &dir, false);
    let ui = |key: &'static str| move |e: &J| e["type"] == "ui" && !e["value"][key].is_null();

    // The call is announced, needs permission, and its output arrives as it runs; then its
    // result, and the reply streams.
    f.send(json!({"type": "event", "name": "user", "payload": {"text": "list files"}}));
    let calls = f.until("the call starting", ui("calls"));
    assert_eq!(calls["value"]["calls"][0]["name"], "run");
    assert_eq!(
        calls["value"]["calls"][0]["args"],
        json!({"command": "echo hi-from-shell"})
    );
    let question = f.until("a question", |e| e["type"] == "question");
    assert_eq!(
        question["ask"],
        json!({"kind": "permission", "tool": "run", "command": "echo hi-from-shell"})
    );
    f.send(json!({"type": "answer", "request": question["request"], "value": "yes"}));
    f.until("the command's output", |e| {
        e["type"] == "output" && e["text"].as_str().is_some_and(|t| t.contains("hi-from-shell"))
    });
    let results = f.until("the result", ui("results"));
    assert_eq!(results["value"]["results"][0]["result"], "hi-from-shell\n");
    assert_eq!(results["value"]["results"][0]["id"], calls["value"]["calls"][0]["id"]);
    f.until("the reply", |e| e["type"] == "delta" && e["text"] == "Done.");
    f.until("idle", |e| e["type"] == "ui" && e["value"]["state"] == "idle");
    assert!(
        f.seen
            .iter()
            .any(|e| e["type"] == "trace" && e["record"]["t"] == "tool")
    );
    // The output is attributed: the trace named the tool call that started the command's task.
    let output = f.seen.iter().find(|e| e["type"] == "output").unwrap().clone();
    let started = f
        .seen
        .iter()
        .find(|e| e["type"] == "trace" && e["record"]["t"] == "call" && e["record"]["task"] == output["task"])
        .expect("a call record for the command (the tool's question to the frontend is another call)");
    assert_eq!(started["record"]["call_id"], calls["value"]["calls"][0]["id"]);
    assert_eq!(started["record"]["tool"], "run");

    // Escape in the middle of a long reply: it stops, and the chat waits for the next message.
    f.send(json!({"type": "event", "name": "user", "payload": {"text": "tell a story"}}));
    f.until("the story", |e| e["type"] == "delta" && e["text"] == "word3 ");
    f.send(json!({"type": "event", "name": "abort"}));
    f.until("idle after the abort", |e| {
        e["type"] == "ui" && e["value"]["state"] == "idle"
    });
    f.send(json!({"type": "close"}));
    f.until("exit", |e| e["type"] == "exit");
    assert!(f.child.wait().unwrap().success());
    let words = f
        .seen
        .iter()
        .filter(|e| e["type"] == "delta" && e["text"].as_str().is_some_and(|t| t.starts_with("word")))
        .count();
    assert!(words < 200, "the stream stopped soon after the abort ({words} words)");

    let journal = std::fs::read_to_string(dir.join("root/journal.jsonl")).unwrap();
    let aborted: Vec<&str> = journal.lines().filter(|l| l.contains(r#""aborted":true"#)).collect();
    assert_eq!(aborted.len(), 1, "{journal}");

    // Closed, not lost: the run resumes from its journal, and a frontend that connects asks for
    // the conversation so far, with the story as it was cut.
    let mut f = start(&url, &dir, true);
    f.send(json!({"type": "event", "name": "history"}));
    let history = f.until("the history", ui("history"));
    let history = history["value"]["history"].as_array().unwrap();
    let kinds: Vec<&str> = history.iter().map(|e| e["kind"].as_str().unwrap()).collect();
    assert_eq!(
        kinds,
        ["input", "model", "action", "model", "input", "model"],
        "{history:?}"
    );
    assert_eq!(history[0]["text"], "list files");
    assert_eq!(history[1]["calls"][0]["name"], "run");
    assert_eq!(history[2]["result"], "hi-from-shell\n");
    assert_eq!(history[3]["text"], "Done.");
    assert_eq!(history[5]["aborted"], true);
    assert!(
        history[5]["text"]
            .as_str()
            .unwrap()
            .starts_with("word0 word1 word2 word3 ")
    );
    f.send(json!({"type": "close"}));
    f.until("exit", |e| e["type"] == "exit");
    assert!(f.child.wait().unwrap().success());
    let _ = std::fs::remove_dir_all(&dir);
}

/// Affordance as another harness's model layer (`views/lib/model_layer.tsx`): the event `generate`
/// carries a conversation and tools; the model server is sent exactly that conversation (no turn
/// of the app's own), the reply streams, and the answer comes back parsed, with the call the model
/// made for the caller to run.
#[test]
fn a_model_layer_serves_another_harnesss_conversation() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    let (seen_tx, seen) = channel::<J>();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
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
            let _ = seen_tx.send(serde_json::from_slice(&body).unwrap());
            let call = json!({"tool_calls": [{"index": 0, "id": "call_9", "type": "function",
                "function": {"name": "bash", "arguments": "{\"command\":\"ls\"}"}}]});
            let events = [
                chunk(json!({"reasoning_content": "Look first."}), None),
                chunk(json!({"content": "Checking."}), None),
                chunk(call, None),
                chunk(json!({}), Some("tool_calls")),
            ];
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n"
            );
            for event in events {
                let _ = stream.write_all(event.as_bytes());
            }
            let _ = stream.write_all(b"data: [DONE]\n\n");
        }
    });
    let dir = std::env::temp_dir().join(format!("aff-layer-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("app.tsx"),
        "import { useModelLayer } from \"@affordance/lib/model_layer.tsx\";\nexport default function Main() { useModelLayer(); return \"\"; }\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("affordance.yaml"),
        format!("root: {{app: app.tsx, model: default}}\nmodel: {{kind: chat, url: \"{url}\", model: local, context: 8192}}\n"),
    )
    .unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_affordance"))
        .arg("host")
        .arg(&dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let (tx, events) = channel();
    let stdout = child.stdout.take().unwrap();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            if tx.send(serde_json::from_str::<J>(&line.unwrap()).unwrap()).is_err() {
                break;
            }
        }
    });
    let mut f = Frontend {
        child,
        events,
        seen: vec![],
    };
    f.until("ready", |e| e["type"] == "ready");
    let messages = json!([
        {"role": "user", "content": "what is here?"},
        {"role": "assistant", "content": "I will look.", "tool_calls": [{"id": "call_1", "name": "bash", "arguments": {"command": "pwd"}}]},
        {"role": "tool", "tool_call_id": "call_1", "name": "bash", "content": "/tmp\n"},
    ]);
    let tools = json!([{"name": "bash", "description": "Run a command.",
        "parameters": {"type": "object", "properties": {"command": {"type": "string"}}, "required": ["command"]}}]);
    f.send(json!({"type": "event", "name": "generate", "payload":
        {"id": "g1", "system": "You are a harness's model.", "messages": messages, "tools": tools, "max_new": 300}}));
    let answer = f.until("the answer", |e| {
        e["type"] == "ui" && e["value"]["generated"]["id"] == "g1"
    });
    let output = &answer["value"]["generated"]["output"];
    assert_eq!(output["text"], "Checking.");
    assert_eq!(output["reasoning"], "Look first.");
    assert_eq!(
        output["calls"],
        json!([{"id": "call_9", "name": "bash", "arguments": {"command": "ls"}}])
    );
    assert_eq!(output["aborted"], false);
    // The reply streamed, reasoning apart from text.
    assert!(
        f.seen
            .iter()
            .any(|e| e["type"] == "delta" && e["reasoning"] == true && e["text"] == "Look first.")
    );
    assert!(
        f.seen
            .iter()
            .any(|e| e["type"] == "delta" && e["reasoning"].is_null() && e["text"] == "Checking.")
    );
    // The server was sent the caller's conversation and nothing else: it ends with the tool result.
    let body = seen.recv_timeout(Duration::from_secs(5)).unwrap();
    let sent = body["messages"].as_array().unwrap();
    let roles: Vec<&str> = sent.iter().map(|m| m["role"].as_str().unwrap()).collect();
    assert_eq!(roles, ["system", "user", "assistant", "tool"], "{body}");
    assert_eq!(sent[0]["content"], "You are a harness's model.");
    assert_eq!(sent[3]["tool_call_id"], "call_1");
    assert_eq!(body["tools"][0]["function"]["name"], "bash");
    assert_eq!(body["max_tokens"], 300);
    f.send(json!({"type": "close"}));
    f.until("exit", |e| e["type"] == "exit");
    let _ = f.child.wait();
    let _ = std::fs::remove_dir_all(&dir);
}
