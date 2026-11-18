//! Affordance's one HTTP client, as a browser has one network stack under every request it makes.
//! Model calls (token-id servers and hosted APIs alike) and an app's `fetch` all go through it:
//! one connection pool, one TLS implementation, one policy for certificates and proxies.
//!
//! - **TLS** is rustls. A server's certificate must chain to a root the operating system trusts or
//!   to the Mozilla roots, so a company's own certificate authority works as it does in a browser,
//!   and a minimal container with no certificate store still reaches public servers.
//! - **Proxies** come from the environment (`HTTPS_PROXY`, `http_proxy`, `ALL_PROXY`, `NO_PROXY`),
//!   as curl reads them. Loopback addresses never go through a proxy.
//! - **Connections** are kept alive and pooled per host; an idle one is closed after 4 s (servers
//!   close idle ones at 5). A request that finds its pooled connection closed by the server before
//!   sending anything is sent again on a new one. Redirects are not followed and nothing else is
//!   retried here: callers decide, from the kind of failure.
//! - **Bounds**, as a browser bounds what it reads: a response body (or one stream event) of at
//!   most `BODY_MAX` bytes, the first `ERROR_BODY_MAX` bytes of a failure's body, a response
//!   head of at most 100 header fields (hyper's limits), and no more than 1 MiB read ahead of
//!   the caller. A misbehaving server fails its own request with a protocol error (not retried)
//!   and never makes the process that hosts every agent allocate what it sends.
//! - **Time**: a non-streaming request has one deadline for the whole exchange; a stream bounds
//!   each wait for data.
//! - **Cancellation** drops the request, which closes its connection: a model server stops
//!   generating for a client that went away.
//! - **Errors** name what failed, never the server's address: they reach journals and traces.
//!
//! The network runs on a small shared I/O runtime. Parsing, and every callback a caller passes,
//! run on the caller's own thread.
use super::cancel::CancelToken;
use serde_json::Value as J;
use std::time::{Duration, Instant};

/// Why a request failed, which decides whether sending it again can help.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    /// Before the request was sent (no address, a refused or timed-out connect): the server holds
    /// nothing of it.
    Unsent,
    /// The exchange broke after it was sent: closed, reset, timed out, stalled or cancelled.
    Transport,
    /// The server answered with this status (not 200).
    Status(u16),
    /// The answer broke the protocol or a bound, or the request could not be formed: sending it
    /// again will not change it.
    Protocol,
}

/// A failed request: what kind of failure, and a message that names what failed (never the
/// server's address).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpError {
    /// The kind of failure.
    pub failure: Failure,
    /// What failed.
    pub message: String,
    /// The `Retry-After` header of a response that failed with a status, as the server wrote it.
    pub retry_after: Option<String>,
}

impl HttpError {
    /// A failure of the given kind.
    pub fn new(failure: Failure, message: impl Into<String>) -> Self {
        HttpError {
            failure,
            message: message.into(),
            retry_after: None,
        }
    }
    /// The connection failed or was cut.
    pub fn transport(message: impl Into<String>) -> Self {
        Self::new(Failure::Transport, message)
    }
    /// The server answered with something that is not the protocol.
    pub fn protocol(message: impl Into<String>) -> Self {
        Self::new(Failure::Protocol, message)
    }
    /// The HTTP status the server answered with, if it answered with one.
    pub fn status(&self) -> Option<u16> {
        match self.failure {
            Failure::Status(s) => Some(s),
            _ => None,
        }
    }
    /// Whether the failure may pass: a broken exchange, an unsent request, 429 or a 5xx.
    pub fn transient(&self) -> bool {
        match self.failure {
            Failure::Unsent | Failure::Transport => true,
            Failure::Status(s) => status_may_pass(s),
            Failure::Protocol => false,
        }
    }
    /// The same failure with its message rewritten (a prefix, a redaction).
    pub fn map(self, f: impl FnOnce(String) -> String) -> Self {
        HttpError {
            failure: self.failure,
            message: f(self.message),
            retry_after: self.retry_after,
        }
    }
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl From<HttpError> for String {
    fn from(e: HttpError) -> String {
        e.message
    }
}

/// Captures and traces record a failure as its message.
impl serde::Serialize for HttpError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.message)
    }
}

/// An HTTP status that a later try may not get: the server is busy (429) or failing (5xx). The
/// one statement of that rule, for every client's retries and every error's `transient`.
pub fn status_may_pass(status: u16) -> bool {
    status == 429 || status >= 500
}

