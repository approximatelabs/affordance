//! Hosted Messages/Responses bindings at offline HTTP endpoints: the request mapping from
//! `MessagesApp` turns, prompt-cache breakpoints, usage, retries and configuration. A live
//! smoke tests run only with AFFORDANCE_ANTHROPIC_SMOKE=1 or AFFORDANCE_RESPONSES_SMOKE=1.
use crate::tokenizer;
use affordance::{
    emit::Encoder,
    runtime::{
        Chat, GenRequest, Model,
        cancel::CancelToken,
        model::{Credentials, FailureKind, from_config},
    },
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
    time::Duration,
};

struct Fixture {
    key: PathBuf,
    enc: Arc<Encoder>,
}
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let key = std::env::temp_dir().join(format!(
            "aff-anthropic-key-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::write(&key, "KEY=fixture-anthropic-secret").unwrap();
        Self {
            key,
            enc: Arc::new(tokenizer::encoder()),
        }
    }
    fn client(&self, url: &str) -> Chat {
        let mut c = Chat::anthropic(
            url,
            "claude-test",
            Some(Credentials::new(self.key.to_str().unwrap(), "KEY")),
            self.enc.clone(),
        );
        c.timeout = Duration::from_secs(5);
        c
    }
    /// A turn request whose prompt is `MessagesApp`'s JSON line.
    fn request(&self, doc: &J) -> GenRequest {
        let ids = self
            .enc
            .tok
            .encode(format!("{doc}\n").as_str(), false)
            .unwrap()
            .get_ids()
            .to_vec();
        GenRequest::new("a", ids, 64, vec![], None, CancelToken::new())
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.key);
    }
}

struct Seen {
    head: String,
    body: J,
}
/// One connection, waited for at most 30 s. A test whose client never connects (its deadline
/// passed first, on a busy machine) fails saying so; with a plain `accept` it waited for ever.
fn accept(listener: &TcpListener) -> (std::net::TcpStream, std::net::SocketAddr) {
    listener.set_nonblocking(true).unwrap();
    let started = std::time::Instant::now();
    loop {
        match listener.accept() {
            Ok((stream, peer)) => {
                listener.set_nonblocking(false).unwrap();
                stream.set_nonblocking(false).unwrap();
                return (stream, peer);
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                assert!(
                    started.elapsed() < std::time::Duration::from_secs(30),
                    "no connection arrived in 30 s"
                );
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            Err(e) => panic!("accept: {e}"),
        }
    }
}

fn receive(stream: &mut TcpStream) -> Seen {
    stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let mut h = vec![];
    let mut b = [0];
    while !h.ends_with(b"\r\n\r\n") {
        stream.read_exact(&mut b).unwrap();
        h.push(b[0]);
    }
    let head = String::from_utf8(h).unwrap();
    let n: usize = head
        .lines()
        .find_map(|s| {
            s.to_ascii_lowercase()
                .strip_prefix("content-length:")
                .map(|s| s.trim().parse().unwrap())
        })
        .unwrap();
    let mut body = vec![0; n];
    stream.read_exact(&mut body).unwrap();
    Seen {
        head,
        body: serde_json::from_slice(&body).unwrap(),
    }
}
/// A server answering each request in turn with `(status, extra headers, body)`.
fn server(replies: Vec<(u16, &'static str, J)>) -> (String, std::thread::JoinHandle<Vec<Seen>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = std::thread::spawn(move || {
        replies
            .into_iter()
            .map(|(status, extra, body)| {
                let (mut stream, _) = accept(&listener);
                let seen = receive(&mut stream);
                let body = body.to_string();
                write!(
                    stream,
                    "HTTP/1.1 {status} Test\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .unwrap();
                seen
            })
            .collect()
    });
    (url, task)
}
fn message(content: J, stop: &str, usage: J) -> J {
    json!({"id": "msg_test", "type": "message", "role": "assistant", "model": "claude-test", "content": content, "stop_reason": stop, "usage": usage})
}
fn ok() -> J {
    message(
        json!([{"type": "text", "text": "ok"}]),
        "end_turn",
        json!({"input_tokens": 3, "output_tokens": 1}),
    )
}
/// Every block in the request that carries a cache breakpoint, as a path.
fn breakpoints(body: &J) -> Vec<String> {
    let mut out = vec![];
    let mut walk = vec![(String::new(), body)];
    while let Some((path, v)) = walk.pop() {
        match v {
            J::Object(o) => {
                if o.contains_key("cache_control") {
                    out.push(path.clone());
                }
                walk.extend(o.iter().map(|(k, v)| (format!("{path}.{k}"), v)));
            }
            J::Array(a) => walk.extend(a.iter().enumerate().map(|(i, v)| (format!("{path}[{i}]"), v))),
            _ => {}
        }
    }
    out.sort();
    out
}

const PNG: &str = "data:image/png;base64,iVBORw0KGgo=";

/// A tool-using session as `MessagesApp` writes it: system, task, a reply with two calls (object
/// and string arguments), their results (one with an image), then the current screen.
fn session() -> J {
    json!({
        "messages": [
            {"role": "system", "content": "You operate the app."},
            {"role": "user", "content": "Find the big number."},
            {"role": "assistant", "content": "Looking.", "reasoning_content": "think first", "tool_calls": [
                {"type": "function", "id": "call_2_0", "function": {"name": "scroll", "arguments": {"by": 3}}},
                {"type": "function", "id": "call_2_1", "function": {"name": "look", "arguments": "{\"at\":\"top\"}"}}]},
            {"role": "tool", "name": "scroll", "tool_call_id": "call_2_0", "content": "scrolled"},
            {"role": "tool", "name": "look", "tool_call_id": "call_2_1", "content": [{"type": "text", "text": "a picture"}, {"type": "image_url", "image_url": {"url": PNG}}]},
            {"role": "user", "content": [{"type": "text", "text": "screen 2"}, {"type": "image_url", "image_url": {"url": "/9j/4AAQSkZJRg=="}}]}
        ],
        "tools": [
            {"type": "function", "function": {"name": "scroll", "description": "Scroll.", "parameters": {"type": "object", "properties": {"by": {"type": "integer"}}}}},
            {"type": "function", "function": {"name": "look", "parameters": {"type": "object", "properties": {"at": {"type": "string"}}}}}
        ]
    })
}

#[test]
fn turns_map_to_messages_with_paired_tool_results_images_and_three_breakpoints() {
    let fixture = Fixture::new();
    let (url, peer) = server(vec![(200, "", ok())]);
    let mut client = fixture.client(&url);
    client
        .headers
        .insert("anthropic-beta".into(), "thinking-binding-controls-2026-08-01".into());
    client.provider_options =
        json!({"thinking": {"type": "adaptive", "block_binding": {"prefix_mismatch_behavior": "drop_block"}}});
    client.sampling = json!({"temperature": 0.5});
    let mut doc = session();
    doc["messages"][2]["provider_state"] = json!(
        json!({"kind":"anthropic","model":"claude-test","content":[
            {"type":"thinking","thinking":"think first","signature":"signed"},
            {"type":"text","text":"Looking."},
            {"type":"tool_use","id":"call_2_0","name":"scroll","input":{"by":3}},
            {"type":"tool_use","id":"call_2_1","name":"look","input":{"at":"top"}}
        ]})
        .to_string()
    );
    client.generate(&fixture.request(&doc)).unwrap();
    let seen = peer.join().unwrap().remove(0);
    assert!(seen.head.starts_with("POST /v1/messages "), "{}", seen.head);
    let headers = seen.head.to_ascii_lowercase();
    assert!(headers.contains("x-api-key: fixture-anthropic-secret\r\n"));
    assert!(headers.contains("anthropic-version: 2023-06-01\r\n"));
    assert!(headers.contains("anthropic-beta: thinking-binding-controls-2026-08-01\r\n"));
    assert!(!headers.contains("authorization"));
    let mark = json!({"type": "ephemeral", "ttl": "5m"});
    let expected = json!({
        "model": "claude-test", "max_tokens": 64, "temperature": 0.5,
        "thinking": {"type": "adaptive", "block_binding": {"prefix_mismatch_behavior": "drop_block"}},
        "system": [{"type": "text", "text": "You operate the app.", "cache_control": mark}],
        "tools": [
            {"name": "scroll", "description": "Scroll.", "input_schema": {"type": "object", "properties": {"by": {"type": "integer"}}}},
            {"name": "look", "input_schema": {"type": "object", "properties": {"at": {"type": "string"}}}, "cache_control": mark}
        ],
        "messages": [
            {"role": "user", "content": [{"type": "text", "text": "Find the big number."}]},
            {"role": "assistant", "content": [
                {"type":"thinking","thinking":"think first","signature":"signed"},
                {"type": "text", "text": "Looking."},
                {"type": "tool_use", "id": "call_2_0", "name": "scroll", "input": {"by": 3}},
                {"type": "tool_use", "id": "call_2_1", "name": "look", "input": {"at": "top"}}]},
            {"role": "user", "content": [
                {"type": "tool_result", "tool_use_id": "call_2_0", "content": [{"type": "text", "text": "scrolled"}]},
                {"type": "tool_result", "tool_use_id": "call_2_1", "content": [
                    {"type": "text", "text": "a picture"},
                    {"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": "iVBORw0KGgo="}}],
                 "cache_control": mark},
                {"type": "text", "text": "screen 2"},
                {"type": "image", "source": {"type": "base64", "media_type": "image/jpeg", "data": "/9j/4AAQSkZJRg=="}}]}
        ]
    });
    assert_eq!(seen.body, expected);
}

