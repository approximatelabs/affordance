//! Scenario tests for apps (`affordance test APP.tsx SCENARIO.json`): run an app as the runtime does, with
//! a scripted model and scripted results for its calls, and check what it did. The scenario is a
//! list of steps, applied in order:
//!
//! ```json
//! {"props": {"task": "..."}, "syntax": "glm", "steps": [
//!   {"event": "message", "payload": {"text": "hi"}},
//!   {"expect": {"model_calls": 1, "prompt_contains": ["hi"]}},
//!   {"model": {"think": "A greeting.", "text": "Hello!"}},
//!   {"model": {"calls": [{"name": "lookup", "args": {"q": "weather"}}]}},
//!   {"result": {"output": "sunny", "exit": 0}},
//!   {"advance_ms": 1000},
//!   {"expect": {"idle": true, "log": [{"kind": "input"}, {"kind": "screen"}, {"kind": "model"}]}}
//! ]}
//! ```
//!
//! - `event`: deliver an event (`payload` defaults to `{}`).
//! - `model`: answer the oldest open model call with a reply in the app's reply `syntax` (`glm`, the
//!   default, or `chat`): `think`, `text` and `calls` (`[{name, args}]`), or a raw string.
//!   With several model calls open, say which with `"where": "text"` (a step key, beside `model`):
//!   the oldest open call whose prompt contains the text.
//!   `"aborted": true`: the reply is what was generated before the app's signal stopped the call
//!   (the output has `aborted: true`). `"cut_off": true`: the reply ran out of tokens.
//! - `result` / `error`: answer the oldest open call (`machine:home` for `harness.machine.call`, `send`, ...;
//!   `"name"` picks the oldest of that name).
//! - `advance_ms`: move the clock (wakes timed waits).
//! - `expect`: check the instance now. Keys: `model_calls` / `calls` (how many are open), `idle`
//!   (no model call and no call open; a `harness.wait` that is waiting is idle), `stopped`
//!   (`true`, `false` or the stop value), `state` and `log` (the records of the app's `useLog()`; subset match: each object key
//!   given must match; a list matches record by record and in length),
//!   `prompt_contains` / `prompt_lacks` (the open model call's prompt text; with several open,
//!   pick one with `"where"` as for `model`), `issued` (calls issued since the last `expect`, as
//!   `[{name, args}]`, subset match).
//! - `dump`: print the state, the log and every open call's prompt (`{"dump": true}`).
//!
//! Unknown keys are errors. The run fails at the first step that cannot be applied or the first
//! expectation not met, and says what it found. Child instances (`<Mount>`) are not run.
use crate::View;
use crate::effects::Outcome;
use crate::emit::Encoder;
use crate::runtime::{Command, Input, Instance, InstanceConfig};
use serde_json::{Value as J, json};
use std::collections::BTreeMap;
use std::path::Path;

const T0: i64 = 1_791_000_000_000;

/// What a scenario run found: `Ok` with a summary, or the first failure with the instance's state.
pub struct Report {
    pub ok: bool,
    pub log: Vec<String>,
    pub dumps: Vec<String>,
    pub failure: Option<String>,
    pub records: Vec<J>,
    pub state: J,
}

struct Run<'a> {
    view: &'a View,
    enc: &'a Encoder,
    agent: Instance,
    clock: i64,
    syntax: String,
    /// Open model calls: (task, prompt ids, stop ids).
    model: Vec<(u64, Vec<u32>, Vec<u32>)>,
    /// Open calls: (task, name, args).
    calls: Vec<(u64, String, J)>,
    /// Calls issued since the last `expect`.
    issued: Vec<J>,
    log: Vec<String>,
    dumps: Vec<String>,
}

/// The keys each kind of step takes.
const STEPS: [(&str, &[&str]); 7] = [
    ("event", &["payload"]),
    ("model", &["where", "aborted", "cut_off"]),
    ("result", &["name"]),
    ("error", &["name"]),
    ("advance_ms", &[]),
    ("expect", &[]),
    ("dump", &[]),
];
const EXPECT: [&str; 11] = [
    "model_calls",
    "calls",
    "idle",
    "stopped",
    "state",
    "log",
    "issued",
    "prompt_contains",
    "prompt_lacks",
    "where",
    "dump",
];

