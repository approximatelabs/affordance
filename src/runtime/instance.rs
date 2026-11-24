//! One agent: a deterministic state machine with no I/O (the browser's "tab").
//!
//! The agent owns its app state (props and `useState` hooks) and its tasks. Everything that
//! happens to it arrives as an [`Input`] (a model reply, an effect result, an event, a clock
//! tick). Inputs are journaled; replaying the journal through a new agent reproduces every render,
//! task and state write. The host (`runtime::Runtime`, or a test loop) executes the [`Command`]s
//! the agent returns.
//!
//! The agent keeps no conversation. An app calls
//! `model(view?, opts)` (the browser's `fetch`): the page's own view, or an element, laid out at
//! the call and sent to the instance's model; the reply is parsed against the view's affordances
//! and returned as data. `dispatch(output)` runs its calls on the handlers of the view the model
//! saw; each tool answers when its body returns. A log of turns is app state (`views/lib/log.tsx`).
//!
//! With `set_blobs`, strings of at least the blob size are kept as blob references (`blobs.rs`)
//! wherever the agent keeps a value: inputs before they are journaled, handler writes and
//! results, call arguments. Writing a blob is the agent's only I/O; blobs
//! are content-addressed, so replaying the journal writes nothing new and reads the same text.
use super::blobs::Blobs;
use crate::View;
use crate::effects::{Outcome, Request};
use crate::emit::{Encoder, RunCache};
use crate::reply::{ActionSet, Problem, Syntax, parse_reply_typed};
use crate::value::{HostResolver, Map, Value, with_host_text};
use serde::{Deserialize, Serialize};
use serde_json::{Value as J, json};
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::rc::Rc;
use std::sync::Arc;
mod images;
mod model_calls;
mod outstanding;
use images::*;
pub(crate) use images::{clip_source, image_source, of_kind};
pub use outstanding::OutstandingSummary;
use outstanding::*;

#[path = "instance_checkpoint.rs"]
pub(crate) mod recovery;

pub(crate) fn valid_instance_name(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('.') && !name.contains(['/', '\\']) && !name.chars().any(char::is_control)
}

/// Everything that can happen to an agent. Journaled in order; the journal is the agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "input", rename_all = "snake_case")]
pub enum Input {
    /// One committed render frame. Members retain their order and microtask checkpoints.
    Batch {
        #[serde(deserialize_with = "deserialize_batch")]
        /// The frame's inputs, in order (events and results).
        inputs: Vec<Input>,
    },
    /// The model's generated ids for task `task` (including its stop token, if any). `prompt`:
    /// `prompt_hash` of the ids the model was given; replay checks the app renders the same.
    Reply {
        /// The task the reply answers.
        task: u64,
        /// The generated ids.
        ids: Vec<u32>,
        /// Generation stopped at its token limit.
        cut_off: bool,
        /// When the reply arrived, on the instance's clock.
        clock_ms: i64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        /// `prompt_hash` of the prompt the model answered.
        prompt: Option<String>,
        /// The call was stopped by its signal (`Command::Abort`): `ids` is what it generated first.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        aborted: bool,
        /// What the server and the client measured, as a fetch's Resource Timing does:
        /// `{cached_tokens, ms, first_token_ms, wall_ms}` (the prefix the server reported reusing,
        /// or null; the call's duration and time to its first token; the reply's arrival, in
        /// milliseconds since the Unix epoch). Journaled, so an app reads the same on replay; the
        /// model call's output carries it as `server`. Absent in journals from before it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        server: Option<J>,
    },
    /// The model call for task `task` failed: one attempt, and what kind of failure it was
    /// (`runtime::model::FailureKind`). Whether it is made again is the retry policy's.
    ModelError {
        /// The task whose call failed.
        task: u64,
        /// What failed.
        error: String,
        /// What kind of failure it was.
        kind: super::model::FailureKind,
        /// How long the server asked the caller to wait before trying again.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        retry_after_ms: Option<u64>,
        /// When the failure arrived, on the instance's clock.
        clock_ms: i64,
    },
    /// The outcome of a `Call` command.
    Result { task: u64, outcome: Outcome, clock_ms: i64 },
    /// Something from outside (a message from another agent, a post, a process exit, ...).
    Event {
        /// The event's name, as `useEvent(name)` hears it.
        name: String,
        /// What it carries.
        payload: J,
        /// Its delivery id, when it has one (a `send`'s message, a keyed post): an event whose id
        /// this instance has already taken is journaled but not taken again.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// When it arrived, on the instance's clock.
        clock_ms: i64,
    },
    /// The clock moved (wakes timed waits).
    Tick { clock_ms: i64 },
    /// The parent re-rendered this instance's `<Mount>` with new props (its callbacks included).
    Props { props: J, clock_ms: i64 },
}

impl Input {
    /// The input's kind, as journaled (`reply`, `result`, `event`, ...).
    pub fn kind_name(&self) -> &'static str {
        match self {
            Input::Batch { .. } => "batch",
            Input::Reply { .. } => "reply",
            Input::ModelError { .. } => "model_error",
            Input::Result { .. } => "result",
            Input::Event { .. } => "event",
            Input::Tick { .. } => "tick",
            Input::Props { .. } => "props",
        }
    }
    /// When the input happened, on the instance's clock.
    pub fn clock_ms(&self) -> i64 {
        match self {
            Input::Batch { inputs } => inputs.iter().map(Input::clock_ms).max().unwrap_or(0),
            Input::Reply { clock_ms, .. }
            | Input::ModelError { clock_ms, .. }
            | Input::Result { clock_ms, .. }
            | Input::Event { clock_ms, .. }
            | Input::Tick { clock_ms }
            | Input::Props { clock_ms, .. } => *clock_ms,
        }
    }
}

/// How many inputs pass between the step boundaries an instance keeps for a recovery image.
const BOUNDARY_GAP: usize = 16;

/// The most inputs one committed frame holds.
pub const MAX_INPUT_BATCH: usize = 64;
fn validate_batch(inputs: &[Input]) -> Result<(), String> {
    if inputs.is_empty()
        || inputs.len() > MAX_INPUT_BATCH
        || inputs
            .iter()
            .any(|input| !matches!(input, Input::Event { .. } | Input::Result { .. }))
    {
        return Err("an input batch requires 1..64 event/result members and no nested batches".into());
    }
    Ok(())
}
fn deserialize_batch<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Vec<Input>, D::Error> {
    let inputs = Vec::<Input>::deserialize(deserializer)?;
    validate_batch(&inputs).map_err(serde::de::Error::custom)?;
    Ok(inputs)
}

/// A model call's own options (`harness.render(<App/>, {max_new, sampling, tag, priority})`); a
/// turn uses the defaults.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GenOptions {
    /// Merged over the model binding's sampling, e.g. `{"temperature": 0.2}`.
    #[serde(default)]
    pub sampling: J,
    /// Recorded with the request's capture and trace, e.g. `"summary"`.
    #[serde(default)]
    pub tag: Option<String>,
    /// Admitted ahead of the model calls waiting in its lane (e.g. a fold that blocks a turn).
    #[serde(default)]
    pub priority: bool,
    /// The call has an `AbortSignal`: it is generated as a stream, which `Command::Abort` can stop.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub abortable: bool,
    /// `model(view, {pipe})`: where the reply's pieces go as they are generated, and in what
    /// frame: `{machine, call, text?, reasoning?, tool_call?}` (`docs/RUNTIME.md`, Serving over
    /// HTTP). None: no pipe. The reply the app receives is the same either way.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pipe: Option<Box<J>>,
}

/// What the agent asks its host to do.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum Command {
    /// Generate from `ids` until a `stop` token or `max_new` tokens; answer with `Input::Reply`.
    /// `ids` is shared (one copy of a long prompt between the agent, the command and the request).
    /// `prompt_hash`: `prompt_hash(ids)` when the agent has it (a turn's, computed incrementally).
    /// `images`: the prompt's `<img>` sources, in order (sent with the ids).
    Generate {
        /// The task to answer with `Input::Reply`.
        task: u64,
        /// The prompt.
        ids: Arc<[u32]>,
        /// Token ids that end the reply.
        stop: Vec<u32>,
        /// The most tokens to generate.
        max_new: usize,
        #[serde(default)]
        /// The call's options.
        opts: GenOptions,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        /// `prompt_hash(ids)`, when the agent already has it.
        prompt_hash: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        /// The prompt's image sources, in order.
        images: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        /// Where each media element's tokens are in `ids`, and its placeholder: (start, count,
        /// placeholder).
        media: Vec<(usize, usize, u32)>,
    },
    /// Run primitive `name` (`exec`, `send`, `store.get`, a deployment executor, ...); answer
    /// with `Input::Result`. The host fills in the caller's identity; the app cannot.
    Call { task: u64, name: String, args: J },
    /// Send `Input::Tick` at (or after) this clock time.
    Timer { at_ms: i64 },
    /// The result of `task` is no longer wanted: abort the work (kill the process, close the
    /// request). A result that still arrives is ignored.
    Cancel { task: u64 },
    /// Stop the model call `task` now and answer with what it generated so far (`Input::Reply`
    /// with `aborted`), unless its whole reply is already on the way. The app aborted its signal.
    Abort { task: u64 },
    /// Start the child instance `key` (this instance's `<Mount>`): component `entry` of the file
    /// `app`, its props, its model binding, and machines (a subset of this instance's).
    Mount {
        /// The child's key under this instance.
        key: String,
        /// The file its app is in.
        app: String,
        /// The component to run.
        entry: String,
        props: J,
        /// The model binding it calls, if not its parent's.
        model: Option<String>,
        /// The machine roles it may use.
        machines: Vec<String>,
    },
    /// The child `key`'s props changed.
    Props { key: String, props: J },
    /// Stop the child `key`. Mounting it again creates a new incarnation.
    Unmount { key: String },
    /// Call the callback prop `name` the parent passed: an event in the parent (not a task: it
    /// is delivered even if the calling handler finishes or the instance stops).
    Callback { name: String, args: J },
}

/// Fixed per-agent configuration (not journaled: supplied again on restore).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceConfig {
    /// The agent's id: its name in events, traces and its directory.
    pub id: String,
    /// The app's root props (its starting state).
    pub props: J,
    /// The app-scoped environment (`harness.env`): plain and secret keys alike.
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    /// The most tokens a reply may have, unless a call says otherwise.
    pub max_new: usize,
    /// Stop after this many model turns.
    #[serde(default)]
    pub max_turns: Option<u64>,
    /// The hard prompt ceiling (the model's context): a prompt longer than this after the app's
    /// own budget handler is never sent; the instance stops (`prompt_over_limit`).
    #[serde(default)]
    pub max_prompt: Option<usize>,
    /// The clock the agent starts at, in milliseconds.
    pub start_ms: i64,
    #[serde(default = "default_seed")]
    /// The seed of its random sequence.
    pub seed: u64,
    /// The roles of the machines bound to this agent (`harness.machines`); the first is
    /// `harness.machine`. An agent reaches no machine it is not bound to.
    #[serde(default)]
    pub machines: Vec<String>,
    /// The model binding this instance takes turns on; `None`: it takes none (an app that only
    /// reacts: a root that mounts others, a coordinator).
    #[serde(default = "default_model")]
    pub model: Option<String>,
    /// Resolved binding presentation, supplied again for replay and checkpoint restore.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<super::model::ModelTemplate>,
    /// The binding's context length: less `max_new`, every prompt's window.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<usize>,
    /// What happens after a model call of this instance fails (its binding's `retry`).
    #[serde(default)]
    pub retry: super::model::RetryPolicy,
}