#[test]
fn the_cached_prefix_survives_a_blanked_screen_and_calls_without_result_ids_pair_by_position() {
    let fixture = Fixture::new();
    let (url, peer) = server(vec![(200, "", ok()), (200, "", ok())]);
    let mut client = fixture.client(&url);
    client.api = affordance::runtime::Api::Anthropic { cache_ttl: "1h".into() };
    let mut doc = session();
    client.generate(&fixture.request(&doc)).unwrap();
    // The next turn: the app blanks the old screen, the model called once more (no result id).
    let turns = doc["messages"].as_array_mut().unwrap();
    turns[5]["content"] = json!("");
    turns.push(json!({"role": "assistant", "content": "", "tool_calls": [{"type": "function", "function": {"name": "scroll", "arguments": {}}}]}));
    turns.push(json!({"role": "tool", "name": "scroll", "content": "done"}));
    turns.push(json!({"role": "user", "content": "screen 3"}));
    client.generate(&fixture.request(&doc)).unwrap();
    let seen = peer.join().unwrap();
    let (first, second) = (&seen[0].body, &seen[1].body);
    // The first request's breakpoint is the second request's prefix, byte for byte.
    let unmarked = |v: &J| -> J {
        let mut v = v.clone();
        let c = v.as_array_mut().unwrap();
        for b in c.iter_mut() {
            b.as_object_mut().unwrap().remove("cache_control");
        }
        v
    };
    let prefix =
        |body: &J, upto: usize| -> J { unmarked(&json!(body["messages"][2]["content"].as_array().unwrap()[..upto])) };
    assert_eq!(prefix(first, 2), prefix(second, 2));
    assert_eq!(
        first["messages"].as_array().unwrap()[..2],
        second["messages"].as_array().unwrap()[..2]
    );
    assert_eq!(
        breakpoints(first),
        [".messages[2].content[1]", ".system[0]", ".tools[1]"]
    );
    assert_eq!(
        second["messages"][2]["content"].as_array().unwrap().len(),
        2,
        "the blanked screen is no block"
    );
    assert_eq!(
        second["messages"][3]["content"],
        json!([{"type": "tool_use", "id": "call_6_0", "name": "scroll", "input": {}}])
    );
    assert_eq!(second["messages"][4]["content"][0]["tool_use_id"], "call_6_0");
    assert_eq!(
        second["messages"][4]["content"][0]["cache_control"],
        json!({"type": "ephemeral", "ttl": "1h"})
    );
    assert_eq!(breakpoints(second).len(), 3);
}

/// An image a request carries is its bytes (a data URL, or base64 sent as one), never a URL or
/// a path the provider would fetch for the agent: those fail before any request is sent.
#[test]
fn image_sources_are_data_never_urls_or_paths() {
    let fixture = Fixture::new();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    for api in [
        affordance::runtime::Api::Chat,
        affordance::runtime::Api::Anthropic { cache_ttl: "5m".into() },
    ] {
        let mut client = fixture.client(&url);
        client.api = api;
        for src in [
            "https://attacker.example/x.png?d=secret",
            "file:///etc/x.png",
            "/x.png",
            "x.png",
            "data:text/html;base64,PGI+",
        ] {
            let mut doc = session();
            doc["messages"][5]["content"][1]["image_url"]["url"] = json!(src);
            let error = client.generate(&fixture.request(&doc)).unwrap_err();
            assert!(
                !error.kind.may_pass() && error.message.contains("an image source must be a data URL"),
                "{src}: {}",
                error.message
            );
        }
    }
    assert!(listener.accept().is_err(), "no request was sent");
}

/// A provider that takes its key in another header (Azure OpenAI's `api-key`) gets it there,
/// read from the key file like the bearer key and redacted the same way; a name outside the
/// known credential headers is refused, and `headers` (non-secret) cannot carry one.
#[test]
fn a_key_header_carries_the_key_where_the_provider_reads_it() {
    let fixture = Fixture::new();
    let (url, peer) = server(vec![(
        200,
        "",
        json!({"choices": [{"message": {"content": "ok"}, "finish_reason": "stop"}], "usage": {"prompt_tokens": 3, "completion_tokens": 1}}),
    )]);
    let mut client = fixture.client(&url);
    client.api = affordance::runtime::Api::Chat;
    client.key_header = Some("api-key".into());
    client.generate(&fixture.request(&session())).unwrap();
    let head = peer.join().unwrap().remove(0).head.to_ascii_lowercase();
    assert!(
        head.contains("api-key: fixture-anthropic-secret\r\n") && !head.contains("authorization"),
        "{head}"
    );
    client.key_header = Some("x-secret".into());
    assert!(
        client
            .generate(&fixture.request(&session()))
            .unwrap_err()
            .message
            .contains("key_header \"x-secret\"")
    );
    client.key_header = None;
    client.headers.insert("Api-Key".into(), "visible".into());
    assert!(
        client
            .generate(&fixture.request(&session()))
            .unwrap_err()
            .message
            .contains("client-owned")
    );
}

