//! The deployment: what a run is configured with, and each agent's spec.
use super::*;

/// How a run is configured: the app, its threads and limits, and where its files go.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// The app: one view file (and its imports) with its root component.
    pub view: PathBuf,
    /// The root component's name (the file's default export when it has no such function).
    pub entry: String,
    /// Render worker threads; each owns a fixed set of agents.
    pub workers: usize,
    /// Maximum event/result members in a committed input frame (1..64); 1 disables grouping.
    pub input_batch: usize,
    /// Threads that make model calls.
    pub model_threads: usize,
    /// Threads that run machine commands and other host calls.
    pub exec_threads: usize,
    /// Model calls in flight at once (the rest wait in arrival order); `None`: `model_threads`.
    /// `Control::set_model_limit` changes it while running, so a controller outside the runtime
    /// can steer it from the model server's own metrics.
    pub model_inflight: Option<usize>,
    /// Where every model request and response is written, by agent (diagnostics, like a browser's
    /// network log; the command sets it from `AFFORDANCE_CAPTURE_DIR`).
    pub capture_dir: Option<PathBuf>,
    /// Instances restoring at once (replaying a journal or loading an image); the others wait
    /// their turn. Each restore holds its whole journal's working set for a moment, so the boot
    /// peak is the steady heap plus this many of them. `None`: the CPUs this process may use.
    pub restore_concurrency: Option<usize>,
    /// The library apps may import from (the installed views), besides their own directory.
    /// `None`: the views tree `view` is installed in (the nearest directory above it with an
    /// `index.json`). Every app the runtime loads, mounted ones included, is confined to the two.
    pub library: Option<PathBuf>,
    /// Journals, traces and storage; `None` keeps everything in memory.
    pub dir: Option<PathBuf>,
    /// With `dir`, strings of at least this many bytes live in `<dir>/blobs`, mapped from disk
    /// instead of held on the heap (`blobs.rs`).
    pub spill_bytes: usize,
    /// Put each agent's state (its hooks, its log included) in the run's `Report`. A fleet runner
    /// that does not read it turns this off, so a shutdown does not materialize every log at once
    /// (each agent's `snapshot.json` has it).
    pub report_state: bool,
    /// On restore, refuse an agent whose journal no longer replays to the same prompts (instead of
    /// recording `diverged_at` and continuing).
    pub strict_replay: bool,
    /// The deployment layer of every agent's environ.
    pub env: BTreeMap<String, String>,
    /// Environ keys whose values are redacted in traces and in `agent.json`.
    pub secrets: BTreeSet<String>,
    /// Give up after this long (agents keep their journals; `restore` continues them).
    pub max_wall: Option<Duration>,
    /// Keep running while instances are idle (a chat waiting for its user, a server waiting for
    /// requests), until every instance stops, `max_wall`, or `Control::close`. Without it (batch),
    /// the run returns once nothing is in flight and no instance waits on anything.
    pub serve: bool,
    /// Bound on a single executor call.
    pub call_timeout: Duration,
    /// Sees what every instance's calls produce as it arrives: a model's new ids or text, a
    /// command's output. For live views (a frontend, a dashboard); never journaled, so what it
    /// sees changes nothing. Runs on model and machine threads; keep it fast.
    pub observer: Option<Observer>,
    /// The observer wants a model's reply as it is generated. With it, every model call is
    /// streamed; without (`Runtime::observe_traces`), the observer sees trace records and command
    /// output only, and a call goes out as its binding says.
    pub observe_deltas: bool,
}