fn default_model() -> Option<String> {
    Some("default".into())
}

fn default_seed() -> u64 {
    1
}

#[derive(Debug, Clone)]
enum Origin {
    /// A call from a model's reply, run by `dispatch(output)`: its dispatch and its place in the
    /// reply's calls.
    Tool {
        name: String,
        call_id: String,
        dispatch: u64,
        index: usize,
        started_ms: i64,
    },
    /// An `onCallback` handler for an event.
    Callback {
        event: String,
    },
    /// A `useEffect` setup, or the cleanup an earlier setup returned.
    Effect {
        key: String,
    },
    Cleanup {
        key: String,
    },
}
impl Origin {
    /// As a trace names it: `tool shell`, `effect chat~0`.
    fn label(&self) -> String {
        match self {
            Origin::Tool { name, .. } => format!("tool {name}"),
            Origin::Callback { event } => format!("callback {event}"),
            Origin::Effect { key } => format!("effect {key}"),
            Origin::Cleanup { key } => format!("cleanup {key}"),
        }
    }
}

#[derive(Debug, Clone)]
struct Handler {
    origin: Origin,
    action: String,
    which: usize,
    args: J,
    /// The state and `harness` context the handler's render saw.
    state: Rc<Map>,
    ctx: Value,
    clock_ms: i64,
    /// Only requests still awaiting a host or runtime completion.
    effects: BTreeMap<usize, Request>,
    next_effect: usize,
    /// The render it runs from (its turn's, or one made when it started): every resumption after
    /// an effect runs from it, without rendering again.
    prepared: Option<Rc<crate::Prepared>>,
    /// The closure it runs instead of an affordance's handler (an effect setup or cleanup).
    closure: Option<Value>,
    /// Journal positions: the render it runs from, and the last event a wait of it consumed. A
    /// `wait` wakes on a matching event journaled after both (one the model had not seen).
    at: u64,
    consumed: u64,
    /// Its body returned this, but work it started is still pending: it lives until that settles.
    done: Option<J>,
}

/// A model call in flight (`model(view?, opts)`), by the task slot that awaits it.
#[derive(Debug, Clone)]
struct ModelCall {
    /// A call on the page's own view (its number is a turn), or on an element (ad hoc).
    page: bool,
    no: u64,
    /// `prompt_hash(prompt)` and the hasher that produced it (continued over the reply): replay
    /// checks a page call's prompt against it.
    hash: String,
    hasher: sha2::Sha256,
    prompt: Arc<[u32]>,
    actions: ActionSet,
    syntax: Syntax,
    in_think: bool,
    stop: Vec<u32>,
    /// The screen record's fields, when the view has a current turn: its ids and images
    /// (`{at, n, src}`), or for a hosted prompt the content sent.
    screen: Option<J>,
    usage: J,
}

/// A page call's output kept for dispatch: the view its calls' handlers run from and the context
/// they run with, where it was issued (`view`, dropped once every call has been started), the
/// reply's calls `(name, args, id, problem)`, which have been started, and each call's result once
/// it has answered (a later dispatch of the same call returns it).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Kept {
    #[serde(skip)]
    view: Option<(Rc<crate::Prepared>, Rc<Map>, Value, i64, u64)>,
    calls: Option<Vec<(String, J, String, Option<String>)>>,
    #[serde(default)]
    started: BTreeSet<usize>,
    #[serde(default)]
    results: Vec<Option<J>>,
}

/// A running `dispatch(output, {calls})`: the output it is of, the slots awaiting it, the calls it
/// answers for (in the order asked), and whether its calls are still being started. It resolves
/// once each of its calls has a result, whichever dispatch started it.
#[derive(Debug, Clone)]
struct Dispatch {
    call: u64,
    waiters: Vec<(u64, usize)>,
    indices: Vec<usize>,
    starting: bool,
}

/// Page outputs kept for dispatch: the newest this many (their results, once dispatched), of
/// which the newest `KEPT_VIEWS` not yet dispatched keep their views.
const KEPT_OUTPUTS: usize = 64;
const KEPT_VIEWS: usize = 8;

#[derive(Debug, Clone)]
/// `harness.wait({ms, events})`: done at `until_ms` (`i64::MAX`: never), or early on an event in `wake_on`.
struct Waiting {
    until_ms: i64,
    wake_on: Vec<String>,
}

/// Per-agent counters, also reported in the trace.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Metrics {
    /// Page calls made.
    pub turns: u64,
    /// Prompt tokens of every model call: page calls and ad-hoc `model(<View/>)` calls.
    pub prompt_tokens: u64,
    /// Tokens the model generated.
    pub generated_tokens: u64,
    /// Turns whose prompt started with the previous prompt plus its generated ids.
    pub exact_appends: u64,
    /// Turns where it did not (folds, or a divergence).
    pub rewrites: u64,
    /// Prompt tokens that were not a prefix of the previous page prompt + reply (uncached work);
    /// an ad-hoc call's all count, so reuse computed from this is a lower bound.
    pub uncached_tokens: u64,
    /// Handlers that failed.
    pub handler_errors: u64,
    /// Model calls that failed.
    pub model_errors: u64,
    /// The longest prompt sent, in tokens.
    pub max_prompt: usize,
}

/// Diagnostic work performed in this process, including replay; not persisted replay metrics.
#[derive(Debug, Clone, Default, Serialize)]
pub struct RenderStats {
    /// Documents evaluated.
    pub evaluations: u64,
    /// Renders answered from the last evaluated document.
    pub reused: u64,
    /// Time spent evaluating documents (microseconds).
    pub render_us: u64,
    /// Documents laid out (a prompt, a model-chosen action), and the time it took.
    pub layouts: u64,
    /// Time spent laying documents out, in microseconds.
    pub layout_us: u64,
}

/// A model call's outcome, marked as stopped by its signal.
fn marked_aborted(outcome: Outcome) -> Outcome {
    match outcome {
        Outcome::Ok(mut value) => {
            value["aborted"] = json!(true);
            Outcome::Ok(value)
        }
        error => error,
    }
}

type EffectState = BTreeMap<String, (Value, Option<(Rc<crate::Prepared>, Value)>)>;

/// Recent events kept for `wait`s that start after them.
const EVENT_LOG: usize = 1024;

/// Passes of render and effects in one input before the instance stops (`max_update_depth`).
const MAX_UPDATE_DEPTH: usize = 50;

struct RenderSnapshot {
    version: u64,
    app: String,
    tokenizer: String,
    context: J,
    reads: crate::observe::Reads,
    prepared: Rc<crate::Prepared>,
}

/// One instance: an app's state and its event loop. It takes inputs (`handle`) and returns the commands its host runs; it does no I/O itself, so the same inputs rebuild it exactly.
/// One open effect of a handler, all of it in one record. A handler's effect (a machine call, a
/// wait, a model call) is open from the moment it is issued until `resolve` answers it; the
/// record is removed there, when its handler finishes, and when the instance stops. Nothing about
/// an effect lives anywhere else, so none of it can be left behind: a wait with no handler, a
/// retry for a call that was answered, a model call that keeps the instance looking busy.
#[derive(Default)]
pub(crate) struct Open {
    /// The abort signal it is bound to (`{signal}`), until that signal fires.
    signal: Option<u64>,
    /// `harness.wait`: until when, and the events that end it sooner.
    wait: Option<Waiting>,
    /// A page model call: what was asked, for reading its reply. An ad-hoc view's call has none.
    model: Option<ModelCall>,
    /// A model call whose last try failed and is tried again.
    retry: Option<Retry>,
    /// The call's own retry policy (`model(view, {retry: ..})`); none: its binding's.
    policy: Option<super::model::RetryPolicy>,
}

/// A model call being tried again after a failure.
pub(crate) struct Retry {
    /// When the next try is due; `None` while that try is out.
    at: Option<i64>,
    /// Consecutive failures so far, and the last one's kind.
    failures: u32,
    kind: &'static str,
    /// The `Generate` to send again.
    cmd: Command,
    /// The instance clock at the first failure: what the policy's `for_ms` is counted from.
    since: i64,
}

pub struct Instance {
    realm: crate::microtasks::Realm,
    /// The handlers run since the last round of jobs ended, as a trace names them: where an
    /// unhandled rejection is reported from, also after its handler has finished.
    ran: HashMap<u64, String>,
    /// Its configuration.
    pub cfg: InstanceConfig,
    /// The app's state: its props and, under `__ui`, its hooks, kept as language values (shared,
    /// never converted to JSON on the way to a render or a task).
    state: Rc<Map>,
    /// The environ as a value (fixed for the agent's life).
    env_v: Value,
    clock_ms: i64,
    rng: u64,
    /// Its abort signals (`AbortController`), the model or machine call awaiting each slot bound
    /// to one, and the model calls told to stop (`Command::Abort`) that have not answered yet.
    signals: crate::effects::Signals,
    aborting: BTreeSet<u64>,
    /// What each open effect of a handler is waiting for, by its slot (handler, effect index):
    /// see `Open`.
    open: BTreeMap<(u64, usize), Open>,
    /// Page views kept for dispatch, by call number; dispatches running.
    kept: BTreeMap<u64, Kept>,
    dispatches: BTreeMap<u64, Dispatch>,
    next_dispatch: u64,
    handlers: BTreeMap<u64, Handler>,
    /// Outstanding host work: task -> (handler, effect index), or the model turn.
    tasks: BTreeMap<u64, (Option<(u64, usize)>, Command)>,
    /// Events not yet delivered to callbacks (delivered when the input that brought them ends).
    /// Events to deliver at the end of the input, with the document whose listeners hear them
    /// when it is not the current one (a prompt's `layout` event goes to the prompt's listeners).
    events: VecDeque<(String, J, Option<Rc<crate::Prepared>>)>,
    /// Recent events with their journal positions: a `wait` wakes on one journaled after the
    /// render its task runs from (the model had not seen it).
    event_log: VecDeque<(u64, String, J)>,
    /// `useEffect`s by identity: their committed deps, and the cleanup their setup returned.
    effect_state: EffectState,
    /// State changes so far, and as of the last effects pass (no change: no render).
    version: u64,
    effects_version: u64,
    /// The latest complete document, shared by lifecycle, model calls and mounts while all
    /// render inputs still match. Probes never publish or reuse this snapshot.
    last_render: Option<RenderSnapshot>,
    next_task: u64,
    next_handler: u64,
    stopped: Option<J>,
    /// The prompt and reply of the last page call (for the exact-append check).
    last_full: Option<(Arc<[u32]>, Vec<u32>)>,
    /// A hasher over `last_full`'s ids: the next prompt's hash continues it when that prompt is an exact append.
    last_hasher: Option<sha2::Sha256>,
    /// Effect outcomes applied so far (each one's `settled` number).
    /// Token length of the last page prompt sent to the model (`harness.prompt_tokens`).
    last_prompt: usize,
    /// Its counters.
    pub metrics: Metrics,
    /// Render work done in this process.
    pub render_stats: RenderStats,
    /// Inputs applied so far. The journal itself is the host's (on disk); the agent keeps no copy.
    journaled: usize,
    /// For `harness.inspect`'s `since`: the journal position at which each hook (and the props,
    /// together) last changed, and the state those were noted against.
    changed_at: HashMap<Rc<str>, usize>,
    props_at: usize,
    noted: Rc<Map>,
    /// The latest step boundary, kept for a recovery image (`agent_checkpoint.rs`).
    boundary: Option<Rc<recovery::Source>>,
    trace: Vec<J>,
    /// Tasks dropped since the last `handle` (sent to the host as `Cancel`).
    cancelled: Vec<u64>,
    /// Mounted children by key: what this instance's last render declared (`Command::Mount`).
    children: BTreeMap<String, J>,
    /// Its parent has been told it stopped (`onStop`).
    stop_reported: bool,
    /// Delivery ids (`Input::Event.id`) of the events already taken: a resend after a crash is not taken twice.
    seen_msgs: std::collections::BTreeSet<String>,
    /// A journaled reply answered a prompt this app no longer renders: the first such turn.
    diverged_at: Option<u64>,
    /// Refuse (an error) instead of recording `diverged_at`.
    pub strict_replay: bool,
    cache: RunCache,
    /// Where strings of at least the runtime's spill size live (see `blobs.rs`); `None`: on the heap.
    blobs: Option<Arc<Blobs>>,
    resolver: Option<HostResolver>,
    /// The collection candidates of this instance's values. The last field: it is dropped after
    /// them, and frees the cycles they leave.
    pub(crate) heap: crate::heap::Written,
}