#[test]
fn usage_reports_cached_and_uncached_input_and_thinking_becomes_reasoning() {
    let fixture = Fixture::new();
    let usage = json!({"input_tokens": 10, "cache_creation_input_tokens": 100, "cache_read_input_tokens": 1000, "output_tokens": 5, "output_tokens_details":{"thinking_tokens":3},
        "cache_creation": {"ephemeral_5m_input_tokens": 100, "ephemeral_1h_input_tokens": 0}});
    let content = json!([
        {"type": "thinking", "thinking": "count first", "signature": "sig"},
        {"type": "text", "text": "Scrolling."},
        {"type": "tool_use", "id": "toolu_1", "name": "scroll", "input": {"by": 3}}
    ]);
    let (url, peer) = server(vec![(200, "", message(content.clone(), "tool_use", usage.clone()))]);
    let reply = fixture.client(&url).generate(&fixture.request(&session())).unwrap();
    peer.join().unwrap();
    assert!(!reply.cut_off);
    assert_eq!(reply.cached_tokens, Some(1000));
    assert_eq!(reply.request_id.as_deref(), Some("msg_test"));
    let mut expected = usage;
    expected["prompt_tokens"] = json!(1110);
    expected["completion_tokens"] = json!(5);
    expected["completion_tokens_details"] = json!({"thinking_tokens":3,"reasoning_tokens":3});
    expected["prompt_tokens_details"] = json!({"cached_tokens": 1000, "cache_write_tokens":100});
    assert_eq!(
        reply.model_meta.unwrap(),
        json!({"usage": expected, "model": "claude-test"})
    );
    assert_eq!(
        reply_json(&fixture.enc, &reply.ids),
        json!({"reasoning": "count first", "content": "Scrolling.", "tool_calls": [{"id":"toolu_1", "name": "scroll", "arguments": {"by": 3}}], "provider_state":json!({"kind":"anthropic", "model":"claude-test", "content":content}).to_string()})
    );
}

#[test]
fn overload_and_rate_limits_retry_after_the_providers_wait() {
    let fixture = Fixture::new();
    let (url, peer) = server(vec![
        (
            529,
            "retry-after: 0\r\n",
            json!({"type": "error", "error": {"type": "overloaded_error"}}),
        ),
        (
            429,
            "retry-after: 0\r\n",
            json!({"type": "error", "error": {"type": "rate_limit_error"}}),
        ),
        (200, "", ok()),
    ]);
    // The client makes one attempt and reports the kind and the wait the provider asked for;
    // the caller's retry policy makes the next.
    let client = fixture.client(&url);
    for status in ["529", "429"] {
        let error = client.generate(&fixture.request(&session())).unwrap_err();
        assert_eq!(
            (error.kind, error.retry_after),
            (FailureKind::Busy, Some(Duration::ZERO)),
            "{error}"
        );
        assert!(
            error.message.starts_with(&format!("anthropic HTTP {status}")),
            "{error}"
        );
    }
    assert_eq!(client.generate(&fixture.request(&session())).unwrap().attempts, 1);
    assert_eq!(peer.join().unwrap().len(), 3);

    // A rejected request is not resent, and a provider echo of the key stays private.
    let (url, peer) = server(vec![(
        400,
        "",
        json!({"type": "error", "error": {"message": "bad key fixture-anthropic-secret"}}),
    )]);
    let error = fixture.client(&url).generate(&fixture.request(&session())).unwrap_err();
    assert!(
        !error.kind.may_pass()
            && !error.message.contains("fixture-anthropic-secret")
            && error.message.contains("[redacted]"),
        "{error}"
    );
    assert_eq!(peer.join().unwrap().len(), 1);
}

#[test]
fn configuration_defaults_identity_and_protected_fields() {
    let fixture = Fixture::new();
    let base = json!({"kind": "anthropic", "context": "none", "model": "claude-test", "key_file": fixture.key, "key_name": "KEY"});
    let client = from_config(&base, fixture.enc.clone()).unwrap();
    assert_eq!(client.template().unwrap().name, "messages");
    let identity = client.binding_identity().unwrap();
    assert_eq!(identity["client"]["kind"], "anthropic");
    assert_eq!(identity["client"]["url"], "https://api.anthropic.com");
    assert_eq!(identity["client"]["cache_ttl"], "5m");
    let mut ttl = base.clone();
    ttl["cache_ttl"] = json!("1h");
    assert_ne!(
        from_config(&ttl, fixture.enc.clone()).unwrap().binding_identity(),
        Some(identity)
    );
    for (key, value) in [
        ("cache_ttl", json!("10m")),
        ("stream", json!(true)),
        ("template", json!("glm53")),
        ("headers", json!({"anthropic-version": "2024-01-01"})),
        ("headers", json!({"X-Api-Key": "secret"})),
        ("provider_options", json!({"system": "replaced"})),
        ("provider_options", json!({"stop_sequences": ["x"]})),
        ("sampling", json!({"max_tokens": 5})),
        ("ranks", json!(2)),
    ] {
        let mut bad = base.clone();
        bad[key] = value;
        assert!(from_config(&bad, fixture.enc.clone()).is_err(), "{bad}");
    }
    let mut no_key = base.clone();
    no_key.as_object_mut().unwrap().remove("key_file");
    assert!(from_config(&no_key, fixture.enc.clone()).is_err());
    // A chat binding still accepts provider headers that are not credentials.
    let chat = json!({"kind": "chat", "context": "none", "url": "https://example.test/v1", "model": "m", "key_file": fixture.key, "key_name": "KEY",
        "headers": {"anthropic-version": "2023-06-01"}});
    assert!(from_config(&chat, fixture.enc.clone()).is_ok());
    let mut stored_chat = chat.clone();
    stored_chat["provider_options"] = json!({"store":false});
    assert!(from_config(&stored_chat, fixture.enc.clone()).is_ok());
}

fn reply_json(enc: &Encoder, ids: &[u32]) -> J {
    serde_json::from_str(&enc.tok.decode(ids, false).unwrap()).unwrap()
}

fn responses(output: J) -> J {
    json!({"id":"resp_test", "model":"gpt-test", "status":"completed", "output":output,
        "usage":{"input_tokens":2000,"input_tokens_details":{"cached_tokens":1800,"cache_write_tokens":200},
            "output_tokens":40,"output_tokens_details":{"reasoning_tokens":32}}})
}

