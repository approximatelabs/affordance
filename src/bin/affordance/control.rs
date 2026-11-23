//! The control protocol: how a program outside the run drives it, as a browser's DevTools protocol
//! drives a page. One JSON object a line, the same over stdio (`affordance host`) and over a Unix
//! socket (`--control PATH` on `run`, `serve` and `host`).
//!
//! In (a request with an `id` is answered `{type: "result", id, value}` or `{type: "error", id,
//! message}`; without one, only a failure is reported, as `{type: "error", message}`):
//!   `{type: "event", target?, name, payload?, key?}`  post an event (answered once it is committed)
//!   `{type: "close", timeout_ms?}`                   close the run (running calls get the timeout)
//!   `{type: "status"}`                               the run's status
//!   `{type: "set_model_limit", limit}`               how many model calls may run at once
//!   `{type: "subscribe", streams: ["status"]}`       push `{type: "status", value}` as it changes
//! Out, besides answers: `{type: "exit", ended}` when the run has ended (socket clients).
//!
//! A socket client that disconnects leaves the run as it is: several may connect, one after
//! another or at once. The socket is the user's alone (mode 0600) and is removed when the run ends.
use affordance::runtime::Control;
use serde_json::{Value as J, json};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Where a connection's lines go: written whole, one at a time.
pub type Sink = Arc<dyn Fn(J) + Send + Sync>;

