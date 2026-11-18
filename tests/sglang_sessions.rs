//! SGLang sessions (`model::Sessions`) against a mock server with SGLang's session semantics
//! (session_controller.py, the copy path): a request continues request `rid` of its session with
//! `sequence(rid)[:offset] + input_ids`, and a session keeps, as a request's output, tokens past
//! the matched stop (as the server's overlap decoding does). The server's sequence must be the
//! agent's prompt on every turn: through continuations, the turn limit, a non-append (a fold), a
//! server restart (sessions lost) and both wire modes.
use affordance::runtime::{
    cancel::CancelToken,
    model::{GenRequest, Model, Sessions, Sglang},
};
use serde_json::{Value as J, json};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Default)]
struct Server {
    /// session -> rid -> the sequence that request's node holds (its prompt and its output).
    sessions: HashMap<String, HashMap<String, Vec<u32>>>,
    /// Every /generate: (the full sequence it ran on, input ids sent, used a session).
    log: Vec<(Vec<u32>, usize, bool)>,
    opened: usize,
    closed: usize,
}

const STOP: u32 = 99;

fn handle(server: &Mutex<Server>, path: &str, body: &J) -> (u16, J) {
    let mut s = server.lock().unwrap();
    match path {
        "/open_session" => {
            s.opened += 1;
            s.sessions
                .insert(body["session_id"].as_str().unwrap().into(), HashMap::new());
            (200, json!(body["session_id"]))
        }
        "/close_session" => {
            s.closed += 1;
            s.sessions.remove(body["session_id"].as_str().unwrap());
            (200, J::Null)
        }
        _ => {
            let input: Vec<u32> = body["input_ids"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as u32)
                .collect();
            let rid = body["rid"].as_str().unwrap().to_string();
            let sp = &body["session_params"];
            let seq = if sp.is_null() {
                input.clone()
            } else {
                let Some(session) = s.sessions.get(sp["id"].as_str().unwrap()) else {
                    return (400, json!({"error": {"message": "session does not exist"}}));
                };
                match sp["rid"].as_str() {
                    None => input.clone(),
                    Some(parent) => {
                        let Some(prev) = session.get(parent) else {
                            return (400, json!({"error": {"message": "Invalid request session id"}}));
                        };
                        let cut = sp["offset"].as_u64().map_or(prev.len(), |o| o as usize);
                        [&prev[..cut], &input[..]].concat()
                    }
                }
            };
            // The reply: two tokens and the stop; the session keeps two more past it.
            let n = seq.len() as u32;
            let out = vec![1000 + n % 7, 2000 + n % 11, STOP];
            if !sp.is_null() {
                let id = sp["id"].as_str().unwrap().to_string();
                let kept = [&seq[..], &out[..], &[7, 7][..]].concat();
                s.sessions.get_mut(&id).unwrap().insert(rid, kept);
            }
            s.log.push((seq.clone(), input.len(), !sp.is_null()));
            (
                200,
                json!({"output_ids": out, "meta_info": {"prompt_tokens": seq.len(), "finish_reason": {"type": "stop", "matched": STOP}}}),
            )
        }
    }
}

fn mock() -> (String, Arc<Mutex<Server>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = Arc::new(Mutex::new(Server::default()));
    let shared = server.clone();
    std::thread::spawn(move || {
        for conn in listener.incoming().flatten() {
            let server = shared.clone();
            std::thread::spawn(move || {
                let mut r = BufReader::new(conn.try_clone().unwrap());
                let mut conn = conn;
                loop {
                    let mut line = String::new();
                    if r.read_line(&mut line).unwrap_or(0) == 0 {
                        return;
                    }
                    let path = line.split_whitespace().nth(1).unwrap_or("").to_string();
                    let mut len = 0;
                    loop {
                        let mut h = String::new();
                        r.read_line(&mut h).unwrap();
                        if h.trim().is_empty() {
                            break;
                        }
                        if let Some(v) = h.to_ascii_lowercase().strip_prefix("content-length:") {
                            len = v.trim().parse().unwrap();
                        }
                    }
                    let mut body = vec![0; len];
                    r.read_exact(&mut body).unwrap();
                    let body: J = serde_json::from_slice(&body).unwrap();
                    let (status, answer) = handle(&server, &path, &body);
                    if body["stream"] == true && status == 200 {
                        let events = format!("data: {answer}\n\ndata: [DONE]\n\n");
                        let _ = conn.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n{events}").as_bytes());
                        return;
                    }
                    let text = answer.to_string();
                    let _ = conn.write_all(
                        format!(
                            "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{text}",
                            text.len()
                        )
                        .as_bytes(),
                    );
                }
            });
        }
    });
    (url, server)
}

