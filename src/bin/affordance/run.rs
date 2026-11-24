//! `affordance run`: run the agents a deployment describes (an app, its model bindings, its
//! machines), with a journal per agent. Nothing here knows the app: tools, state, context policy
//! and reply grammar live in its TSX. The deployment format is in docs/RUNTIME.md; loading it
//! (files, overrides, paths) is `deploy.rs`.
#[cfg(feature = "http")]
use affordance::connectors::http::{Http, HttpConfig};
use affordance::emit::Encoder;
use affordance::runtime::{AgentSpec, CommandMachine, Exec, Executor, Model, Runtime, RuntimeConfig, Target};
use serde_json::{Value as J, json};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

/// Run `f`, sampling it under `AFFORDANCE_FLAMEGRAPH=<path>` (built with `--features profile`):
/// a flame graph there, and the hottest frames by self and inclusive time on stderr.
pub fn with_profile(f: impl FnOnce() -> Result<(), String>) -> Result<(), String> {
    #[cfg(feature = "profile")]
    let profiler = affordance::profile::start(499);
    let result = f();
    #[cfg(feature = "profile")]
    if let Some(profile) = profiler {
        let samples = profile.finish();
        let mut own: std::collections::HashMap<String, usize> = Default::default();
        let mut all: std::collections::HashMap<String, usize> = Default::default();
        let mut total = 0;
        for &(ref names, n) in &samples {
            total += n;
            if let Some(f) = names.first() {
                *own.entry(f.clone()).or_default() += n;
            }
            for name in names.iter().collect::<std::collections::BTreeSet<_>>() {
                if name.starts_with("affordance::") {
                    *all.entry(name.clone()).or_default() += n;
                }
            }
        }
        // Each sample in one phase: the innermost frame matching a phase, leaf first.
        let phases: [(&str, &[&str]); 9] = [
            (
                "scan bookkeeping",
                &["MemoStore::resume", "MemoStore::put", "value::freeze"],
            ),
            ("turns (logTurns, templateInput)", &["turns::"]),
            ("tokenize", &["fasttok::", "Encoder::encode"]),
            ("layout (emit, document)", &["emit::", "interp::document::"]),
            (
                "state check, lower, sweep",
                &[
                    "value::check_graph",
                    "value::freeze_state",
                    "blobs::Blobs::lower",
                    "heap::sweep",
                    "heap::Node",
                ],
            ),
            ("handler start (snapshot)", &["Interp::from_snapshot", "Interp::new"]),
            ("scan steps (app code in memoScan)", &["Interp::memo_scan"]),
            ("app code", &["affordance::interp::"]),
            ("runtime", &["affordance::runtime::"]),
        ];
        let mut by_phase: std::collections::BTreeMap<&str, usize> = Default::default();
        for &(ref names, n) in &samples {
            let mut phase = names
                .iter()
                .find_map(|name| {
                    phases
                        .iter()
                        .find(|(_, pats)| pats.iter().any(|p| name.contains(p)))
                        .map(|(ph, _)| *ph)
                })
                .unwrap_or("other");
            // App code, by where it runs: a scan's step, a handler, or the page's render.
            if phase == "app code" {
                let under = |p: &str| names.iter().any(|n| n.contains(p));
                phase = if under("Interp::memo_scan") {
                    "app code: scan steps"
                } else if under("call_handler") || under("run_task") {
                    "app code: handlers"
                } else {
                    "app code: render"
                };
            }
            if phase == "runtime"
                && let Some(f) = names.iter().find(|f| f.starts_with("affordance::runtime::"))
            {
                *by_phase
                    .entry(Box::leak(
                        format!("runtime: {}", f.trim_start_matches("affordance::runtime::")).into_boxed_str(),
                    ))
                    .or_default() += n;
                continue;
            }
            *by_phase.entry(phase).or_default() += n;
        }
        let mut v: Vec<_> = by_phase.into_iter().collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        for (name, n) in v {
            eprintln!("phase {:5.1}% {name}", 100.0 * n as f64 / total.max(1) as f64);
        }
        // Stacks by their nearest affordance frames (leaf first), for what calls the hot code.
        let mut stacks: std::collections::HashMap<String, usize> = Default::default();
        for &(ref names, n) in &samples {
            let names: Vec<&str> = names
                .iter()
                .map(String::as_str)
                .filter(|n| n.starts_with("affordance::") && !n.contains("{{closure}}"))
                .take(6)
                .collect();
            *stacks.entry(names.join(" < ")).or_default() += n;
        }
        let mut v: Vec<_> = stacks.into_iter().collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        for (name, n) in v.iter().take(40) {
            eprintln!(
                "stack {:5.1}% {}",
                100.0 * *n as f64 / total.max(1) as f64,
                name.replace("affordance::", "").chars().take(330).collect::<String>()
            );
        }
        for (label, map) in [("self", own), ("incl", all)] {
            let mut v: Vec<_> = map.into_iter().collect();
            v.sort_by(|a, b| b.1.cmp(&a.1));
            for (name, n) in v.iter().take(30) {
                eprintln!(
                    "{label} {:5.1}% {}",
                    100.0 * *n as f64 / total.max(1) as f64,
                    name.chars().take(140).collect::<String>()
                );
            }
        }
    }
    result
}

