//! `affordance serve`: a deployment behind HTTP, as a Service Worker answers `fetch` events. Each
//! request reaches the root instance as the event `request` (`{id, method, path, query, headers,
//! body}`); the app answers with calls on the machine `client`: `{respond: id, status, headers,
//! body}`, or `{respond: id, status, headers, stream: true}`, then `{respond: id, chunk}`..., then
//! `{respond: id, end: true}`. Design: the posting primitive is `Control::post`; the rest is this
//! file and TSX.
//!
//! HTTP itself is hyper's server, as a browser's network stack parses what a Service Worker sees:
//! request lines, headers, chunked and length-delimited bodies, keep-alive, and the limits on all
//! of them. Requests from the network are the riskiest input there is to parse by hand. HTTP/1.1,
//! no TLS (put a proxy in front).
use affordance::runtime::{CallContext, Control, Executor, Runtime};
use bytes::Bytes;
use http_body_util::{BodyExt, Full, Limited, combinators::BoxBody};
use hyper::{Request, Response, StatusCode, body::Incoming};
use serde_json::{Map, Value as J, json};
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::oneshot;

/// All of a request's head: its line and its headers.
const HEAD_MAX: usize = 64 << 10;
/// How long a chunk of a streamed response waits for a client that is not reading.
const CHUNK_WAIT: Duration = Duration::from_secs(30);
/// How long a client may take to send a request's head.
const HEAD_TIMEOUT: Duration = Duration::from_secs(30);

pub struct Settings {
    pub listen: String,
    pub max_body: usize,
    /// How long a request waits for the app's first response before a 504.
    pub timeout: Duration,
}

impl Settings {
    pub fn from_config(serve: &J) -> Result<Self, String> {
        if !serve.is_null() {
            crate::format::check(serve, crate::format::SERVE, &[], "serve")?;
        }
        Ok(Settings {
            listen: serve["listen"].as_str().unwrap_or("127.0.0.1:8000").to_string(),
            max_body: serve["max_body_bytes"].as_u64().unwrap_or(8 << 20) as usize,
            timeout: Duration::from_secs(serve["timeout_s"].as_u64().unwrap_or(600)),
        })
    }
}

type Body = BoxBody<Bytes, Infallible>;

/// A request waiting for its response: where its head goes, then (streamed) its body.
enum Pending {
    Waiting(oneshot::Sender<Response<Body>>),
    /// Shared, so two handlers writing to one response (an event stream with several sources)
    /// take turns, and neither finds the response missing while the other sends.
    Streaming(Arc<tokio::sync::Mutex<http_body_util::channel::Sender<Bytes>>>),
}

/// A request being served: where its response goes, and until when. This table is the whole of
/// a request's state. It is listed when it arrives, and unlisted when it is answered whole, when
/// its stream ends, when its client disconnects (`Unlist`), or when its time is up (`reap`):
/// each of those drops what the entry held, which is what ends the response.
struct Served {
    pending: Pending,
    until: tokio::time::Instant,
}

type Waiting = Arc<Mutex<HashMap<String, Served>>>;

/// The machine `client`: the app's answers to requests. Its calls come from the agent's own
/// thread; a streamed chunk waits for room in its connection's buffer.
struct Client {
    waiting: Waiting,
    io: tokio::runtime::Handle,
}

