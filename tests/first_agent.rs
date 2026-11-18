//! The first agent of the getting-started guide (examples/first_agent) runs as written: its
//! deployment against a stand-in OpenAI-compatible server (as llama.cpp, Ollama or vLLM serve
//! one), a real local shell, and an app that stops when the model answers.
use serde_json::{Value as J, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// An OpenAI-compatible `/v1/chat/completions` that answers with `replies` in order, and
/// returns the request bodies it saw.
fn server(replies: Vec<J>) -> (String, std::thread::JoinHandle<Vec<J>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    let task = std::thread::spawn(move || {
        let mut seen = vec![];
        for reply in replies {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut length = 0;
            let mut line = String::new();
            while reader.read_line(&mut line).unwrap() > 0 && line != "\r\n" {
                if let Some(n) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    length = n.trim().parse().unwrap();
                }
                line.clear();
            }
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();
            seen.push(serde_json::from_slice(&body).unwrap());
            let text = reply.to_string();
            write!(
                &stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}",
                text.len()
            )
            .unwrap();
        }
        seen
    });
    (url, task)
}

fn call(name: &str, args: J) -> J {
    json!({"choices": [{"message": {"content": null, "tool_calls": [{"id": format!("call_{name}"), "type": "function",
        "function": {"name": name, "arguments": args.to_string()}}]}, "finish_reason": "tool_calls"}],
        "usage": {"prompt_tokens": 10, "completion_tokens": 5}})
}

#[test]
fn the_first_agent_runs_as_the_guide_writes_it() {
    let (url, peer) = server(vec![
        call("shell", json!({"command": "echo 3"})),
        call("answer", json!({"text": "3 files"})),
    ]);
    let dir = std::env::temp_dir().join(format!("aff-first-agent-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // As the guide runs it, from any directory: the example's own file, the server's address for
    // this run, and a journal directory outside the checkout. Its tokenizer is the default.
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_affordance"))
        .arg("run")
        .arg(root().join("examples/first_agent"))
        .args(["--set", &format!("model.url={url}")])
        .arg("--dir")
        .arg(&dir)
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let report: J = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        report["per_agent"][0]["stopped"],
        json!({"answer": "3 files"}),
        "{report}"
    );
    // The model saw the shell's output before it answered.
    let seen = peer.join().unwrap();
    assert!(seen[1].to_string().contains("3\\n"), "{}", seen[1]);
    let _ = std::fs::remove_dir_all(&dir);
}