#[test]
fn responses_preserves_encrypted_output_call_ids_images_options_and_usage() {
    let f = Fixture::new();
    let output = json!([
        {"type":"reasoning", "id":"rs_1", "summary":[{"type":"summary_text","text":"Check it."}], "encrypted_content":"opaque+/=\n☃", "future_field":{"n":9007199254740993u64}},
        {"type":"message", "id":"msg_1", "role":"assistant", "content":[{"type":"output_text","text":"Looking.","annotations":[]}]},
        {"type":"function_call", "id":"fc_item", "call_id":"call_native", "name":"look", "arguments":"{\"at\":\"top\"}","status":"completed"}
    ]);
    let (url, peer) = server(vec![
        (200, "", responses(output.clone())),
        (200, "", responses(json!([]))),
    ]);
    let mut client = Chat::responses(
        &url,
        "gpt-test",
        Some(Credentials::new(f.key.to_str().unwrap(), "KEY")),
        f.enc.clone(),
    );
    client.provider_options = json!({"reasoning":{"effort":"high","context":"all_turns"}, "text":{"verbosity":"low"}, "prompt_cache_options":{"ttl":"30m"}, "service_tier":"default"});
    let mut doc = json!({"messages":[{"role":"system","content":"Stable instructions"},{"role":"user","content":"Screen one"}],
        "tools":[{"type":"function","function":{"name":"look","parameters":{"type":"object","properties":{"at":{"type":"string"}},"required":["at"]}}}]});
    let reply = client.generate(&f.request(&doc)).unwrap();
    let parsed = reply_json(&f.enc, &reply.ids);
    assert_eq!(parsed["tool_calls"][0]["id"], "call_native");
    assert_eq!(parsed["reasoning"], "Check it.");
    let state: J = serde_json::from_str(parsed["provider_state"].as_str().unwrap()).unwrap();
    assert_eq!(state["output"], output);
    let usage = &reply.model_meta.as_ref().unwrap()["usage"];
    assert_eq!(usage["input_tokens"], 2000);
    assert_eq!(usage["prompt_tokens"], 2000);
    assert_eq!(usage["prompt_tokens_details"], usage["input_tokens_details"]);
    assert_eq!(usage["completion_tokens_details"]["reasoning_tokens"], 32);
    assert_eq!(reply.cached_tokens, Some(1800));
    doc["messages"].as_array_mut().unwrap().extend([
        json!({"role":"assistant","content":"Looking.","provider_state":parsed["provider_state"],"tool_calls":[{"id":"call_native","function":{"name":"look","arguments":{"at":"top"}}}]}),
        json!({"role":"tool","tool_call_id":"call_native","content":[{"type":"text","text":"picture"},{"type":"image_url","image_url":{"url":PNG}}]}),
        json!({"role":"user","content":"Screen two"})]);
    client.generate(&f.request(&doc)).unwrap();
    let seen = peer.join().unwrap();
    assert!(seen[0].head.starts_with("POST /responses "));
    assert!(
        seen[0]
            .head
            .to_ascii_lowercase()
            .contains("authorization: bearer fixture-anthropic-secret")
    );
    let (first, second) = (&seen[0].body, &seen[1].body);
    assert_eq!(first["store"], false);
    assert_eq!(first["include"], json!(["reasoning.encrypted_content"]));
    assert_eq!(first["max_output_tokens"], 64);
    assert!(first.get("max_tokens").is_none());
    assert_eq!(first["tools"][0]["strict"], false);
    assert_eq!(first["text"]["verbosity"], "low");
    assert_eq!(first["prompt_cache_key"], second["prompt_cache_key"]);
    assert_eq!(first["prompt_cache_key"].as_str().unwrap().len(), 64);
    assert_eq!(second["input"].as_array().unwrap()[2..5], *output.as_array().unwrap());
    assert_eq!(
        second["input"][5],
        json!({"type":"function_call_output","call_id":"call_native","output":[{"type":"input_text","text":"picture"},{"type":"input_image","image_url":PNG}]})
    );
}

#[test]
fn responses_configuration_and_terminal_states_are_explicit() {
    let f = Fixture::new();
    let base = json!({"kind":"responses","context":"none","model":"gpt-test","key_file":f.key,"key_name":"KEY","provider_options":{"reasoning":{"effort":"high"},"text":{"verbosity":"low"}}});
    let identity = from_config(&base, f.enc.clone()).unwrap().binding_identity().unwrap();
    assert_eq!(identity["client"]["url"], "https://api.openai.com/v1");
    assert_eq!(identity["client"]["kind"], "responses");
    for (key, value) in [
        ("strict_tools", json!("true")),
        ("stream", json!(true)),
        ("template", json!("glm53")),
        ("provider_options", json!({"store":true})),
        ("provider_options", json!({"input":[]})),
        ("provider_options", json!({"reasoning":"high"})),
        ("provider_options", json!({"prompt_cache_key":""})),
    ] {
        let mut config = base.clone();
        config[key] = value;
        assert!(from_config(&config, f.enc.clone()).is_err(), "{config}");
    }
    let mut changed = base.clone();
    changed["provider_options"]["reasoning"]["effort"] = json!("low");
    assert_ne!(
        from_config(&changed, f.enc.clone()).unwrap().binding_identity(),
        Some(identity)
    );
    let mut incomplete = responses(
        json!([{"type":"reasoning","encrypted_content":"opaque","summary":[]},{"type":"function_call","arguments":"{"}]),
    );
    incomplete["status"] = json!("incomplete");
    incomplete["incomplete_details"] = json!({"reason":"max_output_tokens"});
    let failed = json!({"status":"failed","error":{"message":"fixture-anthropic-secret rejected"}});
    let (url, peer) = server(vec![(200, "", incomplete), (200, "", failed)]);
    let client = Chat::responses(
        &url,
        "gpt-test",
        Some(Credentials::new(f.key.to_str().unwrap(), "KEY")),
        f.enc.clone(),
    );
    let reply = client.generate(&f.request(&session())).unwrap();
    assert!(reply.cut_off);
    let normalized = reply_json(&f.enc, &reply.ids);
    assert_eq!(normalized["tool_calls"], json!([]));
    let state: J = serde_json::from_str(normalized["provider_state"].as_str().unwrap()).unwrap();
    assert_eq!(
        state["output"],
        json!([{"type":"reasoning","encrypted_content":"opaque","summary":[]}])
    );
    let error = client.generate(&f.request(&session())).unwrap_err();
    assert!(!error.kind.may_pass() && !error.message.contains("fixture-anthropic-secret"));
    peer.join().unwrap();
    let content = json!([{"type":"thinking","thinking":"not finished","signature":"sig"},
        {"type":"tool_use","id":"aborted","name":"look","input":{}}]);
    let (url, peer) = server(vec![(200, "", message(content, "max_tokens", json!({})))]);
    let reply = f.client(&url).generate(&f.request(&session())).unwrap();
    assert!(reply.cut_off);
    let normalized = reply_json(&f.enc, &reply.ids);
    assert_eq!(normalized["tool_calls"], json!([]));
    let state: J = serde_json::from_str(normalized["provider_state"].as_str().unwrap()).unwrap();
    assert_eq!(
        state["content"],
        json!([{"type":"thinking","thinking":"not finished","signature":"sig"}])
    );
    peer.join().unwrap();
}

