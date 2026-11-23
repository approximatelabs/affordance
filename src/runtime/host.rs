//! The multi-agent runtime (the browser): many agents (tabs) in one process.
//!
//! - One `Encoder` for the process; each render worker thread loads its own `View` from it and
//!   owns a fixed set of agents (`View` is single-threaded; an agent never changes thread).
//! - Model calls and executor calls run on thread pools and come back to the agent's worker as
//!   inputs. An agent waiting on a slow command or a `wait` costs no model calls.
//! - Built-in primitives: `send` (the runtime sets the sender), `store.*` (blob storage), plus the
//!   agent-internal ones (`wait`, `measure`, `history.push`, `harness.now/random/stop`).
//!   Everything else is a registered executor (`exec`, `summarize`, ...), allowed per agent by
//!   its capabilities.
//! - The environ is app-scoped: deployment layer, then per-agent. Values of `secrets` keys are
//!   redacted in traces; apps may read them (like a page reading its cookies).
//! - With `dir`, each agent has `agent.json`, `journal.jsonl` (its inputs: the agent can be
//!   rebuilt from it), `trace.jsonl` (renders, model calls, effects, timings) and `store.jsonl`.
use super::agent::{Agent, AgentConfig, Command, Input, Metrics};
use super::blobs::Blobs;
use super::cancel::CancelToken;
use super::exec::{CallContext, Executor};
use super::model::{GenRequest, Model};
use super::store::Store;
use crate::View;
use crate::effects::Outcome;
use crate::emit::Encoder;
use serde::{Deserialize, Serialize};
use serde_json::{Value as J, json};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::io::Write;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
mod alloc;
mod batching;
mod config;
mod delivery;
mod durable;
mod model_gate;
mod pipe;
mod status;
use alloc::*;
pub use alloc::{pad_heaps, tune_allocator};
pub use config::{AgentSpec, Observed, Observer, RuntimeConfig};
use durable::{Durable, Journal, Ready};
use model_gate::*;
use pipe::Pipe;
use status::*;

/// The harness contract version a journal is written against (`spec/HARNESS.md`).
pub const CONTRACT_VERSION: u32 = 9;
static NEXT_INCARNATION: AtomicU64 = AtomicU64::new(1);

fn fresh_incarnation() -> u64 {
    (now_ms().max(0) as u64).saturating_mul(1_000_000) + NEXT_INCARNATION.fetch_add(1, Ordering::Relaxed)
}

/// One agent at the end of a run.
#[derive(Debug, Clone, Serialize)]
pub struct AgentReport {
    /// The agent's id.
    pub id: String,
    /// The render worker it ran on.
    pub worker: usize,
    /// The first turn whose journaled prompt this app no longer renders (restore under changed code).
    pub diverged_at: Option<u64>,
    /// Its counters (turns, tokens, appends, errors), as replay recomputes them.
    pub metrics: Metrics,
    /// Render work done in this process, including replay.
    pub render_stats: super::agent::RenderStats,
    /// The value it stopped with (`harness.stop`), if it stopped.
    pub stopped: Option<J>,
    /// Its final state.
    pub state: J,
    /// Inputs in its journal.
    pub journal_len: usize,
    /// Why it failed, if it did.
    pub error: Option<String>,
}

/// The end of a run.
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    /// Every agent, in the order they were added.
    pub agents: Vec<AgentReport>,
    /// Wall-clock seconds the run took.
    pub wall_s: f64,
    /// Every agent stopped.
    pub complete: bool,
    /// How the run ended.
    pub ended: Ended,
    /// Model calls whose prompt the server counted differently from the tokens laid out (should
    /// be zero; `model::Diagnostics::token_mismatches`).
    pub token_mismatches: u64,
    /// Model captures that could not be written.
    pub capture_errors: u64,
}

/// How a run ended. Every way but `Stopped` leaves outstanding work in the journals, which a
/// restore continues.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Ended {
    /// Every agent stopped.
    Stopped,
    /// A batch run (not kept running) had nothing left to do.
    Idle,
    /// The embedder closed it (`Control::close`).
    Closed,
    /// It ran for `max_wall`.
    MaxWall,
    /// A render worker died outside its instances' own code; its instances are reported failed.
    WorkerLost,
}

enum Msg {
    Input(String, InputKind),
    /// Copy these instances (on this worker) for an `inspect` call.
    Inspect(Arc<Gather>, Vec<String>),
    /// A journal commit: these instances' held effects may be released.
    Durable(Vec<String>),
    Shutdown,
}

/// An `inspect` call being answered: each worker adds its instances' copies, and the last one
/// answers the caller.
struct Gather {
    caller: String,
    incarnation: u64,
    worker: usize,
    task: u64,
    select: Vec<super::inspect::Sel>,
    since: J,
    text: bool,
    /// `"*"`: an instance that is not inspectable is left out (named, it is an error entry).
    star: bool,
    pending: AtomicUsize,
    parts: Mutex<BTreeMap<String, (J, Option<J>)>>,
}

/// Copy the instances `ids` of this worker into `g`; the last worker to add its copies answers.
fn inspect_here(shared: &Shared, slots: &BTreeMap<String, Slot>, g: &Gather, ids: &[String]) {
    // Only the instances asked for, copied out: not the whole map, on every worker, per request.
    let status: HashMap<String, Status> = {
        let all = shared.status.lock().unwrap();
        ids.iter()
            .filter_map(|id| all.get(id).map(|s| (id.clone(), *s)))
            .collect()
    };
    let mut parts = vec![];
    for id in ids {
        let entry = match slots.get(id) {
            None => (json!({"error": format!("no instance {id}")}), None),
            Some(slot) if !slot.spec.inspectable => {
                if g.star {
                    continue;
                } else {
                    (json!({"error": format!("{id} is not inspectable")}), None)
                }
            }
            Some(slot) => {
                let supplied = &g.since[id.as_str()];
                let previous = if supplied["incarnation"].as_u64() == Some(slot.incarnation) {
                    supplied
                } else {
                    &J::Null
                };
                let (mut part, mut cursor) = super::inspect::part(&slot.agent, &g.select, previous, g.text);
                cursor["incarnation"] = json!(slot.incarnation);
                part["status"] = serde_json::to_value(status.get(id).copied().unwrap_or_default()).unwrap_or_default();
                (part, Some(cursor))
            }
        };
        parts.push((id.clone(), entry));
    }
    g.parts.lock().unwrap().extend(parts);
    if g.pending.fetch_sub(1, Ordering::SeqCst) == 1 {
        answer_inspect(shared, g)
    }
}

fn answer_inspect(shared: &Shared, g: &Gather) {
    let parts = std::mem::take(&mut *g.parts.lock().unwrap());
    let since: serde_json::Map<String, J> = parts
        .iter()
        .filter_map(|(id, (_, c))| c.clone().map(|c| (id.clone(), c)))
        .collect();
    let value = if g.text {
        let mut text = String::new();
        for (id, (part, _)) in parts {
            let mut line = json!({"id": id});
            line.as_object_mut()
                .unwrap()
                .extend(part.as_object().cloned().unwrap_or_default());
            text.push_str(&line.to_string());
            text.push('\n');
        }
        json!({"since": since, "text": text})
    } else {
        json!({"since": since, "agents": parts.into_iter().map(|(id, (p, _))| (id, p)).collect::<serde_json::Map<_, _>>()})
    };
    shared.post(
        g.worker,
        Msg::Input(
            g.caller.clone(),
            InputKind::Completion {
                incarnation: g.incarnation,
                input: Box::new(InputKind::Result {
                    task: g.task,
                    outcome: Outcome::Ok(value),
                }),
            },
        ),
    );
}

/// An input before the worker stamps it with the agent's clock.
enum InputKind {
    Completion {
        incarnation: u64,
        input: Box<InputKind>,
    },
    Reply {
        task: u64,
        ids: Vec<u32>,
        cut_off: bool,
        stats: J,
        prompt: String,
        aborted: bool,
    },
    ModelError {
        task: u64,
        error: String,
        kind: super::model::FailureKind,
        retry_after_ms: Option<u64>,
    },
    Result {
        task: u64,
        outcome: Outcome,
    },
    /// An event; `ack`: a `send` or a post waiting until this is journaled; `id`: its delivery id.
    Event {
        name: String,
        payload: J,
        ack: Option<delivery::Ack>,
        id: Option<String>,
    },
    /// Start a child instance. Only recovery of its parent may restore the current incarnation.
    Mount {
        app: String,
        entry: String,
        props: J,
        model: Option<String>,
        machines: Vec<String>,
        restore: bool,
    },
    /// Stop an instance and its descendants (their journals stay).
    Unmount,
    Props {
        props: J,
    },
    Tick,
}

/// A fixed pool of threads running boxed jobs.
struct Pool {
    tx: Mutex<Sender<Box<dyn FnOnce() + Send>>>,
    /// Jobs running now.
    running: Arc<AtomicUsize>,
}

impl Pool {
    fn new(n: usize, name: &str) -> Pool {
        let (tx, rx) = channel::<Box<dyn FnOnce() + Send>>();
        let rx = Arc::new(Mutex::new(rx));
        let running = Arc::new(AtomicUsize::new(0));
        for i in 0..n.max(1) {
            let (rx, running) = (rx.clone(), running.clone());
            std::thread::Builder::new()
                .name(format!("{name}-{i}"))
                .spawn(move || {
                    loop {
                        let job = {
                            let r = rx.lock().unwrap();
                            r.recv()
                        };
                        // A job that panics loses its own work only: the thread serves the next one.
                        match job {
                            Ok(job) => {
                                running.fetch_add(1, Ordering::SeqCst);
                                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(job));
                                running.fetch_sub(1, Ordering::SeqCst);
                            }
                            Err(_) => return,
                        }
                    }
                })
                .expect("spawn pool thread");
        }
        Pool {
            tx: Mutex::new(tx),
            running,
        }
    }
    /// Wait until no job is running, at most `limit`.
    fn settle(&self, limit: Duration) {
        let until = Instant::now() + limit;
        while self.running.load(Ordering::SeqCst) > 0 && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    fn submit(&self, job: Box<dyn FnOnce() + Send>) {
        let _ = self.tx.lock().unwrap().send(job);
    }
}

#[derive(Default, Clone, Copy, Serialize)]
struct Status {
    stopped: bool,
    outstanding: usize,
    turns: u64,
    generated_tokens: u64,
    handler_errors: u64,
    last_activity_ms: i64,
    failed: bool,
    phase: &'static str,
    retrying: Option<(u32, &'static str)>,
}

/// A model binding: the tokenizer its prompts are rendered with and the server that runs them.
type ModelBinding = (Arc<Encoder>, Arc<dyn Model>);

struct Shared {
    cfg: RuntimeConfig,
    /// Workers still starting their agents (the last one reports how long the boot took).
    booting: std::sync::atomic::AtomicUsize,
    /// Model bindings by name; "default" is the one the runtime was created with.
    models: BTreeMap<String, ModelBinding>,
    machines: BTreeMap<String, Arc<dyn Executor>>,
    store: Store,
    /// The worker each instance lives on (children live with their parent).
    route: std::sync::RwLock<HashMap<String, usize>>,
    workers: Vec<Sender<Msg>>,
    /// Jobs submitted plus messages not yet processed.
    inflight: AtomicI64,
    status: Mutex<HashMap<String, Status>>,
    /// Live work, by (agent, incarnation, task): one record a call (`Live`), listed when the call
    /// is issued and removed when it is answered (`done`). "Nothing is live" is this map empty.
    cancels: Mutex<HashMap<(String, u64, u64), Live>>,
    model_pool: Pool,
    exec_pool: Pool,
    model_gate: ModelGate,
    /// The run is closing: no new model or machine call starts, and queued deliveries are left
    /// for a restore (`batching::unless_reissued`).
    closing: AtomicBool,
    /// When the runtime started (the boot's own time is reported from it).
    started: Instant,
    /// The journals' clock: logical time, which passes only while the run is running.
    clock: Clock,
    /// What this run notes about its model calls, and where it captures them.
    diagnostics: super::model::Diagnostics,
    /// Instances no worker holds: unmounted ones as they were when they left, and lost ones
    /// (`lost`). The run's report includes them.
    retired: Mutex<BTreeMap<String, AgentReport>>,
    /// Group commit of the journals.
    durable: Arc<Durable>,
    /// Restores in progress, at most `restore_concurrency`.
    restoring: Permits,
}

/// A counting semaphore.
#[derive(Default)]
struct Permits {
    free: Mutex<usize>,
    wake: std::sync::Condvar,
}
struct Permit<'a>(&'a Permits);
impl Permits {
    fn new(n: usize) -> Permits {
        Permits {
            free: Mutex::new(n.max(1)),
            wake: Default::default(),
        }
    }
    fn take(&self) -> Permit<'_> {
        let mut free = self.free.lock().unwrap();
        while *free == 0 {
            free = self.wake.wait(free).unwrap();
        }
        *free -= 1;
        Permit(self)
    }
}
impl Drop for Permit<'_> {
    fn drop(&mut self) {
        *self.0.free.lock().unwrap() += 1;
        self.0.wake.notify_one();
    }
}

/// The journals' clock. It stands still while the agents restore (it starts when the last worker
/// has booted) and from the moment the run begins closing: work finishing then sees that moment
/// and no later, so a timer neither fires nor loses time while the run boots or drains, and a
/// restore continues every timer with what it had left.
struct Clock(Mutex<ClockState>);
struct ClockState {
    at: i64,
    since: Option<Instant>,
    stopped: bool,
}
impl Clock {
    fn new(at: i64) -> Clock {
        Clock(Mutex::new(ClockState {
            at,
            since: None,
            stopped: false,
        }))
    }
    fn now(&self) -> i64 {
        let c = self.0.lock().unwrap();
        c.at + c.since.map_or(0, |t| t.elapsed().as_millis() as i64)
    }
    /// Let time pass (once the agents are running), unless the run is already closing.
    fn start(&self) {
        let mut c = self.0.lock().unwrap();
        if !c.stopped && c.since.is_none() {
            c.since = Some(Instant::now());
        }
    }
    /// Stop it for good at this moment.
    fn stop(&self) {
        let mut c = self.0.lock().unwrap();
        if let Some(t) = c.since.take() {
            c.at += t.elapsed().as_millis() as i64;
        }
        c.stopped = true;
    }
}

