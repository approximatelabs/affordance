//! Executors: how the runtime serves a primitive by name. `exec` runs a shell command in the
//! agent's machine; where that machine is (local bash, SSH, later vsock) is runtime config the
//! app never sees. The app owns the policy: timeouts, what a timeout means, output caps, display.
use serde_json::{Value as J, json};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::process::{Command as Proc, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Who is calling, as the runtime knows it (never taken from the app's arguments).
#[derive(Debug, Clone)]
pub struct CallContext {
    /// The instance that calls.
    pub agent: String,
    /// The call's task, unique within the instance.
    pub task: u64,
    /// The agent's environ (plain and secret values).
    pub env: BTreeMap<String, String>,
    /// Cancelled when the agent no longer needs the result (or the runtime shuts down).
    pub cancel: super::cancel::CancelToken,
    /// Where the runtime keeps large strings: an executor may put a large result there itself
    /// (as `exec` does) instead of returning it on the heap.
    pub blobs: Option<Arc<super::blobs::Blobs>>,
    /// Where an executor hands output as it is produced (a command's stdout and stderr), for
    /// live views (`RuntimeConfig.observer`). The result is unchanged by it.
    pub output: Option<Output>,
}

/// A sink for a call's output as it is produced (`CallContext.output`).
#[derive(Clone)]
pub struct Output(pub Arc<dyn Fn(&[u8]) + Send + Sync>);

impl std::fmt::Debug for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Output")
    }
}

/// A command's merged output, bounded at `cap` bytes. Past the blob size it is written to a
/// file in the blob directory instead of memory, so a large output never sits on the heap.
struct Capture {
    mem: Vec<u8>,
    file: Option<(std::fs::File, std::path::PathBuf)>,
    len: usize,
    truncated: bool,
    error: Option<String>,
}

impl Capture {
    fn push(&mut self, bytes: &[u8], cap: usize, blobs: Option<&super::blobs::Blobs>) {
        let take = bytes.len().min(cap.saturating_sub(self.len));
        if take < bytes.len() {
            self.truncated = true;
        }
        let bytes = &bytes[..take];
        if let (None, Some(b)) = (&self.file, blobs)
            && self.mem.len() + bytes.len() >= b.min_bytes()
            && self.error.is_none()
        {
            let path = b.temp_path();
            match std::fs::File::create(&path).and_then(|mut f| f.write_all(&self.mem).map(|_| f)) {
                Ok(f) => {
                    self.file = Some((f, path));
                    self.mem = Vec::new();
                }
                Err(e) => {
                    let _ = std::fs::remove_file(&path);
                    self.error = Some(e.to_string());
                }
            }
        }
        match &mut self.file {
            Some((f, _)) => {
                if let Err(e) = f.write_all(bytes) {
                    self.error.get_or_insert(e.to_string());
                }
            }
            None => self.mem.extend_from_slice(bytes),
        }
        self.len += take;
    }

    /// The output as a JSON string: text, or a blob reference for a spooled output. Invalid
    /// UTF-8 is replaced as `String::from_utf8_lossy` does.
    fn finish(mut self, blobs: Option<&super::blobs::Blobs>) -> Result<J, String> {
        let Some((f, path)) = self.file.take() else {
            let mem = std::mem::take(&mut self.mem);
            return Ok(J::String(
                String::from_utf8(mem).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned()),
            ));
        };
        drop(f);
        let blobs = blobs.expect("spooled without blobs");
        if let Some(e) = self.error.take() {
            let _ = std::fs::remove_file(&path);
            return Err(format!("exec: capturing output: {e}"));
        }
        if self.truncated {
            lossy_tail(&path).map_err(|e| format!("exec: {e}"))?;
        }
        let stored = match blobs.put_file(&path)? {
            Some(reference) => reference,
            None => {
                let bytes = std::fs::read(&path).map_err(|e| format!("exec: {e}"))?;
                let _ = std::fs::remove_file(&path);
                blobs.put(&String::from_utf8_lossy(&bytes))?
            }
        };
        Ok(J::String(stored))
    }
}

