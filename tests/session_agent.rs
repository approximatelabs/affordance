//! Acceptance for a long-session agent as one view file (`views/agents/session.tsx`) on the
//! generic agent core, with a long scripted scenario and these invariants:
//! - every turn is an exact append, except fold turns and branch returns;
//! - folds follow the layout event (one step per turn while a prompt passes c_hi), so the
//!   prompt stays bounded without measuring candidate renders;
//! - planted facts are recalled from folded history; read_event returns the exact bytes of an
//!   output the layout clipped;
//! - the template head never changes; every reply parses to one valid call;
//! - replaying the journal reproduces the agent.
//!
//! TURNS (default 300), C_HI, THINK_MIN / THINK_MAX override the scenario. VIEW runs
//! another agent view built on session.tsx (an app that imports it),
//! PROPS adds props (JSON).
use affordance::View;
use affordance::effects::Outcome;
use affordance::runtime::{Agent, AgentConfig, Command, Input};
use serde_json::{Value as J, json};
use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::shared;

fn env<T: std::str::FromStr>(k: &str, d: T) -> T {
    std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d)
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn range(&mut self, a: u64, b: u64) -> u64 {
        a + self.next() % (b - a + 1)
    }
    fn chance(&mut self, p: f64) -> bool {
        (self.next() % 10_000) as f64 / 10_000.0 < p
    }
}

const TASK: &str = "You maintain the key-value service in /srv/kv on your VM. Keep it building and its tests green, \
investigate failures, and answer questions from other agents about it. Keep notes of important facts.";

/// The consumer's fake VM shell, unchanged.
fn shell(cmd: &str, rng: &mut Rng) -> (String, i32) {
    let lines = |n: u64, rng: &mut Rng, f: &dyn Fn(u64, &mut Rng) -> String| {
        (0..n).map(|i| f(i, rng)).collect::<Vec<_>>().join("\n")
    };
    if cmd.contains("service.env") {
        return (
            "# kv service\nLISTEN_PORT=48213\nDATA_DIR=/srv/kv/data\nREPLICAS=3\nSEGMENT_SIZE=64MiB\nLOG_LEVEL=info"
                .into(),
            0,
        );
    }
    if cmd.contains("ls -laR") {
        return (
            lines(900, rng, &|i, r| {
                format!(
                    "-rw-r--r-- 1 kv kv {:>7} Oct 12 1{}:{:02} src/mod{}/file_{:04}.rs",
                    r.range(100, 99999),
                    i % 10,
                    i % 60,
                    i % 37,
                    i
                )
            }),
            0,
        );
    }
    if cmd.contains("make test") {
        let mut out = lines(rng.range(150, 450), rng, &|i, r| {
            format!("test store::case_{i:04} ... ok ({} ms)", r.range(1, 900))
        });
        out.push_str("\ntest store::segment_verify ... FAILED\nerror[E1427]: checksum mismatch in segment 0x3fa2 (expected 9c1e, got 4b07)\ntest result: FAILED. 1 failed");
        return (out, 1);
    }
    let n = match rng.range(0, 9) {
        0..=4 => rng.range(5, 60),
        5..=7 => rng.range(60, 300),
        _ => rng.range(300, 1200),
    };
    let fail = rng.chance(0.12);
    let mut out = lines(n, rng, &|i, r| {
        format!(
            "{i:05} {} build/obj/{:08x}.o {} bytes",
            ["CC", "LD", "AR", "GEN"][(i % 4) as usize],
            r.next() as u32,
            r.range(1, 99999)
        )
    });
    if fail {
        out.push_str(&format!(
            "\nerror: command failed: {cmd}\nnote: exit status 2 at stage {}",
            rng.range(1, 9)
        ));
    }
    (out, if fail { 2 } else { 0 })
}

