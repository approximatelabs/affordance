//! `affordance host`: a deployment driven by a frontend over stdio, one JSON object a line, as a
//! browser's DevTools protocol drives a page. Commands come in on stdin; events go out on stdout.
//!
//! In:  the control protocol (`control.rs`: `event`, `close`, `status`, `set_model_limit`,
//!      `subscribe`; an event goes to the first instance unless it names a `target`), and
//!      `{"type":"answer","request","value"}`, which answers a question. The end of stdin closes
//!      the run, which `--restore` continues.
//! Out: `ready {instance}` once the instances are running; `delta {instance, task, text, reasoning?}` (a
//!      model's reply as it is generated); `output {instance, task, text}` (a command's output as it
//!      is written); `trace {instance, record}` (renders, replies, finished tools, handler errors);
//!      `ui {instance, value}` and `question {instance, request, ...}` (the app's calls on the machine
//!      `ui`); answers (`result`, `error`); `exit {error?}`.
//!
//! The machine `ui` is the frontend: a call with `ask` waits for the frontend's `answer` (a
//! permission, a choice) and returns its value; any other call is shown and returns null at once.
//! What to ask, and when, is the app's (`views/lib/frontend.tsx`).
use affordance::emit::Encoder;
use affordance::runtime::{CallContext, Control, Executor, Observed, Observer, Runtime};
use serde_json::{Value as J, json};
use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
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
            self.out
                .send(json!({"type": "ui", "instance": ctx.client.name, "value": args}));
            return Ok(J::Null);
        }
        let request = self.next.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = channel();
        self.questions.lock().unwrap().insert(request, tx);
        // The app's fields first, the protocol's last: an app cannot set the frame's type, agent or
        // request (a forged `exit`, another question's number).
        let mut event = args.as_object().cloned().map(J::Object).unwrap_or_else(|| json!({}));
        event["type"] = json!("question");
        event["instance"] = json!(ctx.client.name);
        event["request"] = json!(request);
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

/// A model's native ids as text, with the tokenizer of the binding each agent runs on, whole
/// characters only: ids that end inside a character wait for the rest.
struct Decoder {
    /// Each root agent's tokenizer; a child decodes with its root's binding unless it names its own.
    by_agent: HashMap<String, Arc<Encoder>>,
    default: Option<Arc<Encoder>>,
    pending: HashMap<(String, u64), Vec<u32>>,
}

impl Decoder {
    fn text(&mut self, agent: &str, task: u64, ids: &[u32]) -> Option<String> {
        let root = agent.split('/').next().unwrap_or(agent);
        let enc = self
            .by_agent
            .get(agent)
            .or_else(|| self.by_agent.get(root))
            .or(self.default.as_ref())?;
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
    let by_agent = rt
        .specs()
        .iter()
        .filter_map(|s| Some((s.id.clone(), rt.encoder(s.model.as_deref().unwrap_or("default"))?)))
        .collect();
    let decoder = Mutex::new(Decoder {
        by_agent,
        default: rt.encoder("default"),
        pending: HashMap::new(),
    });
    let watcher = out.clone();
    rt.observe(Observer(Arc::new(move |agent: &str, task: u64, seen: Observed<'_>| {
        let event = match seen {
            Observed::Ids(ids) => match decoder.lock().unwrap().text(agent, task, ids) {
                Some(text) if !text.is_empty() => {
                    json!({"type": "delta", "instance": agent, "task": task, "text": text})
                }
                _ => return,
            },
            Observed::Text(text) => json!({"type": "delta", "instance": agent, "task": task, "text": text}),
            Observed::Reasoning(text) => {
                json!({"type": "delta", "instance": agent, "task": task, "text": text, "reasoning": true})
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
                json!({"type": "delta", "instance": agent, "task": task, "call": call})
            }
            Observed::Output(bytes) => {
                json!({"type": "output", "instance": agent, "task": task, "text": String::from_utf8_lossy(bytes)})
            }
            Observed::Trace(record) => json!({"type": "trace", "instance": agent, "record": record}),
        };
        watcher.send(event);
    })));
    let control = rt.control();
    // `ready` once the agents are running (a restore has finished), or never if the run ends first.
    {
        let (control, out, target) = (control.clone(), out.clone(), target.clone());
        std::thread::spawn(move || {
            loop {
                match control.status()["state"].as_str() {
                    Some("running") => break out.send(json!({"type": "ready", "instance": target})),
                    Some("closing" | "closed") => break,
                    _ => std::thread::sleep(Duration::from_millis(10)),
                }
            }
        });
    }
    std::thread::spawn(move || read_commands(control, out, questions, target));
    Ok(())
}

fn read_commands(control: Control, out: Out, questions: Questions, target: String) {
    let sink: crate::control::Sink = {
        let out = out.clone();
        Arc::new(move |line: J| out.send(line))
    };
    let done = Arc::new(AtomicBool::new(false));
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
        if crate::control::handle(&control, &command, &target, &sink, &done).is_some() {
            continue;
        }
        let error = match command["type"].as_str() {
            Some("answer") => {
                let request = command["request"].as_u64().unwrap_or(0);
                match questions.lock().unwrap().remove(&request) {
                    Some(tx) => {
                        let _ = tx.send(command.get("value").cloned().unwrap_or(J::Null));
                        None
                    }
                    None => Some(format!("no question {request} is waiting")),
                }
            }
            other => Some(format!(
                "unknown command type {other:?} ({}, answer)",
                crate::control::TYPES
            )),
        };
        if let Some(message) = error {
            out.send(match command.get("id") {
                Some(id) => json!({"type": "error", "id": id, "message": message}),
                None => json!({"type": "error", "message": message}),
            });
        }
    }
    // The frontend has gone: the run closes, its work kept for `--restore`.
    done.store(true, Ordering::SeqCst);
    control.close(Duration::ZERO);
}