/// The cap can cut the last character short: replace that partial character with U+FFFD in
/// place, as `String::from_utf8_lossy` would, so the file stays UTF-8 without being read.
fn lossy_tail(path: &std::path::Path) -> std::io::Result<()> {
    use std::io::{Seek, SeekFrom};
    let mut f = std::fs::OpenOptions::new().read(true).write(true).open(path)?;
    let len = f.metadata()?.len();
    let n = len.min(3) as usize;
    let mut tail = [0u8; 3];
    f.seek(SeekFrom::End(-(n as i64)))?;
    f.read_exact(&mut tail[..n])?;
    let lead = (0..n).rev().find(|&i| tail[i] & 0xC0 != 0x80);
    let Some(i) = lead else { return Ok(()) };
    let width = match tail[i] {
        b if b >= 0xF0 => 4,
        b if b >= 0xE0 => 3,
        b if b >= 0xC0 => 2,
        _ => 1,
    };
    if n - i >= width {
        return Ok(());
    }
    f.set_len(len - (n - i) as u64)?;
    f.seek(SeekFrom::End(0))?;
    f.write_all("\u{FFFD}".as_bytes())
}

impl Drop for Capture {
    // A capture that is not finished (the command was cancelled) leaves no spool file.
    fn drop(&mut self) {
        if let Some((_, path)) = self.file.take() {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// A primitive served by the runtime. `call` blocks; the runtime runs it on its executor pool.
pub trait Executor: Send + Sync {
    /// Run the call and return its result; an error is the call's failure, as the app sees it.
    fn call(&self, ctx: &CallContext, args: &J) -> Result<J, String>;
}

impl<F: Fn(&CallContext, &J) -> Result<J, String> + Send + Sync> Executor for F {
    fn call(&self, ctx: &CallContext, args: &J) -> Result<J, String> {
        self(ctx, args)
    }
}

/// Where `exec` runs.
#[derive(Debug, Clone)]
pub enum Target {
    /// This host's shell. With `home` (`{agent}` filled, created on first use), commands start
    /// there and see it as `$HOME`: a working directory per agent, not a sandbox.
    Local { home: Option<String> },
    /// `user@host` over SSH with key auth; one ControlMaster connection per target is reused,
    /// so a command costs a channel open rather than a handshake. `{agent}` in the target or key
    /// path is replaced by the agent id (one VM per agent).
    Ssh {
        /// `user@host`.
        target: String,
        /// The private key file, if not the user's default.
        key: Option<String>,
        /// Where the shared connection's socket is kept.
        control_dir: String,
        /// The known-hosts file the connection checks.
        known_hosts: String,
    },
    /// A container per agent: `name` (`{agent}` filled) is started from `image` on first use and
    /// kept running; commands run in it with `docker exec`.
    Docker { image: String, name: String },
    /// A bubblewrap sandbox: the system's programs and libraries read-only (`/usr`, `/bin`,
    /// `/sbin`, `/lib*`, `/etc`, `/opt`), `home` (`{agent}` filled, created on first use)
    /// writable, a private /tmp, and nothing else of the host: no other user's files, no home
    /// directories, no mounted data. `read` names more paths to show read-only (a toolchain
    /// kept under a home directory). No network unless `network`.
    Bwrap {
        home: String,
        network: bool,
        read: Vec<String>,
    },
}

/// `exec {cmd, timeout_s?, env?, stdin?}` -> `{output, exit, seconds, timed_out, truncated}`.
/// The command goes to `bash -l -s` on stdin (never re-quoted into an argv string); stdout and
/// stderr are merged; exit 124 means the remote timeout fired. `max_capture` bounds memory.
pub struct Exec {
    /// Where commands run.
    pub target: Target,
    /// A command's time limit in seconds, unless the call gives one.
    pub default_timeout_s: u64,
    /// The most output kept from one command, in bytes.
    pub max_capture: usize,
}

impl Exec {
    /// The local shell, with the default limits.
    pub fn local() -> Self {
        Exec {
            target: Target::Local { home: None },
            default_timeout_s: 60,
            max_capture: 16 << 20,
        }
    }
}

/// A filesystem/container component with an injective UTF-8 byte encoding. Escaping `_`
/// keeps a literal escape spelling distinct from the byte it represents.
pub fn instance_component(id: &str) -> String {
    let mut out = String::from("i-");
    for byte in id.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.') {
            out.push(char::from(byte));
        } else {
            use std::fmt::Write;
            write!(out, "_{byte:02x}").unwrap();
        }
    }
    out
}

impl Executor for Exec {
    fn call(&self, ctx: &CallContext, args: &J) -> Result<J, String> {
        let cmd = args["cmd"].as_str().ok_or("exec needs cmd")?;
        // Any number of seconds (a model may write 0.5 or 30.0), from one second to a week.
        let timeout_s = match &args["timeout_s"] {
            J::Null => self.default_timeout_s,
            v => v
                .as_f64()
                .filter(|s| s.is_finite())
                .map(|s| s.ceil().clamp(1.0, 604_800.0) as u64)
                .ok_or("exec: timeout_s must be a number of seconds")?,
        }
        .max(1);
        let input = match &args["stdin"] {
            J::Null => None,
            J::String(s) => Some(s.as_str()),
            _ => return Err("exec stdin must be a string".into()),
        };
        // Variables the app passes go to the child's environment, not the command text.
        // stderr joins stdout inside the shell, so the two keep their order. A remote machine
        // (ssh, docker) is given the agent's environ the same way, in the script on stdin: never
        // on a command line, where another user of the host could read it in the process list.
        let mut prelude = String::from("exec 2>&1\n");
        let remote_env = matches!(self.target, Target::Ssh { .. } | Target::Docker { .. });
        for (k, v) in ctx.env.iter().filter(|_| remote_env) {
            if !k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') || k.is_empty() {
                return Err(format!("bad env name {k:?}"));
            }
            prelude.push_str(&format!("export {k}='{}'\n", v.replace('\'', "'\\''")));
        }
        for (k, v) in args["env"].as_object().into_iter().flatten() {
            if !k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                return Err(format!("bad env name {k:?}"));
            }
            let v = v.as_str().map(str::to_owned).unwrap_or_else(|| v.to_string());
            prelude.push_str(&format!("export {k}='{}'\n", v.replace('\'', "'\\''")));
        }
        use std::os::unix::process::CommandExt;
        if matches!(self.target, Target::Bwrap { .. }) && !cfg!(target_os = "linux") {
            return Err("a bwrap machine needs Linux (bubblewrap); use kind: local, docker or ssh here".into());
        }
        // The remote shell is limited where it runs (`timeout` on the guest or in the container).
        let remote = format!("timeout -k 5 {timeout_s} bash -l -s 2>&1");
        // How long the local process may run: the command's limit, with room for a remote one
        // to report its own timeout first.
        let limit = match &self.target {
            Target::Local { .. } | Target::Bwrap { .. } => timeout_s,
            Target::Ssh { .. } => timeout_s + 40,
            Target::Docker { .. } => timeout_s.saturating_add(60),
        };
        let started = Instant::now();
        let mut child = match &self.target {
            // A local command sees the agent's environ, never the runtime's own environment.
            // The shell leads its own process group, so a SIGTERM to that group stops everything
            // the command started.
            Target::Local { home } => {
                let mut c = Proc::new("bash");
                c.env_clear();
                for k in ["PATH", "HOME", "LANG"] { if let Ok(v) = std::env::var(k) { c.env(k, v); } }
                if let Some(home) = home {
                    let home = home.replace("{agent}", &instance_component(&ctx.agent));
                    std::fs::create_dir_all(&home).map_err(|e| format!("local home {home}: {e}"))?;
                    c.env("HOME", &home).current_dir(&home);
                }
                c.envs(&ctx.env);
                c.args(["-l", "-s"]).process_group(0).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()
            }
            Target::Ssh { target, key, control_dir, known_hosts } => {
                let target = target.replace("{agent}", &instance_component(&ctx.agent));
                let mut c = Proc::new("ssh");
                // A guest sharing a CPU quota with thousands of others can take tens of seconds to
                // answer a handshake or a keepalive; those are not dead hosts, so the limits are loose.
                c.args(["-o", "BatchMode=yes", "-o", "StrictHostKeyChecking=accept-new",
                        "-o", &format!("UserKnownHostsFile={known_hosts}"), "-o", "ConnectTimeout=30", "-o", "ControlMaster=auto",
                        "-o", &format!("ControlPath={control_dir}/%r@%h:%p"), "-o", "ControlPersist=600", "-o", "ServerAliveInterval=30", "-o", "ServerAliveCountMax=6"]);
                if let Some(k) = key { c.args(["-i", &k.replace("{agent}", &instance_component(&ctx.agent))]); }
                // `--` before the host: a target that starts with `-` is a host name, never an option.
                c.arg("--").arg(target).arg(&remote).process_group(0).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()
            }
            Target::Docker { image, name } => {
                let name = name.replace("{agent}", &instance_component(&ctx.agent));
                if name.is_empty() || !name.as_bytes()[0].is_ascii_alphanumeric()
                    || !name.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-')) {
                    return Err("docker: invalid container name".into());
                }
                // This program is constant. Names, images and limits are separate argv values;
                // quoted positional parameters are never evaluated as host shell source.
                let script = "docker start -- \"$1\" >/dev/null 2>&1 || docker run -d --name \"$1\" -- \"$2\" sleep infinity >/dev/null || exit 125\nexec docker exec -i -- \"$1\" timeout -k 5 \"$3\" bash -l -s";
                let mut c = Proc::new("bash");
                c.args(["-c", script,
                    "affordance-docker", &name, image, &timeout_s.to_string()])
                    .process_group(0).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()
            }
            Target::Bwrap { home, network, read } => {
                let home = home.replace("{agent}", &instance_component(&ctx.agent));
                std::fs::create_dir_all(&home).map_err(|e| format!("bwrap home {home}: {e}"))?;
                let mut c = Proc::new("bwrap");
                c.env_clear();
                for k in ["PATH", "LANG"] { if let Ok(v) = std::env::var(k) { c.env(k, v); } }
                c.env("HOME", &home).envs(&ctx.env);
                // What a command needs to run, and no more of the host: the system's programs
                // and their configuration, never a home directory or mounted data. A path that
                // is a link on this host (`/bin` to `usr/bin`) is shown as the same link.
                for path in ["/usr", "/bin", "/sbin", "/lib", "/lib32", "/lib64", "/libx32", "/etc", "/opt"] {
                    match std::fs::read_link(path) {
                        Ok(to) => {
                            c.args(["--symlink", &to.to_string_lossy(), path]);
                        }
                        Err(_) => {
                            c.args(["--ro-bind-try", path, path]);
                        }
                    }
                }
                for path in read {
                    let path = path.replace("{agent}", &instance_component(&ctx.agent));
                    c.args(["--ro-bind", &path, &path]);
                }
                c.args(["--dev", "/dev", "--proc", "/proc", "--tmpfs", "/tmp",
                        "--bind", &home, &home, "--chdir", &home, "--die-with-parent", "--unshare-all"]);
                if *network { c.arg("--share-net"); }
                c.args(["bash", "-l", "-s"]).process_group(0).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()
            }
        }.map_err(|e| format!("exec: {e}"))?;
        let mut script = prelude.into_bytes();
        match input {
            // `bash -s` reads its script from a pipe a byte at a time, so a command it runs reads
            // on from where the script stopped. The command travels as data (a quoted here-document
            // whose end line it does not contain) and is run with `eval`; `cat` then takes the rest
            // of the stream, which is exactly the input, into a file the command reads as its
            // standard input. Nothing depends on how a tool buffers its reads, and the input is
            // never read as script, whatever the command's syntax.
            Some(input) => {
                let end = (0..)
                    .map(|n| format!("__AFF_COMMAND_{n}"))
                    .find(|end| !cmd.lines().any(|line| line == end))
                    .unwrap();
                script.extend_from_slice(format!("IFS= read -r -d '' __aff_cmd <<'{end}'\n{cmd}\n{end}\n").as_bytes());
                script.extend_from_slice(b"__aff_in=$(mktemp) && cat >\"$__aff_in\" && exec 3<\"$__aff_in\" && rm -f \"$__aff_in\" || exit 125; eval \"$__aff_cmd\" <&3 3<&-\n");
                script.extend_from_slice(input.as_bytes());
            }
            // With no input the command still travels as data and reads an empty standard input.
            // Sent as plain script it would share bash's own input: a program in it that reads
            // standard input (`cat`, `ssh` without -n, `read`) would take the command's later
            // lines, which then never run.
            None => {
                let end = (0..)
                    .map(|n| format!("__AFF_COMMAND_{n}"))
                    .find(|end| !cmd.lines().any(|line| line == end))
                    .unwrap();
                script.extend_from_slice(format!("IFS= read -r -d '' __aff_cmd <<'{end}'\n{cmd}\n{end}\n").as_bytes());
                script.extend_from_slice(b"eval \"$__aff_cmd\" </dev/null\n");
            }
        }
        // Written on its own thread: a large input fills the pipe, and this thread has to watch.
        if let Some(mut stdin) = child.stdin.take() {
            std::thread::spawn(move || {
                let _ = stdin.write_all(&script);
            });
        }
        // Output is read on its own threads (bounded); this thread watches for exit, cancel and
        // the backstop deadline, so a cancelled command is stopped, not abandoned.
        let captured = Arc::new(Mutex::new(Capture {
            mem: Vec::new(),
            file: None,
            len: 0,
            truncated: false,
            error: None,
        }));
        let mut readers = vec![];
        for pipe in [
            child.stdout.take().map(|p| Box::new(p) as Box<dyn Read + Send>),
            child.stderr.take().map(|p| Box::new(p) as Box<dyn Read + Send>),
        ]
        .into_iter()
        .flatten()
        {
            let (captured, cap, blobs, output) = (
                captured.clone(),
                self.max_capture,
                ctx.blobs.clone(),
                ctx.output.clone(),
            );
            readers.push(std::thread::spawn(move || {
                let mut pipe = pipe;
                let mut buf = [0u8; 65536];
                loop {
                    match pipe.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            if let Some(output) = &output {
                                (output.0)(&buf[..n]);
                            }
                            captured.lock().unwrap().push(&buf[..n], cap, blobs.as_deref());
                        }
                    }
                }
            }));
        }
        let deadline = started + Duration::from_secs(limit);
        let mut cancelled = None;
        let mut timed_out = false;
        let status = loop {
            if let Some(status) = child.try_wait().map_err(|e| format!("exec: {e}"))? {
                // The shell has gone, but what it started may ignore SIGTERM: a cancelled or
                // timed-out command leaves nothing of its group running.
                if cancelled.is_some() {
                    let _ = Proc::new("kill")
                        .args(["-KILL", "--", &format!("-{}", child.id())])
                        .stderr(Stdio::null())
                        .status();
                }
                break Some(status);
            }
            if cancelled.is_none() && (ctx.cancel.is_cancelled() || Instant::now() > deadline) {
                // SIGTERM to the command's process group, then SIGKILL after a grace.
                cancelled = Some(Instant::now());
                timed_out = !ctx.cancel.is_cancelled();
                let pid = child.id().to_string();
                let _ = Proc::new("kill")
                    .args(["-TERM", "--", &format!("-{pid}")])
                    .stderr(Stdio::null())
                    .status();
                let _ = Proc::new("kill").args(["-TERM", &pid]).stderr(Stdio::null()).status();
            }
            if cancelled.is_some_and(|t| t.elapsed() > Duration::from_secs(5)) {
                let _ = Proc::new("kill")
                    .args(["-KILL", "--", &format!("-{}", child.id())])
                    .stderr(Stdio::null())
                    .status();
                let _ = child.kill();
                break child.wait().ok();
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        // Readers end when the pipes close; a stray grandchild holding them open is not waited for.
        let until = Instant::now() + Duration::from_secs(2);
        for r in readers {
            while !r.is_finished() && Instant::now() < until {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        let capture = std::mem::replace(
            &mut *captured.lock().unwrap(),
            Capture {
                mem: Vec::new(),
                file: None,
                len: 0,
                truncated: false,
                error: None,
            },
        );
        let truncated = capture.truncated;
        if cancelled.is_some() && ctx.cancel.is_cancelled() {
            return Err(format!("cancelled: {}", ctx.cancel.reason()));
        }
        // 124 is the exit of a command that ran past its limit, as coreutils `timeout` reports it.
        let exit = if timed_out {
            124
        } else {
            status.and_then(|s| s.code()).unwrap_or(255)
        };
        let output = capture.finish(ctx.blobs.as_deref())?;
        Ok(
            json!({"output": output, "exit": exit, "seconds": started.elapsed().as_secs_f64(),
                  "timed_out": exit == 124, "truncated": truncated}),
        )
    }
}

type Job = Box<dyn FnOnce() + Send>;

/// Threads for executor calls: an idle one takes the next call; with none idle, a new one starts
/// (a call stuck past its deadline holds its thread, never the caller).
static IDLE: Mutex<Vec<(u64, std::sync::mpsc::Sender<Job>)>> = Mutex::new(Vec::new());
static THREADS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn run_on_pool(job: Job) {
    let job = match IDLE.lock().unwrap().pop() {
        Some((_, idle)) => match idle.send(job) {
            Ok(()) => return,
            Err(e) => e.0,
        },
        None => job,
    };
    let id = THREADS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    std::thread::spawn(move || {
        let (tx, rx) = std::sync::mpsc::channel::<Job>();
        let mut job = job;
        loop {
            job();
            IDLE.lock().unwrap().push((id, tx.clone()));
            // An idle thread exits after a minute without work.
            match rx.recv_timeout(Duration::from_secs(60)) {
                Ok(next) => job = next,
                Err(_) => {
                    let mut idle = IDLE.lock().unwrap();
                    // Leave the idle list (unless a call was just handed over: then run it).
                    match rx.try_recv() {
                        Ok(next) => {
                            drop(idle);
                            job = next;
                        }
                        Err(_) => {
                            idle.retain(|(i, _)| *i != id);
                            return;
                        }
                    }
                }
            }
        }
    });
}

/// Run an executor with a wall-clock bound. At the deadline the call's token is cancelled, so
/// the executor stops its work (a cooperative executor returns soon after).
pub fn with_deadline(
    f: impl FnOnce() -> Result<J, String> + Send + 'static,
    limit: Duration,
    cancel: &super::cancel::CancelToken,
) -> Result<J, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    run_on_pool(Box::new(move || {
        let _ = tx.send(f());
    }));
    rx.recv_timeout(limit).unwrap_or_else(|_| {
        cancel.cancel(&format!("did not finish within {}s", limit.as_secs()));
        Err(format!("executor did not finish within {}s", limit.as_secs()))
    })
}

/// A machine that brings its own executor: each call runs `argv` in its own process group, with
/// `{agent, task, env, args}` as JSON on stdin, and takes a JSON result from stdout. Cancelling
/// the call sends SIGTERM to the group, then SIGKILL to what is left after a grace period.
/// A reply past `max_reply` bytes fails the call (and stops the command): JSON cut short is not
/// a result.
pub struct CommandMachine {
    /// The command and its arguments; a call's JSON goes to its stdin.
    pub argv: Vec<String>,
    /// The most bytes its reply may have.
    pub max_reply: usize,
}

impl CommandMachine {
    /// Room for an `exec` reply at the full capture (16 MiB, every byte escaped).
    pub const MAX_REPLY: usize = 128 << 20;
    /// A machine that runs `argv` for each call.
    pub fn new(argv: Vec<String>) -> Self {
        CommandMachine {
            argv,
            max_reply: Self::MAX_REPLY,
        }
    }
}

/// What a failed command's error keeps of its stderr.
const STDERR_KEPT: usize = 64 << 10;

/// A process by pid, with its start time, so a reused pid is never taken for it.
#[derive(Clone, Debug)]
struct Process {
    pid: u32,
    start: u64,
    group: u32,
}

#[cfg(target_os = "linux")]
fn process(pid: u32) -> Option<Process> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // The command name can contain spaces and parentheses; the fields follow its final ')'.
    let fields: Vec<&str> = stat.rsplit_once(") ")?.1.split_whitespace().collect();
    Some(Process {
        pid,
        group: fields.get(2)?.parse().ok()?,
        start: fields.get(19)?.parse().ok()?,
    })
}