// This runs the real app log, turn projection, parser and dispatch. The second wire request
// must contain the exact first screen and native assistant output. Recovery must do no HTTP.
fn app_session(f: &Fixture, mut client: Chat, system: &str) -> (J, Vec<J>) {
    client.window = affordance::runtime::Window::Unbounded;
    struct Recorded {
        client: Chat,
        usages: Arc<std::sync::Mutex<Vec<J>>>,
    }
    impl Model for Recorded {
        fn validate(&self) -> Result<(), affordance::runtime::model::ModelError> {
            self.client.validate()
        }
        fn template(&self) -> Option<affordance::runtime::model::ModelTemplate> {
            self.client.template()
        }
        fn binding_identity(&self) -> Option<J> {
            self.client.binding_identity()
        }
        fn generate(
            &self,
            req: &GenRequest,
        ) -> Result<affordance::runtime::GenReply, affordance::runtime::model::ModelError> {
            let reply = self.client.generate(req)?;
            self.usages
                .lock()
                .unwrap()
                .push(reply.model_meta.as_ref().unwrap()["usage"].clone());
            Ok(reply)
        }
    }
    let usages = Arc::new(std::sync::Mutex::new(vec![]));
    use affordance::runtime::{AgentSpec, Runtime, RuntimeConfig};
    let dir = f.key.with_extension("session");
    std::fs::create_dir_all(&dir).unwrap();
    let app = dir.join("app.tsx");
    let views = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("views");
    let source=r#"
      import {useLog} from "$V/lib/log.tsx";
      import {ModelContext} from "$V/lib/model.tsx";
      import {Tool} from "$V/lib/tool.tsx";
      export default function Main({system}) {
        const log=useLog();const [n,setN]=useState(0,'n');
        useEffect(()=>{(async()=>{
          for(let i=0;i<3;i++){
            const out=await log.call();
            if(out.cut_off || out.calls.length!==1)throw new Error('expected one complete tool call');
            await log.dispatch(out);
          }
          harness.stop({done:true});
        })().catch(e=>harness.stop({error:e.message}));},[]);
        return <ModelContext log={log.records} strategy="unrolled" system={system}>
          {`Screen ${n}: use increment.\n`}
          <Tool name="increment" description="Increment the counter. Always call this tool." schema={{type:'object',properties:{}}}
            fn={()=>{setN(x=>x+1);return 'incremented';}}>{'[increment]\n'}</Tool>
        </ModelContext>;
      }
    "#.replace("$V",views.to_str().unwrap());
    std::fs::write(&app, source).unwrap();
    let mut cfg = RuntimeConfig::new(&app);
    cfg.workers = 1;
    cfg.model_threads = 1;
    cfg.exec_threads = 1;
    cfg.library = Some(env!("CARGO_MANIFEST_DIR").into());
    cfg.dir = Some(dir.join("run"));
    cfg.strict_replay = true;
    cfg.max_wall = Some(Duration::from_secs(180));
    let model: Arc<dyn Model> = Arc::new(Recorded {
        client,
        usages: usages.clone(),
    });
    let mut runtime = Runtime::new(cfg.clone(), f.enc.clone(), model.clone());
    let mut spec = AgentSpec::new("a", json!({"system":system}));
    spec.max_new = 4096;
    runtime.add_agent(spec).unwrap();
    let first = runtime.run().unwrap();
    let report = &first.agents[0];
    assert!(report.error.is_none(), "{:?}", report.error);
    assert_eq!(report.stopped, Some(json!({"done":true})), "{:?}", report.stopped);
    assert_eq!(report.state["__ui"]["@n"], 3);
    let state = report.state.clone();
    let restored = Runtime::restore(cfg.clone(), f.enc.clone(), model.clone())
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(restored.agents[0].state, state);
    std::fs::remove_file(dir.join("run/a/checkpoint.json")).unwrap();
    let replay = Runtime::restore(cfg, f.enc.clone(), model).unwrap().run().unwrap();
    assert!(replay.agents[0].error.is_none(), "{:?}", replay.agents[0].error);
    assert_eq!(replay.agents[0].state, state);
    std::fs::remove_dir_all(dir).unwrap();
    let recorded = usages.lock().unwrap().clone();
    assert_eq!(recorded.len(), 3, "replay must not reissue HTTP requests");
    (state, recorded)
}

#[test]
fn native_app_turns_keep_screens_state_and_tool_ids_through_checkpoint_and_full_replay() {
    for kind in ["anthropic", "responses"] {
        let f = Fixture::new();
        let outputs:Vec<J>=(0..3).map(|i|if kind=="anthropic" {
            json!([{"type":"thinking","thinking":"signed thinking","signature":format!("sig-{i}")},
                {"type":"redacted_thinking","data":"opaque"},
                {"type":"tool_use","id":format!("native-{i}"),"name":"increment","input":{}}])
        } else {
            json!([{"type":"reasoning","id":format!("rs-{i}"),"encrypted_content":"opaque+/=","summary":[]},
                {"type":"function_call","id":format!("fc-{i}"),"call_id":format!("native-{i}"),"name":"increment","arguments":"{}"}])
        }).collect();
        let replies = outputs
            .iter()
            .map(|output| {
                (
                    200,
                    "",
                    if kind == "anthropic" {
                        message(output.clone(), "tool_use", json!({"input_tokens":1,"output_tokens":1}))
                    } else {
                        responses(output.clone())
                    },
                )
            })
            .collect();
        let (url, peer) = server(replies);
        let client = if kind == "anthropic" {
            f.client(&url)
        } else {
            Chat::responses(
                &url,
                "gpt-test",
                Some(Credentials::new(f.key.to_str().unwrap(), "KEY")),
                f.enc.clone(),
            )
        };
        let (state, _) = app_session(
            &f,
            client,
            "Always call increment, exactly once per response. Do not answer with prose.",
        );
        let seen = peer.join().unwrap();
        assert_eq!(seen.len(), 3);
        if kind == "anthropic" {
            assert_eq!(seen[0].body["messages"][0], seen[1].body["messages"][0]);
            let mut original = seen[1].body["messages"][1]["content"].clone();
            for part in original.as_array_mut().unwrap() {
                part.as_object_mut().unwrap().remove("cache_control");
            }
            assert_eq!(original, outputs[0]);
            assert_eq!(seen[1].body["messages"][2]["content"][0]["tool_use_id"], "native-0");
        } else {
            assert_eq!(seen[0].body["input"][1], seen[1].body["input"][1]);
            assert_eq!(
                seen[1].body["input"].as_array().unwrap()[2..4],
                *outputs[0].as_array().unwrap()
            );
            assert_eq!(seen[1].body["input"][4]["call_id"], "native-0");
        }
        let log = &state["__ui"]["@log"]["records"];
        assert_eq!(log[0]["content"], "Screen 0: use increment.\n[increment]\n");
        assert!(log[0].get("ids").is_none());
        assert!(log[1]["provider_state"].is_string());
        assert!(log[1]["ids"].is_null());
    }
}