impl Shared {
    /// `j` with blob references replaced by their text (reports and snapshots leave the runtime).
    fn expand(&self, j: &J) -> J {
        self.store.blobs().map_or_else(
            || j.clone(),
            |b| b.expand(j).unwrap_or_else(|e| json!({"blob_error": e})),
        )
    }
    /// A state for `snapshot.json`: references to blobs over 64 KiB stay references (the text is
    /// in `blobs/`), so writing a snapshot never copies an agent's large outputs.
    fn expand_snapshot(&self, j: &J) -> J {
        self.store.blobs().map_or_else(
            || j.clone(),
            |b| b.expand_up_to(j, 64 << 10).unwrap_or_else(|e| json!({"blob_error": e})),
        )
    }
    fn clock_ms(&self) -> i64 {
        self.clock.now()
    }
    /// Begin closing: the clock stops at this moment, no new call starts, and calls still queued
    /// for a model are left outstanding for a restore.
    fn begin_closing(&self) {
        self.clock.stop();
        self.closing.store(true, Ordering::SeqCst);
        suspend_queued_models(self);
    }
    /// List a machine call as live; cancelling its token stops it.
    fn token(&self, agent: &str, incarnation: u64, task: u64) -> CancelToken {
        let token = CancelToken::new();
        self.cancels.lock().unwrap().insert(
            (agent.to_string(), incarnation, task),
            Live {
                token: token.clone(),
                abort: None,
            },
        );
        token
    }
    /// List a model call as live: its token, and the flag the app's abort signal sets.
    fn model_token(&self, agent: &str, incarnation: u64, task: u64) -> (CancelToken, Arc<AtomicBool>) {
        let (token, abort) = (CancelToken::new(), Arc::new(AtomicBool::new(false)));
        self.cancels.lock().unwrap().insert(
            (agent.to_string(), incarnation, task),
            Live {
                token: token.clone(),
                abort: Some(abort.clone()),
            },
        );
        (token, abort)
    }
    /// The model gate's lane for `rank` of `binding`: bindings take consecutive ranges of lanes,
    /// in name order, each as wide as its ranks.
    fn lane_of(&self, binding: &str, rank: usize) -> usize {
        let mut base = 0;
        for (name, (_, model)) in &self.models {
            let ranks = model.ranks().max(1);
            if name == binding {
                return base + rank.min(ranks - 1);
            }
            base += ranks;
        }
        0
    }
    fn done(&self, agent: &str, incarnation: u64, task: u64) {
        self.cancels
            .lock()
            .unwrap()
            .remove(&(agent.to_string(), incarnation, task));
    }
    fn post(&self, worker: usize, msg: Msg) {
        self.inflight.fetch_add(1, Ordering::SeqCst);
        self.answer(worker, msg);
    }
    /// Send a job's reply: the job's unit in flight becomes the message's, or is released if the
    /// worker is gone (nothing would consume it).
    fn answer(&self, worker: usize, msg: Msg) {
        if self.workers[worker].send(msg).is_err() {
            self.inflight.fetch_sub(1, Ordering::SeqCst);
        }
    }
    fn post_to(&self, agent: &str, kind: InputKind) -> bool {
        let w = self.route.read().unwrap().get(agent).copied();
        match w {
            Some(w) => {
                self.post(w, Msg::Input(agent.into(), kind));
                true
            }
            None => false,
        }
    }
}

/// The multi-agent runtime: agents on render workers, model and machine calls on pools, a
/// journal per agent. Configure it, add agents, then `run`.
pub struct Runtime {
    cfg: RuntimeConfig,
    models: BTreeMap<String, ModelBinding>,
    machines: BTreeMap<String, Arc<dyn Executor>>,
    specs: Vec<AgentSpec>,
    /// The agents `restore` found journals for, with each journal's last clock (journals are
    /// streamed from disk when the agents start, never held here).
    journals: HashMap<String, i64>,
    /// The journals' committed cut, recovered by `restore` (`run` recovers it otherwise).
    cut: Option<durable::Cut>,
    /// The lock on the run directory, taken by `restore` (`run` takes it otherwise).
    owner: Option<durable::Owner>,
    /// The embedder's handle and the operations it sends (`control`); `run` drains the receiver.
    control: Option<(Control, Receiver<Op>)>,
}

/// An embedder's handle on a run, as a browser's embedder holds a page: post events into it,
/// read its status, set its model limit, close it. Take it with `Runtime::control` before `run`;
/// it is cheap to clone and usable from any thread, before the run starts and while it runs.
#[derive(Clone)]
pub struct Control {
    ops: Sender<Op>,
    status: Arc<Mutex<J>>,
}

enum Op {
    Post {
        target: String,
        event: String,
        payload: J,
        key: Option<String>,
        reply: Sender<Result<(), String>>,
    },
    Close(Duration),
    ModelLimit(usize),
}

/// A post's answer: `wait` returns once the event's journal line is committed (`Ok`), or with why
/// it was not delivered. Dropping it does not withdraw the post.
pub struct Delivery(Receiver<Result<(), String>>);

impl Delivery {
    /// Wait for the answer.
    pub fn wait(self) -> Result<(), String> {
        self.0
            .recv()
            .unwrap_or_else(|_| Err("the run ended before the post was delivered".into()))
    }
}

impl Control {
    /// Deliver `payload` to instance `target` as the event `event` (`useEvent(event)`), as
    /// `postMessage` does. With a `key`, a post the instance has already taken (posted again after
    /// a lost answer, or after a restore) is journaled but not taken a second time.
    pub fn post(&self, target: &str, event: &str, payload: J, key: Option<&str>) -> Delivery {
        let (reply, answer) = channel();
        let op = Op::Post {
            target: target.into(),
            event: event.into(),
            payload,
            key: key.map(str::to_owned),
            reply: reply.clone(),
        };
        if self.ops.send(op).is_err() {
            let _ = reply.send(Err("the run has ended".into()));
        }
        Delivery(answer)
    }

    /// Close the run: the clock stops and no new call starts; calls still running get `timeout`
    /// to finish, then are cancelled (nothing is lost: they stay outstanding in the journals, and a
    /// restore issues them again). `run` returns once everything is committed. A second close may
    /// shorten the time left; it never lengthens it.
    pub fn close(&self, timeout: Duration) {
        let _ = self.ops.send(Op::Close(timeout));
    }

    /// How many model calls may run at once (0 holds every model call while the rest of the run
    /// goes on).
    pub fn set_model_limit(&self, limit: usize) {
        let _ = self.ops.send(Op::ModelLimit(limit));
    }

    /// The run's status: `state` (`starting`, `running`, `closing`, `closed`), `clock_ms`, each
    /// agent's state, the model bindings' windows and the model queue. Refreshed about every
    /// 100 ms while the run runs.
    pub fn status(&self) -> J {
        self.status.lock().unwrap().clone()
    }
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn stable_hash(s: &str) -> u64 {
    s.bytes().fold(1469598103934665603u64, |h, b| {
        (h ^ b as u64).wrapping_mul(1099511628211)
    })
}

impl Runtime {
    /// A runtime for `cfg` whose agents render with `enc` and call `model` by default.
    pub fn new(cfg: RuntimeConfig, enc: Arc<Encoder>, model: Arc<dyn Model>) -> Self {
        Runtime {
            cfg,
            models: BTreeMap::from([("default".to_string(), (enc, model))]),
            machines: BTreeMap::new(),
            specs: vec![],
            journals: HashMap::new(),
            cut: None,
            owner: None,
            control: None,
        }
    }

    /// The encoder of the model binding `name` (`default`: the deployment's model), if bound.
    pub fn encoder(&self, name: &str) -> Option<Arc<Encoder>> {
        self.models.get(name).map(|(enc, _)| enc.clone())
    }

    /// Watch what every call produces as it arrives (`RuntimeConfig.observer`), set before `run`.
    pub fn observe(&mut self, observer: Observer) -> &mut Self {
        self.cfg.observer = Some(observer);
        self.cfg.observe_deltas = true;
        self
    }

    /// Watch the instances' trace records and command output without changing how model calls
    /// are made: a console that reports, where `observe` is a live view that needs each reply as
    /// it is generated and so streams every call.
    pub fn observe_traces(&mut self, observer: Observer) -> &mut Self {
        self.cfg.observer = Some(observer);
        self.cfg.observe_deltas = false;
        self
    }

    /// The embedder's handle on this run (`Control`). Every call returns a handle on the same run.
    pub fn control(&mut self) -> Control {
        self.control
            .get_or_insert_with(|| {
                let (ops, rx) = channel();
                let status = Arc::new(Mutex::new(json!({"state": "starting"})));
                (Control { ops, status }, rx)
            })
            .0
            .clone()
    }

    /// Bind another model by name: an instance (`AgentSpec.model`, `<Mount model>`) or a render
    /// names the model it runs on; its prompts are rendered with `enc`.
    pub fn bind_model(&mut self, name: &str, enc: Arc<Encoder>, model: Arc<dyn Model>) -> &mut Self {
        self.models.insert(name.into(), (enc, model));
        self
    }

    /// Serve primitive `name` with `executor` (`exec`, `summarize`, `http`, ...).
    /// Bind a machine to this deployment under `role`: agents reach it as `harness.machine` (or
    /// `harness.machine.get(role)`), unless their spec binds other roles.
    pub fn bind_machine(&mut self, role: &str, machine: Arc<dyn Executor>) -> &mut Self {
        self.machines.insert(role.into(), machine);
        self
    }

    /// Add a root agent; ids are unique.
    pub fn add_agent(&mut self, spec: AgentSpec) -> Result<&mut Self, String> {
        if self.specs.iter().any(|s| s.id == spec.id) {
            return Err(format!("duplicate agent {}", spec.id));
        }
        if !super::agent::valid_instance_name(&spec.id) {
            return Err(format!("bad agent id {:?}", spec.id));
        }
        self.specs.push(spec);
        Ok(self)
    }

    /// The agents to run (after `add_agent`, or as read back by `restore`).
    pub fn specs(&self) -> &[AgentSpec] {
        &self.specs
    }

    /// Membership supplied by the controller on restart. Existing journals retain their
    /// original app props for deterministic replay; only fresh identities use new props.
    pub fn membership(&mut self, wanted: Vec<AgentSpec>) -> Result<(), String> {
        let ids: BTreeSet<_> = wanted.iter().map(|s| s.id.clone()).collect();
        self.specs.retain(|s| ids.contains(&s.id));
        self.journals.retain(|id, _| ids.contains(id));
        for spec in wanted {
            if let Some(old) = self.specs.iter_mut().find(|old| old.id == spec.id) {
                old.machines = spec.machines;
                old.capabilities = spec.capabilities;
                old.inspect = spec.inspect;
                old.inspectable = spec.inspectable;
                old.rank = spec.rank;
            } else {
                self.add_agent(spec)?;
            }
        }
        Ok(())
    }

    /// Continue the agents journaled under `cfg.dir`: each is rebuilt from its journal and its
    /// outstanding work is submitted again. Secret env values come from `cfg.env` again.
    pub fn restore(cfg: RuntimeConfig, enc: Arc<Encoder>, model: Arc<dyn Model>) -> Result<Self, String> {
        let dir = cfg.dir.clone().ok_or("restore needs a run directory")?;
        let mut rt = Runtime::new(cfg, enc, model);
        rt.owner = Some(durable::own(&dir)?);
        // A crash leaves the journals as the last commit made them, before anything reads them.
        rt.cut = Some(durable::recover(&dir)?);
        let mut entries: Vec<_> = std::fs::read_dir(&dir)
            .map_err(|e| format!("{}: {e}", dir.display()))?
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.join("agent.json").exists())
            .collect();
        entries.sort();
        for path in entries {
            // One agent's damage is its own: an unreadable spec leaves it not restored (its files as
            // they are), and an unreadable journal fails its own restore.
            let spec = std::fs::read(path.join("agent.json"))
                .map_err(|e| e.to_string())
                .and_then(|b| serde_json::from_slice::<AgentSpec>(&b).map_err(|e| e.to_string()));
            let spec = match spec {
                Ok(spec) => spec,
                Err(e) => {
                    eprintln!("{}: not restored: agent.json: {e}", path.display());
                    continue;
                }
            };
            if !super::agent::valid_instance_name(&spec.id)
                || path.file_name().is_none_or(|name| name != spec.id.as_str())
            {
                return Err(format!(
                    "{}: saved agent id does not match its directory",
                    path.display()
                ));
            }
            let file = path.join("journal.jsonl");
            let last_clock = journal_last_clock(&file).unwrap_or_else(|e| {
                eprintln!("{}: journal clock unreadable: {e}", spec.id);
                None
            });
            if file.exists() {
                rt.journals.insert(spec.id.clone(), last_clock.unwrap_or(0));
            }
            rt.specs.push(spec);
        }
        Ok(rt)
    }

