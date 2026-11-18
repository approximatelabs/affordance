use affordance::runtime::{
    cancel::CancelToken,
    model::{Completions, GenRequest, Model, Sglang},
};
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::TcpListener,
    time::Duration,
};

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

fn model_peer(response: String) -> (String, std::thread::JoinHandle<(String, Value, bool)>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let peer = std::thread::spawn(move || {
        let (mut stream, _) = accept(&listener);
        stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let mut request = Vec::new();
        let mut byte = [0];
        while !request.ends_with(b"\r\n\r\n") {
            stream.read_exact(&mut byte).unwrap();
            request.push(byte[0]);
        }
        let header = String::from_utf8(request).unwrap();
        let length: usize = header
            .lines()
            .find_map(|l| {
                l.to_ascii_lowercase()
                    .strip_prefix("content-length: ")
                    .map(str::to_string)
            })
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        let mut body = vec![0; length];
        stream.read_exact(&mut body).unwrap();
        stream.write_all(response.as_bytes()).unwrap();
        // Closed by the client, or kept alive for its next request.
        stream.set_read_timeout(Some(Duration::from_millis(300))).unwrap();
        let closed = matches!(stream.read(&mut byte), Ok(0));
        (header, serde_json::from_slice(&body).unwrap(), closed)
    });
    (url, peer)
}

fn request() -> GenRequest {
    GenRequest::new(
        "native-history",
        vec![7, 8, 9],
        4096,
        vec![99],
        Some(3),
        CancelToken::new(),
    )
}

#[test]
fn native_request_preserves_ids_bounds_rank_and_raw_cache_tiers() {
    let meta = json!({"cached_tokens":2,"cached_tokens_details":{"device":0,"host":1,"storage":1,"storage_backend":"PackedDPFile"},
        "completion_tokens":2,"finish_reason":{"type":"stop","matched":99},"future_extension":{"kept":true}});
    let reply = json!({"output_ids":[41,42],"meta_info":meta}).to_string();
    let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}", reply.len(), reply);
    let (url, peer) = model_peer(response);
    let mut model = Sglang::new(&url, 8);
    model.stream = false;
    model.sampling = json!({"temperature":0.6,"top_p":0.95});
    // A render's own sampling goes over the binding's (a summary at 0.2).
    let result = model
        .generate(&GenRequest {
            sampling: json!({"temperature":0.2}),
            tag: Some("summary".into()),
            ..request()
        })
        .unwrap();
    let (headers, body, closed) = peer.join().unwrap();
    assert!(headers.starts_with("POST /generate "));
    assert!(!closed, "a whole response keeps the connection");
    assert_eq!(body["input_ids"], json!([7, 8, 9]));
    assert_eq!(body["routed_dp_rank"], 3);
    assert!(body.get("data_parallel_rank").is_none());
    assert_eq!(body["sampling_params"]["max_new_tokens"], 4096);
    assert_eq!(body["sampling_params"]["stop_token_ids"], json!([99]));
    assert_eq!(body["sampling_params"]["skip_special_tokens"], false);
    assert_eq!(body["sampling_params"]["no_stop_trim"], true);
    assert_eq!(
        (
            body["sampling_params"]["temperature"].as_f64(),
            body["sampling_params"]["top_p"].as_f64()
        ),
        (Some(0.2), Some(0.95))
    );
    assert!(body.get("tag").is_none(), "the tag is the harness's, not the server's");
    assert_eq!(result.ids, vec![41, 42, 99]);
    assert_eq!(result.cached_tokens, Some(2));
    assert_eq!(result.model_meta, Some(meta));
    assert_eq!(result.attempts, 1);
    assert_eq!(result.request_id.as_deref(), body["rid"].as_str());
}

#[test]
fn streaming_keeps_reasoning_ids_and_final_metadata_without_duplicate_tokens() {
    // Native IDs contain the reasoning stream too; never strip or re-tokenize them.
    let first = json!({"output_ids":[41],"meta_info":{"completion_tokens":1,"finish_reason":null}});
    let meta = json!({"completion_tokens":2,"cached_tokens":2,"cached_tokens_details":{"device":0,"host":0,"storage":2},"finish_reason":{"type":"length"}});
    let second = json!({"output_ids":[41,42],"meta_info":meta});
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\r\ndata: {first}\n\ndata: {second}\n\ndata: [DONE]\n\n"
    );
    let (url, peer) = model_peer(response);
    let model = Sglang::new(&url, 8);
    let mut observed = vec![];
    let result = model
        .generate_stream(&request(), &mut |ids| {
            let affordance::runtime::Delta::Ids(ids) = ids else {
                panic!("{ids:?}")
            };
            observed.extend_from_slice(ids);
            true
        })
        .unwrap();
    let (_, body, closed) = peer.join().unwrap();
    assert!(closed);
    assert_eq!(body["stream"], true);
    assert_eq!(body["routed_dp_rank"], 3);
    assert_eq!(result.ids, vec![41, 42]);
    assert_eq!(observed, result.ids);
    assert!(result.cut_off);
    assert!(result.first_token_ms.is_some());
    assert_eq!(result.model_meta, Some(meta));
    assert_eq!(result.attempts, 1);
    assert_eq!(result.request_id.as_deref(), body["rid"].as_str());
}

