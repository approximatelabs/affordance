//! `affordance host`: a deployment driven by a frontend over stdio, one JSON object a line, as a
//! browser's DevTools protocol drives a page. Commands come in on stdin; events go out on stdout.
//!
//! In:  `{"type":"prompt","text"}` posts the event `user` (`{text}`) to the root instance;
//!      `{"type":"abort"}` posts the event `abort`; `{"type":"history"}` posts the event `history`
//!      (the app shows the conversation so far); `{"type":"event","name","payload"}` posts any
//!      event; `{"type":"answer","request","value"}` answers a question; `{"type":"shutdown"}`
//!      (or the end of stdin) ends the run, which `--resume` continues.
//! Out: `ready {agent}`; `delta {agent, task, text, reasoning?}` (a model's reply as it is
//!      generated); `output {agent, task, text}` (a command's output as it is written);
//!      `trace {agent, record}` (renders, replies, finished tools, handler errors); `ui {agent,
//!      value}` and `question {agent, request, ...}` (the app's calls on the machine `ui`); `error
//!      {message}`; `exit {error?}`.
//!
//! The machine `ui` is the frontend: a call with `ask` waits for the frontend's `answer` (a
//! permission, a choice) and returns its value; any other call is shown and returns null at once.
//! What to ask, and when, is the app's (`views/lib/frontend.tsx`).
use affordance::emit::Encoder;
use affordance::runtime::{CallContext, Executor, Inbox, Observed, Observer, Runtime};
use serde_json::{Value as J, json};
use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Where events go: stdout, one line each, written whole.
#[derive(Clone)]
pub struct Out(Arc<Mutex<std::io::Stdout>>);

impl Out {
    pub fn new() -> Self {
        Out(Arc::new(Mutex::new(std::io::stdout())))
    }
    pub fn send(&self, event: J) {
        let mut out = self.0.lock().unwrap();
        let _ = writeln!(out, "{event}");
        let _ = out.flush();
    }
}

type Questions = Arc<Mutex<HashMap<u64, Sender<J>>>>;

/// The machine `ui`.
struct Ui {
    out: Out,
    questions: Questions,
    next: AtomicU64,
}

impl Executor for Ui {
    fn call(&self, ctx: &CallContext, args: &J) -> Result<J, String> {
        if args.get("ask").is_none() {
            self.out.send(json!({"type": "ui", "agent": ctx.agent, "value": args}));
            return Ok(J::Null);
        }
        let request = self.next.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = channel();
        self.questions.lock().unwrap().insert(request, tx);
        let mut event = json!({"type": "question", "agent": ctx.agent, "request": request});
        for (k, v) in args.as_object().into_iter().flatten() {
            event[k] = v.clone();
        }
        self.out.send(event);
        // Until the frontend answers, or the call is no longer wanted (an abort, the run's end).
        loop {
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(value) => return Ok(value),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) if !ctx.cancel.is_cancelled() => {}
                Err(_) => {
                    self.questions.lock().unwrap().remove(&request);
                    return Err("the question was withdrawn".into());
                }
            }
        }
    }
}

/// A model's native ids as text, whole characters only: ids that end inside a character wait for
/// the rest.
struct Decoder {
    enc: Option<Arc<Encoder>>,
    pending: HashMap<(String, u64), Vec<u32>>,
}

impl Decoder {
    fn text(&mut self, agent: &str, task: u64, ids: &[u32]) -> Option<String> {
        let enc = self.enc.as_ref()?;
        let key = (agent.to_string(), task);
        let mut pending = self.pending.remove(&key).unwrap_or_default();
        pending.extend_from_slice(ids);
        let text = enc.tok.decode(&pending, false).ok()?;
        if text.ends_with('\u{FFFD}') && pending.len() < 16 {
            // Held for the rest of the character; only such a tail is kept, so the map holds an
            // entry for a call only while one is waiting.
            self.pending.insert(key, pending);
            return None;
        }
        Some(text)
    }
}