/// "A rate-limited call comes back at once with the wait the provider asked for." The client
/// makes one attempt and does not wait or send again: when to try again is the retry policy's.
#[test]
fn a_rate_limited_call_returns_at_once_with_the_providers_wait() {
    let f = Fixture::new();
    let (url, peer) = server(vec![(
        429,
        "retry-after: 10000\r\n",
        json!({"error":{"message":"slow down"}}),
    )]);
    let client = Chat::responses(
        &url,
        "gpt-test",
        Some(Credentials::new(f.key.to_str().unwrap(), "KEY")),
        f.enc.clone(),
    );
    let started = std::time::Instant::now();
    let error = client.generate(&f.request(&session())).unwrap_err();
    assert_eq!(
        (error.kind, error.retry_after),
        (FailureKind::Busy, Some(Duration::from_secs(10000))),
        "{error}"
    );
    assert_eq!(peer.join().unwrap().len(), 1, "one request, not sent again");
    assert!(started.elapsed() < Duration::from_secs(5));
}

// Three real app turns, including checkpoint and full replay. Keys stay in files and are never
// printed. A model change is explicit.
#[test]
fn live_native_sessions_preserve_reasoning_and_read_prompt_cache_from_turn_two() {
    for kind in ["anthropic", "responses"] {
        let flag = if kind == "anthropic" {
            "AFFORDANCE_ANTHROPIC_SMOKE"
        } else {
            "AFFORDANCE_RESPONSES_SMOKE"
        };
        if std::env::var(flag).as_deref() != Ok("1") {
            continue;
        }
        let f = Fixture::new();
        let (file_var, name, model) = if kind == "anthropic" {
            ("AFFORDANCE_ANTHROPIC_KEY_FILE", "ANTHROPIC_API_KEY", "claude-haiku-5-5")
        } else {
            ("AFFORDANCE_RESPONSES_KEY_FILE", "OPENAI_API_KEY", "gpt-6.1-sol")
        };
        let key =
            std::env::var(file_var).unwrap_or_else(|_| panic!("{flag}=1 needs {file_var}: a file with a {name}= line"));
        let mut client = if kind == "anthropic" {
            Chat::anthropic(
                "https://api.anthropic.com",
                model,
                Some(Credentials::new(&key, name)),
                f.enc.clone(),
            )
        } else {
            Chat::responses(
                "https://api.openai.com/v1",
                model,
                Some(Credentials::new(&key, name)),
                f.enc.clone(),
            )
        };
        client.timeout = Duration::from_secs(120);
        client.provider_options = if kind == "anthropic" {
            json!({"thinking":{"type":"adaptive"},"output_config":{"effort":"high"}})
        } else {
            json!({"reasoning":{"effort":"low","context":"all_turns"},"text":{"verbosity":"low"},"prompt_cache_options":{"ttl":"30m"}})
        };
        let rules:String=(1..=160).map(|i|format!("Counter rule {i}: increment increases the counter by one; integers must be reported exactly, reference {i}.\n")).collect();
        let system = format!(
            "You operate a counter. Before each call, carefully verify that 104729 is prime using trial division and think through the computation. Always call increment exactly once per response. Never finish with prose.\n{rules}"
        );
        let (state, usage) = app_session(&f, client, &system);
        let records = state["__ui"]["@log"]["records"].as_array().unwrap();
        for (i, u) in usage.iter().enumerate() {
            eprintln!("{kind} {model} turn {}: {}", i + 1, u);
            if i > 0 {
                assert!(
                    u["prompt_tokens_details"]["cached_tokens"].as_u64().unwrap_or(0) > 0,
                    "{kind}: turn {} did not read cache",
                    i + 1
                );
            }
        }
        let mut reasoning_turns = 0;
        for record in records.iter().filter(|r| r["kind"] == "model") {
            let native: J = serde_json::from_str(record["provider_state"].as_str().unwrap()).unwrap();
            let has_state =
                if kind == "anthropic" {
                    native["content"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|b| b["type"] == "thinking" || b["type"] == "redacted_thinking")
                } else {
                    native["output"].as_array().unwrap().iter().any(|b| {
                        b["type"] == "reasoning" && b["encrypted_content"].as_str().is_some_and(|s| !s.is_empty())
                    })
                };
            if has_state {
                reasoning_turns += 1;
            }
        }
        assert!(
            reasoning_turns > 0,
            "{kind}: smoke did not exercise native reasoning state"
        );
    }
}

#[test]
fn lossy_tokenizers_cannot_corrupt_native_provider_state() {
    let f = Fixture::new();
    let (url, peer) = server(vec![(
        200,
        "",
        message(
            json!([{"type":"thinking","thinking":"Exact TEXT","signature":"SIGNATURE"}]),
            "end_turn",
            json!({}),
        ),
    )]);
    let mut encoder = tokenizer::encoder();
    encoder
        .tok
        .with_normalizer(Some(tokenizers::normalizers::utils::Lowercase))
        .unwrap();
    let mut client = f.client(&url);
    client.enc = Arc::new(encoder);
    let error = client.generate(&f.request(&session())).unwrap_err();
    assert!(!error.kind.may_pass() && error.message.contains("lossy"));
    peer.join().unwrap();
}