/// What an observer sees of one call (`RuntimeConfig.observer`), as it arrives.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Observed<'a> {
    /// A native model's new ids.
    Ids(&'a [u32]),
    /// A hosted model's new reply text.
    Text(&'a str),
    /// A hosted model's new reasoning text.
    Reasoning(&'a str),
    /// More of a hosted model's tool call (`Delta::Call`): its place in the reply, its id and name
    /// when they arrive, and more of its arguments' JSON text.
    Call {
        /// The call's place among the reply's calls, from 0.
        index: usize,
        /// The call's id, on the piece that carries it.
        id: Option<&'a str>,
        /// The tool's name, on the piece that carries it.
        name: Option<&'a str>,
        /// More of the arguments' JSON text (may be empty).
        arguments: &'a str,
    },
    /// A machine call's new output bytes (a command's stdout and stderr, as it writes them).
    Output(&'a [u8]),
    /// A record of the instance's trace (a render, a reply, a model call's statistics, a finished tool, a handler error), as
    /// written to its `trace.jsonl`, secrets hidden. Its task is 0.
    Trace(&'a J),
}

impl<'a> From<crate::runtime::Delta<'a>> for Observed<'a> {
    fn from(delta: crate::runtime::Delta<'a>) -> Self {
        match delta {
            crate::runtime::Delta::Ids(ids) => Observed::Ids(ids),
            crate::runtime::Delta::Text(text) => Observed::Text(text),
            crate::runtime::Delta::Reasoning(text) => Observed::Reasoning(text),
            crate::runtime::Delta::Call {
                index,
                id,
                name,
                arguments,
            } => Observed::Call {
                index,
                id,
                name,
                arguments,
            },
        }
    }
}

/// A live view of calls: `(instance, task, what arrived)`.
#[derive(Clone)]
pub struct Observer(pub Arc<dyn Fn(&str, u64, Observed<'_>) + Send + Sync>);

impl std::fmt::Debug for Observer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Observer")
    }
}

impl RuntimeConfig {
    /// A run of the app at `view` (entry `Main`) with default threads and limits.
    pub fn new(view: impl Into<PathBuf>) -> Self {
        RuntimeConfig {
            view: view.into(),
            entry: "Main".into(),
            workers: 4,
            input_batch: 1,
            model_threads: 64,
            exec_threads: 64,
            model_inflight: None,
            capture_dir: None,
            restore_concurrency: None,
            library: None,
            dir: None,
            spill_bytes: super::super::blobs::DEFAULT_MIN_BYTES,
            report_state: true,
            strict_replay: false,
            env: BTreeMap::new(),
            secrets: BTreeSet::new(),
            max_wall: None,
            serve: false,
            call_timeout: Duration::from_secs(3600),
            observer: None,
            observe_deltas: true,
        }
    }
}

/// One agent to run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSpec {
    /// The agent's id: its directory name, and how other agents address it.
    pub id: String,
    /// A private render worker for this startup root and its descendants.
    #[serde(default, skip_serializing_if = "is_no")]
    pub dedicated_worker: bool,
    /// The root component's props.
    pub props: J,
    /// The per-agent layer of the environ (over the deployment layer).
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    /// The most tokens a reply may generate.
    pub max_new: usize,
    /// Page model calls before the agent stops (`max_turns`); `None`: no limit.
    #[serde(default)]
    pub max_turns: Option<u64>,
    /// The deployment's machines bound to this agent, by role; `None`: all of them.
    #[serde(default)]
    pub machines: Option<Vec<String>>,
    /// The platform APIs this agent may call (`send`, `store.*`; a trailing `*` matches a prefix);
    /// `None`: all. Machines are bound by `machines`, not here. Children inherit their parent's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Vec<String>>,
    /// The instances this agent may `harness.inspect`, by id prefix (`"*"` or `""`: any). None by
    /// default, never implied by `capabilities`, and not inherited by children: a copy of another
    /// instance's state holds its secrets too.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inspect: Vec<String>,
    /// `false`: no instance may inspect this one (nor its children).
    #[serde(default = "yes", skip_serializing_if = "is_yes")]
    pub inspectable: bool,
    /// The hard prompt ceiling (see `AgentConfig::max_prompt`); children inherit it.
    #[serde(default)]
    pub max_prompt: Option<usize>,
    /// Data-parallel rank to pin to; `None`: a stable hash of the id.
    #[serde(default)]
    pub rank: Option<usize>,
    /// The seed of `Math.random` and `harness.random`.
    #[serde(default)]
    pub seed: u64,
    /// Its app (a .tsx file); `None`: the runtime's `view`.
    #[serde(default)]
    pub app: Option<String>,
    /// The app's entry component; `None`: the runtime's `entry`.
    #[serde(default)]
    pub entry: Option<String>,
    /// The model binding it takes turns on; `null`: none (it only reacts, e.g. a root app).
    #[serde(default = "default_model")]
    pub model: Option<String>,
}

pub(super) fn default_model() -> Option<String> {
    Some("default".into())
}
pub(super) fn yes() -> bool {
    true
}
pub(super) fn is_yes(b: &bool) -> bool {
    *b
}
pub(super) fn is_no(b: &bool) -> bool {
    !*b
}

impl AgentSpec {
    /// An agent `id` with root props `props` and default limits.
    pub fn new(id: &str, props: J) -> Self {
        AgentSpec {
            id: id.into(),
            dedicated_worker: false,
            props,
            env: BTreeMap::new(),
            max_new: 2048,
            max_turns: None,
            max_prompt: None,
            machines: None,
            capabilities: None,
            inspect: vec![],
            inspectable: true,
            rank: None,
            seed: 1,
            app: None,
            entry: None,
            model: default_model(),
        }
    }

    /// Whether this agent may call the platform API `name` (`send`, `store.put`, ...).
    pub fn allows(&self, name: &str) -> bool {
        self.capabilities.as_ref().is_none_or(|caps| {
            caps.iter()
                .any(|c| c == name || c.strip_suffix('*').is_some_and(|p| name.starts_with(p)))
        })
    }

    /// Whether this agent may inspect instance `target`.
    pub fn may_inspect(&self, target: &str) -> bool {
        self.inspect.iter().any(|p| target.starts_with(p.trim_end_matches('*')))
    }
}