fn state_map(state: &J) -> Result<std::rc::Rc<crate::value::Map>, String> {
    match Value::from_json(state) {
        Value::Map(m) => {
            crate::value::freeze(&Value::Map(m.clone()));
            Ok(Rc::new(m.into_data()))
        }
        _ => Err("agent state must be an object".into()),
    }
}

/// The hooks of a state (`__ui`), or none yet.
fn hooks(state: &Map) -> Map {
    match state.get("__ui") {
        Some(Value::Map(m)) => m.read().clone(),
        _ => Map::default(),
    }
}

/// The same value: one shared allocation, or equal contents. Shared parts are compared by
/// pointer, so a task's changes are found without walking what it did not touch.
fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        // Equal contents count as the same (a setter that rebuilds an equal value changes nothing);
        // shared parts are not walked.
        (Value::List(x), Value::List(y)) => {
            Rc::ptr_eq(x, y)
                || (x.read().len() == y.read().len() && x.read().iter().zip(y.read().iter()).all(|(a, b)| same(a, b)))
        }
        (Value::Map(x), Value::Map(y)) => {
            Rc::ptr_eq(x, y)
                || (match (x.class(), y.class()) {
                    (None, None) => true,
                    (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                    _ => false,
                } && x.read().len() == y.read().len()
                    && x.read()
                        .iter()
                        .zip(y.read().iter())
                        .all(|((k, a), (l, b))| k == l && same(a, b)))
        }
        (Value::Frag(x), Value::Frag(y)) => Rc::ptr_eq(x, y),
        (Value::Lambda(x, e), Value::Lambda(y, f)) => Rc::ptr_eq(x, y) && Rc::ptr_eq(e, f),
        (Value::Promise(x), Value::Promise(y)) => Rc::ptr_eq(x, y),
        (Value::Str(crate::value::Text::Heap(x)), Value::Str(crate::value::Text::Heap(y))) => {
            Rc::ptr_eq(x, y) || x == y
        }
        (Value::Str(crate::value::Text::Host(x)), Value::Str(crate::value::Text::Host(y))) => {
            Rc::ptr_eq(x, y) || x.json() == y.json()
        }
        (Value::Coll(x), Value::Coll(y)) => Rc::ptr_eq(x, y),
        (Value::Regex(x), Value::Regex(y)) => Rc::ptr_eq(x, y),
        (Value::Func(x), Value::Func(y)) => Rc::ptr_eq(&x.definition, &y.definition),
        (Value::Setter(x), Value::Setter(y)) => x == y,
        (Value::Token(x, _), Value::Token(y, _)) => x == y,
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => x.to_bits() == y.to_bits(),
        (Value::BigInt(x), Value::BigInt(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::None, Value::None) | (Value::Undef, Value::Undef) => true,
        _ => false,
    }
}

fn date(ms: i64) -> Result<chrono::NaiveDateTime, String> {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|d| d.naive_utc())
        .ok_or("clock outside supported range".into())
}

/// A tool result for its record: strings as they are, `null` as empty, a list of content parts
/// (`{type: "text", text}`, `{type: "image_url", image_url: {url}}`, as chat APIs take them) as it
/// is, anything else as JSON text.
fn result_value(v: &J) -> J {
    match v {
        J::String(_) => v.clone(),
        J::Null => json!(""),
        J::Array(parts) if !parts.is_empty() && parts.iter().all(|p| p["type"].is_string()) => v.clone(),
        other => json!(other.to_string()),
    }
}

impl Instance {
    /// An agent for `cfg`, not yet started (`start`).
    pub fn new(cfg: InstanceConfig) -> Result<Instance, String> {
        if !cfg.props.is_object() {
            return Err("agent props must be an object".into());
        }
        if let Some(template) = &cfg.template {
            template.validate()?;
        }
        date(cfg.start_ms)?;
        let env_v = Value::from_json(&serde_json::to_value(&cfg.env).map_err(|e| e.to_string())?);
        Ok(Instance {
            state: Rc::new(Map::default()),
            env_v,
            clock_ms: cfg.start_ms,
            rng: cfg.seed.max(1),
            signals: Default::default(),
            aborting: BTreeSet::new(),
            open: BTreeMap::new(),
            kept: BTreeMap::new(),
            dispatches: BTreeMap::new(),
            next_dispatch: 0,
            handlers: BTreeMap::new(),
            tasks: BTreeMap::new(),
            events: VecDeque::new(),
            event_log: VecDeque::new(),
            effect_state: BTreeMap::new(),
            version: 1,
            effects_version: 0,
            last_render: None,
            next_task: 0,
            realm: crate::microtasks::Queue::new(),
            ran: HashMap::new(),
            next_handler: 0,
            stopped: None,
            last_full: None,
            last_hasher: None,
            last_prompt: 0,
            metrics: Metrics::default(),
            render_stats: RenderStats::default(),
            journaled: 0,
            changed_at: HashMap::new(),
            props_at: 0,
            noted: Rc::new(Map::default()),
            boundary: None,
            trace: vec![],
            cancelled: vec![],
            children: BTreeMap::new(),
            stop_reported: false,
            seen_msgs: Default::default(),
            diverged_at: None,
            strict_replay: false,
            cache: RunCache::new(true),
            blobs: None,
            resolver: None,
            heap: Default::default(),
            cfg,
        })
    }

    /// Rebuild an agent from its journal: every input is applied again, deterministically.
    /// Returns the agent and the commands still outstanding (the host submits them again;
    /// durable external work must be deduplicated by agent and task id).
    pub fn restore(
        cfg: InstanceConfig,
        journal: impl IntoIterator<Item = Input>,
        view: &View,
        enc: &Encoder,
    ) -> Result<(Instance, Vec<Command>), String> {
        Instance::restore_with(cfg, journal, view, enc, |_| {})
    }

    /// `restore`, with `setup` configuring the new agent before its journal is applied (blobs the
    /// journal refers to, strict replay). The journal is streamed: it is never all in memory.
    pub fn restore_with(
        cfg: InstanceConfig,
        journal: impl IntoIterator<Item = Input>,
        view: &View,
        enc: &Encoder,
        setup: impl FnOnce(&mut Instance),
    ) -> Result<(Instance, Vec<Command>), String> {
        let mut agent = Instance::new(cfg)?;
        setup(&mut agent);
        agent.start(view, enc)?;
        agent.trace.clear();
        // `AFFORDANCE_INPUT_TIMING`: time per input type over the replay, with the renders each type
        // caused (a diagnostic for per-input cost).
        let timing = std::env::var_os("AFFORDANCE_INPUT_TIMING").is_some();
        let mut by_kind: BTreeMap<&'static str, [u64; 6]> = BTreeMap::new();
        for input in journal {
            let kind = input.kind_name();
            let (t, before) = (std::time::Instant::now(), agent.render_stats.clone());
            agent.handle(view, enc, input)?;
            if timing {
                let e = by_kind.entry(kind).or_default();
                let r = &agent.render_stats;
                e[0] += 1;
                e[1] += t.elapsed().as_micros() as u64;
                e[2] += r.evaluations - before.evaluations;
                e[3] += r.render_us - before.render_us;
                e[4] += r.layouts - before.layouts;
                e[5] += r.layout_us - before.layout_us;
            }
            // Reconstructed diagnostics are never appended on restore. Drain per input so
            // a long replay does not retain a second in-memory history of completions.
            agent.trace.clear();
        }
        if timing {
            for (kind, [n, us, renders, render_us, layouts, layout_us]) in &by_kind {
                let per = |x: u64| x as f64 / *n as f64;
                eprintln!(
                    "input-timing agent={} kind={kind} n={n} ms_per={:.2} renders_per={:.2} render_ms_per={:.2} layouts_per={:.2} layout_ms_per={:.2}",
                    agent.cfg.id,
                    per(*us) / 1000.0,
                    per(*renders),
                    per(*render_us) / 1000.0,
                    per(*layouts),
                    per(*layout_us) / 1000.0
                );
            }
        }
        let outstanding = agent_outstanding(&agent);
        Ok((agent, outstanding))
    }

    pub fn id(&self) -> &str {
        &self.cfg.id
    }
    /// The state as JSON (large strings as blob references when the agent has blobs).
    pub fn state(&self) -> J {
        J::Object(self.state.iter().map(|(k, v)| (k.to_string(), v.to_wire())).collect())
    }
    /// How many logical inputs the agent has applied (a journal frame may contain several).
    pub fn journal_len(&self) -> usize {
        self.journaled
    }
    /// The value it stopped with, if it stopped.
    pub fn stopped(&self) -> Option<&J> {
        self.stopped.as_ref()
    }
    /// Its clock: the latest input's time.
    pub fn clock_ms(&self) -> i64 {
        self.clock_ms
    }
    /// The first turn whose journaled reply answered a prompt this app no longer renders.
    pub fn diverged_at(&self) -> Option<u64> {
        self.diverged_at
    }
    /// Commands not yet answered.
    pub fn outstanding(&self) -> Vec<Command> {
        agent_outstanding(self)
    }
    /// What it has outstanding, summarized without copying commands.
    pub fn outstanding_summary(&self) -> OutstandingSummary {
        agent_outstanding_summary(self)
    }
    /// Trace entries since the last call (renders, replies, handler errors, ...).
    pub fn take_trace(&mut self) -> Vec<J> {
        std::mem::take(&mut self.trace)
    }
    /// Add a trace record. Each has its time (`at_ms`, the journal's clock): when what it records
    /// began if it says so (a tool's start), else the input being handled.
    pub(super) fn note(&mut self, mut entry: J) {
        if let Some(o) = entry.as_object_mut() {
            o.entry("at_ms").or_insert(json!(self.clock_ms));
        }
        self.trace.push(entry);
    }
    /// True when the agent waits for nothing: stopped, or idle with no outstanding work.
    pub fn quiescent(&self) -> bool {
        self.stopped.is_some() || (self.tasks.is_empty() && self.handlers.is_empty())
    }

    /// Keep strings of at least the blob store's size on disk (set before `start`).
    pub fn set_blobs(&mut self, blobs: Arc<Blobs>) {
        self.resolver = Some(blobs.resolver());
        self.blobs = Some(blobs);
    }

