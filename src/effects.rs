//! Host requests and Promise-facing platform operations.
use crate::interp::{Interp, SUSPEND};
use crate::value::Value;
use serde::{Deserialize, Serialize};
use serde_json::Value as J;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
/// What a handler asks of its instance or host: one effect.
pub enum Request {
    /// A harness API the host or the instance serves (`machine:<role>`, `send`, `store.*`, `wait`, ...).
    Call {
        /// The API's name: `model`, `dispatch`, `wait`, `send`, `store.get`, `machine:<role>`, ...
        app: String,
        /// Its arguments.
        args: J,
        /// The `AbortSignal` the call was given (`{signal}`), by its id.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        signal: Option<u64>,
    },
    /// The instance's clock (`harness.now()`).
    Now,
    /// The next number of the instance's random sequence (`harness.random()`).
    Random,
    /// Stop the instance (`harness.stop(value)`).
    Stop {
        /// What it stops with.
        value: J,
    },
    /// `harness.render(<App/>)`: a fragment laid out to exact ids for the instance's model, with
    /// the spans that say how to read the reply (`protocol`) and what it may call (affordances).
    /// The outcome is `{text, reasoning, calls}`. `options`: the call's `{max_new, sampling, tag,
    /// priority}` (see `runtime::GenOptions`).
    Render {
        /// The view laid out: its exact token ids.
        ids: Vec<u32>,
        /// The spans that read the reply: the protocol and the actions it may call.
        spans: Vec<crate::emit::Span>,
        #[serde(default)]
        /// The call's options.
        options: J,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", content = "value", rename_all = "snake_case")]
/// How a request ended.
pub enum Outcome {
    /// Its result.
    Ok(J),
    /// Why it failed; a typed error keeps its name (`AbortError: ...`).
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// A request with its outcome, once it has one: an entry of a handler's transcript.
pub struct Effect {
    /// What was asked.
    pub request: Request,
    /// How it ended; none while it is outstanding.
    pub outcome: Option<Outcome>,
    /// Outcome arrival order when delivering a complete transcript.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settled: Option<u64>,
}

/// The `AbortSignal`s of one instance (the browser's `AbortController`): a counter naming each
/// new signal, and the aborted ones with their reasons (`{reason?}`; none: the default
/// `AbortError`). Deterministic like the clock and the random state: a handler creates and aborts
/// signals in its own code, so replaying it does the same.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Signals {
    /// The id the next signal gets.
    pub next: u64,
    /// The aborted signals, each with `{reason?}`.
    pub aborted: std::collections::BTreeMap<u64, J>,
    /// Signals aborted since the instance last took them (it aborts the calls bound to them).
    #[serde(skip)]
    pub fresh: Vec<u64>,
}

/// What an aborted call rejects with.
pub(crate) const ABORTED: &str = "AbortError: This operation was aborted";

pub use crate::microtasks::Promise;
pub(crate) use crate::microtasks::{Combinator, Rejection, Settle};

#[derive(Debug, Clone)]
/// A handler's transcript of requests: recorded as it runs, or checked against as it replays.
pub struct Effects {
    /// The requests in order, with the outcomes known so far.
    pub slots: Vec<Effect>,
    /// The index the next request takes.
    pub cursor: usize,
    base: usize,
}
impl Effects {
    /// A transcript to replay against `slots`.
    pub fn new(slots: Vec<Effect>) -> Self {
        Self {
            slots,
            cursor: 0,
            base: 0,
        }
    }
    pub(crate) fn resume(base: usize) -> Self {
        Self {
            slots: vec![],
            cursor: base,
            base,
        }
    }
    pub(crate) fn register(&mut self, request: Request) -> Result<usize, String> {
        let i = self.cursor;
        if let Some(slot) = self.slots.get(i - self.base) {
            if slot.request != request {
                return Err(format!("effect transcript diverged at request {i}"));
            }
        } else {
            self.slots.push(Effect {
                request,
                outcome: None,
                settled: None,
            });
        }
        self.cursor += 1;
        Ok(i)
    }
}

impl Interp<'_> {
    /// `async` calls return promises inside an effectful handler.
    /// Elsewhere (render, plain `act`) they run synchronously and return their value.
    pub(crate) fn async_active(&self) -> bool {
        self.acting && self.effects.is_some()
    }

    pub(crate) fn run_async(
        &mut self,
        body: impl FnOnce(&mut Self, Rc<Promise>) -> Result<Value, String>,
    ) -> Result<Value, String> {
        let target = Promise::pending();
        let result = body(self, target.clone());
        self.complete_call(&target, result);
        Ok(Value::Promise(target))
    }
    /// A started body returns its own promise; native calls and binding failures finish here.
    pub(crate) fn complete_call(&mut self, target: &Rc<Promise>, result: Result<Value, String>) {
        match result {
            Ok(Value::Promise(value)) if Rc::ptr_eq(&value, target) => {}
            Ok(value) => self
                .realm
                .as_ref()
                .expect("execution realm")
                .borrow_mut()
                .resolve(self.task, target, value),
            Err(text) if text == SUSPEND => {}
            Err(text) => {
                let reason = self.caught(&text);
                self.realm
                    .as_ref()
                    .expect("execution realm")
                    .borrow_mut()
                    .settle(target, Settle::Err(Rejection { text, reason }));
            }
        }
    }
    pub(crate) fn promise_method(&mut self, p: &Value, name: &str, args: Vec<Value>) -> Result<Value, String> {
        if !self.async_active() {
            return Err(format!("promise.{name} needs a handler"));
        }
        let Value::Promise(source) = p else { unreachable!() };
        let f = args.first().cloned().filter(Value::is_callable);
        let g = args.get(1).cloned().filter(Value::is_callable);
        let callback = match name {
            "then" => crate::microtasks::Reaction::Then(f, g),
            "catch" => crate::microtasks::Reaction::Then(None, f),
            _ => crate::microtasks::Reaction::Finally(f),
        };
        Ok(Value::Promise(self.realm().borrow_mut().react(
            self.task,
            source.clone(),
            callback,
        )))
    }
    pub(crate) fn realm(&mut self) -> crate::microtasks::Realm {
        if self.realm.is_none() {
            let realm = crate::microtasks::Queue::new();
            realm.borrow_mut().begin(self.task, self.run);
            self.realm = Some(realm);
        }
        let realm = self.realm.as_ref().unwrap().clone();
        if !realm.borrow().groups.contains_key(&self.task) {
            realm.borrow_mut().begin(self.task, self.run);
        }
        realm
    }
    pub(crate) fn promise_combinator(&mut self, kind: Combinator, input: &Value) -> Value {
        match crate::interp::iterate(input) {
            Ok(values) => {
                let realm = self.realm();
                Value::Promise(realm.borrow_mut().combine(self.task, kind, values))
            }
            Err(text) => {
                let reason = self.caught(&text);
                Value::Promise(Promise::settled(Settle::Err(Rejection { text, reason })))
            }
        }
    }
    pub(crate) fn promise_resolve(&mut self, value: Value) -> Value {
        let realm = self.realm();
        Value::Promise(realm.borrow_mut().promise(self.task, value))
    }
    pub(crate) fn resolve_promise(&mut self, value: &Value) -> Result<Value, String> {
        let Value::Promise(p) = value else {
            return Ok(value.clone());
        };
        // Rendering and synchronous source helpers do not retain a suspended native stack.
        // Handler awaits are suspension instructions and never enter this path.
        if !self.async_active()
            && let Some(realm) = self.realm.clone()
        {
            while realm.borrow_mut().next_task() == Some(self.task) {
                crate::microtasks::Queue::native(&realm, self.task)?;
            }
        }
        p.handled.set(true);
        match p.completed() {
            Some(Settle::Ok(value)) => Ok(value),
            Some(Settle::Err(error)) => {
                self.thrown_text = crate::error_message(&error.text).into();
                self.thrown = Some(error.reason);
                Err(error.text)
            }
            _ => Err(SUSPEND.into()),
        }
    }
    pub(crate) fn error_object(&self, e: &str) -> Value {
        crate::microtasks::error(e).reason
    }

    /// A machine call, `call(args, {signal}?)`, on the machine `role` ("": the instance's first).
    pub(crate) fn machine_call(&mut self, role: Value, args: Vec<Value>) -> Result<Value, String> {
        let mut args = args.into_iter();
        let call = args.next().unwrap_or(Value::None);
        let options = args.next().unwrap_or(Value::Undef);
        self.harness_method("machine.call", vec![call, role, options])
    }

    pub(crate) fn harness_method(&mut self, name: &str, args: Vec<Value>) -> Result<Value, String> {
        if !self.acting {
            return Err(format!("harness.{name} is an effect and may only run in a handler"));
        }
        // What goes to the host is JSON: a value that contains itself cannot be sent.
        for arg in &args {
            match crate::value::check_graph(arg, 128) {
                Ok(()) => {}
                Err(crate::value::GraphFault::Cycle) => {
                    return Err("TypeError: Converting circular structure to JSON (a value that contains itself cannot be sent to the host)".into());
                }
                Err(_) => return Err("RangeError: a value sent to the host nests at most 128 levels".into()),
            }
        }
        let a0 = args.first().cloned().unwrap_or(Value::None);
        // The abort signals under `AbortController` (`web.tsx`): synchronous, the instance's own.
        let id = || match &a0 {
            Value::Int(n) if *n >= 0 => Ok(*n as u64),
            _ => Err(format!("harness.{name} expects a signal id")),
        };
        match name {
            "signal.create" => {
                let n = self.signals.next;
                self.signals.next += 1;
                return Ok(Value::Int(n as i64));
            }
            "signal.abort" => {
                let n = id()?;
                if n < self.signals.next && !self.signals.aborted.contains_key(&n) {
                    let reason = match args.get(1) {
                        None | Some(Value::Undef) => serde_json::json!({}),
                        Some(r) => serde_json::json!({"reason": r.to_wire()}),
                    };
                    self.signals.aborted.insert(n, reason);
                    self.signals.fresh.push(n);
                }
                return Ok(Value::Undef);
            }
            "signal.state" => {
                let n = id()?;
                return Ok(self.signals.aborted.get(&n).map_or(Value::None, Value::from_json));
            }
            _ => {}
        }
        // `{signal}` in a call's options: its id goes with the request. A call whose signal is
        // already aborted is never issued; it rejects at once, as `fetch` does.
        let options_at = match name {
            "model" if matches!(a0, Value::None | Value::Undef | Value::Frag(_)) => 1,
            "model" => 0,
            "machine.call" => 2,
            _ => usize::MAX,
        };
        let signal = match args.get(options_at).map(|o| crate::interp::get_attr(o, "signal")) {
            None | Some(Err(_)) | Some(Ok(Value::Undef | Value::None)) => None,
            Some(Ok(s)) => match crate::interp::get_attr(&s, "__signal") {
                Ok(Value::Int(n)) if n >= 0 && (n as u64) < self.signals.next => Some(n as u64),
                _ => return Err("TypeError: the signal option must be an AbortSignal".into()),
            },
        };
        if signal.is_some_and(|n| self.signals.aborted.contains_key(&n)) {
            return Ok(Value::Promise(Promise::settled(Settle::Err(Rejection {
                text: ABORTED.into(),
                reason: self.error_object(ABORTED),
            }))));
        }
        // The options as sent: the signal by its id.
        let with_signal = |mut options: J| {
            if let (Some(n), J::Object(o)) = (signal, &mut options) {
                o.insert("signal".into(), n.into());
            }
            options
        };
        // Platform APIs the host serves, and machine calls: each is one named request.
        let host = |name: &str, args: J| Request::Call {
            app: name.to_owned(),
            args,
            signal: None,
        };
        let request = match name {
            // `harness.send(to, payload, {event})`: the event the receiver gets (default `message`).
            "send" => {
                let mut r =
                    serde_json::json!({"to": a0.to_wire(), "payload": args.get(1).unwrap_or(&Value::None).to_wire()});
                if let Some(e) = args
                    .get(2)
                    .map(Value::to_wire)
                    .and_then(|o| o.get("event").cloned())
                    .filter(|e| e.is_string())
                {
                    r["event"] = e;
                }
                host("send", r)
            }
            "store.put" | "store.get" | "store.search" | "store.list" => host(name, a0.to_wire()),
            // `harness.inspect(targets, {select, since, as})`: a copy of other instances' state.
            "inspect" => {
                let mut r = match args.get(1).map(Value::to_wire) {
                    Some(J::Object(o)) => J::Object(o),
                    None | Some(J::Null) => serde_json::json!({}),
                    _ => return Err("harness.inspect(targets, {select, since, as}): the options are an object".into()),
                };
                r["targets"] = a0.to_wire();
                host("inspect", r)
            }
            "parent" => host(
                "parent",
                serde_json::json!({"name": a0.to_wire(), "args": args.get(1).unwrap_or(&Value::None).to_wire()}),
            ),
            // `harness.wait({ms, events})`: until `ms` passes or one of `events` arrives.
            "wait" if matches!(a0, Value::Map(_)) => host("wait", a0.to_wire()),
            "wait" => return Err("harness.wait expects {ms?, events?}".into()),
            // `harness.machine.call(args)`: this instance's machine; `.get(role).call(args)`: one of several.
            "machine.call" => {
                let role = args.get(1).and_then(Value::as_str).unwrap_or("");
                Request::Call {
                    app: format!("machine:{role}"),
                    args: a0.to_wire(),
                    signal,
                }
            }
            "now" => Request::Now,
            "random" => Request::Random,
            "stop" => Request::Stop { value: a0.to_wire() },
            // `model(opts?)`: the page's own view (the agent lays it out). `model(<View/>, opts?)`
            // below: an ad-hoc view.
            "model" if !matches!(a0, Value::Frag(_)) => {
                let mut opts = if matches!(a0, Value::None | Value::Undef) {
                    args.get(1).map(Value::to_wire).unwrap_or(J::Null)
                } else {
                    a0.to_wire()
                };
                if !matches!(opts, J::Null | J::Object(_)) {
                    return Err(
                        "model(view?, opts?): opts is an object {max_new, sampling, tag, priority, budget, signal}"
                            .into(),
                    );
                }
                if let J::Object(o) = &mut opts {
                    o.remove("signal");
                }
                Request::Call {
                    app: "model".into(),
                    args: if opts.is_null() { serde_json::json!({}) } else { opts },
                    signal,
                }
            }
            // `dispatch(output, {calls?})`: run a model output's calls (all, or those at the
            // indices in `calls`, in that order) on the view it answered.
            "dispatch" => {
                let call = crate::interp::get_attr(&a0, "__call")
                    .map(|v| v.to_wire())
                    .map_err(|_| "dispatch(output): pass the output of model()".to_string())?;
                let mut request = serde_json::json!({"call": call});
                match args.get(1).map(Value::to_wire) {
                    None | Some(serde_json::Value::Null) => {}
                    Some(serde_json::Value::Object(o)) => match o.get("calls") {
                        None | Some(serde_json::Value::Null) => {}
                        Some(serde_json::Value::Array(calls)) if calls.iter().all(serde_json::Value::is_u64) => {
                            request["calls"] = serde_json::Value::Array(calls.clone())
                        }
                        Some(_) => {
                            return Err("dispatch(output, {calls}): calls is a list of call indices (0, 1, ...)".into());
                        }
                    },
                    Some(_) => return Err("dispatch(output, options): options is an object".into()),
                }
                host("dispatch", request)
            }
            // `model(<View/>, opts)`: an ad-hoc prompt, laid out once at the call.
            "model" => {
                let Value::Frag(items) = &a0 else { unreachable!() };
                let enc = self.encoder.ok_or("model(<View/>) needs a bound encoder")?;
                let mut cache = crate::emit::RunCache::new(false);
                let mut live = crate::emit::Live::new(enc, &mut cache);
                live.safe = self.prog.pragmas.iter().any(|p| p == "safe_text");
                // Laid out for the window this call leaves (its own `max_new`, if it gives one).
                let call_max_new = args
                    .get(1)
                    .and_then(|o| crate::interp::get_attr(o, "max_new").ok())
                    .and_then(|v| match v {
                        Value::Int(n) => usize::try_from(n).ok(),
                        Value::Float(n) if n >= 0.0 && n.fract() == 0.0 => Some(n as usize),
                        _ => None,
                    });
                live.window = self.window.map(|w| {
                    crate::emit::Window {
                        max_new: call_max_new.unwrap_or(w.max_new),
                        ..w
                    }
                    .tokens()
                });
                crate::emit::Out::replay(&mut live, items)?;
                let (ids, spans) = live.finish()?;
                if !spans.iter().any(|s| s.label == "protocol" && s.kind == "label") {
                    return Err("model(<View/>): the view renders through no template app (no protocol span)".into());
                }
                // Only what reading the reply needs; the rest of the render stays in the ids.
                let spans = spans
                    .into_iter()
                    .filter(|s| {
                        (s.label == "protocol" && s.kind == "label")
                            || (s.kind == "affordance" && s.meta.get("handler").is_some())
                            || s.meta.get("img").is_some()
                            || s.kind == "box"
                            || (s.label == "hosted" && s.kind == "label")
                    })
                    .collect();
                let options = args.get(1).map(Value::to_wire).unwrap_or(J::Null);
                if !matches!(options, J::Null | J::Object(_)) {
                    return Err("model(<View/>, opts): opts is an object {max_new, sampling, tag, priority}".into());
                }
                Request::Render {
                    ids,
                    spans,
                    options: with_signal(options),
                }
            }
            _ => return Err(format!("unknown harness method {name}")),
        };
        if let Some((clock, rng)) = self.services.as_mut() {
            if name == "now" {
                return Ok(Value::Int(*clock));
            }
            if name == "random" {
                *rng ^= *rng << 13;
                *rng ^= *rng >> 7;
                *rng ^= *rng << 17;
                return Ok(Value::Float((*rng >> 11) as f64 / (1u64 << 53) as f64));
            }
        }
        let i = self
            .effects
            .as_mut()
            .ok_or("harness effects require an execution context")?
            .register(request)?;
        let realm = self.realm();
        let promise = realm.borrow_mut().external(self.task, i);
        if matches!(name, "now" | "random") {
            let slot = &self.effects.as_ref().unwrap().slots[i];
            if let Some(outcome) = &slot.outcome {
                realm.borrow_mut().reply(self.task, i, &slot.request, outcome);
            }
            self.resolve_promise(&Value::Promise(promise))
        } else {
            Ok(Value::Promise(promise))
        }
    }
}

/// A model output (`model()`'s result) as a value: its `ids` and `screen.ids` are token runs.
pub(crate) fn model_output_value(v: &J) -> Value {
    let tokens = |ids: &J| -> Option<Value> {
        let ids: Vec<u32> = ids
            .as_array()?
            .iter()
            .map(|x| x.as_u64().map(|t| t as u32))
            .collect::<Option<_>>()?;
        Some(Value::tokens(ids.into(), Rc::from(Vec::new())))
    };
    let out = Value::from_json(v);
    if let Value::Map(m) = &out {
        let mut m = m.write().expect("a fresh object");
        if let Some(t) = tokens(&v["ids"]) {
            m.insert(Rc::from("ids"), t);
        }
        if let (Some(t), Some(Value::Map(s))) = (tokens(&v["screen"]["ids"]), m.get("screen")) {
            s.write().expect("a fresh object").insert(Rc::from("ids"), t);
        }
    }
    out
}

/// The name of a typed error (`AbortError: ...`, `TimeoutError: ...`), if `e` is one.
pub fn typed_error(e: &str) -> Option<&'static str> {
    [
        "TypeError",
        "ReferenceError",
        "RangeError",
        "SyntaxError",
        "URIError",
        "EvalError",
        "DataCloneError",
        "AbortError",
        "TimeoutError",
        "InvalidStateError",
        "AggregateError",
        "ContextError",
    ]
    .into_iter()
    .find(|n| e.starts_with(n) && e[n.len()..].starts_with(": "))
}
