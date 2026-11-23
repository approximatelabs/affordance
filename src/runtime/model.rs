//! The model client: token ids in, token ids out. Implementations for SGLang's native
//! `/generate` and OpenAI-style `/v1/completions` with token-id prompts (vLLM), plus a closure
//! model for tests and scripted runs. The runtime pins each agent to a data-parallel rank so
//! its prefix stays in one rank's cache.
use super::cancel::CancelToken;
use super::http::{Failure, HttpError, get_json, ids_body, post_json, post_sse};
pub use super::model_config::{Credentials, ModelTemplate, from_config, with_context, with_template};
use super::model_config::{auth_headers, fingerprint, validate_native, validate_options, validate_url};
use serde_json::{Value as J, json};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

mod hosted;
mod hosted_stream;

/// The start of text a server sent, for an error message: errors are journaled, and a server
/// must not be able to make one large.
fn brief(text: &str) -> String {
    const MAX: usize = 300;
    match text.char_indices().nth(MAX) {
        Some((at, _)) => format!("{}... ({} bytes)", &text[..at], text.len()),
        None => text.to_string(),
    }
}

/// An `error` a server sends as an event of a stream that began with 200 (out of memory, a
/// request it refuses late): the call's failure, in the server's words, not a stream that
/// "ended without a finish reason".
fn stream_error(event: &J) -> Result<(), HttpError> {
    match event.get("error").filter(|e| !e.is_null()) {
        Some(error) => Err(HttpError::protocol(format!(
            "the server reported: {}",
            brief(&error.to_string())
        ))),
        None => Ok(()),
    }
}

/// A status that is not a success, as the failure it is: a call that is not streamed fails the
/// way a streamed one does.
fn retryable_status(result: Result<(u16, J), HttpError>, retry_after: Option<String>) -> Result<(u16, J), HttpError> {
    match result {
        Ok((status, body)) if super::http::status_may_pass(status) => {
            let mut error = HttpError::new(
                Failure::Status(status),
                format!("HTTP {status}: {}", brief(&body.to_string())),
            );
            error.retry_after = retry_after;
            Err(error)
        }
        other => other,
    }
}
/// A JSON response with its `Retry-After` header beside it.
fn asked(result: Result<(u16, J, Option<String>), HttpError>) -> (Result<(u16, J), HttpError>, Option<String>) {
    match result {
        Ok((status, body, retry_after)) => (Ok((status, body)), retry_after),
        Err(e) => (Err(e), None),
    }
}

static RID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// What one run notes about its model calls, and where it captures them: each runtime has its own
/// (`RuntimeConfig.capture_dir`), so two runs in one process never count each other's calls. A
/// request carries its run's (`GenRequest.diagnostics`); the default captures nothing.
#[derive(Debug, Clone, Default)]
pub struct Diagnostics(std::sync::Arc<DiagnosticsInner>);

#[derive(Debug, Default)]
struct DiagnosticsInner {
    capture_dir: Option<std::path::PathBuf>,
    capture_errors: AtomicU64,
    token_mismatches: AtomicU64,
}

impl Diagnostics {
    /// Counters for one run, writing every request and response under `capture_dir` if given.
    pub fn new(capture_dir: Option<std::path::PathBuf>) -> Self {
        Diagnostics(std::sync::Arc::new(DiagnosticsInner {
            capture_dir,
            ..Default::default()
        }))
    }
    /// Model calls whose prompt the server counted differently from the tokens laid out. It
    /// should be zero: the tokens laid out are the tokens the model reads. A file whose size the
    /// layout has wrong, or a server that tokenizes a prompt again, shows here.
    pub fn token_mismatches(&self) -> u64 {
        self.0.token_mismatches.load(Ordering::Relaxed)
    }
    /// Captures that could not be written.
    pub fn capture_errors(&self) -> u64 {
        self.0.capture_errors.load(Ordering::Relaxed)
    }
}

/// A token-id server reported how many prompt tokens it processed for `req`.
fn prompt_counted(req: &GenRequest, counted: Option<u64>) {
    let Some(counted) = counted else { return };
    if counted != req.ids.len() as u64 && req.diagnostics.0.token_mismatches.fetch_add(1, Ordering::Relaxed) < 5 {
        eprintln!(
            "affordance: {}: the prompt was laid out as {} tokens and the server counted {counted}. The model did not read the tokens that were laid out (a picture, clip or video whose size is wrong, or a server that tokenized the prompt again)",
            req.agent,
            req.ids.len()
        );
    }
}

/// The run's capture directory (`RuntimeConfig.capture_dir`): every request and response (and
/// stream event) is written there, by agent.
/// `data` (the JSON of what is captured) is only built when capturing is on: a request body holds
/// the whole prompt. A capture is diagnostics, like a browser's network log: a failed write (a
/// full disk) never fails the call. It is counted, logged at the 1st, 2nd, 4th, ... failure, and
/// leaves no file. Each capture is renamed into place (a reader never sees part of one) but not
/// synced: a crash can lose the last captures, which a journal would need and a log does not.
fn capture(req: &GenRequest, rid: &str, kind: &str, data: impl FnOnce() -> Vec<u8>) {
    let agent = req.agent.as_str();
    let Some(root) = &req.diagnostics.0.capture_dir else {
        return;
    };
    // The instance's name, escaped into one path component: never a path of its own.
    let dir = root.join(super::exec::instance_component(agent));
    let path = dir.join(format!("{rid}.{kind}.json"));
    let temporary = path.with_extension("tmp");
    let written = std::fs::create_dir_all(&dir).and_then(|_| {
        let head = json!({"at": chrono::Utc::now().timestamp_millis() as f64/1000.0,"request_id":rid,"agent":agent,"tag":req.tag}).to_string();
        let mut out = head.into_bytes();
        out.pop();
        out.extend_from_slice(b",\"data\":");
        out.extend(data());
        out.push(b'}');
        std::fs::write(&temporary, out)?;
        std::fs::rename(&temporary, &path)
    });
    if let Err(e) = written {
        let _ = std::fs::remove_file(&temporary);
        let n = req.diagnostics.0.capture_errors.fetch_add(1, Ordering::Relaxed) + 1;
        if n.is_power_of_two() {
            eprintln!("request capture failed ({n} so far; the calls go on): {e}")
        }
    }
}

/// A request's capture: `{"body": <the body as sent>, "headers": [...]}`.
fn request_capture(body: &[u8], headers: &[(&str, String)]) -> Vec<u8> {
    let mut out = b"{\"body\":".to_vec();
    out.extend_from_slice(body);
    out.extend_from_slice(b",\"headers\":");
    let safe: Vec<_> = headers
        .iter()
        .map(|(name, value)| {
            (
                *name,
                if credential(name, value).is_some() {
                    "[redacted]"
                } else {
                    value.as_str()
                },
            )
        })
        .collect();
    out.extend(serde_json::to_vec(&safe).unwrap());
    out.push(b'}');
    out
}

/// The headers a hosted binding may carry its key in (`key_header`): a bearer token, or an API
/// key as is (Anthropic, Azure OpenAI, Google).
pub(crate) const KEY_HEADERS: [&str; 4] = ["authorization", "x-api-key", "api-key", "x-goog-api-key"];

/// The secret a credential header carries: a bearer token or an API key.
fn credential<'a>(name: &str, value: &'a str) -> Option<&'a str> {
    if name.eq_ignore_ascii_case("authorization") {
        value.strip_prefix("Bearer ")
    } else if KEY_HEADERS.iter().any(|h| name.eq_ignore_ascii_case(h)) {
        Some(value)
    } else {
        None
    }
}

/// The shortest credential that is hidden wherever a server echoes it.
const MIN_SECRET: usize = 8;

// Providers can echo an invalid key in errors or diagnostic metadata. Keep those paths
// private too; native token IDs and generated content are left to the provider's protocol.
fn redact_credentials(mut text: String, headers: &[(&str, String)]) -> String {
    // A real key is long. A placeholder a local server is given (`none`, `EMPTY`, `token`) is an
    // ordinary word: replacing it everywhere would rewrite metadata (`prompt_tokens`) and names.
    for key in headers
        .iter()
        .filter_map(|(name, value)| credential(name, value))
        .filter(|key| key.len() >= MIN_SECRET)
    {
        let escaped = serde_json::to_string(key).unwrap();
        text = text
            .replace(&escaped[1..escaped.len() - 1], "[redacted]")
            .replace(key, "[redacted]");
    }
    text
}
fn private_metadata(mut value: J, headers: &[(&str, String)]) -> J {
    if !headers.iter().any(|(name, value)| credential(name, value).is_some()) {
        return value;
    }
    let mut work = vec![&mut value];
    while let Some(value) = work.pop() {
        match value {
            J::String(text) => *text = redact_credentials(std::mem::take(text), headers),
            J::Array(items) => work.extend(items.iter_mut()),
            J::Object(items) => {
                *items = std::mem::take(items)
                    .into_iter()
                    .map(|(key, value)| (redact_credentials(key, headers), value))
                    .collect();
                work.extend(items.values_mut());
            }
            _ => {}
        }
    }
    value
}

/// A locally-unique id for one logical generate call, so a failed attempt can be aborted by
/// name on the server instead of left for its own (much slower) disconnect-polling to notice.
fn new_rid() -> String {
    static INSTANCE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    let instance = INSTANCE.get_or_init(|| {
        // A random id distinguishes processes, hosts, and resumed generations: the kernel's UUID
        // on Linux, else 16 bytes of the system's randomness.
        std::fs::read_to_string("/proc/sys/kernel/random/uuid")
            .map(|uuid| uuid.trim().to_string())
            .unwrap_or_else(|_| {
                use std::io::Read;
                let mut bytes = [0u8; 16];
                std::fs::File::open("/dev/urandom")
                    .and_then(|mut f| f.read_exact(&mut bytes))
                    .expect("request namespace randomness");
                bytes.iter().map(|b| format!("{b:02x}")).collect()
            })
    });
    format!("{instance}-{}", RID_COUNTER.fetch_add(1, Ordering::Relaxed))
}

#[derive(Debug, Clone)]
/// One model call: a prompt's token ids and how to generate from them.
pub struct GenRequest {
    /// The instance that asks (for rank pinning, sessions and captures).
    pub agent: String,
    /// The prompt.
    pub ids: std::sync::Arc<[u32]>,
    /// The most tokens to generate.
    pub max_new: usize,
    /// Token ids that end the reply.
    pub stop: Vec<u32>,
    /// The data-parallel rank this agent is pinned to.
    pub rank: Option<usize>,
    /// Cancelled when the reply is no longer wanted: the request is aborted.
    pub cancel: CancelToken,
    /// Merged over the client's own sampling (a render's options); `null` for a turn.
    pub sampling: J,
    /// Recorded in the request's capture (`harness.render`'s `tag`).
    pub tag: Option<String>,
    /// The prompt's images (data URLs, `agent::image_source`), one per placeholder, in order.
    pub images: Vec<String>,
    /// Where each media element's tokens are in `ids`, and its placeholder: (start, count,
    /// placeholder).
    pub media: Vec<(usize, usize, u32)>,
    /// The run's counters and capture directory.
    pub diagnostics: Diagnostics,
}