/// A response body (a non-streaming reply), or one event of a stream: generous (a 100k-token
/// reply is a few MiB of JSON), and still a bound.
pub const BODY_MAX: usize = 64 << 20;
/// What is read of a failure's body: it only goes into an error message.
pub const ERROR_BODY_MAX: usize = 8 << 10;

/// The prefix of an error from before the request was sent (no address, a refused or timed-out
/// connect): the server holds nothing of it.
#[cfg_attr(not(feature = "http"), allow(dead_code))]
const UNSENT: &str = "unreachable: ";

fn over(what: &str, max: usize) -> HttpError {
    HttpError::protocol(format!("ProtocolError: {what} over {max} bytes"))
}

fn cancelled(cancel: &CancelToken) -> HttpError {
    HttpError::transport(format!("cancelled: {}", cancel.reason()))
}

/// The error of a request out of time: past its deadline, or (a stream) waiting too long for `what`.
#[cfg_attr(not(feature = "http"), allow(dead_code))]
fn late(timeout: Duration, deadline: Option<Instant>, what: &str) -> HttpError {
    HttpError::transport(if deadline.is_some() {
        format!("timed out: no complete response in {}s", timeout.as_secs_f64())
    } else {
        format!("stalled: no {what} for {}s", timeout.as_secs_f64())
    })
}

/// A request to send: method, URL, headers and body.
pub struct Request<'a> {
    /// `GET` or `POST`.
    pub method: &'a str,
    /// An `http://` or `https://` URL.
    pub url: &'a str,
    /// Header fields, in order.
    pub headers: &'a [(&'a str, String)],
    /// The body (empty for none).
    pub body: Vec<u8>,
}

/// How long a request may take: `timeout` for the whole exchange (`whole`), or for each wait for
/// data (a stream). An exchange with one deadline may also bound each wait for a piece of its
/// body (`between`).
#[derive(Clone, Copy)]
pub struct Timing {
    /// The limit.
    pub timeout: Duration,
    /// It bounds the whole exchange, not each wait.
    pub whole: bool,
    /// With `whole`: the longest wait for the next piece of the body.
    pub between: Option<Duration>,
}

impl Timing {
    /// One deadline for the whole exchange.
    pub fn deadline(timeout: Duration) -> Self {
        Timing {
            timeout,
            whole: true,
            between: None,
        }
    }
    /// A limit on each wait for data.
    pub fn stall(timeout: Duration) -> Self {
        Timing {
            timeout,
            whole: false,
            between: None,
        }
    }
    /// This timing, with each wait for a piece of the body bounded as well.
    pub fn between(mut self, stall: Option<Duration>) -> Self {
        self.between = stall;
        self
    }
}

/// An exchange in progress: the response's status and headers have arrived; its body is read
/// with `body` or `events`. Dropping it before the body's end closes the connection.
pub struct Exchange {
    /// The response's status.
    pub status: u16,
    /// Its header fields, names in lowercase.
    pub headers: Vec<(String, String)>,
    /// The reason phrase the server sent with its status (`Not Found`), if any.
    pub reason: Option<String>,
    length: Option<u64>,
    #[cfg(feature = "http")]
    inner: net::Body,
    timing: Timing,
    deadline: Option<Instant>,
    cancel: CancelToken,
}