impl Executor for Client {
    fn call(&self, _ctx: &CallContext, args: &J) -> Result<J, String> {
        let id = args["respond"]
            .as_str()
            .ok_or("client calls answer a request: {respond: id, ...}")?;
        let gone = || format!("no pending request {id} (answered, disconnected, or from before a restart)");
        let mut waiting = self.waiting.lock().unwrap();
        let Served { pending, until } = waiting.remove(id).ok_or_else(gone)?;
        let result = match pending {
            Pending::Streaming(tx) => {
                if args["end"].as_bool() == Some(true) {
                    // Dropping the sender ends the body (after a write in progress).
                    Ok(())
                } else if let Some(chunk) = args.get("chunk") {
                    // The response stays listed while this chunk waits for room, so another
                    // handler's chunk for it queues behind this one.
                    waiting.insert(
                        id.to_string(),
                        Served {
                            pending: Pending::Streaming(tx.clone()),
                            until,
                        },
                    );
                    drop(waiting);
                    let bytes = Bytes::from(text(chunk));
                    // A client that has stopped reading does not hold this thread: after
                    // CHUNK_WAIT with no room, the response is closed as if the client had left.
                    let sent = self.io.block_on(async {
                        tokio::time::timeout(CHUNK_WAIT, async { tx.lock().await.send_data(bytes).await })
                            .await
                            .map_err(|_| ())
                            .and_then(|sent| sent.map_err(|_| ()))
                    });
                    waiting = self.waiting.lock().unwrap();
                    match sent {
                        Ok(()) => Ok(()),
                        Err(_) => {
                            waiting.remove(id);
                            Err("the client went away".to_string())
                        }
                    }
                } else {
                    waiting.insert(
                        id.to_string(),
                        Served {
                            pending: Pending::Streaming(tx),
                            until,
                        },
                    );
                    Err(format!(
                        "request {id} is already streaming: send chunks, then {{end: true}}"
                    ))
                }
            }
            Pending::Waiting(head) => {
                if args.get("chunk").is_some() || args["end"].as_bool() == Some(true) {
                    waiting.insert(
                        id.to_string(),
                        Served {
                            pending: Pending::Waiting(head),
                            until,
                        },
                    );
                    Err("a chunk needs a streamed response: {respond, stream: true} first".to_string())
                } else if args["stream"].as_bool() == Some(true) {
                    let (tx, body) = http_body_util::channel::Channel::new(16);
                    match response(args, body.boxed()) {
                        Ok(response) => {
                            let sent = head.send(response).map_err(|_| "the client went away".to_string());
                            if sent.is_ok() {
                                waiting.insert(
                                    id.to_string(),
                                    Served {
                                        pending: Pending::Streaming(Arc::new(tokio::sync::Mutex::new(tx))),
                                        until,
                                    },
                                );
                            }
                            sent
                        }
                        // An answer that cannot be sent (a bad status or header) leaves the
                        // request waiting, so the app can answer it properly.
                        Err(e) => {
                            waiting.insert(
                                id.to_string(),
                                Served {
                                    pending: Pending::Waiting(head),
                                    until,
                                },
                            );
                            Err(e)
                        }
                    }
                } else {
                    let body = args.get("body").map(text).unwrap_or_default();
                    match response(args, Full::new(Bytes::from(body)).boxed()) {
                        Ok(response) => head.send(response).map_err(|_| "the client went away".to_string()),
                        Err(e) => {
                            waiting.insert(
                                id.to_string(),
                                Served {
                                    pending: Pending::Waiting(head),
                                    until,
                                },
                            );
                            Err(e)
                        }
                    }
                }
            }
        };
        result.map(|_| json!({"sent": true})).map_err(|e| {
            if e.starts_with("request ") {
                e
            } else {
                format!("request {id}: {e}")
            }
        })
    }
}

/// A body: a string as it is, anything else as JSON.
fn text(v: &J) -> String {
    v.as_str().map(str::to_owned).unwrap_or_else(|| v.to_string())
}

/// The response an app's answer describes: its status and headers (a content type by default).
fn response(args: &J, body: Body) -> Result<Response<Body>, String> {
    let status = args["status"].as_u64().unwrap_or(200);
    let status = u16::try_from(status)
        .ok()
        .and_then(|s| StatusCode::from_u16(s).ok())
        .ok_or_else(|| format!("status {status} is not an HTTP status"))?;
    let mut out = Response::builder().status(status);
    let mut typed = false;
    for (k, v) in args["headers"].as_object().into_iter().flatten() {
        typed |= k.eq_ignore_ascii_case("content-type");
        let name = hyper::header::HeaderName::from_bytes(k.as_bytes())
            .map_err(|_| format!("header {k:?} is not a valid name"))?;
        let value =
            hyper::header::HeaderValue::from_str(&text(v)).map_err(|_| format!("header {k:?} has an invalid value"))?;
        out = out.header(name, value);
    }
    if !typed {
        let json = args.get("body").is_some_and(|b| !b.is_string());
        out = out.header(
            "content-type",
            if json {
                "application/json"
            } else {
                "text/plain; charset=utf-8"
            },
        );
    }
    out.body(body).map_err(|e| e.to_string())
}

/// An error the server answers itself, as JSON.
fn error(status: u16, message: impl Into<String>) -> Response<Body> {
    let body = json!({"error": {"message": message.into()}}).to_string();
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(body)).boxed())
        .unwrap()
}

/// Unlist the requests whose time is up. Dropping an entry ends its response: a request not yet
/// answered gets 504, a streamed one is cut.
async fn reap(waiting: Waiting) {
    loop {
        tokio::time::sleep(Duration::from_millis(250)).await;
        let now = tokio::time::Instant::now();
        waiting.lock().unwrap().retain(|_, request| request.until > now);
    }
}

