//! A fleet against a mocked SGLang: N agents of `views/agents/session.tsx` on DeepSeek V4.1 (the
//! native ids path), each growing its context to the target, through the real host, journals and
//! the real SGLang client over HTTP to a local server that answers every /generate at once with
//! the same native tool call. Only the harness is measured.
//!
//! `fleet_bench <agents> <context_k> [workers] [model_threads]` prints one JSON line:
//! - throughput: turns/s overall and in the last window (contexts near the target);
//! - CPU: cores used (user+system over wall), and per turn;
//! - memory: RSS and anonymous RSS per agent at the end of the run;
//! - per turn: render ms (agent traces), queue ms before the model call, request bytes, the
//!   HTTP round trip.
use affordance::emit::Encoder;
use affordance::runtime::{CallContext, InstanceSpec, Runtime, RuntimeConfig, Sglang};
use serde_json::{Value as J, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

fn status_kib(field: &str) -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with(field))
                .and_then(|l| l.split_whitespace().nth(1)?.parse().ok())
        })
        .unwrap_or(0)
}

/// User + system CPU seconds of this process.
fn cpu_s() -> f64 {
    let stat = std::fs::read_to_string("/proc/self/stat").unwrap_or_default();
    let f: Vec<&str> = stat
        .rsplit_once(") ")
        .map(|x| x.1)
        .unwrap_or("")
        .split_whitespace()
        .collect();
    let ticks = |i: usize| f.get(i).and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
    (ticks(11) + ticks(12)) / 100.0
}

fn pct(v: &mut [f64], p: f64) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[((v.len() - 1) as f64 * p).round() as usize]
}