impl Exchange {
    /// The first value of header `name` (lowercase).
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }

    /// The next piece of the body; None at its end.
    pub fn next_chunk(&mut self) -> Result<Option<Vec<u8>>, HttpError> {
        self.chunk("response")
    }

    fn chunk(&mut self, what: &str) -> Result<Option<Vec<u8>>, HttpError> {
        let left = self.deadline.map(|d| d.saturating_duration_since(Instant::now()));
        // The wait that ends first, and the limit it is: the whole exchange's, or one wait's.
        let (wait, timeout, deadline) = match (left, self.timing.between) {
            (Some(left), Some(stall)) if stall < left => (stall, stall, None),
            (Some(left), _) => (left, self.timing.timeout, self.deadline),
            (None, _) => (self.timing.timeout, self.timing.timeout, None),
        };
        #[cfg(feature = "http")]
        {
            self.inner.next(wait, &self.cancel, || late(timeout, deadline, what))
        }
        #[cfg(not(feature = "http"))]
        {
            let _ = (wait, timeout, deadline, what, &self.cancel);
            Ok(None)
        }
    }

    /// The whole body, at most `max` bytes: over it, a protocol error for a success, and the body
    /// cut at `max` for a failure (it only goes into an error message).
    pub fn body(mut self, max: usize) -> Result<Vec<u8>, HttpError> {
        let success = (200..300).contains(&self.status);
        if success && self.length.is_some_and(|n| n > max as u64) {
            return Err(over("response body", max));
        }
        let mut data = vec![];
        while let Some(piece) = self.chunk("response")? {
            if data.len() + piece.len() > max {
                if success {
                    return Err(over("response body", max));
                }
                data.extend_from_slice(&piece[..max - data.len()]);
                return Ok(data);
            }
            data.extend_from_slice(&piece);
        }
        Ok(data)
    }

    /// Read the body as server-sent events, calling `on_event(name, data)` for each (`name` is
    /// `message` unless the event names itself). Chunk boundaries and event boundaries are
    /// independent. Returning false stops reading and closes the connection.
    pub fn events(self, on_event: impl FnMut(&str, &str) -> Result<bool, HttpError>) -> Result<(), HttpError> {
        self.events_until(&std::cell::Cell::new(false), on_event)
    }

    /// `events`, for a stream that says when it is over (`[DONE]`): once `on_event` sets `over`,
    /// what is left of the body is read to its end, for a moment, and nothing in it is an error.
    /// The body's end is what returns the connection to the pool; a response dropped at its last
    /// event, with the body's end still on its way, closes the connection instead, and the next
    /// call pays for a new one.
    pub fn events_until(
        mut self,
        over: &std::cell::Cell<bool>,
        mut on_event: impl FnMut(&str, &str) -> Result<bool, HttpError>,
    ) -> Result<(), HttpError> {
        let mut parser = SseParser::default();
        loop {
            // A body with no length that is not chunked ends when the server closes the
            // connection: there is nothing to keep, so nothing to wait for.
            let framed = self.length.is_some()
                || self
                    .header("transfer-encoding")
                    .is_some_and(|v| v.to_ascii_lowercase().contains("chunked"));
            if over.get() && !framed {
                return Ok(());
            }
            if over.get() {
                let until = Instant::now() + Duration::from_millis(250);
                while let Ok(Some(_)) = self.chunk_within(until.saturating_duration_since(Instant::now())) {}
                return Ok(());
            }
            let piece = self.chunk("data")?;
            let end = piece.is_none();
            if !parser.feed(&piece.unwrap_or_default(), end, &mut on_event)? || end {
                return Ok(());
            }
        }
    }

    /// The next piece of the body if it comes within `wait`.
    fn chunk_within(&mut self, wait: Duration) -> Result<Option<Vec<u8>>, HttpError> {
        #[cfg(feature = "http")]
        {
            self.inner
                .next(wait, &self.cancel, || HttpError::transport("the body did not end"))
        }
        #[cfg(not(feature = "http"))]
        {
            let _ = wait;
            Ok(None)
        }
    }
}

/// Server-sent events from the bytes of a stream, however they are cut into pieces: `event:`
/// names one (`message` otherwise), `data:` lines are its data, a blank line ends it. The one
/// parser: a live response and a recorded stream in a test go through it.
#[derive(Default)]
pub struct SseParser {
    pending: Vec<u8>,
    /// Where the search for the next line end resumes: bytes before it hold none.
    searched: usize,
    name: String,
    data: String,
}

