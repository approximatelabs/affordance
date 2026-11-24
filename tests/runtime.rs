//! The agent runtime core (`runtime::instance`): a deterministic agent driven by a scripted GLM-5.3
//! model in a single-threaded host loop. Checks the app contract end to end: exact appends,
//! tool results as `action` records, hooks, primitives, messages, waits, and journal replay.
use affordance::View;
use affordance::effects::Outcome;
use affordance::emit::Encoder;
use affordance::runtime::{Command, Input, Instance, InstanceConfig};
use serde_json::{Value as J, json};
use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
use crate::shared;
fn encoder() -> Arc<Encoder> {
    shared::tokenizer(&root().join("ref/tokenizers/zai-org__GLM-5.3.json"))
}
fn view(enc: &Encoder) -> View {
    View::load(&root().join("views/demo/runtime/agent.tsx"), "Main", enc).unwrap()
}
const T0: i64 = 1_791_000_000_000;

fn config(id: &str, task: &str) -> InstanceConfig {
    InstanceConfig {
        context: None,
        retry: Default::default(),
        template: None,
        id: id.into(),
        props: json!({"task": task}),
        env: BTreeMap::new(),
        max_new: 512,
        max_turns: Some(8),
        max_prompt: None,
        start_ms: T0,
        seed: 7,
        machines: vec!["home".into()],
        model: Some("default".into()),
    }
}

/// Apply an input to an agent and keep it in the test's journal (an agent keeps none of its own).
fn apply(agent: &mut Instance, journal: &mut Vec<Input>, v: &View, enc: &Encoder, input: Input) -> Vec<Command> {
    journal.push(input.clone());
    agent.handle(v, enc, input).unwrap()
}

/// GLM-5.3 replies as a server returns them: ids including the stop token.
fn reply(enc: &Encoder, text: &str, stop: &str) -> Vec<u32> {
    let mut ids = enc.tok.encode(text, false).unwrap().get_ids().to_vec();
    ids.push(enc.token_id(stop).unwrap());
    ids
}

/// A host loop: answers commands with a scripted model and scripted primitives, routes `send`
/// between agents, advances a fake clock for timers.
struct Host<'a> {
    enc: &'a Encoder,
    view: &'a View,
    instances: BTreeMap<String, Instance>,
    queue: VecDeque<(String, Command)>,
    timers: Vec<(String, i64)>,
    clock: i64,
    script: Box<dyn FnMut(&str, u64) -> String + 'a>,
    turns: BTreeMap<String, u64>,
    prompts: Vec<(String, Vec<u32>, Vec<u32>)>,
    /// What each agent was fed: its journal (an agent keeps none of its own).
    journals: BTreeMap<String, Vec<Input>>,
}

impl<'a> Host<'a> {
    fn new(enc: &'a Encoder, view: &'a View, script: impl FnMut(&str, u64) -> String + 'a) -> Self {
        Host {
            enc,
            view,
            instances: BTreeMap::new(),
            queue: VecDeque::new(),
            timers: vec![],
            clock: T0,
            script: Box::new(script),
            turns: BTreeMap::new(),
            prompts: vec![],
            journals: BTreeMap::new(),
        }
    }
    fn add(&mut self, cfg: InstanceConfig) {
        let id = cfg.id.clone();
        let mut agent = Instance::new(cfg).unwrap();
        for c in agent.start(self.view, self.enc).unwrap() {
            self.queue.push_back((id.clone(), c));
        }
        self.instances.insert(id, agent);
    }
    fn feed(&mut self, id: &str, input: Input) {
        self.journals.entry(id.to_string()).or_default().push(input.clone());
        let cmds = self
            .instances
            .get_mut(id)
            .unwrap()
            .handle(self.view, self.enc, input)
            .unwrap_or_else(|e| panic!("{id}: {e}"));
        for c in cmds {
            self.queue.push_back((id.to_string(), c));
        }
    }
    fn run(&mut self) {
        loop {
            while let Some((id, cmd)) = self.queue.pop_front() {
                self.clock += 10;
                match cmd {
                    Command::Generate { task, ids, .. } => {
                        let n = {
                            let t = self.turns.entry(id.clone()).or_default();
                            *t += 1;
                            *t
                        };
                        let text = (self.script)(&id, n);
                        let stop = if text.contains("<tool_call>") {
                            "<|observation|>"
                        } else {
                            "<|user|>"
                        };
                        let out = reply(self.enc, &text, stop);
                        self.prompts.push((id.clone(), ids.to_vec(), out.clone()));
                        self.feed(
                            &id,
                            Input::Reply {
                                server: None,
                                task,
                                ids: out,
                                cut_off: false,
                                clock_ms: self.clock,
                                prompt: None,
                                aborted: false,
                            },
                        );
                    }
                    Command::Call { task, name, args } => {
                        let outcome = match name.as_str() {
                            "machine:home" => Outcome::Ok(
                                json!({"output": format!("ran: {}", args["cmd"].as_str().unwrap()), "exit": 0}),
                            ),
                            "send" => {
                                let to = args["to"].as_str().unwrap().to_string();
                                let payload = json!({"from": id, "text": args["payload"]["text"]});
                                self.feed(
                                    &to,
                                    Input::Event {
                                        name: "message".into(),
                                        payload,
                                        id: None,
                                        clock_ms: self.clock,
                                    },
                                );
                                Outcome::Ok(json!({"delivered": true}))
                            }
                            other => Outcome::Error(format!("no executor {other}")),
                        };
                        self.feed(
                            &id,
                            Input::Result {
                                task,
                                outcome,
                                clock_ms: self.clock,
                            },
                        );
                    }
                    Command::Timer { at_ms } => self.timers.push((id, at_ms)),
                    Command::Cancel { .. }
                    | Command::Abort { .. }
                    | Command::Mount { .. }
                    | Command::Props { .. }
                    | Command::Unmount { .. }
                    | Command::Callback { .. } => {}
                }
            }
            // Nothing queued: jump the clock to the next timer.
            let Some(i) = (0..self.timers.len()).min_by_key(|&i| self.timers[i].1) else {
                return;
            };
            let (id, at) = self.timers.remove(i);
            self.clock = self.clock.max(at);
            self.feed(&id, Input::Tick { clock_ms: self.clock });
        }
    }
}

fn glm_call(think: &str, name: &str, args: &[(&str, &str)]) -> String {
    let a: String = args
        .iter()
        .map(|(k, v)| format!("<arg_key>{k}</arg_key><arg_value>{v}</arg_value>"))
        .collect();
    format!("{think}</think><tool_call>{name}{a}</tool_call>")
}

