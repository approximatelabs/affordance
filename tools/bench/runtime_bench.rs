//! The harness runtime's own cost: N agents of `views/agents/session.tsx` for T turns each,
//! through the real host (workers, journals, model gate, blob store), with an instant scripted
//! model and an in-process machine, so only the harness is measured.
//!
//! `runtime_bench <agents> <turns> [workers] [output_kib]` prints one JSON line:
//! turns per second, render time per turn (mean and p95, from the traces), prompt size, prefix
//! reuse, peak and per-agent memory (at the peak, and anonymous memory while running: what a
//! cgroup cannot reclaim). With `AFFORDANCE_BENCH_REPLAY=1` it then restores the run's directory (every agent has
//! stopped, so the restore is a replay of every journal) and reports the replay's time and memory.
use affordance::emit::Encoder;
use affordance::runtime::{CallContext, FnModel, GenReply, GenRequest, InstanceSpec, Runtime, RuntimeConfig};
use serde_json::{Value as J, json};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

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

fn call(name: &str, args: &[(&str, &str)]) -> String {
    let a: String = args
        .iter()
        .map(|(k, v)| format!("<arg_key>{k}</arg_key><arg_value>{v}</arg_value>"))
        .collect();
    format!("Next step: look at the output and decide.</think><tool_call>{name}{a}</tool_call>")
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let arg = |i: usize, d: usize| args.get(i).and_then(|s| s.parse().ok()).unwrap_or(d);
    let (agents, turns, workers, output_kib) = (arg(1, 64), arg(2, 50), arg(3, 8), arg(4, 2));
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let enc =
        Arc::new(Encoder::from_file(root.join("ref/tokenizers/zai-org__GLM-5.3.json").to_str().unwrap()).unwrap());
    let dir = std::env::temp_dir().join(format!("aff-runtime-bench-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let config = || {
        let mut cfg = RuntimeConfig::new(root.join("views/agents/session.tsx"));
        cfg.workers = workers;
        cfg.dir = Some(dir.clone());
        cfg.model_threads = 256;
        cfg.exec_threads = 256;
        cfg
    };
    let e = enc.clone();
    let model = Arc::new(FnModel(move |req: &GenRequest| {
        let n = req
            .ids
            .iter()
            .filter(|&&t| t == e.token_id("<|assistant|>").unwrap())
            .count();
        let text = match n % 10 {
            3 => call("set_notes", &[("text", &format!("progress at turn {n}"))]),
            7 => call("set_todo", &[("items", "[x] build\nrun tests\nreview")]),
            _ => call("shell", &[("command", &format!("make -j8 test-{n}"))]),
        };
        let mut ids = e.tok.encode(text.as_str(), false).unwrap().get_ids().to_vec();
        ids.push(e.token_id("<|observation|>").unwrap());
        Ok(GenReply {
            ids,
            cut_off: false,
            ..Default::default()
        })
    }));
    let model = affordance::runtime::model::with_template(
        model,
        affordance::runtime::model::ModelTemplate::new("glm53", json!({})).unwrap(),
    )
    .unwrap();
    affordance::runtime::host::tune_allocator(config().workers); // as the agents binary does
    let mut rt = Runtime::new(config(), enc.clone(), model.clone());
    let line = "test store::case_0042 ... ok (13 ms)\n";
    let out = line.repeat(output_kib * 1024 / line.len());
    let machine = move |_: &CallContext, _: &J| Ok(json!({"output": out, "exit": 0}));
    rt.bind_machine("home", Arc::new(machine.clone()));
    for i in 0..agents {
        let mut spec = InstanceSpec::new(
            &format!("agent-{i:04}"),
            json!({"id": format!("agent-{i:04}"), "n_agents": agents, "task": "Keep the build green."}),
        );
        spec.max_turns = Some(turns as u64);
        rt.add_instance(spec).unwrap();
    }
    let rss0 = status_kib("VmRSS:");
    let anon0 = status_kib("RssAnon:");
    let t0 = Instant::now();
    // Resident memory sampled while the run is in progress (the report built at the end holds
    // every transcript as JSON, so the peak overstates what the run itself keeps).
    let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let sampler = {
        let done = done.clone();
        std::thread::spawn(move || {
            let mut samples = vec![];
            while !done.load(std::sync::atomic::Ordering::Relaxed) {
                samples.push(status_kib("RssAnon:"));
                std::thread::sleep(std::time::Duration::from_millis(250));
            }
            samples
        })
    };
    let report = rt.run().unwrap();
    let wall = t0.elapsed().as_secs_f64();
    done.store(true, std::sync::atomic::Ordering::Relaxed);
    let samples = sampler.join().unwrap();
    // The last sample of the run proper: before the final 2% of wall time, when reports are built.
    let running = samples.get(samples.len() * 98 / 100).copied().unwrap_or(0);
    let total: u64 = report.instances.iter().map(|a| a.metrics.turns).sum();
    let prompt: u64 = report.instances.iter().map(|a| a.metrics.prompt_tokens).sum();
    let uncached: u64 = report.instances.iter().map(|a| a.metrics.uncached_tokens).sum();
    let max_prompt = report.instances.iter().map(|a| a.metrics.max_prompt).max().unwrap_or(0);
    let errors = report.instances.iter().filter(|a| a.error.is_some()).count();
    let mut render_ms: Vec<f64> = vec![];
    for a in &report.instances {
        for l in std::fs::read_to_string(dir.join(&a.id).join("trace.jsonl"))
            .unwrap_or_default()
            .lines()
        {
            if let Ok(v) = serde_json::from_str::<J>(l)
                && v["t"] == "render"
                && let Some(ms) = v["render_ms"].as_f64()
            {
                render_ms.push(ms);
            }
        }
    }
    render_ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mean = render_ms.iter().sum::<f64>() / render_ms.len().max(1) as f64;
    let p95 = render_ms.get(render_ms.len() * 95 / 100).copied().unwrap_or(0.0);
    let peak = status_kib("VmHWM:");
    println!(
        "{}",
        json!({
            "agents": agents, "turns_each": turns, "workers": workers, "tool_output_kib": output_kib,
            "turns": total, "errors": errors, "wall_s": (wall * 100.0).round() / 100.0,
            "turns_per_s": (total as f64 / wall).round(),
            "render_ms_mean": (mean * 100.0).round() / 100.0, "render_ms_p95": (p95 * 100.0).round() / 100.0,
            "max_prompt_tokens": max_prompt, "prefix_reuse": ((1.0 - uncached as f64 / prompt.max(1) as f64) * 1000.0).round() / 1000.0,
            "rss_start_mib": rss0 / 1024, "peak_rss_mib": peak / 1024,
            "mib_per_agent": (((peak.saturating_sub(rss0)) as f64 / 1024.0 / agents as f64) * 100.0).round() / 100.0,
            "log_records_mean": report.instances.iter().map(|a| a.state["__ui"]["@log"]["records"].as_array().map_or(0, |r| r.len())).sum::<usize>() / agents.max(1),
            "running_anon_mib_per_agent": (((running.saturating_sub(anon0)) as f64 / 1024.0 / agents as f64) * 100.0).round() / 100.0,
        })
    );
    if std::env::var_os("AFFORDANCE_BENCH_REPLAY").is_some() {
        drop(report);
        let anon = status_kib("RssAnon:");
        let cfg = config();
        let t0 = Instant::now();
        let mut rt = Runtime::restore(cfg, enc, model).unwrap();
        rt.bind_machine("home", Arc::new(machine));
        let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let sampler = {
            let done = done.clone();
            std::thread::spawn(move || {
                let mut most = 0;
                while !done.load(std::sync::atomic::Ordering::Relaxed) {
                    most = most.max(status_kib("RssAnon:"));
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                most
            })
        };
        let replayed = rt.run().unwrap();
        done.store(true, std::sync::atomic::Ordering::Relaxed);
        let most = sampler.join().unwrap();
        println!(
            "{}",
            json!({
                "replay_agents": replayed.instances.len(), "replay_s": (t0.elapsed().as_secs_f64() * 100.0).round() / 100.0,
                "replay_errors": replayed.instances.iter().filter(|a| a.error.is_some() || a.diverged_at.is_some()).count(),
                "replay_peak_anon_mib_per_agent": ((most.saturating_sub(anon) as f64 / 1024.0 / agents as f64) * 100.0).round() / 100.0,
            })
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}