impl SseParser {
    /// Take more of the stream (`end`: there is no more). Calls `on_event(name, data)` for each
    /// event completed; false from it stops the stream, and this returns false.
    pub fn feed(
        &mut self,
        piece: &[u8],
        end: bool,
        on_event: &mut impl FnMut(&str, &str) -> Result<bool, HttpError>,
    ) -> Result<bool, HttpError> {
        self.pending.extend_from_slice(piece);
        let mut from = 0;
        loop {
            // Every complete line; at the end, the rest as a last line.
            let line_end = match self.pending[self.searched.max(from)..].iter().position(|b| *b == b'\n') {
                Some(i) => self.searched.max(from) + i + 1,
                None if end && from < self.pending.len() => self.pending.len(),
                None => break,
            };
            let line = std::str::from_utf8(&self.pending[from..line_end])
                .map_err(|_| HttpError::protocol("event stream is not UTF-8"))?
                .trim_end_matches(['\r', '\n']);
            (from, self.searched) = (line_end, line_end);
            if line.is_empty() {
                if !self.data.is_empty() {
                    let event = if self.name.is_empty() { "message" } else { &self.name };
                    if !on_event(event, self.data.trim_end_matches('\n'))? {
                        return Ok(false);
                    }
                }
                self.name.clear();
                self.data.clear();
            } else if let Some(value) = line.strip_prefix("data:") {
                let value = value.strip_prefix(' ').unwrap_or(value);
                if self.data.len() + value.len() > BODY_MAX {
                    return Err(over("event", BODY_MAX));
                }
                self.data.push_str(value);
                self.data.push('\n');
            } else if let Some(value) = line.strip_prefix("event:") {
                self.name = value.trim().to_string();
            }
        }
        // What is left is the start of a line still arriving: kept, and not searched again.
        self.pending.drain(..from);
        self.searched = self.pending.len();
        if self.pending.len() > BODY_MAX {
            return Err(over("event stream line", BODY_MAX));
        }
        if end && !self.data.is_empty() {
            let event = if self.name.is_empty() { "message" } else { &self.name };
            on_event(event, self.data.trim_end_matches('\n'))?;
        }
        Ok(true)
    }
}

/// Send `request` and wait for the response's status and headers.
pub fn exchange(request: Request<'_>, timing: Timing, cancel: &CancelToken) -> Result<Exchange, HttpError> {
    if cancel.is_cancelled() {
        return Err(cancelled(cancel));
    }
    let deadline = timing.whole.then(|| Instant::now() + timing.timeout);
    #[cfg(feature = "http")]
    {
        let net::Head {
            status,
            headers,
            reason,
            length,
            body: inner,
        } = net::send(request, timing, deadline, cancel)?;
        Ok(Exchange {
            status,
            headers,
            reason,
            length,
            inner,
            timing,
            deadline,
            cancel: cancel.clone(),
        })
    }
    #[cfg(not(feature = "http"))]
    {
        let _ = (request, deadline);
        Err(HttpError::new(
            Failure::Unsent,
            "network requests need Affordance built with the http feature",
        ))
    }
}

/// POST a JSON body; return the HTTP status and decoded JSON. A failure's body need not be JSON
/// (a proxy's 401, a 502 page): it comes back as a string, and its status decides what follows.
/// `timeout` bounds the whole request, from connecting to the body's last byte.
pub fn post_json(
    url: &str,
    payload: &[u8],
    headers: &[(&str, String)],
    timeout: Duration,
    cancel: &CancelToken,
) -> Result<(u16, J), HttpError> {
    post_json_asked(url, payload, headers, timeout, cancel).map(|(status, json, _)| (status, json))
}

/// `post_json`, with the response's `Retry-After` header as the server wrote it.
pub fn post_json_asked(
    url: &str,
    payload: &[u8],
    headers: &[(&str, String)],
    timeout: Duration,
    cancel: &CancelToken,
) -> Result<(u16, J, Option<String>), HttpError> {
    json_request("POST", url, payload.to_vec(), headers, timeout, cancel)
}

/// GET a JSON resource (a server's `/v1/models`); the answer is read as `post_json` reads it.
pub fn get_json(
    url: &str,
    headers: &[(&str, String)],
    timeout: Duration,
    cancel: &CancelToken,
) -> Result<(u16, J), HttpError> {
    json_request("GET", url, vec![], headers, timeout, cancel).map(|(status, json, _)| (status, json))
}

fn json_request(
    method: &str,
    url: &str,
    body: Vec<u8>,
    headers: &[(&str, String)],
    timeout: Duration,
    cancel: &CancelToken,
) -> Result<(u16, J, Option<String>), HttpError> {
    let mut fields: Vec<(&str, String)> = vec![];
    if method == "POST" {
        fields.push(("Content-Type", "application/json".into()));
    }
    fields.extend(headers.iter().cloned());
    let request = Request {
        method,
        url,
        headers: &fields,
        body,
    };
    let response = exchange(request, Timing::deadline(timeout), cancel)?;
    let status = response.status;
    let retry_after = response.header("retry-after").map(str::to_string);
    let data = response.body(if status == 200 { BODY_MAX } else { ERROR_BODY_MAX })?;
    let text = || String::from_utf8_lossy(&data[..data.len().min(300)]).into_owned();
    let json = match serde_json::from_slice(&data) {
        Ok(json) => json,
        Err(_) if status != 200 => J::String(String::from_utf8_lossy(&data).into_owned()),
        Err(e) => {
            return Err(HttpError::protocol(format!(
                "HTTP {status}, response is not JSON ({e}): {}",
                text()
            )));
        }
    };
    Ok((status, json, retry_after))
}