impl GenRequest {
    /// A request with no sampling overrides, tag or images.
    pub fn new(
        agent: &str,
        ids: impl Into<std::sync::Arc<[u32]>>,
        max_new: usize,
        stop: Vec<u32>,
        rank: Option<usize>,
        cancel: CancelToken,
    ) -> Self {
        GenRequest {
            agent: agent.into(),
            ids: ids.into(),
            max_new,
            stop,
            rank,
            cancel,
            sampling: J::Null,
            tag: None,
            images: vec![],
            media: vec![],
            diagnostics: Diagnostics::default(),
        }
    }
}

/// `over` merged into the JSON object `into`.
fn merge(into: &mut J, over: &J) {
    for (k, v) in over.as_object().into_iter().flatten() {
        into[k] = v.clone();
    }
}

#[derive(Debug, Clone, Default)]
/// What a model call returned.
pub struct GenReply {
    /// Generated ids, including the stop token that ended generation (if one did).
    pub ids: Vec<u32>,
    /// Generation stopped at `max_new` tokens.
    pub cut_off: bool,
    /// Prompt tokens the server reports as served from its prefix cache.
    pub cached_tokens: Option<u64>,
    /// Milliseconds until the first generated token arrived (streaming clients).
    pub first_token_ms: Option<f64>,
    /// Server metadata kept verbatim, including actual cache tiers and finish/usage details.
    /// These are observations from the serving engine, separate from logical prefix overlap.
    pub model_meta: Option<J>,
    /// Id of the completed native request attempt, when the protocol supports client ids.
    pub request_id: Option<String>,
    /// Number of transport attempts made for this logical call.
    pub attempts: usize,
}

/// One failed attempt at a model call: what failed, and what kind of failure it was. The
/// instance's retry policy decides from the kind whether the call is made again.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelError {
    /// What failed, without the server's address.
    pub message: String,
    /// What kind of failure it was: what a retry policy decides on.
    pub kind: FailureKind,
    /// How long the server asked the caller to wait before trying again (`Retry-After`).
    pub retry_after: Option<Duration>,
}

/// The kinds of failure a model call can end in. A client makes one attempt and reports one of
/// these; whether the call is made again, and when, is the retry policy's (`RetryPolicy`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    /// Nothing was sent: the server refused the connection, has no address, or did not answer
    /// the connect in time.
    Unreachable,
    /// The request was sent and the exchange broke: cut, reset, stalled or timed out.
    Interrupted,
    /// The server said it is busy: HTTP 429, 503 or 529.
    Busy,
    /// The server failed: another 5xx.
    Server,
    /// The server refused the request (another 4xx), or its reply broke the protocol: sending
    /// the same request again will not change the answer.
    Refused,
    /// This side stopped the call. Never tried again.
    Cancelled,
    /// The call failed after part of its reply was written to a pipe. Never tried again: whoever
    /// reads the pipe would get the beginning twice.
    Partial,
}

/// What happens after a model call fails: the one retry policy. A binding states it (`retry` in
/// a deployment), a call may state its own (`model(view, {retry: ..})`), and the instance applies
/// it on its journaled clock, so every wait and every further attempt is in the journal and the
/// trace, and a replay retries the same way. Model clients never retry.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RetryPolicy {
    /// How long, from the first failure, a call is tried again; `None`: for as long as it fails.
    pub for_ms: Option<u64>,
    /// The first wait and the longest; the wait doubles from one to the other.
    pub wait_ms: (u64, u64),
    /// The kinds of failure that are tried again within `for_ms`.
    pub on: Vec<FailureKind>,
    /// Further attempts after a `refused` failure (which the same request will meet again).
    pub refused: u32,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        RetryPolicy {
            for_ms: None,
            wait_ms: (1_000, 30_000),
            on: vec![
                FailureKind::Unreachable,
                FailureKind::Interrupted,
                FailureKind::Busy,
                FailureKind::Server,
            ],
            refused: 0,
        }
    }
}

/// The longest a server's `Retry-After` is honored for: past it, the policy's own wait applies.
const RETRY_AFTER_MAX_MS: u64 = 300_000;

impl RetryPolicy {
    /// The policy `value` states over `self`: `false` (no further attempt at all), or an object
    /// with any of `for_s` (seconds, or "forever"), `wait_s` (`[first, longest]`), `on` (kinds'
    /// names) and `refused` (a count).
    pub fn with(&self, value: &J) -> Result<Self, String> {
        let mut policy = self.clone();
        match value {
            J::Null => {}
            J::Bool(false) => {
                policy.on = vec![];
                policy.refused = 0;
            }
            J::Object(fields) => {
                for (key, v) in fields {
                    match key.as_str() {
                        "for_s" => {
                            policy.for_ms = match v {
                                J::String(s) if s == "forever" => None,
                                v => Some(
                                    v.as_f64()
                                        .filter(|s| s.is_finite() && *s >= 0.0)
                                        .map(|s| (s * 1000.0) as u64)
                                        .ok_or("retry.for_s must be a number of seconds, or \"forever\"")?,
                                ),
                            }
                        }
                        "wait_s" => {
                            let seconds: Vec<f64> = v
                                .as_array()
                                .into_iter()
                                .flatten()
                                .filter_map(J::as_f64)
                                .filter(|s| s.is_finite() && *s >= 0.0)
                                .collect();
                            let [first, longest] = seconds[..] else {
                                return Err("retry.wait_s must be [first, longest], in seconds".into());
                            };
                            policy.wait_ms = ((first * 1000.0) as u64, (longest.max(first) * 1000.0) as u64);
                        }
                        "on" => {
                            policy.on = v
                                .as_array()
                                .ok_or("retry.on must be a list of failure kinds")?
                                .iter()
                                .map(|name| {
                                    name.as_str()
                                        .and_then(FailureKind::named)
                                        .filter(|kind| kind.may_pass())
                                        .ok_or_else(|| {
                                            format!(
                                                "retry.on: {name} is not one of unreachable, interrupted, busy, server"
                                            )
                                        })
                                })
                                .collect::<Result<_, _>>()?;
                        }
                        "refused" => {
                            policy.refused = v
                                .as_u64()
                                .and_then(|n| u32::try_from(n).ok())
                                .ok_or("retry.refused must be a count")?
                        }
                        other => return Err(format!("retry: unknown field `{other}` (for_s, wait_s, on, refused)")),
                    }
                }
            }
            _ => return Err("retry must be false or {for_s, wait_s, on, refused}".into()),
        }
        Ok(policy)
    }

    /// After failure number `failures` of a call (of `kind`, at `now_ms`, the first at
    /// `since_ms`, the server asking for `retry_after_ms`): how long to wait before the next
    /// attempt, or `None` when there is none and the caller gets the failure.
    pub fn next(
        &self,
        kind: FailureKind,
        failures: u32,
        since_ms: i64,
        now_ms: i64,
        retry_after_ms: Option<u64>,
    ) -> Option<u64> {
        let doubled = self
            .wait_ms
            .0
            .saturating_mul(1u64 << failures.saturating_sub(1).min(20));
        let wait = doubled
            .min(self.wait_ms.1)
            .max(retry_after_ms.unwrap_or(0).min(RETRY_AFTER_MAX_MS));
        match kind {
            FailureKind::Cancelled | FailureKind::Partial => None,
            FailureKind::Refused => (failures <= self.refused).then_some(wait),
            kind if self.on.contains(&kind) => self
                .for_ms
                .is_none_or(|limit| (now_ms - since_ms).max(0) as u64 + wait <= limit)
                .then_some(wait),
            _ => None,
        }
    }
}

impl FailureKind {
    /// Its name in a deployment, a journal and a trace.
    pub fn name(self) -> &'static str {
        match self {
            FailureKind::Unreachable => "unreachable",
            FailureKind::Interrupted => "interrupted",
            FailureKind::Busy => "busy",
            FailureKind::Server => "server",
            FailureKind::Refused => "refused",
            FailureKind::Cancelled => "cancelled",
            FailureKind::Partial => "partial",
        }
    }
    /// The kind of that name.
    pub fn named(name: &str) -> Option<Self> {
        [
            FailureKind::Unreachable,
            FailureKind::Interrupted,
            FailureKind::Busy,
            FailureKind::Server,
            FailureKind::Refused,
            FailureKind::Cancelled,
            FailureKind::Partial,
        ]
        .into_iter()
        .find(|kind| kind.name() == name)
    }
    /// The kind of an HTTP status that is not a success.
    pub fn of_status(status: u16) -> Self {
        match status {
            429 | 503 | 529 => FailureKind::Busy,
            500..=599 => FailureKind::Server,
            _ => FailureKind::Refused,
        }
    }
    /// A failure a later attempt may not meet: the default kinds a policy tries again.
    pub fn may_pass(self) -> bool {
        matches!(
            self,
            FailureKind::Unreachable | FailureKind::Interrupted | FailureKind::Busy | FailureKind::Server
        )
    }
}

impl ModelError {
    /// A failure that retrying will not fix (a rejected request, a malformed reply).
    pub fn fatal(message: impl Into<String>) -> Self {
        Self::of(FailureKind::Refused, message)
    }
    /// A failure of `kind`.
    pub fn of(kind: FailureKind, message: impl Into<String>) -> Self {
        ModelError {
            message: message.into(),
            kind,
            retry_after: None,
        }
    }
    /// The call was stopped by this side.
    pub fn cancelled(message: impl Into<String>) -> Self {
        Self::of(FailureKind::Cancelled, message)
    }
    /// An error from the HTTP transport, as the kind its failure is.
    fn transport(e: HttpError) -> Self {
        let kind = match e.failure {
            Failure::Unsent => FailureKind::Unreachable,
            Failure::Transport if e.message.starts_with("cancelled") => FailureKind::Cancelled,
            Failure::Transport => FailureKind::Interrupted,
            Failure::Status(status) => FailureKind::of_status(status),
            Failure::Protocol => FailureKind::Refused,
        };
        let mut error = Self::of(kind, e.message);
        error.retry_after = e.retry_after.and_then(|after| hosted::retry_delay(&after, &[]));
        error
    }
    fn status(status: u16, message: String) -> Self {
        Self::of(FailureKind::of_status(status), message)
    }

    /// A transport failure of `endpoint` (e.g. `sglang /generate`), classified on the transport's
    /// own message, then named by the binding and endpoint, never the server's address.
    fn transport_of(endpoint: &str, raw: HttpError, url: &str) -> Self {
        let mut e = Self::transport(raw);
        e.message = without_endpoint(&format!("{endpoint}: {}", e.message), url);
        e
    }

    /// The message without the model server's address (see `without_endpoint`).
    fn without_endpoint(mut self, url: &str) -> Self {
        self.message = without_endpoint(&self.message, url);
        self
    }
}

