use affordance::runtime::{
    cancel::CancelToken,
    http::{Failure, HttpError, post_json, post_sse},
};
use serde_json::json;
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

fn server(response: &'static [u8]) -> (String, std::thread::JoinHandle<bool>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/generate", listener.local_addr().unwrap());
    let thread = std::thread::spawn(move || {
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
        stream.read_exact(&mut vec![0; length]).unwrap();
        stream.write_all(response).unwrap();
        closed(&mut stream)
    });
    (url, thread)
}
/// Read one request (head and body) from `stream`.
fn read_request(stream: &mut std::net::TcpStream) -> bool {
    let mut request = Vec::new();
    let mut byte = [0];
    while !request.ends_with(b"\r\n\r\n") {
        if stream.read(&mut byte).map_or(true, |n| n == 0) {
            return false;
        }
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
    stream.read_exact(&mut vec![0; length]).unwrap();
    true
}
#[test]
fn a_finished_request_keeps_its_connection_for_the_next() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/generate", listener.local_addr().unwrap());
    let peer = std::thread::spawn(move || {
        let (mut stream, _) = accept(&listener);
        stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let mut served = 0;
        while read_request(&mut stream) {
            served += 1;
            stream
                .write_all(if served == 2 {
                    b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n2\r\n{}\r\n0\r\n\r\n" as &[u8]
                } else {
                    b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}"
                })
                .unwrap();
            if served == 3 {
                break;
            }
        }
        listener.set_nonblocking(true).unwrap();
        (served, listener.accept().is_err())
    });
    let cancel = CancelToken::new();
    for _ in 0..3 {
        assert_eq!(
            post_json(&url, b"{}", &[], Duration::from_secs(1), &cancel).unwrap(),
            (200, json!({}))
        );
        // The pool takes the connection back a moment after the body's end (from a background
        // task, so on a loaded machine 20 ms was sometimes too short).
        std::thread::sleep(Duration::from_millis(200));
    }
    assert_eq!(peer.join().unwrap(), (3, true), "three requests on one connection");
    assert!(!cancel.is_cancelled());
}
#[test]
fn a_pooled_connection_the_server_closed_is_retried_on_a_new_one() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/generate", listener.local_addr().unwrap());
    let peer = std::thread::spawn(move || {
        for _ in 0..2 {
            let (mut stream, _) = accept(&listener);
            assert!(read_request(&mut stream));
            // Kept alive by the client, closed by the server after its answer.
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}")
                .unwrap();
        }
    });
    let cancel = CancelToken::new();
    assert_eq!(
        post_json(&url, b"{}", &[], Duration::from_secs(1), &cancel).unwrap().0,
        200
    );
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(
        post_json(&url, b"{}", &[], Duration::from_secs(1), &cancel).unwrap().0,
        200
    );
    peer.join().unwrap();
}
#[test]
fn a_finished_event_stream_keeps_its_connection() {
    // The first event is 19 (0x13) bytes; transfer framing must match its payload.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/generate", listener.local_addr().unwrap());
    let peer = std::thread::spawn(move || {
        let (mut stream, _) = accept(&listener);
        stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let mut served = 0;
        while served < 2 && read_request(&mut stream) {
            served += 1;
            stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n13\r\ndata: {\"token\":1}\n\n\r\ne\r\ndata: [DONE]\n\n\r\n0\r\n\r\n").unwrap();
        }
        listener.set_nonblocking(true).unwrap();
        (served, listener.accept().is_err())
    });
    let cancel = CancelToken::new();
    for _ in 0..2 {
        let mut events = vec![];
        post_sse(&url, b"{}", &[], Duration::from_secs(1), &cancel, |e| {
            events.push(e);
            Ok(true)
        })
        .unwrap();
        assert_eq!(events, vec![json!({"token":1})]);
    }
    // Both streams were served on the first connection, and the client opened no other: a stream
    // is read to its body's end, which is what returns its connection to the pool.
    assert_eq!(peer.join().unwrap(), (2, true), "two streams on one connection");
}
/// A complete response whose JSON is bad is a protocol error. Its connection was framed correctly,
/// so it may stay pooled; nothing of it is held by the request's token.
#[test]
fn invalid_json_is_a_protocol_error_and_the_token_lives() {
    let (url, peer) = server(b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\n\r\n{");
    let cancel = CancelToken::new();
    let e = post_json(&url, b"{}", &[], Duration::from_secs(1), &cancel).unwrap_err();
    assert!(e.message.contains("not JSON") && e.failure == Failure::Protocol, "{e}");
    peer.join().unwrap();
    assert!(!cancel.is_cancelled());
}
#[test]
fn invalid_sse_closes_socket_while_shared_token_lives() {
    let (url, peer) = server(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\r\ndata: {invalid}\n\n");
    let cancel = CancelToken::new();
    assert!(
        post_sse(&url, b"{}", &[], Duration::from_secs(1), &cancel, |_| Ok(true))
            .unwrap_err()
            .message
            .contains("bad event")
    );
    assert!(peer.join().unwrap());
    assert!(!cancel.is_cancelled());
}
#[test]
fn callback_failure_closes_socket_while_shared_token_lives() {
    let (url, peer) = server(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\r\ndata: {}\n\n");
    let cancel = CancelToken::new();
    assert_eq!(
        post_sse(&url, b"{}", &[], Duration::from_secs(1), &cancel, |_| Err(
            HttpError::protocol("consumer failure")
        ))
        .unwrap_err(),
        HttpError::protocol("consumer failure")
    );
    assert!(peer.join().unwrap());
    assert!(!cancel.is_cancelled());
}
#[test]
fn cancellation_interrupts_read_and_closes_socket() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/generate", listener.local_addr().unwrap());
    let (ready, started) = std::sync::mpsc::channel();
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
        stream.read_exact(&mut vec![0; length]).unwrap();
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\n{")
            .unwrap();
        ready.send(()).unwrap();
        closed(&mut stream)
    });
    let cancel = CancelToken::new();
    let worker_token = cancel.clone();
    let client = std::thread::spawn(move || post_json(&url, b"{}", &[], Duration::from_secs(10), &worker_token));
    // The request reached the peer, so the socket cleanup registration exists.
    started.recv_timeout(Duration::from_secs(2)).unwrap();
    cancel.cancel("test cancellation");
    assert!(client.join().unwrap().is_err());
    assert!(peer.join().unwrap());
}
#[test]
fn timeout_closes_socket_while_shared_token_lives() {
    let cancel = CancelToken::new();
    for _ in 0..3 {
        let (url, peer) = server(b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\n{");
        // Long enough to connect and send on a busy machine; the response's body never ends.
        // The request was sent and then timed out waiting for the body. (`Unsent` here would mean
        // the client never reached the server at all.)
        let error = post_json(&url, b"{}", &[], Duration::from_millis(500), &cancel).unwrap_err();
        assert_eq!(error.failure, Failure::Transport, "{}", error.message);
        assert!(
            peer.join().unwrap(),
            "timed-out request must deliver EOF before next attempt"
        );
        assert!(!cancel.is_cancelled());
    }
}
#[test]
fn stopping_sse_callback_closes_socket_without_cancelling_token() {
    let (url, peer) = server(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\r\ndata: {\"token\":1}\n\n");
    let cancel = CancelToken::new();
    post_sse(&url, b"{}", &[], Duration::from_secs(1), &cancel, |_| Ok(false)).unwrap();
    assert!(peer.join().unwrap());
    assert!(!cancel.is_cancelled());
}

/// A failure says what kind it is: the status a server answered with (whatever its body says),
/// an answer that breaks the protocol, or a request never sent.
#[test]
fn a_failure_carries_its_kind_not_just_its_message() {
    let (url, peer) = server(b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 13\r\n\r\nnot HTTP 400!");
    let e = post_sse(&url, b"{}", &[], Duration::from_secs(1), &CancelToken::new(), |_| {
        Ok(true)
    })
    .unwrap_err();
    assert_eq!(
        (e.failure, e.status(), e.transient()),
        (Failure::Status(503), Some(503), true)
    );
    assert_eq!(e.message, "HTTP 503: not HTTP 400!");
    peer.join().unwrap();
    let (url, peer) = server(b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\n\r\n{");
    let e = post_json(&url, b"{}", &[], Duration::from_secs(1), &CancelToken::new()).unwrap_err();
    assert_eq!((e.failure, e.transient()), (Failure::Protocol, false));
    peer.join().unwrap();
    let closed = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let e = post_json(
        &format!("http://127.0.0.1:{closed}/x"),
        b"{}",
        &[],
        Duration::from_secs(1),
        &CancelToken::new(),
    )
    .unwrap_err();
    assert_eq!((e.failure, e.transient()), (Failure::Unsent, true));
}

#[test]
fn chunk_boundaries_and_trailers_do_not_change_sse_events() {
    let text = "data: {\n".to_owned() + "data: \"message\": \"snowman ☃\"}\n\ndata: [DONE]\n\n";
    let mut wire = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n".to_vec();
    // Splits inside JSON, SSE field names, CRLF, and a multibyte UTF-8 character.
    for byte in text.as_bytes() {
        wire.extend_from_slice(b"1;extension=yes\r\n");
        wire.push(*byte);
        wire.extend_from_slice(b"\r\n");
    }
    wire.extend_from_slice(b"0\r\nX-Trace: complete\r\n\r\n");
    let wire = Box::leak(wire.into_boxed_slice());
    let (url, peer) = server(wire);
    let mut events = vec![];
    post_sse(&url, b"{}", &[], Duration::from_secs(2), &CancelToken::new(), |event| {
        events.push(event);
        Ok(true)
    })
    .unwrap();
    assert_eq!(events, vec![json!({"message":"snowman ☃"})]);
    assert!(peer.join().unwrap());
}

#[test]
fn a_truncated_chunk_is_a_transport_error() {
    let (url, peer) =
        server(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n20\r\ndata: {}\n\n");
    // The peer waits for us to close; the stalled incomplete chunk must fail, not succeed.
    let result = post_sse(
        &url,
        b"{}",
        &[],
        Duration::from_millis(500),
        &CancelToken::new(),
        |_| Ok(true),
    );
    assert!(result.is_err());
    assert!(peer.join().unwrap());
}

/// A server that answers a request with `head`, then `filler` bytes of `fill` in 1 MiB writes,
/// then holds the connection open. It returns the bytes it wrote once the client closed it.
fn flood(head: &[u8], fill: u8, filler: usize) -> (String, std::thread::JoinHandle<usize>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/generate", listener.local_addr().unwrap());
    let head = head.to_vec();
    let thread = std::thread::spawn(move || {
        let (mut stream, _) = accept(&listener);
        assert!(read_request(&mut stream));
        stream.write_all(&head).unwrap();
        let (block, mut written) = (vec![fill; 1 << 20], 0);
        while written < filler {
            match stream.write(&block[..block.len().min(filler - written)]) {
                Ok(n) => written += n,
                Err(_) => return written,
            }
        }
        // On macOS setting a timeout on a socket the client already closed is an error
        // (EINVAL); the read below then returns at once, which is all this waits for.
        let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
        let _ = stream.read(&mut [0]);
        written
    });
    (url, thread)
}

/// What a response may hold is bounded: its body, a header line, its header fields. Over a
/// limit, the request fails at once with a protocol error (not retried) after reading at most
/// the limit, instead of allocating what the server sends. A failure's body is read only as far
/// as its error message needs.
#[test]
fn a_response_over_a_limit_fails_without_reading_it() {
    use affordance::runtime::http::BODY_MAX;
    let request = |url: &str| post_json(url, b"{}", &[], Duration::from_secs(30), &CancelToken::new());
    // A declared body over the limit fails before any of it is read.
    let (url, peer) = flood(b"HTTP/1.1 200 OK\r\nContent-Length: 4000000000\r\n\r\n", b' ', 0);
    let started = std::time::Instant::now();
    let e = request(&url).unwrap_err();
    assert!(
        e.message.starts_with("ProtocolError: response body over") && started.elapsed() < Duration::from_secs(5),
        "{e}"
    );
    peer.join().unwrap();
    // So does an undeclared one (read to the close), after at most the limit.
    let (url, peer) = flood(
        b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n",
        b' ',
        BODY_MAX + (16 << 20),
    );
    let e = request(&url).unwrap_err();
    assert_eq!(e.failure, Failure::Protocol);
    assert!(e.message.starts_with("ProtocolError: response body over"), "{e}");
    assert!(
        peer.join().unwrap() < BODY_MAX + (16 << 20),
        "the client stopped reading"
    );
    // An endless header line, and endless header fields.
    let (url, peer) = flood(b"HTTP/1.1 200 OK\r\nX-Pad: ", b'a', 1 << 20);
    let e = request(&url).unwrap_err();
    assert_eq!(e.failure, Failure::Protocol);
    assert!(e.message.starts_with("ProtocolError: response head too large"), "{e}");
    peer.join().unwrap();
    let (url, peer) = flood(
        format!("HTTP/1.1 200 OK\r\n{}", "X-Field: 1\r\n".repeat(1000)).as_bytes(),
        b' ',
        0,
    );
    let e = request(&url).unwrap_err();
    assert_eq!(e.failure, Failure::Protocol);
    assert!(e.message.starts_with("ProtocolError: response head too large"), "{e}");
    peer.join().unwrap();
    // A failure's endless body comes back cut short, as the failure it is.
    let (url, peer) = flood(
        b"HTTP/1.1 503 Service Unavailable\r\nConnection: close\r\n\r\n",
        b'x',
        1 << 20,
    );
    let (status, body) = request(&url).unwrap();
    assert_eq!(status, 503);
    assert!(
        body.as_str()
            .is_some_and(|b| !b.is_empty() && b.bytes().all(|c| c == b'x')),
        "{body}"
    );
    peer.join().unwrap();
}

/// A non-streaming request's timeout bounds the whole exchange, not each read: a server that
/// drips its response a byte at a time fails the request at its deadline.
#[test]
fn a_dripping_response_fails_at_the_request_deadline() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/generate", listener.local_addr().unwrap());
    let peer = std::thread::spawn(move || {
        let (mut stream, _) = accept(&listener);
        assert!(read_request(&mut stream));
        // 100 bytes of valid JSON, a byte every 100 ms (10 s in all).
        let body = format!("{}{{}}", " ".repeat(98));
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\n")
            .unwrap();
        for byte in body.bytes() {
            std::thread::sleep(Duration::from_millis(100));
            if stream.write_all(&[byte]).is_err() {
                return;
            }
        }
    });
    let started = std::time::Instant::now();
    let e = post_json(&url, b"{}", &[], Duration::from_secs(1), &CancelToken::new()).unwrap_err();
    assert_eq!(e, HttpError::transport("timed out: no complete response in 1s"));
    assert!(
        started.elapsed() < Duration::from_millis(1500),
        "{:?}",
        started.elapsed()
    );
    peer.join().unwrap();
}

/// A pooled connection idle too long to be reused is closed when any request pools one, not
/// kept until its own host is contacted again.
#[test]
fn a_stale_pooled_connection_is_closed_when_another_is_pooled() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/generate", listener.local_addr().unwrap());
    let stale = std::thread::spawn(move || {
        let (mut stream, _) = accept(&listener);
        assert!(read_request(&mut stream));
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}")
            .unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
        closed(&mut stream)
    });
    let cancel = CancelToken::new();
    assert_eq!(
        post_json(&url, b"{}", &[], Duration::from_secs(1), &cancel).unwrap().0,
        200
    );
    // Past the pool's idle limit (4 s), another host's connection is pooled.
    std::thread::sleep(Duration::from_millis(4500));
    let (other, peer) = server(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}");
    assert_eq!(
        post_json(&other, b"{}", &[], Duration::from_secs(1), &cancel)
            .unwrap()
            .0,
        200
    );
    assert!(stale.join().unwrap(), "the stale connection was closed");
    peer.join().unwrap();
}

/// The client closed the connection: the peer reads its end, or (on macOS, where an aborted
/// connection is reset) a reset. A timeout would mean it stayed open.
fn closed(stream: &mut std::net::TcpStream) -> bool {
    match stream.read(&mut [0]) {
        Ok(0) => true,
        Err(e) => matches!(
            e.kind(),
            std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted
        ),
        Ok(_) => false,
    }
}