/// POST JSON and stream SSE data events, each decoded as JSON, until `[DONE]` or the end.
/// Returning false from `on_event` closes the request to stop generation. `stall` bounds each
/// wait for data.
pub fn post_sse(
    url: &str,
    payload: &[u8],
    headers: &[(&str, String)],
    stall: Duration,
    cancel: &CancelToken,
    mut on_event: impl FnMut(J) -> Result<bool, HttpError>,
) -> Result<(), HttpError> {
    let mut fields: Vec<(&str, String)> = vec![
        ("Content-Type", "application/json".into()),
        ("Accept", "text/event-stream".into()),
    ];
    fields.extend(headers.iter().cloned());
    let request = Request {
        method: "POST",
        url,
        headers: &fields,
        body: payload.to_vec(),
    };
    let response = exchange(request, Timing::stall(stall), cancel)?;
    let status = response.status;
    if status != 200 {
        let retry_after = response.header("retry-after").map(str::to_string);
        let rest = response.body(ERROR_BODY_MAX)?;
        let mut error = HttpError::new(
            Failure::Status(status),
            format!("HTTP {status}: {}", String::from_utf8_lossy(&rest).trim()),
        );
        error.retry_after = retry_after;
        return Err(error);
    }
    let over = std::cell::Cell::new(false);
    response.events_until(&over, |_, data| {
        if data == "[DONE]" {
            over.set(true);
            return Ok(true);
        }
        let event = serde_json::from_str(data).map_err(|e| {
            let shown: String = data.chars().take(300).collect();
            HttpError::protocol(format!("bad event ({e}): {shown}"))
        })?;
        on_event(event)
    })
}

/// A JSON object whose first field is a token-id array, written straight to bytes (no JSON value
/// per id: a 100k-token prompt is one pass of integer formatting), then `rest`'s fields.
pub fn ids_body(key: &str, ids: &[u32], rest: &J) -> Vec<u8> {
    let mut out = Vec::with_capacity(ids.len() * 7 + 512);
    out.extend_from_slice(b"{");
    serde_json::to_writer(&mut out, key).unwrap();
    out.extend_from_slice(b":[");
    let mut buf = itoa::Buffer::new();
    for (i, t) in ids.iter().enumerate() {
        if i > 0 {
            out.push(b',');
        }
        out.extend_from_slice(buf.format(*t).as_bytes());
    }
    out.push(b']');
    for (k, v) in rest.as_object().into_iter().flatten() {
        out.push(b',');
        serde_json::to_writer(&mut out, k).unwrap();
        out.push(b':');
        serde_json::to_writer(&mut out, v).unwrap();
    }
    out.push(b'}');
    out
}

/// The network itself: the shared client and the I/O runtime it runs on.
#[cfg(feature = "http")]
mod net {
    use super::{CancelToken, Failure, HttpError, Request, Timing, UNSENT, cancelled, late};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
    use std::sync::{Arc, OnceLock};
    use std::time::{Duration, Instant};

    /// What the I/O task hands the caller: the body's pieces, then its end or a failure.
    enum Piece {
        Data(bytes::Bytes),
        End,
        Failed(HttpError),
    }

    /// How far the I/O task reads ahead of its caller: a server that floods a response is read
    /// no further than the caller asked, plus this.
    const READ_AHEAD: usize = 1 << 20;

    /// The body of a response being read on the I/O runtime.
    pub(super) struct Body {
        pieces: Receiver<Piece>,
        /// Bytes handed over and not yet taken, and the wake-up for a task waiting on them.
        queued: Arc<(AtomicUsize, tokio::sync::Notify)>,
        task: tokio::task::AbortHandle,
        done: bool,
    }

    impl Drop for Body {
        fn drop(&mut self) {
            // Dropping the response before its end closes the connection.
            self.task.abort();
        }
    }

    impl Body {
        pub(super) fn next(
            &mut self,
            wait: Duration,
            cancel: &CancelToken,
            late: impl FnOnce() -> HttpError,
        ) -> Result<Option<Vec<u8>>, HttpError> {
            if self.done {
                return Ok(None);
            }
            match self.pieces.recv_timeout(wait) {
                Ok(Piece::Data(b)) => {
                    self.queued.0.fetch_sub(b.len(), Ordering::SeqCst);
                    self.queued.1.notify_one();
                    Ok(Some(b.to_vec()))
                }
                Ok(Piece::End) => {
                    self.done = true;
                    Ok(None)
                }
                Ok(Piece::Failed(e)) => Err(e),
                Err(_) if cancel.is_cancelled() => Err(cancelled(cancel)),
                Err(RecvTimeoutError::Timeout) => Err(late()),
                Err(RecvTimeoutError::Disconnected) => Err(HttpError::transport("the connection closed")),
            }
        }
    }

