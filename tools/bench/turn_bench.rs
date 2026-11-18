//! Per-turn cost of one agent at large contexts: `views/agents/session.tsx` on a template app
//! (default DeepSeek V4.1, the native ids path), each turn appending a native tool call (with
//! reasoning) and a ~1.4k-token tool result, until the context reaches the largest target. At each
//! target it reports, over the next turns: the agent's own time per turn (the tool handler, the
//! render of the next prompt, tokenizing what is new), the SGLang request body (JSON-encoding the
//! prompt's input_ids), and the bytes each costs.
//!
//! `turn_bench [template] [targets_k] [fold|msg]`, e.g. `turn_bench deepseek_v41 24,100,200`. With
//! `fold`, the session folds at the (single) target (c_hi = target, c_lo = half): it reports the
//! fold turn (the overflow handler, its `measure` probes and the summary render) against a normal
//! turn, and the turn after it. With `msg`, a message from another agent arrives while each
//! model call is out, and each row also reports the agent's time for it (`event_ms_median`).
use affordance::View;
use affordance::emit::Encoder;
use affordance::runtime::{Agent, AgentConfig, Command, Input};
use serde_json::json;
use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;
use std::time::Instant;

fn pct(v: &mut [f64], p: f64) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[((v.len() - 1) as f64 * p).round() as usize]
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let template = args.get(1).cloned().unwrap_or_else(|| "deepseek_v41".into());
    let targets: Vec<usize> = args
        .get(2)
        .map(|s| s.split(',').map(|x| x.parse::<usize>().unwrap() * 1000).collect())
        .unwrap_or(vec![24_000, 100_000, 200_000]);
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let model = match template.as_str() {
        "glm53" => "zai-org__GLM-5.3",
        _ => "deepseek-ai__DeepSeek-V4.1-Flash",
    };
    let enc = Encoder::from_file(root.join(format!("ref/tokenizers/{model}.json")).to_str().unwrap()).unwrap();
    let view = View::load(&root.join("views/agents/session.tsx"), "Main", &enc).unwrap();
    let fold = args.get(3).is_some_and(|a| a == "fold");
    let msg = args.get(3).is_some_and(|a| a == "msg");
    let mut event_ms: Vec<f64> = vec![];
    let (c_hi, c_lo) = if fold {
        (targets[0], targets[0] / 2)
    } else {
        (10_000_000, 5_000_000)
    };
    let props = json!({"id": "agent-0001", "n_agents": 1, "task": "Keep the build green.", "c_hi": c_hi, "c_lo": c_lo});
    let cfg = AgentConfig {
        context: None,
        retry: Default::default(),
        template: Some(
            affordance::runtime::model::ModelTemplate::new(template.clone(), json!({"enable_thinking": true})).unwrap(),
        ),
        id: "agent-0001".into(),
        props,
        env: BTreeMap::new(),
        max_new: 4096,
        max_turns: None,
        max_prompt: None,
        start_ms: 1_791_000_000_000,
        seed: 7,
        machines: vec!["home".into()],
        model: Some("default".into()),
    };
    let mut agent = Agent::new(cfg).unwrap();
    let eos = enc
        .token_id("<｜end▁of▁sentence｜>")
        .or_else(|| enc.token_id("<|observation|>"))
        .unwrap();
    let reasoning: String = (0..40)
        .map(|i| format!("Step {i}: check the build output for regressions and compare timings. "))
        .collect();
    let reply = |n: usize| -> Vec<u32> {
        let call = if template == "glm53" {
            format!(
                "{reasoning} Turn {n}.</think><tool_call>shell<arg_key>command</arg_key><arg_value>make -j8 test-{n}</arg_value></tool_call>"
            )
        } else {
            format!(
                "{reasoning} Turn {n}.</think>\n\n<｜DSML｜ calls>\n<｜DSML｜ invoke name=\"shell\">\n<｜DSML｜ parameter name=\"command\" string=\"true\">make -j8 test-{n}</｜DSML｜ parameter>\n</｜DSML｜ invoke>\n</｜DSML｜ calls>"
            )
        };
        let mut ids = enc.tok.encode(call.as_str(), false).unwrap().get_ids().to_vec();
        ids.push(eos);
        ids
    };
    // A distinct tool output each turn (~1.4k tokens, under the session app's inline cap).
    let output = |n: usize| -> String {
        (0..110)
            .map(|i| format!("test suite::case_{n:04}_{i:03} ... ok ({} ms)\n", (n * 7 + i * 13) % 97))
            .collect()
    };
    let mut queue: VecDeque<Command> = agent.start(&view, &enc).unwrap().into();
    let mut clock = 1_791_000_000_000i64;
    let (mut turn, mut target_i) = (0usize, 0usize);
    // (target, render ms, input ms, prompt tokens, cached tokens, turn ms) while measuring a target.
    type Measuring = (usize, Vec<f64>, Vec<f64>, Vec<usize>, Vec<usize>, Vec<f64>);
    let mut measuring: Option<Measuring> = None;
    let mut last_render_ms = 0.0;
    let mut last_input_ms = 0.0;
    let mut rows = vec![];
    let started = Instant::now();
    // Fold mode: the agent's time between prompts (a fold turn includes its summary render).
    let (mut since_prompt_ms, mut fold_rows, mut prev_ctx, mut summaries) = (0.0f64, vec![], 0usize, 0usize);
    // Exact append: every prompt extends the last one and its reply, except right after a fold.
    let (mut last_full, mut appends, mut folded) = (Vec::<u32>::new(), 0usize, 0usize);
    let mut check = |ids: &[u32], reply: &[u32]| {
        if ids.len() >= last_full.len() {
            assert!(
                ids.starts_with(&last_full),
                "turn prompt is not an exact append of the last prompt and its reply"
            );
            appends += 1;
        } else {
            folded += 1;
        }
        last_full = [ids, reply].concat();
    };
    #[cfg(feature = "profile")]
    let profiler = affordance::profile::start(
        std::env::var("AFFORDANCE_FLAMEGRAPH_HZ")
            .ok()
            .and_then(|h| h.parse().ok())
            .unwrap_or(2000),
    );
    // AFFORDANCE_PROMPT_HASHES=1: print every prompt's hash (to compare two builds byte for byte).
    let dump = std::env::var("AFFORDANCE_PROMPT_HASHES").is_ok();
    while let Some(cmd) = queue.pop_front() {
        clock += 10;
        if let (true, Command::Generate { ids, .. }) = (dump, &cmd) {
            println!("prompt {}", affordance::runtime::agent::prompt_hash(ids));
        }
        let input = match cmd {
            // A summary render (the fold): answered with a short summary, not counted as a turn.
            Command::Generate { task, ids, opts, .. } if opts.tag.as_deref() == Some("summary") => {
                summaries += 1;
                let _ = ids;
                let mut out = enc
                    .tok
                    .encode("Summary.</think>Build green; test_upload_retry flaky.", false)
                    .unwrap()
                    .get_ids()
                    .to_vec();
                out.push(eos);
                Input::Reply {
                    server: None,
                    task,
                    ids: out,
                    cut_off: false,
                    clock_ms: clock,
                    prompt: None,
                    aborted: false,
                }
            }
            Command::Generate { task, ids, .. } if fold => {
                turn += 1;
                // A turn whose prompt is shorter than the last one followed a fold.
                if prev_ctx > 0 {
                    fold_rows.push((prev_ctx, ids.len(), since_prompt_ms, summaries));
                }
                prev_ctx = ids.len();
                since_prompt_ms = 0.0;
                if turn > 200 {
                    break;
                }
                let out = reply(turn);
                check(&ids, &out);
                Input::Reply {
                    server: None,
                    task,
                    ids: out,
                    cut_off: false,
                    clock_ms: clock,
                    prompt: None,
                    aborted: false,
                }
            }
            Command::Generate { task, ids, .. } => {
                turn += 1;
                // The request body the native client sends (sglang /generate).
                let t0 = Instant::now();
                let body = affordance::runtime::http::ids_body(
                    "input_ids",
                    &ids,
                    &json!({"sampling_params": {"max_new_tokens": 4096, "stop_token_ids": [eos]}, "stream": true, "rid": "bench"}),
                );
                let body_ms = t0.elapsed().as_secs_f64() * 1000.0;
                if let Some((_, turn_ms, enc_ms, bytes, ctx, render_ms)) = measuring.as_mut() {
                    turn_ms.push(last_input_ms);
                    render_ms.push(last_render_ms);
                    enc_ms.push(body_ms);
                    bytes.push(body.len());
                    ctx.push(ids.len());
                    if turn_ms.len() == 1 && std::env::var("AFFORDANCE_TSX_PROFILE").is_ok() {
                        affordance::profile::tsx_start();
                    }
                    if turn_ms.len() == 12 {
                        if std::env::var("AFFORDANCE_TSX_PROFILE").is_ok() {
                            for (f, n, ms) in affordance::profile::tsx_take().into_iter().take(25) {
                                eprintln!("tsx {:8.2} ms/turn {:6} calls/turn  {f}", ms / 11.0, n / 11);
                            }
                        }
                        let (target, mut t, mut e, b, c, mut r) = measuring.take().unwrap();
                        rows.push(json!({"target_tokens": target, "context_tokens": c[0], "appended_per_turn": (c[c.len() - 1] - c[0]) / (c.len() - 1),
                            "turn_ms_median": (pct(&mut t, 0.5) * 100.0).round() / 100.0, "render_ms_median": (pct(&mut r, 0.5) * 100.0).round() / 100.0, "turn_ms_p99": (pct(&mut t, 0.99) * 100.0).round() / 100.0,
                            "body_json_ms_median": (pct(&mut e, 0.5) * 100.0).round() / 100.0, "body_json_kib": b[0] / 1024,
                            "prompt_ids_kib": c[0] * 4 / 1024}));
                        if msg {
                            rows.last_mut().unwrap()["event_ms_median"] =
                                json!((pct(&mut event_ms, 0.5) * 100.0).round() / 100.0);
                            event_ms.clear();
                        }
                        eprintln!("{}", rows.last().unwrap());
                        target_i += 1;
                        if target_i == targets.len() {
                            break;
                        }
                    }
                } else if ids.len() >= targets[target_i] {
                    measuring = Some((targets[target_i], vec![], vec![], vec![], vec![], vec![]));
                }
                if msg {
                    let t0 = Instant::now();
                    let more = agent
                        .handle(
                            &view,
                            &enc,
                            Input::Event {
                                name: "message".into(),
                                payload: json!({"from": "agent-0002", "text": format!("status {turn}?")}),
                                op: None,
                                clock_ms: clock,
                            },
                        )
                        .unwrap();
                    if measuring.is_some() {
                        event_ms.push(t0.elapsed().as_secs_f64() * 1000.0);
                    }
                    queue.extend(more);
                }
                let out = reply(turn);
                check(&ids, &out);
                Input::Reply {
                    server: None,
                    task,
                    ids: out,
                    cut_off: false,
                    clock_ms: clock,
                    prompt: None,
                    aborted: false,
                }
            }
            Command::Call { task, .. } => Input::Result {
                task,
                outcome: affordance::effects::Outcome::Ok(json!({"output": output(turn), "exit": 0})),
                clock_ms: clock,
            },
            Command::Timer { at_ms } => {
                clock = clock.max(at_ms);
                Input::Tick { clock_ms: clock }
            }
            _ => continue,
        };
        let t0 = Instant::now();
        let out = agent.handle(&view, &enc, input).unwrap();
        // The agent's time for the input that leads to the next prompt (the result, then a render).
        last_input_ms = t0.elapsed().as_secs_f64() * 1000.0;
        since_prompt_ms += last_input_ms;
        // The render of the prompt (the agent's trace), apart from the rest of its turn.
        last_render_ms = agent
            .take_trace()
            .iter()
            .filter(|e| e["t"] == "render")
            .filter_map(|e| e["render_ms"].as_f64())
            .sum();
        queue.extend(out);
    }
    #[cfg(feature = "profile")]
    if let Some(profile) = profiler {
        let samples = profile.finish();
        // The hottest frames, by self samples: a text summary next to the flame graph.
        let mut own: std::collections::HashMap<String, usize> = Default::default();
        for &(ref names, n) in &samples {
            if let Some(f) = names.first() {
                *own.entry(f.clone()).or_default() += n;
            }
        }
        let mut own: Vec<_> = own.into_iter().collect();
        own.sort_by(|a, b| b.1.cmp(&a.1));
        let total: usize = own.iter().map(|x| x.1).sum();
        for (name, n) in own.iter().take(25) {
            eprintln!(
                "self {:5.1}% {}",
                100.0 * *n as f64 / total as f64,
                name.chars().take(140).collect::<String>()
            );
        }
        // Short stacks (a leaf and its nearest named callers) by samples: what allocates and why.
        let mut stacks: std::collections::HashMap<String, usize> = Default::default();
        for &(ref names, n) in &samples {
            let names: Vec<&str> = names
                .iter()
                .map(String::as_str)
                .filter(|n| {
                    !n.starts_with("core::") && !n.starts_with("alloc::") && !n.contains("drop_in_place")
                        || n.contains("drop_in_place")
                })
                .take(3)
                .collect();
            *stacks.entry(names.join(" <- ")).or_default() += n;
        }
        let mut stacks: Vec<_> = stacks.into_iter().collect();
        stacks.sort_by(|a, b| b.1.cmp(&a.1));
        for (name, n) in stacks.iter().take(30) {
            eprintln!(
                "stack {:5.1}% {}",
                100.0 * *n as f64 / total as f64,
                name.chars().take(230).collect::<String>()
            );
        }
        // Allocation: the first affordance frame above each malloc/free/realloc.
        let mut alloc: std::collections::HashMap<String, usize> = Default::default();
        for &(ref names, n) in &samples {
            if names.first().is_some_and(|l| {
                l.contains("malloc")
                    || l.contains("free")
                    || l.contains("realloc")
                    || l.contains("drop_in_place")
                    || l.contains("drop_slow")
            }) {
                let caller = names
                    .iter()
                    .find(|n| n.starts_with("affordance::"))
                    .cloned()
                    .unwrap_or_default();
                *alloc.entry(caller).or_default() += n;
            }
        }
        let mut alloc: Vec<_> = alloc.into_iter().collect();
        alloc.sort_by(|a, b| b.1.cmp(&a.1));
        for (name, n) in alloc.iter().take(20) {
            eprintln!("alloc {:5.1}% {}", 100.0 * *n as f64 / total as f64, name);
        }
        // Inclusive: samples with the function anywhere on the stack.
        let mut incl: std::collections::HashMap<String, usize> = Default::default();
        for &(ref names, n) in &samples {
            for name in names.iter().collect::<std::collections::HashSet<_>>() {
                *incl.entry(name.clone()).or_default() += n;
            }
        }
        let mut incl: Vec<_> = incl
            .into_iter()
            .filter(|(k, _)| k.contains("affordance") || k.contains("serde_json") || k.contains("tokenizers"))
            .collect();
        incl.sort_by(|a, b| b.1.cmp(&a.1));
        for (name, n) in incl.iter().take(45) {
            eprintln!(
                "incl {:5.1}% {}",
                100.0 * *n as f64 / total as f64,
                name.chars().take(150).collect::<String>()
            );
        }
    }
    if fold {
        // The fold turns (the prompt shrank), the median normal turn near the fold, and the turn after.
        let folds: Vec<usize> = (0..fold_rows.len())
            .filter(|&i| fold_rows[i].1 < fold_rows[i].0)
            .collect();
        for &i in &folds {
            let (before, after, ms, n) = fold_rows[i];
            let mut normal: Vec<f64> = fold_rows[i.saturating_sub(6)..i].iter().map(|r| r.2).collect();
            let next = fold_rows.get(i + 1).map(|r| r.2).unwrap_or(0.0);
            rows.push(json!({"fold_at_tokens": before, "after_fold_tokens": after, "fold_turn_ms": (ms * 100.0).round() / 100.0,
                "normal_turn_ms_median": (pct(&mut normal, 0.5) * 100.0).round() / 100.0, "turn_after_fold_ms": (next * 100.0).round() / 100.0, "summary_renders_so_far": n}));
        }
    }
    println!(
        "{}",
        json!({"template": template, "turns": turn, "exact_appends": appends, "folds": folded, "wall_s": (started.elapsed().as_secs_f64() * 10.0).round() / 10.0, "rows": rows})
    );
}