    /// Run every agent until all have stopped, nothing can make progress, or `max_wall` passes.
    pub fn run(mut self) -> Result<Report, String> {
        if !(1..=super::agent::MAX_INPUT_BATCH).contains(&self.cfg.input_batch) {
            return Err("input_batch must be an integer from 1 to 64".into());
        }
        for (name, (_, model)) in &self.models {
            model.validate().map_err(|e| format!("model binding {name:?}: {e}"))?;
        }
        // Each binding's window, as validating established it: logged once, and in the status.
        let windows: serde_json::Map<String, J> = self
            .models
            .iter()
            .map(|(name, (_, model))| {
                let (context, source) = (model.context(), model.window_source());
                if let (Some(n), Some(s)) = (context, &source) {
                    eprintln!(
                        "{}model binding {name:?}: window {n} tokens ({s})",
                        if s.contains("reported none") { "warning: " } else { "" }
                    );
                }
                (name.clone(), json!({"context": context, "source": source}))
            })
            .collect();
        let started = Instant::now();
        let saved_clock = self
            .cfg
            .dir
            .as_ref()
            .and_then(|d| std::fs::read_to_string(d.join("clock.json")).ok())
            .and_then(|s| serde_json::from_str::<J>(&s).ok())
            .and_then(|v| v["clock_ms"].as_i64());
        let journal_clock = self.journals.values().copied().max();
        let clock_base = saved_clock
            .into_iter()
            .chain(journal_clock)
            .max()
            .unwrap_or_else(now_ms);
        let dedicated = self.specs.iter().filter(|spec| spec.dedicated_worker).count();
        let shared_workers = self.cfg.workers.max(1).min(self.specs.len() - dedicated);
        let n_workers = (shared_workers + dedicated).max(1);
        let mut route = HashMap::new();
        let mut per_worker: Vec<Vec<AgentSpec>> = vec![vec![]; n_workers];
        let (mut shared_next, mut dedicated_next) = (0, shared_workers);
        for spec in &self.specs {
            let worker = if spec.dedicated_worker {
                let worker = dedicated_next;
                dedicated_next += 1;
                worker
            } else {
                let worker = shared_next % shared_workers;
                shared_next += 1;
                worker
            };
            route.insert(spec.id.clone(), worker);
            per_worker[worker].push(spec.clone());
        }
        let (txs, rxs): (Vec<Sender<Msg>>, Vec<Receiver<Msg>>) = (0..n_workers).map(|_| channel()).unzip();
        // Blob and store files are synced by the journal commits, with the lines that refer to them.
        let unsynced = Arc::new(super::blobs::Unsynced::default());
        if let Some(dir) = &self.cfg.dir {
            // A journal holds everything an agent was told: prompts, tool output, and a served
            // request's headers. A directory this run makes is its user's alone (an existing one
            // keeps the mode its owner gave it).
            if !dir.exists() {
                use std::os::unix::fs::DirBuilderExt;
                std::fs::DirBuilder::new()
                    .recursive(true)
                    .mode(0o700)
                    .create(dir)
                    .map_err(|e| e.to_string())?;
            }
        }
        let blobs = match &self.cfg.dir {
            Some(d) => Some(Arc::new(
                Blobs::new(d.join("blobs"), self.cfg.spill_bytes)?.deferring(unsynced.clone()),
            )),
            None => None,
        };
        let store = Store::with_blobs(self.cfg.dir.clone(), blobs).deferring();
        // Held until the run returns, after its last commit (`committing` drops first).
        let _owner = match (&self.cfg.dir, self.owner.take()) {
            (Some(dir), None) => Some(durable::own(dir)?),
            (_, owner) => owner,
        };
        let cut = match (&self.cfg.dir, &self.cut) {
            (Some(_), Some(cut)) => Some(cut.clone()),
            (Some(dir), None) => Some(durable::recover(dir)?),
            _ => None,
        };
        let durable = Arc::new(Durable::new());
        let shared = Arc::new(Shared {
            model_pool: Pool::new(self.cfg.model_threads, "model"),
            exec_pool: Pool::new(self.cfg.exec_threads, "exec"),
            cfg: self.cfg.clone(),
            models: self.models.clone(),
            machines: self.machines.clone(),
            store,
            route: std::sync::RwLock::new(route),
            workers: txs,
            inflight: AtomicI64::new(0),
            status: Mutex::new(HashMap::new()),
            cancels: Mutex::new(HashMap::new()),
            // One lane a rank of every binding, each binding's lanes after the previous one's
            // (`lane_of`): two bindings never share a lane.
            model_gate: ModelGate::new(
                self.models.values().map(|(_, m)| m.ranks().max(1)).sum(),
                self.cfg.model_inflight.unwrap_or(self.cfg.model_threads),
            ),
            closing: AtomicBool::new(false),
            started,
            clock: Clock::new(clock_base),
            diagnostics: super::model::Diagnostics::new(self.cfg.capture_dir.clone()),
            retired: Default::default(),
            booting: Default::default(),
            durable: durable.clone(),
            restoring: Permits::new(
                self.cfg
                    .restore_concurrency
                    .unwrap_or_else(|| std::thread::available_parallelism().map_or(8, |n| n.get())),
            ),
        });
        if let (Some(dir), Some(cut)) = (&self.cfg.dir, cut) {
            durable.start(dir, cut, unsynced.clone(), Arc::downgrade(&shared))?;
        }
        // However the run ends, what was written is committed and the barrier thread stops.
        struct Committing(Arc<Durable>, bool);
        impl Drop for Committing {
            fn drop(&mut self) {
                let _ = self.0.finish(self.1);
            }
        }
        let mut committing = Committing(durable.clone(), false);
        shared.booting.store(n_workers, Ordering::SeqCst);
        let mut handles = vec![];
        for (w, (rx, specs)) in rxs.into_iter().zip(per_worker).enumerate() {
            let shared = shared.clone();
            let restored: HashSet<String> = specs
                .iter()
                .filter(|s| self.journals.contains_key(&s.id))
                .map(|s| s.id.clone())
                .collect();
            {
                let mut status = shared.status.lock().unwrap();
                for s in &specs {
                    status.insert(
                        s.id.clone(),
                        Status {
                            stopped: false,
                            outstanding: 1,
                            ..Status::default()
                        },
                    );
                }
            }
            handles.push(
                std::thread::Builder::new()
                    .name(format!("render-{w}"))
                    .spawn(move || worker(w, shared, specs, restored, rx))
                    .map_err(|e| e.to_string())?,
            );
        }
        // Run until every agent stopped, a batch has nothing left to do, `max_wall` passes, a
        // worker is lost, or the embedder closes the run. Every end goes through closing: the
        // clock stops, no new call starts, and calls still running get the close's time to finish.
        let (control, ops) = match self.control.take() {
            Some((control, ops)) => (Some(control), Some(ops)),
            None => (None, None),
        };
        let mut last_status = Instant::now() - Duration::from_secs(1);
        let mut last_trim = Instant::now();
        let status_writer = StatusWriter::start(self.cfg.dir.clone(), durable.clone());
        // Closing: when running calls are cancelled, and why the run ends. A later close may bring
        // that moment forward, never put it back.
        let mut closing: Option<(Instant, Ended)> = None;
        let close = |closing: &mut Option<(Instant, Ended)>, timeout: Duration, why: Ended| {
            let deadline = Instant::now() + timeout;
            match closing {
                Some((at, _)) => *at = (*at).min(deadline),
                None => {
                    shared.begin_closing();
                    *closing = Some((deadline, why));
                }
            }
        };
        let ended = loop {
            std::thread::sleep(Duration::from_millis(5));
            while let Some(op) = ops.as_ref().and_then(|rx| rx.try_recv().ok()) {
                match op {
                    Op::Post { reply, .. } if closing.is_some() => {
                        let _ = reply.send(Err(delivery::CLOSING.into()));
                    }
                    Op::Post {
                        target,
                        event,
                        payload,
                        key,
                        reply,
                    } => delivery::post(&shared, &target, event, payload, key, reply),
                    Op::Close(timeout) => close(&mut closing, timeout, Ended::Closed),
                    Op::ModelLimit(limit) => {
                        if shared.model_gate.set_limit(limit) {
                            pump_model(&shared);
                        }
                    }
                }
            }
            if last_trim.elapsed() >= Duration::from_secs(10) {
                last_trim = Instant::now();
                trim_heap();
            }
            let booted = shared.booting.load(Ordering::SeqCst) == 0;
            if last_status.elapsed() >= Duration::from_millis(100) {
                last_status = Instant::now();
                let state = match (&closing, booted) {
                    (Some(_), _) => "closing",
                    (None, false) => "starting",
                    (None, true) => "running",
                };
                let value = status_value(&shared, state, &windows);
                status_writer.put(json!({"clock_ms": value["clock_ms"]}));
                if let Some(control) = &control {
                    *control.status.lock().unwrap() = value;
                }
            }
            if let Some((deadline, why)) = closing {
                if Instant::now() >= deadline {
                    // Taken out first: a cancel runs callbacks, which must not run under the lock
                    // every call takes.
                    let tokens: Vec<CancelToken> = shared
                        .cancels
                        .lock()
                        .unwrap()
                        .values()
                        .map(|l| l.token.clone())
                        .collect();
                    for token in tokens {
                        token.cancel_by_host("the run is closing");
                    }
                }
                // A lost worker's messages are never consumed: the drain cannot finish, and need not.
                if (shared.inflight.load(Ordering::SeqCst) == 0 && shared.cancels.lock().unwrap().is_empty())
                    || handles.iter().any(|h| h.is_finished())
                {
                    break if handles.iter().any(|h| h.is_finished()) {
                        Ended::WorkerLost
                    } else {
                        why
                    };
                }
                continue;
            }
            let status = shared.status.lock().unwrap();
            let all_stopped = status.values().all(|s| s.stopped);
            // Idle only after every worker has started its agents (before that the status is partial).
            let idle = booted
                && shared.inflight.load(Ordering::SeqCst) == 0
                && status.values().all(|s| s.stopped || s.outstanding == 0);
            drop(status);
            if all_stopped {
                break Ended::Stopped;
            }
            if idle && !self.cfg.serve {
                break Ended::Idle;
            }
            // A worker only ends when told to: one that ended died outside its instances' own
            // code, and the work it owed is lost with it. Its agents are reported failed, the rest
            // can be restored.
            if handles.iter().any(|h| h.is_finished()) {
                close(&mut closing, Duration::ZERO, Ended::WorkerLost);
            } else if self.cfg.max_wall.is_some_and(|m| started.elapsed() > m) {
                close(&mut closing, Duration::ZERO, Ended::MaxWall);
            }
        };
        // Whatever is still running is stopped (commands are killed, requests closed). No periodic
        // clock lands after this (the final one below must be the last).
        status_writer.finish();
        shared.begin_closing();
        for (_, live) in shared.cancels.lock().unwrap().drain() {
            live.token.cancel_by_host("the run is closing");
        }
        // The commands just cancelled are being stopped on the machine threads (SIGTERM, then
        // SIGKILL after five seconds). The run does not return, and the process does not exit,
        // while one is still being stopped: its children would be left running.
        shared.exec_pool.settle(Duration::from_secs(7));
        for tx in &shared.workers {
            let _ = tx.send(Msg::Shutdown);
        }
        let mut agents = vec![];
        let joined: Vec<_> = handles.into_iter().map(|h| h.join()).collect();
        for (w, joined) in joined.into_iter().enumerate() {
            match joined {
                Ok(reports) => agents.extend(reports?),
                // Its instances are failed (a restore rebuilds them from their journals).
                Err(panic) => {
                    let error = format!("render worker {w} {}", panicked(panic));
                    let held: Vec<String> = shared
                        .route
                        .read()
                        .unwrap()
                        .iter()
                        .filter(|(_, at)| **at == w)
                        .map(|(id, _)| id.clone())
                        .collect();
                    for id in held
                        .into_iter()
                        .filter(|id| !shared.retired.lock().unwrap().contains_key(id))
                    {
                        lost(&shared, w, &id, error.clone());
                    }
                }
            }
        }
        agents.extend(std::mem::take(&mut *shared.retired.lock().unwrap()).into_values());
        // Every worker has stopped: no line is written after this commit, which closes the log.
        committing.1 = true;
        drop(committing);
        if let Some(error) = durable.failed() {
            return Err(error);
        }
        if let Some(dir) = &self.cfg.dir {
            atomic_value(&dir.join("clock.json"), &json!({"clock_ms": shared.clock_ms()}))?;
        }
        if let Some(control) = &control {
            let mut status = status_value(&shared, "closed", &windows);
            status["ended"] = json!(ended);
            *control.status.lock().unwrap() = status;
        }
        agents.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(Report {
            agents,
            wall_s: started.elapsed().as_secs_f64(),
            complete: ended == Ended::Stopped,
            ended,
            token_mismatches: shared.diagnostics.token_mismatches(),
            capture_errors: shared.diagnostics.capture_errors(),
        })
    }
}

/// The run's status for `Control::status`: its state, its clock, each agent, the model bindings'
/// windows and the model queue.
fn status_value(shared: &Shared, state: &str, windows: &serde_json::Map<String, J>) -> J {
    // Copied out under the lock (every worker takes it after every input); the JSON is built after
    // it is released. A model call retrying a failure is shown as such; the agent is still healthy
    // (an outage is retried for as long as it lasts).
    let statuses: Vec<(String, Status)> = shared
        .status
        .lock()
        .unwrap()
        .iter()
        .map(|(id, s)| (id.clone(), *s))
        .collect();
    let agents: Vec<J> = statuses.iter().map(|(id,s)| json!({"id":id,"state":if s.failed {"failed"} else if s.stopped {"stopped"} else if s.last_activity_ms == 0 {"starting"} else if s.retrying.is_some() {"retrying"} else {s.phase}, "healthy":!s.failed && !s.stopped && s.last_activity_ms > 0,
        "retrying":s.retrying.map(|(failures, error)| json!({"failures":failures,"error":error})),"turns":s.turns,"generated_tokens":s.generated_tokens,"handler_errors":s.handler_errors,"last_activity_ms":s.last_activity_ms,"outstanding":s.outstanding})).collect();
    let (model_active, model_queued) = shared.model_gate.counts();
    json!({"state": state, "clock_ms": shared.clock_ms(), "wall_ms": now_ms(), "agents": agents, "models": windows,
        "model_active": model_active, "model_queued": model_queued, "capture_errors": shared.diagnostics.capture_errors()})
}