    struct Net {
        runtime: tokio::runtime::Runtime,
        client: reqwest::Client,
    }

    fn net() -> Result<&'static Net, HttpError> {
        static NET: OnceLock<Result<Net, String>> = OnceLock::new();
        NET.get_or_init(|| {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .thread_name("affordance-net")
                .enable_all()
                .build()
                .map_err(|e| format!("cannot start the network: {e}"))?;
            let client = {
                let _entered = runtime.enter();
                let mut builder = reqwest::Client::builder()
                    .redirect(reqwest::redirect::Policy::none())
                    .retry(reqwest::retry::never())
                    .no_proxy()
                    .connect_timeout(Duration::from_secs(10))
                    .pool_idle_timeout(Duration::from_secs(4))
                    .pool_max_idle_per_host(1024)
                    .tcp_nodelay(true)
                    // Some hosted APIs and the proxies in front of them refuse a request with none.
                    .user_agent(concat!("affordance/", env!("CARGO_PKG_VERSION")))
                    .http1_only();
                for proxy in proxies() {
                    builder = builder.proxy(proxy);
                }
                builder
                    .build()
                    .map_err(|e| format!("cannot create the HTTP client: {e}"))?
            };
            Ok(Net { runtime, client })
        })
        .as_ref()
        .map_err(|e| HttpError::new(Failure::Unsent, e.clone()))
    }

    /// The environment's proxies, as curl reads them, never for loopback addresses.
    fn proxies() -> Vec<reqwest::Proxy> {
        let var = |names: &[&str]| {
            names
                .iter()
                .find_map(|n| std::env::var(n).ok().filter(|v| !v.trim().is_empty()))
        };
        let skip = format!(
            "localhost,127.0.0.0/8,::1,{}",
            var(&["NO_PROXY", "no_proxy"]).unwrap_or_default()
        );
        let no_proxy = || reqwest::NoProxy::from_string(&skip);
        let mut out = vec![];
        if let Some(p) = var(&["HTTPS_PROXY", "https_proxy"]).and_then(|u| reqwest::Proxy::https(u).ok()) {
            out.push(p.no_proxy(no_proxy()));
        }
        // `HTTP_PROXY` in upper case is ignored, as curl ignores it: in a CGI program a client
        // sets it with a `Proxy:` header.
        if let Some(p) = var(&["http_proxy"]).and_then(|u| reqwest::Proxy::http(u).ok()) {
            out.push(p.no_proxy(no_proxy()));
        }
        if let Some(p) = var(&["ALL_PROXY", "all_proxy"]).and_then(|u| reqwest::Proxy::all(u).ok()) {
            out.push(p.no_proxy(no_proxy()));
        }
        out
    }

    /// What failed in `e`, without the URL, and whether the request was sent.
    fn failure(e: reqwest::Error) -> HttpError {
        let e = e.without_url();
        let mut causes: Vec<String> = vec![];
        let mut certificate = false;
        let mut source: Option<&dyn std::error::Error> = std::error::Error::source(&e);
        while let Some(s) = source {
            if s.downcast_ref::<rustls::Error>().is_some() {
                certificate = true;
            }
            causes.push(s.to_string());
            source = s.source();
        }
        causes.dedup();
        let detail = causes.last().cloned().unwrap_or_else(|| e.to_string());
        let any = |text: &str| causes.iter().any(|c| c.contains(text));
        if certificate || any("certificate") || any("InvalidContentType") || any("received corrupt message") {
            HttpError::protocol(format!("TLS: {detail}"))
        } else if e.is_connect() {
            HttpError::new(Failure::Unsent, format!("{UNSENT}{detail}"))
        } else if any("message head is too large") || any("too many headers") {
            HttpError::protocol(
                "ProtocolError: response head too large (over 100 header fields, or a header over the read buffer)",
            )
        } else if e.is_builder() {
            HttpError::protocol(format!("invalid request: {detail}"))
        } else {
            HttpError::transport(detail)
        }
    }

    /// A response's head, and its body to read.
    pub(super) struct Head {
        pub(super) status: u16,
        pub(super) headers: Vec<(String, String)>,
        pub(super) reason: Option<String>,
        pub(super) length: Option<u64>,
        pub(super) body: Body,
    }
    type Received = (u16, Vec<(String, String)>, Option<String>, Option<u64>);

    pub(super) fn send(
        request: Request<'_>,
        timing: Timing,
        deadline: Option<Instant>,
        cancel: &CancelToken,
    ) -> Result<Head, HttpError> {
        let net = net()?;
        let method = reqwest::Method::from_bytes(request.method.as_bytes())
            .map_err(|_| HttpError::protocol("invalid request: method"))?;
        let url = reqwest::Url::parse(request.url).map_err(|_| HttpError::protocol("invalid request: URL"))?;
        if !matches!(url.scheme(), "http" | "https") {
            return Err(HttpError::protocol("invalid request: only http:// and https:// URLs"));
        }
        // A field name goes on the wire once: a sender must not repeat a singleton field such as
        // Content-Type (RFC 9110, 5.3), and strict servers refuse a request that does.
        let mut fields = reqwest::header::HeaderMap::new();
        for (name, value) in request.headers {
            let field = reqwest::header::HeaderName::from_bytes(name.as_bytes())
                .map_err(|_| HttpError::protocol(format!("invalid request: header name {name}")))?;
            let value = reqwest::header::HeaderValue::from_str(value)
                .map_err(|_| HttpError::protocol(format!("invalid request: the value of header {name}")))?;
            if fields.insert(field, value).is_some() {
                return Err(HttpError::protocol(format!(
                    "invalid request: header {name} given twice"
                )));
            }
        }
        let builder = net.client.request(method, url).headers(fields).body(request.body);
        let built = builder
            .build()
            .map_err(|_| HttpError::protocol("invalid request: a header or the body"))?;
        let (head_tx, head_rx) = channel::<Result<Received, HttpError>>();
        let (piece_tx, pieces) = channel::<Piece>();
        let queued = Arc::new((AtomicUsize::new(0), tokio::sync::Notify::new()));
        let counted = queued.clone();
        let client = net.client.clone();
        let task = net.runtime.spawn(async move {
            let mut response = match client.execute(built).await {
                Ok(r) => r,
                Err(e) => {
                    let _ = head_tx.send(Err(failure(e)));
                    return;
                }
            };
            let headers = response
                .headers()
                .iter()
                .map(|(k, v)| {
                    (
                        k.as_str().to_string(),
                        String::from_utf8_lossy(v.as_bytes()).into_owned(),
                    )
                })
                .collect();
            let length = response.content_length();
            let reason = response
                .extensions()
                .get::<hyper::ext::ReasonPhrase>()
                .map(|r| r.as_bytes().iter().map(|b| char::from(*b)).collect());
            if head_tx
                .send(Ok((response.status().as_u16(), headers, reason, length)))
                .is_err()
            {
                return;
            }
            loop {
                // Wait while the caller is behind: what it has not taken stays in the socket.
                loop {
                    let woken = counted.1.notified();
                    if counted.0.load(Ordering::SeqCst) <= READ_AHEAD {
                        break;
                    }
                    woken.await;
                }
                match response.chunk().await {
                    Ok(Some(b)) => {
                        counted.0.fetch_add(b.len(), Ordering::SeqCst);
                        if piece_tx.send(Piece::Data(b)).is_err() {
                            return;
                        }
                    }
                    Ok(None) => {
                        // Let the connection reach the pool before the caller sends its next
                        // request (hyper returns it a moment after the body's end).
                        drop(response);
                        tokio::task::yield_now().await;
                        let _ = piece_tx.send(Piece::End);
                        return;
                    }
                    Err(e) => {
                        let _ = piece_tx.send(Piece::Failed(failure(e)));
                        return;
                    }
                }
            }
        });
        let abort = task.abort_handle();
        let on_cancel = abort.clone();
        cancel.on_cancel(move || on_cancel.abort());
        let body = Body {
            pieces,
            queued,
            task: abort,
            done: false,
        };
        let wait = deadline.map_or(timing.timeout, |d| d.saturating_duration_since(Instant::now()));
        match head_rx.recv_timeout(wait) {
            Ok(Ok((status, headers, reason, length))) => Ok(Head {
                status,
                headers,
                reason,
                length,
                body,
            }),
            Ok(Err(e)) => Err(e),
            Err(_) if cancel.is_cancelled() => Err(cancelled(cancel)),
            Err(RecvTimeoutError::Timeout) => Err(late(timing.timeout, deadline, "response")),
            Err(RecvTimeoutError::Disconnected) => Err(HttpError::transport("the connection closed")),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn ids_body_is_the_json_of_the_object() {
        let rest = serde_json::json!({"sampling_params": {"max_new_tokens": 8, "stop_token_ids": [2]}, "stream": true, "rid": "r\"1"});
        let raw = super::ids_body("input_ids", &[0, 7, 4294967295], &rest);
        let mut want = serde_json::json!({"input_ids": [0, 7, 4294967295u32]});
        for (k, v) in rest.as_object().unwrap() {
            want[k] = v.clone();
        }
        assert_eq!(serde_json::from_slice::<serde_json::Value>(&raw).unwrap(), want);
    }

    /// A repeated field (two Content-Type lines) makes a strict server refuse the request, so the
    /// client refuses to send one, whichever layer added the second.
    #[test]
    #[cfg(feature = "http")]
    fn a_header_given_twice_is_refused_before_anything_is_sent() {
        use super::{CancelToken, Failure, Request, Timing, exchange};
        let headers = [
            ("Content-Type", "application/json".to_string()),
            ("content-type", "application/json".to_string()),
        ];
        let request = Request {
            method: "POST",
            url: "http://127.0.0.1:9/",
            headers: &headers,
            body: vec![],
        };
        let timing = Timing::deadline(std::time::Duration::from_secs(1));
        let error = exchange(request, timing, &CancelToken::new()).err().unwrap();
        assert_eq!(error.failure, Failure::Protocol);
        assert_eq!(error.message, "invalid request: header content-type given twice");
    }

    /// A request cancelled while it waits for a response is reported cancelled, not as the
    /// server closing the connection; and no error names the server's address.
    #[cfg(feature = "http")]
    #[test]
    fn a_cancelled_request_is_cancelled_not_closed() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        // The request is cancelled once the server holds its connection, however long the
        // client took to start (its first use loads the system's certificates).
        let token = super::CancelToken::new();
        let t = token.clone();
        let held = std::thread::spawn(move || {
            let (s, _) = listener.accept().unwrap();
            std::thread::sleep(std::time::Duration::from_millis(100));
            t.cancel("controller pause");
            std::thread::sleep(std::time::Duration::from_millis(700));
            drop(s)
        });
        let url = format!("http://127.0.0.1:{port}/generate");
        let e = super::post_json(&url, b"{}", &[], std::time::Duration::from_secs(5), &token).unwrap_err();
        assert_eq!(e.message, "cancelled: controller pause");
        held.join().unwrap();
        let closed = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let e = super::post_json(
            &format!("http://127.0.0.1:{closed}/generate"),
            b"{}",
            &[],
            std::time::Duration::from_secs(2),
            &super::CancelToken::new(),
        )
        .unwrap_err();
        assert_eq!(e.failure, super::Failure::Unsent);
        assert!(
            !e.message.contains("127.0.0.1") && !e.message.contains(&closed.to_string()),
            "{e}"
        );
    }

    /// A server that answers the TLS handshake with plain HTTP fails as a protocol error: sending
    /// again will not help.
    #[cfg(feature = "http")]
    #[test]
    fn a_failed_tls_handshake_is_not_retried() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let _ = s.read(&mut [0; 512]);
            let _ = s.write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n");
        });
        let e = super::get_json(
            &format!("https://127.0.0.1:{port}/v1/models"),
            &[],
            std::time::Duration::from_secs(5),
            &super::CancelToken::new(),
        )
        .unwrap_err();
        assert_eq!(e.failure, super::Failure::Protocol, "{e}");
        assert!(e.message.starts_with("TLS: "), "{e}");
        server.join().unwrap();
    }

    /// A real hosted server over TLS (needs the network): `cargo test -- --ignored https_reaches`.
    #[cfg(feature = "http")]
    #[test]
    #[ignore]
    fn https_reaches_a_hosted_server() {
        let (status, body) = super::get_json(
            "https://api.github.com/",
            &[("User-Agent", "affordance-test".into())],
            std::time::Duration::from_secs(20),
            &super::CancelToken::new(),
        )
        .unwrap();
        assert_eq!(status, 200, "{body}");
        assert!(body["current_user_url"].is_string(), "{body}");
    }
}