/// "A bad call does not stop a good call to the same tool." One reply calls `set_notes` with no
/// text (outside its schema) and again with text: the first is answered with its problem, the
/// second runs and the notes are saved.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn a_bad_call_does_not_stop_a_good_call_to_the_same_tool() {
    let enc = encoder();
    let v = view(&enc);
    let mut host = Host::new(&enc, &v, |_, turn| {
        match turn {
        1 => "Two calls.</think><tool_call>set_notes</tool_call><tool_call>set_notes<arg_key>text</arg_key><arg_value>saved</arg_value></tool_call>".into(),
        _ => glm_call("Idle.", "wait", &[("seconds", "5")]),
    }
    });
    host.add(config("agent-0001", "Take notes."));
    host.run();
    let agent = &host.instances["agent-0001"];
    let results: Vec<J> = affordance::scenario::log_records(&agent.state())
        .iter()
        .filter(|r| r["kind"] == "action")
        .take(2)
        .map(|r| r["result"].clone())
        .collect();
    assert!(
        results[0]
            .as_str()
            .is_some_and(|s| s.starts_with("error: bad arguments for set_notes")),
        "{results:?}"
    );
    assert_eq!(results[1], "notes saved", "{results:?}");
    assert_eq!(agent.state()["__ui"]["@notes"], "saved");
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn one_agent_session_is_exact_and_replays() {
    let enc = encoder();
    let v = view(&enc);
    let mut host = Host::new(&enc, &v, |_, turn| match turn {
        1 => glm_call("Look first.  \n", "shell", &[("command", "ls /srv")]),
        2 => glm_call("Remember it.", "set_notes", &[("text", "srv has kv")]),
        3 => "Thinking out loud.</think>All good so far.".into(),
        4 => glm_call("", "fly", &[("to", "moon")]),
        5 => glm_call("Bad args.", "wait", &[("seconds", "soon")]),
        _ => glm_call("Idle.", "wait", &[("seconds", "5")]),
    });
    host.add(config("agent-0001", "Keep /srv healthy."));
    host.run();
    if std::env::var("DBG").is_ok() {
        let a = host.instances.get_mut("agent-0001").unwrap();
        for r in affordance::scenario::log_records(&a.state()) {
            let mut r = r.clone();
            r.as_object_mut().unwrap().remove("ids");
            eprintln!("T {r}");
        }
        for t in a.take_trace() {
            eprintln!("TR {t}");
        }
        eprintln!("M {:?}", a.metrics);
    }
    let agent = &host.instances["agent-0001"];
    assert_eq!(agent.stopped(), Some(&json!({"reason": "max_turns"})));
    // Every turn after the first is an exact append of the previous prompt + reply.
    for w in host.prompts.windows(2) {
        let prev = [w[0].1.as_slice(), w[0].2.as_slice()].concat();
        assert!(w[1].1.starts_with(&prev), "turn did not append exactly");
    }
    assert_eq!(
        (agent.metrics.turns, agent.metrics.exact_appends, agent.metrics.rewrites),
        (8, 7, 0)
    );
    let t = affordance::scenario::log_records(&agent.state());
    let kinds: Vec<&str> = t.iter().map(|r| r["kind"].as_str().unwrap()).collect();
    assert_eq!(
        &kinds[..9],
        &[
            "screen", "model", "action", "screen", "model", "action", "screen", "model", "screen"
        ]
    );
    assert_eq!(t[2]["result"], "ran: ls /srv\n[exit 0]");
    assert_eq!(t[1]["reasoning"], "Look first.");
    assert_eq!(t[5]["result"], "notes saved");
    assert_eq!(t[7]["content"], "All good so far.");
    assert_eq!(t[7]["problems"], json!(["no_action"]));
    assert!(
        t.iter()
            .any(|r| r["result"] == "error: there is no tool named `fly` on this screen")
    );
    assert!(t.iter().any(|r| {
        r["result"]
            .as_str()
            .is_some_and(|s| s.starts_with("error: bad arguments for wait"))
    }));
    assert!(t.iter().any(|r| r["result"] == "waited 5s"));
    // The hook persisted, and the text-only nudge appeared on the screen after turn 3.
    let notes = agent.state()["__ui"]
        .as_object()
        .unwrap()
        .values()
        .any(|v| v == "srv has kv");
    assert!(notes, "{}", agent.state());
    let screen_after_text = enc
        .tok
        .decode(
            t[8]["ids"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_u64().unwrap() as u32)
                .collect::<Vec<_>>()
                .as_slice(),
            false,
        )
        .unwrap();
    assert!(
        screen_after_text.contains("Your last reply had no tool call"),
        "{screen_after_text}"
    );
    // Replaying the journal reproduces the agent exactly.
    let (replayed, outstanding) =
        Instance::restore(agent.cfg.clone(), host.journals[agent.id()].clone(), &v, &enc).unwrap();
    assert_eq!(
        affordance::scenario::log_records(&replayed.state()),
        affordance::scenario::log_records(&agent.state())
    );
    assert_eq!(replayed.state(), agent.state());
    assert_eq!(replayed.metrics, agent.metrics);
    assert!(outstanding.is_empty());
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn messages_arrive_with_the_next_turn_and_wake_waits() {
    let enc = encoder();
    let v = view(&enc);
    let mut host = Host::new(&enc, &v, |id, turn| match (id, turn) {
        ("agent-a", 1) => glm_call("", "wait", &[("seconds", "600")]),
        ("agent-b", 1) => glm_call("", "send_message", &[("to", "agent-a"), ("text", "port is 48213")]),
        _ => glm_call("", "wait", &[("seconds", "1")]),
    });
    let mut a = config("agent-a", "Wait for b.");
    a.max_turns = Some(3);
    let mut b = config("agent-b", "Tell a the port.");
    b.max_turns = Some(2);
    host.add(a);
    host.add(b);
    host.run();
    let t = affordance::scenario::log_records(&host.instances["agent-a"].state()).to_vec();
    // a's 600 s wait was woken by the message, long before its deadline.
    let woke = t
        .iter()
        .position(|r| r["result"] == "woken by a message")
        .expect("woken");
    assert!(!t.iter().any(|r| r["result"] == "waited 600s"));
    // The message is an input after the wait's result, with the runtime-set sender.
    assert_eq!(
        t[woke + 1],
        json!({"kind": "input", "content": "[message from agent-b] port is 48213", "id": format!("e{}", woke + 1)})
    );
    let screen = enc
        .tok
        .decode(
            t[woke + 2]["ids"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_u64().unwrap() as u32)
                .collect::<Vec<_>>()
                .as_slice(),
            false,
        )
        .unwrap();
    assert!(screen.contains("unread: 1"), "{screen}");
    // Both agents stayed exact across the message.
    for a in host.instances.values() {
        assert_eq!(a.metrics.rewrites, 0, "{}", a.id());
    }
}

// ---- the multi-agent runtime (threads, pools, journals) ----

use affordance::runtime::{Exec, FnModel, GenReply, GenRequest, InstanceSpec, Runtime, RuntimeConfig};
use std::sync::{Arc, Mutex};

/// A scripted GLM-5.3 model: each agent's n-th reply comes from `script(agent, n)`.
fn scripted(
    enc: Arc<Encoder>,
    delay_ms: u64,
    script: fn(&str, usize) -> String,
) -> Arc<FnModel<impl Fn(&GenRequest) -> Result<GenReply, String> + Send + Sync>> {
    let counts: Mutex<BTreeMap<String, usize>> = Mutex::new(BTreeMap::new());
    Arc::new(FnModel(move |req: &GenRequest| {
        // The turn number is the count of model turns already in the prompt (stateless, so a
        // restored run continues the same script).
        let asst = enc.token_id("<|assistant|>").unwrap();
        let n = req.ids.iter().filter(|&&t| t == asst).count();
        counts.lock().unwrap().insert(req.agent.clone(), n);
        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
        let text = script(&req.agent, n);
        let stop = if text.contains("<tool_call>") {
            "<|observation|>"
        } else {
            "<|user|>"
        };
        let mut ids = enc.tok.encode(text.as_str(), false).unwrap().get_ids().to_vec();
        ids.push(enc.token_id(stop).unwrap());
        Ok(GenReply {
            ids,
            cut_off: false,
            ..Default::default()
        })
    }))
}

fn fleet_script(agent: &str, n: usize) -> String {
    let i: usize = agent.trim_start_matches("agent-").parse().unwrap();
    match n {
        1 => glm_call(
            "Look around.",
            "shell",
            &[(
                "command",
                &format!("echo hello from {agent}; echo $DEPLOY_TOKEN | wc -c"),
            )],
        ),
        2 => glm_call(
            "Tell my neighbour.",
            "send_message",
            &[
                ("to", &format!("agent-{}", (i + 1) % 6)),
                ("text", &format!("hi from {agent}")),
            ],
        ),
        3 => glm_call("Note it.", "set_notes", &[("text", &format!("{agent} said hello"))]),
        _ => glm_call("Wait for mail.", "wait", &[("seconds", "1")]),
    }
}

fn fleet_runtime(
    dir: &std::path::Path,
    delay_ms: u64,
    max_wall: Option<std::time::Duration>,
    restore: bool,
) -> affordance::runtime::Report {
    let enc = encoder();
    let mut cfg = RuntimeConfig::new(root().join("views/demo/runtime/agent.tsx"));
    cfg.workers = 3;
    cfg.dir = Some(dir.to_path_buf());
    cfg.max_wall = max_wall;
    cfg.env.insert("DEPLOY_TOKEN".into(), "tok-5ecret-value".into());
    cfg.secrets.insert("DEPLOY_TOKEN".into());
    let model = scripted(enc.clone(), delay_ms, fleet_script);
    let mut rt = if restore {
        Runtime::restore(cfg, enc, model).unwrap()
    } else {
        Runtime::new(cfg, enc, model)
    };
    // exec runs real bash; the agent's environ reaches the command only through `env`.
    rt.bind_machine(
        "home",
        Arc::new(|ctx: &affordance::runtime::CallContext, args: &J| {
            let mut args = args.clone();
            args["env"] = json!({"DEPLOY_TOKEN": ctx.env["DEPLOY_TOKEN"]});
            affordance::runtime::Executor::call(&Exec::local(), ctx, &args)
        }),
    );
    if !restore {
        for i in 0..6 {
            let mut spec = InstanceSpec::new(&format!("agent-{i}"), json!({"task": "Say hello to your neighbour."}));
            spec.max_turns = Some(5);
            spec.max_new = 256;
            spec.machines = Some(vec!["home".into()]);
            rt.add_instance(spec).unwrap();
        }
    }
    rt.run().unwrap()
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn six_agents_on_three_workers_message_each_other_and_resume_from_journals() {
    let dir = std::env::temp_dir().join(format!("aff-runtime-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    // A run interrupted mid-way (a crash, from the agents' point of view) ...
    let first = fleet_runtime(&dir, 40, Some(std::time::Duration::from_millis(300)), false);
    assert!(!first.complete);
    let partial: Vec<u64> = first.instances.iter().map(|a| a.metrics.turns).collect();
    assert!(
        partial.iter().all(|&t| t < 5) && partial.iter().any(|&t| t > 0),
        "interrupted mid-run: {partial:?}"
    );
    // ... continues from the journals to completion.
    let report = fleet_runtime(&dir, 0, Some(std::time::Duration::from_secs(60)), true);
    assert!(
        report.complete,
        "{:?}",
        report
            .instances
            .iter()
            .map(|a| (&a.id, &a.stopped, &a.error))
            .collect::<Vec<_>>()
    );
    assert_eq!(report.instances.len(), 6);
    for a in &report.instances {
        assert!(a.error.is_none(), "{}: {:?}", a.id, a.error);
        assert_eq!(a.metrics.turns, 5, "{}", a.id);
        assert_eq!(
            a.metrics.rewrites, 0,
            "{}: every turn appends exactly, across the restore",
            a.id
        );
        let records = affordance::scenario::log_records(&a.state);
        let results: Vec<&str> = records
            .iter()
            .filter(|r| r["kind"] == "action")
            .map(|r| r["result"].as_str().unwrap())
            .collect();
        let i: usize = a.id.trim_start_matches("agent-").parse().unwrap();
        // The app can use its own secret (17 = its length + newline) ...
        assert_eq!(results[0], format!("hello from {}\n17\n\n[exit 0]", a.id));
        assert_eq!(results[1], format!("sent to agent-{}", (i + 1) % 6));
        // ... the neighbour's message arrived as an input with the runtime-set sender.
        let from = format!("[message from agent-{}] hi from agent-{}", (i + 5) % 6, (i + 5) % 6);
        assert!(
            affordance::scenario::log_records(&a.state)
                .iter()
                .any(|r| r["kind"] == "input" && r["content"] == from.as_str()),
            "{}: {:?}",
            a.id,
            affordance::scenario::log_records(&a.state)
                .iter()
                .filter(|r| r["kind"] == "input")
                .collect::<Vec<_>>()
        );
        // ... but the runtime never writes the secret to disk.
        let trace = std::fs::read_to_string(crate::shared::instance_dir(&dir, &a.id).join("trace.jsonl")).unwrap();
        let spec = std::fs::read_to_string(crate::shared::instance_dir(&dir, &a.id).join("instance.json")).unwrap();
        assert!(!trace.contains("tok-5ecret-value") && !spec.contains("tok-5ecret-value"));
        assert!(trace.contains("\"t\":\"render\""));
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// `exec` over real SSH to localhost (skipped when key-based `ssh localhost` is not set up):
/// quoting survives (the command goes to `bash -s` on stdin), exit codes, the remote timeout,
/// and app-passed env vars.
#[test]
fn exec_over_ssh_localhost() {
    use affordance::runtime::{CallContext, Executor, Target};
    let probe = std::process::Command::new("ssh")
        .args([
            "-o",
            "BatchMode=yes",
            "-o",
            "ConnectTimeout=3",
            "-o",
            "StrictHostKeyChecking=accept-new",
            "-o",
            "UserKnownHostsFile=/tmp/aff-test-known-hosts",
            "localhost",
            "true",
        ])
        .output();
    if !probe.is_ok_and(|o| o.status.success()) {
        eprintln!("skipping: no key-based ssh to localhost");
        return;
    }
    let exec = Exec {
        target: Target::Ssh {
            target: "localhost".into(),
            key: None,
            control_dir: "/tmp".into(),
            known_hosts: "/tmp/aff-test-known-hosts".into(),
        },
        default_timeout_s: 10,
        max_capture: 1 << 20,
    };
    let ctx = CallContext {
        agent: "agent-0001".into(),
        instance: String::new(),
        task: 0,
        env: BTreeMap::new(),
        cancel: Default::default(),
        blobs: None,
        output: None,
    };
    let r = exec
        .call(
            &ctx,
            &json!({"cmd": r#"echo "it's \"quoted\" $X; done"; printf 'a\tb\n'"#, "env": {"X": "set"}}),
        )
        .unwrap();
    assert_eq!(r["output"], "it's \"quoted\" set; done\na\tb\n");
    assert_eq!(r["exit"], 0);
    let r = exec.call(&ctx, &json!({"cmd": "ls /no/such/path"})).unwrap();
    assert_eq!(r["exit"], 2);
    let r = exec.call(&ctx, &json!({"cmd": "sleep 30", "timeout_s": 2})).unwrap();
    assert_eq!((r["exit"].as_i64(), r["timed_out"].as_bool()), (Some(124), Some(true)));
}

/// A refused model call is tried again as many times as the retry policy's `refused` says (here
/// four), after a doubling wait and with the same prompt; then the app gets the error.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn model_errors_retry_with_backoff_then_stop() {
    let enc = encoder();
    let v = view(&enc);
    let mut cfg = config("agent-0001", "Task.");
    cfg.retry.refused = 4;
    let mut agent = Instance::new(cfg).unwrap();
    let cmds = agent.start(&v, &enc).unwrap();
    let Command::Generate { task, ids, .. } = cmds[0].clone() else {
        panic!()
    };
    let mut clock = T0;
    let mut task = task;
    for failure in 1..=4u32 {
        let out = agent
            .handle(
                &v,
                &enc,
                Input::ModelError {
                    task,
                    error: "HTTP 400 invalid request".into(),
                    kind: affordance::runtime::model::FailureKind::Refused,
                    retry_after_ms: None,
                    clock_ms: clock,
                },
            )
            .unwrap();
        let [Command::Timer { at_ms }] = out.as_slice() else {
            panic!("{out:?}")
        };
        assert_eq!(*at_ms, clock + 1000 * (1 << (failure - 1)));
        clock = *at_ms;
        let out = agent.handle(&v, &enc, Input::Tick { clock_ms: clock }).unwrap();
        let [
            Command::Generate {
                task: t, ids: again, ..
            },
        ] = out.as_slice()
        else {
            panic!("{out:?}")
        };
        assert_eq!(again, &ids, "same prompt");
        task = *t;
    }
    agent
        .handle(
            &v,
            &enc,
            Input::ModelError {
                task,
                error: "HTTP 400 invalid request".into(),
                kind: affordance::runtime::model::FailureKind::Refused,
                retry_after_ms: None,
                clock_ms: clock,
            },
        )
        .unwrap();
    // The call fails after its retries; the loop (views/lib/loop.tsx) stops the instance with the error.
    assert_eq!(agent.stopped().unwrap()["reason"], "loop_error");
    assert!(
        agent.stopped().unwrap()["error"].as_str().unwrap().contains("HTTP 400"),
        "{:?}",
        agent.stopped()
    );
    assert_eq!(agent.metrics.model_errors, 5);
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn inference_outage_preserves_agent_and_retry_across_restore() {
    let enc = encoder();
    let v = view(&enc);
    let cfg = config("agent-outage", "Task.");
    let mut agent = Instance::new(cfg.clone()).unwrap();
    let first = agent.start(&v, &enc).unwrap();
    let Command::Generate {
        mut task,
        ids: ref prompt,
        ..
    } = first[0]
    else {
        panic!()
    };
    let mut clock = T0;
    let mut journal = vec![];
    for _ in 0..8 {
        let out = apply(
            &mut agent,
            &mut journal,
            &v,
            &enc,
            Input::ModelError {
                task,
                error: "connection refused".into(),
                kind: affordance::runtime::model::FailureKind::Interrupted,
                retry_after_ms: None,
                clock_ms: clock,
            },
        );
        let [Command::Timer { at_ms }] = out.as_slice() else {
            panic!("{out:?}")
        };
        assert!(*at_ms - clock <= 30000);
        let (restored, pending) = Instance::restore(cfg.clone(), journal.clone(), &v, &enc).unwrap();
        assert!(restored.stopped().is_none());
        assert!(pending.iter().any(|c| matches!(c,Command::Timer{at_ms:t} if t==at_ms)));
        agent = restored;
        clock = *at_ms;
        let out = apply(&mut agent, &mut journal, &v, &enc, Input::Tick { clock_ms: clock });
        let [Command::Generate { task: t, ids, .. }] = out.as_slice() else {
            panic!("{out:?}")
        };
        assert_eq!(ids, prompt);
        task = *t;
    }
}

/// A reply cut off mid-thinking is recorded as thinking with
/// `unclosed_think` + `cut_off`, never as content; a call cut off mid-way is never run.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn cut_off_replies_are_classified_not_misread() {
    let enc = encoder();
    let v = view(&enc);
    let mut agent = Instance::new(config("agent-0001", "Task.")).unwrap();
    let Command::Generate { task, .. } = agent.start(&v, &enc).unwrap()[0].clone() else {
        panic!()
    };
    let ids = enc
        .tok
        .encode("The puzzle has three parts and the second one", false)
        .unwrap()
        .get_ids()
        .to_vec();
    let out = agent
        .handle(
            &v,
            &enc,
            Input::Reply {
                server: None,
                task,
                ids,
                cut_off: true,
                clock_ms: T0,
                prompt: None,
                aborted: false,
            },
        )
        .unwrap();
    let r = affordance::scenario::log_records(&agent.state())[1].clone();
    assert_eq!(
        (r["problems"].clone(), r["content"].clone(), r["cut_off"].clone()),
        (json!(["unclosed_think", "cut_off"]), json!(""), json!(true))
    );
    assert_eq!(r["reasoning"], "The puzzle has three parts and the second one");
    let Command::Generate { task, .. } = out[0].clone() else {
        panic!("next turn")
    };
    let ids = enc
        .tok
        .encode(
            "ok</think><tool_call>shell<arg_key>command</arg_key><arg_value>rm -rf /tm",
            false,
        )
        .unwrap()
        .get_ids()
        .to_vec();
    agent
        .handle(
            &v,
            &enc,
            Input::Reply {
                server: None,
                task,
                ids,
                cut_off: true,
                clock_ms: T0 + 1,
                prompt: None,
                aborted: false,
            },
        )
        .unwrap();
    assert!(
        affordance::scenario::log_records(&agent.state())
            .iter()
            .all(|r| r["kind"] != "action"),
        "no call is run from a fragment"
    );
    assert_eq!(
        affordance::scenario::log_records(&agent.state())[3]["problems"],
        json!(["bad_syntax", "no_action", "cut_off"])
    );
}

fn running(marker: &str) -> bool {
    std::process::Command::new("pgrep")
        .args(["-f", marker])
        .output()
        .is_ok_and(|o| o.status.success())
}

/// Cancelling an `exec` call stops the command and its children (not just the wait for it).
#[test]
fn cancelled_exec_kills_the_process_group() {
    use affordance::runtime::{CallContext, CancelToken, Executor};
    let marker = format!("aff-cancel-{}", std::process::id());
    let token = CancelToken::new();
    let ctx = CallContext {
        agent: "a".into(),
        instance: String::new(),
        task: 0,
        env: BTreeMap::new(),
        cancel: token.clone(),
        blobs: None,
        output: None,
    };
    let cmd = format!("sleep 30 && echo {marker}-never; bash -c 'exec -a {marker} sleep 30' & wait");
    let started = std::time::Instant::now();
    let h = std::thread::spawn(move || Exec::local().call(&ctx, &json!({"cmd": cmd, "timeout_s": 60})));
    std::thread::sleep(std::time::Duration::from_millis(500));
    token.cancel("test");
    let r = h.join().unwrap();
    assert!(r.as_ref().is_err_and(|e| e.starts_with("cancelled")), "{r:?}");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(8),
        "returned in {:?}",
        started.elapsed()
    );
    std::thread::sleep(std::time::Duration::from_millis(300));
    assert!(!running(&marker), "process group was killed");
}

/// A runtime that stops (here: max_wall) aborts commands its agents are still running.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn runtime_shutdown_cancels_running_commands() {
    let marker = format!("aff-shutdown-{}", std::process::id());
    let enc = encoder();
    let mut cfg = RuntimeConfig::new(root().join("views/demo/runtime/agent.tsx"));
    cfg.max_wall = Some(std::time::Duration::from_millis(800));
    static MARKER: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    MARKER.set(marker.clone()).unwrap();
    let model = scripted(enc.clone(), 0, |_, _| {
        glm_call(
            "",
            "shell",
            &[("command", &format!("exec -a {} sleep 60", MARKER.get().unwrap()))],
        )
    });
    let mut rt = Runtime::new(cfg, enc, model);
    rt.bind_machine("home", Arc::new(Exec::local()));
    rt.add_instance(InstanceSpec::new("agent-0", json!({"task": "Run it."})))
        .unwrap();
    let report = rt.run().unwrap();
    assert!(!report.complete);
    std::thread::sleep(std::time::Duration::from_millis(1500));
    assert!(!running(&marker), "the sleep was killed at shutdown");
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn a_pause_freezes_the_wait_clock_and_mail_is_durable() {
    use std::time::Duration;
    let dir = std::env::temp_dir().join(format!("aff-pause-clock-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let enc = encoder();
    let config = |millis| {
        let mut c = RuntimeConfig::new(root().join("views/demo/runtime/agent.tsx"));
        c.dir = Some(dir.clone());
        c.max_wall = Some(Duration::from_millis(millis));
        c
    };
    // Timings leave room for a loaded machine and still catch the regression: the run and the
    // restore take 0.9 + 0.4 s of logical time, under the 2 s wait; the pause between them makes
    // 3.0 s of real time, past it (0.9 s, and not less, because the first run's budget also
    // covers starting up and its first turn).
    let make_model = || scripted(enc.clone(), 0, |_, _| glm_call("Waiting.", "wait", &[("seconds", "2")]));
    let mut first = Runtime::new(config(900), enc.clone(), make_model());
    let mut spec = InstanceSpec::new("agent-a", json!({"task":"Wait for an operator message."}));
    spec.max_turns = Some(2);
    first.add_instance(spec).unwrap();
    let report = first.run().unwrap();
    assert_eq!(report.instances[0].metrics.turns, 1);
    std::thread::sleep(Duration::from_millis(1700));
    let report = Runtime::restore(config(400), enc.clone(), make_model())
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(
        report.instances[0].metrics.turns, 1,
        "suspended time must not expire the wait"
    );
    // A post with a key is answered once its line is committed; posted again (its answer lost,
    // say), it is journaled but not taken a second time.
    let mail = json!({"from":"operator","text":"Please wake up."});
    let mut rt = Runtime::restore(config(1500), enc.clone(), make_model()).unwrap();
    let delivered = rt.control().post("agent-a", "message", mail.clone(), Some("abc123"));
    let report = rt.run().unwrap();
    assert_eq!(delivered.wait(), Ok(()));
    assert_eq!(report.instances[0].metrics.turns, 2);
    let mut rt = Runtime::restore(config(400), enc.clone(), make_model()).unwrap();
    let again = rt.control().post("agent-a", "message", mail, Some("abc123"));
    let report = rt.run().unwrap();
    assert_eq!(again.wait(), Ok(()));
    assert_eq!(report.instances[0].metrics.turns, 2, "the same key is taken once");
    let journal = std::fs::read_to_string(crate::shared::inst(&dir, "agent-a/journal.jsonl")).unwrap();
    assert_eq!(
        journal.matches("\"id\":\".post:abc123\"").count(),
        2,
        "the key is on the journaled event, not in the app's payload"
    );
    let trace = std::fs::read_to_string(crate::shared::inst(&dir, "agent-a/trace.jsonl")).unwrap();
    assert!(
        trace.contains("duplicate_message"),
        "the second post is noted as a duplicate"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_timer_does_not_run_down_while_a_pause_waits_for_another_agents_command() {
    // A 2.5 s timer in one agent; a 3.5 s command in another. A pause once the command has
    // started waits for it to finish, by when the timer would be due; but the timer's time stops at the pause:
    // it neither fires during the drain nor after the restore until the ~2.2 s it had left pass. (Found by Multiplet: a 240 s
    // wait fired a second after its fleet restored.)
    use std::process::Command as Process;
    use std::time::{Duration, Instant};
    let dir = std::env::temp_dir().join(format!("aff-pause-drain-clock-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let home = dir.join("home");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(
        dir.join("app.tsx"),
        r#"export function Main({busy}) {
  useEffect(() => {
    async function run() {
      const home = harness.machine.get("home");
      if (busy) { await home.call({cmd: "touch busy-started; sleep 3.5"}); return; }
      await harness.wait({ms: 2500, events: []});
      await home.call({cmd: "touch timer-fired"});
      harness.stop({});
    }
    run();
  }, []);
  return <box>timer</box>;
}"#,
    )
    .unwrap();
    let deployment = json!({
        "app": {"view": "app.tsx", "entry": "Main"},
        "model": {"kind": "chat", "url": "http://127.0.0.1:9/v1", "model": "x", "context": 1000},
        "machines": {"home": {"kind": "local", "home": home}},
        "instances": [
            {"id": "busy", "model": null, "props": {"busy": true}},
            {"id": "timer", "model": null, "props": {"busy": false}}
        ],
        "dir": "agents",
        "max_wall_s": 20
    });
    std::fs::write(dir.join("deployment.json"), deployment.to_string()).unwrap();
    let socket = dir.join("control.sock");
    let run = |restore: bool| {
        let mut command = Process::new(env!("CARGO_BIN_EXE_affordance"));
        command.arg("run").arg("deployment.json").current_dir(&dir);
        command.arg("--control").arg(&socket);
        if restore {
            command.arg("--restore");
        }
        command
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        command
    };
    let mut first = run(false).spawn().unwrap();
    // The pause comes once the command is running (and the timer with it), so the drain is real.
    let deadline = Instant::now() + Duration::from_secs(20);
    while !home.join("busy-started").exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    std::thread::sleep(Duration::from_millis(300));
    crate::controller::Controller::connect(&socket).close(150_000);
    assert!(first.wait().unwrap().success());
    assert!(
        !home.join("timer-fired").exists(),
        "the timer fired while the pause drained"
    );
    let restored = Instant::now();
    assert!(run(true).status().unwrap().success());
    assert!(home.join("timer-fired").exists());
    let waited = std::fs::metadata(home.join("timer-fired"))
        .unwrap()
        .modified()
        .unwrap()
        .duration_since(std::time::SystemTime::now() - restored.elapsed())
        .unwrap_or_default();
    eprintln!("after the restore the timer fired in {waited:?}");
    assert!(
        waited >= Duration::from_millis(2000),
        "after the restore the timer fired in {waited:?}, not the ~2.2 s it had left"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn recovery_repairs_a_torn_tail_before_appending() {
    use std::io::Write;
    let dir = std::env::temp_dir().join(format!("aff-torn-tail-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    fleet_runtime(&dir, 40, Some(std::time::Duration::from_millis(300)), false);
    let path = crate::shared::inst(&dir, "agent-0/journal.jsonl");
    let before = std::fs::read(&path).unwrap();
    std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap()
        .write_all(b"{\"Reply\":")
        .unwrap();
    let result = fleet_runtime(&dir, 0, Some(std::time::Duration::from_secs(30)), true);
    assert!(result.complete);
    let after = std::fs::read_to_string(&path).unwrap();
    assert!(after.as_bytes().starts_with(&before));
    for line in after.lines() {
        serde_json::from_str::<Input>(line).unwrap();
    }
    assert!(
        std::fs::read_dir(crate::shared::instance_dir(&dir, "agent-0"))
            .unwrap()
            .flatten()
            .any(|p| p.file_name().to_string_lossy().starts_with("torn-journal-"))
    );
    std::fs::remove_dir_all(dir).unwrap();
}

/// A failing overflow handler (a summary that fails, perhaps for good) does not stop the agent:
/// the turn proceeds with the prompt as it is, and the journal replays to the same agent.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn a_failing_overflow_handler_lets_the_turn_proceed() {
    let enc = encoder();
    let view = View::load(&root().join("views/demo/runtime/overflow_fails.tsx"), "Main", &enc).unwrap();
    let mut host = Host::new(&enc, &view, |_, _| glm_call("Go.", "nothing", &[]));
    let unbound = || InstanceConfig {
        context: None,
        retry: Default::default(),
        machines: vec![],
        ..config("a", "Work.")
    };
    host.add(unbound());
    host.run();
    let a = &host.instances["a"];
    // The app's fold fails (no machine); it catches that, and the step goes on.
    assert!(!host.prompts.is_empty(), "the turn proceeded after the fold failed");
    assert_eq!(a.stopped().unwrap()["reason"], "max_turns", "{:?}", a.stopped());
    let (b, _) = Instance::restore(unbound(), host.journals["a"].clone(), &view, &enc).unwrap();
    assert_eq!(
        affordance::scenario::log_records(&b.state()),
        affordance::scenario::log_records(&a.state())
    );
}

fn pinned_run(
    dir: &std::path::Path,
    view: &std::path::Path,
    restore: bool,
    strict: bool,
) -> affordance::runtime::Report {
    let enc = encoder();
    let mut cfg = RuntimeConfig::new(view);
    cfg.workers = 1;
    cfg.dir = Some(dir.to_path_buf());
    cfg.strict_replay = strict;
    let model = scripted(enc.clone(), 0, |_, n| {
        glm_call("Note.", "set_notes", &[("text", &format!("n{n}"))])
    });
    let mut rt = if restore {
        Runtime::restore(cfg, enc, model).unwrap()
    } else {
        Runtime::new(cfg, enc, model)
    };
    if !restore {
        let mut spec = InstanceSpec::new("agent-0", json!({"task": "Keep notes."}));
        spec.max_turns = Some(3);
        rt.add_instance(spec).unwrap();
    }
    rt.run().unwrap()
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn resume_pins_the_app_and_detects_a_changed_one() {
    let dir = std::env::temp_dir().join(format!("aff-pins-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let original = root().join("views/demo/runtime/agent.tsx");
    let first = pinned_run(&dir, &original, false, false);
    assert_eq!(first.instances[0].metrics.turns, 3);
    let pins: J =
        serde_json::from_slice(&std::fs::read(crate::shared::inst(&dir, "agent-0/instance.json")).unwrap()).unwrap();
    assert_eq!(pins["pins"]["contract"], affordance::runtime::host::CONTRACT_VERSION);
    assert_eq!(pins["pins"]["app"].as_str().unwrap().len(), 64);
    // The same app replays to the same prompts.
    let same = pinned_run(&dir, &original, true, false);
    assert_eq!(same.instances[0].diverged_at, None);
    assert_eq!(same.instances[0].state, first.instances[0].state);
    // A changed status line: the first turn's prompt differs.
    let changed = root().join(format!("views/demo/runtime/.changed-{}.tsx", std::process::id()));
    std::fs::write(
        &changed,
        std::fs::read_to_string(&original)
            .unwrap()
            .replace("| turn ${harness.turn}", "| step ${harness.turn}"),
    )
    .unwrap();
    let loose = pinned_run(&dir, &changed, true, false);
    let strict = pinned_run(&dir, &changed, true, true);
    std::fs::remove_file(&changed).unwrap();
    assert_eq!(loose.instances[0].diverged_at, Some(1), "turns count from 1");
    assert!(
        strict.instances[0]
            .error
            .as_deref()
            .is_some_and(|e| e.contains("replay diverged at turn 1")),
        "{:?}",
        strict.instances[0].error
    );
    let after: J =
        serde_json::from_slice(&std::fs::read(crate::shared::inst(&dir, "agent-0/instance.json")).unwrap()).unwrap();
    assert_eq!(
        after["pins"], pins["pins"],
        "the journal keeps the pins it was written against"
    );
    // A journal of another contract is refused, not replayed.
    let mut old = after.clone();
    old["pins"]["contract"] = json!(1);
    std::fs::write(
        crate::shared::inst(&dir, "agent-0/instance.json"),
        serde_json::to_string(&old).unwrap(),
    )
    .unwrap();
    let refused = pinned_run(&dir, &original, true, false);
    let err = refused.instances[0].error.clone().unwrap_or_default();
    assert!(err.contains("restore refused") && err.contains("contract 1"), "{err}");
    assert_eq!(refused.instances[0].metrics.turns, 0);
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn harness_render_runs_a_component_once_on_the_model_through_the_harness() {
    let enc = encoder();
    let view = View::load(&root().join("views/demo/runtime/render_once.tsx"), "Main", &enc).unwrap();
    let mut host = Host::new(&enc, &view, |_, n| match n {
        1 => glm_call("Polish it.", "polish", &[("text", "hi all")]),
        2 => "Short and kind.</think>Hello, everyone!".into(),
        _ => glm_call("Done.", "polish", &[("text", "bye")]),
    });
    let mut cfg = config("a", "Greet.");
    cfg.max_turns = Some(2);
    host.add(cfg);
    host.run();
    // The render's prompt is the component's own ids, not the agent's context.
    let (_, render_prompt, _) = &host.prompts[1];
    let text = enc.tok.decode(render_prompt, false).unwrap();
    assert!(
        text.contains("Rewrite the user's text politely") && text.contains("hi all") && !text.contains("Greet."),
        "{text}"
    );
    let a = &host.instances["a"];
    let action = affordance::scenario::log_records(&a.state())
        .into_iter()
        .find(|r| r["kind"] == "action")
        .unwrap();
    assert_eq!(action["result"], "Hello, everyone!");
    let (b, _) = Instance::restore(config("a", "Greet."), host.journals["a"].clone(), &view, &enc).unwrap();
    assert_eq!(
        affordance::scenario::log_records(&b.state()),
        affordance::scenario::log_records(&a.state())
    );
}

/// An ad-hoc model call (`model(<View/>)`) that fails transiently (a rate limit, a serving restart)
/// is retried with the same prompt after a capped backoff, for as long as it fails, across a
/// restore; the handler never sees the failure.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn a_render_rate_limited_is_retried_not_failed() {
    let enc = encoder();
    let v = View::load(&root().join("views/demo/runtime/render_once.tsx"), "Main", &enc).unwrap();
    let cfg = config("a", "Greet.");
    let mut agent = Instance::new(cfg.clone()).unwrap();
    let Command::Generate { task, ids: page, .. } = agent.start(&v, &enc).unwrap()[0].clone() else {
        panic!()
    };
    let mut journal = vec![];
    let out = apply(
        &mut agent,
        &mut journal,
        &v,
        &enc,
        Input::Reply {
            server: None,
            task,
            ids: reply(
                &enc,
                &glm_call("Polish it.", "polish", &[("text", "hi all")]),
                "<|observation|>",
            ),
            cut_off: false,
            clock_ms: T0,
            prompt: None,
            aborted: false,
        },
    );
    let [Command::Generate { task, ids, .. }] = out.as_slice() else {
        panic!("{out:?}")
    };
    let (mut task, prompt) = (*task, ids.clone());
    let mut clock = T0;
    for failure in 1..=14u32 {
        let out = apply(
            &mut agent,
            &mut journal,
            &v,
            &enc,
            Input::ModelError {
                task,
                error: "HTTP 429 too many requests".into(),
                kind: affordance::runtime::model::FailureKind::Interrupted,
                retry_after_ms: None,
                clock_ms: clock,
            },
        );
        let [Command::Timer { at_ms }] = out.as_slice() else {
            panic!("{out:?}")
        };
        assert_eq!(*at_ms, clock + 1000 * (1_i64 << (failure - 1).min(5)).min(30));
        let (restored, pending) = Instance::restore(cfg.clone(), journal.clone(), &v, &enc).unwrap();
        assert!(
            pending.contains(&Command::Timer { at_ms: *at_ms }),
            "a restore keeps the retry: {pending:?}"
        );
        agent = restored;
        clock = *at_ms;
        let out = apply(&mut agent, &mut journal, &v, &enc, Input::Tick { clock_ms: clock });
        let [Command::Generate { task: t, ids, .. }] = out.as_slice() else {
            panic!("{out:?}")
        };
        assert_eq!(ids, &prompt, "same prompt");
        task = *t;
    }
    // The ad-hoc prompt is model work like the page's: counted once, however often it was retried.
    assert_eq!(agent.metrics.prompt_tokens, (page.len() + prompt.len()) as u64);
    assert_eq!(agent.metrics.uncached_tokens, agent.metrics.prompt_tokens);
    agent
        .handle(
            &v,
            &enc,
            Input::Reply {
                server: None,
                task,
                ids: reply(&enc, "Kind.</think>Hello, everyone!", "<|user|>"),
                cut_off: false,
                clock_ms: clock,
                prompt: None,
                aborted: false,
            },
        )
        .unwrap();
    let action = affordance::scenario::log_records(&agent.state())
        .into_iter()
        .find(|r| r["kind"] == "action")
        .unwrap();
    assert_eq!(action["result"], "Hello, everyone!");
    assert_eq!(agent.metrics.handler_errors, 0);
}

/// A render's options reach its model call (budget, sampling, tag, priority); a render prompt over
/// `max_prompt` is never sent, and the handler sees the error.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn a_render_has_its_own_options_and_respects_max_prompt() {
    use affordance::runtime::GenOptions;
    let enc = encoder();
    let v = View::load(&root().join("views/demo/runtime/render_once.tsx"), "Main", &enc).unwrap();
    let polish = |text: &str| {
        reply(
            &enc,
            &glm_call("Polish it.", "polish", &[("text", text)]),
            "<|observation|>",
        )
    };
    let mut a = Instance::new(config("a", "Greet.")).unwrap();
    let Command::Generate { task, .. } = a.start(&v, &enc).unwrap()[0].clone() else {
        panic!()
    };
    let out = a
        .handle(
            &v,
            &enc,
            Input::Reply {
                server: None,
                task,
                ids: polish("hi all"),
                cut_off: false,
                clock_ms: T0,
                prompt: None,
                aborted: false,
            },
        )
        .unwrap();
    let [Command::Generate { max_new, opts, .. }] = out.as_slice() else {
        panic!("{out:?}")
    };
    assert_eq!(*max_new, 64);
    assert_eq!(
        opts,
        &GenOptions {
            sampling: json!({"temperature": 0.2}),
            tag: Some("polish".into()),
            priority: true,
            abortable: false,
            pipe: None,
        }
    );
    // A long text makes the render's prompt longer than the agent's first prompt.
    let mut b = Instance::new(config("b", "Greet.")).unwrap();
    let Command::Generate { task, ids, .. } = b.start(&v, &enc).unwrap()[0].clone() else {
        panic!()
    };
    let mut b = Instance::new(InstanceConfig {
        context: None,
        retry: Default::default(),
        max_prompt: Some(ids.len() + 20),
        ..config("b", "Greet.")
    })
    .unwrap();
    b.start(&v, &enc).unwrap();
    let out = b
        .handle(
            &v,
            &enc,
            Input::Reply {
                server: None,
                task,
                ids: polish(&"hello there ".repeat(200)),
                cut_off: false,
                clock_ms: T0,
                prompt: None,
                aborted: false,
            },
        )
        .unwrap();
    assert!(
        !out.iter().any(|c| matches!(c, Command::Generate { .. })) || b.stopped().is_some(),
        "the render was not sent: {out:?}"
    );
    let action = affordance::scenario::log_records(&b.state())
        .into_iter()
        .find(|r| r["kind"] == "action")
        .unwrap();
    assert!(
        action["result"].as_str().unwrap().contains("over max_prompt"),
        "{action}"
    );
}

#[test]
fn a_docker_machine_is_one_container_per_agent() {
    use affordance::runtime::{CallContext, Executor, Target};
    let docker = std::process::Command::new("docker").arg("info").output();
    if !docker.is_ok_and(|o| o.status.success()) {
        eprintln!("no docker daemon: skipped");
        return;
    }
    let name = format!("aff-test-{}", std::process::id());
    let m = Exec {
        target: Target::Docker {
            image: "ubuntu:24.04".into(),
            name: name.clone(),
        },
        default_timeout_s: 60,
        max_capture: 1 << 20,
    };
    let ctx = |agent: &str| CallContext {
        agent: agent.into(),
        instance: String::new(),
        task: 0,
        env: BTreeMap::new(),
        cancel: Default::default(),
        blobs: None,
        output: None,
    };
    let run = |agent: &str, cmd: &str| m.call(&ctx(agent), &json!({"cmd": cmd})).unwrap();
    let first = run("a", "grep -m1 PRETTY_NAME /etc/os-release; echo hi > /root/mark");
    assert!(first["output"].as_str().unwrap().contains("Ubuntu 24.04"), "{first}");
    assert_eq!(
        run("a", "cat /root/mark")["output"],
        "hi\n",
        "the container persists between calls"
    );
    assert!(
        run("b", "cat /root/mark 2>&1 || true")["output"]
            .as_str()
            .unwrap()
            .contains("No such file"),
        "each agent has its own"
    );
    assert_eq!(run("a", "exit 3")["exit"], 3);
    // The machine names an instance's container `<name>-<instance id>` (outside a run, the name
    // escaped: `i-a`).
    for agent in ["a", "b"] {
        let container = format!("{name}-i-{agent}");
        let removed = std::process::Command::new("docker")
            .args(["rm", "-f", &container])
            .output()
            .unwrap();
        assert_eq!(
            String::from_utf8_lossy(&removed.stdout).trim(),
            container,
            "the test's container is removed"
        );
    }
}

/// "A command in a bwrap machine cannot read my files or change the system." It sees the
/// system's programs, its own home (writable) and the paths the deployment lists under `read`,
/// and nothing else of the host: a file elsewhere is not there for it.
#[test]
fn a_bwrap_machine_shows_a_command_the_system_and_its_home_and_no_other_files() {
    use affordance::runtime::{CallContext, Executor, Target};
    // Where bubblewrap is missing, or the kernel does not let this user make a sandbox (Ubuntu's
    // default since 24.04, GitHub's runners), there is nothing to run.
    let works = std::process::Command::new("bwrap")
        .args([
            "--ro-bind",
            "/usr",
            "/usr",
            "--symlink",
            "usr/bin",
            "/bin",
            "--symlink",
            "usr/lib",
            "/lib",
        ])
        .args(["--symlink", "usr/lib64", "/lib64", "--unshare-all", "/usr/bin/true"])
        .output()
        .is_ok_and(|o| o.status.success());
    if !works {
        eprintln!("bwrap cannot make a sandbox here: skipped");
        return;
    }
    // Two directories that are not the system's and not /tmp (which the sandbox replaces): one
    // the deployment shows the machine, one it does not.
    let base = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("aff-bwrap-{}", std::process::id()));
    let (shown, kept) = (base.join("shown"), base.join("kept"));
    for dir in [&shown, &kept] {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("note"), "a note\n").unwrap();
    }
    let home = base.join("homes");
    let m = Exec {
        target: Target::Bwrap {
            home: home.display().to_string(),
            per_instance: true,
            network: false,
            read: vec![shown.display().to_string()],
        },
        default_timeout_s: 30,
        max_capture: 1 << 20,
    };
    let ctx = CallContext {
        agent: "a".into(),
        instance: String::new(),
        task: 0,
        env: BTreeMap::new(),
        cancel: Default::default(),
        blobs: None,
        output: None,
    };
    let cmd = format!(
        "echo ok > here && cat here; (touch /etc/aff-escape 2>&1 || true); cat {}/note; (cat {}/note 2>&1 || true); (touch {}/new 2>&1 || true)",
        shown.display(),
        kept.display(),
        shown.display()
    );
    let r = m.call(&ctx, &json!({"cmd": cmd})).unwrap();
    let out = r["output"].as_str().unwrap();
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "ok", "its home is writable: {out}");
    assert!(lines[1].contains("Read-only"), "the system is read-only: {out}");
    assert_eq!(lines[2], "a note", "a path under `read` is shown: {out}");
    assert!(
        lines[3].contains("No such file"),
        "a file elsewhere on the host is not there: {out}"
    );
    assert!(
        lines[4].contains("Read-only"),
        "a path under `read` cannot be written: {out}"
    );
    let _ = std::fs::remove_dir_all(&base);
}

fn tree_run(dir: &std::path::Path, restore: bool) -> affordance::runtime::Report {
    let enc = encoder();
    let mut cfg = RuntimeConfig::new(root().join("views/demo/tree/root.tsx"));
    cfg.workers = 1;
    cfg.dir = Some(dir.to_path_buf());
    let e = enc.clone();
    let model = Arc::new(FnModel(move |req: &GenRequest| {
        let text = e.tok.decode(&req.ids, false).unwrap();
        let turns = req
            .ids
            .iter()
            .filter(|&&t| t == e.token_id("<|assistant|>").unwrap())
            .count();
        let reply = if text.contains("Rewrite the user's text politely") {
            "Kind.</think>Hello, everyone!".to_string()
        } else if turns == 1 {
            glm_call("Who am I?", "hostname", &[])
        } else {
            glm_call("Greet.", "greet", &[("text", "hi all")])
        };
        let stop = if reply.contains("<tool_call>") {
            "<|observation|>"
        } else {
            "<|user|>"
        };
        let mut ids = e.tok.encode(reply.as_str(), false).unwrap().get_ids().to_vec();
        ids.push(e.token_id(stop).unwrap());
        Ok(GenReply {
            ids,
            cut_off: false,
            ..Default::default()
        })
    }));
    let mut rt = if restore {
        Runtime::restore(cfg, enc, model).unwrap()
    } else {
        Runtime::new(cfg, enc, model)
    };
    for m in ["m0", "m1"] {
        let host = format!("host-{m}");
        rt.bind_machine(
            m,
            Arc::new(move |_: &affordance::runtime::CallContext, _: &J| Ok(json!({"output": host, "exit": 0}))),
        );
    }
    if !restore {
        let mut spec = InstanceSpec::new("root", json!({"names": ["alice", "bob"]}));
        spec.model = None;
        spec.machines = Some(vec!["m0".into(), "m1".into()]);
        rt.add_instance(spec).unwrap();
    }
    rt.run().unwrap()
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn an_instance_tree_mounts_children_binds_their_machines_and_passes_props_and_callbacks() {
    let dir = std::env::temp_dir().join(format!("aff-tree-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let report = tree_run(&dir, false);
    // "Only I can read a run's journals." The directory a run makes holds what its agents
    // were told, so no other user may enter it.
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&dir).unwrap().permissions().mode();
        assert_eq!(mode & 0o077, 0, "the run directory's mode is {:o}", mode & 0o777);
    }
    let by = |id: &str| {
        report.instances.iter().find(|a| a.id == id).unwrap_or_else(|| {
            panic!(
                "no {id}: {:?}",
                report.instances.iter().map(|a| &a.id).collect::<Vec<_>>()
            )
        })
    };
    let (root_, alice, bob) = (by("root"), by("root/alice"), by("root/bob"));
    for a in [root_, alice, bob] {
        assert!(a.error.is_none(), "{}: {:?}", a.id, a.error);
    }
    assert_eq!(root_.metrics.turns, 0, "the root has no model");
    // Each child reached only the machine its parent handed it.
    let host_of = |a: &affordance::runtime::InstanceReport| {
        affordance::scenario::log_records(&a.state)
            .iter()
            .find(|r| r["name"] == "hostname")
            .map(|r| r["result"].clone())
            .unwrap()
    };
    assert_eq!((host_of(alice), host_of(bob)), (json!("host-m0"), json!("host-m1")));
    // Callbacks up: the root owns both greetings, written by the polished render.
    let ui = &root_.state["__ui"];
    let mut greetings: Vec<String> = ui["@greetings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g.as_str().unwrap().to_string())
        .collect();
    // The harness told the root each child stopped, with its stop value.
    let mut stopped: Vec<&str> = ui["@stopped"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g.as_str().unwrap())
        .collect();
    stopped.sort();
    assert_eq!(stopped, ["alice: greeted", "bob: greeted"]);
    greetings.sort();
    assert_eq!(greetings, ["alice: Hello, everyone!", "bob: Hello, everyone!"]);
    // Props down: both children's props now hold both greetings.
    for a in [alice, bob] {
        assert_eq!(a.stopped.as_ref().unwrap(), "greeted");
        let mut heard: Vec<&str> = a.state["heard"]
            .as_array()
            .unwrap()
            .iter()
            .map(|g| g.as_str().unwrap())
            .collect();
        heard.sort();
        assert_eq!(heard, greetings, "{} heard", a.id);
        assert!(a.state["onGreet"]["__parent"] == "onGreet");
    }
    // The tree resumes from its journals: the root replays, then each child replays its own.
    let again = tree_run(&dir, true);
    for id in ["root", "root/alice", "root/bob"] {
        let (a, b) = (by(id), again.instances.iter().find(|x| x.id == id).unwrap());
        assert!(b.error.is_none() && b.diverged_at.is_none(), "{id}: {:?}", b.error);
        assert_eq!(b.state, a.state, "{id}");
    }
}

/// The research ladder (views/demo/research) on a scripted chat model: the root mounts an agent
/// per policy, each searches the docs corpus on its own machine, compacts (a model(<Handoff/>)
/// render, on the layout event) when a prompt passes `trig` of the window (the binding's
/// context less max_new), answers and submits; the root grades the answers.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn the_research_ladder_runs_its_policies_and_grades_answers() {
    let dir = std::env::temp_dir().join(format!("aff-research-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let enc = encoder();
    let mut cfg = RuntimeConfig::new(root().join("views/demo/research/ladder.tsx"));
    cfg.workers = 1;
    cfg.dir = Some(dir.join("run"));
    cfg.env.insert(
        "PATH".into(),
        format!(
            "{}:/usr/local/bin:/usr/bin:/bin",
            root().join("tools/examples/search").display()
        ),
    );
    cfg.env
        .insert("BCP_CORPUS".into(), root().join("docs").display().to_string());
    let e = enc.clone();
    let model = Arc::new(FnModel(move |req: &GenRequest| {
        let text = e.tok.decode(&req.ids, false).unwrap();
        let doc: J = serde_json::from_str(text.lines().next().unwrap()).unwrap();
        let msgs = doc["messages"].as_array().unwrap();
        let call = |cmd: &str| json!({"reasoning": "", "content": "THOUGHT: next step.", "tool_calls": [{"name": "bash", "arguments": {"command": cmd}}]});
        let reply = if msgs[0]["content"].as_str().unwrap().contains("CONTEXT CHECKPOINT") {
            json!({"content": "Searched for the spill size; doc on blobs found. Next: write the answer.", "tool_calls": []})
        } else {
            // Decide from what the context shows, as a model would: a fold's summary or two
            // searches means the answer is known; a written answer means submit.
            let calls: Vec<String> = msgs
                .iter()
                .filter(|m| m["role"] == "assistant")
                .flat_map(|m| m["tool_calls"].as_array().cloned().unwrap_or_default())
                .map(|c| c["function"]["arguments"].to_string())
                .collect();
            let summarized = msgs.iter().any(|m| {
                m["content"]
                    .as_str()
                    .is_some_and(|c| c.contains("Another language model"))
            });
            if calls.last().is_some_and(|c| c.contains("printf")) {
                call("echo COMPLETE_TASK_AND_SUBMIT_FINAL_OUTPUT")
            } else if summarized || calls.len() >= 2 {
                call(
                    "printf 'Explanation: the runtime docs [1].\\nExact Answer: 64 KiB\\nConfidence: 90%%\\n' > ~/answer.txt",
                )
            } else if calls.len() == 1 {
                call("bcp_search \"64 KiB spill\" | head -40")
            } else {
                call("bcp_search \"spill_bytes default strings blobs\"")
            }
        };
        Ok(GenReply {
            ids: e
                .tok
                .encode(reply.to_string().as_str(), false)
                .unwrap()
                .get_ids()
                .to_vec(),
            cut_off: false,
            ..Default::default()
        })
    }));
    // A 1,500-token window once the reply's room (max_new 2048) is out: "none" fits its whole
    // run, "compact" passes 85% of it and summarizes.
    let model = affordance::runtime::model::with_context(model, 2048 + 1500);
    let mut rt = Runtime::new(cfg, enc, model);
    for n in 0..2 {
        rt.bind_machine(
            &format!("m{n}"),
            Arc::new(Exec {
                target: affordance::runtime::Target::Local {
                    home: Some(dir.join("homes").display().to_string()),
                    per_instance: true,
                },
                default_timeout_s: 30,
                max_capture: 1 << 20,
            }),
        );
    }
    let mut spec = InstanceSpec::new(
        "root",
        json!({"questions": [{"question": "What is the default blob spill size?", "expect": ["64"]}], "policies": ["none", "compact"]}),
    );
    spec.model = None;
    spec.max_turns = Some(12);
    rt.add_instance(spec).unwrap();
    let report = rt.run().unwrap();
    for a in &report.instances {
        assert!(a.error.is_none(), "{}: {:?}", a.id, a.error);
    }
    let root_ = report.instances.iter().find(|a| a.id == "root").unwrap();
    let results = &root_.state["__ui"]["@results"];
    for key in ["q0-none", "q0-compact"] {
        assert_eq!(results[key]["correct"], true, "{key}: {results}");
        assert_eq!(results[key]["exact"], "64 KiB");
    }
    // The compact run folded: its handoff summary is in its state, written by a render.
    let compact = report.instances.iter().find(|a| a.id == "root/q0-compact").unwrap();
    let fold = &compact.state["__ui"]["@fold"];
    assert!(fold["summary"].as_str().unwrap().contains("spill size"), "{fold}");
    // Search ran on the machine: the first tool result lists docids.
    assert!(
        affordance::scenario::log_records(&compact.state)
            .iter()
            .any(|r| r["kind"] == "action" && r["result"].as_str().is_some_and(|s| s.contains("docid:")))
    );
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn send_completes_when_the_receiver_has_journaled_and_a_resend_is_taken_once() {
    let dir = std::env::temp_dir().join(format!("aff-send-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let enc = encoder();
    let mut cfg = RuntimeConfig::new(root().join("views/demo/runtime/messenger.tsx"));
    cfg.workers = 2;
    cfg.dir = Some(dir.clone());
    let model = scripted(enc.clone(), 0, |agent, n| match (agent, n) {
        ("a", 1) => glm_call("", "send", &[("to", "b")]),
        ("a", 2) => glm_call("", "send", &[("to", "ghost")]),
        _ => "Done.</think>ok".into(),
    });
    let mut rt = Runtime::new(cfg, enc.clone(), model);
    for id in ["a", "b"] {
        let mut s = InstanceSpec::new(id, json!({"task": "Talk."}));
        s.max_turns = Some(3);
        rt.add_instance(s).unwrap();
    }
    let report = rt.run().unwrap();
    let a = report.instances.iter().find(|x| x.id == "a").unwrap();
    let results: Vec<J> = affordance::scenario::log_records(&a.state)
        .iter()
        .filter(|r| r["kind"] == "action")
        .map(|r| serde_json::from_str(r["result"].as_str().unwrap()).unwrap())
        .collect();
    assert_eq!(
        results,
        [
            json!({"to": "b", "delivered": true}),
            json!({"to": "ghost", "delivered": false})
        ]
    );
    // b journaled the message (with its id) before a's send completed.
    // The trace places each tool on the run's timeline under the model call that asked for it:
    // tool (`turn`) -> reply and render (`turn`) -> render (`task`) -> generate (`task`), every
    // record with its time (`at_ms`, the journal's clock), and a tool's start after its reply.
    let trace: Vec<J> = std::fs::read_to_string(crate::shared::inst(&dir, "a/trace.jsonl"))
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert!(trace.iter().all(|e| e["at_ms"].is_i64()), "every record has its time");
    // The start records the logical clock against wall time (a fresh run's clock is wall time).
    let clock = trace.iter().find(|e| e["t"] == "clock").expect("a clock record");
    assert!(
        (clock["wall_ms"].as_i64().unwrap() - clock["at_ms"].as_i64().unwrap()).abs() < 5_000,
        "{clock}"
    );
    let find = |t: &str, key: &str, value: &J| {
        trace
            .iter()
            .find(|e| e["t"] == t && &e[key] == value)
            .unwrap_or_else(|| panic!("{t} with {key}={value}"))
            .clone()
    };
    let tool = trace.iter().find(|e| e["t"] == "tool" && e["tool"] == "send").unwrap();
    let (reply, render) = (
        find("reply", "turn", &tool["turn"]),
        find("render", "turn", &tool["turn"]),
    );
    let generate = find("generate", "task", &render["task"]);
    let at = |e: &J| e["at_ms"].as_i64().unwrap();
    assert!(
        at(&render) <= at(&generate) && at(&generate) <= at(&reply) && at(&reply) <= at(tool),
        "{render} {generate} {reply} {tool}"
    );
    let b_journal = std::fs::read_to_string(crate::shared::inst(&dir, "b/journal.jsonl")).unwrap();
    let msg = b_journal
        .lines()
        .find(|l| l.contains("\"name\":\"message\""))
        .expect("b journaled the message");
    // The message's delivery id is on the journaled event, not in the app's payload.
    let parsed: J = serde_json::from_str(msg).unwrap();
    assert!(parsed["id"].as_str().is_some_and(|id| id.starts_with("a:")), "{msg}");
    assert!(parsed["payload"].get("_msg").is_none(), "{msg}");
    let a_journal = std::fs::read_to_string(crate::shared::inst(&dir, "a/journal.jsonl")).unwrap();
    let ack = a_journal.lines().find(|l| l.contains("\"delivered\":true")).unwrap();
    let clock = |l: &str| serde_json::from_str::<J>(l).unwrap()["clock_ms"].as_i64().unwrap();
    assert!(clock(msg) <= clock(ack));
    // The same message again (a resend after a crash) is journaled but not taken twice.
    let event: Input = serde_json::from_str(msg).unwrap();
    let v = View::load(&root().join("views/demo/runtime/messenger.tsx"), "Main", &enc).unwrap();
    let mut b = Instance::new(config("b", "Talk.")).unwrap();
    b.start(&v, &enc).unwrap();
    b.handle(&v, &enc, event.clone()).unwrap();
    b.handle(&v, &enc, event).unwrap();
    let dups = b
        .take_trace()
        .into_iter()
        .filter(|t| t["t"] == "duplicate_message")
        .count();
    assert_eq!(dups, 1, "the second copy is recognized and dropped");
    // An event from outside whose payload happens to hold `_msg` is an ordinary event: both
    // copies are taken.
    let outside = Input::Event {
        name: "message".into(),
        payload: json!({"_msg": "a:1", "text": "hello"}),
        id: None,
        clock_ms: 0,
    };
    b.handle(&v, &enc, outside.clone()).unwrap();
    b.handle(&v, &enc, outside).unwrap();
    assert!(
        !b.take_trace().iter().any(|t| t["t"] == "duplicate_message"),
        "a payload's own fields never make an event a duplicate"
    );
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn a_prompt_over_the_hard_ceiling_is_never_sent() {
    let enc = encoder();
    let v = view(&enc);
    let mut host = Host::new(&enc, &v, |_, _| glm_call("Go.", "wait", &[("seconds", "1")]));
    host.add(InstanceConfig {
        context: None,
        retry: Default::default(),
        max_prompt: Some(50),
        ..config("a", "A task.")
    });
    host.run();
    assert!(host.prompts.is_empty());
    assert_eq!(host.instances["a"].stopped().unwrap()["reason"], "prompt_over_limit");
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn a_mount_boundary_takes_one_component_with_json_props_and_callbacks() {
    let enc = encoder();
    let dir = std::env::temp_dir().join(format!("aff-mount-rules-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("child.tsx"),
        "export function Child({ n }) { return <text>{String(n)}</text>; }\n",
    )
    .unwrap();
    let render = |body: &str| -> Result<affordance::Rendered, String> {
        std::fs::write(
            dir.join("main.tsx"),
            format!("import {{ Child }} from \"./child.tsx\";\nexport default function Main() {{ return {body}; }}\n"),
        )
        .unwrap();
        let v = View::load(&dir.join("main.tsx"), "Main", &enc)?;
        v.render(
            &enc,
            affordance::value::state(&json!({})),
            &mut affordance::emit::RunCache::new(false),
            chrono::NaiveDate::from_ymd_opt(2026, 10, 16)
                .unwrap()
                .and_hms_opt(12, 0, 0)
                .unwrap(),
        )
    };
    let err = |body: &str| match render(body) {
        Err(e) => e,
        Ok(_) => panic!("rendered: {body}"),
    };
    // The child is recorded, not rendered: no tokens, one mount span naming its file and component.
    let r = render(r#"<Mount key="a"><Child n={3} onDone={x => x}/></Mount>"#).unwrap();
    assert!(r.ids.is_empty());
    let m = r.spans.iter().find(|s| s.label == "mount").unwrap();
    assert_eq!(
        (
            m.meta["entry"].as_str(),
            m.meta["props"].clone(),
            m.meta["callbacks"].clone()
        ),
        (Some("Child"), json!({"n": 3}), json!(["onDone"]))
    );
    assert!(m.meta["app"].as_str().unwrap().ends_with("child.tsx"));
    assert!(err(r#"<Mount key="a"><Child n={1}/><Child n={2}/></Mount>"#).contains("exactly one component element"));
    assert!(err(r#"<Mount key="a">text</Mount>"#).contains("exactly one component element"));
    assert!(err(r#"<Mount key="a"><Nope/></Mount>"#).contains("unknown component <Nope>"));
    assert!(err(r#"<Mount key="a"><Child n={<text>x</text>}/></Mount>"#).contains("prop n of <Child> is markup"));
    assert!(err(r#"<Mount key="a"><Child n={x => x}/></Mount>"#).contains("prop n of <Child> is a function"));
    assert!(err(r#"<Mount key="a"><Child n={[/re/]}/></Mount>"#).contains("is a regular expression"));
    assert!(err(r#"<Mount key="a"><Child n={1}>inside</Child></Mount>"#).contains("takes no children"));
}

/// Context folding (R5): a branch's turns leave the prompt when it returns; its call and summary stay.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn a_returned_branch_folds_to_its_call_and_summary() {
    let dir = std::env::temp_dir().join(format!("aff-fold-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let enc = encoder();
    let mut cfg = RuntimeConfig::new(root().join("views/demo/research/ladder.tsx"));
    cfg.workers = 1;
    cfg.dir = Some(dir.join("run"));
    cfg.env.insert(
        "PATH".into(),
        format!(
            "{}:/usr/local/bin:/usr/bin:/bin",
            root().join("tools/examples/search").display()
        ),
    );
    cfg.env
        .insert("BCP_CORPUS".into(), root().join("docs").display().to_string());
    let prompts = Arc::new(std::sync::Mutex::new(vec![]));
    let (e, seen) = (enc.clone(), prompts.clone());
    let model = Arc::new(FnModel(move |req: &GenRequest| {
        let text = e.tok.decode(&req.ids, false).unwrap();
        seen.lock().unwrap().push(text.clone());
        let doc: J = serde_json::from_str(text.lines().next().unwrap()).unwrap();
        let n = doc["messages"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|m| m["role"] == "assistant")
            .count();
        let call = |name: &str, args: J| json!({"content": "THOUGHT: next.", "tool_calls": [{"name": name, "arguments": args}]});
        let reply = match n {
            0 => call("branch", json!({"description": "find the spill size"})),
            1 => call(
                "bash",
                json!({"command": "bcp_search \"spill_bytes default\" | head -30"}),
            ),
            2 => call(
                "return",
                json!({"summary": "The default spill size is 64 KiB (RUNTIME.md)."}),
            ),
            3 => call(
                "bash",
                json!({"command": "printf 'Explanation: [1]\\nExact Answer: 64 KiB\\nConfidence: 90%%\\n' > ~/answer.txt"}),
            ),
            _ => call("bash", json!({"command": "echo COMPLETE_TASK_AND_SUBMIT_FINAL_OUTPUT"})),
        };
        Ok(GenReply {
            ids: e
                .tok
                .encode(reply.to_string().as_str(), false)
                .unwrap()
                .get_ids()
                .to_vec(),
            cut_off: false,
            ..Default::default()
        })
    }));
    let mut rt = Runtime::new(cfg, enc, model);
    rt.bind_machine(
        "m0",
        Arc::new(Exec {
            target: affordance::runtime::Target::Local {
                home: Some(dir.join("homes").display().to_string()),
                per_instance: true,
            },
            default_timeout_s: 30,
            max_capture: 1 << 20,
        }),
    );
    let mut spec = InstanceSpec::new(
        "root",
        json!({"questions": [{"question": "What is the default blob spill size?", "expect": ["64"]}], "policies": ["fold"]}),
    );
    spec.model = None;
    spec.max_turns = Some(10);
    rt.add_instance(spec).unwrap();
    let report = rt.run().unwrap();
    let root_ = report.instances.iter().find(|a| a.id == "root").unwrap();
    assert_eq!(
        root_.state["__ui"]["@results"]["q0-fold"]["correct"], true,
        "{}",
        root_.state
    );
    let prompts = prompts.lock().unwrap();
    let (inside, after) = (&prompts[2], &prompts[3]);
    assert!(
        inside.contains("IN BRANCH: find the spill size") && inside.contains("docid:"),
        "the branch sees its own work"
    );
    assert!(after.contains("[branch finished: find the spill size]") && after.contains("64 KiB (RUNTIME.md)"));
    assert!(!after.contains("docid:"), "the branch's search output is folded away");
}

/// Permanent memory (views/demo/memory, after OptMem) on a scripted chat model: three sessions
/// in turn under one root. The first two note facts and filler; each note that completes a block
/// of the memory's tree is compressed by the root (harness.render). The third wakes with older
/// memories collapsed into summaries, finds the fact with `recall` and answers from it.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn a_memory_outlives_its_sessions_compresses_old_notes_and_recalls_them() {
    let dir = std::env::temp_dir().join(format!("aff-memory-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let enc = encoder();
    let mut cfg = RuntimeConfig::new(root().join("views/demo/memory/memory.tsx"));
    cfg.workers = 1;
    cfg.dir = Some(dir.clone());
    let e = enc.clone();
    let model = Arc::new(FnModel(move |req: &GenRequest| {
        let text = e.tok.decode(&req.ids, false).unwrap();
        let doc: J = serde_json::from_str(text.lines().next().unwrap()).unwrap();
        let msgs = doc["messages"].as_array().unwrap();
        let first = msgs.iter().find(|m| m["role"] == "user").unwrap()["content"]
            .as_str()
            .unwrap()
            .to_string();
        let call = |name: &str, args: J| json!({"content": "", "tool_calls": [{"name": name, "arguments": args}]});
        let reply = if first.starts_with("Summarize memories") {
            // A summary keeps the facts' words: the memory lines without their numbers.
            let lines: Vec<String> = first
                .lines()
                .filter(|l| l.starts_with("  #"))
                .map(|l| l.trim().split_once(' ').map(|x| x.1).unwrap_or("").to_string())
                .collect();
            json!({"content": lines.join("; "), "tool_calls": []})
        } else {
            let n = msgs.iter().filter(|m| m["role"] == "assistant").count();
            let last_tool = msgs
                .iter()
                .rev()
                .find(|m| m["role"] == "tool")
                .map(|m| m["content"].as_str().unwrap_or("").to_string());
            if first.contains("Remember these facts") {
                match n {
                    0 => call("note", json!({"text": "deploy key is K-7731"})),
                    1 => call("note", json!({"text": "staging host is st-4"})),
                    _ => call("answer", json!({"text": "remembered"})),
                }
            } else if first.contains("Log observations") {
                if n < 6 {
                    call("note", json!({"text": format!("observation {n}: build green")}))
                } else {
                    call("answer", json!({"text": "logged"}))
                }
            } else if n == 0 {
                call("recall", json!({"pattern": "deploy key"}))
            } else {
                call("answer", json!({"text": last_tool.unwrap_or_default()}))
            }
        };
        Ok(GenReply {
            ids: e
                .tok
                .encode(reply.to_string().as_str(), false)
                .unwrap()
                .get_ids()
                .to_vec(),
            cut_off: false,
            ..Default::default()
        })
    }));
    let mut rt = Runtime::new(cfg, enc, model);
    let sessions = [
        "Remember these facts for later.",
        "Log observations of the build.",
        "What is the deploy key?",
    ];
    let spec = InstanceSpec::new("root", json!({"sessions": sessions, "wake_lines": 4}));
    rt.add_instance(spec).unwrap();
    let report = rt.run().unwrap();
    for a in &report.instances {
        assert!(a.error.is_none(), "{}: {:?}", a.id, a.error);
    }
    let by = |id: &str| {
        report
            .instances
            .iter()
            .find(|a| a.id == id)
            .unwrap_or_else(|| panic!("no {id}"))
    };
    let ui = &by("root").state["__ui"];
    assert_eq!(ui["@log"].as_array().unwrap().len(), 8);
    assert_eq!(ui["@current"], 3, "every session ran");
    // Every complete block of the 8 memories was summarized, from its halves.
    let tree = ui["@tree"].as_object().unwrap();
    let mut keys: Vec<&str> = tree.keys().map(|k| k.as_str()).collect();
    keys.sort();
    assert_eq!(keys, ["0-2", "0-4", "0-8", "2-4", "4-6", "4-8", "6-8"]);
    assert!(tree["0-2"].as_str().unwrap().contains("K-7731"), "{tree:?}");
    // The last session woke with the facts collapsed into a summary, and recalled them word for word.
    let last = by("root/session-2");
    let wake = last.state["__ui"]["@woke"].as_str().unwrap();
    assert!(wake.lines().count() <= 4 && wake.starts_with("#0-"), "{wake}");
    // A session wakes once: the notes it makes change its parent's memory, not its first message.
    assert_eq!(
        by("root/session-1").state["__ui"]["@woke"],
        "#0 deploy key is K-7731\n#1 staging host is st-4"
    );
    assert!(
        last.stopped
            .as_ref()
            .unwrap()
            .as_str()
            .unwrap()
            .contains("#0 deploy key is K-7731"),
        "{:?}",
        last.stopped
    );
}

/// The Recursive LM (views/demo/rlm) on a scripted chat model and a real Python REPL: the root
/// model inspects `context` with code, then chunks it and classifies every chunk with one
/// `llm_batch`; the harness answers the batch with parallel sub-LLM renders and resumes the cell,
/// which counts and sets `answer`. The bench grades the count against the generator's.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn a_recursive_lm_works_on_its_context_in_a_repl_and_batches_sub_llm_calls() {
    let py = "python3";
    if std::process::Command::new(py).arg("--version").output().is_err() {
        eprintln!("{py} not installed: skipped");
        return;
    }
    let dir = std::env::temp_dir().join(format!("aff-rlm-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let out = std::process::Command::new(py)
        .arg(root().join("tools/fixtures/make_rlm_tasks.py"))
        .args(["7", "300"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let tasks: J = serde_json::from_slice(&out.stdout).unwrap();
    let task = tasks[0].clone();
    let enc = encoder();
    let mut cfg = RuntimeConfig::new(root().join("views/demo/rlm/rlm.tsx"));
    cfg.workers = 1;
    cfg.dir = Some(dir.join("run"));
    cfg.env.insert(
        "PATH".into(),
        format!(
            "{}:/usr/local/bin:/usr/bin:/bin",
            root().join("tools/examples/rlm").display()
        ),
    );
    let e = enc.clone();
    let model = Arc::new(FnModel(move |req: &GenRequest| {
        let text = e.tok.decode(&req.ids, false).unwrap();
        let doc: J = serde_json::from_str(text.lines().next().unwrap()).unwrap();
        let msgs = doc["messages"].as_array().unwrap();
        let python =
            |code: &str| json!({"content": "", "tool_calls": [{"name": "python", "arguments": {"code": code}}]});
        let reply = if msgs[0]["content"]
            .as_str()
            .unwrap()
            .starts_with("You are a careful assistant")
        {
            // A sub-LLM: one category per message line, read from its words.
            let prompt = msgs[1]["content"].as_str().unwrap();
            if prompt == "FAIL" {
                return Err("HTTP 400 bad request".into());
            }
            let cats: Vec<&str> = prompt
                .lines()
                .filter(|l| l.contains("Message:"))
                .map(|l| {
                    let l = l.to_lowercase();
                    if [
                        "stopped working",
                        "broken",
                        "disappointed",
                        "on hold",
                        "charged twice",
                        "crashing",
                        "reboots every hour",
                        "unacceptable",
                        "thanks for nothing",
                    ]
                    .iter()
                    .any(|k| l.contains(k))
                    {
                        "complaint"
                    } else if l.trim_end().ends_with('?') {
                        "question"
                    } else if ["please", "could you", "i'd like", "kindly"]
                        .iter()
                        .any(|k| l.contains(k))
                    {
                        "request"
                    } else {
                        "praise"
                    }
                })
                .collect();
            json!({"content": cats.join("\n"), "tool_calls": []})
        } else {
            match msgs.iter().filter(|m| m["role"] == "assistant").count() {
                0 => python(
                    "print(type(context).__name__, len(context))\nprint(context[:300])\nprint(llm_query(\"FAIL\"))",
                ),
                _ => python(
                    "lines = context.split(\"\\n\")\nchunks = [\"\\n\".join(lines[i:i+50]) for i in range(0, len(lines), 50)]\nreplies = llm_batch([\"Classify each message below as complaint, praise, question or request. Reply with one category per line, in order.\\n\\n\" + c for c in chunks])\nn = sum(r.split().count(\"complaint\") for r in replies)\nanswer[\"content\"] = str(n)\nanswer[\"ready\"] = True\nprint(len(chunks), n)",
                ),
            }
        };
        Ok(GenReply {
            ids: e
                .tok
                .encode(reply.to_string().as_str(), false)
                .unwrap()
                .get_ids()
                .to_vec(),
            cut_off: false,
            ..Default::default()
        })
    }));
    let mut rt = Runtime::new(cfg, enc, model);
    rt.bind_machine(
        "m0",
        Arc::new(Exec {
            target: affordance::runtime::Target::Local {
                home: Some(dir.join("homes").display().to_string()),
                per_instance: true,
            },
            default_timeout_s: 60,
            max_capture: 1 << 20,
        }),
    );
    let mut spec = InstanceSpec::new("root", json!({"tasks": [task]}));
    spec.model = None;
    spec.max_turns = Some(6);
    spec.machines = Some(vec!["m0".into()]);
    rt.add_instance(spec).unwrap();
    let report = rt.run().unwrap();
    // The REPL daemon outlives the run (it belongs to the machine): stop it.
    for home in std::fs::read_dir(dir.join("homes")).unwrap().flatten() {
        let _ = std::process::Command::new(root().join("tools/examples/rlm/rlm_repl"))
            .arg("stop")
            .env("HOME", home.path())
            .status();
    }
    for a in &report.instances {
        assert!(a.error.is_none(), "{}: {:?}", a.id, a.error);
    }
    let id = task["id"].as_str().unwrap();
    let rlm = report
        .instances
        .iter()
        .find(|a| a.id == format!("root/{id}-rlm"))
        .unwrap();
    // The first cell's output is what the model saw of the context: its type and size, not the text.
    let first = affordance::scenario::log_records(&rlm.state)
        .iter()
        .find(|r| r["kind"] == "action")
        .unwrap()["result"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(first.starts_with("str "), "{first}");
    assert!(
        first.contains("[sub-LLM error: "),
        "a failed sub-LLM call is an error the code sees: {first}"
    );
    let results = &report.instances.iter().find(|a| a.id == "root").unwrap().state["__ui"]["@results"];
    let r = &results[format!("{id}-rlm")];
    assert_eq!(r["correct"], true, "{results}");
    assert_eq!(
        r["subcalls"], 7,
        "the failed one, then 300 lines in chunks of 50: one batch of six"
    );
}

/// A long run keeps store payloads on disk: outputs over the
/// inline cap stay in the store file, `read` returns a stored output word for word, and a restore
/// (streamed from the journal) rebuilds the same agent. The app's log is its own state, bounded
/// by its own policy (`log.release`, views/lib/log.tsx).
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn a_long_run_keeps_store_payloads_on_disk_and_resumes_exactly() {
    fn run(dir: &std::path::Path, restore: bool) -> affordance::runtime::Report {
        let enc = encoder();
        let mut cfg = RuntimeConfig::new(root().join("views/agents/session.tsx"));
        cfg.workers = 1;
        cfg.dir = Some(dir.to_path_buf());
        let e = enc.clone();
        let model = Arc::new(FnModel(move |req: &GenRequest| {
            let n = req
                .ids
                .iter()
                .filter(|&&t| t == e.token_id("<|assistant|>").unwrap())
                .count();
            // Every fifth turn reads back an output the layout clipped, by the id its marker names.
            let prompt = e.tok.decode(&req.ids, false).unwrap();
            let clipped = prompt
                .match_indices("read_event(\"")
                .last()
                .map(|(at, m)| prompt[at + m.len()..].split('"').next().unwrap().to_string());
            let text = match clipped {
                Some(id) if n % 5 == 4 => glm_call("Read it.", "read_event", &[("id", &id)]),
                _ => glm_call("Next.", "shell", &[("command", &format!("run step {n}"))]),
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
            affordance::runtime::model::ModelTemplate::new("glm53", json!({"keep_replies": true})).unwrap(),
        )
        .unwrap();
        let mut rt = if restore {
            Runtime::restore(cfg, enc, model).unwrap()
        } else {
            Runtime::new(cfg, enc, model)
        };
        // Outputs over the layout's cap (session.tss) are clipped in the prompt: each turn writes a distinct one.
        rt.bind_machine(
            "home",
            Arc::new(|_: &affordance::runtime::CallContext, args: &J| {
                let cmd = args["cmd"].as_str().unwrap_or("").to_string();
                Ok(json!({"output": format!("{cmd}\n").repeat(1200), "exit": 0}))
            }),
        );
        if !restore {
            let mut spec = InstanceSpec::new("a", json!({"id": "a", "n_agents": 1, "task": "Keep the build green."}));
            spec.max_turns = Some(40);
            rt.add_instance(spec).unwrap();
        }
        rt.run().unwrap()
    }
    let base = std::env::temp_dir().join(format!("aff-resident-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let first = run(&base, false);
    let p = &first.instances[0];
    assert!(p.error.is_none(), "{:?}", p.error);
    assert_eq!(p.metrics.turns, 40);
    // A clipped output read back is the output, word for word.
    let records = affordance::scenario::log_records(&p.state);
    let read = records.iter().find(|r| r["name"] == "read_event").expect("a read ran");
    assert!(
        read["result"].as_str().unwrap().contains(&"run step 3\n".repeat(50)),
        "{}",
        read["result"]
    );
    // Resume streams the journal and rebuilds the same agent.
    let again = run(&base, true);
    assert_eq!(affordance::scenario::log_records(&again.instances[0].state), records);
    assert_eq!(again.instances[0].diverged_at, None);
}

/// A machine that brings its own executor (`command`): the call's JSON goes in on stdin, its
/// result comes out on stdout; a failing command is an error.
#[test]
fn a_command_machine_runs_its_program_per_call() {
    use affordance::runtime::{CallContext, CommandMachine, Executor};
    let ctx = CallContext {
        agent: "a".into(),
        instance: String::new(),
        task: 7,
        env: BTreeMap::from([("K".into(), "v".into())]),
        cancel: Default::default(),
        blobs: None,
        output: None,
    };
    let echo = CommandMachine::new(vec!["sh".into(), "-c".into(), "cat".into()]);
    let r = echo.call(&ctx, &json!({"cmd": "ls"})).unwrap();
    assert_eq!(
        r,
        json!({"agent": "a", "task": 7, "env": {"K": "v"}, "args": {"cmd": "ls"}})
    );
    let fails = CommandMachine::new(vec!["sh".into(), "-c".into(), "echo nope >&2; exit 3".into()]);
    assert!(fails.call(&ctx, &json!({})).unwrap_err().contains("nope"));
}

/// A command's reply is read up to `max_reply` bytes, not to its end: one that prints 64 MiB
/// (an agent `cat`-ing a binary) fails the call and is stopped; its stderr is bounded too.
#[test]
fn a_command_reply_past_its_bound_fails_the_call() {
    use affordance::runtime::{CallContext, CommandMachine, Executor};
    let ctx = CallContext {
        agent: "a".into(),
        instance: String::new(),
        task: 1,
        env: BTreeMap::new(),
        cancel: Default::default(),
        blobs: None,
        output: None,
    };
    let mut flood = CommandMachine::new(vec![
        "sh".into(),
        "-c".into(),
        "head -c 67108864 /dev/zero; head -c 67108864 /dev/zero >&2; sleep 30".into(),
    ]);
    flood.max_reply = 1 << 20;
    let started = std::time::Instant::now();
    let error = flood.call(&ctx, &json!({})).unwrap_err();
    assert!(error.contains("result exceeds 1048576 bytes"), "{error}");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(20),
        "the command was not stopped"
    );
    let noisy = CommandMachine::new(vec![
        "sh".into(),
        "-c".into(),
        "head -c 67108864 /dev/zero | tr '\\0' x >&2; exit 3".into(),
    ]);
    let error = noisy.call(&ctx, &json!({})).unwrap_err();
    assert!(
        error.len() < 128 << 10 && error.contains("xxxx"),
        "{} bytes",
        error.len()
    );
}

fn command_fixture(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("aff-command-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn wait_for(path: &std::path::Path) {
    let started = std::time::Instant::now();
    while !path.exists() {
        assert!(started.elapsed() < std::time::Duration::from_secs(5));
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

/// Cancelling a command call returns at once; the command gets SIGTERM and can clean up.
#[test]
fn a_cancelled_command_call_lets_the_command_clean_up() {
    use affordance::runtime::{CallContext, CancelToken, CommandMachine, Executor};
    let py = "python3";
    if std::process::Command::new(py).arg("--version").output().is_err() {
        return;
    }
    let dir = command_fixture("term");
    let (ready, cleaned, script) = (dir.join("ready"), dir.join("cleaned"), dir.join("helper.py"));
    std::fs::write(&script, "import json,signal,sys,time,pathlib\njson.load(sys.stdin)\ndef stop(signum,frame):\n pathlib.Path(sys.argv[2]).write_text('aborted')\n raise SystemExit(0)\nsignal.signal(signal.SIGTERM,stop)\npathlib.Path(sys.argv[1]).write_text('ready')\nwhile True: time.sleep(.05)\n").unwrap();
    let m = CommandMachine::new(vec![
        py.into(),
        script.display().to_string(),
        ready.display().to_string(),
        cleaned.display().to_string(),
    ]);
    let cancel = CancelToken::new();
    let ctx = CallContext {
        agent: "a".into(),
        instance: String::new(),
        task: 0,
        env: BTreeMap::new(),
        cancel: cancel.clone(),
        blobs: None,
        output: None,
    };
    let call = std::thread::spawn(move || m.call(&ctx, &json!({})));
    wait_for(&ready);
    let started = std::time::Instant::now();
    cancel.cancel("abandoned");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(1),
        "cancel does not wait"
    );
    assert!(call.join().unwrap().unwrap_err().contains("cancelled"));
    wait_for(&cleaned);
    assert_eq!(std::fs::read_to_string(&cleaned).unwrap(), "aborted");
    std::fs::remove_dir_all(dir).unwrap();
}

/// A descendant that ignores SIGTERM is killed with the command's process group.
#[cfg(target_os = "linux")] // reads the descendant's state in `/proc`
#[test]
fn a_cancelled_command_call_reaps_a_stubborn_descendant() {
    use affordance::runtime::{CallContext, CancelToken, CommandMachine, Executor};
    let py = "python3";
    if std::process::Command::new(py).arg("--version").output().is_err() {
        return;
    }
    let dir = command_fixture("group");
    let (ready, script) = (dir.join("ready"), dir.join("helper.py"));
    std::fs::write(&script, "import json,signal,subprocess,sys,time,pathlib\njson.load(sys.stdin)\nsignal.signal(signal.SIGTERM,lambda *_: sys.exit(0))\nchild=subprocess.Popen([sys.executable,'-c','import signal,time; signal.signal(signal.SIGTERM,signal.SIG_IGN); print(\"ready\",flush=True); time.sleep(60)'],stdout=subprocess.PIPE)\nchild.stdout.readline()\npathlib.Path(sys.argv[1]).write_text(str(child.pid))\nwhile True: time.sleep(.05)\n").unwrap();
    let m = CommandMachine::new(vec![
        py.into(),
        script.display().to_string(),
        ready.display().to_string(),
    ]);
    let cancel = CancelToken::new();
    let ctx = CallContext {
        agent: "a".into(),
        instance: String::new(),
        task: 0,
        env: BTreeMap::new(),
        cancel: cancel.clone(),
        blobs: None,
        output: None,
    };
    let call = std::thread::spawn(move || m.call(&ctx, &json!({})));
    wait_for(&ready);
    // The helper writes its child's process id there: read once it is all written (the file
    // exists before its content does).
    let asked = std::time::Instant::now();
    let pid: u32 = loop {
        if let Some(pid) = std::fs::read_to_string(&ready)
            .ok()
            .and_then(|text| text.trim().parse().ok())
        {
            break pid;
        }
        assert!(
            asked.elapsed() < std::time::Duration::from_secs(10),
            "the helper wrote no process id"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    };
    cancel.cancel("stop the group");
    assert!(call.join().unwrap().is_err());
    // Gone (or a zombie awaiting its parent) within the grace period plus a margin.
    let alive = || {
        std::fs::read_to_string(format!("/proc/{pid}/stat"))
            .is_ok_and(|s| !s.rsplit_once(") ").unwrap().1.starts_with("Z "))
    };
    let started = std::time::Instant::now();
    while alive() {
        assert!(
            started.elapsed() < std::time::Duration::from_secs(8),
            "the descendant survived"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    std::fs::remove_dir_all(dir).unwrap();
}

/// An agent's `capabilities` limit the platform APIs it may call: without `send`, a send is an
/// error result; machines are bound by `machines`, not by capabilities.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn capabilities_limit_the_platform_apis_an_agent_may_call() {
    let enc = encoder();
    let mut cfg = RuntimeConfig::new(root().join("views/demo/runtime/messenger.tsx"));
    cfg.workers = 1;
    let model = scripted(enc.clone(), 0, |agent, n| match (agent, n) {
        ("a", 1) => glm_call("", "send", &[("to", "b")]),
        _ => "Done.</think>ok".into(),
    });
    let mut rt = Runtime::new(cfg, enc, model);
    for id in ["a", "b"] {
        let mut s = InstanceSpec::new(id, json!({"task": "Talk."}));
        s.max_turns = Some(2);
        if id == "a" {
            s.capabilities = Some(vec!["store.*".into()]);
        }
        rt.add_instance(s).unwrap();
    }
    let report = rt.run().unwrap();
    let a = report.instances.iter().find(|x| x.id == "a").unwrap();
    let result = affordance::scenario::log_records(&a.state)
        .iter()
        .find(|r| r["kind"] == "action")
        .unwrap()["result"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(result.contains("this agent may not call send"), "{result}");
}

/// session.tsx on each model family: its first prompt renders through the chosen template app.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn the_session_app_renders_for_each_template() {
    let ds = Encoder::from_file(
        root()
            .join("ref/tokenizers/deepseek-ai__DeepSeek-V4.1-Flash.json")
            .to_str()
            .unwrap(),
    )
    .unwrap();
    let glm = encoder();
    for (template, enc, marker, options) in [
        (
            "deepseek_v41",
            &ds,
            "DSML",
            json!({"enable_thinking": true, "reasoning_effort": "high"}),
        ),
        ("messages", &glm, "\"messages\"", json!({})),
        ("glm53", &glm, "<|assistant|>", json!({"reasoning_effort": "high"})),
    ] {
        let v = View::load(&root().join("views/agents/session.tsx"), "Main", enc).unwrap();
        let mut a = Instance::new(InstanceConfig {
            context: None,
            retry: Default::default(),
            template: Some(affordance::runtime::model::ModelTemplate::new(template, options).unwrap()),
            props: json!({"id": "a", "task": "Build it."}),
            ..config("a", "Build it.")
        })
        .unwrap();
        let cmds = a.start(&v, enc).unwrap_or_else(|e| panic!("{template}: {e}"));
        let Some(Command::Generate { ids, .. }) = cmds.first() else {
            panic!("{template}: {cmds:?}")
        };
        let text = enc.tok.decode(ids, false).unwrap();
        assert!(
            text.contains(marker) && text.contains("Build it."),
            "{template}: {}",
            &text[..text.len().min(400)]
        );
    }
}

/// A message that arrives while the model is generating has not been seen by the model. A wait
/// its reply then calls wakes at once on it, instead of sleeping until its deadline.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn a_message_during_the_turn_wakes_the_wait_that_turn_chose() {
    let enc = encoder();
    let v = view(&enc);
    let mut agent = Instance::new(config("agent-a", "Wait for b.")).unwrap();
    let mut journal = vec![];
    let Command::Generate { task, .. } = agent.start(&v, &enc).unwrap()[0].clone() else {
        panic!()
    };
    // The message lands during generation; then the reply asks to wait 600 s.
    apply(
        &mut agent,
        &mut journal,
        &v,
        &enc,
        Input::Event {
            name: "message".into(),
            payload: json!({"from": "agent-b", "text": "port is 48213"}),
            id: None,
            clock_ms: T0 + 100,
        },
    );
    let out = apply(
        &mut agent,
        &mut journal,
        &v,
        &enc,
        Input::Reply {
            server: None,
            task,
            ids: reply(&enc, &glm_call("", "wait", &[("seconds", "600")]), "<|observation|>"),
            cut_off: false,
            clock_ms: T0 + 200,
            prompt: None,
            aborted: false,
        },
    );
    let t = affordance::scenario::log_records(&agent.state());
    assert!(
        t.iter().any(|r| r["result"] == "woken by a message"),
        "the wait woke at once"
    );
    assert!(
        !out.iter()
            .any(|c| matches!(c, Command::Timer { at_ms } if *at_ms >= T0 + 600_000)),
        "no 600 s timer: {out:?}"
    );
    // The message also reached the app (its callback), and the next turn is under way.
    assert!(t.iter().any(|r| r["content"] == "[message from agent-b] port is 48213"));
    assert!(out.iter().any(|c| matches!(c, Command::Generate { .. })));
    // Replay reaches the same agent.
    let (b, _) = Instance::restore(config("agent-a", "Wait for b."), journal, &v, &enc).unwrap();
    assert_eq!(
        affordance::scenario::log_records(&b.state()),
        affordance::scenario::log_records(&agent.state())
    );
}

/// An immediate pause cancels a machine call in flight. That is the operator's, not the app's:
/// no error result is journaled, and the restored fleet runs the call again.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn a_pause_does_not_turn_a_cancelled_call_into_a_tool_error() {
    use std::time::Duration;
    let dir = std::env::temp_dir().join(format!("aff-pause-call-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let enc = encoder();
    let config = || {
        let mut c = RuntimeConfig::new(root().join("views/demo/runtime/agent.tsx"));
        c.dir = Some(dir.clone());
        c.workers = 1;
        c.max_wall = Some(Duration::from_secs(60));
        c
    };
    let model = || {
        scripted(enc.clone(), 0, |_, n| {
            if n <= 1 {
                glm_call("", "shell", &[("command", "make")])
            } else {
                "Done.</think>Built.".into()
            }
        })
    };
    let started = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut rt = Runtime::new(config(), enc.clone(), model());
    let flag = started.clone();
    rt.bind_machine(
        "home",
        Arc::new(move |ctx: &affordance::runtime::CallContext, _: &J| {
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
            while !ctx.cancel.is_cancelled() {
                std::thread::sleep(Duration::from_millis(10))
            }
            Err(format!("cancelled: {}", ctx.cancel.reason()))
        }),
    );
    let mut spec = InstanceSpec::new("agent-a", json!({"task": "Build it."}));
    spec.max_turns = Some(2);
    rt.add_instance(spec).unwrap();
    let control = rt.control();
    let closer = std::thread::spawn(move || {
        while !started.load(std::sync::atomic::Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(10))
        }
        control.close(Duration::ZERO);
    });
    let report = rt.run().unwrap();
    closer.join().unwrap();
    assert!(
        !affordance::scenario::log_records(&report.instances[0].state)
            .iter()
            .any(|r| r["kind"] == "action"),
        "no tool result yet: {:?}",
        affordance::scenario::log_records(&report.instances[0].state)
    );
    let mut rt = Runtime::restore(config(), enc.clone(), model()).unwrap();
    rt.bind_machine(
        "home",
        Arc::new(|_: &affordance::runtime::CallContext, _: &J| Ok(json!({"output": "built ok", "exit": 0}))),
    );
    let report = rt.run().unwrap();
    let results: Vec<_> = affordance::scenario::log_records(&report.instances[0].state)
        .iter()
        .filter(|r| r["kind"] == "action")
        .map(|r| r["result"].clone())
        .collect();
    assert_eq!(results.len(), 1, "{results:?}");
    assert!(results[0].as_str().unwrap().contains("built ok"), "{results:?}");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The same for a model call: a pause (or the run ending) cancels it, but that is the host's, not
/// a model failure. No model error is journaled, and the restored run issues the call again.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn a_pause_does_not_turn_a_cancelled_model_call_into_a_model_error() {
    use std::time::Duration;
    let dir = std::env::temp_dir().join(format!("aff-pause-model-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let enc = encoder();
    let config = || {
        let mut c = RuntimeConfig::new(root().join("views/demo/runtime/agent.tsx"));
        c.dir = Some(dir.clone());
        c.workers = 1;
        c.max_wall = Some(Duration::from_secs(60));
        c
    };
    let started = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = started.clone();
    // The first run's model never answers: it waits until its call is cancelled.
    let hanging = Arc::new(FnModel(move |req: &GenRequest| {
        flag.store(true, std::sync::atomic::Ordering::SeqCst);
        while !req.cancel.is_cancelled() {
            std::thread::sleep(Duration::from_millis(10))
        }
        Err(format!("cancelled: {}", req.cancel.reason()))
    }));
    let mut rt = Runtime::new(config(), enc.clone(), hanging);
    let mut spec = InstanceSpec::new("agent-a", json!({"task": "Build it."}));
    spec.max_turns = Some(1);
    rt.add_instance(spec).unwrap();
    let control = rt.control();
    let closer = std::thread::spawn(move || {
        while !started.load(std::sync::atomic::Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(10))
        }
        control.close(Duration::ZERO);
    });
    rt.run().unwrap();
    closer.join().unwrap();
    let journal = std::fs::read_to_string(crate::shared::inst(&dir, "agent-a/journal.jsonl")).unwrap();
    assert!(
        !journal.contains("model_error"),
        "a host cancel is not a model error: {journal}"
    );
    let rt = Runtime::restore(
        config(),
        enc.clone(),
        scripted(enc.clone(), 0, |_, _| "Done.</think>Built.".into()),
    )
    .unwrap();
    let report = rt.run().unwrap();
    let a = &report.instances[0];
    assert!(a.error.is_none(), "{:?}", a.error);
    assert_eq!(
        a.metrics.turns, 1,
        "the restored run issued the call again and the model answered"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Kept running: an idle instance (nothing in flight, waiting only for events) keeps the run up,
/// and a post with a named event reaches its listener. In batch mode the same run returns at once.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn serve_keeps_idle_instances_up_and_the_inbox_names_its_event() {
    use std::time::{Duration, Instant};
    let dir = std::env::temp_dir().join(format!("aff-serve-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let app = dir.join("waiter.tsx");
    std::fs::write(
        &app,
        r#"export default function Main() {
  return <aff event="ping" onCallback={({ text }) => harness.stop({heard: text})}/>;
}"#,
    )
    .unwrap();
    let enc = encoder();
    let config = |serve| {
        let mut c = RuntimeConfig::new(&app);
        c.dir = Some(dir.join("run"));
        c.serve = serve;
        c.max_wall = Some(Duration::from_secs(30));
        c
    };
    let model = || scripted(enc.clone(), 0, |_, _| "unused".into());
    // Batch: idle at once, so the run returns without the event.
    let mut rt = Runtime::new(config(false), enc.clone(), model());
    let mut spec = InstanceSpec::new("a", json!({}));
    spec.model = None;
    rt.add_instance(spec.clone()).unwrap();
    let report = rt.run().unwrap();
    assert!(!report.complete);
    // Kept running: the run waits; a posted `ping` stops the instance.
    let _ = std::fs::remove_dir_all(dir.join("run"));
    let mut rt = Runtime::new(config(true), enc.clone(), model());
    rt.add_instance(spec).unwrap();
    let started = Instant::now();
    let control = rt.control();
    let writer = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(800));
        control.post("a", "ping", json!({"text": "hello"}), None).wait()
    });
    let report = rt.run().unwrap();
    assert_eq!(writer.join().unwrap(), Ok(()));
    assert!(report.complete, "stopped by the ping");
    assert!(
        started.elapsed() >= Duration::from_millis(700),
        "it waited for the event"
    );
    assert_eq!(report.instances[0].stopped, Some(json!({"heard": "hello"})));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A handler error that is not a tool's (an effect, a callback) has no awaiter: the run's status
/// counts it, so a host sees an instance that threw and went idle, and the trace keeps it.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn the_status_feed_counts_handler_errors() {
    use std::time::{Duration, Instant};
    let dir = std::env::temp_dir().join(format!("aff-handler-errors-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let app = dir.join("thrower.tsx");
    std::fs::write(
        &app,
        r#"export default function Main() {
  useEffect(() => { throw new Error("effect failed"); }, []);
  return <aff event="ping" onCallback={() => harness.stop({})}/>;
}"#,
    )
    .unwrap();
    let enc = encoder();
    let mut cfg = RuntimeConfig::new(&app);
    (cfg.dir, cfg.serve, cfg.max_wall) = (Some(dir.join("run")), true, Some(Duration::from_secs(30)));
    let mut rt = Runtime::new(cfg, enc.clone(), scripted(enc.clone(), 0, |_, _| "unused".into()));
    let mut spec = InstanceSpec::new("a", json!({}));
    spec.model = None;
    rt.add_instance(spec).unwrap();
    let control = rt.control();
    let watcher = std::thread::spawn(move || {
        let started = Instant::now();
        let seen = loop {
            let observed = control.status();
            let a = observed["instances"]
                .as_array()
                .and_then(|a| a.iter().find(|a| a["id"] == "a").cloned());
            if let Some(a) = a.filter(|a| a["handler_errors"] == 1) {
                break Some(a);
            }
            if started.elapsed() > Duration::from_secs(20) {
                break None;
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        drop(control.post("a", "ping", json!({}), None));
        seen
    });
    let report = rt.run().unwrap();
    let seen = watcher.join().unwrap().expect("the status counts the effect's error");
    assert_eq!(
        (seen["state"].as_str(), seen["healthy"].as_bool()),
        (Some("idle"), Some(true)),
        "{seen}"
    );
    assert!(report.complete);
    let trace = std::fs::read_to_string(crate::shared::inst(&dir, "run/a/trace.jsonl")).unwrap();
    assert!(
        trace
            .lines()
            .any(|l| l.contains(r#""t":"handler_error""#) && l.contains("effect failed")),
        "{trace}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A `<Mount>`'s callbacks (`mount:<key>:<prop>`) are the runtime's to deliver: a peer cannot
/// send one (as a page cannot forge a trusted event), and a plain `<aff>` cannot claim a handler.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn mount_callbacks_and_handlers_cannot_be_forged() {
    let dir = std::env::temp_dir().join(format!("aff-forge-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let app = dir.join("app.tsx");
    std::fs::write(
        &app,
        r#"export default function Main() {
  useEffect(() => { (async () => {
    let error = null;
    try { await harness.send("b", {}, {event: "mount:kid:onDone"}); } catch (e) { error = e.message; }
    harness.stop({reason: "done", error});
  })(); }, []);
  return <>a</>;
}"#,
    )
    .unwrap();
    let enc = encoder();
    let mut cfg = RuntimeConfig::new(&app);
    cfg.workers = 1;
    let mut rt = Runtime::new(
        cfg,
        enc.clone(),
        Arc::new(FnModel(|_: &GenRequest| panic!("no model call"))),
    );
    for id in ["a", "b"] {
        let mut s = InstanceSpec::new(id, json!({}));
        s.model = None;
        rt.add_instance(s).unwrap();
    }
    let report = rt.run().unwrap();
    let stopped = report
        .instances
        .iter()
        .find(|x| x.id == "a")
        .unwrap()
        .stopped
        .clone()
        .unwrap();
    assert!(
        stopped["error"].as_str().is_some_and(|e| e.contains("reserved")),
        "{stopped}"
    );
    // An <aff> without an on* prop names no handler (an index would bind another's closure).
    let forged = dir.join("forged.tsx");
    std::fs::write(&forged, "export default function Main() { return <><aff label=\"x\" handler={0}>[x]</aff><aff label=\"y\" onRun={() => 1}>[y]</aff></>; }").unwrap();
    let view = View::load(&forged, "Main", &enc).unwrap();
    let error = view
        .render(
            &enc,
            affordance::state_from_json(&json!({"messages": [], "kwargs": {}}), &json!({})),
            &mut affordance::emit::RunCache::new(true),
            chrono::DateTime::from_timestamp(1_791_000_000, 0).unwrap().naive_utc(),
        )
        .err()
        .unwrap();
    assert!(error.contains("`handler` is set by the runtime"), "{error}");
}

/// A secret is redacted as a value, not as its JSON spelling: one holding newlines, quotes and
/// backslashes (a PEM key, a service account's JSON) never reaches instance.json or the trace, and
/// the agent starts. A secret too short to redact is refused.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn secrets_with_newlines_and_quotes_are_redacted() {
    let dir = std::env::temp_dir().join(format!("aff-secrets-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let app = dir.join("app.tsx");
    std::fs::write(&app, "export default function Main({ note }) { useEffect(() => { harness.stop({seen: harness.env.KEY}); }, []); return <>{note}</>; }").unwrap();
    let key = "-----BEGIN KEY-----\nab\"c\\d\n-----END KEY-----";
    let run = |key: &str| {
        let mut cfg = RuntimeConfig::new(&app);
        cfg.workers = 1;
        cfg.dir = Some(dir.join("run"));
        cfg.env.insert("KEY".into(), key.into());
        cfg.secrets.insert("KEY".into());
        let mut rt = Runtime::new(
            cfg,
            encoder(),
            Arc::new(FnModel(|_: &GenRequest| panic!("no model call"))),
        );
        let mut spec = InstanceSpec::new("a", json!({"note": format!("the key is {key}")}));
        spec.model = None;
        rt.add_instance(spec).unwrap();
        rt.run().unwrap()
    };
    let report = run(key);
    assert!(report.instances[0].error.is_none(), "{:?}", report.instances[0].error);
    for file in ["instance.json", "trace.jsonl"] {
        let text = std::fs::read_to_string(crate::shared::instance_dir(&dir.join("run"), "a").join(file)).unwrap();
        let escaped = serde_json::to_string(key).unwrap();
        assert!(
            !text.contains(key) && !text.contains(escaped.trim_matches('"')) && !text.contains("ab\\\"c"),
            "{file}: {text}"
        );
    }
    assert!(
        std::fs::read_to_string(crate::shared::inst(&dir, "run/a/instance.json"))
            .unwrap()
            .contains("the key is [secret:KEY]")
    );
    let report = run("pin");
    assert!(
        report.instances[0]
            .error
            .as_deref()
            .is_some_and(|e| e.contains("too short to redact")),
        "{:?}",
        report.instances[0].error
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// "A reply's calls run one after another by default, and an app can ask for them all at once."
/// One reply asks for two steps; each step's machine call takes 200 ms. In order (the library's
/// default) the second starts after the first has answered; with `{order: "parallel"}` both start
/// before either answers. Every call runs once either way.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn a_replys_calls_run_in_order_by_default_and_at_once_when_asked() {
    use std::time::{Duration, Instant};
    let dir = std::env::temp_dir().join(format!("aff-dispatch-order-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let app = dir.join("app.tsx");
    std::fs::write(
        &app,
        r#"import { useLog } from "@ROOT@/views/lib/log.tsx";
import { useLoop } from "@ROOT@/views/lib/loop.tsx";
import { TurnStrategy } from "@ROOT@/views/strategies/turn_strategy.tsx";
import { GLM53App } from "@ROOT@/views/template_apps/glm53.tsx";
export default function Main({ order }) {
  const log = useLog();
  useLoop(log, {dispatch: order ? {order} : undefined});
  const step = <aff label="step" tool_name="step" description="Take a step."
      args_schema={{type: "object", properties: {n: {type: "integer"}}, required: ["n"]}}
      onStep={async ({ n }) => { await harness.machine.call({n}); return "step " + String(n); }}>{"[step]"}</aff>;
  return <GLM53App keep_replies={true}>
    <TurnStrategy strategy="unrolled" log={[{kind: "input", content: "Take two steps."}, ...log.records]}>{step}</TurnStrategy>
  </GLM53App>;
}"#
        .replace("@ROOT@", env!("CARGO_MANIFEST_DIR")),
    )
    .unwrap();
    let enc = encoder();
    let run = |order: Option<&str>| -> Vec<(i64, Instant, Instant)> {
        let mut cfg = RuntimeConfig::new(&app);
        cfg.max_wall = Some(Duration::from_secs(30));
        cfg.library = Some(root().join("views"));
        let model = scripted(enc.clone(), 0, |_, n| {
            match n {
            1 => "Two steps.</think><tool_call>step<arg_key>n</arg_key><arg_value>1</arg_value></tool_call><tool_call>step<arg_key>n</arg_key><arg_value>2</arg_value></tool_call>".into(),
            _ => "Done.</think>Both steps are taken.".into(),
        }
        });
        let mut rt = Runtime::new(cfg, enc.clone(), model);
        let seen: Arc<Mutex<Vec<(i64, Instant, Instant)>>> = Arc::default();
        let log = seen.clone();
        rt.bind_machine(
            "home",
            Arc::new(move |_: &affordance::runtime::CallContext, args: &J| {
                let started = Instant::now();
                std::thread::sleep(Duration::from_millis(200));
                log.lock()
                    .unwrap()
                    .push((args["n"].as_i64().unwrap(), started, Instant::now()));
                Ok(json!({}))
            }),
        );
        let props = match order {
            Some(o) => json!({"order": o}),
            None => json!({}),
        };
        rt.add_instance(InstanceSpec::new("a", props)).unwrap();
        let report = rt.run().unwrap();
        assert!(report.instances[0].error.is_none(), "{:?}", report.instances[0].error);
        if std::env::var_os("DBG").is_some() {
            eprintln!("stopped {:?}", report.instances[0].stopped);
            for r in affordance::scenario::log_records(&report.instances[0].state) {
                let mut r = r.clone();
                r.as_object_mut().unwrap().remove("ids");
                eprintln!("R {r}");
            }
        }
        let mut seen = seen.lock().unwrap().clone();
        seen.sort_by_key(|(_, started, _)| *started);
        seen
    };
    let in_order = run(None);
    assert_eq!(
        in_order.iter().map(|(n, ..)| *n).collect::<Vec<_>>(),
        [1, 2],
        "each call once, in order"
    );
    assert!(
        in_order[1].1 >= in_order[0].2,
        "the second call started after the first answered"
    );
    let at_once = run(Some("parallel"));
    assert_eq!(at_once.len(), 2, "each call once");
    assert!(at_once[1].1 < at_once[0].2, "both calls started before either answered");
    std::fs::remove_dir_all(&dir).unwrap();
}
