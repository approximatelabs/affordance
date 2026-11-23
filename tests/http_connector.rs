#![cfg(feature = "http")]
use crate::http_fixture as fixture;
use affordance::{
    connectors::http::{Http, HttpConfig},
    runtime::{CallContext, CancelToken, Executor},
};
use fixture::{Response, Server};
use serde_json::{Value as J, json};
use std::{
    collections::BTreeMap,
    sync::{Arc, atomic::Ordering},
    time::{Duration, Instant},
};

fn context() -> CallContext {
    CallContext {
        agent: "caller".into(),
        task: 7,
        env: BTreeMap::from([("TOKEN".into(), "hidden-token".into())]),
        cancel: CancelToken::new(),
        blobs: None,
        output: None,
    }
}
fn http(url: &str, options: J) -> Http {
    let mut config = json!({"url": url, "timeout_ms": 1000});
    for (key, value) in options.as_object().unwrap() {
        config[key] = value.clone();
    }
    Http::new(serde_json::from_value::<HttpConfig>(config).unwrap()).unwrap()
}

#[test]
fn ordinary_http_is_independent_of_mcp_and_reuses_then_closes_idle_connections() {
    let server = Server::new(|request| {
        assert_eq!(request.headers["authorization"], "Bearer hidden-token");
        let mut response = Response::text(if request.method == "GET" { 200 } else { 409 }, "héllo\n");
        response.headers = vec![("X-Value".into(), "one".into()), ("X-Value".into(), "two".into())];
        response.chunked = true;
        response
    });
    let transport = http(
        &format!("{}/api/", server.url),
        json!({"headers": {"Authorization": {"env": "TOKEN", "prefix": "Bearer "}}}),
    );
    let ctx = context();
    let a = transport.call(&ctx, &json!({"path": "items?q=tea"})).unwrap();
    // The client puts a finished connection back in its pool in the background (as a browser
    // does); the second request is sent once the first connection is idle, so it reuses it.
    std::thread::sleep(Duration::from_millis(200));
    let b = transport
        .call(
            &ctx,
            &json!({"path": "/other", "method": "PATCH", "body": "plain text"}),
        )
        .unwrap();
    assert_eq!(a["status"], 200);
    assert_eq!(a["headers"]["x-value"], json!(["one", "two"]));
    assert_eq!(a["body"], "héllo\n");
    assert_eq!(b["status"], 409, "HTTP errors are data");
    let seen = server.requests.lock().unwrap();
    assert_eq!(seen[0].path, "/api/items?q=tea");
    assert_eq!(
        (seen[1].method.as_str(), seen[1].path.as_str(), seen[1].body.as_str()),
        ("PATCH", "/other", "plain text")
    );
    drop(seen);
    assert_eq!(server.connections.load(Ordering::SeqCst), 1);
    // The pool is the process's (as a browser's is), so an idle connection closes at its idle
    // limit (4 s), whether or not the executor is still there.
    drop(transport);
    let deadline = Instant::now() + Duration::from_secs(8);
    while server.closed.load(Ordering::SeqCst) == 0 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(server.closed.load(Ordering::SeqCst), 1, "the idle connection is closed");
    // Cancelling an old caller must not retain a socket or close a later caller's connection.
    ctx.cancel.cancel("after completion");
}

#[test]
fn authority_and_request_limits_are_enforced_before_network_io() {
    let server = Server::new(|_| Response::text(200, "ok"));
    let transport = http(
        &server.url,
        json!({"headers": {"Authorization": "private"}, "max_request_bytes": 4}),
    );
    let ctx = context();
    for request in [
        json!({"path": "http://example.invalid/"}),
        json!({"path": "//example.invalid/"}),
        json!({"path": "file:///etc/passwd"}),
        json!({"path": "#fragment"}),
        json!({"headers": {"authorization": "replacement"}}),
        json!({"headers": {"Host": "other"}}),
        json!({"headers": {"X-Injected": "value\r\nX-Other: bad"}}),
        json!({"method": "CONNECT"}),
        json!({"timeout_ms": 1001}),
        json!({"timeout_ms": 0}),
        json!({"body": "12345"}),
        json!({"body": {"unexpected": "object"}}),
        json!({"typo": true}),
    ] {
        assert!(transport.call(&ctx, &request).is_err(), "accepted {request}");
    }
    assert!(server.requests.lock().unwrap().is_empty());
    assert!(
        http(&server.url, json!({"headers": {"X-Key": {"env": "MISSING"}}}))
            .call(&ctx, &json!({}))
            .unwrap_err()
            .contains("missing")
    );
    for url in [
        "file:///tmp/data",
        "http://user:pass@localhost/",
        "https://localhost/#part",
    ] {
        let config = serde_json::from_value(json!({"url": url})).unwrap();
        assert!(Http::new(config).is_err());
    }
}

#[test]
fn redirects_are_returned_without_contacting_another_origin() {
    let other = Server::new(|_| panic!("redirect followed"));
    let target = other.url.clone();
    let server = Server::new(move |_| {
        let mut response = Response::text(302, "redirect");
        response.headers.push(("Location".into(), target.clone()));
        response
    });
    let response = http(&server.url, json!({"headers": {"Authorization": "private"}}))
        .call(&context(), &json!({}))
        .unwrap();
    assert_eq!(response["status"], 302);
    assert!(other.requests.lock().unwrap().is_empty());
}