pub(super) fn atomic_value(path: &std::path::Path, value: &J) -> Result<(), String> {
    atomic_bytes(path, value.to_string().as_bytes())
}
pub(super) fn atomic_bytes(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let temp = path.with_extension("tmp");
    let mut file = std::fs::File::create(&temp).map_err(|e| e.to_string())?;
    file.write_all(bytes).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    std::fs::rename(temp, path).map_err(|e| e.to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::File::open(parent)
            .and_then(|file| file.sync_all())
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Replace secret values with `[secret:KEY]` in a serialized trace or config line.
/// The secrets' values replaced in every string and key of `value`, before it is serialized: a
/// value is matched as itself, so a secret holding a newline, a quote or a backslash (a PEM key,
/// a service account's JSON) is found too, and no replacement can break the JSON around it.
fn redact(mut value: J, secrets: &[(String, String)]) -> J {
    if secrets.is_empty() {
        return value;
    }
    let clean = |text: String| {
        secrets.iter().fold(text, |t, (k, v)| {
            if t.contains(v.as_str()) {
                t.replace(v.as_str(), &format!("[secret:{k}]"))
            } else {
                t
            }
        })
    };
    let mut work = vec![&mut value];
    while let Some(value) = work.pop() {
        match value {
            J::String(text) => *text = clean(std::mem::take(text)),
            J::Array(items) => work.extend(items.iter_mut()),
            J::Object(items) => {
                *items = std::mem::take(items)
                    .into_iter()
                    .map(|(key, value)| (clean(key), value))
                    .collect();
                work.extend(items.values_mut());
            }
            _ => {}
        }
    }
    value
}

/// The shortest secret the runtime redacts: a shorter one would also replace ordinary text.
const MIN_SECRET: usize = 4;

struct Slot {
    incarnation: u64,
    agent: Agent,
    spec: AgentSpec,
    app: PathBuf,
    secrets: Vec<(String, String)>,
    journal: Option<Journal>,
    checkpoint: Option<super::checkpoint::Checkpoint>,
    trace: Option<std::fs::File>,
    error: Option<String>,
    /// The commit epoch of this instance's latest input: its effects wait for it.
    epoch: u64,
    /// Effects waiting for their epoch's commit, in the order they were issued.
    held: VecDeque<(u64, Held)>,
}

/// An effect that leaves the process, held until the input that caused it is committed.
// A held command is moved as it was sent: a slot holds a few, so its size is not boxed away.
#[allow(clippy::large_enum_variant)]
enum Held {
    Command(Command),
    /// An embedder's post, answered once its event is committed.
    Answer(delivery::Ack),
}

/// A worker's loaded apps, by (file, entry, model binding): a view is bound to its model's tokenizer.
struct Views {
    loaded: HashMap<(PathBuf, String, String), Rc<View>>,
}

impl RuntimeConfig {
    /// Where apps may import from besides their own directory (see `library`).
    pub fn library_roots(&self) -> Vec<PathBuf> {
        if let Some(l) = &self.library {
            return vec![l.clone()];
        }
        let view = self.view.canonicalize().unwrap_or_else(|_| self.view.clone());
        view.ancestors()
            .skip(1)
            .find(|d| d.join("index.json").is_file())
            .map(|d| vec![d.to_path_buf()])
            .unwrap_or_default()
    }
}

impl Views {
    fn get(
        &mut self,
        shared: &Shared,
        app: &std::path::Path,
        entry: &str,
        model: Option<&str>,
    ) -> Result<(Rc<View>, Arc<Encoder>), String> {
        // An instance without a model still renders (its mounts, its handlers): with the default tokenizer.
        let name = model.unwrap_or("default");
        let enc = shared
            .models
            .get(name)
            .ok_or_else(|| format!("no model binding {name:?}"))?
            .0
            .clone();
        let key = (app.to_path_buf(), entry.to_string(), name.to_string());
        if !self.loaded.contains_key(&key) {
            // A view is loaded for one binding: its template is the one the app renders for.
            let templates: Vec<String> = shared.models[name].1.template().map(|t| t.name).into_iter().collect();
            let view = contained(|| View::load_for(app, entry, &enc, &shared.cfg.library_roots(), &templates))?;
            self.loaded.insert(key.clone(), Rc::new(view));
        }
        Ok((self.loaded[&key].clone(), enc))
    }
}

/// Thread-time spent in each phase of restoring agents (printed with the boot line under
/// `AFFORDANCE_RESTORE_TIMING`): a diagnostic for restore cost.
static PHASES: [std::sync::atomic::AtomicU64; 8] = [const { std::sync::atomic::AtomicU64::new(0) }; 8];
const PHASE_NAMES: [&str; 8] = [
    "store_load",
    "image_load",
    "image_restore",
    "boundary_pass",
    "validate",
    "suffix",
    "full_replay",
    "start_rest",
];
fn timed<R>(phase: usize, f: impl FnOnce() -> R) -> R {
    let t = Instant::now();
    let r = f();
    PHASES[phase].fetch_add(t.elapsed().as_micros() as u64, Ordering::Relaxed);
    r
}
pub(crate) fn restore_timing() -> String {
    PHASE_NAMES
        .iter()
        .zip(&PHASES)
        .map(|(n, p)| format!("{n}={:.1}s", p.load(Ordering::Relaxed) as f64 / 1e6))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Run an instance's own code (loading its app, starting it, replaying its journal, handling an
/// input) with a panic in it as its error. Like a crashed tab, the instance fails and nothing
/// else does: an error stops it for good, so the state a panic left half-updated is never run
/// again or saved as an image, and its worker goes on serving the other instances.
fn contained<T>(f: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).unwrap_or_else(|panic| Err(panicked(panic)))
}

fn panicked(panic: Box<dyn std::any::Any + Send>) -> String {
    let text = panic
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| panic.downcast_ref::<String>().cloned());
    format!("panicked: {}", text.as_deref().unwrap_or("(no message)"))
}

/// An instance whose agent never was (its start failed) or is gone (its worker died): failed,
/// its report the error.
fn lost(shared: &Shared, w: usize, id: &str, error: String) {
    eprintln!("{id}: {error}");
    shared.status.lock().unwrap().insert(
        id.to_string(),
        Status {
            stopped: true,
            failed: true,
            last_activity_ms: now_ms(),
            ..Status::default()
        },
    );
    shared.retired.lock().unwrap().insert(
        id.to_string(),
        AgentReport {
            id: id.to_string(),
            worker: w,
            diverged_at: None,
            metrics: Metrics::default(),
            render_stats: Default::default(),
            stopped: None,
            state: J::Null,
            journal_len: 0,
            error: Some(error),
        },
    );
}

/// What a recovery image was computed under: what the agent computes and how its journal
/// replays. Only that: an operational setting (workers, thread pools, `model_inflight`,
/// `restore_concurrency`, timeouts, machines' commands, where the app is installed) changes
/// none of it, and an image kept across a change of one saves its agent a full replay.
fn checkpoint_identity(shared: &Shared, cfg: &AgentConfig, view: &View, enc: &Encoder) -> String {
    // Which model answers and how prompts are laid out (`replay_identity`), not how it is
    // reached: moving a server or changing a timeout keeps images.
    let model = cfg
        .model
        .as_ref()
        .and_then(|name| shared.models.get(name))
        .and_then(|(_, m)| m.replay_identity());
    // The app is its files' contents and their paths relative to the root view (`source_hash`),
    // and which of them is the root: its name, not its absolute path. A deploy that installs
    // the same files in a new release directory moved every image's root path, and each agent
    // of four 256-agent fleets replayed its whole journal on restore (347 s).
    let root = std::path::Path::new(&*view.prog.root)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned());
    super::model_config::fingerprint(json!({"contract":CONTRACT_VERSION,"app":view.source_hash,"root":root,
        "entry":&*view.prog.entry,"tokenizer":enc.identity,"model":model,"config":cfg,
        "strict":shared.cfg.strict_replay,"spill_bytes":shared.cfg.spill_bytes}))
}

/// Rebuild an agent from its recovery image and the journal after it, or from the whole journal.
/// Returns where the journal stands, and the image's input count (`saved`) if one was used.
fn restore_instance(
    shared: &Shared,
    cfg: AgentConfig,
    view: &View,
    enc: &Encoder,
    dir: &std::path::Path,
    allow_cache: bool,
) -> Result<(Agent, Vec<Command>, super::checkpoint::Seed), String> {
    let key = checkpoint_identity(shared, &cfg, view, enc);
    let setup = |a: &mut Agent| {
        if let Some(b) = shared.store.blobs() {
            a.set_blobs(b.clone());
        }
        a.strict_replay = shared.cfg.strict_replay;
    };
    let mut cached = None;
    if allow_cache {
        let attempt = timed(1, || {
            super::checkpoint::load(&dir.join(&cfg.id), &key, shared.store.blobs().map(AsRef::as_ref))
        })
        .and_then(|saved| {
            saved
                .map(|saved| {
                    let inputs = saved.image.inputs();
                    timed(2, || saved.image.restore(cfg.clone(), view, setup))
                        .map(|a| (a, saved.offset, inputs, saved.refs))
                })
                .transpose()
        });
        match attempt {
            Ok(image) => cached = image,
            Err(e) => eprintln!("{}: checkpoint ignored: {e}", cfg.id),
        }
    }
    let offset = cached.as_ref().map_or(0, |(_, offset, _, _)| *offset);
    let mut refs = std::collections::BTreeSet::new();
    let path = dir.join(&cfg.id).join("journal.jsonl");
    // Validate the history this restore reads: from the image's offset (the image's own blobs
    // were checked when it loaded), or all of it for a full replay. An image does not excuse an
    // incomplete journal; a full replay, when one happens, still reads every record.
    let validate = Instant::now();
    if let Some(reader) = super::jsonl::JsonLines::<Input>::open_at(&path, "journal", offset)? {
        for row in reader {
            let input = serde_json::to_value(&row?.value).unwrap_or_default();
            if let Some(missing) = shared.store.blobs().and_then(|b| b.missing(&input)) {
                return Err(format!(
                    "the journal refers to a missing blob ({missing}); copy the run's blobs/ directory with it"
                ));
            }
            super::checkpoint::refs_in(&input.to_string(), &mut refs);
        }
    }
    PHASES[4].fetch_add(validate.elapsed().as_micros() as u64, Ordering::Relaxed);
    let journal = super::jsonl::JsonLines::<Input>::open_at(&path, "journal", offset)?
        .into_iter()
        .flatten();
    if let Some((mut agent, _, from, mut seen)) = cached {
        let mut boundary = None;
        let replay = (|| -> Result<(), String> {
            // An image taken at a step boundary: the rest of its input's effects pass first.
            timed(3, || agent.finish_boundary(view, enc))?;
            let suffix = Instant::now();
            for row in journal {
                let row = row?;
                agent.handle(view, enc, row.value)?;
                agent.take_trace();
                if let Some(n) = agent.boundary_inputs().filter(|n| *n == agent.journal_len()) {
                    boundary = Some((n, row.end));
                }
            }
            PHASES[5].fetch_add(suffix.elapsed().as_micros() as u64, Ordering::Relaxed);
            Ok(())
        })();
        if let Err(error) = replay {
            eprintln!("{}: checkpoint suffix failed ({error}); retrying full replay", cfg.id);
            drop(agent);
            return restore_instance(shared, cfg, view, enc, dir, false);
        }
        let bytes = std::fs::metadata(&path).map_err(|e| e.to_string())?.len();
        let commands = agent.outstanding();
        let replayed = agent.journal_len() - from;
        agent.trace_checkpoint_restore(from, replayed);
        seen.extend(refs);
        Ok((
            agent,
            commands,
            super::checkpoint::Seed {
                saved: Some(from),
                bytes,
                boundary,
                refs: seen,
            },
        ))
    } else {
        // The complete validation pass above guarantees the iterator contains only valid inputs.
        // Where the latest step boundary falls is noted, so the next image need not wait for a new one.
        // Each line's end, by the inputs applied through it (a batch line holds several).
        let ends = std::cell::RefCell::new(std::collections::HashMap::new());
        let count = std::cell::Cell::new(0usize);
        let full = Instant::now();
        let (agent, commands) = Agent::restore_with(
            cfg,
            journal.map_while(Result::ok).map(|row| {
                count.set(
                    count.get()
                        + match &row.value {
                            Input::Batch { inputs } => inputs.len(),
                            _ => 1,
                        },
                );
                ends.borrow_mut().insert(count.get(), row.end);
                row.value
            }),
            view,
            enc,
            setup,
        )?;
        PHASES[6].fetch_add(full.elapsed().as_micros() as u64, Ordering::Relaxed);
        let bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let boundary = agent
            .boundary_inputs()
            .and_then(|n| ends.borrow().get(&n).map(|end| (n, *end)));
        Ok((
            agent,
            commands,
            super::checkpoint::Seed {
                saved: None,
                bytes,
                boundary,
                refs,
            },
        ))
    }
}

fn save_checkpoint(shared: &Shared, slot: &mut Slot, view: &View, force: bool) {
    if slot.error.is_some() {
        return;
    }
    if let Some(cache) = &mut slot.checkpoint {
        // An image is synced when written, and may refer to blobs no committed line refers to yet.
        match cache.save(&slot.agent, view, force, &|| shared.durable.sync_files()) {
            Ok(None) => {}
            // Why a restore will replay more of the journal than an interval.
            Ok(Some(reason)) => {
                let inputs = slot.agent.journal_len();
                trace(slot, json!({"t":"checkpoint_refused","inputs":inputs,"reason":reason}));
            }
            Err(error) => trace(slot, json!({"t":"checkpoint_error","error":error})),
        }
    }
}

/// The image to write for an instance as its worker stops, after everything held for a commit
/// has been released.
fn due_image(slot: &mut Slot, view: &View) -> Option<super::checkpoint::Ready> {
    if slot.error.is_some() {
        return None;
    }
    let due = slot.checkpoint.as_mut()?.prepare(&slot.agent, view, true);
    match due {
        Ok(super::checkpoint::Due::Image(image)) => return Some(image),
        Ok(super::checkpoint::Due::Nothing(None)) => {}
        Ok(super::checkpoint::Due::Nothing(Some(reason))) => {
            let inputs = slot.agent.journal_len();
            trace(slot, json!({"t":"checkpoint_refused","inputs":inputs,"reason":reason}));
        }
        Err(error) => trace(slot, json!({"t":"checkpoint_error","error":error})),
    }
    None
}

/// Run `write` for every item, several at a time, and give each one's result in order. For
/// writes to separate files that each wait for the disk: together they wait once.
fn together<T: Sync>(items: &[T], write: impl Fn(&T) -> Result<(), String> + Sync) -> Vec<Result<(), String>> {
    const AT_ONCE: usize = 32;
    if items.len() <= 1 {
        return items.iter().map(write).collect();
    }
    let write = &write;
    std::thread::scope(|scope| {
        let parts: Vec<_> = items
            .chunks(items.len().div_ceil(AT_ONCE))
            .map(|part| scope.spawn(move || part.iter().map(write).collect::<Vec<_>>()))
            .collect();
        parts
            .into_iter()
            .zip(items.chunks(items.len().div_ceil(AT_ONCE)))
            .flat_map(|(part, items)| {
                part.join()
                    .unwrap_or_else(|_| items.iter().map(|_| Err("a write panicked".to_string())).collect())
            })
            .collect()
    })
}