#[test]
fn summary_renders_disable_hosted_calls_without_changing_descriptions_or_active_requests() {
    use affordance::runtime::{AgentSpec, Runtime, RuntimeConfig};
    for kind in ["chat", "anthropic", "responses"] {
        for per_call in [false, true] {
            let f = Fixture::new();
            let forced = if kind == "anthropic" {
                json!({"type":"any"})
            } else {
                json!("required")
            };
            let (summary, active, violation) = match kind {
                "anthropic" => {
                    let call = json!([{"type":"tool_use","id":"active","name":"touch","input":{}}]);
                    (
                        message(json!([{"type":"text","text":"summary"}]), "end_turn", json!({})),
                        message(call.clone(), "tool_use", json!({})),
                        message(call, "max_tokens", json!({})),
                    )
                }
                "responses" => {
                    let call = json!([{"type":"function_call","id":"fc_active","call_id":"active","name":"touch","arguments":"{}"}]);
                    let mut partial = responses(json!([{"type":"function_call","arguments":"{"}]));
                    partial["status"] = json!("incomplete");
                    partial["incomplete_details"] = json!({"reason":"max_output_tokens"});
                    (
                        responses(
                            json!([{"type":"message","role":"assistant","content":[{"type":"output_text","text":"summary"}]}]),
                        ),
                        responses(call),
                        partial,
                    )
                }
                _ => {
                    let call = json!([{"id":"active","type":"function","function":{"name":"touch","arguments":"{}"}}]);
                    (
                        json!({"choices":[{"finish_reason":"stop","message":{"content":"summary"}}]}),
                        json!({"choices":[{"finish_reason":"tool_calls","message":{"content":null,"tool_calls":call}}]}),
                        json!({"choices":[{"finish_reason":"length","message":{"content":null,"tool_calls":[{"function":{"name":"touch","arguments":"{"}}]}}]}),
                    )
                }
            };
            let (url, peer) = server(vec![(200, "", summary), (200, "", active), (200, "", violation)]);
            let mut client = match kind {
                "anthropic" => f.client(&url),
                "responses" => Chat::responses(
                    &url,
                    "gpt-test",
                    Some(Credentials::new(f.key.to_str().unwrap(), "KEY")),
                    f.enc.clone(),
                ),
                _ => Chat::new(
                    &url,
                    "chat-test",
                    Some(Credentials::new(f.key.to_str().unwrap(), "KEY")),
                    f.enc.clone(),
                ),
            };
            if per_call {
                client.sampling = json!({"tool_choice":forced});
            } else {
                client.provider_options = json!({"tool_choice":forced});
            }
            client.window = affordance::runtime::Window::Unbounded;
            let dir = f.key.with_extension("passive");
            std::fs::create_dir_all(&dir).unwrap();
            let app = dir.join("app.tsx");
            let views = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("views");
            std::fs::write(
                &app,
                r#"
              import {ModelContext} from "$V/lib/model.tsx";
              import {Tool} from "$V/lib/tool.tsx";
              function Context({callable=true}) {
                return <ModelContext callable={callable} system="Describe this app.">
                  <Tool name="touch" description="Touch the app." schema={{type:'object',properties:{}}}
                    fn={()=>{throw new Error('summary must never execute a tool');}}>{'[touch]'}</Tool>
                </ModelContext>;
              }
              export default function Main({sampling}) {
                useEffect(()=>{(async()=>{
                  const summary=await model(<Context callable={false}/>,{sampling});
                  const active=await model(<Context/>,{sampling});
                  let violation='not rejected';
                  try{await model(<Context callable={false}/>,{sampling});}catch(e){violation=e.message;}
                  harness.stop({summary:summary.text,calls:summary.calls.length,active:active.calls.length,violation});
                })().catch(e=>harness.stop({error:e.message}));},[]);
                return <Context/>;
              }
            "#
                .replace("$V", views.to_str().unwrap()),
            )
            .unwrap();
            let mut cfg = RuntimeConfig::new(&app);
            cfg.dir = Some(dir.join("run"));
            cfg.strict_replay = true;
            cfg.library = Some(env!("CARGO_MANIFEST_DIR").into());
            cfg.workers = 1;
            cfg.model_threads = 1;
            cfg.exec_threads = 1;
            cfg.max_wall = Some(Duration::from_secs(10));
            let client: Arc<dyn Model> = Arc::new(client);
            let mut runtime = Runtime::new(cfg.clone(), f.enc.clone(), client.clone());
            runtime
                .add_agent(AgentSpec::new(
                    "a",
                    json!({"sampling":if per_call {json!({"tool_choice":forced})}else{json!({})}}),
                ))
                .unwrap();
            let first = runtime.run().unwrap();
            let report = &first.agents[0];
            assert!(report.error.is_none(), "{:?}", report.error);
            let stop = report.stopped.as_ref().unwrap();
            assert_eq!(stop["summary"], "summary", "{kind}: {stop}");
            assert_eq!(stop["calls"], 0);
            assert_eq!(stop["active"], 1);
            assert!(
                stop["violation"].as_str().unwrap().contains("non-callable render"),
                "{kind}: {stop}"
            );
            let recovered = Runtime::restore(cfg.clone(), f.enc.clone(), client.clone())
                .unwrap()
                .run()
                .unwrap();
            assert_eq!(recovered.agents[0].stopped, report.stopped);
            std::fs::remove_file(dir.join("run/a/checkpoint.json")).unwrap();
            let replay = Runtime::restore(cfg, f.enc.clone(), client).unwrap().run().unwrap();
            assert_eq!(replay.agents[0].stopped, report.stopped);
            assert!(replay.agents[0].error.is_none(), "{:?}", replay.agents[0].error);
            let seen = peer.join().unwrap();
            assert_eq!(seen.len(), 3);
            let none = if kind == "anthropic" {
                json!({"type":"none"})
            } else {
                json!("none")
            };
            assert_eq!(seen[0].body["tool_choice"], none);
            assert_eq!(seen[1].body["tool_choice"], forced);
            assert_eq!(seen[2].body["tool_choice"], none);
            assert_eq!(seen[0].body["tools"], seen[1].body["tools"]);
            assert_eq!(seen[1].body["tools"], seen[2].body["tools"]);
            assert_eq!(
                seen[0].body[if kind == "responses" { "input" } else { "messages" }],
                seen[1].body[if kind == "responses" { "input" } else { "messages" }]
            );
            std::fs::remove_dir_all(dir).unwrap();
        }
    }
}

#[test]
fn passive_toolless_requests_clear_forced_and_legacy_options_and_reject_invalid_flags() {
    let f = Fixture::new();
    for kind in ["chat", "anthropic", "responses"] {
        let reply = match kind {
            "anthropic" => ok(),
            "responses" => responses(json!([])),
            _ => json!({"choices":[{"finish_reason":"stop","message":{"content":"ok"}}]}),
        };
        let (url, peer) = server(vec![(200, "", reply)]);
        let mut client = match kind {
            "anthropic" => f.client(&url),
            "responses" => Chat::responses(
                &url,
                "gpt-test",
                Some(Credentials::new(f.key.to_str().unwrap(), "KEY")),
                f.enc.clone(),
            ),
            _ => Chat::new(
                &url,
                "chat-test",
                Some(Credentials::new(f.key.to_str().unwrap(), "KEY")),
                f.enc.clone(),
            ),
        };
        client.provider_options =
            json!({"tool_choice":if kind=="anthropic" {json!({"type":"any"})}else{json!("required")}});
        if kind == "chat" {
            client.provider_options["functions"] = json!([{"name":"legacy"}]);
            client.provider_options["function_call"] = json!({"name":"legacy"});
        }
        let mut doc = json!({"messages":[{"role":"user","content":"summarize"}],"tools":[],"callable":false});
        client.generate(&f.request(&doc)).unwrap();
        let body = peer.join().unwrap().remove(0).body;
        for key in ["tool_choice", "functions", "function_call"] {
            assert!(body.get(key).is_none(), "{kind}: {key}");
        }
        for bad in [json!(null), json!("false"), json!(0)] {
            doc["callable"] = bad;
            let error = client.generate(&f.request(&doc)).unwrap_err();
            assert!(
                !error.kind.may_pass() && error.message.contains("callable must be a boolean"),
                "{error}"
            );
        }
    }
}

/// A server that takes no key (a local llama.cpp, Ollama or vLLM) needs no key file: without
/// key_file and key_name the binding sends no credential header.
#[test]
fn a_keyless_server_needs_no_key_file() {
    let fixture = Fixture::new();
    let reply = json!({"choices": [{"message": {"content": "ok"}, "finish_reason": "stop"}], "usage": {"prompt_tokens": 3, "completion_tokens": 1}});
    let (url, peer) = server(vec![(200, "", reply)]);
    let client = from_config(
        &json!({"kind": "chat", "url": url, "model": "local", "context": 4096}),
        fixture.enc.clone(),
    )
    .unwrap();
    client.generate(&fixture.request(&session())).unwrap();
    let head = peer.join().unwrap().remove(0).head.to_ascii_lowercase();
    assert!(!head.contains("authorization") && !head.contains("x-api-key"), "{head}");
}