/// A machine pool's role names: `{n}` and `{n:04}` in a role (`"vm-{n:02}": {count: 3}`).
fn pool_role(role: &str, n: usize) -> String {
    match fill(&J::String(role.into()), n) {
        J::String(r) => r,
        other => other.to_string(),
    }
}

/// `{n}` and `{n:04}` in every string of a pool member's fields.
fn fill_all(v: &J, n: usize) -> J {
    match v {
        J::Array(a) => J::Array(a.iter().map(|x| fill_all(x, n)).collect()),
        J::Object(o) => J::Object(o.iter().map(|(k, x)| (k.clone(), fill_all(x, n))).collect()),
        other => fill(other, n),
    }
}

fn fill(v: &J, n: usize) -> J {
    match v {
        J::String(s) => {
            let mut out = s.replace("{n}", &n.to_string());
            while let Some(i) = out.find("{n:0") {
                let Some(j) = out[i..].find('}') else { break };
                // A width is a few digits; anything else is left as written.
                let Some(width) = out[i + 4..i + j].parse::<usize>().ok().filter(|w| *w <= 32) else {
                    break;
                };
                out.replace_range(i..i + j + 1, &format!("{n:0width$}"));
            }
            J::String(out)
        }
        other => other.clone(),
    }
}

/// A machine binding from the config: `{kind: local | ssh | docker | bwrap | command | http, ...}`.
fn machine(role: &str, spec: &J) -> Result<Arc<dyn Executor>, String> {
    let s = |k: &str| spec[k].as_str().map(str::to_owned);
    let target = match spec["kind"].as_str() {
        #[cfg(feature = "http")]
        Some("http") => {
            let mut config = spec.clone();
            config.as_object_mut().unwrap().remove("kind");
            let config: HttpConfig = serde_json::from_value(config).map_err(|e| format!("machine {role}: {e}"))?;
            return Ok(Arc::new(Http::new(config)?));
        }
        #[cfg(not(feature = "http"))]
        Some("http") => return Err("HTTP bindings require the `http` Cargo feature (enabled by default)".into()),
        Some("command") => {
            // Each argument as text: a number or a boolean in YAML (`[serve, 8080, true]`) is the
            // argument it was written as, never dropped.
            let argv: Vec<String> = spec["argv"]
                .as_array()
                .ok_or("a command machine needs argv")?
                .iter()
                .map(|a| match a {
                    J::String(s) => Ok(s.clone()),
                    J::Number(_) | J::Bool(_) => Ok(a.to_string()),
                    other => Err(format!("machine {role}: argv holds {other}, which is not an argument")),
                })
                .collect::<Result<_, _>>()?;
            if argv.is_empty() {
                return Err(format!("machine {role}: argv is empty"));
            }
            let mut m = CommandMachine::new(argv);
            if let Some(n) = spec["max_reply_bytes"].as_u64() {
                m.max_reply = n as usize;
            }
            return Ok(Arc::new(m));
        }
        Some(kind @ ("local" | "ssh" | "docker" | "bwrap")) => {
            if !spec["timeout_s"].is_null() && spec["timeout_s"].as_u64().is_none() {
                return Err(format!("machine {role}: timeout_s must be a whole number of seconds"));
            }
            match kind {
                "local" => Target::Local {
                    home: s("home"),
                    per_instance: spec["per_instance"].as_bool().unwrap_or(false),
                },
                "ssh" => Target::Ssh {
                    target: s("target").ok_or("an ssh machine needs target")?,
                    key: s("key"),
                    control_dir: match s("control_dir") {
                        Some(dir) => dir,
                        None => private_ssh_dir()?,
                    },
                    known_hosts: match s("known_hosts") {
                        Some(file) => file,
                        None => format!("{}/known_hosts", private_ssh_dir()?),
                    },
                },
                "docker" => Target::Docker {
                    image: s("image").ok_or("a docker machine needs image")?,
                    name: s("name").unwrap_or_else(|| "aff".into()),
                },
                _ => Target::Bwrap {
                    home: s("home").ok_or("a bwrap machine needs home")?,
                    per_instance: spec["per_instance"].as_bool().unwrap_or(false),
                    network: spec["network"].as_bool().unwrap_or(false),
                    read: spec["read"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|p| p.as_str().map(String::from))
                        .collect(),
                },
            }
        }
        other => {
            return Err(format!(
                "machine {role}: unknown kind {other:?} (local, ssh, docker, bwrap, command, http)"
            ));
        }
    };
    Ok(Arc::new(Exec {
        target,
        default_timeout_s: spec["timeout_s"].as_u64().unwrap_or(60),
        max_capture: 16 << 20,
    }))
}