/// A vLLM data-parallel server routes `/v1/completions` by the `X-data-parallel-rank` header: an
/// agent's pinned rank goes there (its prefix stays in one engine's cache); one rank, no header.
#[test]
fn completions_send_the_pinned_rank_to_a_data_parallel_server() {
    let reply =
        json!({"choices":[{"token_ids":[41,42],"finish_reason":"stop","stop_reason":99}],"usage":{"prompt_tokens":3}})
            .to_string();
    let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}", reply.len(), reply);
    let (url, peer) = model_peer(response.clone());
    let mut model = Completions::new(&url, "m");
    model.stream = false;
    model.ranks = 8;
    assert_eq!(model.ranks(), 8);
    let result = model.generate(&request()).unwrap();
    let (headers, body, _) = peer.join().unwrap();
    assert!(headers.starts_with("POST /v1/completions "));
    assert!(
        headers
            .lines()
            .any(|l| l.eq_ignore_ascii_case("X-data-parallel-rank: 3")),
        "{headers}"
    );
    assert!(body.get("data_parallel_rank").is_none(), "the rank travels as a header");
    assert_eq!(result.ids, vec![41, 42, 99]);
    let (url, peer) = model_peer(response);
    let mut one = Completions::new(&url, "m");
    one.stream = false;
    one.generate(&request()).unwrap();
    let (headers, _, _) = peer.join().unwrap();
    assert!(
        !headers.to_ascii_lowercase().contains("x-data-parallel-rank"),
        "{headers}"
    );
}

#[test]
fn malformed_native_ids_fail_in_both_protocols_and_stream_modes() {
    for native in [true, false] {
        for streaming in [true, false] {
            for invalid in [json!("bad"), json!(-1), json!(1.5), json!(4294967296u64)] {
                for stop in [false, true] {
                    let ids = if stop { json!([41]) } else { json!([41, invalid]) };
                    let matched = if stop { invalid.clone() } else { Value::Null };
                    // Text stop reasons are valid, so use a non-string invalid stop ID here.
                    if stop && invalid.is_string() {
                        continue;
                    }
                    let body = if native {
                        json!({"output_ids":ids,"meta_info":{"finish_reason":{"type":"stop","matched":matched}}})
                    } else {
                        json!({"choices":[{"token_ids":ids,"finish_reason":"stop","stop_reason":matched}]})
                    }
                    .to_string();
                    let response = if streaming {
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\r\ndata: {body}\n\ndata: [DONE]\n\n"
                        )
                    } else {
                        format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}", body.len())
                    };
                    let (url, peer) = model_peer(response);
                    let model: Box<dyn Model> = if native {
                        let mut m = Sglang::new(&url, 1);
                        m.stream = streaming;
                        Box::new(m)
                    } else {
                        let mut m = Completions::new(&url, "m");
                        m.stream = streaming;
                        Box::new(m)
                    };
                    let result = model.generate(&request());
                    peer.join().unwrap();
                    let error = result.expect_err(&format!(
                        "native={native} streaming={streaming} stop={stop} invalid={invalid}"
                    ));
                    assert!(!error.kind.may_pass(), "protocol errors must not retry: {error:?}");
                    assert!(error.message.contains("ProtocolError"), "{error:?}");
                }
            }
        }
    }
}

/// A reply over the transport's size limit is a protocol error of the binding: sent once, not
/// retried, and not transient (resending would get the same reply).
#[test]
fn a_reply_over_the_size_limit_is_not_retried() {
    let (url, peer) = model_peer("HTTP/1.1 200 OK\r\nContent-Length: 4000000000\r\n\r\n".into());
    let mut model = Sglang::new(&url, 1);
    model.stream = false;
    let error = model.generate(&request()).unwrap_err();
    peer.join().unwrap();
    assert!(
        !error.kind.may_pass()
            && error
                .message
                .starts_with("sglang /generate: ProtocolError: response body over"),
        "{error:?}"
    );
}
