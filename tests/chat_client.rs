use crate::tokenizer;
use affordance::{
    emit::Encoder,
    runtime::{Chat, GenRequest, Model, cancel::CancelToken, model::Credentials},
};
use serde_json::{Value as J, json};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

struct Fixture {
    key: PathBuf,
    enc: Arc<Encoder>,
}
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let key = std::env::temp_dir().join(format!(
            "aff-chat-key-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::write(&key, "KEY=local-test-key").unwrap();
        let enc = Arc::new(tokenizer::encoder());
        Self { key, enc }
    }
    fn chat(&self, url: &str) -> Chat {
        let mut c = Chat::new(url, "test", Some(Credentials::new(&self.key, "KEY")), self.enc.clone());
        c.timeout = Duration::from_secs(2);
        c
    }
    fn request(&self) -> GenRequest {
        let ids = self
            .enc
            .tok
            .encode(r#"{"messages":[{"role":"user","content":"hello"}]}"#, false)
            .unwrap()
            .get_ids()
            .to_vec();
        GenRequest::new("a", ids, 10, vec![], None, CancelToken::new())
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.key);
    }
}
fn receive(stream: &mut TcpStream) -> J {
    stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    let mut h = vec![];
    let mut b = [0];
    while !h.ends_with(b"\r\n\r\n") {
        stream.read_exact(&mut b).unwrap();
        h.push(b[0]);
    }
    let text = String::from_utf8(h).unwrap();
    let n: usize = text
        .lines()
        .find_map(|s| {
            s.to_ascii_lowercase()
                .strip_prefix("content-length:")
                .map(|s| s.trim().parse().unwrap())
        })
        .unwrap();
    let mut body = vec![0; n];
    stream.read_exact(&mut body).unwrap();
    serde_json::from_slice(&body).unwrap()
}
fn reply(stream: &mut TcpStream, status: u16, body: &J) {
    let body = body.to_string();
    write!(
        stream,
        "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .unwrap();
}
fn server(status: u16, body: J) -> (String, std::thread::JoinHandle<J>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let req = receive(&mut stream);
        reply(&mut stream, status, &body);
        req
    });
    (url, task)
}
fn valid(cost: f64) -> J {
    json!({"choices":[{"message":{"content":"ok"},"finish_reason":"stop"}],"usage":{"cost":cost}})
}

#[test]
fn concurrent_requests_do_not_share_a_spending_ledger() {
    let fixture = Fixture::new();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let peer = std::thread::spawn(move || {
        // Every request is held open until all four have arrived, so all four are in flight at
        // once (30 s is only the limit for a broken run).
        let until = Instant::now() + Duration::from_secs(30);
        let mut streams = vec![];
        while streams.len() < 4 && Instant::now() < until {
            match listener.accept() {
                Ok((mut s, _)) => {
                    // On macOS an accepted socket inherits the listener's non-blocking mode.
                    s.set_nonblocking(false).unwrap();
                    receive(&mut s);
                    streams.push(s);
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(2)),
                Err(e) => panic!("{e}"),
            }
        }
        let count = streams.len();
        for mut s in streams {
            reply(&mut s, 200, &valid(0.75));
        }
        count
    });
    let chat = Arc::new(fixture.chat(&url));
    let barrier = Arc::new(std::sync::Barrier::new(4));
    let mut jobs = vec![];
    for _ in 0..4 {
        let c = chat.clone();
        let b = barrier.clone();
        let req = fixture.request();
        jobs.push(std::thread::spawn(move || {
            b.wait();
            c.generate(&req)
        }));
    }
    let results: Vec<_> = jobs.into_iter().map(|j| j.join().unwrap()).collect();
    assert_eq!(peer.join().unwrap(), 4);
    for result in results {
        assert_eq!(result.unwrap().model_meta.unwrap()["usage"], json!({"cost":0.75}));
    }
}
#[test]
fn malformed_success_is_an_error_and_http_status_sets_retryability() {
    let fixture = Fixture::new();
    for (status, body, transient) in [
        (200, json!({}), false),
        (200, json!({"usage":{"cost":0},"choices":[]}), false),
        (500, json!({}), true),
        (403, json!({}), false),
    ] {
        let (url, peer) = server(status, body);
        let chat = fixture.chat(&url);
        let error = chat.generate(&fixture.request()).unwrap_err();
        assert_eq!(error.kind.may_pass(), transient, "{error}");
        peer.join().unwrap();
    }
}
#[test]
fn usage_is_preserved_without_requiring_provider_accounting() {
    let fixture = Fixture::new();
    for usage in [
        None,
        Some(
            json!({"cost":99999,"prompt_tokens":17,"completion_tokens":4,"prompt_tokens_details":{"cached_tokens":9},"provider_usage":{"currency":"EUR","raw":true}}),
        ),
    ] {
        let mut response = valid(0.0);
        if let Some(usage) = &usage {
            response["usage"] = usage.clone();
        } else {
            response.as_object_mut().unwrap().remove("usage");
        }
        let (url, peer) = server(200, response);
        let reply = fixture.chat(&url).generate(&fixture.request()).unwrap();
        assert_eq!(reply.model_meta.as_ref().unwrap()["usage"], usage.unwrap_or(J::Null));
        assert!(reply.model_meta.as_ref().unwrap().get("cost_usd").is_none());
        peer.join().unwrap();
    }
}
#[test]
fn per_call_sampling_cannot_override_host_owned_limits() {
    let fixture = Fixture::new();
    let chat = fixture.chat("http://127.0.0.1:1");
    let mut req = fixture.request();
    req.sampling = json!({"max_tokens":100});
    assert!(chat.generate(&req).unwrap_err().message.contains("host-owned"));
}