/// Bind the frontend to the runtime before it runs: the machine `ui`, the observer, and the
/// reader of stdin, which posts to `target`.
pub fn start(rt: &mut Runtime, out: Out, target: String) -> Result<(), String> {
    let questions: Questions = Default::default();
    rt.bind_machine(
        "ui",
        Arc::new(Ui {
            out: out.clone(),
            questions: questions.clone(),
            next: AtomicU64::new(1),
        }),
    );
    let decoder = Mutex::new(Decoder {
        enc: rt.encoder("default"),
        pending: HashMap::new(),
    });
    let watcher = out.clone();
    rt.observe(Observer(Arc::new(move |agent: &str, task: u64, seen: Observed<'_>| {
        let event = match seen {
            Observed::Ids(ids) => match decoder.lock().unwrap().text(agent, task, ids) {
                Some(text) if !text.is_empty() => json!({"type": "delta", "agent": agent, "task": task, "text": text}),
                _ => return,
            },
            Observed::Text(text) => json!({"type": "delta", "agent": agent, "task": task, "text": text}),
            Observed::Reasoning(text) => {
                json!({"type": "delta", "agent": agent, "task": task, "text": text, "reasoning": true})
            }
            Observed::Call {
                index,
                id,
                name,
                arguments,
            } => {
                let mut call = json!({"index": index, "arguments": arguments});
                if let Some(id) = id {
                    call["id"] = json!(id);
                }
                if let Some(name) = name {
                    call["name"] = json!(name);
                }
                json!({"type": "delta", "agent": agent, "task": task, "call": call})
            }
            Observed::Output(bytes) => {
                json!({"type": "output", "agent": agent, "task": task, "text": String::from_utf8_lossy(bytes)})
            }
            Observed::Trace(record) => json!({"type": "trace", "agent": agent, "record": record}),
        };
        watcher.send(event);
    })));
    let inbox = rt.inbox();
    out.send(json!({"type": "ready", "agent": target}));
    std::thread::spawn(move || read_commands(inbox, out, questions, target));
    Ok(())
}

fn read_commands(inbox: Inbox, out: Out, questions: Questions, target: String) {
    // Lines as bytes: one that is not UTF-8 is a bad command (reported below, as bad JSON is),
    // not the end of the input.
    for line in std::io::stdin().lock().split(b'\n') {
        let Ok(line) = line else { break };
        let line = String::from_utf8_lossy(&line);
        if line.trim().is_empty() {
            continue;
        }
        let command: J = match serde_json::from_str(&line) {
            Ok(c) => c,
            Err(e) => {
                out.send(json!({"type": "error", "message": format!("not a JSON command: {e}")}));
                continue;
            }
        };
        let posted = match command["type"].as_str() {
            Some("prompt") => match command["text"].as_str() {
                Some(text) => inbox.post(&target, "user", json!({"text": text})),
                None => Err("prompt needs text".into()),
            },
            Some("abort") => inbox.post(&target, "abort", json!({})),
            Some("history") => inbox.post(&target, "history", json!({})),
            Some("event") => match command["name"].as_str() {
                Some(name) => inbox.post(&target, name, command.get("payload").cloned().unwrap_or(json!({}))),
                None => Err("event needs a name".into()),
            },
            Some("answer") => {
                let request = command["request"].as_u64().unwrap_or(0);
                match questions.lock().unwrap().remove(&request) {
                    Some(tx) => {
                        let _ = tx.send(command.get("value").cloned().unwrap_or(J::Null));
                        Ok(())
                    }
                    None => Err(format!("no question {request} is waiting")),
                }
            }
            Some("shutdown") => break,
            other => Err(format!(
                "unknown command type {other:?} (prompt, abort, history, event, answer, shutdown)"
            )),
        };
        if let Err(message) = posted {
            out.send(json!({"type": "error", "message": message}));
        }
    }
    inbox.close();
}