fn check_keys(step: &J) -> Result<(), String> {
    let Some(m) = step.as_object() else {
        return Err("a step is an object".into());
    };
    let kinds: Vec<&str> = STEPS.iter().map(|s| s.0).filter(|k| m.contains_key(*k)).collect();
    let [kind] = kinds[..] else {
        return Err(format!("a step has exactly one of {:?}", STEPS.map(|s| s.0)));
    };
    let extra = STEPS.iter().find(|s| s.0 == kind).unwrap().1;
    if let Some(k) = m.keys().find(|k| k.as_str() != kind && !extra.contains(&k.as_str())) {
        return Err(format!("unknown key `{k}` in a `{kind}` step (it takes {:?})", extra));
    }
    if let Some(r) = m.get("model").and_then(|r| r.as_object())
        && let Some(k) = r.keys().find(|k| !["think", "text", "calls"].contains(&k.as_str()))
    {
        return Err(format!(
            "unknown key `{k}` in a model reply (it takes think, text, calls; `where` goes beside `model`)"
        ));
    }
    if let Some(k) = m
        .get("expect")
        .and_then(|e| e.as_object())
        .and_then(|e| e.keys().find(|k| !EXPECT.contains(&k.as_str())).cloned())
    {
        return Err(format!("unknown expectation `{k}` (expect takes {EXPECT:?})"));
    }
    Ok(())
}

impl<'a> Run<'a> {
    fn absorb(&mut self, cmds: Vec<Command>) {
        for c in cmds {
            match c {
                Command::Generate { task, ids, stop, .. } => {
                    self.log
                        .push(format!("model call (task {task}, {} prompt tokens)", ids.len()));
                    self.model.push((task, ids.to_vec(), stop));
                }
                Command::Call { task, name, args } => {
                    self.log.push(format!("call {name} {args}"));
                    self.issued.push(json!({"name": name, "args": args}));
                    self.calls.push((task, name, args));
                }
                Command::Cancel { task } => {
                    self.model.retain(|m| m.0 != task);
                    self.calls.retain(|c| c.0 != task);
                    self.log.push(format!("cancelled task {task}"));
                }
                // The next `model` step answers it (with `aborted: true` for what it generated first).
                Command::Abort { task } => self.log.push(format!("aborted task {task}")),
                Command::Timer { .. } => {}
                other => self.log.push(format!(
                    "not run by affordance test: {}",
                    serde_json::to_string(&other).unwrap_or_default()
                )),
            }
        }
    }

    fn feed(&mut self, input: Input) -> Result<(), String> {
        let cmds = self.agent.handle(self.view, self.enc, input)?;
        self.absorb(cmds);
        Ok(())
    }

    fn tick(&mut self) -> i64 {
        self.clock += 10;
        self.clock
    }

    fn prompt(&self, i: usize) -> String {
        self.enc.tok.decode(&self.model[i].1, false).unwrap_or_default()
    }

    /// The open model call a step means: the one open, or the oldest whose prompt has `where`.
    fn pick(&self, where_: Option<&str>) -> Result<usize, String> {
        match (where_, self.model.len()) {
            (_, 0) => Err("no model call is open".into()),
            (Some(w), _) => (0..self.model.len())
                .find(|&i| self.prompt(i).contains(w))
                .ok_or(format!("no open model call's prompt contains {w:?}")),
            (None, 1) => Ok(0),
            (None, n) => Err(format!(
                "{n} model calls are open; say which with \"where\": text in its prompt. Their prompts end:\n{}",
                (0..n)
                    .map(|i| format!("- {}", tail(&self.prompt(i), 300)))
                    .collect::<Vec<_>>()
                    .join("\n")
            )),
        }
    }

    /// A scripted reply as the model would generate it in this app's reply syntax.
    fn reply_ids(&self, reply: &J, stop: &[u32]) -> Result<Vec<u32>, String> {
        let calls: Vec<(String, J)> = reply["calls"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|c| (c["name"].as_str().unwrap_or_default().to_string(), c["args"].clone()))
            .collect();
        let (text, stop_tok) = match (reply, self.syntax.as_str()) {
            (J::String(s), _) => (s.clone(), None),
            (_, "glm") => {
                let mut t = format!(
                    "{}</think>{}",
                    reply["think"].as_str().unwrap_or(""),
                    reply["text"].as_str().unwrap_or("")
                );
                for (name, args) in &calls {
                    t.push_str(&format!("<tool_call>{name}"));
                    for (k, v) in args.as_object().into_iter().flatten() {
                        let v = v.as_str().map(str::to_owned).unwrap_or_else(|| v.to_string());
                        t.push_str(&format!("<arg_key>{k}</arg_key><arg_value>{v}</arg_value>"));
                    }
                    t.push_str("</tool_call>");
                }
                (
                    t,
                    Some(if calls.is_empty() {
                        "<|user|>"
                    } else {
                        "<|observation|>"
                    }),
                )
            }
            (_, "chat") => {
                let calls: Vec<J> = calls.iter().map(|(n, a)| json!({"name": n, "arguments": a})).collect();
                (json!({"reasoning": reply["think"].as_str().unwrap_or(""), "content": reply["text"].as_str().unwrap_or(""), "tool_calls": calls}).to_string(), None)
            }
            (_, s) => {
                return Err(format!(
                    "affordance test writes replies in the `glm` and `chat` syntaxes, not `{s}`; give the reply as a raw string"
                ));
            }
        };
        let mut ids = self
            .enc
            .tok
            .encode(text.as_str(), false)
            .map_err(|e| e.to_string())?
            .get_ids()
            .to_vec();
        if let Some(s) = stop_tok
            .and_then(|s| self.enc.token_id(s))
            .filter(|id| stop.contains(id))
        {
            ids.push(s);
        }
        Ok(ids)
    }