/// The mock server, in its own process (its CPU is not the harness's): answers every /generate
/// with `response`; prints its address, then the number of requests and body bytes at exit.
fn mock(response: String) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    println!("{}", listener.local_addr().unwrap());
    std::io::stdout().flush().unwrap();
    let (served, body_bytes) = (Arc::new(AtomicU64::new(0)), Arc::new(AtomicU64::new(0)));
    {
        let (served, body_bytes) = (served.clone(), body_bytes.clone());
        std::thread::spawn(move || {
            // Stdin closes when the bench is done: report and exit.
            let mut s = String::new();
            let _ = std::io::stdin().read_to_string(&mut s);
            println!(
                "{} {}",
                served.load(Ordering::Relaxed),
                body_bytes.load(Ordering::Relaxed)
            );
            std::process::exit(0);
        });
    }
    for conn in listener.incoming().flatten() {
        let (response, served, body_bytes) = (response.clone(), served.clone(), body_bytes.clone());
        // Keep-alive, as SGLang's server: requests on a connection until the client closes it.
        std::thread::spawn(move || {
            let mut r = BufReader::new(conn.try_clone().unwrap());
            let mut conn = conn;
            conn.set_nodelay(true).ok();
            loop {
                let (mut line, mut len) = (String::new(), 0usize);
                loop {
                    line.clear();
                    if r.read_line(&mut line).unwrap_or(0) == 0 {
                        return;
                    }
                    let l = line.trim_end();
                    if l.is_empty() {
                        break;
                    }
                    if let Some(v) = l.to_ascii_lowercase().strip_prefix("content-length:") {
                        len = v.trim().parse().unwrap_or(0);
                    }
                }
                let mut body = vec![0; len];
                if r.read_exact(&mut body).is_err() {
                    return;
                }
                body_bytes.fetch_add(len as u64, Ordering::Relaxed);
                served.fetch_add(1, Ordering::Relaxed);
                // One write (as a server does): pieces would wait on the client's delayed ACK.
                let answer = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    response.len(),
                    response
                );
                if conn.write_all(answer.as_bytes()).is_err() {
                    return;
                }
            }
        });
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--mock") {
        return mock(std::fs::read_to_string(&args[2]).unwrap());
    }
    let arg = |i: usize, d: usize| args.get(i).and_then(|s| s.parse().ok()).unwrap_or(d);
    let (agents, context_k, workers, model_threads) = (arg(1, 100), arg(2, 100), arg(3, 16), arg(4, 512));
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let enc = Arc::new(
        Encoder::from_file(
            root.join("ref/tokenizers/deepseek-ai__DeepSeek-V4.1-Flash.json")
                .to_str()
                .unwrap(),
        )
        .unwrap(),
    );
    let eos = enc.token_id("<｜end▁of▁sentence｜>").unwrap();

    // The mock server: every /generate gets the same native tool call (with reasoning), at once.
    let reasoning: String = (0..40)
        .map(|i| format!("Step {i}: check the build output for regressions and compare timings. "))
        .collect();
    let call = format!(
        "{reasoning}</think>\n\n<｜DSML｜ calls>\n<｜DSML｜ invoke name=\"shell\">\n<｜DSML｜ parameter name=\"command\" string=\"true\">make -j8 test</｜DSML｜ parameter>\n</｜DSML｜ invoke>\n</｜DSML｜ calls>"
    );
    let out_ids = enc.tok.encode(call.as_str(), false).unwrap().get_ids().to_vec();
    let response = json!({"output_ids": out_ids, "meta_info": {"finish_reason": {"type": "stop", "matched": eos}, "cached_tokens": 0, "completion_tokens": out_ids.len()}}).to_string();
    let response_file = std::env::temp_dir().join(format!("aff-fleet-mock-{}.json", std::process::id()));
    std::fs::write(&response_file, &response).unwrap();
    let mut server = std::process::Command::new(std::env::current_exe().unwrap())
        .arg("--mock")
        .arg(&response_file)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut server_out = BufReader::new(server.stdout.take().unwrap());
    let mut addr = String::new();
    server_out.read_line(&mut addr).unwrap();
    let url = format!("http://{}", addr.trim());
    let dir = std::env::temp_dir().join(format!("aff-fleet-bench-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut cfg = RuntimeConfig::new(root.join("views/agents/session.tsx"));
    cfg.report_state = false;
    cfg.workers = workers;
    cfg.dir = Some(dir.clone());
    cfg.model_threads = model_threads;
    cfg.exec_threads = 64;
    let mut client = Sglang::new(&url, 1);
    client.stream = false;
    // The mock reports no context length; the fleet runs without a window.
    client.window = affordance::runtime::model::Window::Unbounded;
    let client = affordance::runtime::model::with_template(
        Arc::new(client),
        affordance::runtime::model::ModelTemplate::new("deepseek_v41", json!({})).unwrap(),
    )
    .unwrap();
    affordance::runtime::host::tune_allocator(cfg.workers); // as the agents binary does
    let mut rt = Runtime::new(cfg, enc.clone(), client);
    // ~1.4k-token tool outputs, distinct per call.
    rt.bind_machine(
        "home",
        Arc::new(|ctx: &CallContext, _: &J| {
            let out: String = (0..110)
                .map(|i| {
                    format!(
                        "test suite::case_{:04}_{i:03} ... ok ({} ms)\n",
                        ctx.call % 10000,
                        (ctx.call as usize * 7 + i * 13) % 97
                    )
                })
                .collect();
            Ok(json!({"output": out, "exit": 0}))
        }),
    );
    // Turns to reach the context, at ~2.1k tokens a turn, then a window measured near it.
    let window = 6;
    let turns = (context_k * 1000).div_ceil(2100) as u64 + window;
    for i in 0..agents {
        let id = format!("agent-{i:05}");
        let mut spec = InstanceSpec::new(
            &id,
            json!({"id": id, "n_agents": agents, "task": "Keep the build green.", "c_hi": 10_000_000, "c_lo": 5_000_000}),
        );
        spec.max_turns = Some(turns);
        spec.max_new = 4096;
        rt.add_instance(spec).unwrap();
    }
    let (rss0, anon0) = (status_kib("VmRSS:"), status_kib("RssAnon:"));
    let started = Instant::now();
    let cpu0 = cpu_s();
    // Sample turns over time: the last window's rate is the rate at the full context.
    let samples = Arc::new(std::sync::Mutex::new(vec![]));
    let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let sampler = {
        let (samples, done, control) = (samples.clone(), done.clone(), rt.control());
        std::thread::spawn(move || {
            let mut peak = (0, 0);
            while !done.load(Ordering::Relaxed) {
                // Turns so far: the run's status (refreshed every 100 ms).
                let turns: u64 = control.status()["agents"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|a| a["turns"].as_u64())
                    .sum();
                samples
                    .lock()
                    .unwrap()
                    .push((started.elapsed().as_secs_f64(), turns, cpu_s()));
                peak = (peak.0.max(status_kib("VmRSS:")), peak.1.max(status_kib("RssAnon:")));
                std::thread::sleep(Duration::from_millis(500));
            }
            peak
        })
    };
    #[cfg(feature = "profile")]
    let profiler = affordance::profile::start(1000);
    let report = rt.run().unwrap();
    done.store(true, Ordering::Relaxed);
    #[cfg(feature = "profile")]
    if let Some(profile) = profiler {
        let samples = profile.finish();
        let mut incl: std::collections::HashMap<String, usize> = Default::default();
        let mut total = 0;
        for &(ref names, n) in &samples {
            total += n;
            for name in names.iter().collect::<std::collections::HashSet<_>>() {
                *incl.entry(name.clone()).or_default() += n;
            }
        }
        let mut incl: Vec<_> = incl.into_iter().collect();
        incl.sort_by(|a, b| b.1.cmp(&a.1));
        let mut alloc: std::collections::HashMap<String, usize> = Default::default();
        for &(ref names, n) in &samples {
            if names.first().is_some_and(|l| {
                l.contains("malloc")
                    || l.contains("free")
                    || l.contains("realloc")
                    || l.contains("memmove")
                    || l.contains("memcpy")
            }) {
                let caller: Vec<String> = names
                    .iter()
                    .filter(|n| n.starts_with("affordance::"))
                    .take(2)
                    .cloned()
                    .collect();
                *alloc
                    .entry(format!(
                        "{} :: {}",
                        names[0].chars().take(20).collect::<String>(),
                        caller.join(" <- ")
                    ))
                    .or_default() += n;
            }
        }
        let mut alloc: Vec<_> = alloc.into_iter().collect();
        alloc.sort_by(|a, b| b.1.cmp(&a.1));
        for (name, n) in alloc.iter().take(40) {
            eprintln!(
                "alloc {:5.1}% {}",
                100.0 * *n as f64 / total as f64,
                name.chars().take(220).collect::<String>()
            );
        }
        for (name, n) in incl.iter().take(70) {
            eprintln!(
                "incl {:5.1}% {}",
                100.0 * *n as f64 / total as f64,
                name.chars().take(150).collect::<String>()
            );
        }
    }
    let (peak_rss, peak_anon) = sampler.join().unwrap();
    let wall = started.elapsed().as_secs_f64();
    let cpu = cpu_s() - cpu0;
    let total: u64 = report.instances.iter().map(|a| a.metrics.turns).sum();
    // The last window: requests served in the final `window` turns of every agent.
    let samples = samples.lock().unwrap().clone();
    let last_start = total.saturating_sub(window * agents as u64);
    let (t0, n0, c0) = samples
        .iter()
        .find(|s| s.1 >= last_start)
        .copied()
        .unwrap_or((0.0, 0, 0.0));
    let (t1, n1, c1) = samples.last().copied().unwrap_or((wall, total, cpu));
    // Per-turn costs from the traces of a sample of agents.
    let (mut render, mut queue, mut http) = (vec![], vec![], vec![]);
    for a in report.instances.iter().take(64) {
        for l in std::fs::read_to_string(dir.join(&a.id).join("trace.jsonl"))
            .unwrap_or_default()
            .lines()
        {
            let Ok(v) = serde_json::from_str::<J>(l) else { continue };
            if v["t"] == "render"
                && v["prompt_tokens"].as_u64().unwrap_or(0) >= (context_k as u64 * 900)
                && let Some(ms) = v["render_ms"].as_f64()
            {
                render.push(ms)
            }
            if v["t"] == "generate" && v["prompt_tokens"].as_u64().unwrap_or(0) >= (context_k as u64 * 900) {
                if let Some(ms) = v["harness_queue_ms"].as_f64() {
                    queue.push(ms)
                }
                if let Some(ms) = v["ms"].as_f64() {
                    http.push(ms)
                }
            }
        }
    }
    let r2 = |x: f64| (x * 100.0).round() / 100.0;
    drop(server.stdin.take());
    let mut counts = String::new();
    server_out.read_line(&mut counts).unwrap();
    let _ = server.wait();
    let _ = std::fs::remove_file(&response_file);
    let c: Vec<u64> = counts.split_whitespace().filter_map(|x| x.parse().ok()).collect();
    let request_kib = c.get(1).copied().unwrap_or(0) / c.first().copied().unwrap_or(1).max(1) / 1024;
    println!(
        "{}",
        json!({
            "agents": agents, "context_k": context_k, "workers": workers, "turns": total, "errors": report.instances.iter().filter(|a| a.error.is_some()).count(),
            "wall_s": r2(wall), "turns_per_s": r2(total as f64 / wall), "turns_per_s_at_context": r2((n1 - n0) as f64 / (t1 - t0).max(1e-9)),
            "cores": r2(cpu / wall), "cores_at_context": r2((c1 - c0) / (t1 - t0).max(1e-9)), "cpu_ms_per_turn": r2(cpu * 1000.0 / total as f64),
            "cpu_ms_per_turn_at_context": r2((c1 - c0) * 1000.0 / ((n1 - n0).max(1)) as f64),
            "render_ms_median": r2(pct(&mut render, 0.5)), "render_ms_p99": r2(pct(&mut render, 0.99)),
            "queue_ms_median": r2(pct(&mut queue, 0.5)), "http_ms_median": r2(pct(&mut http, 0.5)), "http_ms_p99": r2(pct(&mut http, 0.99)),
            "request_kib_mean": request_kib,
            "peak_rss_mib": peak_rss / 1024, "rss_mib_per_agent": r2((peak_rss.saturating_sub(rss0)) as f64 / 1024.0 / agents as f64),
            "anon_mib_per_agent": r2((peak_anon.saturating_sub(anon0)) as f64 / 1024.0 / agents as f64),
        })
    );
    let _ = std::fs::remove_dir_all(&dir);
}
