//! Process memory with N render threads.
//!
//! `render_threads <shared|per-thread> <threads> [turns] [renders]`
//! - `per-thread`: every thread loads its own `Encoder` and `View` (the consumer's setup before).
//! - `shared`: one `Encoder` loaded once, shared as `Arc<Encoder>`; each thread binds its own `View`.
//!
//! Each thread re-renders a GLM-5.3 session (views/demo/consumer/glm53.tsx, `turns` model/tool
//! turns of text records) `renders` times with its own `RunCache`, then all threads park while
//! RSS is read from /proc/self/status. Prints one JSON line.
use affordance::View;
use affordance::emit::{Encoder, RunCache};
use serde_json::{Value as J, json};
use std::path::PathBuf;
use std::sync::{Arc, Barrier};
use std::time::Instant;

fn status_kib(key: &str) -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .unwrap()
        .lines()
        .find(|l| l.starts_with(key))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

fn session(turns: usize) -> J {
    let mut log = vec![json!({"kind": "input", "content": "Task: keep /srv/kv healthy."})];
    for t in 0..turns {
        log.push(json!({"kind": "screen", "content": format!("agent-0001 | unread: {}\n[shell][send_message] [set_notes]\n", t % 3)}));
        log.push(json!({"kind": "model", "content": "", "reasoning": format!("Turn {t}: {}", "inspect the build output and decide the next command. ".repeat(6)),
                        "calls": [{"type": "function", "function": {"name": "shell", "arguments": {"command": format!("make -j8 test-{t}")}}}]}));
        log.push(json!({"kind": "action", "name": "shell", "result": format!("{}[exit 0]", format!("test-{t}: segment ok, checksum ok, 1234 bytes\n").repeat(12))}));
    }
    json!({"log": log, "handle": "agent-0001"})
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(String::as_str).unwrap_or("shared");
    let threads: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);
    let turns: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(100);
    let renders: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(20);
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let tokenizer = root.join("ref/tokenizers/zai-org__GLM-5.3.json").display().to_string();
    let view_path = root.join("views/demo/consumer/glm53.tsx");
    let state = session(turns);
    let rss_start = status_kib("VmRSS:");
    let shared = (mode == "shared").then(|| Arc::new(Encoder::from_file(&tokenizer).unwrap()));
    let rss_encoder = status_kib("VmRSS:");
    let barrier = Arc::new(Barrier::new(threads + 1));
    let t0 = Instant::now();
    let handles: Vec<_> = (0..threads)
        .map(|_| {
            let (shared, barrier, tokenizer, view_path, state) = (
                shared.clone(),
                barrier.clone(),
                tokenizer.clone(),
                view_path.clone(),
                state.clone(),
            );
            std::thread::spawn(move || {
                let enc = shared.unwrap_or_else(|| Arc::new(Encoder::from_file(&tokenizer).unwrap()));
                let view = View::load(&view_path, "Main", &enc).unwrap();
                let mut cache = RunCache::new(true);
                let m = affordance::value::state(&state);
                let mut ms = vec![];
                let mut len = 0;
                for _ in 0..renders {
                    let t = Instant::now();
                    len = view
                        .render(
                            &enc,
                            m.clone(),
                            &mut cache,
                            chrono::NaiveDate::from_ymd_opt(2026, 10, 13)
                                .unwrap()
                                .and_hms_opt(12, 0, 0)
                                .unwrap(),
                        )
                        .unwrap()
                        .ids
                        .len();
                    ms.push(t.elapsed().as_secs_f64() * 1000.0);
                }
                barrier.wait(); // all threads alive, views and caches held: main reads RSS
                barrier.wait();
                (len, ms)
            })
        })
        .collect();
    barrier.wait();
    let rss_all = status_kib("VmRSS:");
    let hwm = status_kib("VmHWM:");
    barrier.wait();
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    let mut ms: Vec<f64> = results.iter().flat_map(|r| r.1.iter().skip(1).copied()).collect();
    ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!(
        "{}",
        json!({
            "mode": mode, "threads": threads, "prompt_tokens": results[0].0, "renders_per_thread": renders,
            "rss_mib": rss_all as f64 / 1024.0, "peak_rss_mib": hwm as f64 / 1024.0,
            "rss_before_load_mib": rss_start as f64 / 1024.0, "rss_after_shared_encoder_mib": rss_encoder as f64 / 1024.0,
            "warm_render_ms_p50": ms.get(ms.len() / 2), "warm_render_ms_max": ms.last(), "wall_s": t0.elapsed().as_secs_f64(),
        })
    );
}