    /// Replace the large strings of `input` with blob references (the host does this before
    /// journaling; `handle` does it again, which changes nothing).
    pub fn lower_input(&self, input: &mut Input) -> Result<(), String> {
        if let Input::Batch { inputs } = input {
            validate_batch(inputs)?;
            for input in inputs {
                self.lower_input(input)?;
            }
            return Ok(());
        }
        let Some(b) = &self.blobs else { return Ok(()) };
        match input {
            Input::Result {
                outcome: Outcome::Ok(v),
                ..
            } => b.lower(v),
            Input::Event { payload, .. } | Input::Props { props: payload, .. } => b.lower(payload),
            // Errors reach the app as thrown messages, not values: they stay text.
            Input::Result {
                outcome: Outcome::Error(_),
                ..
            }
            | Input::ModelError { .. }
            | Input::Reply { .. }
            | Input::Tick { .. } => Ok(()),
            Input::Batch { .. } => unreachable!(),
        }
    }

    /// The first commands (the first turn, the first children).
    pub fn start(&mut self, view: &View, enc: &Encoder) -> Result<Vec<Command>, String> {
        let result = self.start_scoped(view, enc);
        self.keep_console_of_a_failure(&result);
        result
    }

    /// What the app wrote to its console during an input that failed stays in this instance's
    /// trace. Left in the thread's list, it would appear in the trace of whichever instance the
    /// worker ran next.
    /// Where its large strings live, for a second agent that must read the same ones.
    pub(crate) fn blobs_handle(&self) -> Option<Arc<Blobs>> {
        self.blobs.clone()
    }

    fn keep_console_of_a_failure<T>(&mut self, result: &Result<T, String>) {
        if result.is_err() {
            for (level, text) in crate::console::take() {
                self.note(json!({"t": "console", "level": level, "text": text}));
            }
        }
    }