/// Start one instance on this worker: a fresh start, or its journal replayed. A restored
/// instance's children are started (and restored) after it.
#[allow(clippy::too_many_arguments)]
fn start_instance(
    shared: &Arc<Shared>,
    w: usize,
    views: &mut Views,
    slots: &mut BTreeMap<String, Slot>,
    spec: AgentSpec,
    app: PathBuf,
    env: BTreeMap<String, String>,
    restored: bool,
    timers: &mut Timers,
    local: &mut VecDeque<Msg>,
) -> Result<(), String> {
    let incarnation = fresh_incarnation();
    let secrets: Vec<(String, String)> = env
        .iter()
        .filter(|(k, _)| shared.cfg.secrets.contains(*k))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let machines = spec
        .machines
        .clone()
        .unwrap_or_else(|| shared.machines.keys().cloned().collect());
    let mut cfg = AgentConfig {
        id: spec.id.clone(),
        props: spec.props.clone(),
        env,
        max_new: spec.max_new,
        max_turns: spec.max_turns,
        max_prompt: spec.max_prompt,
        start_ms: shared.clock_ms(),
        seed: spec.seed.max(1),
        machines,
        model: spec.model.clone(),
        template: spec
            .model
            .as_ref()
            .and_then(|name| shared.models.get(name))
            .and_then(|(_, client)| client.template()),
        context: spec
            .model
            .as_ref()
            .and_then(|name| shared.models.get(name))
            .and_then(|(_, client)| client.context()),
        retry: spec
            .model
            .as_ref()
            .and_then(|name| shared.models.get(name))
            .map(|(_, client)| client.retry())
            .unwrap_or_default(),
    };
    shared.route.write().unwrap().insert(spec.id.clone(), w);
    let fail = |slots: &mut BTreeMap<String, Slot>,
                cfg: AgentConfig,
                spec: AgentSpec,
                app: PathBuf,
                e: String|
     -> Result<(), String> {
        eprintln!("{}: {e}", spec.id);
        let id = spec.id.clone();
        let mut slot = Slot {
            incarnation,
            agent: Agent::new(cfg)?,
            spec,
            app,
            secrets: vec![],
            journal: None,
            checkpoint: None,
            trace: None,
            error: Some(e),
            epoch: 0,
            held: VecDeque::new(),
        };
        flush(shared, &mut slot, &mut Timers::default());
        slots.insert(id, slot);
        Ok(())
    };
    if let Some((k, _)) = secrets.iter().find(|(_, v)| v.len() < MIN_SECRET) {
        return fail(
            slots,
            cfg,
            spec,
            app,
            format!("secret {k} is shorter than {MIN_SECRET} bytes, too short to redact"),
        );
    }
    let entry = spec.entry.clone().unwrap_or_else(|| shared.cfg.entry.clone());
    let (view, enc) = match views.get(shared, &app, &entry, cfg.model.as_deref()) {
        Ok(v) => v,
        Err(e) => return fail(slots, cfg, spec, app, e),
    };
    let (mut journal_file, mut trace_file) = (None, None);
    if !restored {
        shared.store.forget_tree(&spec.id);
    }
    if let Some(dir) = &shared.cfg.dir {
        let d = dir.join(&spec.id);
        if !restored && d.exists() {
            let archive = dir.join(".retired").join(incarnation.to_string()).join(&spec.id);
            std::fs::create_dir_all(archive.parent().unwrap()).map_err(|e| e.to_string())?;
            std::fs::rename(&d, &archive).map_err(|e| format!("archive {}: {e}", d.display()))?;
            for parent in [archive.parent().unwrap(), d.parent().unwrap()] {
                std::fs::File::open(parent)
                    .and_then(|f| f.sync_all())
                    .map_err(|e| e.to_string())?;
            }
        }
        std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
        let mut stored = spec.clone();
        for k in &shared.cfg.secrets {
            stored.env.remove(k);
        }
        let before = restored
            .then(|| std::fs::read(d.join("agent.json")).ok())
            .flatten()
            .and_then(|b| serde_json::from_slice::<J>(&b).ok());
        let start = before.as_ref().and_then(|v| v["start_ms"].as_i64());
        // What the journal was written against. A restore under other code or tokenizer is
        // reported, and replay then checks every turn's prompt (`diverged_at`). A journal of
        // another contract is refused: its inputs mean something else (like IndexedDB's
        // `VersionError`), so it needs the binary it was written with.
        let mut pins = json!({"contract": CONTRACT_VERSION, "app": view.source_hash, "tokenizer": enc.identity});
        if let Some((name, client)) = cfg
            .model
            .as_ref()
            .and_then(|name| shared.models.get(name).map(|(_, client)| (name, client)))
            && let Some(identity) = client.replay_identity()
        {
            pins["model"] = super::model_config::fingerprint(json!({"binding":name,"client":identity})).into();
        }
        let pinned = before.as_ref().map(|v| v["pins"].clone()).filter(|p| !p.is_null());
        if let Some(old) = pinned
            .as_ref()
            .map(|p| p["contract"].clone())
            .filter(|c| *c != json!(CONTRACT_VERSION))
        {
            return fail(
                slots,
                cfg,
                spec,
                app,
                format!(
                    "restore refused: the journal was written under harness contract {old}, and this binary runs contract {CONTRACT_VERSION}; restore it with a contract-{old} binary"
                ),
            );
        }
        if restored && pinned.as_ref().and_then(|old| old.get("model")) != pins.get("model") {
            if shared.cfg.strict_replay {
                return fail(slots,cfg,spec,app,"strict restore refused: model binding identity changed or was not recorded; restore the original binding configuration, or use warning replay explicitly".into());
            }
            eprintln!(
                "{}: warning: model binding identity changed or was not recorded",
                spec.id
            );
        }
        if let Some(old) = pinned.as_ref().filter(|old| **old != pins) {
            eprintln!("{}: restoring under changed pins {old} -> {pins}", spec.id);
        }
        let mut stored_json = serde_json::to_value(&stored).map_err(|e| e.to_string())?;
        // A restored instance replays under what it was started with: its seed and its retry
        // policy. Either one read from today's deployment could send the replay down a branch
        // the journal did not take.
        if restored && let Some(old) = &before {
            let was: super::model::RetryPolicy =
                serde_json::from_value(old["retry"].clone()).map_err(|e| format!("agent.json retry: {e}"))?;
            if was != cfg.retry {
                eprintln!(
                    "{}: model calls keep the retry policy this run was started with; a fresh run takes the new one",
                    spec.id
                );
            }
            cfg.retry = was;
            if let Some(seed) = old["seed"].as_u64() {
                cfg.seed = seed.max(1);
            }
        }
        stored_json["retry"] = json!(cfg.retry);
        stored_json["seed"] = json!(cfg.seed);
        stored_json["start_ms"] = json!(start.unwrap_or(cfg.start_ms));
        stored_json["pins"] = pinned.unwrap_or_else(|| {
            let mut initial = pins;
            // A warning restore cannot certify the model used by an unpinned old journal.
            if restored && initial.get("model").is_some() {
                initial["model"] = J::Null;
            }
            initial
        });
        stored_json["incarnation"] = if restored {
            before.as_ref().and_then(|v| v["incarnation"].as_u64()).unwrap_or(0)
        } else {
            incarnation
        }
        .into();
        let clean = redact(stored_json, &secrets);
        atomic_value(&d.join("agent.json"), &clean)?;
        let open = |name: &str| {
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(d.join(name))
                .map_err(|e| e.to_string())
        };
        journal_file = Some(open("journal.jsonl")?);
        trace_file = Some(open("trace.jsonl")?);
        if let Err(e) = timed(0, || shared.store.load(&spec.id)) {
            return fail(slots, cfg, spec, app, format!("store recovery failed: {e}"));
        }
    }
    let dir = shared.cfg.dir.clone().filter(|_| restored);
    let (agent, commands, error, restored_from) = match &dir {
        Some(d) => {
            let start = std::fs::read(d.join(&spec.id).join("agent.json"))
                .ok()
                .and_then(|b| serde_json::from_slice::<J>(&b).ok())
                .and_then(|v| v["start_ms"].as_i64())
                .unwrap_or(cfg.start_ms);
            let replay_cfg = AgentConfig {
                start_ms: start,
                ..cfg.clone()
            };
            // A restore's working set (its journal's parsed inputs, the replay's garbage) is
            // freed when it returns, but glibc keeps the pages: they are returned before the
            // next restore takes the turn, so the boot peak is the steady heap plus the restores
            // in progress.
            let restored = {
                let _turn = shared.restoring.take();
                let restored = contained(|| restore_instance(shared, replay_cfg, &view, &enc, d, secrets.is_empty()));
                trim_heap();
                restored
            };
            match restored {
                Ok((a, cmds, seed)) => (a, cmds, None, Some(seed)),
                Err(e) => {
                    eprintln!("{}: restore failed: {e}", spec.id);
                    (Agent::new(cfg)?, vec![], Some(format!("restore failed: {e}")), None)
                }
            }
        }
        None => {
            let mut a = Agent::new(cfg)?;
            if let Some(b) = shared.store.blobs() {
                a.set_blobs(b.clone());
            }
            match contained(|| a.start(&view, &enc)) {
                Ok(c) => (a, c, None, None),
                Err(e) => (a, vec![], Some(e), None),
            }
        }
    };
    let checkpoint = shared
        .cfg
        .dir
        .as_ref()
        .filter(|_| secrets.is_empty() && error.is_none())
        .and_then(|d| {
            let key = checkpoint_identity(shared, &agent.cfg, &view, &enc);
            match super::checkpoint::Checkpoint::new(&d.join(&spec.id), key, restored_from) {
                Ok(cache) => Some(cache),
                Err(e) => {
                    eprintln!("{}: checkpoint cache unavailable: {e}", spec.id);
                    None
                }
            }
        });
    let id = spec.id.clone();
    let children: Vec<(String, J)> = if dir.is_some() {
        agent.children().iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    } else {
        vec![]
    };
    let incarnation = if restored {
        shared
            .cfg
            .dir
            .as_ref()
            .and_then(|d| std::fs::read(d.join(&id).join("agent.json")).ok())
            .and_then(|b| serde_json::from_slice::<J>(&b).ok())
            .and_then(|v| v["incarnation"].as_u64())
            .unwrap_or(0)
    } else {
        incarnation
    };
    let journal = match journal_file {
        Some(file) => Some(Journal::new(file, &id, incarnation)?),
        None => None,
    };
    if let (Some(journal), Some(dir), false) = (&journal, &shared.cfg.dir, restored) {
        shared.durable.register(journal, &dir.join(&id));
    }
    // Its start's effects wait for whatever caused it (a parent's mount) to be committed.
    let epoch = shared.durable.written();
    slots.insert(
        id.clone(),
        Slot {
            incarnation,
            agent,
            spec,
            app,
            secrets,
            journal,
            checkpoint,
            trace: trace_file,
            error,
            epoch,
            held: VecDeque::new(),
        },
    );
    let slot = slots.get_mut(&id).unwrap();
    // The journal's clock is logical: it stands still while the run restores and from when it
    // begins closing (a wait does not expire while the run is closed). Each start records where it stands against wall time, so a host can map any
    // later `at_ms` or `clock_ms` to wall time: wall = at_ms + (wall_ms - at_ms) of this record.
    trace(
        slot,
        json!({"t": "clock", "at_ms": shared.clock_ms(), "wall_ms": now_ms()}),
    );
    for c in commands {
        dispatch(shared, w, slot, c, timers, local);
    }
    // A restored instance's children were mounted by its replay: start them too. They are queued
    // (in flight) before this instance's status says idle, so the runtime cannot look finished
    // in between and stop without them.
    for (key, d) in children {
        let machines = d["machines"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|m| m.as_str().map(str::to_owned))
            .collect();
        push_local(
            shared,
            local,
            (
                format!("{id}/{key}"),
                InputKind::Mount {
                    app: d["app"].as_str().unwrap_or_default().into(),
                    entry: d["entry"].as_str().unwrap_or_default().into(),
                    props: d["props"].clone(),
                    model: d["model"].as_str().map(str::to_owned),
                    machines,
                    restore: true,
                },
            ),
        );
    }
    flush(shared, slot, timers);
    save_checkpoint(shared, slot, &view, slot.agent.stopped().is_some());
    Ok(())
}

/// Queue work for this worker: it counts as in flight until the worker has fed it, so the
/// coordinator never sees an idle runtime with work still queued here.
fn push_local(shared: &Shared, local: &mut VecDeque<Msg>, item: (String, InputKind)) {
    shared.inflight.fetch_add(1, Ordering::SeqCst);
    local.push_back(Msg::Input(item.0, item.1));
}

