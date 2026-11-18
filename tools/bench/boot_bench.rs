//! How long an agent takes to come up: `views/agents/session.tsx` on a template app, N agents
//! created and started on one thread (`Agent::new` and `Agent::start`: the first render and its
//! effects), after the view is loaded once. `boot_bench [template] [agents] [threads]` (with
//! threads, each loads its own view and boots its share, as the host's workers do);
//! `AFFORDANCE_FLAMEGRAPH=out.svg` with `--features profile`.
use affordance::View;
use affordance::emit::Encoder;
use affordance::runtime::{Agent, AgentConfig};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let template = args.get(1).cloned().unwrap_or_else(|| "deepseek_v41".into());
    let n: usize = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(512);
    let threads: usize = args.get(3).and_then(|a| a.parse().ok()).unwrap_or(1);
    // As the agents binary does (`host::tune_allocator`), unless the environment sets it.
    if std::env::var_os("MALLOC_TOP_PAD_").is_none() {
        affordance::runtime::host::pad_heaps();
    }
    if threads > 1 {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let model = match template.as_str() {
            "glm53" => "zai-org__GLM-5.3",
            _ => "deepseek-ai__DeepSeek-V4.1-Flash",
        };
        let enc = std::sync::Arc::new(
            Encoder::from_file(root.join(format!("ref/tokenizers/{model}.json")).to_str().unwrap()).unwrap(),
        );
        let started = Instant::now();
        let hs: Vec<_> = (0..threads)
            .map(|w| {
                let (enc, template, root) = (enc.clone(), template.clone(), root.clone());
                std::thread::spawn(move || {
                    let view = View::load(&root.join("views/agents/session.tsx"), "Main", &enc).unwrap();
                    let t = Instant::now();
                    let mut agents = vec![];
                    for i in (w..n).step_by(threads) {
                        let id = format!("agent-{i:04}");
                        let cfg = AgentConfig {
                            context: None,
                            retry: Default::default(),
                            template: Some(
                                affordance::runtime::model::ModelTemplate::new(
                                    template.clone(),
                                    json!({"enable_thinking": true}),
                                )
                                .unwrap(),
                            ),
                            id: id.clone(),
                            props: json!({"id": id, "n_agents": n, "task": "Keep the build green."}),
                            env: BTreeMap::new(),
                            max_new: 4096,
                            max_turns: None,
                            max_prompt: None,
                            start_ms: 1_791_000_000_000,
                            seed: i as u64,
                            machines: vec!["home".into()],
                            model: Some("default".into()),
                        };
                        let mut a = Agent::new(cfg).unwrap();
                        a.start(&view, &enc).unwrap();
                        agents.push(a);
                    }
                    (agents.len(), t.elapsed().as_secs_f64() * 1000.0)
                })
            })
            .collect();
        let per: Vec<(usize, f64)> = hs.into_iter().map(|h| h.join().unwrap()).collect();
        let worst = per.iter().map(|p| p.1).fold(0.0, f64::max);
        println!(
            "{}",
            json!({"template": template, "agents": n, "threads": threads, "wall_ms": (started.elapsed().as_secs_f64() * 1000.0).round(), "slowest_thread_ms": worst.round(),
            "ms_per_agent_in_a_thread": ((worst / per[0].0.max(1) as f64) * 1000.0).round() / 1000.0})
        );
        return;
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let model = match template.as_str() {
        "glm53" => "zai-org__GLM-5.3",
        _ => "deepseek-ai__DeepSeek-V4.1-Flash",
    };
    let t = Instant::now();
    let enc = Encoder::from_file(root.join(format!("ref/tokenizers/{model}.json")).to_str().unwrap()).unwrap();
    let enc_ms = t.elapsed().as_secs_f64() * 1000.0;
    let t = Instant::now();
    let view = View::load(&root.join("views/agents/session.tsx"), "Main", &enc).unwrap();
    let view_ms = t.elapsed().as_secs_f64() * 1000.0;
    #[cfg(feature = "profile")]
    let profiler = affordance::profile::start(5000);
    let mut times = vec![];
    let started = Instant::now();
    let mut agents = vec![];
    for i in 0..n {
        let id = format!("agent-{i:04}");
        let props = json!({"id": id, "n_agents": n, "task": "Keep the build green."});
        let cfg = AgentConfig {
            context: None,
            retry: Default::default(),
            template: Some(
                affordance::runtime::model::ModelTemplate::new(template.clone(), json!({"enable_thinking": true}))
                    .unwrap(),
            ),
            id: id.clone(),
            props,
            env: BTreeMap::new(),
            max_new: 4096,
            max_turns: None,
            max_prompt: None,
            start_ms: 1_791_000_000_000,
            seed: i as u64,
            machines: vec!["home".into()],
            model: Some("default".into()),
        };
        let t = Instant::now();
        let mut a = Agent::new(cfg).unwrap();
        let cmds = a.start(&view, &enc).unwrap();
        times.push(t.elapsed().as_secs_f64() * 1000.0);
        assert!(!cmds.is_empty());
        agents.push(a);
    }
    let total = started.elapsed().as_secs_f64() * 1000.0;
    #[cfg(feature = "profile")]
    if let Some(profile) = profiler {
        let samples = profile.finish();
        let mut incl: std::collections::HashMap<String, usize> = Default::default();
        let mut tot = 0;
        for &(ref names, c) in &samples {
            tot += c;
            for name in names.iter().collect::<std::collections::HashSet<_>>() {
                *incl.entry(name.clone()).or_default() += c;
            }
        }
        let mut incl: Vec<_> = incl
            .into_iter()
            .filter(|(k, _)| k.contains("affordance") || k.contains("tokenizers") || k.contains("serde"))
            .collect();
        incl.sort_by(|a, b| b.1.cmp(&a.1));
        for (name, c) in incl.iter().take(40) {
            eprintln!(
                "incl {:5.1}% {}",
                100.0 * *c as f64 / tot as f64,
                name.chars().take(140).collect::<String>()
            );
        }
    }
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!(
        "{}",
        json!({"template": template, "agents": n, "encoder_ms": enc_ms.round(), "view_load_ms": view_ms.round(),
        "boot_ms_total": total.round(), "boot_ms_median": (times[n / 2] * 1000.0).round() / 1000.0, "boot_ms_first": (times[0] * 1000.0).round() / 1000.0, "boot_ms_p99": (times[n * 99 / 100] * 1000.0).round() / 1000.0})
    );
}