/// `key_env` takes a binding's key from the environment, read at call time like a key file's,
/// and an unset variable fails the call naming it, not the key.
#[test]
fn a_binding_takes_its_key_from_the_environment() {
    let fixture = Fixture::new();
    let reply = json!({"choices": [{"message": {"content": "ok"}, "finish_reason": "stop"}], "usage": {"prompt_tokens": 3, "completion_tokens": 1}});
    let (url, peer) = server(vec![(200, "", reply)]);
    let var = format!("AFF_TEST_KEY_{}", std::process::id());
    // SAFETY: the variable is unique to this test and process.
    unsafe { std::env::set_var(&var, "sk-from-env") };
    let config = json!({"kind": "chat", "url": url, "model": "local", "context": 4096, "key_env": var});
    let client = from_config(&config, fixture.enc.clone()).unwrap();
    client.generate(&fixture.request(&session())).unwrap();
    let head = peer.join().unwrap().remove(0).head;
    assert!(head.contains("Bearer sk-from-env"), "{head}");
    unsafe { std::env::remove_var(&var) };
    let error = client.generate(&fixture.request(&session())).unwrap_err().to_string();
    assert!(error.contains(&var) && !error.contains("sk-from-env"), "{error}");
}

/// A server answering one request with server-sent events, `pause` apart; it stops when the
/// client closes the connection. Returns the request it saw and how many events it wrote.
fn sse_server(events: Vec<String>, pause: Duration) -> (String, std::thread::JoinHandle<(Seen, usize)>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = std::thread::spawn(move || {
        let (mut stream, _) = accept(&listener);
        let seen = receive(&mut stream);
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        let mut written = 0;
        for event in events {
            std::thread::sleep(pause);
            if stream.write_all(event.as_bytes()).and_then(|_| stream.flush()).is_err() {
                break;
            }
            written += 1;
        }
        (seen, written)
    });
    (url, task)
}

fn chat_chunks(words: &[&str]) -> Vec<String> {
    let mut events: Vec<String> = words
        .iter()
        .map(|w| {
            format!(
                "data: {}\n\n",
                json!({"id": "c1", "model": "local", "choices": [{"index": 0, "delta": {"content": w}}]})
            )
        })
        .collect();
    events.push(format!(
        "data: {}\n\n",
        json!({"id": "c1", "model": "local", "choices": [{"index": 0, "delta": {}, "finish_reason": "stop"}]})
    ));
    events.push(format!(
        "data: {}\n\ndata: [DONE]\n\n",
        json!({"id": "c1", "model": "local", "choices": [], "usage": {"prompt_tokens": 3, "completion_tokens": 4}})
    ));
    events
}

/// "A hosted stream that goes silent fails after `stall_s`, not after the whole request's time."
/// The server sends one word and then nothing for two seconds. With `stall_s: 0.3` and a
/// 30 second request limit the call ends as interrupted well inside a second, with the word it
/// got; without `stall_s` the same stream is waited on and completes.
#[test]
fn a_silent_hosted_stream_ends_after_its_stall_limit() {
    let fixture = Fixture::new();
    let config = |url: &str, stall: Option<f64>| {
        let mut c = json!({"kind": "chat", "url": url, "model": "local", "context": 4096, "request_timeout_s": 30});
        if let Some(stall) = stall {
            c["stall_s"] = json!(stall);
        }
        c
    };
    let (url, _peer) = sse_server(chat_chunks(&["Hel", "lo"]), Duration::from_millis(1500));
    let client = from_config(&config(&url, Some(0.3)), fixture.enc.clone()).unwrap();
    let started = std::time::Instant::now();
    let error = client
        .generate_stream(&fixture.request(&session()), &mut |_| true)
        .unwrap_err();
    assert!(
        started.elapsed() < Duration::from_millis(1400),
        "{:?}",
        started.elapsed()
    );
    assert_eq!(error.kind, FailureKind::Interrupted, "{error}");
    assert!(error.message.contains("stalled: no data for 0.3s"), "{error}");

    let (url, peer) = sse_server(chat_chunks(&["Hel", "lo"]), Duration::from_millis(400));
    let client = from_config(&config(&url, None), fixture.enc.clone()).unwrap();
    let reply = client
        .generate_stream(&fixture.request(&session()), &mut |_| true)
        .unwrap();
    assert!(!reply.cut_off);
    peer.join().unwrap();
}

/// A streamed hosted reply hands out its text as it arrives and ends as the same reply ids a
/// whole one has; stopped, it ends with the text so far, cut off, and the connection closes.
#[test]
fn a_hosted_reply_streams_its_text_and_stops_when_told() {
    use affordance::runtime::Delta;
    let fixture = Fixture::new();
    let config = |url: &str| json!({"kind": "chat", "url": url, "model": "local", "context": 4096});
    let words = ["Hel", "lo", " there", "."];
    let whole = json!({"choices": [{"message": {"content": "Hello there."}, "finish_reason": "stop"}], "usage": {"prompt_tokens": 3, "completion_tokens": 4}});
    let (url, peer) = server(vec![(200, "", whole)]);
    let unstreamed = from_config(&config(&url), fixture.enc.clone())
        .unwrap()
        .generate(&fixture.request(&session()))
        .unwrap();
    assert!(peer.join().unwrap()[0].body.get("stream").is_none());

    let (url, peer) = sse_server(chat_chunks(&words), Duration::from_millis(20));
    let client = from_config(&config(&url), fixture.enc.clone()).unwrap();
    let mut seen = vec![];
    let streamed = client
        .generate_stream(&fixture.request(&session()), &mut |d| {
            let Delta::Text(t) = d else { panic!("{d:?}") };
            seen.push(t.to_string());
            true
        })
        .unwrap();
    assert_eq!(seen, words);
    assert_eq!(streamed.ids, unstreamed.ids);
    assert!(!streamed.cut_off);
    let (request, _) = peer.join().unwrap();
    assert_eq!(request.body["stream"], true);
    assert_eq!(request.body["stream_options"], json!({"include_usage": true}));

    let many: Vec<String> = (0..200).map(|i| format!("w{i} ")).collect();
    let many: Vec<&str> = many.iter().map(String::as_str).collect();
    let (url, peer) = sse_server(chat_chunks(&many), Duration::from_millis(5));
    let client = from_config(&config(&url), fixture.enc.clone()).unwrap();
    let mut n = 0;
    let stopped = client
        .generate_stream(&fixture.request(&session()), &mut |_| {
            n += 1;
            n < 3
        })
        .unwrap();
    assert!(stopped.cut_off);
    let text = fixture.enc.tok.decode(&stopped.ids, false).unwrap();
    assert!(text.contains(r#""content":"w0 w1 w2 ""#), "{text}");
    let (_, written) = peer.join().unwrap();
    assert!(
        written < 100,
        "the server stopped writing when the client left ({written} events)"
    );
}