    fn start_scoped(&mut self, view: &View, enc: &Encoder) -> Result<Vec<Command>, String> {
        let _heap = self.heap.scope();
        with_host_text(self.resolver.clone(), || {
            self.realm.borrow_mut().fuel = 0;
            crate::microtasks::within(self.realm.clone(), || {
                let mut out = self.start_inner(view, enc)?;
                out.extend(self.checkpoint(view, enc)?);
                self.finish_commands(&mut out);
                Ok(out)
            })
        })
    }
    fn drain_jobs(&mut self, view: &View, enc: &Encoder) -> Result<Vec<Command>, String> {
        let mut out = vec![];
        loop {
            let Some(owner) = self.realm.borrow_mut().next_task() else {
                break;
            };
            if self.stopped.is_some() {
                break;
            }
            if owner == crate::microtasks::DOCUMENT {
                crate::microtasks::Queue::native(&self.realm, owner)?;
                continue;
            }
            if !self.handlers.contains_key(&owner) {
                self.realm.borrow_mut().finish(owner);
                continue;
            }
            out.extend(self.run_handler(view, enc, owner)?);
        }
        // A promise that rejected with nothing awaiting or catching it (an async function called
        // and not awaited): the platform's `unhandledrejection`. The task goes on; the failure is
        // on the trace and counted, not lost.
        let unhandled = self.realm.borrow_mut().take_unhandled();
        let origins = std::mem::take(&mut self.ran);
        for (task, error) in unhandled {
            let origin = task
                .and_then(|t| {
                    origins
                        .get(&t)
                        .cloned()
                        .or_else(|| self.handlers.get(&t).map(|h| h.origin.label()))
                })
                .unwrap_or_else(|| "render".into());
            self.metrics.handler_errors += 1;
            self.note(json!({"t": "handler_error", "origin": origin, "error": error, "unhandled_rejection": true}));
        }
        // Finalize returned bodies after all requests from the phase have been issued.
        // A last, unawaited host completion need not enqueue a JavaScript job.
        let finished = self
            .handlers
            .iter()
            .filter_map(|(id, handler)| {
                if handler.effects.is_empty() && !self.realm.borrow().has_jobs(*id) {
                    handler.done.clone().map(|value| (*id, value))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        for (id, value) in finished {
            self.finish_handler(id, Ok(value));
        }
        Ok(out)
    }
    /// The renders' Promise group, once its jobs have run, goes: an agent that used a Promise
    /// during render is as idle (and as imageable) as one that did not. Whether a render ran or
    /// was reused, the original, a restored image and a full replay all drop it at the same step
    /// boundaries and ends, and the next render begins a fresh one.
    pub(super) fn forget_render_promises(&mut self) {
        self.realm.borrow_mut().forget_spent(crate::microtasks::DOCUMENT);
    }
    fn checkpoint(&mut self, view: &View, enc: &Encoder) -> Result<Vec<Command>, String> {
        let mut out = vec![];
        let mut updates = 0;
        loop {
            out.extend(self.drain_jobs(view, enc)?);
            if self.stopped.is_some() {
                break;
            }
            // Events raised while the step ran (a prompt's `layout`) reach their listeners now.
            if !self.events.is_empty() {
                out.extend(self.pump(view, enc)?);
                continue;
            }
            // The step that ended has drained and its effects pass has not run: a boundary.
            // Keeping one copies the instance's bookkeeping (its event log, the message ids it has
            // seen, kept outputs, children's props). While that is small every boundary is kept,
            // so a restore starts from the last one. Once it is large, one is kept every
            // BOUNDARY_GAP inputs: a recovery image is written far less often than that, and a
            // restore replays at most that many inputs more.
            let bookkeeping = self.event_log.len()
                + self.seen_msgs.len()
                + self.children.len()
                + self.kept.len()
                + self.changed_at.len();
            if updates == 0
                && self.effects_version != self.version
                && (bookkeeping <= BOUNDARY_GAP * 16
                    || self
                        .boundary_inputs()
                        .is_none_or(|at| self.journaled >= at + BOUNDARY_GAP))
            {
                self.note_boundary();
            }
            if self.effects_version != self.version {
                updates += 1;
                if updates > MAX_UPDATE_DEPTH {
                    self.stopped = Some(json!({"reason":"max_update_depth"}));
                    self.note(json!({"t":"stop","reason":"max_update_depth"}));
                    break;
                }
                out.extend(self.run_effects(view, enc)?);
                continue;
            }
            break;
        }
        out.extend(self.reconcile(view, enc)?);
        out.extend(self.report_stop());
        self.note_changes();
        Ok(out)
    }
    fn start_inner(&mut self, view: &View, enc: &Encoder) -> Result<Vec<Command>, String> {
        self.state = state_map(&self.cfg.props)?;
        self.pump(view, enc)
    }

    /// Note which hooks this input changed. Kept state is frozen, so an unchanged hook is the same
    /// value (`same` stops at a shared pointer): this costs one compare per hook, and nothing
    /// when the input changed no state.
    fn note_changes(&mut self) {
        if Rc::ptr_eq(&self.state, &self.noted) {
            return;
        }
        let at = self.journaled;
        let ui = |s: &Map| match s.get("__ui") {
            Some(Value::Map(m)) => Some(m.clone()),
            _ => None,
        };
        match (ui(&self.noted), ui(&self.state)) {
            (Some(a), Some(b)) if Rc::ptr_eq(&a, &b) => {}
            (a, Some(b)) => {
                let (a, b) = (a.as_ref().map(|a| a.read()), b.read());
                for (k, v) in b.iter() {
                    if a.as_ref().and_then(|a| a.get(k)).is_none_or(|o| !same(o, v)) {
                        self.changed_at.insert(k.clone(), at);
                    }
                }
                // A hook whose element is gone has no state left to have changed.
                if self.changed_at.len() > b.len() {
                    self.changed_at.retain(|k, _| b.contains_key(k));
                }
            }
            _ => {}
        }
        let props = |s: &Map| s.iter().filter(|(k, _)| &***k != "__ui").count();
        if props(&self.state) != props(&self.noted)
            || self
                .state
                .iter()
                .any(|(k, v)| &**k != "__ui" && self.noted.get(k).is_none_or(|o| !same(o, v)))
        {
            self.props_at = at;
        }
        self.noted = self.state.clone();
    }

    /// The state as kept (props and `__ui`), for `harness.inspect` on this agent's worker.
    pub fn kept_state(&self) -> &Rc<Map> {
        &self.state
    }
    /// The journal position at which hook `key` (or, for `None`, the props) last changed.
    pub fn changed_at(&self, key: Option<&str>) -> usize {
        key.map_or(self.props_at, |k| self.changed_at.get(k).copied().unwrap_or(0))
    }

    /// A stopped child tells its parent once, with its stop value, if the parent passed `onStop`.
    fn report_stop(&mut self) -> Option<Command> {
        let stopped = self.stopped.clone()?;
        if self.stop_reported {
            return None;
        }
        self.stop_reported = true;
        (self
            .state
            .get("onStop")
            .is_some_and(|v| v.to_wire()["__parent"] == "onStop"))
        .then(|| Command::Callback {
            name: "onStop".into(),
            args: json!([stopped]),
        })
    }

    /// The children this instance has mounted (key, `{app, props, model, machines}`).
    pub fn children(&self) -> &BTreeMap<String, J> {
        &self.children
    }

    /// Mount, update and unmount children to match this render's `<Mount>` declarations. A
    /// stopped instance has no children.
    fn reconcile(&mut self, view: &View, enc: &Encoder) -> Result<Vec<Command>, String> {
        let mut declared: BTreeMap<String, J> = BTreeMap::new();
        if self.stopped.is_none() {
            let prepared = self.evaluate(view, enc)?;
            for meta in &prepared.mounts {
                let key = meta["key"].as_str().unwrap_or_default().to_string();
                if !valid_instance_name(&key) {
                    return Err(format!(
                        "<Mount key={key:?}>: a key must be a non-empty name without a leading '.', path separators, or control characters"
                    ));
                }
                let machines: Vec<String> = meta["machines"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|m| m.as_str().map(str::to_owned))
                    .collect();
                if let Some(m) = machines.iter().find(|m| !self.cfg.machines.contains(m)) {
                    return Err(format!(
                        "<Mount key={key:?}>: machine {m} is not bound to this instance (a child gets a subset of its parent's)"
                    ));
                }
                // Callback props become stubs that call back into this instance.
                let mut props = meta["props"].clone();
                for cb in meta["callbacks"].as_array().into_iter().flatten().filter_map(J::as_str) {
                    props[cb] = json!({"__parent": cb});
                }
                let model = meta["model"]
                    .as_str()
                    .map(str::to_owned)
                    .or_else(|| self.cfg.model.clone());
                if declared.insert(key.clone(), json!({"app": meta["app"], "entry": meta["entry"], "props": props, "model": model, "machines": machines})).is_some() {
                    return Err(format!("two <Mount>s with key {key:?}"))
                }
            }
        }
        let mut out = vec![];
        for key in self.children.keys().filter(|k| !declared.contains_key(*k)) {
            out.push(Command::Unmount { key: key.clone() });
        }
        for (key, d) in &declared {
            match self.children.get(key) {
                Some(old)
                    if old["app"] == d["app"]
                        && old["entry"] == d["entry"]
                        && old["model"] == d["model"]
                        && old["machines"] == d["machines"] =>
                {
                    if old["props"] != d["props"] {
                        out.push(Command::Props {
                            key: key.clone(),
                            props: d["props"].clone(),
                        });
                    }
                }
                old => {
                    if old.is_some() {
                        out.push(Command::Unmount { key: key.clone() });
                    }
                    let machines = d["machines"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|m| m.as_str().map(str::to_owned))
                        .collect();
                    out.push(Command::Mount {
                        key: key.clone(),
                        app: d["app"].as_str().unwrap_or_default().into(),
                        entry: d["entry"].as_str().unwrap_or_default().into(),
                        props: d["props"].clone(),
                        model: d["model"].as_str().map(str::to_owned),
                        machines,
                    });
                }
            }
        }
        self.children = declared;
        Ok(out)
    }

    /// Apply one input and return the commands it leads to.
    pub fn handle(&mut self, view: &View, enc: &Encoder, input: Input) -> Result<Vec<Command>, String> {
        let result = self.handle_scoped(view, enc, input);
        self.keep_console_of_a_failure(&result);
        result
    }

    fn handle_scoped(&mut self, view: &View, enc: &Encoder, input: Input) -> Result<Vec<Command>, String> {
        let _heap = self.heap.scope();
        with_host_text(self.resolver.clone(), || {
            self.realm.borrow_mut().fuel = 0;
            crate::microtasks::within(self.realm.clone(), || {
                let mut out = self.handle_inner(view, enc, input)?;
                out.extend(self.checkpoint(view, enc)?);
                self.finish_commands(&mut out);
                Ok(out)
            })
        })
    }

    fn handle_inner(&mut self, view: &View, enc: &Encoder, mut input: Input) -> Result<Vec<Command>, String> {
        self.lower_input(&mut input)?;
        // A batch is one journal record of several inputs, written together; each input still takes
        // its own turn (its render, its handlers, their microtasks, its effects pass), as it would
        // arriving alone, so how inputs are batched never changes what an app computes. One that
        // stops the instance ends the frame (the rest are journaled and not taken, as after a stop).
        if let Input::Batch { inputs } = input {
            let last = inputs.len().saturating_sub(1);
            let mut commands = vec![];
            for (i, input) in inputs.into_iter().enumerate() {
                if self.stopped.is_some() {
                    self.journaled += 1;
                    continue;
                }
                self.realm.borrow_mut().fuel = 0;
                commands.extend(self.handle_lowered(view, enc, input)?);
                if i < last {
                    commands.extend(self.checkpoint(view, enc)?);
                }
            }
            return Ok(commands);
        }
        self.handle_lowered(view, enc, input)
    }

    fn handle_lowered(&mut self, view: &View, enc: &Encoder, input: Input) -> Result<Vec<Command>, String> {
        self.journaled += 1;
        self.clock_ms = self.clock_ms.max(input.clock_ms());
        // An input answers the kind of call its task is: a reply or a model error answers a model
        // call, a result answers a machine call. One that says otherwise was not written by this
        // instance (a journal edited by hand), and is refused before it can leave a call half
        // answered.
        let wrong_kind = match &input {
            Input::Reply { task, .. } | Input::ModelError { task, .. } => self
                .tasks
                .get(task)
                .is_some_and(|(_, c)| !matches!(c, Command::Generate { .. })),
            Input::Result { task, .. } => self
                .tasks
                .get(task)
                .is_some_and(|(_, c)| matches!(c, Command::Generate { .. })),
            _ => false,
        };
        if wrong_kind {
            return Err(format!(
                "the journal's `{}` input answers a task of another kind; the journal does not belong to this instance",
                input.kind_name()
            ));
        }
        let mut out = vec![];
        match input {
            Input::Batch { .. } => return Err("nested input batch".into()),
            Input::Reply {
                task,
                ids,
                cut_off,
                prompt,
                aborted,
                server,
                ..
            } => match self.tasks.remove(&task) {
                Some((Some((h, i)), _)) => {
                    // An aborted reply is cut short: its text is kept, and a call it began is not made.
                    let cut_off = cut_off || aborted;
                    self.metrics.generated_tokens += ids.len() as u64;
                    let outcome = match self.open.get_mut(&(h, i)).and_then(|o| o.model.take()) {
                        Some(call) => {
                            if let Some(p) = prompt.as_ref().filter(|p| call.page && **p != call.hash) {
                                let _ = p;
                                let n = call.no;
                                if self.strict_replay {
                                    return Err(format!(
                                        "replay diverged at turn {n}: the app renders a different prompt than the one the model answered"
                                    ));
                                }
                                self.diverged_at.get_or_insert(n);
                                self.note(json!({"t": "replay_diverged", "turn": n}));
                            }
                            self.model_output(enc, call, &ids, cut_off)
                        }
                        None => self.render_result(enc, h, i, &ids, cut_off),
                    };
                    let mut outcome = if aborted { marked_aborted(outcome) } else { outcome };
                    if let Outcome::Ok(out) = &mut outcome
                        && out.is_object()
                    {
                        out["server"] = server.unwrap_or(J::Null);
                    }
                    self.resolve(h, i, outcome);
                }
                // A cancelled task's late reply (or a duplicate) changes nothing.
                _ => self.note(json!({"t": "late_reply", "task": task})),
            },
            Input::ModelError {
                task,
                error,
                kind,
                retry_after_ms,
                ..
            } => {
                let owner = self.tasks.get(&task).map(|(o, _)| *o);
                // A call told to stop that fails instead ends as stopped, with nothing generated.
                if let Some(Some((h, i))) = owner.filter(|_| self.aborting.contains(&task)) {
                    self.tasks.remove(&task);
                    let outcome = self.aborted_reply(enc, h, i, &[]);
                    self.resolve(h, i, outcome);
                    out.extend(self.pump(view, enc)?);
                    return Ok(out);
                }
                if let Some(Some((h, i))) = owner {
                    let (_, cmd) = self.tasks.remove(&task).unwrap();
                    self.metrics.model_errors += 1;
                    // One attempt failed. The retry policy (the call's own, else its binding's)
                    // says whether there is another and after how long, from the kind of failure,
                    // how many there have been, how long since the first, and the wait the server
                    // asked for. All of that is journaled, so a replay decides the same.
                    let slot = self.open.entry((h, i)).or_default();
                    let (failures, since) = slot
                        .retry
                        .take()
                        .map_or((1, self.clock_ms), |r| (r.failures + 1, r.since));
                    let page = slot.model.as_ref().is_some_and(|c| c.page);
                    let policy = slot.policy.clone().unwrap_or_else(|| self.cfg.retry.clone());
                    let wait = policy.next(kind, failures, since, self.clock_ms, retry_after_ms);
                    let mut record = json!({"t": if page { "model_error" } else { "render_error" }, "error": error, "kind": kind.name(), "failures": failures});
                    if let Some(wait) = wait {
                        record["retry_in_ms"] = json!(wait);
                    }
                    self.note(record);
                    if let Some(wait) = wait {
                        let at = self.clock_ms + wait as i64;
                        self.open.entry((h, i)).or_default().retry = Some(Retry {
                            at: Some(at),
                            failures,
                            kind: kind.name(),
                            cmd,
                            since,
                        });
                        out.push(Command::Timer { at_ms: at });
                        return Ok(out);
                    }
                    // A page call that failed for good has no output to dispatch: its view goes.
                    if let Some(no) = self.open.get(&(h, i)).and_then(|o| o.model.as_ref()).map(|c| c.no) {
                        self.kept.remove(&no);
                    }
                    self.resolve(h, i, Outcome::Error(error));
                    out.extend(self.pump(view, enc)?);
                    return Ok(out);
                }
                self.tasks.remove(&task);
                self.note(json!({"t": "late_reply", "task": task}));
            }
            Input::Result { task, outcome, .. } => match self.tasks.remove(&task) {
                Some((Some((h, i)), _)) => {
                    self.resolve(h, i, outcome);
                }
                _ => self.note(json!({"t": "late_result", "task": task})),
            },
            Input::Event { name, id: Some(id), .. } if !self.seen_msgs.insert(id.clone()) => {
                self.note(json!({"t": "duplicate_message", "event": name, "id": id}));
            }
            Input::Event { name, payload, .. } => {
                // A matching wait resolves at once (it consumes the event for its task); the event
                // also goes to the listeners when this input ends.
                let at = self.journaled as u64;
                self.event_log.push_back((at, name.clone(), payload.clone()));
                if self.event_log.len() > EVENT_LOG {
                    self.event_log.pop_front();
                }
                let hits: Vec<(u64, usize)> = self
                    .open
                    .iter()
                    .filter(|(_, o)| o.wait.as_ref().is_some_and(|w| w.wake_on.contains(&name)))
                    .map(|(k, _)| *k)
                    .collect();
                for (h, i) in hits {
                    if let Some(handler) = self.handlers.get_mut(&h) {
                        handler.consumed = at;
                    }
                    let value = json!({"woke": "event", "event": name, "payload": payload});
                    self.resolve(h, i, Outcome::Ok(value));
                }
                // A stopped instance has no listeners left to run: its events are journaled (the
                // input above) and not queued, or a finished worker that others keep messaging
                // would hold every one of them.
                if self.stopped.is_none() {
                    self.events.push_back((name, payload, None));
                }
            }
            Input::Props { props, .. } => {
                // New props: the parent's values replace the old; this instance's hooks stay.
                let ui = self.state.get("__ui").cloned();
                self.state = state_map(&props)?;
                self.version += 1;
                if let Some(h) = ui {
                    Rc::make_mut(&mut self.state).insert(Rc::from("__ui"), h);
                }
            }
            Input::Tick { .. } => {
                // Retry renders whose model call failed transiently, with the same prompt.
                let due: Vec<(u64, usize)> = self
                    .open
                    .iter()
                    .filter(|(_, o)| {
                        o.retry
                            .as_ref()
                            .and_then(|r| r.at)
                            .is_some_and(|at| at <= self.clock_ms)
                    })
                    .map(|(k, _)| *k)
                    .collect();
                for owner in due {
                    let entry = self.open.get_mut(&owner).and_then(|o| o.retry.as_mut()).unwrap();
                    entry.at = None;
                    let Command::Generate {
                        ids,
                        stop,
                        max_new,
                        opts,
                        prompt_hash,
                        images,
                        media,
                        ..
                    } = entry.cmd.clone()
                    else {
                        continue;
                    };
                    out.push(self.new_task(Some(owner), |task| Command::Generate {
                        task,
                        ids,
                        stop,
                        max_new,
                        opts,
                        prompt_hash,
                        images,
                        media,
                    }));
                }
                let due: Vec<(u64, usize)> = self
                    .open
                    .iter()
                    .filter(|(_, o)| o.wait.as_ref().is_some_and(|w| w.until_ms <= self.clock_ms))
                    .map(|(k, _)| *k)
                    .collect();
                for (h, i) in due {
                    self.resolve(h, i, Outcome::Ok(json!({"woke": "timeout"})));
                }
            }
        }
        out.extend(self.pump(view, enc)?);
        Ok(out)
    }

    fn finish_commands(&mut self, out: &mut Vec<Command>) {
        // What the app wrote to its console while this input was handled goes to the trace.
        for (level, text) in crate::console::take() {
            self.note(json!({"t": "console", "level": level, "text": text}));
        }
        // A stopped agent needs nothing it asked for; a finished handler needs none of its calls.
        if self.stopped.is_some() {
            let tasks: Vec<u64> = self.tasks.keys().copied().collect();
            self.cancelled.extend(tasks);
            self.tasks.clear();
            for id in self.handlers.keys().copied().collect::<Vec<_>>() {
                self.realm.borrow_mut().finish(id);
            }
            self.realm.borrow_mut().finish(crate::microtasks::DOCUMENT);
            self.handlers.clear();
            self.open.clear();
        } else {
            self.forget_render_promises();
        }
        // An effect is open only while its handler awaits it.
        self.open
            .retain(|(h, i), _| self.handlers.get(h).is_some_and(|h| h.effects.contains_key(i)));
        self.aborting.retain(|t| self.tasks.contains_key(t));
        out.retain(|c| !matches!(c, Command::Generate { task, .. } | Command::Call { task, .. } | Command::Abort { task } if !self.tasks.contains_key(task)));
        out.extend(self.cancelled.drain(..).map(|task| Command::Cancel { task }));
    }

    /// The `harness` const of a render: env, clock and counters.
    fn context(&self) -> Value {
        let mut m = crate::value::Map::default();
        let mut put = |k: &str, v: Value| {
            m.insert(std::rc::Rc::from(k), v);
        };
        put("instance", Value::str(&self.cfg.id));
        put("turn", Value::Int(self.metrics.turns as i64));
        put("clock_ms", Value::Int(self.clock_ms));
        put("env", self.env_v.clone());
        put("prompt_tokens", Value::Int(self.last_prompt as i64));
        put("owner", Value::str("agent"));
        put(
            "machines",
            Value::list(self.cfg.machines.iter().map(|m| Value::str(m)).collect()),
        );
        put("model", Value::from_json(&self.cfg.model.as_ref().map(|name| json!({
            "name": name, "template": self.cfg.template.as_ref().map(|t| &t.name),
            "template_options": self.cfg.template.as_ref().map(|t| t.options.clone()).unwrap_or(json!({}))
        })).unwrap_or(J::Null)));
        // `harness` is the runtime's: frozen.
        let v = Value::map(m);
        crate::value::freeze(&v);
        v
    }

    /// The model this instance's prompts are laid out for: its binding's context, with the
    /// instance's reply room (a call's own `max_new` is checked against the context when sent).
    fn window(&self) -> Option<crate::emit::Window> {
        self.cfg.context.map(|context| crate::emit::Window {
            context,
            max_new: self.cfg.max_new,
        })
    }

    /// A prompt that does not fit the model's context with its reply's room is never sent (the
    /// server would reject or truncate it); the call fails with the boxes that hold the prompt,
    /// so the sheet that should have capped one can be fixed.
    fn over_context(&self, prompt: usize, max_new: usize, spans: &[crate::emit::Span]) -> Option<String> {
        let context = self.cfg.context?;
        let prompt = sent_tokens(prompt, spans);
        if prompt.saturating_add(max_new) <= context {
            return None;
        }
        let mut boxes: Vec<&crate::emit::Span> = spans.iter().filter(|s| s.kind == "box").collect();
        boxes.sort_by_key(|s| std::cmp::Reverse(s.end - s.start));
        let named: Vec<String> = boxes
            .iter()
            .take(5)
            .map(|s| {
                format!(
                    "{} {} tokens",
                    if s.label.is_empty() { "box" } else { &s.label },
                    s.end - s.start
                )
            })
            .collect();
        Some(format!(
            "ContextError: a prompt of {prompt} tokens and a reply of {max_new} do not fit the model's context ({context}); window {}{}",
            context.saturating_sub(max_new),
            if named.is_empty() {
                "; no box caps anything (give the history a max-tokens)".to_string()
            } else {
                format!("; largest boxes: {}", named.join(", "))
            }
        ))
    }

    /// The page evaluated (handlers, effects, mounts, listeners) and laid out: what a prompt
    /// needs.
    fn render_prepared(
        &mut self,
        view: &View,
        enc: &Encoder,
    ) -> Result<(Rc<crate::Rendered>, Rc<crate::Prepared>), String> {
        let prepared = self.evaluate(view, enc)?;
        let r = self.laid_out(view, enc, &prepared)?;
        Ok((r, prepared))
    }

    /// The page's layout, done once per evaluated
    /// document, when something reads it: a prompt, a model-chosen action.
    fn laid_out(
        &mut self,
        view: &View,
        enc: &Encoder,
        prepared: &crate::Prepared,
    ) -> Result<Rc<crate::Rendered>, String> {
        if let Some(r) = prepared.doc.laid_out() {
            return Ok(r);
        }
        // Token runs live per layout: an evaluation that is not laid out keeps them.
        self.cache.next_run_generation();
        let started = std::time::Instant::now();
        let r = view.layout(enc, &prepared.doc, &mut self.cache)?;
        self.render_stats.layouts += 1;
        self.render_stats.layout_us += started.elapsed().as_micros() as u64;
        Ok(r)
    }

    /// The page evaluated, not laid out: what an effects pass, a reconcile and event delivery
    /// read. Reused while nothing it read has changed.
    fn evaluate(&mut self, view: &View, enc: &Encoder) -> Result<Rc<crate::Prepared>, String> {
        let ctx = self.context();
        let context = ctx.to_wire();
        if let Some(snapshot) = &self.last_render
            && snapshot.version == self.version
            && snapshot.app == view.source_hash
            && snapshot.tokenizer == enc.identity
            && snapshot.reads.unchanged(&snapshot.context, &context)
        {
            static VERIFY: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
            if *VERIFY.get_or_init(|| std::env::var_os("AFFORDANCE_RENDER_VERIFY").is_some()) {
                let (fresh, prepared) = view.render_prepared(
                    enc,
                    self.state.clone(),
                    &mut RunCache::new(false),
                    date(self.clock_ms)?,
                    ctx.clone(),
                    self.window(),
                )?;
                let kept = snapshot.prepared.clone();
                let kept_ids = view.layout(enc, &kept.doc, &mut RunCache::new(false))?.ids.clone();
                let effects = |p: &crate::Prepared| {
                    p.effects
                        .iter()
                        .map(|(key, deps, _)| (key.clone(), deps.to_wire()))
                        .collect::<Vec<_>>()
                };
                if fresh.ids != kept_ids
                    || !fresh.mounted.is_empty()
                    || prepared.mounts != kept.mounts
                    || effects(&prepared) != effects(&kept)
                {
                    return Err("render cache validity: a reused document differs from fresh evaluation".into());
                }
            }
            self.render_stats.reused += 1;
            return Ok(snapshot.prepared.with_context(ctx, date(self.clock_ms)?));
        }
        // Scan lifetime follows evaluated documents, including apps with no model calls.
        self.cache.memo.borrow_mut().next_generation();
        let started = std::time::Instant::now();
        let window = self.window();
        let (prepared, reads) = crate::observe::capture(&ctx, || {
            view.evaluate(
                enc,
                self.state.clone(),
                &self.cache,
                date(self.clock_ms)?,
                ctx.clone(),
                window,
            )
        })?;
        let us = started.elapsed().as_micros() as u64;
        self.render_stats.evaluations += 1;
        self.render_stats.render_us += us;
        // Scans in the page (this agent's cache) and in projections (the view's, e.g. templates).
        let misses = std::mem::take(&mut self.cache.memo.borrow_mut().misses);
        for m in misses {
            self.note(m);
        }
        let projected = std::mem::take(&mut view.checkpoint_context().1.borrow_mut().misses);
        for m in projected {
            self.note(m);
        }
        // Hooks used for the first time keep their initial values.
        let mounted = prepared.doc.mounted.clone();
        if !mounted.is_empty() {
            self.mount_hooks(&mounted);
        }
        self.last_render = if mounted.is_empty() {
            Some(RenderSnapshot {
                version: self.version,
                app: view.source_hash.clone(),
                tokenizer: enc.identity.clone(),
                context,
                reads,
                prepared: prepared.clone(),
            })
        } else {
            None
        };
        Ok(prepared)
    }

    /// Hooks used for the first time, with their initial values.
    fn mount_hooks(&mut self, mounted: &[(String, Value)]) {
        let mut ui = hooks(&self.state);
        for (k, v) in mounted {
            crate::value::freeze(v);
            ui.insert(Rc::from(k.as_str()), v.clone());
        }
        Rc::make_mut(&mut self.state).insert(Rc::from("__ui"), Value::Map(Rc::new(crate::value::Obj::frozen(ui))));
    }

    fn new_task(&mut self, owner: Option<(u64, usize)>, make: impl FnOnce(u64) -> Command) -> Command {
        let task = self.next_task;
        self.next_task += 1;
        let cmd = make(task);
        self.tasks.insert(task, (owner, cmd.clone()));
        cmd
    }

    /// The end of an input: deliver the events it brought to their listeners (each a task, run at
    /// once; nothing waits for other tasks). Effects run after the microtask checkpoint.
    fn pump(&mut self, view: &View, enc: &Encoder) -> Result<Vec<Command>, String> {
        let mut out = vec![];
        if self.stopped.is_some() {
            return Ok(out);
        }
        while let Some((event, payload, doc)) = self.events.pop_front() {
            // The listeners are found in this document (evaluated: no layout; a listener in a
            // cropped or hidden box is live), and run from it while the state is the one it
            // rendered (a listener that changed it makes the next one render again).
            let version = self.version;
            // The document the event was raised on (a prompt's `layout` goes to the listeners of
            // the document that was laid out, as an event goes to the listeners registered when it
            // fired), or the current one.
            let raised = doc.is_some();
            let prepared = match doc {
                Some(prepared) => prepared.with_context(self.context(), date(self.clock_ms)?),
                None => self.evaluate(view, enc)?,
            };
            // A child's callback (`mount:<key>:onX`) runs that `<Mount>`'s handler.
            if event.starts_with("mount:") {
                if !prepared.listeners.iter().any(|s| s.label == event) {
                    self.note(json!({"t": "event_dropped", "event": event}));
                    continue;
                }
                let ctx = self.context();
                let (state, clock) = (self.state.clone(), self.clock_ms);
                out.extend(self.start_handler_at(
                    view,
                    enc,
                    Origin::Callback { event: event.clone() },
                    &event,
                    0,
                    payload,
                    state,
                    ctx,
                    clock,
                    Some(prepared),
                    None,
                )?);
                continue;
            }
            let listeners: Vec<usize> = prepared
                .listeners
                .iter()
                .filter(|s| s.label == "callback")
                .enumerate()
                .filter(|(_, s)| s.meta["props"]["event"].as_str() == Some(event.as_str()))
                .map(|(i, _)| i)
                .collect();
            if listeners.is_empty() {
                self.note(json!({"t": "event_dropped", "event": event}));
            }
            let ctx = self.context();
            for which in listeners {
                let (state, clock) = (self.state.clone(), self.clock_ms);
                let kept = (raised || self.version == version).then(|| prepared.clone());
                out.extend(self.start_handler_at(
                    view,
                    enc,
                    Origin::Callback { event: event.clone() },
                    "callback",
                    which,
                    payload.clone(),
                    state,
                    ctx.clone(),
                    clock,
                    kept,
                    None,
                )?);
            }
        }
        Ok(out)
    }

    /// Effects: render, and for each `useEffect` whose deps changed (or that went away) run its
    /// old cleanup, then its setup, each as a task. The checkpoint drains jobs before another
    /// pass and bounds the number of state-driven updates.
    fn run_effects(&mut self, view: &View, enc: &Encoder) -> Result<Vec<Command>, String> {
        let mut out = vec![];
        if self.stopped.is_some() || self.version == self.effects_version {
            return Ok(out);
        }
        self.effects_version = self.version;
        // Lifecycle, model projection and mounts consume the same completed document snapshot.
        let prepared = self.evaluate(view, enc)?;
        let mut ui = hooks(&self.state);
        let before = ui.len();
        ui.retain(|key, _| !key.starts_with("agent/") || prepared.snap.contains_hook(key));
        if ui.len() != before {
            Rc::make_mut(&mut self.state).insert(Rc::from("__ui"), Value::Map(Rc::new(crate::value::Obj::frozen(ui))));
            self.version += 1;
        }
        let declared: BTreeMap<&String, &Value> = prepared.effects.iter().map(|(k, d, _)| (k, d)).collect();
        let gone: Vec<String> = self
            .effect_state
            .keys()
            .filter(|k| !declared.contains_key(k))
            .cloned()
            .collect();
        let changed: Vec<(String, Value, Value)> = prepared
            .effects
            .iter()
            .filter(|(k, d, _)| {
                self.effect_state
                    .get(k)
                    .is_none_or(|(old, _)| crate::jsvalue::deps_changed(old, d))
            })
            .map(|(k, d, f)| (k.clone(), d.clone(), f.clone()))
            .collect();
        if gone.is_empty() && changed.is_empty() {
            return Ok(out);
        }
        // Cleanups first (the effects that went away, then the ones about to run again), then setups.
        for key in gone.iter().chain(changed.iter().map(|(k, _, _)| k)) {
            let cleanup = match self.effect_state.get_mut(key) {
                Some((_, c)) => c.take(),
                None => None,
            };
            if let Some((p, f)) = cleanup {
                out.extend(self.start_closure(view, enc, Origin::Cleanup { key: key.clone() }, p, f)?);
            }
        }
        for key in &gone {
            self.effect_state.remove(key);
        }
        for (key, deps, setup) in changed {
            self.effect_state.insert(key.clone(), (deps, None));
            out.extend(self.start_closure(view, enc, Origin::Effect { key }, prepared.clone(), setup)?);
        }
        Ok(out)
    }

    /// Run a closure of a kept render (an effect setup or cleanup) as a task.
    fn start_closure(
        &mut self,
        view: &View,
        enc: &Encoder,
        origin: Origin,
        prepared: Rc<crate::Prepared>,
        f: Value,
    ) -> Result<Vec<Command>, String> {
        let id = self.next_handler;
        self.next_handler += 1;
        let at = self.journaled as u64;
        self.handlers.insert(
            id,
            Handler {
                origin,
                action: String::new(),
                which: 0,
                args: json!({"$args": []}),
                state: self.state.clone(),
                ctx: self.context(),
                clock_ms: self.clock_ms,
                effects: BTreeMap::new(),
                next_effect: 0,
                prepared: Some(prepared),
                closure: Some(f),
                at,
                consumed: at,
                done: None,
            },
        );
        self.run_handler(view, enc, id)
    }

    /// Stop the calls bound to the signals aborted since the last time. A model call stops
    /// generating and answers with what it has (`Command::Abort`); one waiting to retry answers
    /// now, with nothing. A machine call is cancelled and rejects with `AbortError`.
    fn take_aborts(&mut self, enc: &Encoder) -> Vec<Command> {
        let mut out = vec![];
        for signal in std::mem::take(&mut self.signals.fresh) {
            let slots: Vec<(u64, usize)> = self
                .open
                .iter()
                .filter(|(_, o)| o.signal == Some(signal))
                .map(|(slot, _)| *slot)
                .collect();
            for (h, i) in slots {
                if let Some(o) = self.open.get_mut(&(h, i)) {
                    o.signal = None;
                }
                let task = self
                    .tasks
                    .iter()
                    .find(|(_, (owner, _))| *owner == Some((h, i)))
                    .map(|(t, (_, cmd))| (*t, matches!(cmd, Command::Generate { .. })));
                match task {
                    Some((task, true)) => {
                        if self.aborting.insert(task) {
                            out.push(Command::Abort { task });
                        }
                    }
                    Some((task, false)) => {
                        self.tasks.remove(&task);
                        self.cancelled.push(task);
                        self.resolve(h, i, Outcome::Error(crate::effects::ABORTED.into()));
                    }
                    None if self.open.get_mut(&(h, i)).is_some_and(|o| o.retry.take().is_some()) => {
                        let outcome = self.aborted_reply(enc, h, i, &[]);
                        self.resolve(h, i, outcome);
                    }
                    None => {}
                }
            }
        }
        out
    }

    /// The outcome of a model call stopped by its signal, after generating `ids`.
    fn aborted_reply(&mut self, enc: &Encoder, h: u64, i: usize, ids: &[u32]) -> Outcome {
        let outcome = match self.open.get_mut(&(h, i)).and_then(|o| o.model.take()) {
            Some(call) => self.model_output(enc, call, ids, true),
            None => self.render_result(enc, h, i, ids, true),
        };
        marked_aborted(outcome)
    }

    fn resolve(&mut self, h: u64, i: usize, outcome: Outcome) {
        // The effect is answered: everything about it goes with its record.
        self.open.remove(&(h, i));
        if let Some(handler) = self.handlers.get_mut(&h)
            && let Some(request) = handler.effects.remove(&i)
        {
            self.realm.borrow_mut().reply(h, i, &request, &outcome);
        }
    }

    /// Execute one handler prefix or one queued continuation and issue its new requests.
    fn run_handler(&mut self, view: &View, enc: &Encoder, id: u64) -> Result<Vec<Command>, String> {
        let out = self.run_task(view, enc, id)?;
        self.finish_dispatches();
        Ok(out)
    }

    fn run_task(&mut self, view: &View, enc: &Encoder, id: u64) -> Result<Vec<Command>, String> {
        let mut out = vec![];
        let Some(h) = self.handlers.get(&id).cloned() else {
            return Ok(out);
        };
        self.realm.borrow_mut().current = Some(id);
        self.ran.entry(id).or_insert_with(|| h.origin.label());
        // The render it runs from: kept (its turn's, or rendered once when it first ran).
        let snapshot = match h.prepared.clone() {
            Some(p) => p,
            None => {
                let window = self.window();
                let evaluated = view
                    .evaluate(
                        enc,
                        h.state.clone(),
                        &self.cache,
                        date(h.clock_ms)?,
                        h.ctx.clone(),
                        window,
                    )
                    .and_then(|p| {
                        // A model-chosen action runs from the laid-out document; a listener does not need it.
                        if h.closure.is_none() && !crate::is_harness_callback(&h.action) {
                            view.layout(enc, &p.doc, &mut self.cache)?;
                        }
                        Ok(p)
                    });
                match evaluated {
                    Ok(p) => {
                        self.handlers.get_mut(&id).unwrap().prepared = Some(p.clone());
                        p
                    }
                    Err(e) => {
                        self.finish_handler(id, Err(e));
                        return Ok(out);
                    }
                }
            }
        };
        let prepared = match &h.closure {
            Some(f) => view.prepare_closure(enc, &snapshot, f),
            None => match view.prepare_from(enc, &snapshot, &h.action, h.which) {
                Ok(p) => p,
                Err(e) => {
                    self.finish_handler(id, Err(e));
                    return Ok(out);
                }
            },
        };
        {
            let next_effect = self.handlers[&id].next_effect;
            // Setters apply directly to the current hook state in execution order.
            let old = hooks(&self.state);
            let live = Rc::new(std::cell::RefCell::new(old.clone()));
            let run = prepared.step(
                &self.handlers[&id].args,
                next_effect,
                live.clone(),
                self.realm.clone(),
                id,
                (self.clock_ms, &mut self.rng, &mut self.signals),
            );
            let mut ui = live.borrow().clone();
            let changed: Vec<Rc<str>> = ui
                .iter()
                .filter(|(k, v)| old.get(&**k).is_none_or(|o| !same(o, v)))
                .map(|(k, _)| k.clone())
                .collect();
            if !changed.is_empty() || ui.len() != old.len() {
                // Large strings the handler wrote go to disk before they are kept.
                if let Some(b) = &self.blobs {
                    for k in &changed {
                        let v = b.lower_value(&ui[&*k.clone()])?;
                        ui.insert(k.clone(), v);
                    }
                }
                // What the agent keeps is frozen (a setter froze its value; lowering may have
                // rebuilt a parent).
                for k in &changed {
                    crate::value::freeze(&ui[&*k.clone()]);
                }
                // Refs (`useRef`, `~` keys) do not render: writes to them alone keep the last
                // evaluated document current (a render cannot read a ref).
                if changed.iter().any(|k| !k.starts_with('~')) || ui.len() != old.len() {
                    self.version += 1;
                }
                Rc::make_mut(&mut self.state)
                    .insert(Rc::from("__ui"), Value::Map(Rc::new(crate::value::Obj::frozen(ui))));
            }
            let mut run = match run {
                Ok(run) => run,
                Err(e) => {
                    out.extend(self.take_aborts(enc));
                    self.finish_handler(id, Err(e));
                    return Ok(out);
                }
            };
            if let Some(b) = &self.blobs {
                b.lower(&mut run.value)?;
            }
            for effect in run.effects {
                let handler = self.handlers.get_mut(&id).unwrap();
                let i = handler.next_effect;
                handler.next_effect += 1;
                handler.effects.insert(i, effect.request.clone());
                let signal = match &effect.request {
                    Request::Call { signal, .. } => *signal,
                    Request::Render { options, .. } => options["signal"].as_u64(),
                    _ => None,
                };
                self.open.insert(
                    (id, i),
                    Open {
                        signal,
                        ..Open::default()
                    },
                );
                match effect.request {
                    Request::Now => {
                        self.resolve(id, i, Outcome::Ok(json!(self.clock_ms)));
                    }
                    Request::Random => {
                        self.rng ^= self.rng << 13;
                        self.rng ^= self.rng >> 7;
                        self.rng ^= self.rng << 17;
                        self.resolve(id, i, Outcome::Ok(json!((self.rng >> 11) as f64 / (1u64 << 53) as f64)));
                    }
                    Request::Stop { value } => {
                        self.stopped = Some(value.clone());
                        self.note(json!({"t": "stop", "value": value}));
                        self.resolve(id, i, Outcome::Ok(J::Null));
                    }
                    // `model(<View/>)`, an ad-hoc call: its ids go to this instance's model; the reply resolves it.
                    // Options: `max_new` (default the instance's), `sampling` over the binding's, a
                    // `tag` for captures and traces, `priority` in admission. A prompt over
                    // `max_prompt` is never sent: the render fails and the handler can catch it.
                    Request::Render { ids, spans, options } => {
                        let call_max_new = options["max_new"].as_u64().map_or(self.cfg.max_new, |n| n as usize);
                        if let Some(e) = self.over_context(ids.len(), call_max_new, &spans) {
                            self.resolve(id, i, Outcome::Error(e));
                            continue;
                        }
                        if let Some(m) = self.cfg.max_prompt.filter(|m| ids.len() > *m) {
                            self.resolve(
                                id,
                                i,
                                Outcome::Error(format!(
                                    "model(): a prompt of {} tokens is over max_prompt ({m})",
                                    ids.len()
                                )),
                            );
                            continue;
                        }
                        let stop = protocol_stop(&spans);
                        let media = media_runs(&spans);
                        let images = match image_sources(&spans) {
                            Ok(images) => images,
                            Err(e) => {
                                self.resolve(id, i, Outcome::Error(format!("model(): {e}")));
                                continue;
                            }
                        };
                        self.metrics.prompt_tokens += ids.len() as u64;
                        self.metrics.uncached_tokens += ids.len() as u64;
                        self.metrics.max_prompt = self.metrics.max_prompt.max(ids.len());
                        // The call's own retry policy, if it states one, over its binding's.
                        match self.cfg.retry.with(&options["retry"]) {
                            Ok(policy) if !options["retry"].is_null() => {
                                self.open.entry((id, i)).or_default().policy = Some(policy)
                            }
                            Ok(_) => {}
                            Err(e) => {
                                self.resolve(id, i, Outcome::Error(format!("model(): {e}")));
                                continue;
                            }
                        }
                        let max_new = options["max_new"].as_u64().map_or(self.cfg.max_new, |n| n as usize);
                        let opts = GenOptions {
                            sampling: options["sampling"].clone(),
                            tag: options["tag"].as_str().map(str::to_owned),
                            priority: options["priority"].as_bool().unwrap_or(false),
                            abortable: self.open.get(&(id, i)).is_some_and(|o| o.signal.is_some()),
                            pipe: (!options["pipe"].is_null()).then(|| Box::new(options["pipe"].clone())),
                        };
                        out.push(self.new_task(Some((id, i)), |task| Command::Generate {
                            task,
                            ids: ids.into(),
                            stop,
                            max_new,
                            opts,
                            prompt_hash: None,
                            images,
                            media,
                        }));
                    }
                    Request::Call { app, args, .. } => match app.as_str() {
                        // `model(opts)`: the page's own view to the model.
                        "model" => {
                            let (outcome, commands) = self.issue_model(view, enc, (id, i), &args)?;
                            out.extend(commands);
                            if let Some(outcome) = outcome {
                                self.resolve(id, i, outcome);
                            }
                        }
                        // `dispatch(output)`: run its calls (the result arrives when they have answered).
                        "dispatch" => {
                            let only = args["calls"]
                                .as_array()
                                .map(|a| a.iter().map(|v| v.as_u64()).collect::<Vec<_>>());
                            let (outcome, commands) = self.dispatch(view, enc, (id, i), args["call"].as_u64(), only)?;
                            out.extend(commands);
                            if let Some(outcome) = outcome {
                                self.resolve(id, i, outcome);
                            }
                        }
                        // `harness.wait({ms, events})`: until `ms` passes (none: no deadline) or an event in `events` arrives.
                        "wait" => {
                            let wake_on: Vec<String> = args
                                .get("events")
                                .and_then(J::as_array)
                                .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect())
                                .unwrap_or_default();
                            // Milliseconds as `setTimeout` reads them: any number, truncated (a fraction is not "no deadline").
                            let until_ms = args
                                .get("ms")
                                .and_then(J::as_f64)
                                .filter(|ms| ms.is_finite())
                                .map_or(i64::MAX, |ms| self.clock_ms.saturating_add(ms.max(0.0) as i64));
                            // An event journaled after the render this task runs from (the model had
                            // not seen it), and after the last one a wait of this task consumed,
                            // wakes the wait at once.
                            let since = self.handlers.get(&id).map_or(u64::MAX, |h| h.at.max(h.consumed));
                            if let Some((at, name, payload)) = self
                                .event_log
                                .iter()
                                .find(|(at, name, _)| *at > since && wake_on.contains(name))
                                .cloned()
                            {
                                if let Some(handler) = self.handlers.get_mut(&id) {
                                    handler.consumed = at;
                                }
                                self.resolve(
                                    id,
                                    i,
                                    Outcome::Ok(json!({"woke": "event", "event": name, "payload": payload})),
                                );
                                continue;
                            }
                            self.open.entry((id, i)).or_default().wait = Some(Waiting { until_ms, wake_on });
                            if until_ms != i64::MAX {
                                out.push(Command::Timer { at_ms: until_ms });
                            }
                        }
                        // A machine call: only to a machine bound to this agent ("" is its first).
                        name if name.starts_with("machine:") => {
                            let role = &name["machine:".len()..];
                            let bound = if role.is_empty() {
                                self.cfg.machines.first()
                            } else {
                                self.cfg.machines.iter().find(|m| *m == role)
                            };
                            match bound.cloned() {
                                None => {
                                    let err = if role.is_empty() {
                                        "no machine is bound to this agent".to_string()
                                    } else {
                                        format!("no machine {role} is bound to this agent")
                                    };
                                    self.resolve(id, i, Outcome::Error(err));
                                }
                                Some(role) => {
                                    let mut args = args.clone();
                                    if let Some(b) = &self.blobs {
                                        b.lower(&mut args)?;
                                    }
                                    out.push(self.new_task(Some((id, i)), |task| Command::Call {
                                        task,
                                        name: format!("machine:{role}"),
                                        args,
                                    }));
                                    // Whose call it is: a frontend showing a command's output as it
                                    // runs (the observer's `Output`, by task) can put it under the tool
                                    // call that started it, also when several run at once.
                                    if let Origin::Tool { name, call_id, .. } = &h.origin {
                                        let task = self.next_task - 1;
                                        self.note(json!({"t": "call", "task": task, "machine": role, "tool": trace_tool_name(name), "call_id": call_id}));
                                    }
                                }
                            }
                        }
                        // A setter applies its write inside the interpreter. If its updater
                        // suspends, that write stays pending while its own requests settle.
                        "write" => {}
                        "parent" => {
                            out.push(Command::Callback {
                                name: args["name"].as_str().unwrap_or_default().into(),
                                args: args["args"].clone(),
                            });
                            self.resolve(id, i, Outcome::Ok(J::Null));
                        }
                        "send" | "store.put" | "store.get" | "store.search" | "store.list" | "inspect" => {
                            let mut args = args.clone();
                            if let Some(b) = &self.blobs {
                                b.lower(&mut args)?;
                            }
                            out.push(self.new_task(Some((id, i)), |task| Command::Call {
                                task,
                                name: app.clone(),
                                args: args.clone(),
                            }))
                        }
                        other => {
                            self.resolve(id, i, Outcome::Error(format!("{other} is not a harness API (model, dispatch, harness.machine.call, harness.store.*, harness.send, harness.inspect, ...)")));
                        }
                    },
                }
            }
            // Signals it aborted stop their calls, including ones it just made.
            out.extend(self.take_aborts(enc));
            if !self.handlers.contains_key(&id) {
                return Ok(out);
            }
            if run.done {
                // A setup's returned function is its cleanup.
                if let (Origin::Effect { key }, Value::Lambda(..) | Value::Func(_) | Value::Bound(_)) =
                    (&h.origin, &run.returned)
                    && let Some((_, cleanup)) = self.effect_state.get_mut(key)
                {
                    crate::value::freeze(&run.returned);
                    *cleanup = Some((snapshot.clone(), run.returned.clone()));
                }
                // Work it started and did not await lives on: the task ends when that settles.
                let effects = &self.handlers[&id].effects;
                let pending = !effects.is_empty() || self.realm.borrow().has_jobs(id);

                if pending {
                    // A tool has answered when its body returns (a Service Worker's `respondWith`;
                    // the work it did not await goes on, as with `waitUntil`).
                    if self.handlers[&id].done.is_none() {
                        self.respond(id, &h.origin, &Ok(run.value.clone()));
                        self.handlers.get_mut(&id).unwrap().done = Some(run.value);
                    }
                    return Ok(out);
                }
                self.finish_handler(id, Ok(run.value));
                return Ok(out);
            }
            Ok(out)
        }
    }

    /// A finished handler: a tool's return value (or error) becomes its dispatch's result.
    fn finish_handler(&mut self, id: u64, result: Result<J, String>) {
        let Some(h) = self.handlers.remove(&id) else { return };
        self.realm.borrow_mut().finish(id);
        // Its open effects end with it: a wait, or a model call waiting to be tried again, must
        // not outlive a handler that is gone, and none may keep the instance looking busy.
        self.open.retain(|(hid, _), _| *hid != id);
        let orphans: Vec<u64> = self
            .tasks
            .iter()
            .filter(|(_, (o, _))| o.is_some_and(|(hid, _)| hid == id))
            .map(|(t, _)| *t)
            .collect();
        for t in orphans {
            self.tasks.remove(&t);
            self.cancelled.push(t);
        }
        if let Err(e) = &result {
            self.metrics.handler_errors += 1;
            let origin = h.origin.label();
            self.note(json!({"t": "handler_error", "origin": origin, "error": e}));
        }
        // A tool whose body returned while its un-awaited work went on has answered already.
        if h.done.is_none() {
            self.respond(id, &h.origin, &result)
        }
    }

    /// A tool's answer: its return value (or error), as its result in its dispatch:
    /// `{name, id, result, error?, ...meta}`.
    fn respond(&mut self, _handler: u64, origin: &Origin, result: &Result<J, String>) {
        if let Origin::Tool {
            name,
            call_id,
            dispatch,
            index,
            started_ms,
        } = origin
        {
            let record = match result {
                // `{result, ...meta}`: the text is the tool result; the other keys are metadata the
                // app keeps (templates render only the result).
                Ok(J::Object(o)) if o.contains_key("result") => {
                    let mut r = o.clone();
                    r.insert("result".into(), result_value(&o["result"]));
                    r.insert("name".into(), json!(name));
                    r.insert("id".into(), json!(call_id));
                    J::Object(r)
                }
                Ok(v) => json!({"name": name, "id": call_id, "result": result_value(v)}),
                Err(e) => {
                    json!({"name": name, "id": call_id, "result": format!("error: {}", crate::error_message(e)), "error": true})
                }
            };
            // The semantic answer, not its machine/store/send subcalls or the later end of
            // unawaited work. `done` and the dispatch slot ensure one completion. Keep only
            // bounded identifiers and scalar metadata: never arguments, results or errors.
            // `at_ms` is its start; `turn` is the page call whose reply asked for it.
            let mut entry = None;
            let n = self.dispatches.get(dispatch).map(|d| d.call);
            if let Some(slot) = n
                .and_then(|n| self.kept.get_mut(&n))
                .and_then(|k| k.results.get_mut(*index))
            {
                if slot.is_none() {
                    let timed_out = record["timed_out"] == true;
                    let error = result.is_err()
                        || record["error"] == true
                        || timed_out
                        || record["exit"].as_i64().is_some_and(|exit| exit != 0);
                    entry = Some(
                        json!({"t": "tool", "tool": trace_tool_name(name), "at_ms": *started_ms, "turn": n,
                        "dispatch": dispatch, "index": index, "ms": self.clock_ms.saturating_sub(*started_ms).max(0),
                        "status": if error { "error" } else { "ok" }, "timed_out": timed_out}),
                    );
                }
                *slot = Some(record);
            }
            if let Some(entry) = entry {
                self.note(entry);
            }
        }
    }
}

/// Bound application-defined names without merging distinct long names into one metric key.
fn trace_tool_name(name: &str) -> String {
    if name.chars().count() <= 128 {
        return name.to_owned();
    }
    use sha2::Digest;
    let mut hash = sha2::Sha256::default();
    hash.update(name.as_bytes());
    format!("{}#{}", name.chars().take(63).collect::<String>(), hex_digest(&hash))
}

/// The stop ids a template app's `protocol` span declares.
fn protocol_stop(spans: &[crate::emit::Span]) -> Vec<u32> {
    spans
        .iter()
        .find(|s| s.label == "protocol" && s.kind == "label")
        .and_then(|p| {
            p.meta["stop"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_u64().map(|x| x as u32)).collect())
        })
        .unwrap_or_default()
}

/// The identity of a prompt (sha256 over its ids, little-endian u32).
pub fn prompt_hash(ids: &[u32]) -> String {
    let mut h = sha2::Sha256::default();
    sha_ids(&mut h, ids);
    hex_digest(&h)
}

fn sha_ids(h: &mut sha2::Sha256, ids: &[u32]) {
    use sha2::Digest;
    for chunk in ids.chunks(4096) {
        let bytes: Vec<u8> = chunk.iter().flat_map(|i| i.to_le_bytes()).collect();
        h.update(&bytes);
    }
}

fn hex_digest(h: &sha2::Sha256) -> String {
    use sha2::Digest;
    h.clone().finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// The tokens a prompt sends: a hosted prompt's JSON line (its `hosted` span), else all of its ids.
fn sent_tokens(ids: usize, spans: &[crate::emit::Span]) -> usize {
    spans
        .iter()
        .find(|s| s.label == "hosted" && s.kind == "label")
        .map_or(ids, |s| s.end - s.start)
}

#[cfg(test)]
mod tests {}