/// An agent's turns: each prompt is the last prompt, its reply and a new screen; turn `fold_at`
/// rewrites the prompt (a fold); the server restarts (forgets its sessions) before `restart_at`.
fn run(stream: bool) {
    let (url, server) = mock();
    let mut model = Sglang::new(&url, 1);
    model.stream = stream;
    model.sessions = Some(Sessions::new(4, Duration::from_secs(600)));
    let (fold_at, restart_at) = (7, 11);
    let mut prompt: Vec<u32> = (0..50).collect();
    for turn in 0..16u32 {
        if turn == restart_at {
            server.lock().unwrap().sessions.clear();
        }
        let reply = model
            .generate(&GenRequest::new(
                "agent-1",
                prompt.clone(),
                64,
                vec![STOP],
                None,
                CancelToken::new(),
            ))
            .unwrap();
        assert_eq!(reply.ids.last(), Some(&STOP));
        // A summary render (tagged) shares the prefix but never uses the session.
        if turn == 3 {
            let summary = GenRequest {
                tag: Some("summary".into()),
                ..GenRequest::new(
                    "agent-1",
                    [&prompt[..], &[555][..]].concat(),
                    64,
                    vec![STOP],
                    None,
                    CancelToken::new(),
                )
            };
            model.generate(&summary).unwrap();
        }
        let screen = [300 + turn, 301, 302];
        prompt = if turn + 1 == fold_at {
            [&prompt[..20], &screen[..]].concat()
        } else {
            [&prompt[..], &reply.ids[..], &screen[..]].concat()
        };
    }
    let s = server.lock().unwrap();
    let turns: Vec<_> = s.log.iter().filter(|(seq, _, _)| !seq.ends_with(&[555])).collect();
    // The server ran on exactly the agent's prompt every time (the mock computes it from the session).
    let mut want: Vec<u32> = (0..50).collect();
    let mut resent = 0;
    for (k, (seq, sent, session)) in turns.iter().enumerate() {
        assert_eq!(seq, &want, "turn {k}");
        assert!(*session || k == restart_at as usize, "turn {k} used a session");
        if *sent == seq.len() {
            resent += 1
        } else {
            assert!(*sent < 10, "turn {k} sent {sent} ids")
        }
        let n = seq.len() as u32;
        let reply = [1000 + n % 7, 2000 + n % 11, STOP];
        let screen = [300 + k as u32, 301, 302];
        want = if k + 1 == fold_at as usize {
            [&want[..20], &screen[..]].concat()
        } else {
            [&want[..], &reply[..], &screen[..]].concat()
        };
    }
    // Full prompts: the first turn, every 4th turn after a new session, the fold, and after the restart.
    assert_eq!(
        turns.len(),
        16,
        "one request a turn (the refused one after the restart is not run)"
    );
    assert!((5..=7).contains(&resent), "full prompts sent: {resent}");
    assert!(
        s.log.iter().any(|(seq, _, session)| seq.ends_with(&[555]) && !session),
        "the summary went without a session"
    );
    assert!(
        s.closed >= 3,
        "sessions closed at the turn limit and the fold: {}",
        s.closed
    );
}

#[test]
fn a_turn_sends_only_new_ids_and_the_server_runs_on_exactly_the_prompt() {
    run(false);
}

#[test]
fn sessions_over_server_sent_events() {
    run(true);
}