/// Bind `client` and start the listener, delivering requests to `target`. Requests are numbered
/// within this process run, so ids from a run before a restart never collide with new ones.
pub fn start(rt: &mut Runtime, settings: Settings, target: String) -> Result<(), String> {
    let io = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .thread_name("affordance-serve")
        .enable_all()
        .build()
        .map_err(|e| format!("serve: cannot start the server: {e}"))?;
    let listener = {
        let _entered = io.enter();
        let std = std::net::TcpListener::bind(&settings.listen)
            .map_err(|e| format!("serve: listen on {}: {e}", settings.listen))?;
        std.set_nonblocking(true).map_err(|e| e.to_string())?;
        tokio::net::TcpListener::from_std(std).map_err(|e| e.to_string())?
    };
    let addr = listener.local_addr().map_err(|e| e.to_string())?;
    eprintln!("affordance: serving on http://{addr}");
    if !addr.ip().is_loopback() {
        eprintln!(
            "affordance: warning: listening beyond this machine on {addr}, with no TLS and no login. Anyone who reaches it can talk to the app. Check the Authorization header in the app, or put a proxy in front for TLS and auth."
        );
    }
    let waiting: Waiting = Arc::default();
    rt.bind_machine(
        "client",
        Arc::new(Client {
            waiting: waiting.clone(),
            io: io.handle().clone(),
        }),
    );
    let shared = Arc::new(Shared {
        control: rt.control(),
        target,
        settings,
        waiting,
        run: format!("{:x}", chrono::Utc::now().timestamp_millis()),
        next: std::sync::atomic::AtomicU64::new(0),
    });
    std::thread::spawn(move || {
        io.block_on(async move {
            // One task ends every request whose time is up, answered or streaming.
            tokio::spawn(reap(shared.waiting.clone()));
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    // Out of descriptors, say: wait, do not spin.
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    continue;
                };
                let shared = shared.clone();
                tokio::spawn(async move {
                    let service = hyper::service::service_fn(move |req| {
                        let shared = shared.clone();
                        async move { Ok::<_, Infallible>(shared.handle(req).await) }
                    });
                    let _ = hyper::server::conn::http1::Builder::new()
                        .timer(hyper_util::rt::TokioTimer::new())
                        .header_read_timeout(HEAD_TIMEOUT)
                        .max_buf_size(HEAD_MAX)
                        .serve_connection(hyper_util::rt::TokioIo::new(stream), service)
                        .await;
                });
            }
        })
    });
    Ok(())
}

struct Shared {
    control: Control,
    target: String,
    settings: Settings,
    waiting: Waiting,
    run: String,
    next: std::sync::atomic::AtomicU64,
}

impl Shared {
    /// Read one request, post it, and wait until the app answers (or time out with a 504).
    async fn handle(&self, req: Request<Incoming>) -> Response<Body> {
        let n = self.next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let id = format!("{}-{n}", self.run);
        let (parts, body) = req.into_parts();
        let body = match Limited::new(body, self.settings.max_body).collect().await {
            Ok(b) => b.to_bytes(),
            Err(e) if e.downcast_ref::<http_body_util::LengthLimitError>().is_some() => {
                return error(413, format!("body over {} bytes", self.settings.max_body));
            }
            Err(e) => return error(400, format!("body: {e}")),
        };
        let Ok(body) = String::from_utf8(body.to_vec()) else {
            return error(400, "the body must be UTF-8 text");
        };
        let mut headers = Map::new();
        for (k, v) in &parts.headers {
            let v = String::from_utf8_lossy(v.as_bytes()).into_owned();
            let value = match headers.get(k.as_str()).and_then(J::as_str) {
                Some(prev) => format!("{prev}, {v}"),
                None => v,
            };
            headers.insert(k.as_str().to_string(), J::String(value));
        }
        let (head, answered) = oneshot::channel();
        let until = tokio::time::Instant::now() + self.settings.timeout;
        self.waiting.lock().unwrap().insert(
            id.clone(),
            Served {
                pending: Pending::Waiting(head),
                until,
            },
        );
        let request = json!({"id": id, "method": parts.method.as_str(), "path": parts.uri.path(),
            "query": parts.uri.query().unwrap_or(""), "headers": headers, "body": body});
        // Not delivered (the run is closing): 503, and the client may try again.
        let delivery = self.control.post(&self.target, "request", request, None);
        let mut delivered = Some(tokio::task::spawn_blocking(move || delivery.wait()));
        // A client that disconnects drops this future: its request then stops waiting too.
        struct Unlist(Waiting, String, bool);
        impl Drop for Unlist {
            fn drop(&mut self) {
                if self.2 {
                    self.0.lock().unwrap().remove(&self.1);
                }
            }
        }
        let mut unlist = Unlist(self.waiting.clone(), id.clone(), true);
        // Answered, or unlisted with no answer: its time was up (`reap`).
        tokio::pin!(answered);
        loop {
            tokio::select! {
                response = &mut answered => break match response {
                    Ok(response) => {
                        unlist.2 = false;
                        response
                    }
                    Err(_) => error(504, "the app did not answer in time"),
                },
                posted = async { delivered.as_mut().unwrap().await }, if delivered.is_some() => {
                    delivered = None;
                    if let Ok(Err(e)) = posted {
                        return error(503, e);
                    }
                }
            }
        }
    }
}