#[cfg(target_os = "linux")]
fn group_members(owner: &Process) -> Vec<Process> {
    std::fs::read_dir("/proc")
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| e.file_name().to_str()?.parse::<u32>().ok())
        .filter_map(process)
        .filter(|p| p.group == owner.group)
        .collect()
}

/// Without `/proc` (macOS, the BSDs) the process table is read with `ps`: each line a pid, its
/// group, and when it started (hashed: only equality matters).
#[cfg(not(target_os = "linux"))]
fn processes(selector: &[&str]) -> Vec<Process> {
    let Ok(out) = Proc::new("ps")
        .args(selector)
        .args(["-o", "pid=,pgid=,lstart="])
        .output()
    else {
        return vec![];
    };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let (pid, group) = (fields.next()?.parse().ok()?, fields.next()?.parse().ok()?);
            let started: Vec<&str> = fields.collect();
            let start = started.join(" ").bytes().fold(1469598103934665603u64, |h, b| {
                (h ^ b as u64).wrapping_mul(1099511628211)
            });
            Some(Process { pid, start, group })
        })
        .collect()
}

#[cfg(not(target_os = "linux"))]
fn process(pid: u32) -> Option<Process> {
    processes(&["-p", &pid.to_string()]).into_iter().next()
}

#[cfg(not(target_os = "linux"))]
fn group_members(owner: &Process) -> Vec<Process> {
    processes(&["-ax"])
        .into_iter()
        .filter(|p| p.group == owner.group)
        .collect()
}