fn worker(
    w: usize,
    shared: Arc<Shared>,
    specs: Vec<AgentSpec>,
    restored: HashSet<String>,
    rx: Receiver<Msg>,
) -> Result<Vec<AgentReport>, String> {
    let mut views = Views { loaded: HashMap::new() };
    let mut slots: BTreeMap<String, Slot> = BTreeMap::new();
    let mut timers = Timers::default();
    let mut local: VecDeque<Msg> = VecDeque::new();
    let mut pending: VecDeque<Msg> = VecDeque::new();
    // The boot's own time, from the runtime's start to the last worker's last agent started. A
    // worker counts down however its boot ends, a panic included, so the runtime can finish.
    struct Booting<'a>(&'a Shared);
    impl Drop for Booting<'_> {
        fn drop(&mut self) {
            let shared = self.0;
            if shared.booting.fetch_sub(1, Ordering::SeqCst) != 1 {
                return;
            }
            // Every agent is running: time passes from now (a restore takes none from timers).
            shared.clock.start();
            if std::thread::panicking() {
                return;
            }
            eprintln!(
                "affordance: started {} agents on {} workers in {} ms{}",
                shared.route.read().unwrap().len(),
                shared.workers.len(),
                shared.started.elapsed().as_millis(),
                if std::env::var("AFFORDANCE_RESTORE_TIMING").is_ok() {
                    format!(" [{}]", restore_timing())
                } else {
                    String::new()
                }
            );
        }
    }
    let booting = Booting(&shared);
    for spec in specs {
        let mut env = shared.cfg.env.clone();
        env.extend(spec.env.clone());
        let app = spec
            .app
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| shared.cfg.view.clone());
        let (id, journal) = (spec.id.clone(), restored.contains(&spec.id));
        // An agent that cannot start fails alone; the worker starts the rest.
        if let Err(error) = start_instance(
            &shared,
            w,
            &mut views,
            &mut slots,
            spec,
            app,
            env,
            journal,
            &mut timers,
            &mut local,
        ) {
            match slots.get_mut(&id) {
                Some(slot) => {
                    eprintln!("{id}: {error}");
                    slot.error = Some(error);
                    flush(&shared, slot, &mut timers);
                }
                None => lost(&shared, w, &id, error),
            }
        }
    }
    drop(booting);
    loop {
        // Give each source a turn: local completions cannot starve timers or control.
        let mut progressed = false;
        if let Some(Msg::Input(id, kind)) = local.pop_front() {
            let inputs = batching::collect(&mut local, &id, kind, shared.cfg.input_batch);
            let count = inputs.len() as i64;
            batching::feed_group(&shared, w, &mut views, &mut slots, &id, inputs, &mut timers, &mut local);
            shared.inflight.fetch_sub(count, Ordering::SeqCst);
            progressed = true;
        }
        let now = shared.clock_ms();
        if let Some(id) = timers.pop_due(now) {
            feed(
                &shared,
                w,
                &mut views,
                &mut slots,
                &id,
                InputKind::Tick,
                &mut timers,
                &mut local,
            );
            progressed = true;
        }
        let wait = if progressed {
            0
        } else {
            timers.next_at().map_or(50, |at| (at - now).max(1) as u64).min(50)
        };
        // Nothing queued here: what this worker wrote is committed now, not after a group window.
        if !progressed && pending.is_empty() && local.is_empty() {
            shared.durable.request();
        }
        if pending.is_empty() {
            match rx.recv_timeout(Duration::from_millis(wait)) {
                Ok(message) => {
                    pending.push_back(message);
                    batching::fill(&mut pending, &rx, shared.cfg.input_batch);
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            }
        }
        match pending.pop_front() {
            Some(Msg::Input(id, kind)) => {
                let inputs = batching::collect(&mut pending, &id, kind, shared.cfg.input_batch);
                let count = inputs.len() as i64;
                batching::feed_group(&shared, w, &mut views, &mut slots, &id, inputs, &mut timers, &mut local);
                shared.inflight.fetch_sub(count, Ordering::SeqCst);
            }
            Some(Msg::Inspect(g, ids)) => {
                inspect_here(&shared, &slots, &g, &ids);
                shared.inflight.fetch_sub(1, Ordering::SeqCst);
            }
            Some(Msg::Durable(ids)) => {
                for id in ids {
                    if let Some(slot) = slots.get_mut(&id) {
                        release(&shared, w, slot, &mut timers, &mut local);
                    }
                }
                shared.inflight.fetch_sub(1, Ordering::SeqCst);
            }
            Some(Msg::Shutdown) => break,
            None => {}
        }
    }
    // Images and snapshots are of committed journals.
    shared.durable.wait(shared.durable.written())?;
    // Then what was held for it: posts are answered; held requests are not issued while the
    // runtime stops, so they stay outstanding for the restore.
    for slot in slots.values_mut() {
        release(&shared, w, slot, &mut timers, &mut local);
    }
    // Each instance's recovery image and snapshot are worked out here, one instance after
    // another, and then written together. They are separate files that do not refer to each
    // other, and each write waits for the disk twice (the file, then its directory): written in
    // turn, a stop would take that long for every instance; written together, it takes that
    // long once, whatever the number of instances.
    let mut images = vec![];
    let mut snapshots = vec![];
    for slot in slots.values_mut() {
        let entry = slot.spec.entry.as_deref().unwrap_or(&shared.cfg.entry);
        if let Ok((view, _)) = views.get(&shared, &slot.app, entry, slot.agent.cfg.model.as_deref())
            && let Some(image) = due_image(slot, &view)
        {
            images.push((slot.spec.id.clone(), image));
        }
        if let Some(dir) = &shared.cfg.dir {
            let value = json!({"state": shared.expand_snapshot(&slot.agent.state()), "metrics": slot.agent.metrics, "clock_ms": slot.agent.clock_ms(), "error": slot.error});
            snapshots.push((
                dir.join(&slot.spec.id).join("snapshot.json"),
                value.to_string().into_bytes(),
            ));
        }
    }
    // An image may refer to blobs no committed line refers to yet: they are synced first.
    let blobs = if images.is_empty() {
        Ok(())
    } else {
        shared.durable.sync_files()
    };
    let written = match &blobs {
        Ok(()) => together(&images, |(_, image)| image.write()),
        Err(error) => images.iter().map(|_| Err(error.clone())).collect(),
    };
    for ((id, image), result) in images.iter().zip(written) {
        let Some(slot) = slots.get_mut(id) else { continue };
        match result {
            Ok(()) => {
                if let Some(cache) = &mut slot.checkpoint {
                    cache.wrote(image);
                }
            }
            Err(error) => trace(slot, json!({"t":"checkpoint_error","error":error})),
        }
    }
    let snapshots_written = together(&snapshots, |(path, bytes)| atomic_bytes(path, bytes));
    // The traces last: a failed image is a line in one.
    let traces: Vec<&std::fs::File> = slots.values().filter_map(|slot| slot.trace.as_ref()).collect();
    let traces_synced = together(&traces, |file| file.sync_all().map_err(|e| e.to_string()));
    traces_synced
        .into_iter()
        .chain(snapshots_written)
        .collect::<Result<(), String>>()?;
    Ok(slots.values().map(|s| report_of(&shared, s)).collect())
}

fn report_of(shared: &Shared, s: &Slot) -> AgentReport {
    AgentReport {
        id: s.spec.id.clone(),
        worker: shared.route.read().unwrap().get(&s.spec.id).copied().unwrap_or(0),
        diverged_at: s.agent.diverged_at(),
        metrics: s.agent.metrics.clone(),
        render_stats: s.agent.render_stats.clone(),
        stopped: s.agent.stopped().cloned(),
        state: if shared.cfg.report_state {
            shared.expand(&s.agent.state())
        } else {
            J::Null
        },
        journal_len: s.agent.journal_len(),
        error: s.error.clone(),
    }
}

#[allow(clippy::too_many_arguments)]
fn feed(
    shared: &Arc<Shared>,
    w: usize,
    views: &mut Views,
    slots: &mut BTreeMap<String, Slot>,
    id: &str,
    kind: InputKind,
    timers: &mut Timers,
    local: &mut VecDeque<Msg>,
) {
    let kind = match kind {
        InputKind::Completion { incarnation, input } => {
            if slots.get(id).is_none_or(|slot| slot.incarnation != incarnation) {
                return;
            }
            *input
        }
        kind => kind,
    };
    match kind {
        InputKind::Mount {
            app,
            entry,
            props,
            model,
            machines,
            restore,
        } => {
            let Some((parent, _)) = id.rsplit_once('/') else { return };
            let Some(p) = slots.get(parent) else { return };
            if slots.contains_key(id) {
                return;
            }
            shared.retired.lock().unwrap().remove(id);
            // The child's bindings come from its parent: machines a subset, the model one that exists.
            let base = PathBuf::from(&app);
            let mut spec = AgentSpec::new(id, props);
            spec.max_new = p.spec.max_new;
            // Each child its own random sequence: from its parent's seed and its own id.
            spec.seed = p.spec.seed.wrapping_mul(31).wrapping_add(stable_hash(id));
            spec.machines = Some(
                machines
                    .into_iter()
                    .filter(|m| p.agent.cfg.machines.contains(m))
                    .collect(),
            );
            spec.model = model;
            spec.app = Some(base.display().to_string());
            spec.entry = Some(entry);
            spec.rank = p.spec.rank;
            spec.max_turns = p.spec.max_turns;
            spec.max_prompt = p.spec.max_prompt;
            spec.capabilities = p.spec.capabilities.clone();
            spec.inspectable = p.spec.inspectable;
            let env = p.agent.cfg.env.clone();
            let restored = restore
                && shared
                    .cfg
                    .dir
                    .as_ref()
                    .is_some_and(|d| d.join(id).join("journal.jsonl").exists());
            // A restored child replays from the props it started with; the parent's current props,
            // if different, are then one more input.
            let current = spec.props.clone();
            if restored
                && let Some(first) = shared
                    .cfg
                    .dir
                    .as_ref()
                    .and_then(|d| std::fs::read(d.join(id).join("agent.json")).ok())
                    .and_then(|b| serde_json::from_slice::<J>(&b).ok())
                    .map(|v| v["props"].clone())
            {
                spec.props = first;
            }
            if let Err(e) = start_instance(shared, w, views, slots, spec, base, env, restored, timers, local) {
                // As for an instance that fails at boot: it is reported failed, so the run's
                // report names it and a send to it says why it was not delivered.
                lost(shared, w, id, e);
                return;
            }
            if let Some(slot) = slots.get(id) {
                let mut now = slot.agent.state().clone();
                if let Some(fields) = now.as_object_mut() {
                    fields.remove("__ui");
                }
                if now != current {
                    push_local(shared, local, (id.to_string(), InputKind::Props { props: current }));
                }
            }
            return;
        }
        InputKind::Unmount => {
            let gone: Vec<String> = slots
                .keys()
                .filter(|k| *k == id || k.starts_with(&format!("{id}/")))
                .cloned()
                .collect();
            for k in gone {
                let Some(mut slot) = slots.remove(&k) else { continue };
                // Its held effects are dropped: they stay outstanding in its journal.
                shared
                    .inflight
                    .fetch_sub(std::mem::take(&mut slot.held).len() as i64, Ordering::SeqCst);
                for ((agent, _, _), live) in shared.cancels.lock().unwrap().iter() {
                    if *agent == k {
                        live.token.cancel("unmounted");
                    }
                }
                timers.forget(&k);
                let entry = slot.spec.entry.as_deref().unwrap_or(&shared.cfg.entry);
                if let Ok((view, _)) = views.get(shared, &slot.app, entry, slot.agent.cfg.model.as_deref()) {
                    save_checkpoint(shared, &mut slot, &view, true);
                }
                let entries = slot.agent.take_trace();
                for e in entries {
                    trace(&mut slot, e);
                }
                if let Some(dir) = &shared.cfg.dir {
                    let _ = atomic_value(
                        &dir.join(&k).join("snapshot.json"),
                        &json!({"state": shared.expand_snapshot(&slot.agent.state()), "metrics": slot.agent.metrics, "clock_ms": slot.agent.clock_ms(), "error": slot.error}),
                    );
                }
                shared
                    .retired
                    .lock()
                    .unwrap()
                    .insert(k.clone(), report_of(shared, &slot));
                shared.status.lock().unwrap().remove(&k);
                shared.route.write().unwrap().remove(&k);
            }
            return;
        }
        _ => {}
    }
    // A message the sender waits on: acknowledged once journaled here, refused if this instance
    // is gone or failed.
    let (ack, kind) = match kind {
        InputKind::Event { name, payload, id, ack } => (
            ack,
            InputKind::Event {
                name,
                payload,
                id,
                ack: None,
            },
        ),
        other => (None, other),
    };
    let Some(slot) = slots.get_mut(id) else { return };
    if slot.error.is_some() {
        return;
    }
    let clock_ms = shared.clock_ms().max(slot.agent.clock_ms());
    let input = match kind {
        InputKind::Reply {
            task,
            ids,
            cut_off,
            stats,
            prompt,
            aborted,
        } => {
            // A watcher sees the call's statistics too (time to first token, tokens per second).
            if let Some(f) = &shared.cfg.observer {
                (f.0)(
                    &slot.spec.id,
                    task,
                    Observed::Trace(&redact(stats.clone(), &slot.secrets)),
                );
            }
            let server = json!({"cached_tokens": stats["cached_tokens"], "ms": stats["ms"], "first_token_ms": stats["first_token_ms"],
                "wall_ms": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)});
            trace(slot, stats);
            Input::Reply {
                task,
                ids,
                cut_off,
                clock_ms,
                prompt: Some(prompt),
                aborted,
                server: Some(server),
            }
        }
        InputKind::ModelError {
            task,
            error,
            kind,
            retry_after_ms,
        } => Input::ModelError {
            task,
            error,
            kind,
            retry_after_ms,
            clock_ms,
        },
        InputKind::Result { task, outcome } => Input::Result {
            task,
            outcome,
            clock_ms,
        },
        InputKind::Event { name, payload, id, .. } => Input::Event {
            name,
            payload,
            id,
            clock_ms,
        },
        InputKind::Props { props } => Input::Props { props, clock_ms },
        InputKind::Tick => Input::Tick { clock_ms },
        InputKind::Mount { .. } | InputKind::Unmount | InputKind::Completion { .. } => unreachable!(),
    };
    let entry = slot.spec.entry.clone().unwrap_or_else(|| shared.cfg.entry.clone());
    let (view, enc) = match views.get(shared, &slot.app, &entry, slot.agent.cfg.model.as_deref()) {
        Ok(v) => v,
        Err(e) => {
            slot.error = Some(e);
            return;
        }
    };
    let journaled = apply(shared, w, &view, &enc, slot, input, timers, local);
    settle(shared, w, slot, ack, journaled, timers, local);
}

/// Answer a delivery's sender once its event is journaled here: a `send` at once (its answer is
/// itself an input, committed after this one), an embedder's post once the line is committed.
fn settle(
    shared: &Arc<Shared>,
    w: usize,
    slot: &mut Slot,
    ack: Option<delivery::Ack>,
    journaled: bool,
    timers: &mut Timers,
    local: &mut VecDeque<Msg>,
) {
    let Some(ack) = ack else { return };
    if journaled && ack.waits_for_commit() && slot.journal.is_some() {
        hold(shared, w, slot, Held::Answer(ack), timers, local);
    } else {
        ack.finish(journaled);
    }
}

#[allow(clippy::too_many_arguments)]
/// Journal `input`, then apply it. Returns whether it was journaled (durable).
fn apply(
    shared: &Arc<Shared>,
    w: usize,
    view: &View,
    enc: &Encoder,
    slot: &mut Slot,
    mut input: Input,
    timers: &mut Timers,
    local: &mut VecDeque<Msg>,
) -> bool {
    // Large strings are on disk before the journal line that refers to them is written.
    if let Err(e) = slot.agent.lower_input(&mut input) {
        slot.error = Some(format!("blob storage failed: {e}"));
        flush(shared, slot, timers);
        return false;
    }
    // Journal first. The line is durable at its epoch's commit, before any effect it causes
    // leaves the process (`hold`).
    if let Some(journal) = slot.journal.as_mut() {
        let line = format!("{}\n", serde_json::to_string(&input).unwrap());
        match shared.durable.append(journal, line.as_bytes()) {
            Ok(epoch) => slot.epoch = epoch,
            Err(e) => {
                slot.error = Some(format!("journal durability failed: {e}"));
                flush(shared, slot, timers);
                return false;
            }
        }
        if let Some(cache) = &mut slot.checkpoint {
            cache.appended(line.as_bytes());
        }
    }
    let was_stopped = slot.agent.stopped().is_some();
    match contained(|| slot.agent.handle(view, enc, input)) {
        Ok(commands) => {
            for c in commands {
                dispatch(shared, w, slot, c, timers, local);
            }
        }
        Err(e) => {
            // An app error that escapes the agent (not a handler error, which becomes a tool
            // result), or a panic, stops this agent; the others continue.
            slot.error = Some(e.clone());
            trace(slot, json!({"t": "agent_error", "error": e}));
        }
    }
    flush(shared, slot, timers);
    if let Some(cache) = &mut slot.checkpoint {
        cache.handled(&slot.agent);
    }
    save_checkpoint(shared, slot, view, !was_stopped && slot.agent.stopped().is_some());
    true
}

/// One live call of an instance: what cancels it (the run closing, the agent no longer
/// needing it), and for a model call the flag its app's abort signal sets.
struct Live {
    token: CancelToken,
    abort: Option<Arc<AtomicBool>>,
}