/// Handle one control request: `Some` if it is one of this protocol's (its answer, sent on `out`
/// now or, for a post, once it is committed); `None` for a type it does not know.
pub fn handle(control: &Control, command: &J, default_target: &str, out: &Sink, done: &Arc<AtomicBool>) -> Option<()> {
    let id = command.get("id").cloned();
    let answer = {
        let out = out.clone();
        let id = id.clone();
        move |result: Result<J, String>| match (result, &id) {
            (Ok(value), Some(id)) => out(json!({"type": "result", "id": id, "value": value})),
            (Ok(_), None) => {}
            (Err(message), Some(id)) => out(json!({"type": "error", "id": id, "message": message})),
            (Err(message), None) => out(json!({"type": "error", "message": message})),
        }
    };
    match command["type"].as_str()? {
        "event" => {
            let Some(name) = command["name"].as_str() else {
                answer(Err("event needs a name".into()));
                return Some(());
            };
            let target = command["target"].as_str().unwrap_or(default_target);
            let payload = command.get("payload").cloned().unwrap_or(json!({}));
            let delivery = control.post(target, name, payload, command["key"].as_str());
            // Answered once committed, without holding up the next request.
            std::thread::spawn(move || answer(delivery.wait().map(|()| J::Null)));
        }
        "close" => {
            let timeout = command["timeout_ms"]
                .as_u64()
                .map_or(Duration::ZERO, Duration::from_millis);
            control.close(timeout);
            answer(Ok(J::Null));
        }
        "status" => answer(Ok(control.status())),
        "set_model_limit" => match command["limit"].as_u64() {
            Some(limit) => {
                control.set_model_limit(limit as usize);
                answer(Ok(J::Null));
            }
            None => answer(Err("set_model_limit needs a limit (a whole number)".into())),
        },
        "subscribe" => {
            let streams: Vec<&str> = command["streams"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(J::as_str)
                .collect();
            if let Some(unknown) = streams.iter().find(|s| **s != "status") {
                answer(Err(format!("no stream {unknown:?} (status)")));
                return Some(());
            }
            if !streams.is_empty() {
                let (control, out, done) = (control.clone(), out.clone(), done.clone());
                std::thread::spawn(move || {
                    let mut last = J::Null;
                    while !done.load(Ordering::SeqCst) {
                        let mut status = control.status();
                        // The wall time changes every time; a change is anything else.
                        let wall = status.as_object_mut().and_then(|o| o.remove("wall_ms"));
                        if status != last {
                            last = status.clone();
                            status["wall_ms"] = wall.unwrap_or(J::Null);
                            out(json!({"type": "status", "value": status}));
                        }
                        std::thread::sleep(Duration::from_millis(100));
                    }
                });
            }
            answer(Ok(J::Null));
        }
        _ => return None,
    }
    Some(())
}

/// The types this protocol knows, for an error naming them.
pub const TYPES: &str = "event, close, status, set_model_limit, subscribe";

/// A listening control socket; dropping it says `exit` to its clients and removes the file.
pub struct Socket {
    path: PathBuf,
    clients: Arc<Mutex<Vec<Sink>>>,
    done: Arc<AtomicBool>,
    pub ended: Arc<Mutex<J>>,
}

impl Drop for Socket {
    fn drop(&mut self) {
        self.done.store(true, Ordering::SeqCst);
        let ended = self.ended.lock().unwrap().clone();
        for client in self.clients.lock().unwrap().iter() {
            client(json!({"type": "exit", "ended": ended}));
        }
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Listen on `path` for controllers of the run behind `control`. A file already there is refused
/// unless it is a socket nothing answers on (a run that crashed left it).
pub fn listen(path: &Path, control: Control, default_target: String) -> Result<Socket, String> {
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::{UnixListener, UnixStream};
    if path.exists() {
        if UnixStream::connect(path).is_ok() {
            return Err(format!("{}: another run is listening there", path.display()));
        }
        use std::os::unix::fs::FileTypeExt;
        let socket = std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_socket());
        if !socket {
            return Err(format!("{}: a file is there that is not a socket", path.display()));
        }
        std::fs::remove_file(path).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    let listener = UnixListener::bind(path).map_err(|e| format!("--control {}: {e}", path.display()))?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let clients: Arc<Mutex<Vec<Sink>>> = Default::default();
    let done = Arc::new(AtomicBool::new(false));
    {
        let (clients, done) = (clients.clone(), done.clone());
        std::thread::Builder::new()
            .name("control".into())
            .spawn(move || {
                for stream in listener.incoming() {
                    let Ok(stream) = stream else { continue };
                    if done.load(Ordering::SeqCst) {
                        break;
                    }
                    let Ok(writer) = stream.try_clone() else { continue };
                    let writer = Mutex::new(writer);
                    let sink: Sink = Arc::new(move |line: J| {
                        let mut w = writer.lock().unwrap();
                        let _ = writeln!(w, "{line}");
                        let _ = w.flush();
                    });
                    clients.lock().unwrap().push(sink.clone());
                    let (control, target, done) = (control.clone(), default_target.clone(), done.clone());
                    std::thread::spawn(move || serve_client(stream, &control, &target, &sink, &done));
                }
            })
            .map_err(|e| e.to_string())?;
    }
    eprintln!("affordance: control socket at {}", path.display());
    Ok(Socket {
        path: path.to_path_buf(),
        clients,
        done,
        ended: Arc::new(Mutex::new(J::Null)),
    })
}

fn serve_client(
    stream: std::os::unix::net::UnixStream,
    control: &Control,
    target: &str,
    out: &Sink,
    done: &Arc<AtomicBool>,
) {
    // This client's subscriptions end with it.
    let gone = Arc::new(AtomicBool::new(false));
    for line in std::io::BufReader::new(stream).split(b'\n') {
        let Ok(line) = line else { break };
        if done.load(Ordering::SeqCst) {
            break;
        }
        let line = String::from_utf8_lossy(&line);
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<J>(&line) {
            Ok(command) => {
                if handle(control, &command, target, out, &gone).is_none() {
                    let message = format!("unknown command type {:?} ({TYPES})", command["type"]);
                    out(match command.get("id") {
                        Some(id) => json!({"type": "error", "id": id, "message": message}),
                        None => json!({"type": "error", "message": message}),
                    });
                }
            }
            Err(e) => out(json!({"type": "error", "message": format!("not a JSON command: {e}")})),
        }
    }
    gone.store(true, Ordering::SeqCst);
}

static SIGNALS: AtomicUsize = AtomicUsize::new(0);

extern "C" fn on_signal(_: i32) {
    SIGNALS.fetch_add(1, Ordering::SeqCst);
}

/// How long running calls get to finish when the run is closed by a signal.
const SIGNAL_GRACE: Duration = Duration::from_secs(30);

/// SIGINT (Ctrl-C) and SIGTERM close the run: running calls get 30 seconds, and a second signal
/// closes it at once. Either way the journals are committed and the run can be restored.
pub fn close_on_signals(control: Control) {
    unsafe extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    const SIGINT: i32 = 2;
    const SIGTERM: i32 = 15;
    // SAFETY: the handler only increments an atomic, which is async-signal-safe.
    unsafe {
        signal(SIGINT, on_signal as extern "C" fn(i32) as usize);
        signal(SIGTERM, on_signal as extern "C" fn(i32) as usize);
    }
    std::thread::spawn(move || {
        let mut seen = 0;
        loop {
            std::thread::sleep(Duration::from_millis(50));
            let now = SIGNALS.load(Ordering::SeqCst);
            if now == seen {
                continue;
            }
            if seen == 0 {
                eprintln!(
                    "affordance: closing: running calls get {} s to finish (signal again to close now)",
                    SIGNAL_GRACE.as_secs()
                );
                control.close(SIGNAL_GRACE);
            }
            if now > 1 {
                control.close(Duration::ZERO);
            }
            seen = now;
        }
    });
}