/// A model server from the config: `{kind: sglang | completions | chat | anthropic | responses, url, ...}`. `ranks` (sglang
/// and completions): the server's data-parallel size; each agent is pinned to one rank (SGLang:
/// `routed_dp_rank`; vLLM `/v1/completions`: the `X-data-parallel-rank` header). An `sglang`
/// binding may set `sessions: {turns, timeout_s}` (or `true`: 4 turns, 600 s) to send turns over
/// SGLang's session API, only the new ids each turn (`model::Sessions`); off by default. A `chat`
/// binding (a hosted API) takes `model`, `key_file`, `key_name` and optional non-secret
/// `headers`; an `anthropic` binding (Anthropic Messages with prompt caching) takes the same and
/// `cache_ttl`; `responses` selects native OpenAI Responses. Hosted bindings render through
/// `MessagesApp`.
fn model_client(m: &J, enc: &Arc<Encoder>) -> Result<Arc<dyn Model>, String> {
    // A run from the command keeps trying a failing server for five minutes unless the binding's
    // `retry` says otherwise (`for_s`: seconds, or "forever" for a fleet that must outlast an outage).
    let mut m = m.clone();
    if let Some(o) = m.as_object_mut() {
        let retry = o.entry("retry").or_insert(json!({}));
        if let Some(retry) = retry.as_object_mut() {
            retry.entry("for_s").or_insert(json!(300));
        }
    }
    affordance::runtime::model::from_config(&m, enc.clone())
}

/// A deployment's defaults, each stated once: its worker threads, and the most tokens a reply
/// may have.
const WORKERS: u64 = 4;
const MAX_NEW: u64 = 4096;

fn duration(value: &J, name: &str) -> Result<Duration, String> {
    let seconds = value.as_f64().ok_or_else(|| format!("{name} must be a number"))?;
    Duration::try_from_secs_f64(seconds).map_err(|_| format!("{name} must be finite and nonnegative"))
}