    fn step(&mut self, step: &J) -> Result<(), String> {
        check_keys(step)?;
        if !step["dump"].is_null() {
            let prompts: Vec<String> = (0..self.model.len())
                .map(|i| format!("open model call {}:\n{}", i + 1, tail(&self.prompt(i), 3000)))
                .collect();
            let calls: Vec<String> = self
                .calls
                .iter()
                .map(|c| format!("open call {} {}", c.1, c.2))
                .collect();
            self.dumps.push(format!(
                "state: {}\nlog: {}\n{}\n{}",
                short(&self.agent.state()),
                short(&json!(log_records(&self.agent.state()))),
                prompts.join("\n"),
                calls.join("\n")
            ));
            return Ok(());
        }
        if let Some(name) = step["event"].as_str() {
            let clock = self.tick();
            let payload = if step["payload"].is_null() {
                json!({})
            } else {
                step["payload"].clone()
            };
            return self.feed(Input::Event {
                name: name.into(),
                payload,
                id: None,
                clock_ms: clock,
            });
        }
        if !step["model"].is_null() {
            let i = self.pick(step["where"].as_str())?;
            let (task, _, stop) = self.model.remove(i);
            let ids = self.reply_ids(&step["model"], &stop)?;
            let clock = self.tick();
            return self.feed(Input::Reply {
                server: None,
                task,
                ids,
                cut_off: step["cut_off"].as_bool().unwrap_or(false),
                clock_ms: clock,
                prompt: None,
                aborted: step["aborted"].as_bool().unwrap_or(false),
            });
        }
        if !step["result"].is_null() || !step["error"].is_null() {
            let i = match step["name"].as_str() {
                Some(n) => self
                    .calls
                    .iter()
                    .position(|c| c.1 == n)
                    .ok_or(format!("no open `{n}` call to answer"))?,
                None if self.calls.is_empty() => return Err("a `result` step, but the app has no open call".into()),
                None => 0,
            };
            let (task, _, _) = self.calls.remove(i);
            let outcome = match step["error"].as_str() {
                Some(e) => Outcome::Error(e.into()),
                None => Outcome::Ok(step["result"].clone()),
            };
            let clock = self.tick();
            return self.feed(Input::Result {
                task,
                outcome,
                clock_ms: clock,
            });
        }
        if let Some(ms) = step["advance_ms"].as_i64() {
            self.clock += ms;
            let clock = self.clock;
            return self.feed(Input::Tick { clock_ms: clock });
        }
        if let Some(e) = step["expect"].as_object() {
            return self.expect(e);
        }
        Err(format!("unknown step {step}"))
    }

    fn expect(&mut self, e: &serde_json::Map<String, J>) -> Result<(), String> {
        let mut failed = vec![];
        for (k, want) in e {
            let got = match k.as_str() {
                "model_calls" => json!(self.model.len()),
                "calls" => json!(self.calls.len()),
                "idle" => json!(self.model.is_empty() && self.calls.is_empty()),
                "where" => continue,
                "stopped" => match (want, self.agent.stopped()) {
                    (J::Bool(_), s) => json!(s.is_some()),
                    (_, s) => s.cloned().unwrap_or(J::Null),
                },
                "state" => self.agent.state(),
                "log" => json!(log_records(&self.agent.state())),
                "issued" => json!(self.issued),
                "prompt_contains" | "prompt_lacks" => {
                    let text = match self.pick(e.get("where").and_then(|w| w.as_str())) {
                        Ok(i) => self.prompt(i),
                        Err(m) => {
                            failed.push(format!("{k}: {m}"));
                            continue;
                        }
                    };
                    for s in want.as_array().into_iter().flatten().filter_map(|s| s.as_str()) {
                        if text.contains(s) != (k == "prompt_contains") {
                            failed.push(format!("{k} {s:?}: the prompt is\n{}", tail(&text, 1500)));
                        }
                    }
                    continue;
                }
                other => {
                    failed.push(format!("unknown expectation `{other}`"));
                    continue;
                }
            };
            if !matches(want, &got) {
                failed.push(format!("{k}: expected {want}, found {}", short(&got)));
            }
        }
        self.issued.clear();
        if failed.is_empty() {
            Ok(())
        } else {
            Err(failed.join("\n"))
        }
    }
}