/// Stop the call's process group on its own thread: a cancel (a fleet closing cancels many calls at
/// once) must never wait or scan /proc.
fn terminate(owned: std::sync::Weak<Mutex<Option<Process>>>, grace: Duration) {
    std::thread::spawn(move || {
        let Some(state) = owned.upgrade() else { return };
        let members = {
            let guard = state.lock().unwrap();
            let Some(owner) = guard.as_ref() else { return };
            if !process(owner.pid).is_some_and(|live| live.start == owner.start && live.group == owner.group) {
                return;
            }
            // Note the members while the leader still owns the group, so a stubborn descendant
            // is found even after a cooperative leader has exited.
            let members = group_members(owner);
            let _ = Proc::new("kill")
                .args(["-TERM", "--", &format!("-{}", owner.group)])
                .status();
            members
        };
        std::thread::sleep(grace);
        let guard = state.lock().unwrap();
        let Some(owner) = guard.as_ref() else { return };
        if members
            .iter()
            .any(|m| process(m.pid).is_some_and(|live| live.start == m.start && live.group == owner.group))
        {
            let _ = Proc::new("kill")
                .args(["-KILL", "--", &format!("-{}", owner.group)])
                .status();
        }
    });
}

impl Executor for CommandMachine {
    fn call(&self, ctx: &CallContext, args: &J) -> Result<J, String> {
        use std::os::unix::process::CommandExt;
        ctx.cancel.check()?;
        let mut child = Proc::new(&self.argv[0])
            .args(&self.argv[1..])
            .process_group(0)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("{}: {e}", self.argv[0]))?;
        let owned = Arc::new(Mutex::new(process(child.id())));
        let weak = Arc::downgrade(&owned);
        ctx.cancel.on_cancel(move || terminate(weak, Duration::from_secs(5)));
        // The command is deployment code: it keeps the runtime's environment and gets the agent's
        // environ in the payload.
        // Written on its own thread (a large `stdin` must not wait on a command that writes first),
        // and a command that exits without reading is judged by its exit, not by the broken pipe.
        let payload = json!({"agent": ctx.agent, "task": ctx.task, "env": ctx.env, "args": args}).to_string();
        let mut input = child.stdin.take().unwrap();
        std::thread::spawn(move || {
            let _ = input.write_all(payload.as_bytes());
        });
        // Both pipes are read to the end (a command must never block on a full one), and kept
        // only up to their bounds.
        let mut stderr = child.stderr.take().unwrap();
        let errors = std::thread::spawn(move || {
            let mut kept = vec![];
            let _ = (&mut stderr).take(STDERR_KEPT as u64).read_to_end(&mut kept);
            let _ = std::io::copy(&mut stderr, &mut std::io::sink());
            kept
        });
        let mut reply = vec![];
        child
            .stdout
            .take()
            .unwrap()
            .take(self.max_reply as u64 + 1)
            .read_to_end(&mut reply)
            .map_err(|e| e.to_string())?;
        let over = reply.len() > self.max_reply;
        if over {
            reply = Vec::new();
            terminate(Arc::downgrade(&owned), Duration::from_secs(5));
        }
        let status = child.wait().map_err(|e| e.to_string())?;
        let stderr = errors.join().unwrap_or_default();
        *owned.lock().unwrap() = None;
        ctx.cancel.check()?;
        if over {
            return Err(format!(
                "{}: result exceeds {} bytes (max_reply_bytes)",
                self.argv[0], self.max_reply
            ));
        }
        if !status.success() {
            return Err(format!(
                "{} exited {}: {}",
                self.argv[0],
                status,
                String::from_utf8_lossy(&stderr)
            ));
        }
        serde_json::from_slice(&reply).map_err(|e| format!("{}: result is not JSON ({e})", self.argv[0]))
    }
}