fn think(rng: &mut Rng, turn: usize, lo: u64, hi: u64) -> String {
    let words = [
        "check", "segment", "checksum", "replica", "build", "log", "test", "plan", "compare", "verify", "notes", "next",
    ];
    let n = rng.range(lo, hi) as usize;
    let mut s = format!("Turn {turn}. ");
    for i in 0..n {
        s.push_str(words[(rng.next() as usize + i) % words.len()]);
        s.push(if i % 13 == 12 { '.' } else { ' ' });
    }
    // A real model sometimes leaves whitespace before </think>; exact ids keep that.
    if turn.is_multiple_of(7) {
        s.push_str("  \n");
    } else {
        s = s.trim_end().to_string() + ".";
    }
    s
}

fn call(name: &str, args: &[(&str, &str)]) -> String {
    let mut s = format!("<tool_call>{name}");
    for (k, v) in args {
        s.push_str(&format!("<arg_key>{k}</arg_key><arg_value>{v}</arg_value>"));
    }
    s + "</tool_call>"
}

#[derive(Default)]
struct Script {
    pending_q: usize,
    found_port: Option<String>,
    found_code: Option<String>,
    answers: usize,
    answers_ok: usize,
    reads: usize,
    ls_key: String,
    branch_end: Option<usize>,
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn long_session_agent_is_one_view_file_on_the_generic_runtime() {
    let turns: usize = env("TURNS", 300);
    let (tmin, tmax): (u64, u64) = (env("THINK_MIN", 250), env("THINK_MAX", 700));
    let c_hi: u64 = env("C_HI", 48_000);
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let enc = shared::tokenizer(&root.join("ref/tokenizers/zai-org__GLM-5.3.json"));
    let view_path = std::env::var("VIEW")
        .map(PathBuf::from)
        .unwrap_or_else(|_| root.join("views/agents/session.tsx"));
    let view = View::load(&view_path, "Main", &enc).unwrap();

    let mut props = json!({"id": "agent-0007", "n_agents": 512, "task": TASK, "c_hi": c_hi});
    if let Ok(extra) = std::env::var("PROPS") {
        for (k, v) in serde_json::from_str::<serde_json::Map<String, J>>(&extra).unwrap() {
            props[k] = v;
        }
    }
    let cfg = AgentConfig {
        context: None,
        retry: Default::default(),
        template: Some(affordance::runtime::model::ModelTemplate::new("glm53", json!({"keep_replies": true})).unwrap()),
        id: "agent-0007".into(),
        props,
        env: BTreeMap::new(),
        max_new: 4096,
        max_turns: Some(turns as u64),
        max_prompt: None,
        start_ms: 1_791_000_000_000,
        seed: 3,
        machines: vec!["home".into()],
        model: Some("default".into()),
    };
    let mut agent = Agent::new(cfg.clone()).unwrap();
    let user = enc.token_id("<|user|>").unwrap();
    let obs = enc.token_id("<|observation|>").unwrap();
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15 ^ 7919);
    let mut sc = Script::default();
    let mut clock = cfg.start_ms;
    let mut journal: Vec<Input> = vec![];
    let mut queue: std::collections::VecDeque<Command> = agent.start(&view, &enc).unwrap().into();
    let (mut prev_full, mut head, mut turn) = (Vec::<u32>::new(), None::<Vec<u32>>, 0usize);
    let (mut exact, mut expected_breaks, mut folds, mut summaries, mut max_prompt) = (0, 0, 0, 0, 0usize);
    let mut violations: Vec<String> = vec![];
    let mut last_epoch = 0u64;
    let mut expect_break = false;
    let mut fold_pending = false;
    let mut last_prompt: Vec<u32> = vec![];
    let mut fold_after: Vec<(usize, usize, bool)> = vec![];
    let mut render_shared: Vec<(usize, usize)> = vec![];
    while let Some(cmd) = queue.pop_front() {
        clock += 50;
        let input = match cmd {
            // A summary render (from the fold policy, on the layout event of the prompt just sent):
            // the agent's own context plus a maintenance request. It shares that prompt's cached
            // prefix, up to the prompt's generation opener.
            Command::Generate { task, ids, .. }
                if enc.tok.decode(&ids, false).unwrap().contains("[Context maintenance") =>
            {
                let shared = last_prompt.iter().zip(ids.iter()).take_while(|(a, b)| a == b).count();
                render_shared.push((shared, last_prompt.len()));
                let text = enc.tok.decode(&ids, false).unwrap();
                let tail = &text[text.rfind("[Context maintenance").unwrap()..];
                let cmds = tail.matches("[call] shell").count();
                let notes = tail
                    .split("Your notes: ")
                    .nth(1)
                    .and_then(|r| r.lines().next())
                    .unwrap_or("");
                let summary = format!(
                    "Brief.</think>1. Task: maintain /srv/kv.\n2. Commands run: {cmds}\n3. Notes then: {notes}"
                );
                let mut out = enc.tok.encode(summary.as_str(), false).unwrap().get_ids().to_vec();
                out.push(user);
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
            Command::Generate {
                task, ids, prompt_hash, ..
            } => {
                turn += 1;
                // Invariants on the prompt. Its hash (continued from the last turn's) is the full one.
                if prompt_hash.as_deref() != Some(affordance::runtime::agent::prompt_hash(&ids).as_str()) {
                    violations.push(format!("t{turn}: prompt hash"));
                }
                let h = ids[..ids.iter().position(|&x| x == user).unwrap()].to_vec();
                match &head {
                    None => head = Some(h),
                    Some(h0) if *h0 != h => violations.push(format!("t{turn}: head changed")),
                    _ => {}
                }
                let epoch = agent.state()["__ui"]
                    .as_object()
                    .and_then(|u| u.values().find_map(|v| v.get("epoch").and_then(J::as_u64)))
                    .unwrap_or(0);
                let folded = epoch > last_epoch;
                if folded {
                    folds += 1;
                    last_epoch = epoch;
                    let fold = agent.state()["__ui"]
                        .as_object()
                        .unwrap()
                        .values()
                        .find(|v| v.get("epoch").is_some())
                        .unwrap()
                        .clone();
                    let summarized = fold["last"]["summarized"] == true;
                    if summarized {
                        summaries += 1;
                    }
                    fold_after.push((turn, ids.len(), summarized));
                }
                // A fold is taken after a prompt is laid out (the layout event), so the prompt that
                // changes is the next one: a fold seen by now may break this prompt or the next.
                let fold_break = fold_pending || folded;
                fold_pending = folded;
                if !prev_full.is_empty() {
                    if ids.starts_with(&prev_full) {
                        exact += 1
                    } else if fold_break || expect_break {
                        expected_breaks += 1
                    } else {
                        let at = prev_full.iter().zip(ids.iter()).take_while(|(a, b)| a == b).count();
                        violations.push(format!(
                            "t{turn}: diverged at {at}/{}: {:?} vs {:?}",
                            prev_full.len(),
                            enc.tok
                                .decode(&prev_full[at.saturating_sub(10)..(at + 15).min(prev_full.len())], false)
                                .unwrap(),
                            enc.tok
                                .decode(&ids[at.saturating_sub(10)..(at + 15).min(ids.len())], false)
                                .unwrap()
                        ));
                    }
                }
                expect_break = false;
                max_prompt = max_prompt.max(ids.len());
                if ids.len() as u64 > c_hi + 12_000 {
                    violations.push(format!("t{turn}: prompt {} far above c_hi", ids.len()));
                }
                // The planted DMs (from outside: the test host delivers them between turns).
                if [70, 150, 230].contains(&turn) && turn < turns {
                    queue.push_back(Command::Call {
                        task: u64::MAX,
                        name: "__dm".into(),
                        args: J::Null,
                    });
                }
                // Scripted decision (the consumer's scenario).
                let last_result = affordance::scenario::log_records(&agent.state())
                    .iter()
                    .rev()
                    .find(|r| r["kind"] == "action")
                    .map(|r| r["result"].as_str().unwrap_or("").to_string())
                    .unwrap_or_default();
                if let Some(p) = last_result.find("LISTEN_PORT=") {
                    sc.found_port = Some(
                        last_result[p + 12..]
                            .chars()
                            .take_while(|c| c.is_ascii_digit())
                            .collect(),
                    );
                }
                if let Some(p) = last_result.find("error[E") {
                    sc.found_code = Some(
                        last_result[p + 6..]
                            .chars()
                            .take_while(|c| c.is_ascii_alphanumeric())
                            .collect(),
                    );
                }
                // The long listing's record: its output is clipped in the prompt (session.tss); read_event reads the rest.
                if sc.ls_key.is_empty()
                    && let Some(r) = affordance::scenario::log_records(&agent.state()).iter().find(|r| {
                        r["kind"] == "action"
                            && r["result"]
                                .as_str()
                                .is_some_and(|t| t.contains("src/mod0/file_0000.rs"))
                    })
                {
                    sc.ls_key = r["id"].as_str().unwrap().to_string();
                    STORE.with(|s| {
                        s.borrow_mut()
                            .insert("ls".into(), r["result"].as_str().unwrap().to_string())
                    });
                }
                let in_branch = sc.branch_end.is_some();
                let tool = if turn == 1 {
                    call("shell", &[("command", "cat /srv/kv/config/service.env")])
                } else if turn == 2 {
                    call(
                        "set_todo",
                        &[("items", "fix E-code test failure\nreview build warnings\nanswer peers")],
                    )
                } else if turn == 3 {
                    call("shell", &[("command", "ls -laR /srv/kv")])
                } else if turn == 4 {
                    call("shell", &[("command", "cd /srv/kv &amp;&amp; make test")])
                } else if turn == 5 {
                    call(
                        "set_notes",
                        &[(
                            "text",
                            "make test fails: segment_verify checksum mismatch. Investigate segment code.",
                        )],
                    )
                } else if sc.pending_q == 3 {
                    sc.pending_q = 2;
                    call("search_history", &[("query", "LISTEN_PORT")])
                } else if sc.pending_q == 2 {
                    sc.pending_q = 1;
                    call("search_history", &[("query", "checksum mismatch")])
                } else if sc.pending_q == 1 {
                    sc.pending_q = 0;
                    sc.answers += 1;
                    if sc.found_port.as_deref() == Some("48213") && sc.found_code.as_deref() == Some("E1427") {
                        sc.answers_ok += 1;
                    }
                    call(
                        "send_message",
                        &[
                            ("to", "agent-0042"),
                            (
                                "text",
                                &format!(
                                    "port {}, error {}",
                                    sc.found_port.clone().unwrap_or("?".into()),
                                    sc.found_code.clone().unwrap_or("?".into())
                                ),
                            ),
                        ],
                    )
                } else if (turn == 120 || turn == 260) && !sc.ls_key.is_empty() {
                    sc.reads += 1;
                    call(
                        "read_event",
                        &[("id", &sc.ls_key), ("offset", "2400"), ("limit", "3000")],
                    )
                } else if !in_branch && (turn == 40 || turn == 175) {
                    sc.branch_end = Some(turn + 13);
                    call(
                        "branch",
                        &[("description", &format!("investigate flaky segment test (turn {turn})"))],
                    )
                } else if sc.branch_end.is_some_and(|end| turn >= end) {
                    sc.branch_end = None;
                    expect_break = true;
                    call(
                        "branch_return",
                        &[(
                            "summary",
                            "Segment test flakiness comes from an unflushed write before verify; fix: fsync in segment::seal.",
                        )],
                    )
                } else if turn % 12 == 0 {
                    call(
                        "set_notes",
                        &[(
                            "text",
                            &format!("make test fails: segment_verify checksum mismatch. Progress at turn {turn}."),
                        )],
                    )
                } else if turn % 15 == 0 {
                    call(
                        "set_todo",
                        &[(
                            "items",
                            &format!(
                                "[x] reproduce failure\nfix checksum failure (turn {turn})\nreview build warnings"
                            ),
                        )],
                    )
                } else if turn % 31 == 0 {
                    call("wait", &[("seconds", "30")])
                } else {
                    call(
                        "shell",
                        &[(
                            "command",
                            if rng.chance(0.15) {
                                "cd /srv/kv &amp;&amp; make test"
                            } else {
                                "cd /srv/kv &amp;&amp; make -j8 2>&amp;1"
                            },
                        )],
                    )
                };
                let text = format!("{}</think>{tool}", think(&mut rng, turn, tmin, tmax)).replace("&amp;", "&");
                let mut out = enc.tok.encode(text.as_str(), false).unwrap().get_ids().to_vec();
                out.push(obs);
                prev_full = [&ids[..], out.as_slice()].concat();
                last_prompt = ids.to_vec();
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
            Command::Call { name, .. } if name == "__dm" => {
                sc.pending_q = 3;
                Input::Event {
                    name: "message".into(),
                    op: None,
                    payload: json!({"from": "agent-0042", "text": "Quick question: which port does your kv service listen on, and what was the error code of the checksum failure you hit early on?"}),
                    clock_ms: clock,
                }
            }
            Command::Call { task, name, args } => {
                let outcome = match name.as_str() {
                    "machine:home" => {
                        let (out, code) = shell(args["cmd"].as_str().unwrap(), &mut rng);
                        Outcome::Ok(json!({"output": out, "exit": code, "timed_out": false}))
                    }
                    "store.put" => {
                        let key = args["key"]
                            .as_str()
                            .map(str::to_string)
                            .unwrap_or_else(|| format!("b{}", turn));
                        STORE.with(|s| {
                            s.borrow_mut()
                                .insert(key.clone(), args["text"].as_str().unwrap().to_string())
                        });
                        Outcome::Ok(json!({"key": key}))
                    }
                    "store.get" => STORE.with(|s| {
                        let s = s.borrow();
                        match s.get(args["key"].as_str().unwrap()) {
                            Some(t) => Outcome::Ok(json!({"key": args["key"], "text": t})),
                            None => Outcome::Ok(json!({"missing": true})),
                        }
                    }),
                    "store.search" => STORE.with(|s| {
                        let q = args["query"].as_str().unwrap().to_lowercase();
                        let hits: Vec<J> = s
                            .borrow()
                            .iter()
                            .filter_map(|(k, t)| {
                                t.to_lowercase().find(&q).map(
                                    |p| json!({"key": k, "snippet": t[p.saturating_sub(40)..(p + 60).min(t.len())]}),
                                )
                            })
                            .collect();
                        Outcome::Ok(json!({"hits": hits}))
                    }),
                    "send" => Outcome::Ok(json!({"delivered": true})),
                    other => Outcome::Error(format!("no executor {other}")),
                };
                Input::Result {
                    task,
                    outcome,
                    clock_ms: clock,
                }
            }
            Command::Timer { at_ms } => {
                clock = clock.max(at_ms);
                Input::Tick { clock_ms: clock }
            }
            Command::Cancel { .. }
            | Command::Abort { .. }
            | Command::Mount { .. }
            | Command::Props { .. }
            | Command::Unmount { .. }
            | Command::Callback { .. } => continue,
        };
        journal.push(input.clone());
        let cmds = agent
            .handle(&view, &enc, input)
            .unwrap_or_else(|e| panic!("t{turn}: {e}"));
        queue.extend(cmds);
    }
    let trace = agent.take_trace();
    for e in trace.iter().filter(|e| e["t"] == "handler_error").take(3) {
        eprintln!("HANDLER ERROR {e}");
    }
    let render_ms: Vec<f64> = trace
        .iter()
        .filter(|e| e["t"] == "render")
        .filter_map(|e| e["render_ms"].as_f64())
        .collect();
    eprintln!(
        "turn renders: {} mean {:.2} ms, max {:.2} ms",
        render_ms.len(),
        render_ms.iter().sum::<f64>() / render_ms.len().max(1) as f64,
        render_ms.iter().cloned().fold(0.0, f64::max)
    );
    // The log in full: released records are read back from their store batches (as log.read does).
    let state = agent.state();
    let mut t = affordance::scenario::log_records(&state);
    for b in state["__ui"]["@log"]["released"].as_array().into_iter().flatten() {
        let batch: Vec<J> = STORE.with(|s| serde_json::from_str(&s.borrow()[b["key"].as_str().unwrap()]).unwrap());
        let from = b["from"].as_u64().unwrap() as usize;
        for (j, r) in batch.into_iter().enumerate() {
            t[from + j] = r;
        }
    }
    // read_event returned the stored bytes exactly.
    for r in t.iter().filter(|r| r["name"] == "read_event") {
        let res = r["result"].as_str().unwrap_or_else(|| panic!("read_event record: {r}"));
        let stored = STORE.with(|s| s.borrow()["ls"].clone());
        let want: String = stored.chars().skip(2400).take(3000).collect();
        assert!(
            res.ends_with(&want),
            "read_event mismatch: {}",
            &res[..200.min(res.len())]
        );
    }
    let problems = t
        .iter()
        .filter(|r| r["kind"] == "model" && r["problems"] != json!([]))
        .count();
    let errors = t.iter().filter(|r| r["kind"] == "action" && r["error"] == true).count();
    let summary = json!({"turns": turn, "exact_appends": exact, "expected_breaks": expected_breaks, "folds": folds, "summaries": summaries, "summary_renders": render_shared,
        "fold_landings": fold_after, "max_prompt": max_prompt, "metrics": agent.metrics, "answers": [sc.answers, sc.answers_ok], "reads": sc.reads,
        "reply_problems": problems, "tool_errors": errors, "violations": violations});
    eprintln!("{}", serde_json::to_string_pretty(&summary).unwrap());
    assert!(violations.is_empty(), "{violations:#?}");
    // It shares all of the last prompt's history; only the screen (whose status line shows the
    // prompt's size, just updated) is laid out anew.
    for (shared, prompt) in &render_shared {
        assert!(
            shared + 256 >= *prompt && shared < prompt,
            "a summary render starts with the last prompt, so it reuses its cache: {shared} of {prompt}"
        );
    }
    assert_eq!(problems, 0);
    assert_eq!(
        errors,
        0,
        "{:?}",
        t.iter().filter(|r| r["error"] == true).collect::<Vec<_>>()
    );
    assert_eq!(exact + expected_breaks, turn - 1);
    if turns >= 300 {
        assert!(folds >= 5, "folds: {folds}");
        assert!(
            summaries >= 1 && !render_shared.is_empty(),
            "a fold summarized through harness.render"
        );
        assert_eq!((sc.answers, sc.answers_ok), (3, 3), "recall from folded history");
        let summarized_upto = agent.state()["__ui"]["@fold"]["summary_upto"].as_u64().unwrap() as usize;
        let released = affordance::scenario::log_records(&agent.state())
            .iter()
            .take_while(|r| r["released"] == true)
            .count();
        assert!(
            summarized_upto > 0 && released == summarized_upto,
            "summarized records are released: {released} of {summarized_upto}"
        );
        assert_eq!(sc.reads, 2);
    }
    // The journal rebuilds the agent exactly.
    let (replayed, _) = Agent::restore(cfg, journal, &view, &enc).unwrap();
    assert_eq!(
        affordance::scenario::log_records(&replayed.state()),
        affordance::scenario::log_records(&agent.state())
    );
    assert_eq!(replayed.state(), agent.state());
}

thread_local! { static STORE: std::cell::RefCell<BTreeMap<String, String>> = const { std::cell::RefCell::new(BTreeMap::new()) }; }