/// A worker's timers: when each of its instances is next due. Ordered by time, and listed by
/// instance, so finding the next one and dropping an instance's stale ones cost what that
/// instance has, not what the whole worker has.
#[derive(Default)]
pub(super) struct Timers {
    due: BTreeSet<(i64, String)>,
    of: HashMap<String, Vec<i64>>,
}

impl Timers {
    fn push(&mut self, at: i64, id: String) {
        if self.due.insert((at, id.clone())) {
            self.of.entry(id).or_default().push(at);
        }
    }
    /// The instance whose timer is due first, taken off.
    fn pop_due(&mut self, now: i64) -> Option<String> {
        let (at, id) = self.due.first().filter(|(at, _)| *at <= now)?.clone();
        self.due.remove(&(at, id.clone()));
        if let Some(ats) = self.of.get_mut(&id) {
            ats.retain(|t| *t != at);
        }
        Some(id)
    }
    fn next_at(&self) -> Option<i64> {
        self.due.first().map(|(at, _)| *at)
    }
    /// Keep only `id`'s timers that are in `live` (sorted): a wait that ended early leaves none.
    fn keep(&mut self, id: &str, live: &[i64]) {
        let Timers { due, of } = self;
        if let Some(ats) = of.get_mut(id) {
            ats.retain(|at| {
                let live = live.binary_search(at).is_ok();
                if !live {
                    due.remove(&(*at, id.to_string()));
                }
                live
            });
            if ats.is_empty() {
                of.remove(id);
            }
        }
    }
    fn forget(&mut self, id: &str) {
        for at in self.of.remove(id).unwrap_or_default() {
            self.due.remove(&(at, id.to_string()));
        }
    }
}

/// Run `item` once the input that caused it is committed: now if it is, otherwise after the
/// effects held before it.
fn hold(shared: &Arc<Shared>, w: usize, slot: &mut Slot, item: Held, timers: &mut Timers, local: &mut VecDeque<Msg>) {
    if slot.held.is_empty() {
        match shared.durable.watch(w, &slot.spec.id, slot.epoch) {
            Ready::Now => return run_held(shared, w, slot, item, timers, local),
            Ready::Failed(error) => {
                slot.error = Some(error);
                return flush(shared, slot, timers);
            }
            Ready::Later => {}
        }
    }
    // A held item counts as in flight: the run is not idle while it waits.
    shared.inflight.fetch_add(1, Ordering::SeqCst);
    slot.held.push_back((slot.epoch, item));
}

/// After a commit: run the held effects it covers, in order.
fn release(shared: &Arc<Shared>, w: usize, slot: &mut Slot, timers: &mut Timers, local: &mut VecDeque<Msg>) {
    while let Some(&(epoch, _)) = slot.held.front() {
        match shared.durable.watch(w, &slot.spec.id, epoch) {
            Ready::Later => return,
            Ready::Failed(error) => {
                shared
                    .inflight
                    .fetch_sub(std::mem::take(&mut slot.held).len() as i64, Ordering::SeqCst);
                slot.error = Some(error);
                return flush(shared, slot, timers);
            }
            Ready::Now => {
                let (_, item) = slot.held.pop_front().unwrap();
                shared.inflight.fetch_sub(1, Ordering::SeqCst);
                run_held(shared, w, slot, item, timers, local);
            }
        }
    }
}

fn run_held(
    shared: &Arc<Shared>,
    w: usize,
    slot: &mut Slot,
    item: Held,
    timers: &mut Timers,
    local: &mut VecDeque<Msg>,
) {
    match item {
        Held::Command(cmd) => dispatch_now(shared, w, slot, cmd, timers, local),
        Held::Answer(ack) => ack.finish(true),
    }
}

fn trace(slot: &mut Slot, mut entry: J) {
    // Every record has its time, in the journal's clock (the agent's records carry their own).
    if let Some(o) = entry.as_object_mut() {
        o.entry("at_ms").or_insert(json!(slot.agent.clock_ms()));
    }
    if let Some(f) = slot.trace.as_mut() {
        let line = serde_json::to_string(&redact(entry, &slot.secrets)).unwrap();
        let _ = writeln!(f, "{line}");
    }
}

/// Write the agent's trace entries and publish its status for the coordinator.
fn flush(shared: &Shared, slot: &mut Slot, timers: &mut Timers) {
    let entries = slot.agent.take_trace();
    for e in entries {
        if let Some(f) = &shared.cfg.observer {
            (f.0)(&slot.spec.id, 0, Observed::Trace(&redact(e.clone(), &slot.secrets)));
        }
        trace(slot, e);
    }
    let summary = slot.agent.outstanding_summary();
    // The agent's own timers are the live ones: a wait that ended early leaves none behind.
    let mut live = summary.timers;
    live.sort_unstable();
    timers.keep(&slot.spec.id, &live);
    let (outstanding, phase, retrying) = (summary.count, summary.phase, summary.retrying);
    let stopped = slot.agent.stopped().is_some() || slot.error.is_some();
    shared.status.lock().unwrap().insert(
        slot.spec.id.clone(),
        Status {
            stopped,
            outstanding,
            turns: slot.agent.metrics.turns,
            generated_tokens: slot.agent.metrics.generated_tokens,
            handler_errors: slot.agent.metrics.handler_errors,
            last_activity_ms: now_ms(),
            failed: slot.error.is_some(),
            phase,
            retrying,
        },
    );
}

/// Issue a command. What leaves the process (a model request, a machine call, a store
/// operation) is held until the input that caused it is committed; deliveries to other
/// instances, timers and the instance tree are not.
fn dispatch(
    shared: &Arc<Shared>,
    w: usize,
    slot: &mut Slot,
    cmd: Command,
    timers: &mut Timers,
    local: &mut VecDeque<Msg>,
) {
    let waits = match &cmd {
        Command::Generate { .. } => true,
        Command::Call { name, .. } => name.starts_with("machine:"),
        // A fresh mount over an old directory archives it, which no recovery undoes: the worker
        // waits for the commit of the input that mounts it (rare, and in order with the tree).
        Command::Mount { key, .. }
            if shared
                .cfg
                .dir
                .as_ref()
                .is_some_and(|d| d.join(&slot.spec.id).join(key).exists()) =>
        {
            if let Err(error) = shared.durable.wait(slot.epoch) {
                slot.error = Some(error);
                return flush(shared, slot, timers);
            }
            false
        }
        Command::Cancel { task } => {
            let found = slot.held.iter().position(|(_, held)| matches!(held, Held::Command(Command::Generate { task: t, .. } | Command::Call { task: t, .. }) if t == task));
            // A held request that is no longer needed is never issued.
            if let Some(i) = found {
                slot.held.remove(i);
                shared.inflight.fetch_sub(1, Ordering::SeqCst);
                return;
            }
            false
        }
        // A held model call that is stopped before it starts answers at once, with nothing.
        Command::Abort { task } => {
            let task = *task;
            let found = slot
                .held
                .iter()
                .position(|(_, held)| matches!(held, Held::Command(Command::Generate { task: t, .. }) if *t == task));
            if let Some(i) = found {
                let Some((_, Held::Command(Command::Generate { ids, prompt_hash, .. }))) = slot.held.remove(i) else {
                    unreachable!()
                };
                shared.inflight.fetch_sub(1, Ordering::SeqCst);
                let reply = InputKind::Reply {
                    task,
                    ids: vec![],
                    cut_off: false,
                    aborted: true,
                    stats: json!({"t": "generate", "task": task, "tokens": 0, "aborted": true}),
                    prompt: prompt_hash.unwrap_or_else(|| super::agent::prompt_hash(&ids)),
                };
                return push_local(shared, local, (slot.spec.id.clone(), reply));
            }
            false
        }
        _ => false,
    };
    if waits {
        hold(shared, w, slot, Held::Command(cmd), timers, local)
    } else {
        dispatch_now(shared, w, slot, cmd, timers, local)
    }
}