#[test]
fn response_limits_and_utf8_are_checked_without_silent_truncation() {
    let server = Server::new(|request| {
        let mut response = Response::text(200, "12345");
        if request.path == "/chunked" {
            response.chunked = true;
        }
        if request.path == "/invalid" {
            response.body = vec![255];
        }
        response
    });
    let transport = http(&server.url, json!({"max_response_bytes": 4}));
    for path in ["/length", "/chunked", "/invalid"] {
        let error = transport.call(&context(), &json!({"path": path})).unwrap_err();
        assert!(
            error.contains(if path == "/invalid" { "UTF-8" } else { "byte limit" }),
            "{error}"
        );
    }
}

#[test]
fn cancellation_and_deadlines_close_requests_and_leave_the_executor_usable() {
    let server = Server::new(|request| {
        let mut response = Response::text(200, "ok");
        response.stall = request.path != "/ok";
        response
    });
    let transport = Arc::new(http(&server.url, json!({})));
    let cancelled = context();
    cancelled.cancel.cancel("cancelled before starting");
    assert!(
        transport
            .call(&cancelled, &json!({"path": "/ok"}))
            .unwrap_err()
            .starts_with("AbortError:")
    );
    assert!(server.requests.lock().unwrap().is_empty());
    let ctx = context();
    let (worker, call) = (transport.clone(), ctx.clone());
    let thread = std::thread::spawn(move || worker.call(&call, &json!({"path": "/stall"})));
    server.wait_for(1);
    let started = Instant::now();
    ctx.cancel.cancel("test cancellation");
    let error = thread.join().unwrap().unwrap_err();
    assert!(error.starts_with("AbortError:"), "{error}");
    assert!(started.elapsed() < Duration::from_secs(1));
    let error = transport
        .call(&context(), &json!({"path": "/timeout", "timeout_ms": 50}))
        .unwrap_err();
    assert!(error.starts_with("TimeoutError:"), "{error}");
    assert_eq!(
        transport.call(&context(), &json!({"path": "/ok"})).unwrap()["body"],
        "ok"
    );
    let deadline = Instant::now() + Duration::from_secs(2);
    while server.closed.load(Ordering::SeqCst) < 2 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(server.closed.load(Ordering::SeqCst) >= 2, "abandoned sockets stay open");
}

#[test]
fn completed_caller_cannot_cancel_a_later_request_on_its_pooled_connection() {
    let server = Server::new(|request| {
        let mut response = Response::text(200, "ok");
        response.stall = request.path == "/stall";
        response
    });
    let transport = Arc::new(http(&server.url, json!({})));
    let old = context();
    transport.call(&old, &json!({"path": "/ok"})).unwrap();
    // The client returns a finished connection to its pool in the background: give it a moment,
    // so the next request reuses that connection (what this test is about) and opens no new one.
    std::thread::sleep(Duration::from_millis(200));
    let current = context();
    let (worker, call) = (transport.clone(), current.clone());
    let (send, receive) = std::sync::mpsc::channel();
    let thread = std::thread::spawn(move || {
        send.send(worker.call(&call, &json!({"path": "/stall"}))).unwrap();
    });
    server.wait_for(2);
    assert_eq!(server.connections.load(Ordering::SeqCst), 1);
    old.cancel.cancel("old caller ended");
    assert!(
        receive.recv_timeout(Duration::from_millis(40)).is_err(),
        "old token cancelled another caller"
    );
    current.cancel.cancel("current caller ended");
    assert!(
        receive
            .recv_timeout(Duration::from_secs(1))
            .unwrap()
            .unwrap_err()
            .starts_with("AbortError:")
    );
    thread.join().unwrap();
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs the test tokenizers: tools/fetch-test-tokenizers.sh")]
fn binary_configures_http_bindings_with_each_callers_credentials() {
    let server = Server::new(|request| {
        let caller = request.body.as_str();
        assert_eq!(request.headers["authorization"], format!("Bearer {caller}-private"));
        Response::json(json!({"caller": caller}))
    });
    let dir = std::env::temp_dir().join(format!("aff-http-bin-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let app = dir.join("app.tsx");
    std::fs::write(
        &app,
        r#"export default function Main() {
useEffect(() => { (async () => {
 const response = await harness.machine.get("api").call({method: "POST", body: harness.agent});
 harness.stop(JSON.parse(response.body));
})(); }, []); return "waiting";
}"#,
    )
    .unwrap();
    let config = json!({
        "app": {"view": app, "tokenizer": concat!(env!("CARGO_MANIFEST_DIR"), "/ref/tokenizers/Qwen__Qwen3-4B-Instruct-2507.json")},
        "model": {"kind": "sglang", "url": "http://127.0.0.1:1", "context": "none"},
        "machines": {"api": {"kind": "http", "url": server.url, "headers": {"Authorization": {"env": "TOKEN", "prefix": "Bearer "}}}},
        "agents": [{"id": "alice", "model": null, "env": {"TOKEN": "alice-private"}},
                   {"id": "bob", "model": null, "env": {"TOKEN": "bob-private"}}],
        "secrets": ["TOKEN"], "dir": dir.join("run"), "max_wall_s": 5
    });
    let path = dir.join("config.json");
    std::fs::write(&path, config.to_string()).unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_affordance"))
        .arg("run")
        .arg(path)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let report: J = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["complete"], true, "{report}");
    assert_eq!(server.requests.lock().unwrap().len(), 2);
    for id in ["alice", "bob"] {
        let journal = std::fs::read_to_string(dir.join(format!("run/{id}/journal.jsonl"))).unwrap();
        assert!(!journal.contains("-private"));
        assert!(journal.contains(id));
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn head_may_advertise_a_large_representation_without_exceeding_body_limit() {
    let server = Server::new(|_| Response::text(200, "a large representation"));
    let response = http(&server.url, json!({"max_response_bytes": 1}))
        .call(&context(), &json!({"method": "HEAD"}))
        .unwrap();
    assert_eq!(response["body"], "");
}