#[test]
fn a_server_prompt_of_another_length_turns_sessions_off() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for conn in listener.incoming().flatten() {
            std::thread::spawn(move || {
                let mut r = BufReader::new(conn.try_clone().unwrap());
                let mut conn = conn;
                loop {
                    let mut line = String::new();
                    if r.read_line(&mut line).unwrap_or(0) == 0 {
                        return;
                    }
                    let mut len = 0;
                    loop {
                        let mut h = String::new();
                        r.read_line(&mut h).unwrap();
                        if h.trim().is_empty() {
                            break;
                        }
                        if let Some(v) = h.to_ascii_lowercase().strip_prefix("content-length:") {
                            len = v.trim().parse().unwrap();
                        }
                    }
                    let mut body = vec![0; len];
                    r.read_exact(&mut body).unwrap();
                    let body: J = serde_json::from_slice(&body).unwrap();
                    if body.get("input_ids").is_none() {
                        let _ = conn.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nnull");
                        continue;
                    }
                    // A server whose session sequence is one token longer than the prompt.
                    let n =
                        body["input_ids"].as_array().unwrap().len() + usize::from(!body["session_params"].is_null());
                    let text = json!({"output_ids": [5, STOP], "meta_info": {"prompt_tokens": n, "finish_reason": {"type": "stop", "matched": STOP}}}).to_string();
                    let _ = conn.write_all(
                        format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{text}", text.len()).as_bytes(),
                    );
                }
            });
        }
    });
    let mut model = Sglang::new(&url, 1);
    model.stream = false;
    model.sessions = Some(Sessions::new(4, Duration::from_secs(600)));
    // The session's answer is refused, the next attempt goes without one, and sessions stay off.
    let request = || GenRequest::new("a", vec![1, 2, 3], 8, vec![STOP], None, CancelToken::new());
    assert!(model.generate(&request()).unwrap_err().kind.may_pass());
    let reply = model.generate(&request()).unwrap();
    assert_eq!(reply.ids, vec![5, STOP]);
    assert!(model.sessions.as_ref().unwrap().is_off());
}

/// Live, on demand: one agent, 20 turns, against the SGLang server at `AFFORDANCE_SGLANG_URL` (e.g.
/// `AFFORDANCE_SGLANG_URL=http://host:port cargo test --release --test sglang_sessions live -- --ignored`).
/// Every turn's prompt_tokens must equal the prompt (checked by the client: a mismatch turns
/// sessions off), and turns after the first of each session send only the new ids.
#[test]
#[ignore]
fn live_sessions_against_a_server() {
    let url = std::env::var("AFFORDANCE_SGLANG_URL").expect("AFFORDANCE_SGLANG_URL");
    let enc = affordance::emit::Encoder::from_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/ref/tokenizers/deepseek-ai__DeepSeek-V4.1-Flash.json"
    ))
    .unwrap();
    let ids = |s: &str| enc.tok.encode(s, false).unwrap().get_ids().to_vec();
    let eos = enc.token_id("<｜end▁of▁sentence｜>").unwrap();
    let mut model = Sglang::new(&url, 1);
    model.stream = std::env::var("AFFORDANCE_STREAM").is_ok();
    model.sampling = json!({"temperature": 0.0});
    model.sessions = Some(Sessions::new(4, Duration::from_secs(120)));
    let mut prompt = ids(
        "<｜begin▁of▁sentence｜><｜User｜>Count upward from 1, one number per reply, and nothing else.<｜Assistant｜></think>",
    );
    for turn in 0..20 {
        let r = model
            .generate(&GenRequest::new(
                "aff-live",
                prompt.clone(),
                16,
                vec![eos],
                None,
                CancelToken::new(),
            ))
            .unwrap();
        let meta = r.model_meta.clone().unwrap_or_default();
        eprintln!(
            "turn {turn}: prompt {} cached {:?} prompt_tokens {} reply {:?}",
            prompt.len(),
            r.cached_tokens,
            meta["prompt_tokens"],
            enc.tok.decode(&r.ids, false).unwrap()
        );
        assert!(
            !model.sessions.as_ref().unwrap().is_off(),
            "sessions went off at turn {turn}"
        );
        assert_eq!(meta["prompt_tokens"].as_u64(), Some(prompt.len() as u64));
        prompt = [
            &prompt[..],
            &r.ids[..],
            &ids("<｜User｜>Next.<｜Assistant｜></think>")[..],
        ]
        .concat();
    }
}