fn dispatch_now(
    shared: &Arc<Shared>,
    w: usize,
    slot: &mut Slot,
    cmd: Command,
    timers: &mut Timers,
    local: &mut VecDeque<Msg>,
) {
    let id = slot.spec.id.clone();
    let incarnation = slot.incarnation;
    match cmd {
        Command::Timer { at_ms } => timers.push(at_ms, id),
        // The instance tree: children live on their parent's worker.
        Command::Mount {
            key,
            app,
            entry,
            props,
            model,
            machines,
        } => push_local(
            shared,
            local,
            (
                format!("{id}/{key}"),
                InputKind::Mount {
                    app,
                    entry,
                    props,
                    model,
                    machines,
                    restore: false,
                },
            ),
        ),
        Command::Props { key, props } => push_local(shared, local, (format!("{id}/{key}"), InputKind::Props { props })),
        Command::Unmount { key } => push_local(shared, local, (format!("{id}/{key}"), InputKind::Unmount)),
        // A child calls a callback its parent passed in props: an event in the parent.
        Command::Callback { name, args } => {
            if let Some((parent, key)) = id.rsplit_once('/') {
                push_local(
                    shared,
                    local,
                    (
                        parent.to_string(),
                        InputKind::Event {
                            name: format!("mount:{key}:{name}"),
                            payload: json!({"$args": args}),
                            ack: None,
                            id: None,
                        },
                    ),
                );
            }
        }
        Command::Cancel { task } => {
            if let Some(live) = shared.cancels.lock().unwrap().remove(&(id.clone(), incarnation, task)) {
                live.token.cancel("the agent no longer needs it");
            }
        }
        // The model call stops at its next tokens and answers with what it has. A call already
        // answered is not found: its whole reply is journaled.
        Command::Abort { task } => {
            if let Some(flag) = shared
                .cancels
                .lock()
                .unwrap()
                .get(&(id.clone(), incarnation, task))
                .and_then(|live| live.abort.as_ref())
            {
                flag.store(true, Ordering::SeqCst);
            }
        }
        Command::Generate {
            task,
            ids,
            stop,
            max_new,
            opts,
            prompt_hash,
            images,
            media,
        } => {
            if shared.closing.load(Ordering::SeqCst) {
                return;
            }
            let Some((_, model)) = slot
                .agent
                .cfg
                .model
                .as_ref()
                .and_then(|m| shared.models.get(m))
                .cloned()
            else {
                shared.done(&id, incarnation, task);
                return push_local(
                    shared,
                    local,
                    (
                        id.clone(),
                        InputKind::ModelError {
                            task,
                            kind: super::model::FailureKind::Refused,
                            retry_after_ms: None,
                            error: format!("no model binding {:?}", slot.agent.cfg.model),
                        },
                    ),
                );
            };
            // The call is live from here: one record, with its token and its abort flag.
            let (cancel, abort) = shared.model_token(&id, incarnation, task);
            // Listed, then checked: the run's close either sees this call or this call sees the
            // close (and stays outstanding for a restore), never neither.
            if shared.closing.load(Ordering::SeqCst) {
                shared.done(&id, incarnation, task);
                return;
            }
            let ranks = model.ranks();
            let rank = slot
                .spec
                .rank
                .or_else(|| (ranks > 1).then(|| (stable_hash(&id) % ranks as u64) as usize));
            let req = GenRequest {
                sampling: opts.sampling,
                tag: opts.tag,
                images,
                media,
                diagnostics: shared.diagnostics.clone(),
                ..GenRequest::new(&id, ids, max_new, stop, rank, cancel)
            };
            // A pipe: the machine its pieces are written to, which the agent must be bound to (the
            // host holds the authority, as for any machine call), and the call's context.
            let pipe = if let Some(options) = opts.pipe.as_deref() {
                let role = options["machine"].as_str().unwrap_or_default();
                let exec = shared
                    .machines
                    .get(role)
                    .cloned()
                    .filter(|_| slot.agent.cfg.machines.iter().any(|m| m == role));
                let Some(exec) = exec else {
                    shared.done(&id, incarnation, task);
                    return push_local(
                        shared,
                        local,
                        (
                            id.clone(),
                            InputKind::ModelError {
                                task,
                                kind: super::model::FailureKind::Refused,
                                retry_after_ms: None,
                                error: format!("model(): pipe: no machine {role:?} is bound to this agent"),
                            },
                        ),
                    );
                };
                let ctx = CallContext {
                    agent: id.clone(),
                    task,
                    env: slot.agent.cfg.env.clone(),
                    // The call's own token: cancelling the call stops the generation and the pipe.
                    cancel: req.cancel.clone(),
                    blobs: None,
                    output: None,
                };
                Some(Pipe::new(exec, ctx, options))
            } else {
                None
            };
            let stream = opts.abortable || pipe.is_some();
            // The job's inflight unit is released when the worker consumes the reply it posts.
            shared.inflight.fetch_add(1, Ordering::SeqCst);
            let job_shared = shared.clone();
            let queued_at = Instant::now();
            submit_model(
                shared,
                shared.lane_of(slot.agent.cfg.model.as_deref().unwrap_or("default"), rank.unwrap_or(0)),
                id.clone(),
                incarnation,
                task,
                opts.priority,
                Box::new(move || {
                    let (started, at_ms) = (Instant::now(), job_shared.clock_ms());
                    let queue_ms = queued_at.elapsed().as_secs_f64() * 1000.0;
                    let observer = job_shared
                        .cfg
                        .observer
                        .clone()
                        .filter(|_| job_shared.cfg.observe_deltas);
                    let mut pipe = pipe;
                    // Stopped by its signal: what was generated before is the reply.
                    let mut aborted = abort.load(Ordering::SeqCst);
                    // A client that panics fails this call, which the agent hears as a model error.
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        if aborted {
                            return Ok(super::model::GenReply::default());
                        }
                        if observer.is_none() && !stream {
                            return model.generate(&req);
                        }
                        model.generate_stream(&req, &mut |delta| {
                            if let Some(pipe) = &mut pipe {
                                pipe.write(delta);
                            }
                            if let Some(f) = &observer {
                                (f.0)(&req.agent, task, delta.into());
                            }
                            aborted = abort.load(Ordering::SeqCst);
                            !aborted
                        })
                    }))
                    .unwrap_or_else(|panic| {
                        Err(super::model::ModelError::fatal(format!(
                            "the model client {}",
                            panicked(panic)
                        )))
                    });
                    // A call the host cancelled (the run closing) is not the model's failure:
                    // nothing is journaled, the call stays outstanding, and a restore issues it again.
                    if result.is_err() && req.cancel.cancelled_by_host() {
                        job_shared.done(&req.agent, incarnation, task);
                        job_shared.inflight.fetch_sub(1, Ordering::SeqCst);
                        return;
                    }
                    let kind = match result {
                        Ok(r) => {
                            let ms = started.elapsed().as_secs_f64() * 1000.0;
                            let stats = json!({"t": "generate", "task": task, "at_ms": at_ms, "prompt_tokens": req.ids.len(), "tokens": r.ids.len(), "ms": ms, "harness_queue_ms": queue_ms,
                            "first_token_ms": r.first_token_ms, "tokens_per_s": r.ids.len() as f64 / (ms / 1000.0).max(1e-9),
                            "cached_tokens": r.cached_tokens, "cut_off": r.cut_off, "aborted": aborted, "rank": req.rank, "tag": req.tag,
                            "model_meta": r.model_meta, "request_id": r.request_id, "attempts": r.attempts});
                            InputKind::Reply {
                                task,
                                ids: r.ids,
                                cut_off: r.cut_off,
                                aborted,
                                stats,
                                prompt: prompt_hash.unwrap_or_else(|| super::agent::prompt_hash(&req.ids)),
                            }
                        }
                        // A reply partly written to a pipe cannot be started again: whoever reads
                        // the pipe would get its beginning twice. The app gets the error.
                        Err(e) if pipe.as_ref().is_some_and(|p| p.wrote) => InputKind::ModelError {
                            task,
                            kind: super::model::FailureKind::Partial,
                            retry_after_ms: None,
                            error: format!(
                                "{e}, after part of the reply was written to its pipe ({}ms)",
                                started.elapsed().as_millis()
                            ),
                        },
                        Err(e) => InputKind::ModelError {
                            task,
                            kind: e.kind,
                            retry_after_ms: e.retry_after.map(|d| d.as_millis() as u64),
                            error: format!("{e} ({}ms)", started.elapsed().as_millis()),
                        },
                    };
                    job_shared.done(&req.agent, incarnation, task);
                    job_shared.answer(
                        w,
                        Msg::Input(
                            req.agent,
                            InputKind::Completion {
                                incarnation,
                                input: Box::new(kind),
                            },
                        ),
                    );
                }),
            );
        }
        Command::Call { task, name, args } => {
            // Held requests remain outstanding and are reissued by journal recovery.
            if shared.closing.load(Ordering::SeqCst) {
                return;
            }
            let reply = |local: &mut VecDeque<Msg>, outcome: Outcome| {
                push_local(shared, local, (id.clone(), InputKind::Result { task, outcome }))
            };
            if name != "inspect" && !name.starts_with("machine:") && !slot.spec.allows(&name) {
                return reply(local, Outcome::Error(format!("this agent may not call {name}")));
            }
            match name.as_str() {
                "send" => {
                    if let Err(error) = delivery::send(shared, &id, incarnation, task, &args) {
                        reply(local, Outcome::Error(error));
                    }
                }
                // A copy of other instances' state, made on their workers between their inputs.
                "inspect" => {
                    if slot.spec.inspect.is_empty() {
                        return reply(
                            local,
                            Outcome::Error("this agent may not call inspect (its spec grants no `inspect`)".into()),
                        );
                    }
                    let select = match super::inspect::parse_select(&args["select"]) {
                        Ok(s) => s,
                        Err(e) => return reply(local, Outcome::Error(e)),
                    };
                    let text = match &args["as"] {
                        J::Null => false,
                        J::String(a) if a == "text" => true,
                        _ => return reply(local, Outcome::Error("inspect: `as` is \"text\" or absent".into())),
                    };
                    let star = args["targets"] == "*";
                    let route = shared.route.read().unwrap();
                    let ids: Vec<String> = match &args["targets"] {
                        _ if star => {
                            let mut ids: Vec<String> = route
                                .keys()
                                .filter(|t| **t != id && slot.spec.may_inspect(t))
                                .cloned()
                                .collect();
                            ids.sort();
                            ids
                        }
                        J::String(t) => vec![t.clone()],
                        J::Array(a) if a.iter().all(J::is_string) => {
                            a.iter().map(|t| t.as_str().unwrap().to_string()).collect()
                        }
                        _ => {
                            return reply(
                                local,
                                Outcome::Error("inspect(targets): an id, a list of ids, or \"*\"".into()),
                            );
                        }
                    };
                    if let Some(t) = ids.iter().find(|t| !slot.spec.may_inspect(t)) {
                        return reply(
                            local,
                            Outcome::Error(format!(
                                "this agent may not inspect {t} (its spec's `inspect` does not name it)"
                            )),
                        );
                    }
                    let mut by_worker: BTreeMap<usize, Vec<String>> = BTreeMap::new();
                    let mut parts = BTreeMap::new();
                    for t in ids {
                        match route.get(&t) {
                            Some(&tw) => by_worker.entry(tw).or_default().push(t),
                            None => {
                                parts.insert(t.clone(), (json!({"error": format!("no instance {t}")}), None));
                            }
                        }
                    }
                    drop(route);
                    let g = Arc::new(Gather {
                        caller: id.clone(),
                        incarnation,
                        worker: w,
                        task,
                        select,
                        since: args["since"].clone(),
                        text,
                        star,
                        pending: AtomicUsize::new(by_worker.len().max(1)),
                        parts: Mutex::new(parts),
                    });
                    if by_worker.is_empty() {
                        answer_inspect(shared, &g)
                    }
                    for (tw, ids) in by_worker {
                        shared.post(tw, Msg::Inspect(g.clone(), ids));
                    }
                }
                n if n.starts_with("store.") => {
                    let before = (n == "store.put").then(|| shared.store.file_len(&id)).flatten();
                    let outcome = shared.store.call(&id, n, &args);
                    // A put's record is committed with the journals: synced and recorded in the
                    // cut before any line that learns its key, and cut back with them.
                    if let Some((path, was)) = before
                        && let Some((_, len)) = shared.store.file_len(&id)
                        && len != was
                    {
                        shared.durable.stored(&id, incarnation, &path, len, was == 0);
                    }
                    reply(local, outcome.map_or_else(Outcome::Error, Outcome::Ok))
                }
                // A machine call, checked against the agent's bindings here too: the host, not the
                // agent, holds the authority.
                n if n.starts_with("machine:") => match shared
                    .machines
                    .get(&n["machine:".len()..])
                    .cloned()
                    .filter(|_| slot.agent.cfg.machines.iter().any(|m| *m == n["machine:".len()..]))
                {
                    None => reply(
                        local,
                        Outcome::Error(format!("no machine {} is bound to this agent", &n["machine:".len()..])),
                    ),
                    Some(exec) => {
                        let ctx = CallContext {
                            agent: id.clone(),
                            task,
                            env: slot.agent.cfg.env.clone(),
                            cancel: shared.token(&id, incarnation, task),
                            blobs: shared.store.blobs().cloned(),
                            // A command's output as it is written, for live views.
                            output: shared.cfg.observer.clone().map(|f| {
                                let agent = id.clone();
                                super::exec::Output(Arc::new(move |bytes: &[u8]| {
                                    (f.0)(&agent, task, Observed::Output(bytes))
                                }))
                            }),
                        };
                        // Listed (above), then checked, as a model call is.
                        if shared.closing.load(Ordering::SeqCst) {
                            shared.done(&id, incarnation, task);
                            return;
                        }
                        let limit = shared.cfg.call_timeout;
                        shared.inflight.fetch_add(1, Ordering::SeqCst);
                        let job_shared = shared.clone();
                        shared.exec_pool.submit(Box::new(move || {
                            let (agent, token) = (ctx.agent.clone(), ctx.cancel.clone());
                            // Executors get text, never references; a large result goes to disk here,
                            // on the executor's thread, before it is queued for the agent.
                            let blobs = job_shared.store.blobs().cloned();
                            let outcome = match blobs.as_ref().map_or_else(|| Ok(args.clone()), |b| b.expand(&args)) {
                                Ok(args) => super::exec::with_deadline(move || exec.call(&ctx, &args), limit, &token),
                                Err(e) => Err(e),
                            };
                            // A call the host cancelled (the run closing) is not the app's
                            // failure: no result is journaled, the call stays outstanding, and a
                            // restore issues it again.
                            if outcome.is_err() && token.cancelled_by_host() {
                                job_shared.done(&agent, incarnation, task);
                                job_shared.inflight.fetch_sub(1, Ordering::SeqCst);
                                return;
                            }
                            let outcome = match (outcome, &blobs) {
                                (Ok(mut v), Some(b)) => b.lower(&mut v).map(|_| v),
                                (other, _) => other,
                            }
                            .map_or_else(Outcome::Error, Outcome::Ok);
                            job_shared.done(&agent, incarnation, task);
                            job_shared.answer(
                                w,
                                Msg::Input(
                                    id,
                                    InputKind::Completion {
                                        incarnation,
                                        input: Box::new(InputKind::Result { task, outcome }),
                                    },
                                ),
                            );
                        }));
                    }
                },
                other => reply(local, Outcome::Error(format!("{other} is not a harness API"))),
            }
        }
    }
}

#[cfg(test)]
mod containment_tests {
    use super::super::model::{FnModel, GenReply};
    use super::*;

    /// `panics`: "start" panics in the first render; true panics in the render after a 20 ms
    /// wait, as a timer's input; otherwise the agent stops after the wait.
    const APP: &str = r#"
export default function Main({panics}) {
  const [n, setN] = useState(0, "n");
  useEffect(() => { harness.wait({ms: 20}).then(() => { if (panics) setN(1); else harness.stop({n: 1}); }); }, []);
  if (panics === "start" || n == 1) __testPanic("render failed");
  return "n=" + n;
}"#;

    struct Scratch(PathBuf);
    impl Scratch {
        fn new(name: &str) -> Scratch {
            let d = std::env::temp_dir().join(format!("aff-containment-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&d);
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join("app.tsx"), APP).unwrap();
            Scratch(d)
        }
        fn config(&self, workers: usize) -> RuntimeConfig {
            let mut cfg = RuntimeConfig::new(self.0.join("app.tsx"));
            (cfg.workers, cfg.model_threads, cfg.exec_threads) = (workers, 1, 1);
            cfg.max_wall = Some(Duration::from_secs(60));
            cfg.dir = Some(self.0.join("run"));
            cfg
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn encoder() -> Arc<Encoder> {
        Arc::new(Encoder::from_file(crate::test_support::path().to_str().unwrap()).unwrap())
    }
    fn model() -> Arc<dyn Model> {
        Arc::new(FnModel(|_: &GenRequest| Ok(GenReply::default())))
    }
    fn spec(id: &str, props: J) -> AgentSpec {
        let mut s = AgentSpec::new(id, props);
        s.model = None;
        s
    }
    fn agent<'a>(report: &'a Report, id: &str) -> &'a AgentReport {
        report.agents.iter().find(|a| a.id == id).unwrap()
    }

    /// A render that panics on a timer's input fails its agent alone: the worker it shares with
    /// a healthy agent goes on, and the run completes at once rather than at `max_wall` (a reply
    /// parser's panic once stalled a whole run until its stop deadline). The
    /// panicking input is journaled, so a restore replays it and fails the agent again, and
    /// nothing else.
    #[test]
    fn a_panicking_agent_fails_alone_and_again_on_restore() {
        let scratch = Scratch::new("handle");
        let cfg = scratch.config(1);
        let mut rt = Runtime::new(cfg.clone(), encoder(), model());
        rt.add_agent(spec("crash", json!({"panics": true})))
            .unwrap()
            .add_agent(spec("healthy", json!({"panics": false})))
            .unwrap();
        let started = Instant::now();
        let report = rt.run().unwrap();
        assert!(started.elapsed() < Duration::from_secs(20), "{:?}", started.elapsed());
        assert!(report.complete);
        assert_eq!(
            agent(&report, "crash").error.as_deref(),
            Some("panicked: render failed")
        );
        assert_eq!(
            (
                agent(&report, "healthy").error.as_deref(),
                agent(&report, "healthy").stopped.clone()
            ),
            (None, Some(json!({"n": 1})))
        );
        let report = Runtime::restore(cfg, encoder(), model()).unwrap().run().unwrap();
        assert_eq!(
            agent(&report, "crash").error.as_deref(),
            Some("restore failed: panicked: render failed")
        );
        assert_eq!(agent(&report, "healthy").error, None);
    }

    /// An agent that cannot start, by an error (props that are not an object) or a panic in its
    /// first render, fails alone; its worker starts and runs the others.
    #[test]
    fn an_agent_that_cannot_start_fails_alone() {
        let scratch = Scratch::new("boot");
        let mut rt = Runtime::new(scratch.config(1), encoder(), model());
        rt.add_agent(spec("bad", json!("not an object")))
            .unwrap()
            .add_agent(spec("crash", json!({"panics": "start"})))
            .unwrap()
            .add_agent(spec("healthy", json!({"panics": false})))
            .unwrap();
        let started = Instant::now();
        let report = rt.run().unwrap();
        assert!(started.elapsed() < Duration::from_secs(20), "{:?}", started.elapsed());
        assert!(
            agent(&report, "bad")
                .error
                .as_deref()
                .unwrap()
                .contains("props must be an object"),
            "{:?}",
            agent(&report, "bad").error
        );
        assert_eq!(
            agent(&report, "crash").error.as_deref(),
            Some("panicked: render failed")
        );
        assert_eq!(
            (
                agent(&report, "healthy").error.as_deref(),
                agent(&report, "healthy").stopped.clone()
            ),
            (None, Some(json!({"n": 1})))
        );
        assert_eq!(report.agents.len(), 3);
    }

    /// A worker that dies outside its agents' code (here a model binding that panics while an
    /// agent starts) ends the run at once: its agents are failed, the other worker's reported.
    #[test]
    fn a_dead_worker_ends_the_run_with_every_report() {
        struct Broken;
        impl Model for Broken {
            fn template(&self) -> Option<super::super::model_config::ModelTemplate> {
                panic!("a broken binding")
            }
            fn generate(&self, _: &GenRequest) -> Result<GenReply, super::super::model::ModelError> {
                unreachable!()
            }
        }
        let scratch = Scratch::new("worker");
        let mut rt = Runtime::new(scratch.config(2), encoder(), model());
        rt.bind_model("broken", encoder(), Arc::new(Broken));
        let mut dies = spec("dies", json!({"panics": false}));
        dies.model = Some("broken".into());
        rt.add_agent(dies)
            .unwrap()
            .add_agent(spec("healthy", json!({"panics": false})))
            .unwrap();
        let started = Instant::now();
        let report = rt.run().unwrap();
        assert!(started.elapsed() < Duration::from_secs(20), "{:?}", started.elapsed());
        assert!(!report.complete);
        assert_eq!(
            agent(&report, "dies").error.as_deref(),
            Some("render worker 0 panicked: a broken binding")
        );
        assert_eq!(agent(&report, "healthy").error, None);
    }
}