/// `message` with the model server's address (`url`, its host:port and its host) replaced by
/// "the model server". Errors reach journals, traces, captures and apps; where the server is
/// belongs to the deployment (the binding's config and the operator's log), not to run data.
fn without_endpoint(message: &str, url: &str) -> String {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    let hostport = rest.split('/').next().unwrap_or("");
    let host = match hostport.rsplit_once(':') {
        Some((h, p)) if !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()) => h,
        _ => hostport,
    };
    let mut m = message.to_string();
    for s in [url.trim_end_matches('/'), hostport, host] {
        if !s.is_empty() {
            m = m.replace(s, "the model server");
        }
    }
    m
}

/// A client's own failure (a file, a decode, a malformed reply) is not transient.
impl From<String> for ModelError {
    fn from(message: String) -> Self {
        ModelError::fatal(message)
    }
}
impl From<&str> for ModelError {
    fn from(message: &str) -> Self {
        ModelError::fatal(message)
    }
}

impl std::fmt::Display for ModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

/// What a streaming model call hands its caller as it arrives (`Model::generate_stream`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Delta<'a> {
    /// New generated ids (native bindings).
    Ids(&'a [u32]),
    /// New text of a hosted reply, whose ids are made from the whole reply when it ends.
    Text(&'a str),
    /// New text of a hosted reply's reasoning.
    Reasoning(&'a str),
    /// More of a hosted reply's tool call, as the server streams it: the call's place in the reply,
    /// its id and name when they arrive (with the first piece), and more of its arguments' JSON
    /// text. The whole call, parsed, is in the reply when it ends.
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
}

/// A model server as the runtime calls it. Implement `generate` for a server of your own; the built-in clients are `Sglang`, `Completions` and `Chat`.
pub trait Model: Send + Sync {
    /// Validate configuration before workers or external calls start.
    fn validate(&self) -> Result<(), ModelError> {
        Ok(())
    }
    /// Non-secret binding identity, pinned on durable instances. Custom adapters may opt in.
    fn binding_identity(&self) -> Option<J> {
        None
    }
    /// Which model an instance talks to and how its prompts are laid out: the binding's kind,
    /// model and provider options (a LoRA), and its template and context (`Presented`). Not how it
    /// is reached (address, credentials, ranks, timeouts, retry policy, cache TTL) or sampled: a moved
    /// server or a changed timeout restores. The restore pin and recovery images use this.
    fn replay_identity(&self) -> Option<J> {
        self.binding_identity()
            .map(|b| json!({"kind": b["kind"], "model": b["model"], "provider_options": b["provider_options"]}))
    }
    /// Optional app presentation; native clients do not guess a model family.
    fn template(&self) -> Option<ModelTemplate> {
        None
    }
    /// The model's context length in tokens (the binding's `context`): with the reply's room
    /// taken out it is every prompt's window.
    fn context(&self) -> Option<usize> {
        None
    }
    /// What happens after a call of this binding fails (the binding's `retry`).
    fn retry(&self) -> RetryPolicy {
        RetryPolicy::default()
    }
    /// Where `context` came from, for the host's report (`server /v1/models`, `configured`).
    fn window_source(&self) -> Option<String> {
        self.context().map(|_| "configured".into())
    }
    /// Generate a reply to `req`, blocking until it is whole.
    fn generate(&self, req: &GenRequest) -> Result<GenReply, ModelError>;
    /// Generate, handing what arrives to `on_tokens` (new ids, or a hosted reply's new text).
    /// Returning false stops generation (the connection closes, which aborts the request on the
    /// server) and the reply is what arrived. The default streams the whole reply as one chunk.
    fn generate_stream(
        &self,
        req: &GenRequest,
        on_tokens: &mut dyn FnMut(Delta<'_>) -> bool,
    ) -> Result<GenReply, ModelError> {
        let r = self.generate(req)?;
        on_tokens(Delta::Ids(&r.ids));
        Ok(r)
    }
    /// How many data-parallel ranks to pin agents across (1: no pinning).
    fn ranks(&self) -> usize {
        1
    }
}

fn token_ids(value: &J, required: bool) -> Result<Vec<u32>, String> {
    if !required && value.is_null() {
        return Ok(vec![]);
    }
    let values = value
        .as_array()
        .ok_or("ProtocolError: native token IDs must be an array")?;
    values
        .iter()
        .map(|v| {
            v.as_u64()
                .and_then(|n| u32::try_from(n).ok())
                .ok_or_else(|| "ProtocolError: native token IDs must be unsigned 32-bit integers".into())
        })
        .collect()
}

fn stop_id(value: &J) -> Result<Option<u32>, String> {
    if value.is_null() || value.is_string() {
        return Ok(None);
    }
    value
        .as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .map(Some)
        .ok_or_else(|| "ProtocolError: invalid native stop token ID".into())
}

/// A model from a closure (scripted models, tests, adapters to other clients).
pub struct FnModel<F: Fn(&GenRequest) -> Result<GenReply, String> + Send + Sync>(pub F);

impl<F: Fn(&GenRequest) -> Result<GenReply, String> + Send + Sync> Model for FnModel<F> {
    // A scripted model's errors are fatal: tests of retrying feed transient errors as inputs.
    fn generate(&self, req: &GenRequest) -> Result<GenReply, ModelError> {
        (self.0)(req).map_err(ModelError::fatal)
    }
}

/// Where a built-in client's window comes from: the model's context length (the binding's
/// `context`), as a page's viewport comes from the screen it is shown on. There is no window by
/// omission: a binding whose window cannot be established fails `validate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Window {
    /// What the server reports, asked once at `validate` (native clients). A hosted API
    /// reports none, so a hosted binding needs `context`.
    Server,
    /// The host's `context`: at most what a native server reports.
    Configured(usize),
    /// `"context": "none"`: no window, by the host's choice; the server is not asked.
    Unbounded,
}

/// A native client's window once `validate` established it: its size and where it came from.
type Established = std::sync::OnceLock<(usize, String)>;

impl Window {
    /// Establish a native client's window (`model`: the name to find in `/v1/models`, if the
    /// client has one). Kept once established: a client asks its server once.
    fn establish(
        self,
        established: &Established,
        url: &str,
        model: Option<&str>,
        headers: Result<Vec<(&str, String)>, ModelError>,
    ) -> Result<(), ModelError> {
        if self == Window::Unbounded || established.get().is_some() {
            return Ok(());
        }
        let window = match (
            self,
            headers
                .map_err(|e| e.message)
                .and_then(|h| server_window(url, model, &h)),
        ) {
            (Window::Configured(c), Ok((n, path))) if c > n => {
                return Err(ModelError::fatal(format!(
                    "context {c} is more than the model's context length the server reports ({n} tokens, {path}); a configured context may only lower it"
                )));
            }
            (Window::Configured(c), Ok(_)) => (c, "configured".into()),
            (Window::Configured(c), Err(why)) => (c, format!("configured (the server reported none: {why})")),
            (_, Ok((n, path))) => (n, format!("server {path}")),
            (_, Err(why)) => {
                return Err(ModelError::fatal(format!(
                    "the model server reports no context length ({why}): set \"context\" to the model's context length in tokens, or \"context\": \"none\" to run without a window"
                )));
            }
        };
        let _ = established.set(window);
        Ok(())
    }
    fn context(self, established: &Established) -> Option<usize> {
        match self {
            Window::Configured(c) => Some(c),
            Window::Server => established.get().map(|w| w.0),
            Window::Unbounded => None,
        }
    }
    fn source(self, established: &Established) -> Option<String> {
        match self {
            Window::Unbounded => None,
            _ => established
                .get()
                .map(|w| w.1.clone())
                .or_else(|| self.context(established).map(|_| "configured".into())),
        }
    }
}

/// The model's context length as its server reports it, and the endpoint that reported it:
/// `/v1/models` (vLLM, SGLang and other OpenAI-compatible servers: the entry named `model`, or
/// without a name the only entry; its `max_model_len` or `context_length`), else SGLang's
/// `/get_server_info` (`context_length`). A server still starting (unreachable, HTTP 5xx) is
/// asked again, twice. The error names each endpoint and what it answered, never the address.
/// `headers`: what the binding sends with every request (its key where its server reads it, its
/// fixed headers), so the question reaches the server exactly as a call does.
fn server_window(url: &str, model: Option<&str>, headers: &[(&str, String)]) -> Result<(usize, &'static str), String> {
    let mut why = vec![];
    for attempt in 0..3 {
        if attempt > 0 {
            std::thread::sleep(Duration::from_secs(1 << (attempt - 1)))
        }
        why.clear();
        let mut starting = false;
        for path in ["/v1/models", "/get_server_info"] {
            let found = match get_json(
                &format!("{url}{path}"),
                headers,
                Duration::from_secs(10),
                &CancelToken::new(),
            ) {
                Ok((200, body)) => reported(path, &body, model),
                Ok((status, _)) => {
                    starting |= status >= 500;
                    Err(format!("HTTP {status}"))
                }
                Err(e) => {
                    starting = true;
                    Err(e.message)
                }
            };
            match found {
                Ok(n) => return Ok((n, path)),
                Err(e) => why.push(format!("GET {path}: {e}")),
            }
        }
        if !starting {
            break;
        }
    }
    Err(without_endpoint(&why.join("; "), url))
}

/// The context length in a server's answer at `path`.
fn reported(path: &str, body: &J, model: Option<&str>) -> Result<usize, String> {
    let positive = |v: &J| v.as_u64().and_then(|n| usize::try_from(n).ok()).filter(|n| *n > 0);
    if path == "/get_server_info" {
        return positive(&body["context_length"]).ok_or_else(|| "no context_length".into());
    }
    let entries = body["data"].as_array().map_or(&[][..], Vec::as_slice);
    let entry = match model {
        Some(name) => entries
            .iter()
            .find(|e| e["id"] == name)
            .ok_or_else(|| format!("no model {name:?}"))?,
        None if entries.len() == 1 => &entries[0],
        None => return Err(format!("{} models and no model name to choose one", entries.len())),
    };
    positive(&entry["max_model_len"])
        .or_else(|| positive(&entry["context_length"]))
        .ok_or_else(|| "no max_model_len or context_length".into())
}

/// Ensure the matched stop token ends the ids (servers differ on whether they include it).
fn with_stop(mut ids: Vec<u32>, matched: Option<u32>) -> Vec<u32> {
    if let Some(m) = matched
        && ids.last() != Some(&m)
    {
        ids.push(m);
    }
    ids
}

/// SGLang `/generate` with `input_ids`. `sampling` is merged into `sampling_params`.
/// With `stream` (the default), the reply is read as server-sent events: a cancelled or stalled
/// request is closed, which aborts it on the server; `stall` bounds the wait for each event.
/// `sessions`: SGLang's session API (see `Sessions`); off by default.
/// `window`: the model's context length (`Window`), from the server by default.
pub struct Sglang {
    /// The server's address, without a trailing slash.
    pub url: String,
    /// The server's data-parallel size: an agent's requests go to one rank, so its prefix stays in one cache.
    pub ranks: usize,
    /// Sampling parameters sent with every request.
    pub sampling: J,
    /// The longest a request may take.
    pub timeout: Duration,
    /// Stream the reply (default); a cancelled stream stops generation on the server.
    pub stream: bool,
    /// The longest a stream may go without a token before it is aborted and retried.
    pub stall: Duration,
    /// Incremental sessions, if the server offers them: a turn sends only what is new.
    pub sessions: Option<Sessions>,
    /// Fixed provider options merged into every request (a LoRA, for example).
    pub provider_options: J,
    /// Where its key comes from, if the server takes one.
    pub auth: Option<Credentials>,
    /// Where its context length comes from.
    pub window: Window,
    /// What happens after a call fails (the binding's `retry`).
    pub retry: RetryPolicy,
    established: Established,
}

/// SGLang sessions: a turn whose prompt extends its agent's last one sends only the new ids.
///
/// The server's sequence is exactly the prompt. A turn continues the session only when its ids
/// start with the session's last prompt (compared, not assumed), and it sends
/// `session_params {id, rid: <last request>, offset: <last prompt's length>}` with the ids after
/// that offset: the server cuts the session's sequence at the offset (dropping what it holds as
/// the last output, which can run past the matched stop) and appends them. Each response's
/// `prompt_tokens` must equal the prompt's length; a mismatch turns sessions off for the client.
///
/// A session holds a copy of each request's sequence until it closes, so it is closed after
/// `turns` turns, or at the first turn that is not an append (a fold), and the next turn opens a
/// new one with the full prompt. A session the server lost (a restart, a timeout, an unknown
/// rid: HTTP 400, or an aborted request) is dropped and the turn resent with the full prompt.
/// Only turns use sessions; a tagged request (a summary render) always sends its full prompt.
/// The session's requests must reach the same data-parallel rank: set `ranks` to the server's.
pub struct Sessions {
    /// Turns a session lasts before the next one opens with the full prompt.
    pub turns: usize,
    /// How long the server keeps an idle session.
    pub timeout: Duration,
    open: std::sync::Mutex<std::collections::HashMap<String, Session>>,
    off: std::sync::atomic::AtomicBool,
    identity: std::sync::OnceLock<String>,
}

struct Session {
    id: String,
    rid: String,
    prompt: std::sync::Arc<[u32]>,
    turns: usize,
    /// When it was last used: the server drops it `timeout` after that, and so does this table.
    used: std::time::Instant,
}

impl Sessions {
    /// Sessions of at most `turns` turns.
    pub fn new(turns: usize, timeout: Duration) -> Self {
        Sessions {
            turns: turns.max(1),
            timeout,
            open: Default::default(),
            off: Default::default(),
            identity: Default::default(),
        }
    }
    /// Whether sessions were turned off for this client (a server that miscounted a prompt).
    pub fn is_off(&self) -> bool {
        self.off.load(Ordering::Relaxed)
    }
}

/// How one request goes on the wire.
#[derive(Clone, Debug, PartialEq)]
enum Wire {
    Plain,
    /// A new session, with the full prompt.
    Open(String),
    /// The ids after `offset`, continuing the session after request `parent`.
    Continue {
        id: String,
        parent: String,
        offset: usize,
    },
}

impl Sglang {
    /// A client for the SGLang server at `url` with `ranks` data-parallel ranks.
    pub fn new(url: &str, ranks: usize) -> Self {
        // This is a self-hosted server on a private network: a stall almost never means the
        // server gave up, it means it's busy. The stall bound exists only to notice a genuinely
        // dead connection, not to ration how long one agent's turn may take, so it is generous.
        Sglang {
            url: url.trim_end_matches('/').to_string(),
            ranks: ranks.max(1),
            sampling: json!({}),
            timeout: Duration::from_secs(5400),
            stream: true,
            stall: Duration::from_secs(5400),
            sessions: None,
            provider_options: json!({}),
            auth: None,
            window: Window::Server,
            retry: RetryPolicy::default(),
            established: Established::new(),
        }
    }

    fn headers(&self, req: &GenRequest) -> Result<Vec<(&'static str, String)>, ModelError> {
        let mut headers = auth_headers(self.auth.as_ref())?;
        if let Some(rank) = req.rank.filter(|_| self.ranks > 1) {
            headers.push(("X-Data-Parallel-Rank", rank.to_string()));
        }
        Ok(headers)
    }

    /// The wire for a request: continue the agent's session, open a new one (closing the old),
    /// or plain.
    fn wire(&self, req: &GenRequest, headers: &[(&str, String)]) -> Wire {
        let Some(sessions) = self
            .sessions
            .as_ref()
            .filter(|s| !s.is_off() && req.tag.is_none() && req.images.is_empty())
        else {
            return Wire::Plain;
        };
        let old = {
            let mut open = sessions.open.lock().unwrap();
            match open.get(&req.agent) {
                Some(s)
                    if s.turns < sessions.turns && req.ids.len() > s.prompt.len() && req.ids.starts_with(&s.prompt) =>
                {
                    return Wire::Continue {
                        id: s.id.clone(),
                        parent: s.rid.clone(),
                        offset: s.prompt.len(),
                    };
                }
                _ => open.remove(&req.agent),
            }
        };
        if let Some(old) = old {
            self.close_session(&old.id, headers)
        }
        let id = new_rid();
        let opened = post_json(
            &format!("{}/open_session", self.url),
            json!({"session_id": id, "capacity_of_str_len": 1 << 20, "timeout": sessions.timeout.as_secs_f64()})
                .to_string()
                .as_bytes(),
            headers,
            Duration::from_secs(30),
            &req.cancel,
        );
        match opened {
            Ok((200, _)) => Wire::Open(id),
            _ => Wire::Plain,
        }
    }

    fn close_session(&self, id: &str, headers: &[(&str, String)]) {
        let _ = post_json(
            &format!("{}/close_session", self.url),
            json!({"session_id": id}).to_string().as_bytes(),
            headers,
            Duration::from_secs(10),
            &CancelToken::new(),
        );
    }

    /// A request on `wire` was answered: keep the agent's session at it. With `prompt_tokens`
    /// that differ from the prompt, the server's sequence was not ours: sessions go off.
    fn answered(
        &self,
        req: &GenRequest,
        wire: &Wire,
        rid: &str,
        prompt_tokens: Option<u64>,
        headers: &[(&str, String)],
    ) -> Result<(), HttpError> {
        let Some(sessions) = &self.sessions else { return Ok(()) };
        let id = match wire {
            Wire::Plain => return Ok(()),
            Wire::Open(id) | Wire::Continue { id, .. } => id,
        };
        if prompt_tokens != Some(req.ids.len() as u64) {
            sessions.off.store(true, Ordering::Relaxed);
            sessions.open.lock().unwrap().remove(&req.agent);
            self.close_session(id, headers);
            return Err(HttpError::transport(format!(
                "sglang session: the server's prompt has {prompt_tokens:?} tokens, the agent's {}; sessions are now off for this model",
                req.ids.len()
            )));
        }
        let mut open = sessions.open.lock().unwrap();
        let turns = match wire {
            Wire::Continue { .. } => open.get(&req.agent).map_or(0, |s| s.turns),
            _ => 0,
        };
        // An agent that ended leaves its session here; the server has dropped any that idled
        // past the timeout, so those are dropped too and the table holds live agents only.
        let timeout = sessions.timeout;
        open.retain(|_, s| s.used.elapsed() < timeout);
        open.insert(
            req.agent.clone(),
            Session {
                id: id.clone(),
                rid: rid.into(),
                prompt: req.ids.clone(),
                turns: turns + 1,
                used: std::time::Instant::now(),
            },
        );
        Ok(())
    }

    /// The session a request was sent on is gone (or refused it): forget it.
    fn lost(&self, req: &GenRequest, wire: &Wire, headers: &[(&str, String)]) {
        if let (Some(sessions), Wire::Open(id) | Wire::Continue { id, .. }) = (&self.sessions, wire) {
            sessions.open.lock().unwrap().remove(&req.agent);
            self.close_session(id, headers);
        }
    }

    /// The `/generate` body (input_ids written straight to bytes) and its headers.
    fn request(&self, req: &GenRequest, stream: bool, rid: &str, wire: &Wire) -> Vec<u8> {
        let mut sampling = json!({"max_new_tokens": req.max_new, "stop_token_ids": req.stop, "skip_special_tokens": false, "no_stop_trim": true});
        merge(&mut sampling, &self.sampling);
        merge(&mut sampling, &req.sampling);
        let mut rest = json!({"sampling_params": sampling, "stream": stream, "rid": rid});
        merge(&mut rest, &self.provider_options);
        // Images, audio and video go with the ids, one per placeholder the prompt holds, each
        // kind in its own field in prompt order. A source's type says which it is.
        for (kind, field) in [
            ("image", "image_data"),
            ("audio", "audio_data"),
            ("video", "video_data"),
        ] {
            let sources = super::agent::of_kind(&req.images, kind);
            if !sources.is_empty() {
                rest[field] = json!(sources);
            }
        }
        let ids = match wire {
            Wire::Plain => &req.ids[..],
            Wire::Open(id) => {
                rest["session_params"] = json!({"id": id});
                &req.ids[..]
            }
            Wire::Continue { id, parent, offset } => {
                rest["session_params"] = json!({"id": id, "rid": parent, "offset": offset});
                &req.ids[*offset..]
            }
        };
        if let Some(rank) = req.rank.filter(|_| self.ranks > 1) {
            rest["routed_dp_rank"] = json!(rank);
        }
        // SGLang expands a media placeholder into the file's tokens itself, so each run that
        // layout holds for a file goes as one placeholder. (Requests with files use no session.)
        if !req.media.is_empty() && matches!(wire, Wire::Plain) {
            let mut sent = Vec::with_capacity(ids.len());
            let mut from = 0;
            for &(start, count, placeholder) in &req.media {
                if start < from || start + count > ids.len() || count == 0 {
                    continue;
                }
                sent.extend_from_slice(&ids[from..start]);
                sent.push(placeholder);
                from = start + count;
            }
            sent.extend_from_slice(&ids[from..]);
            return ids_body("input_ids", &sent, &rest);
        }
        ids_body("input_ids", ids, &rest)
    }

    /// Tell the server to drop a request we gave up on (best-effort: if it already finished or
    /// the server doesn't recognize the id, there is nothing to clean up). Without this, a
    /// stalled or retried request lingers in the server's running/queue accounting until its
    /// own disconnect-polling notices, which under load can take far longer than our own stall.
    fn abort(&self, rid: &str, headers: &[(&str, String)]) {
        let _ = post_json(
            &format!("{}/abort_request", self.url),
            json!({"rid": rid}).to_string().as_bytes(),
            headers,
            Duration::from_secs(10),
            &CancelToken::new(),
        );
    }

    /// The server finished a request with `abort` (it shut down, ran out of memory, was told
    /// to): what it generated is not a reply. Transient: the agent resends the prompt.
    fn aborted(&self, finish: &J, headers: &[(&str, String)]) -> ModelError {
        let why = finish["message"]
            .as_str()
            .map_or_else(|| finish.to_string(), str::to_string);
        ModelError::of(
            FailureKind::Interrupted,
            redact_credentials(
                format!("sglang /generate: the server aborted the request: {why}"),
                headers,
            ),
        )
        .without_endpoint(&self.url)
    }
}

impl Sglang {
    /// The configuration's own validity (no request): `validate` less the window.
    pub(crate) fn check(&self) -> Result<(), ModelError> {
        validate_native(&self.url, self.ranks, self.timeout, self.stall, self.auth.as_ref())?;
        validate_options(&self.provider_options, &self.sampling, &J::Null)?;
        if let Some(s) = &self.sessions {
            if s.turns == 0 || s.timeout.is_zero() {
                return Err(ModelError::fatal("session turns and timeout must be positive"));
            }
            let id = fingerprint(self.binding_identity().unwrap());
            if s.identity.get_or_init(|| id.clone()) != &id {
                return Err(ModelError::fatal(
                    "SGLang session binding identity changed; use new Sessions for a new endpoint/model/adapter configuration",
                ));
            }
        }
        Ok(())
    }
}

impl Model for Sglang {
    fn validate(&self) -> Result<(), ModelError> {
        self.check()?;
        self.window
            .establish(&self.established, &self.url, None, auth_headers(self.auth.as_ref()))
    }
    fn context(&self) -> Option<usize> {
        self.window.context(&self.established)
    }
    fn retry(&self) -> RetryPolicy {
        self.retry.clone()
    }
    fn window_source(&self) -> Option<String> {
        self.window.source(&self.established)
    }
    fn binding_identity(&self) -> Option<J> {
        Some(
            json!({"kind":"sglang","url":self.url,"ranks":self.ranks,"sampling":self.sampling,"provider_options":self.provider_options,
        "auth":self.auth.as_ref().map(Credentials::identity),"sessions":self.sessions.as_ref().map(|s|json!({"turns":s.turns,"timeout_s":s.timeout.as_secs_f64()}))}),
        )
    }
    fn ranks(&self) -> usize {
        self.ranks
    }
    fn generate(&self, req: &GenRequest) -> Result<GenReply, ModelError> {
        self.check()?;
        validate_options(&self.provider_options, &self.sampling, &req.sampling)?;
        if self.stream {
            return self.generate_stream(req, &mut |_| true);
        }
        let headers = self.headers(req)?;
        let base = new_rid();
        let mut completed_rid = None;
        let mut attempts = 0;
        let (status, resp) = attempt(
            &req.cancel,
            |attempt| self.abort(&format!("{base}-{attempt}"), &headers),
            |attempt| {
                let rid = format!("{base}-{attempt}");
                completed_rid = Some(rid.clone());
                attempts = attempt + 1;
                let mut wire = self.wire(req, &headers);
                loop {
                    let body = self.request(req, false, &rid, &wire);
                    capture(req, &rid, "request", || request_capture(&body, &headers));
                    let started = std::time::Instant::now();
                    let (result, retry_after) = asked(super::http::post_json_asked(
                        &format!("{}/generate", self.url),
                        &body,
                        &headers,
                        self.timeout,
                        &req.cancel,
                    ));
                    capture(req, &rid, "response", || {
                        private_metadata(
                            json!({"result":result,"seconds":started.elapsed().as_secs_f64()}),
                            &headers,
                        )
                        .to_string()
                        .into_bytes()
                    });
                    // A session the server lost or refused: resend the whole prompt.
                    let refused = matches!(&result, Ok((400, _)))
                        || matches!(&result, Ok((200, r)) if r["meta_info"]["finish_reason"]["type"] == "abort");
                    if wire != Wire::Plain && refused {
                        self.lost(req, &wire, &headers);
                        wire = Wire::Plain;
                        continue;
                    }
                    if let Ok((200, r)) = &result {
                        self.answered(req, &wire, &rid, r["meta_info"]["prompt_tokens"].as_u64(), &headers)?;
                        prompt_counted(req, r["meta_info"]["prompt_tokens"].as_u64());
                    }
                    return retryable_status(result, retry_after);
                }
            },
        )
        .map_err(|error| {
            ModelError::transport_of(
                "sglang /generate",
                error.map(|m| redact_credentials(m, &headers)),
                &self.url,
            )
        })?;
        if status != 200 {
            return Err(ModelError::status(
                status,
                redact_credentials(format!("sglang HTTP {status}: {resp}"), &headers),
            )
            .without_endpoint(&self.url));
        }
        let finish = &resp["meta_info"]["finish_reason"];
        if finish["type"] == "abort" {
            return Err(self.aborted(finish, &headers));
        }
        let ids = token_ids(&resp["output_ids"], true).map_err(ModelError::fatal)?;
        let matched = if finish["type"] == "stop" {
            stop_id(&finish["matched"]).map_err(ModelError::fatal)?
        } else {
            None
        };
        Ok(GenReply {
            ids: with_stop(ids, matched),
            cut_off: finish["type"] == "length",
            cached_tokens: resp["meta_info"]["cached_tokens"].as_u64(),
            first_token_ms: None,
            model_meta: resp.get("meta_info").cloned().map(|v| private_metadata(v, &headers)),
            request_id: completed_rid,
            attempts,
        })
    }

    fn generate_stream(
        &self,
        req: &GenRequest,
        on_tokens: &mut dyn FnMut(Delta<'_>) -> bool,
    ) -> Result<GenReply, ModelError> {
        self.check()?;
        validate_options(&self.provider_options, &self.sampling, &req.sampling)?;
        let headers = self.headers(req)?;
        let base = new_rid();
        let started = std::time::Instant::now();
        let mut reply = GenReply::default();
        let mut finish = J::Null;
        let mut stopped = false;
        attempt(&req.cancel, |attempt| self.abort(&format!("{base}-{attempt}"), &headers), |attempt| {
            // A retry starts the reply over, under a fresh rid (the failed attempt is aborted
            // above, by `attempt`'s rid, before the next one starts).
            reply = GenReply::default();
            reply.request_id = Some(format!("{base}-{attempt}"));
            reply.attempts = attempt + 1;
            finish = J::Null;
            let rid = format!("{base}-{attempt}");
            let attempt_started = std::time::Instant::now();
            let mut wire = self.wire(req, &headers);
            let mut prompt_tokens = None;
            let mut checked = false;
            let mut event_index = 0usize;
            let sent = loop {
                let body = self.request(req, true, &rid, &wire);
                capture(req,&rid,"request",|| request_capture(&body, &headers));
                let result = post_sse(&format!("{}/generate", self.url), &body, &headers, self.stall, &req.cancel, |ev| {
                capture(req,&rid,&format!("event-{event_index}"),|| private_metadata(ev.clone(),&headers).to_string().into_bytes());
                event_index += 1;
                stream_error(&ev)?;
                // A session's sequence is checked before any token is passed on.
                if !checked {
                    if let Some(n) = ev["meta_info"]["prompt_tokens"].as_u64() { if !checked { prompt_counted(req, Some(n)); } prompt_tokens = Some(n); checked = true; self.answered(req, &wire, &rid, prompt_tokens, &headers)?; }
                    else if ev["meta_info"]["finish_reason"]["type"] == "abort" { finish = ev["meta_info"]["finish_reason"].clone(); return Ok(false) }
                }
                let ids = token_ids(&ev["output_ids"], false).map_err(HttpError::protocol)?;
                // Chunks carry either all ids so far or only the new ones; the count says which.
                let total = ev["meta_info"]["completion_tokens"].as_u64().map(|n| n as usize);
                let new: &[u32] = if total == Some(ids.len()) && ids.len() >= reply.ids.len() { &ids[reply.ids.len()..] } else { &ids };
                if !new.is_empty() {
                    reply.first_token_ms.get_or_insert(started.elapsed().as_secs_f64() * 1000.0);
                    reply.ids.extend_from_slice(new);
                }
                if let Some(c) = ev["meta_info"]["cached_tokens"].as_u64() { reply.cached_tokens = Some(c); }
                if let Some(meta) = ev.get("meta_info") { reply.model_meta = Some(private_metadata(meta.clone(),&headers)); }
                if !ev["meta_info"]["finish_reason"].is_null() { finish = ev["meta_info"]["finish_reason"].clone(); }
                stopped = !new.is_empty() && !on_tokens(Delta::Ids(new));
                Ok(!stopped)
                });
                // A session the server lost or refused (HTTP 400, an abort before any token): resend the whole prompt.
                let refused = matches!(&result, Err(e) if e.status() == Some(400)) || (finish["type"] == "abort" && reply.ids.is_empty());
                if wire != Wire::Plain && refused {
                    self.lost(req, &wire, &headers);
                    wire = Wire::Plain;
                    finish = J::Null;
                    continue;
                }
                break result;
            };
            let result = sent.and_then(|_| {
                if !checked && wire != Wire::Plain { self.answered(req, &wire, &rid, prompt_tokens, &headers)?; }
                // A stream that ends without a finish reason was cut by the server or the network.
                if finish.is_null() && !stopped { return Err(HttpError::transport("stream ended without a finish reason")) }
                Ok(())
            });
            capture(req,&rid,"response",|| private_metadata(json!({"result":result,"finish_reason":finish,"seconds":attempt_started.elapsed().as_secs_f64()}),&headers).to_string().into_bytes());
            result
        }).map_err(|error|ModelError::transport_of("sglang /generate", error.map(|m| redact_credentials(m, &headers)), &self.url))?;
        if finish["type"] == "abort" {
            return Err(self.aborted(&finish, &headers));
        }
        let matched = if finish["type"] == "stop" {
            stop_id(&finish["matched"]).map_err(ModelError::fatal)?
        } else {
            None
        };
        reply.cut_off = finish["type"] == "length";
        reply.ids = with_stop(std::mem::take(&mut reply.ids), matched);
        Ok(reply)
    }
}

/// OpenAI-style `/v1/completions` with a token-id prompt (vLLM). Needs a server that returns
/// generated token ids (`return_token_ids`); text is never re-tokenized.
/// `ranks`: the server's data-parallel size. An agent's requests carry its pinned rank in the
/// `X-data-parallel-rank` header (vLLM routes `/v1/completions` by it), so its prefix stays in one
/// engine's cache; with one rank, no header.
/// `window`: as `Sglang`'s; the server's `/v1/models` entry named `model`.
pub struct Completions {
    /// The server's address, without a trailing slash.
    pub url: String,
    /// The model's name on the server.
    pub model: String,
    /// The server's data-parallel size.
    pub ranks: usize,
    /// Sampling parameters sent with every request.
    pub sampling: J,
    /// The longest a request may take.
    pub timeout: Duration,
    /// Stream the reply (default).
    pub stream: bool,
    /// The longest a stream may go without a token.
    pub stall: Duration,
    /// Fixed provider options merged into every request.
    pub provider_options: J,
    /// Where its key comes from, if the server takes one.
    pub auth: Option<Credentials>,
    /// Where its context length comes from.
    pub window: Window,
    /// What happens after a call fails (the binding's `retry`).
    pub retry: RetryPolicy,
    established: Established,
}

impl Completions {
    /// A client for the model `model` on the server at `url`.
    pub fn new(url: &str, model: &str) -> Self {
        Completions {
            url: url.trim_end_matches('/').to_string(),
            model: model.into(),
            ranks: 1,
            sampling: json!({}),
            timeout: Duration::from_secs(5400),
            stream: true,
            stall: Duration::from_secs(5400),
            provider_options: json!({}),
            auth: None,
            window: Window::Server,
            retry: RetryPolicy::default(),
            established: Established::new(),
        }
    }

    fn headers(&self, req: &GenRequest) -> Result<Vec<(&'static str, String)>, ModelError> {
        let mut headers = auth_headers(self.auth.as_ref())?;
        if let Some(rank) = req.rank.filter(|_| self.ranks > 1) {
            headers.push(("X-Data-Parallel-Rank", rank.to_string()));
        }
        Ok(headers)
    }

    /// The `/v1/completions` body (the prompt ids written straight to bytes).
    fn request(&self, req: &GenRequest, stream: bool) -> Vec<u8> {
        let mut rest = json!({"model": self.model, "max_tokens": req.max_new, "stop_token_ids": req.stop,
            "skip_special_tokens": false, "include_stop_str_in_output": true, "return_token_ids": true, "stream": stream});
        if stream {
            rest["stream_options"] = json!({"include_usage": true});
        }
        merge(&mut rest, &self.provider_options);
        merge(&mut rest, &self.sampling);
        merge(&mut rest, &req.sampling);
        ids_body("prompt", &req.ids, &rest)
    }

    /// The configuration's own validity (no request): `validate` less the window.
    pub(crate) fn check(&self) -> Result<(), ModelError> {
        validate_native(&self.url, self.ranks, self.timeout, self.stall, self.auth.as_ref())?;
        if self.model.is_empty() {
            return Err(ModelError::fatal("completions model name must be nonempty"));
        }
        validate_options(&self.provider_options, &self.sampling, &J::Null)
    }
}

impl Model for Completions {
    fn validate(&self) -> Result<(), ModelError> {
        self.check()?;
        self.window.establish(
            &self.established,
            &self.url,
            Some(&self.model),
            auth_headers(self.auth.as_ref()),
        )
    }
    fn context(&self) -> Option<usize> {
        self.window.context(&self.established)
    }
    fn retry(&self) -> RetryPolicy {
        self.retry.clone()
    }
    fn window_source(&self) -> Option<String> {
        self.window.source(&self.established)
    }
    fn binding_identity(&self) -> Option<J> {
        Some(
            json!({"kind":"completions","url":self.url,"model":self.model,"ranks":self.ranks,
        "sampling":self.sampling,"provider_options":self.provider_options,"auth":self.auth.as_ref().map(Credentials::identity)}),
        )
    }
    fn ranks(&self) -> usize {
        self.ranks
    }

    fn generate_stream(
        &self,
        req: &GenRequest,
        on_tokens: &mut dyn FnMut(Delta<'_>) -> bool,
    ) -> Result<GenReply, ModelError> {
        self.check()?;
        validate_options(&self.provider_options, &self.sampling, &req.sampling)?;
        if !req.images.is_empty() {
            return Err(ModelError::fatal(
                "this binding sends token ids to a completions endpoint, which takes no images, audio or video (an <img>, <audio> or <video> in the prompt); bind the model with kind \"chat\" and the server will apply its own template to them",
            ));
        }
        let headers = self.headers(req)?;
        let body = self.request(req, true);
        let base = new_rid();
        let started = std::time::Instant::now();
        let mut reply = GenReply::default();
        let (mut finish, mut stop_reason) = (J::Null, J::Null);
        let mut stopped = false;
        // No abort endpoint in the generic OpenAI completions API to tell the server to drop a
        // failed attempt; a retry here still relies on the TCP close to signal cancellation.
        attempt(
            &req.cancel,
            |_| {},
            |attempt| {
                reply = GenReply::default();
                reply.attempts = attempt + 1;
                finish = J::Null;
                let rid = format!("{base}-{attempt}");
                let mut event_index = 0usize;
                capture(req, &rid, "request", || request_capture(&body, &headers));
                let attempt_started = std::time::Instant::now();
                let result = post_sse(
                    &format!("{}/v1/completions", self.url),
                    &body,
                    &headers,
                    self.stall,
                    &req.cancel,
                    |ev| {
                        capture(req, &rid, &format!("event-{event_index}"), || {
                            private_metadata(ev.clone(), &headers).to_string().into_bytes()
                        });
                        stream_error(&ev)?;
                        event_index += 1;
                        if let Some(usage) = ev.get("usage").filter(|v| !v.is_null()) {
                            prompt_counted(req, usage["prompt_tokens"].as_u64());
                            reply.model_meta = Some(private_metadata(usage.clone(), &headers));
                        }
                        if let Some(c) = ev["usage"]["prompt_tokens_details"]["cached_tokens"].as_u64() {
                            reply.cached_tokens = Some(c);
                        }
                        let choice = &ev["choices"][0];
                        if choice.is_null() {
                            return Ok(true);
                        }
                        let new = token_ids(&choice["token_ids"], false).map_err(HttpError::protocol)?;
                        if !choice["finish_reason"].is_null() {
                            finish = choice["finish_reason"].clone();
                            stop_reason = choice["stop_reason"].clone();
                        }
                        if new.is_empty() {
                            return Ok(true);
                        }
                        reply
                            .first_token_ms
                            .get_or_insert(started.elapsed().as_secs_f64() * 1000.0);
                        reply.ids.extend_from_slice(&new);
                        stopped = !on_tokens(Delta::Ids(&new));
                        Ok(!stopped)
                    },
                )
                .and_then(|_| {
                    if finish.is_null() && !stopped {
                        Err(HttpError::transport("stream ended without a finish reason"))
                    } else {
                        Ok(())
                    }
                });
                capture(req, &rid, "response", || {
                    private_metadata(
                        json!({"result":result,"seconds":attempt_started.elapsed().as_secs_f64()}),
                        &headers,
                    )
                    .to_string()
                    .into_bytes()
                });
                result
            },
        )
        .map_err(|error| {
            ModelError::transport_of(
                "completions /v1/completions",
                error.map(|m| redact_credentials(m, &headers)),
                &self.url,
            )
        })?;
        let matched = stop_id(&stop_reason)
            .map_err(ModelError::fatal)?
            .filter(|_| finish == "stop");
        reply.cut_off = finish == "length";
        reply.ids = with_stop(std::mem::take(&mut reply.ids), matched);
        Ok(reply)
    }

    fn generate(&self, req: &GenRequest) -> Result<GenReply, ModelError> {
        self.check()?;
        validate_options(&self.provider_options, &self.sampling, &req.sampling)?;
        if !req.images.is_empty() {
            return Err(ModelError::fatal(
                "this binding sends token ids to a completions endpoint, which takes no images, audio or video (an <img>, <audio> or <video> in the prompt); bind the model with kind \"chat\" and the server will apply its own template to them",
            ));
        }
        if self.stream {
            return self.generate_stream(req, &mut |_| true);
        }
        let headers = self.headers(req)?;
        let body = self.request(req, false);
        let base = new_rid();
        let mut attempts = 0;
        let (status, resp) = attempt(
            &req.cancel,
            |_| {},
            |attempt| {
                attempts = attempt + 1;
                let rid = format!("{base}-{attempt}");
                capture(req, &rid, "request", || request_capture(&body, &headers));
                let started = std::time::Instant::now();
                let (result, retry_after) = asked(super::http::post_json_asked(
                    &format!("{}/v1/completions", self.url),
                    &body,
                    &headers,
                    self.timeout,
                    &req.cancel,
                ));
                capture(req, &rid, "response", || {
                    private_metadata(
                        json!({"result":result,"seconds":started.elapsed().as_secs_f64()}),
                        &headers,
                    )
                    .to_string()
                    .into_bytes()
                });
                retryable_status(result, retry_after)
            },
        )
        .map_err(|error| {
            ModelError::transport_of(
                "completions /v1/completions",
                error.map(|m| redact_credentials(m, &headers)),
                &self.url,
            )
        })?;
        if status != 200 {
            return Err(ModelError::status(
                status,
                redact_credentials(format!("completions HTTP {status}: {resp}"), &headers),
            )
            .without_endpoint(&self.url));
        }
        prompt_counted(req, resp["usage"]["prompt_tokens"].as_u64());
        let choice = &resp["choices"][0];
        let ids = token_ids(&choice["token_ids"], true).map_err(ModelError::fatal)?;
        let matched = stop_id(&choice["stop_reason"])
            .map_err(ModelError::fatal)?
            .filter(|_| choice["finish_reason"] == "stop");
        Ok(GenReply {
            ids: with_stop(ids, matched),
            cut_off: choice["finish_reason"] == "length",
            cached_tokens: resp["usage"]["prompt_tokens_details"]["cached_tokens"].as_u64(),
            first_token_ms: None,
            model_meta: resp.get("usage").cloned().map(|v| private_metadata(v, &headers)),
            request_id: None,
            attempts,
        })
    }
}

/// One attempt at `f`. A failure after the request was sent runs `on_fail`: a server may still
/// hold the request, and the client tells it to drop it. The failure goes back as it is: whether
/// the call is made again is the retry policy's, not the client's.
fn attempt<T>(
    cancel: &CancelToken,
    mut on_fail: impl FnMut(usize),
    mut f: impl FnMut(usize) -> Result<T, HttpError>,
) -> Result<T, HttpError> {
    cancel.check().map_err(HttpError::transport)?;
    f(0).inspect_err(|e| {
        if matches!(e.failure, Failure::Transport | Failure::Protocol) {
            on_fail(0)
        }
    })
}

/// A hosted chat API. The app renders through `MessagesApp`, whose first line is the turns and
/// tools as JSON; the reply comes back as `{reasoning, content, tool_calls}` JSON in ids of the
/// binding's tokenizer (`syntax="chat"`). The key, if the server takes one, is read at call time
/// from its file (a `NAME=value` line) and sent only as a request header: never in logs or traces.
/// `api` selects the provider protocol: OpenAI-compatible chat completions, Anthropic Messages or OpenAI
/// Responses. Native thinking/continuation state stays in the app log, never in this client.
pub struct Chat {
    /// The API's base address (`https://api.example.com/v1`).
    pub url: String,
    /// The model's name.
    pub model: String,
    /// The key's source; none for a server that takes no key (a local llama.cpp, Ollama or vLLM).
    pub auth: Option<Credentials>,
    /// The header the key is sent in (`KEY_HEADERS`), for a provider that does not take the API's
    /// own (`api-key` for Azure OpenAI). None: `Authorization: Bearer` (chat, responses) or
    /// `x-api-key` (anthropic).
    pub key_header: Option<String>,
    /// Sampling parameters sent with every request.
    pub sampling: J,
    /// Fixed provider options merged into every request.
    pub provider_options: J,
    /// The longest a request may take.
    pub timeout: Duration,
    /// The longest a streamed reply may go without data (`stall_s`); none: only `timeout` bounds it.
    pub stall: Option<Duration>,
    /// Fixed non-secret request headers. Credentials and Content-Type are client-owned.
    pub headers: std::collections::BTreeMap<String, String>,
    /// The binding's tokenizer: prompts and replies cross as its ids.
    pub enc: std::sync::Arc<crate::emit::Encoder>,
    /// The provider protocol.
    pub api: Api,
    /// The model's context length: `Configured`, `Unbounded`, or `Server`: what an
    /// OpenAI-compatible server reports at `{url}/models` (vLLM, SGLang, llama.cpp). A hosted API
    /// reports none, so `Server` fails `validate` there.
    pub window: Window,
    /// What happens after a call fails (the binding's `retry`).
    pub retry: RetryPolicy,
    established: Established,
}

/// A hosted binding's provider protocol.
#[derive(Debug, Clone, PartialEq)]
pub enum Api {
    /// `POST {url}/chat/completions` with a bearer key.
    Chat,
    /// `POST {url}/v1/messages` with `x-api-key`. Cache breakpoints (`cache_ttl`: `5m` or `1h`)
    /// follow the tools, the system prompt and the history before the current turn, so each turn
    /// reads the previous turn's prefix.
    Anthropic { cache_ttl: String },
    /// Stateless Responses with native reasoning and function-call history.
    Responses { strict_tools: bool },
}

const ANTHROPIC_VERSION: &str = "2023-06-01";

impl Chat {
    /// An OpenAI-compatible chat completions client.
    pub fn new(url: &str, model: &str, auth: Option<Credentials>, enc: std::sync::Arc<crate::emit::Encoder>) -> Self {
        Chat {
            url: url.trim_end_matches('/').into(),
            model: model.into(),
            auth,
            key_header: None,
            sampling: json!({}),
            provider_options: json!({}),
            timeout: Duration::from_secs(600),
            stall: None,
            headers: Default::default(),
            enc,
            api: Api::Chat,
            window: Window::Server,
            retry: RetryPolicy::default(),
            established: Established::new(),
        }
    }

    /// An Anthropic Messages client (`url`: e.g. `https://api.anthropic.com`).
    pub fn anthropic(
        url: &str,
        model: &str,
        auth: Option<Credentials>,
        enc: std::sync::Arc<crate::emit::Encoder>,
    ) -> Self {
        Chat {
            api: Api::Anthropic { cache_ttl: "5m".into() },
            ..Chat::new(url, model, auth, enc)
        }
    }

    /// A native OpenAI Responses client (`url`: e.g. `https://api.openai.com/v1`).
    pub fn responses(
        url: &str,
        model: &str,
        auth: Option<Credentials>,
        enc: std::sync::Arc<crate::emit::Encoder>,
    ) -> Self {
        Chat {
            api: Api::Responses { strict_tools: false },
            ..Chat::new(url, model, auth, enc)
        }
    }

    fn key(&self) -> Result<Option<String>, String> {
        self.auth.as_ref().map(|a| a.read().map_err(|e| e.message)).transpose()
    }

    /// The configuration's own validity: `validate` less the window.
    pub(crate) fn check(&self) -> Result<(), ModelError> {
        validate_url(&self.url, true)?;
        self.validate_hosted_options(&J::Null)?;
        if self.model.is_empty() || self.timeout.is_zero() {
            return Err(ModelError::fatal(
                "chat model and request timeout must be nonempty/positive",
            ));
        }
        if let Api::Anthropic { cache_ttl } = &self.api
            && !matches!(cache_ttl.as_str(), "5m" | "1h")
        {
            return Err(ModelError::fatal("anthropic cache_ttl must be \"5m\" or \"1h\""));
        }
        super::model_config::validate_chat_headers(&self.headers, matches!(self.api, Api::Anthropic { .. }))?;
        if let Some(h) = &self.key_header
            && !KEY_HEADERS.iter().any(|k| h.eq_ignore_ascii_case(k))
        {
            return Err(ModelError::fatal(format!(
                "key_header {h:?}: use one of {}",
                KEY_HEADERS.join(", ")
            )));
        }
        self.key()?;
        Ok(())
    }
}

impl Model for Chat {
    fn template(&self) -> Option<ModelTemplate> {
        Some(ModelTemplate {
            name: "messages".into(),
            options: json!({}),
        })
    }
    fn validate(&self) -> Result<(), ModelError> {
        self.check()?;
        match (self.window, &self.api) {
            // An OpenAI-compatible server (vLLM, SGLang, llama.cpp) reports its window; a hosted
            // API answers the same request without one, and the error says to configure it.
            (Window::Server, Api::Chat) => {
                let base = self.url.strip_suffix("/v1").unwrap_or(&self.url);
                Window::Server
                    .establish(&self.established, base, Some(&self.model), self.request_headers(false))
                    .map_err(|e| ModelError::fatal(format!("chat: {}", e.message)).without_endpoint(&self.url))
            }
            (Window::Server, _) => Err(ModelError::fatal(format!(
                "{}: a hosted API does not report the model's context length: set \"context\" to it in tokens, or \"context\": \"none\" to run without a window",
                self.kind()
            ))),
            _ => Ok(()),
        }
    }
    fn context(&self) -> Option<usize> {
        self.window.context(&self.established)
    }
    fn retry(&self) -> RetryPolicy {
        self.retry.clone()
    }
    fn window_source(&self) -> Option<String> {
        self.window.source(&self.established)
    }
    fn binding_identity(&self) -> Option<J> {
        let mut identity = json!({"kind":"chat","url":self.url,"model":self.model,"sampling":self.sampling,"provider_options":self.provider_options,
            "auth":self.auth.as_ref().map(Credentials::identity),"headers":self.headers});
        if let Some(h) = &self.key_header {
            identity["key_header"] = json!(h.to_ascii_lowercase());
        }
        if let Api::Anthropic { cache_ttl } = &self.api {
            identity["kind"] = json!("anthropic");
            identity["cache_ttl"] = json!(cache_ttl);
        }
        if let Api::Responses { strict_tools } = self.api {
            identity["kind"] = json!("responses");
            identity["strict_tools"] = json!(strict_tools);
        }
        if self.api != Api::Chat {
            identity["request_timeout_s"] = json!(self.timeout.as_secs_f64());
        }
        Some(identity)
    }
    fn generate(&self, req: &GenRequest) -> Result<GenReply, ModelError> {
        // A hosted error (a provider may echo its own address) without the address.
        self.call(req, None).map_err(|e| e.without_endpoint(&self.url))
    }
    fn generate_stream(&self, req: &GenRequest, on: &mut dyn FnMut(Delta<'_>) -> bool) -> Result<GenReply, ModelError> {
        self.call(req, Some(on)).map_err(|e| e.without_endpoint(&self.url))
    }
}

impl Chat {
    fn validate_hosted_options(&self, overrides: &J) -> Result<(), ModelError> {
        let responses = matches!(self.api, Api::Responses { .. });
        super::model_config::validate_options_allowing(
            &self.provider_options,
            &self.sampling,
            overrides,
            if responses { &["text"] } else { &[] },
        )?;
        for options in [&self.provider_options, &self.sampling, overrides] {
            if let Some(o) = options.as_object() {
                if responses
                    && let Some(key) = o.keys().find(|k| {
                        matches!(
                            k.to_ascii_lowercase().as_str(),
                            "store" | "include" | "previous_response_id" | "conversation" | "background"
                        )
                    })
                {
                    return Err(ModelError::fatal(format!("{}: {key} is transport-owned", self.kind())));
                }
                if responses {
                    for key in ["reasoning", "text", "prompt_cache_options"] {
                        if o.get(key).is_some_and(|v| !v.is_object()) {
                            return Err(ModelError::fatal(format!("responses: {key} must be an object")));
                        }
                    }
                    if o.get("prompt_cache_key")
                        .is_some_and(|v| !v.as_str().is_some_and(|s| !s.is_empty() && s.len() <= 64))
                    {
                        return Err(ModelError::fatal(
                            "responses: prompt_cache_key must be a nonempty string of at most 64 bytes",
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    /// One call; `on`: stream it, handing its text to `on` as it arrives (false stops it).
    /// The headers every request of this binding carries, in order: its key in the header its API
    /// (or the binding's `key_header`) reads, the API's own, `Content-Type` for a request with a
    /// body, and the binding's fixed `headers`. The one place they are built: a call and the
    /// question about the model's context length send the same.
    fn request_headers(&self, body: bool) -> Result<Vec<(&str, String)>, ModelError> {
        let (default_header, mut headers): (&str, Vec<(&str, String)>) = match self.api {
            Api::Chat | Api::Responses { .. } => ("Authorization", vec![]),
            Api::Anthropic { .. } => ("x-api-key", vec![("anthropic-version", ANTHROPIC_VERSION.into())]),
        };
        let name = self.key_header.as_deref().unwrap_or(default_header);
        if let Some(key) = self.key()? {
            let value = if name.eq_ignore_ascii_case("authorization") {
                format!("Bearer {key}")
            } else {
                key
            };
            headers.insert(0, (name, value));
        }
        if body {
            headers.push(("Content-Type", "application/json".into()));
        }
        headers.extend(self.headers.iter().map(|(name, value)| (name.as_str(), value.clone())));
        Ok(headers)
    }

    fn call(&self, req: &GenRequest, on: Option<&mut dyn FnMut(Delta<'_>) -> bool>) -> Result<GenReply, ModelError> {
        self.check()?;
        self.validate_hosted_options(&req.sampling)?;
        req.cancel.check().map_err(ModelError::cancelled)?;
        let text = self.enc.tok.decode(&req.ids, false).map_err(|e| e.to_string())?;
        let mut doc: J = serde_json::from_str(text.lines().next().unwrap_or(""))
            .map_err(|e| format!("chat: the prompt is not MessagesApp JSON ({e})"))?;
        if !doc["messages"].is_array() {
            return Err(ModelError::fatal("chat: MessagesApp must contain a messages array"));
        }
        hosted::images(&mut doc).map_err(ModelError::fatal)?;
        let callable = match doc.get("callable") {
            None => true,
            Some(J::Bool(b)) => *b,
            _ => return Err(ModelError::fatal("MessagesApp callable must be a boolean")),
        };
        let mut body = match &self.api {
            Api::Chat => {
                let mut body = json!({"messages": hosted::messages(&doc)});
                if doc["tools"].as_array().is_some_and(|t| !t.is_empty()) {
                    body["tools"] = doc["tools"].clone();
                }
                body
            }
            Api::Anthropic { cache_ttl } => hosted::anthropic_messages(&doc, cache_ttl, &self.model)?,
            Api::Responses { strict_tools } => hosted::responses_messages(&doc, &self.model, *strict_tools)?,
        };
        body["model"] = json!(self.model);
        let limit = if matches!(self.api, Api::Responses { .. }) {
            "max_output_tokens"
        } else {
            "max_tokens"
        };
        body[limit] = json!(req.max_new);
        merge(&mut body, &self.provider_options);
        merge(&mut body, &self.sampling);
        merge(&mut body, &req.sampling);
        if !callable {
            // Keep definitions for descriptions/caching, while disabling every invocation.
            let fields = body.as_object_mut().unwrap();
            fields.remove("functions");
            fields.remove("function_call");
            if fields.get("tools").and_then(J::as_array).is_some_and(|t| !t.is_empty()) {
                fields.insert(
                    "tool_choice".into(),
                    if matches!(self.api, Api::Anthropic { .. }) {
                        json!({"type":"none"})
                    } else {
                        json!("none")
                    },
                );
            } else {
                fields.remove("tool_choice");
            }
        }
        if body.get("n").is_some_and(|n| n != &json!(1))
            || body["stream"] == true
            || body.get("max_completion_tokens").is_some()
        {
            return Err(ModelError::fatal("chat: use one non-streaming choice and max_tokens"));
        }
        body[limit]
            .as_u64()
            .filter(|n| *n > 0)
            .ok_or("hosted output limit must be positive")?;
        if matches!(self.api, Api::Responses { .. }) {
            body["store"] = json!(false);
            body["include"] = json!(["reasoning.encrypted_content"]);
            if body.get("prompt_cache_key").is_none() {
                body["prompt_cache_key"] = json!(fingerprint(
                    json!({"binding":self.binding_identity(), "agent":req.agent})
                ));
            }
        }
        if matches!(self.api, Api::Anthropic { .. })
            && body["thinking"]["type"] == "enabled"
            && body["thinking"]["budget_tokens"]
                .as_u64()
                .is_some_and(|n| n >= req.max_new as u64)
        {
            return Err(ModelError::fatal(
                "anthropic: thinking.budget_tokens must be less than max_new",
            ));
        }
        if matches!(self.api, Api::Anthropic { .. }) && body["thinking"]["type"] == "enabled" {
            hosted::validate_anthropic_history(&doc, &self.model)?;
        }
        if on.is_some() {
            body["stream"] = json!(true);
            if self.api == Api::Chat {
                body["stream_options"] = json!({"include_usage": true});
            }
        }
        let bytes = body.to_string();
        let endpoint = match self.api {
            Api::Chat => "chat/completions",
            Api::Responses { .. } => "responses",
            Api::Anthropic { .. } => "v1/messages",
        };
        let headers = self.request_headers(true)?;
        // One attempt. Whether a failed call is made again, and when, is the retry policy's: the
        // failure says what kind it was and how long the server asked the caller to wait.
        let rid = format!("{}-0", new_rid());
        capture(req, &rid, "request", || {
            redact_credentials(
                String::from_utf8(request_capture(bytes.as_bytes(), &headers)).unwrap(),
                &headers,
            )
            .into_bytes()
        });
        let started = std::time::Instant::now();
        let mut assembler = on.is_some().then(|| hosted_stream::Assembler::new(&self.api));
        let stream = match (assembler.as_mut(), on) {
            (Some(a), Some(on)) => Some((a, on)),
            _ => None,
        };
        let result = self
            .post(req, endpoint, bytes.as_bytes(), &headers, stream)
            .and_then(|(status, resp, delay, stopped)| match assembler {
                Some(a) if (200..300).contains(&status) => a
                    .finish(stopped)
                    .map(|resp| (status, resp, delay))
                    .map_err(|e| ModelError::fatal(format!("{}: {e}", self.kind()))),
                _ => Ok((status, resp, delay)),
            })
            .map_err(|e| e.without_endpoint(&self.url));
        let shown = result
            .as_ref()
            .map(|(status, resp, _)| (status, resp))
            .map_err(|e| &e.message);
        capture(req, &rid, "response", || {
            private_metadata(
                json!({"result": shown, "seconds": started.elapsed().as_secs_f64()}),
                &headers,
            )
            .to_string()
            .into_bytes()
        });
        let (status, resp, retry_after) = result?;
        let attempts = 1;
        if !(200..300).contains(&status) {
            let mut error = ModelError::status(
                status,
                redact_credentials(format!("{} HTTP {status}: {resp}", self.kind()), &headers)
                    .chars()
                    .take(300)
                    .collect::<String>(),
            );
            error.retry_after = retry_after;
            return Err(error);
        }
        if resp.get("error").is_some_and(|e| !e.is_null()) {
            return Err(ModelError::fatal(redact_credentials(
                format!("{}: {}", self.kind(), brief(&resp["error"].to_string())),
                &headers,
            )));
        }
        if !callable && hosted::has_tool_call(&resp, &self.api) {
            return Err(ModelError::fatal(format!(
                "{}: provider returned a tool call for a non-callable render",
                self.kind()
            )));
        }
        let (reply, cut_off, usage) = match self.api {
            Api::Chat => hosted::chat_reply(&resp),
            Api::Anthropic { .. } => hosted::anthropic_reply(&resp, &self.model),
            Api::Responses { .. } => hosted::responses_reply(&resp, &self.model),
        }?;
        let wire = reply.to_string();
        let ids = self
            .enc
            .tok
            .encode(wire.as_str(), false)
            .map_err(|e| e.to_string())?
            .get_ids()
            .to_vec();
        if self.enc.tok.decode(&ids, false).map_err(|e| e.to_string())? != wire {
            return Err(ModelError::fatal(
                "hosted reply tokenizer is lossy; use a tokenizer with an exact text roundtrip to preserve provider state",
            ));
        }
        Ok(GenReply {
            ids,
            cut_off,
            cached_tokens: usage["prompt_tokens_details"]["cached_tokens"].as_u64(),
            first_token_ms: None,
            model_meta: Some(private_metadata(
                json!({"usage": usage, "model": resp["model"]}),
                &headers,
            )),
            request_id: resp["id"]
                .as_str()
                .map(|id| redact_credentials(id.to_string(), &headers)),
            attempts,
        })
    }

    fn kind(&self) -> &'static str {
        match self.api {
            Api::Chat => "chat",
            Api::Anthropic { .. } => "anthropic",
            Api::Responses { .. } => "responses",
        }
    }

    /// POST the body; the HTTP status, the reply, the delay a rate-limited answer asks for, and
    /// whether the caller stopped the stream. A stream's events go to its assembler as they
    /// arrive; its reply is the assembler's, so the reply here is null. `request_timeout_s`
    /// bounds the whole exchange, streamed or not.
    #[allow(clippy::type_complexity)]
    fn post<'f>(
        &self,
        req: &GenRequest,
        endpoint: &str,
        bytes: &[u8],
        headers: &[(&str, String)],
        stream: Option<(&mut hosted_stream::Assembler, &mut (dyn FnMut(Delta<'_>) -> bool + 'f))>,
    ) -> Result<(u16, J, Option<Duration>, bool), ModelError> {
        use super::http::{ERROR_BODY_MAX, Request, Timing, exchange};
        // Classified as the native bindings' are (a refused connection is `unreachable`), and
        // named by the binding, the key never in the message.
        let failed = |e: HttpError| {
            let message = redact_credentials(format!("{}: {e}", self.kind()), headers);
            let mut error = ModelError::transport(e);
            error.message = message;
            error
        };
        // `headers` is the whole list the caller built (and the capture records), Content-Type included.
        let mut fields: Vec<(&str, String)> = headers.to_vec();
        if stream.is_some() && !fields.iter().any(|(name, _)| name.eq_ignore_ascii_case("accept")) {
            fields.push(("Accept", "text/event-stream".into()));
        }
        let url = format!("{}/{endpoint}", self.url);
        let request = Request {
            method: "POST",
            url: &url,
            headers: &fields,
            body: bytes.to_vec(),
        };
        let timing = Timing::deadline(self.timeout).between(self.stall.filter(|_| stream.is_some()));
        let response = exchange(request, timing, &req.cancel).map_err(|e| {
            if req.cancel.is_cancelled() {
                ModelError::cancelled(format!("cancelled: {}", req.cancel.reason()))
            } else {
                failed(e)
            }
        })?;
        let status = response.status;
        let header = |name: &str| response.header(name).unwrap_or_default().trim().to_string();
        let resets = [
            "anthropic-ratelimit-tokens-reset",
            "anthropic-ratelimit-requests-reset",
            "x-ratelimit-reset-project-tokens",
            "x-ratelimit-reset-tokens",
            "x-ratelimit-reset-requests",
        ]
        .map(header);
        let resets: Vec<&str> = resets.iter().map(String::as_str).collect();
        let retry_after = hosted::retry_delay(&header("retry-after"), if status == 429 { &resets } else { &[] });
        let success = (200..300).contains(&status);
        if let (Some((assembler, on)), true) = (stream, success) {
            let mut stopped = false;
            let mut problem = None;
            let read = response.events(|name, data| match assembler.event(name, data, on) {
                Ok(true) => Ok(true),
                Ok(false) => {
                    stopped = true;
                    Ok(false)
                }
                Err(e) => {
                    problem = Some(e);
                    Ok(false)
                }
            });
            req.cancel.check().map_err(ModelError::cancelled)?;
            // A malformed stream is a malformed reply, as it is without streaming.
            if let Some(e) = problem {
                return Err(ModelError::fatal(redact_credentials(
                    format!("{}: {e}", self.kind()),
                    headers,
                )));
            }
            read.map_err(failed)?;
            // Stopped by its caller: what arrived is the reply (the connection is closed).
            return Ok((status, J::Null, retry_after, stopped));
        }
        let data = response
            .body(if success { super::http::BODY_MAX } else { ERROR_BODY_MAX })
            .map_err(failed)?;
        req.cancel.check().map_err(ModelError::cancelled)?;
        let resp: J = match serde_json::from_slice(&data) {
            Ok(value) => value,
            Err(_) if !success => J::String(String::from_utf8_lossy(&data).into_owned()),
            Err(e) => return Err(format!("{}: reply is not JSON ({e})", self.kind()).into()),
        };
        Ok((status, resp, retry_after, false))
    }
}

#[cfg(test)]
mod endpoint_tests {
    /// A model error names the binding and endpoint, never the server's address: errors reach
    /// journals, traces and apps.
    #[test]
    fn model_errors_do_not_name_the_server() {
        assert_eq!(
            super::without_endpoint(
                "upstream http://10.1.2.3:30001/v1 refused; 10.1.2.3:30001 down; 10.1.2.3 unknown",
                "http://10.1.2.3:30001/v1"
            ),
            "upstream the model server refused; the model server down; the model server unknown"
        );
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let mut s = super::Sglang::new(&format!("http://127.0.0.1:{port}"), 1);
        s.stream = false;
        let e = super::Model::generate(
            &s,
            &super::GenRequest::new("a", vec![1, 2], 4, vec![], None, super::CancelToken::new()),
        )
        .unwrap_err();
        assert!(
            e.message.starts_with("sglang /generate: ")
                && !e.message.contains("127.0.0.1")
                && !e.message.contains(&port.to_string()),
            "{}",
            e.message
        );
        assert_eq!(
            e.kind,
            super::FailureKind::Unreachable,
            "a refused connection is unreachable"
        );
    }
}