/// Run a loaded deployment (`deploy::load`): fresh, or `restore` from its journals. `prepare` sees
/// the runtime before it starts (`serve` binds its `client` machine and takes its control there).
/// The summary at the end goes to stdout, or to stderr when stdout is a protocol's (`host`).
/// SIGINT and SIGTERM close the run; with `control_socket`, controllers connect there.
pub fn run(
    cfg: J,
    restore: bool,
    keep_running: bool,
    summary_on_stdout: bool,
    control_socket: Option<&std::path::Path>,
    prepare: impl FnOnce(&mut Runtime) -> Result<(), String>,
) -> Result<(), String> {
    crate::format::validate(&cfg)?;
    if let Some(replay) = cfg.get("replay")
        && !matches!(replay.as_str(), Some("strict" | "warn"))
    {
        return Err("replay must be strict or warn".into());
    }
    for key in [
        "workers",
        "input_batch",
        "model_threads",
        "exec_threads",
        "model_inflight",
        "restore_concurrency",
        "spill_bytes",
        "call_timeout_s",
    ] {
        if cfg.get(key).is_some_and(|v| !v.is_u64()) {
            return Err(format!("{key} must be a nonnegative integer"));
        }
    }
    if cfg.get("keep_running").is_some_and(|v| !v.is_boolean()) {
        return Err("keep_running must be a boolean".into());
    }
    for key in ["app", "models", "machines", "env"] {
        if cfg.get(key).is_some_and(|v| !v.is_object()) {
            return Err(format!("{key} must be an object"));
        }
    }
    // The instances: `instances: [{id, app?, ...}]`, each running its own `app` or the deployment's
    // (`app: {view, entry?}`); an entry with `count: N` is N copies, `<id>-0` .. `<id>-(N-1)`.
    let app = &cfg["app"];
    let listed = cfg["instances"]
        .as_array()
        .ok_or("instances missing: a deployment lists its instances")?;
    if listed.is_empty() {
        return Err("instances is empty".into());
    }
    let view = app["view"]
        .as_str()
        .or_else(|| listed.iter().find_map(|i| i["app"].as_str()))
        .ok_or("no app: give app.view, or an app for each instance")?;
    // A chat server tokenizes the messages itself, so a hosted binding may name no tokenizer and
    // size its layout with the bundled default; a token-id binding needs its model's own.
    let default_tokenizer;
    let tokenizer = match cfg["model"]["tokenizer"].as_str().or_else(|| app["tokenizer"].as_str()) {
        Some(t) => t,
        None if matches!(cfg["model"]["kind"].as_str(), Some("chat" | "anthropic" | "responses")) => {
            default_tokenizer = affordance::library::default_tokenizer()?;
            default_tokenizer.to_str().ok_or("cache path is not UTF-8")?
        }
        None => return Err("model.tokenizer missing: a token-id binding needs its model's tokenizer".into()),
    };
    let enc = Arc::new(Encoder::from_file(tokenizer)?);
    let mut rc = RuntimeConfig::new(view);
    rc.entry = app["entry"].as_str().unwrap_or("Main").into();
    // The library `@affordance/` imports name: the deployment's, else the checkout the app lives
    // in, else the one bundled with this binary.
    rc.library = Some(match cfg["library"].as_str() {
        Some(dir) => dir.into(),
        None => affordance::library::for_app(std::path::Path::new(view))?,
    });
    if !cfg["serve"].is_null() && !cfg["serve"].is_object() {
        return Err("serve must be a section ({listen, max_body_bytes, timeout_s})".into());
    }
    rc.dir = match &cfg["dir"] {
        J::Null => None,
        J::String(dir) => Some(dir.into()),
        other => return Err(format!("dir must be a path, not {other}")),
    };
    rc.workers = cfg["workers"].as_u64().unwrap_or(WORKERS) as usize;
    rc.input_batch = cfg["input_batch"].as_u64().unwrap_or(1) as usize;
    if cfg
        .get("input_batch")
        .is_some_and(|v| !v.as_u64().is_some_and(|n| (1..=64).contains(&n)))
    {
        return Err("input_batch must be an integer from 1 to 64".into());
    }
    rc.model_threads = cfg["model_threads"].as_u64().unwrap_or(256) as usize;
    rc.exec_threads = cfg["exec_threads"].as_u64().unwrap_or(256) as usize;
    rc.model_inflight = cfg["model_inflight"].as_u64().map(|n| n as usize);
    rc.restore_concurrency = cfg["restore_concurrency"].as_u64().map(|n| n as usize);
    if let Some(n) = cfg["spill_bytes"].as_u64() {
        rc.spill_bytes = n as usize;
    }
    rc.strict_replay = cfg["replay"].as_str() == Some("strict");
    rc.call_timeout = Duration::from_secs(cfg["call_timeout_s"].as_u64().unwrap_or(3600));
    rc.max_wall = cfg.get("max_wall_s").map(|v| duration(v, "max_wall_s")).transpose()?;
    // Keep running (stay up while instances are idle), or run the batch until nothing is left to do.
    // The summary printed at the end reads no state (each agent's snapshot.json has it).
    rc.report_state = false;
    rc.serve = keep_running || cfg["keep_running"].as_bool().unwrap_or(false);
    rc.capture_dir = std::env::var_os("AFFORDANCE_CAPTURE_DIR").map(Into::into);
    rc.env = cfg["env"]
        .as_object()
        .into_iter()
        .flatten()
        .map(|(k, v)| {
            (
                k.clone(),
                v.as_str().map(str::to_owned).unwrap_or_else(|| v.to_string()),
            )
        })
        .collect();
    rc.secrets = serde_json::from_value(cfg.get("secrets").cloned().unwrap_or(json!([])))
        .map_err(|e| format!("secrets: {e}"))?;
    let model = model_client(&cfg["model"], &enc)?;
    // Validate every binding before opening journals. Bindings with the same tokenizer
    // share its immutable encoder while keeping distinct clients and credentials.
    let mut encoders = BTreeMap::from([(tokenizer.to_string(), enc.clone())]);
    let mut named_models = vec![];
    for (name, m) in cfg["models"].as_object().into_iter().flatten() {
        let path = m["tokenizer"]
            .as_str()
            .ok_or_else(|| format!("models.{name}.tokenizer missing"))?;
        if !encoders.contains_key(path) {
            encoders.insert(path.to_string(), Arc::new(Encoder::from_file(path)?));
        }
        let enc = encoders[path].clone();
        let client = model_client(m, &enc).map_err(|e| format!("models.{name}: {e}"))?;
        named_models.push((name.clone(), enc, client));
    }

    let mut specs: Vec<AgentSpec> = vec![];
    for (n, entry) in listed.iter().enumerate() {
        crate::format::check(entry, crate::format::INSTANCE, &[], &format!("instance {n}"))?;
        let id = entry["id"]
            .as_str()
            .ok_or_else(|| format!("instance {n}: id missing"))?;
        let copies: Vec<String> = match &entry["count"] {
            J::Null => vec![id.to_string()],
            J::Number(c) if c.as_u64().is_some() => (0..c.as_u64().unwrap()).map(|i| format!("{id}-{i}")).collect(),
            other => return Err(format!("instance {id}: count must be a whole number, not {other}")),
        };
        for name in copies {
            let mut base = json!({"max_new": MAX_NEW, "props": {}, "seed": specs.len() + 1});
            for (k, v) in entry.as_object().unwrap() {
                if k != "count" {
                    base[k] = v.clone();
                }
            }
            base["id"] = json!(name);
            specs.push(serde_json::from_value(base).map_err(|e| format!("instance {name}: {e}"))?);
        }
    }
    let mut rt = if restore {
        Runtime::restore(rc, enc, model)?
    } else {
        Runtime::new(rc, enc, model)
    };
    if restore {
        rt.membership(specs)?;
    } else {
        for spec in specs {
            rt.add_agent(spec)?;
        }
    }
    for (name, enc, client) in named_models {
        rt.bind_model(&name, enc, client);
    }
    for (role, spec) in cfg["machines"].as_object().into_iter().flatten() {
        // A pool: {"vm-{n:04}": {..., "count": 1024}} binds roles vm-0000 .. vm-1023, `{n}` (the
        // member's number, never an instance's name) filled in each member's role and fields.
        if let Some(count) = spec["count"].as_u64() {
            let mut one = spec.clone();
            one.as_object_mut().unwrap().remove("count");
            for n in 0..count as usize {
                let r = pool_role(role, n);
                rt.bind_machine(&r, machine(&r, &fill_all(&one, n))?);
            }
            continue;
        }
        rt.bind_machine(role, machine(role, spec)?);
    }
    affordance::runtime::host::tune_allocator(cfg["workers"].as_u64().unwrap_or(WORKERS) as usize);
    prepare(&mut rt)?;
    let control = rt.control();
    crate::control::close_on_signals(control.clone());
    let socket = match control_socket {
        Some(path) => {
            let target = rt.specs().first().map(|s| s.id.clone()).unwrap_or_default();
            Some(crate::control::listen(path, control, target)?)
        }
        None => None,
    };
    let report = rt.run();
    if let (Some(socket), Ok(report)) = (&socket, &report) {
        *socket.ended.lock().unwrap() = json!(report.ended);
    }
    drop(socket);
    let report = report?;
    let total =
        |f: &dyn Fn(&affordance::runtime::Metrics) -> u64| report.agents.iter().map(|a| f(&a.metrics)).sum::<u64>();
    let prompt = total(&|m| m.prompt_tokens);
    let summary = serde_json::to_string_pretty(&json!({
        "complete": report.complete, "ended": report.ended, "wall_s": report.wall_s, "agents": report.agents.len(),
        "turns": total(&|m| m.turns), "exact_appends": total(&|m| m.exact_appends), "rewrites": total(&|m| m.rewrites),
        "prompt_tokens": prompt, "uncached_tokens": total(&|m| m.uncached_tokens), "generated_tokens": total(&|m| m.generated_tokens),
        "prefix_reuse": if prompt > 0 { 1.0 - total(&|m| m.uncached_tokens) as f64 / prompt as f64 } else { 0.0 },
        "handler_errors": total(&|m| m.handler_errors), "model_errors": total(&|m| m.model_errors),
        "capture_errors": report.capture_errors,
        "token_mismatches": report.token_mismatches,
        "render_stats": {"evaluations": report.agents.iter().map(|a| a.render_stats.evaluations).sum::<u64>(), "reused": report.agents.iter().map(|a| a.render_stats.reused).sum::<u64>()},
        "per_agent": report.agents.iter().map(|a| json!({"id": a.id, "worker":a.worker, "turns": a.metrics.turns, "stopped": a.stopped, "error": a.error})).collect::<Vec<_>>(),
    })).unwrap();
    if summary_on_stdout {
        println!("{summary}");
    } else {
        eprintln!("{summary}");
    }
    // An agent that failed (to load, to start, or later) fails the run, after the report.
    // So does one whose app stopped it on an error (`useLoop` stops with `loop_error` when its
    // model call fails for good): a script running the command must be able to tell.
    let failed: Vec<String> = report
        .agents
        .iter()
        .filter_map(|a| {
            let stopped_on_error = a.stopped.as_ref().and_then(|s| {
                let reason = s["reason"].as_str()?;
                reason.ends_with("_error").then(|| match s["error"].as_str() {
                    Some(error) => format!("{reason}: {error}"),
                    None => reason.to_string(),
                })
            });
            a.error.clone().or(stopped_on_error).map(|e| format!("{}: {e}", a.id))
        })
        .collect();
    if !failed.is_empty() {
        return Err(format!(
            "{} of {} agents failed:\n  {}",
            failed.len(),
            report.agents.len(),
            failed.join("\n  ")
        ));
    }
    Ok(())
}

/// Where an ssh machine keeps its known-hosts file and its shared connections when the
/// deployment names no place: a directory only this user can enter (`~/.config/affordance/ssh`,
/// mode 0700). Never a shared one such as /tmp, where another local user could plant a host key
/// or a socket before the first connection.
fn private_ssh_dir() -> Result<String, String> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    let home =
        std::env::var("HOME").map_err(|_| "an ssh machine needs control_dir and known_hosts when HOME is not set")?;
    let dir = std::path::Path::new(&home).join(".config/affordance/ssh");
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?;
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))
        .map_err(|e| format!("{}: {e}", dir.display()))?;
    Ok(dir.to_string_lossy().into_owned())
}