/// The records of the app's log (`useLog()`, hook `log`: `views/lib/log.tsx`).
pub fn log_records(state: &J) -> Vec<J> {
    state["__ui"]["@log"]["records"].as_array().cloned().unwrap_or_default()
}

/// `want` matches `got`: objects by the keys `want` gives, lists element by element and in length.
fn matches(want: &J, got: &J) -> bool {
    match (want, got) {
        (J::Object(w), J::Object(g)) => w
            .iter()
            .all(|(k, v)| g.get(k).is_some_and(|gv| matches(v, gv)) || (v.is_null() && !g.contains_key(k))),
        (J::Array(w), J::Array(g)) => w.len() == g.len() && w.iter().zip(g).all(|(a, b)| matches(a, b)),
        (J::Number(a), J::Number(b)) => a.as_f64() == b.as_f64(),
        _ => want == got,
    }
}

fn tail(s: &str, n: usize) -> String {
    let cs: Vec<char> = s.chars().collect();
    if cs.len() <= n {
        s.to_string()
    } else {
        format!("...{}", cs[cs.len() - n..].iter().collect::<String>())
    }
}

/// A value for an error message, without record token ids.
fn short(v: &J) -> String {
    fn strip(v: &J) -> J {
        match v {
            J::Object(m) => J::Object(
                m.iter()
                    .filter(|(k, _)| k.as_str() != "ids")
                    .map(|(k, v)| (k.clone(), strip(v)))
                    .collect(),
            ),
            J::Array(a) => J::Array(a.iter().map(strip).collect()),
            v => v.clone(),
        }
    }
    let s = strip(v).to_string();
    if s.len() > 2000 {
        format!("{}...", &s[..s.char_indices().nth(2000).map_or(s.len(), |c| c.0)])
    } else {
        s
    }
}

/// Run `scenario` against the app at `app` (component `entry`), bound to `enc`.
pub fn run(app: &Path, entry: &str, enc: &Encoder, scenario: &J) -> Result<Report, String> {
    // The app's library as `affordance run` finds it, so an app that imports `@affordance/...`
    // (the one `affordance init` writes) can be tested.
    let library = crate::library::for_app(app)?;
    let templates: Vec<String> = scenario["template"].as_str().map(str::to_string).into_iter().collect();
    let view = View::load_for(app, entry, enc, &[library], &templates)?;
    let cfg = InstanceConfig {
        context: None,
        retry: Default::default(),
        // `"template": "messages"`: the app runs as on a hosted chat binding (what `affordance
        // init` writes), so `ModelContext` has a binding to render for.
        template: match scenario["template"].as_str() {
            Some(name) => Some(crate::runtime::model::ModelTemplate::new(name, json!({}))?),
            None => None,
        },
        id: "app".into(),
        props: if scenario["props"].is_null() {
            json!({})
        } else {
            scenario["props"].clone()
        },
        env: BTreeMap::new(),
        max_new: 1024,
        max_turns: Some(scenario["max_turns"].as_u64().unwrap_or(50)),
        max_prompt: None,
        start_ms: T0,
        seed: 7,
        machines: vec!["home".into()],
        model: Some("default".into()),
    };
    let mut agent = Instance::new(cfg)?;
    let start = agent.start(&view, enc)?;
    let mut r = Run {
        view: &view,
        enc,
        agent,
        clock: T0,
        syntax: scenario["syntax"]
            .as_str()
            .unwrap_or(if scenario["template"] == "messages" {
                "chat"
            } else {
                "glm"
            })
            .into(),
        model: vec![],
        calls: vec![],
        issued: vec![],
        log: vec![],
        dumps: vec![],
    };
    r.absorb(start);
    let mut failure = None;
    // A scenario is its steps: one with none (a misspelled key) must not pass by doing nothing.
    if let Some(key) = scenario.as_object().and_then(|o| {
        o.keys().find(|k| {
            ![
                "steps",
                "tokenizer",
                "entry",
                "props",
                "name",
                "app",
                "syntax",
                "template",
                "max_turns",
            ]
            .contains(&k.as_str())
        })
    }) {
        return Err(format!("unknown key `{key}` in the scenario"));
    }
    let Some(steps) = scenario["steps"].as_array() else {
        return Err("a scenario needs `steps`: a list".into());
    };
    for (i, step) in steps.iter().enumerate() {
        r.log.push(format!("step {i}: {step}"));
        if let Err(e) = r.step(step) {
            failure = Some(format!("step {i} {step}:\n{e}"));
            break;
        }
    }
    Ok(Report {
        ok: failure.is_none(),
        failure,
        log: r.log,
        dumps: r.dumps,
        records: log_records(&r.agent.state()),
        state: r.agent.state().clone(),
    })
}
