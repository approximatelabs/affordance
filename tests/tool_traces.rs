//! Semantic completion traces from the real dispatch/respond path. No model service.
use affordance::{
    View,
    effects::Outcome,
    emit::Encoder,
    runtime::{CallContext, CancelToken, Command, Exec, Executor, Input, Instance, InstanceConfig},
};
use serde_json::{Value as J, json};
use std::{collections::BTreeMap, path::PathBuf};
const T0: i64 = 1_791_000_000_000;
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn enc() -> Encoder {
    Encoder::from_file(root().join("ref/tokenizers/zai-org__GLM-5.3.json").to_str().unwrap()).unwrap()
}
fn cfg() -> InstanceConfig {
    InstanceConfig {
        context: None,
        retry: Default::default(),
        template: None,
        id: "agent-0000".into(),
        props: json!({}),
        env: BTreeMap::new(),
        max_new: 512,
        max_turns: None,
        max_prompt: None,
        start_ms: T0,
        seed: 7,
        machines: vec!["home".into()],
        model: Some("default".into()),
    }
}
fn fixture(enc: &Encoder, body: &str) -> View {
    fixture_named(enc, body, "probe")
}
fn fixture_named(enc: &Encoder, body: &str, tool: &str) -> View {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "aff-tooltrace-{}-{}",
        std::process::id(),
        N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let src = format!(
        r#"import {{ GLM53App }} from "{template}";
import {{ TurnStrategy }} from "{strategy}";
export function Main() {{
  const [answers, setAnswers] = useState(null, "answers");
  useEffect(() => {{ const run = async () => {{
    const out = await model();
    await harness.wait({{ms: 50}});
    const first = await dispatch(out);
    const second = await dispatch(out);
    setAnswers({{first, second}});
    return {{first, second}};
  }}; run(); }}, []);
  return <GLM53App><TurnStrategy strategy="unrolled" log={{[{{kind:"input",content:"generic fixture"}}]}}><aff tool_name="{tool}" args_schema={{{{type:"object",properties:{{}}}}}} onRun={{async () => {{{body}}}}}>probe</aff></TurnStrategy></GLM53App>;
}}
"#,
        template = root().join("views/template_apps/glm53.tsx").display(),
        strategy = root().join("views/strategies/turn_strategy.tsx").display()
    );
    std::fs::write(dir.join("main.tsx"), src).unwrap();
    View::load(&dir.join("main.tsx"), "Main", enc).unwrap()
}
fn reply(enc: &Encoder, name: &str) -> Vec<u32> {
    let mut ids = enc
        .tok
        .encode(format!("private-thought</think><tool_call>{name}</tool_call>"), false)
        .unwrap()
        .get_ids()
        .to_vec();
    ids.push(enc.token_id("<|observation|>").unwrap());
    ids
}
fn apply(a: &mut Instance, j: &mut Vec<Input>, v: &View, e: &Encoder, input: Input) -> Vec<Command> {
    j.push(input.clone());
    a.handle(v, e, input).unwrap()
}
fn begin(v: &View, e: &Encoder) -> (Instance, Vec<Input>, Vec<Command>) {
    let mut a = Instance::new(cfg()).unwrap();
    let cmds = a.start(v, e).unwrap();
    let task = cmds
        .iter()
        .find_map(|c| {
            if let Command::Generate { task, .. } = c {
                Some(*task)
            } else {
                None
            }
        })
        .unwrap();
    let mut j = vec![];
    let cmds = apply(
        &mut a,
        &mut j,
        v,
        e,
        Input::Reply {
            server: None,
            task,
            ids: reply(e, "probe"),
            cut_off: false,
            clock_ms: T0 + 100,
            prompt: None,
            aborted: false,
        },
    );
    assert!(cmds.iter().any(|c| matches!(c,Command::Timer{at_ms} if *at_ms==T0+150)));
    let cmds = apply(&mut a, &mut j, v, e, Input::Tick { clock_ms: T0 + 150 });
    (a, j, cmds)
}
fn tools(a: &mut Instance) -> Vec<J> {
    a.take_trace().into_iter().filter(|e| e["t"] == "tool").collect()
}
fn call(cmds: &[Command]) -> (u64, String, J) {
    cmds.iter()
        .find_map(|c| {
            if let Command::Call { task, name, args } = c {
                Some((*task, name.clone(), args.clone()))
            } else {
                None
            }
        })
        .unwrap()
}
fn check(e: &J, ms: i64, status: &str, timeout: bool) {
    // `at_ms` is the tool's start in the journal's clock (its end is `at_ms + ms`); `turn` is the
    // page call whose reply asked for it.
    let mut e = e.clone();
    let at = e
        .as_object_mut()
        .unwrap()
        .remove("at_ms")
        .and_then(|v| v.as_i64())
        .expect("at_ms");
    assert_eq!(
        at,
        T0 + 150,
        "the dispatch began when the app's 50 ms wait after the reply ended"
    );
    assert_eq!(
        e,
        json!({"t":"tool","tool":"probe","turn":1,"dispatch":0,"index":0,"ms":ms,"status":status,"timed_out":timeout})
    );
    let s = e.to_string();
    for secret in ["private-thought", "private-command", "private-output", "private-error"] {
        assert!(!s.contains(secret));
    }
}
fn save(name: &str, events: &[J]) {
    if let Ok(dir) = std::env::var("AFFORDANCE_TOOL_TRACE_FIXTURES") {
        let p = PathBuf::from(dir).join(name);
        std::fs::create_dir_all(&p).unwrap();
        std::fs::write(
            p.join("trace.jsonl"),
            events.iter().map(|e| format!("{e}\n")).collect::<String>(),
        )
        .unwrap();
    }
}
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn completion_uses_dispatch_clock_and_preserves_answers_and_replay() {
    let e = enc();
    let v = fixture(
        &e,
        "const r = await harness.machine.call({cmd:'private-command'}); return {result:r.output, exit:r.exit, timed_out:r.timed_out};",
    );
    let (mut a, mut j, cmds) = begin(&v, &e);
    let (task, name, _) = call(&cmds);
    assert_eq!(name, "machine:home");
    assert!(tools(&mut a).is_empty());
    let outcome = Outcome::Ok(json!({"output":"private-output","exit":0,"timed_out":false}));
    apply(
        &mut a,
        &mut j,
        &v,
        &e,
        Input::Result {
            task,
            outcome: outcome.clone(),
            clock_ms: T0 + 400,
        },
    );
    let trace = tools(&mut a);
    assert_eq!(trace.len(), 1);
    check(&trace[0], 250, "ok", false);
    save("success", &trace);
    let state = a.state();
    let answers = state["__ui"]
        .as_object()
        .unwrap()
        .values()
        .find(|v| v.get("first").is_some())
        .unwrap();
    assert_eq!(
        answers["first"], answers["second"],
        "dispatch cache returns identical results"
    );
    assert_eq!(answers["first"][0]["result"], "private-output");
    assert_eq!(answers["first"][0]["exit"], 0);
    apply(
        &mut a,
        &mut j,
        &v,
        &e,
        Input::Result {
            task,
            outcome,
            clock_ms: T0 + 900,
        },
    );
    assert!(
        tools(&mut a).is_empty(),
        "duplicate host result and second dispatch do not emit twice"
    );
    let (mut r, pending) = Instance::restore_with(cfg(), j, &v, &e, |a| a.strict_replay = true).unwrap();
    assert!(pending.is_empty());
    assert_eq!(r.state(), a.state());
    assert!(tools(&mut r).is_empty());
    save("completed-replay", &tools(&mut r));
}
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn thrown_error_and_abort_are_errors_without_invented_timeout() {
    let e = enc();
    let v = fixture(&e, "return await harness.machine.call({cmd:'private-command'});");
    for (label, message) in [
        ("error", "private-error timeout in arbitrary text"),
        ("abort", "AbortError: private-error"),
    ] {
        let (mut a, mut j, cmds) = begin(&v, &e);
        let (task, _, _) = call(&cmds);
        apply(
            &mut a,
            &mut j,
            &v,
            &e,
            Input::Result {
                task,
                outcome: Outcome::Error(message.into()),
                clock_ms: T0 + 175,
            },
        );
        let trace = tools(&mut a);
        assert_eq!(trace.len(), 1);
        check(&trace[0], 25, "error", false);
        save(label, &trace);
    }
}
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn real_executor_timeout_is_explicit_and_not_an_exit_code_guess() {
    let e = enc();
    let v = fixture(
        &e,
        "const r=await harness.machine.call({cmd:'sleep 3',timeout_s:1});return {result:r.output,exit:r.exit,timed_out:r.timed_out};",
    );
    let (mut a, mut j, cmds) = begin(&v, &e);
    let (task, _, args) = call(&cmds);
    let ctx = CallContext {
        client: affordance::runtime::Client {
            id: String::new(),
            name: "agent-0000".into(),
        },
        call: task,
        env: BTreeMap::new(),
        cancel: CancelToken::new(),
        blobs: None,
        output: None,
    };
    let value = Exec::local().call(&ctx, &args).unwrap();
    assert_eq!(value["timed_out"], true);
    apply(
        &mut a,
        &mut j,
        &v,
        &e,
        Input::Result {
            task,
            outcome: Outcome::Ok(value),
            clock_ms: T0 + 1150,
        },
    );
    let trace = tools(&mut a);
    assert_eq!(trace.len(), 1);
    check(&trace[0], 1000, "error", true);
    save("timeout", &trace);
    let v = fixture(&e, "return {result:'private-output timeout',exit:124};");
    let (mut a, _, _) = begin(&v, &e);
    let trace = tools(&mut a);
    check(&trace[0], 0, "error", false);
    save("exit-not-timeout", &trace);
}
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn early_answer_background_failure_and_interrupted_recovery_count_once() {
    let e = enc();
    let v = fixture(
        &e,
        "harness.machine.call({cmd:'private-command'}); return 'private-output';",
    );
    let (mut a, mut j, cmds) = begin(&v, &e);
    let (task, _, _) = call(&cmds);
    let trace = tools(&mut a);
    assert_eq!(trace.len(), 1);
    check(&trace[0], 0, "ok", false);
    save("background", &trace);
    let (mut restored, outstanding) = Instance::restore(cfg(), j.clone(), &v, &e).unwrap();
    assert_eq!(outstanding.len(), 1);
    assert!(tools(&mut restored).is_empty());
    apply(
        &mut restored,
        &mut j,
        &v,
        &e,
        Input::Result {
            task,
            outcome: Outcome::Error("private-error".into()),
            clock_ms: T0 + 900,
        },
    );
    assert!(
        tools(&mut restored).is_empty(),
        "late background error is not another semantic completion"
    );
    let v = fixture(
        &e,
        "const r=await harness.machine.call({cmd:'private-command'});return r.output;",
    );
    let (a, j, cmds) = begin(&v, &e);
    let (task, _, _) = call(&cmds);
    let (mut r, out) = Instance::restore_with(cfg(), j.clone(), &v, &e, |a| a.strict_replay = true).unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(r.state(), a.state());
    assert!(tools(&mut r).is_empty());
    let mut j = j;
    apply(
        &mut r,
        &mut j,
        &v,
        &e,
        Input::Result {
            task,
            outcome: Outcome::Ok(json!({"output":"private-output"})),
            clock_ms: T0 + 450,
        },
    );
    let trace = tools(&mut r);
    assert_eq!(trace.len(), 1);
    check(&trace[0], 300, "ok", false);
    save("recovered", &trace);
}
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn pure_notes_send_wait_and_unavailable_or_undispatched_calls() {
    let e = enc();
    for (label, body, expected_ms) in [
        ("notes", "return 'private-output';", 0),
        (
            "send",
            "await harness.send('peer',{text:'private-output'});return 'sent';",
            50,
        ),
        ("wait", "await harness.wait({ms:10});return 'waited';", 10),
    ] {
        let v = fixture(&e, body);
        let (mut a, mut j, cmds) = begin(&v, &e);
        if label == "send" {
            let (task, name, _) = call(&cmds);
            assert_eq!(name, "send");
            apply(
                &mut a,
                &mut j,
                &v,
                &e,
                Input::Result {
                    task,
                    outcome: Outcome::Ok(json!({"delivered":true})),
                    clock_ms: T0 + 200,
                },
            );
        }
        if label == "wait" {
            apply(&mut a, &mut j, &v, &e, Input::Tick { clock_ms: T0 + 160 });
        }
        let trace = tools(&mut a);
        assert_eq!(trace.len(), 1);
        check(&trace[0], expected_ms, "ok", false);
        save(label, &trace);
    }
    let v = fixture(&e, "return 'unused';");
    let mut a = Instance::new(cfg()).unwrap();
    let cmds = a.start(&v, &e).unwrap();
    let task = cmds
        .iter()
        .find_map(|c| {
            if let Command::Generate { task, .. } = c {
                Some(*task)
            } else {
                None
            }
        })
        .unwrap();
    assert!(tools(&mut a).is_empty());
    save("pending", &[]);
    a.handle(
        &v,
        &e,
        Input::Reply {
            server: None,
            task,
            ids: reply(&e, "unknown_tool"),
            cut_off: false,
            clock_ms: T0 + 100,
            prompt: None,
            aborted: false,
        },
    )
    .unwrap();
    a.handle(&v, &e, Input::Tick { clock_ms: T0 + 150 }).unwrap();
    assert!(tools(&mut a).is_empty());
    save("undispatched", &[]);
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn session_shell_keeps_structured_timeout_through_inline_and_spilled_answers() {
    let e = enc();
    let v = View::load(&root().join("views/agents/session.tsx"), "Main", &e).unwrap();
    for (label, output) in [
        ("session-inline", "private-output".to_owned()),
        ("session-spill", "private-output ".repeat(5000)),
    ] {
        let mut cfg = cfg();
        cfg.template = Some(affordance::runtime::model::ModelTemplate::new("glm53", json!({})).unwrap());
        cfg.props = json!({"id":"agent-0000","task":"generic fixture"});
        cfg.max_turns = Some(2);
        let mut a = Instance::new(cfg).unwrap();
        let cmds = a.start(&v, &e).unwrap();
        let task = cmds
            .iter()
            .find_map(|c| {
                if let Command::Generate { task, .. } = c {
                    Some(*task)
                } else {
                    None
                }
            })
            .unwrap();
        let mut ids = e
            .tok
            .encode(
                "</think><tool_call>shell<arg_key>command</arg_key><arg_value>private-command</arg_value></tool_call>",
                false,
            )
            .unwrap()
            .get_ids()
            .to_vec();
        ids.push(e.token_id("<|observation|>").unwrap());
        let cmds = a
            .handle(
                &v,
                &e,
                Input::Reply {
                    server: None,
                    task,
                    ids,
                    cut_off: false,
                    clock_ms: T0 + 100,
                    prompt: None,
                    aborted: false,
                },
            )
            .unwrap();
        let (task, _, _) = call(&cmds);
        let mut cmds = a
            .handle(
                &v,
                &e,
                Input::Result {
                    task,
                    outcome: Outcome::Ok(json!({"output":output,"exit":124,"timed_out":true,"seconds":1.25})),
                    clock_ms: T0 + 1350,
                },
            )
            .unwrap();
        while let Some((task, name, _)) = cmds.iter().find_map(|c| {
            if let Command::Call { task, name, args } = c {
                Some((*task, name.clone(), args.clone()))
            } else {
                None
            }
        }) {
            assert_eq!(name, "store.put");
            cmds = a
                .handle(
                    &v,
                    &e,
                    Input::Result {
                        task,
                        outcome: Outcome::Ok(json!({"key":"b-fixture"})),
                        clock_ms: T0 + 1400,
                    },
                )
                .unwrap();
        }
        let trace = tools(&mut a);
        assert_eq!(trace.len(), 1);
        assert_eq!(trace[0]["tool"], "shell");
        assert_eq!(trace[0]["timed_out"], true);
        assert_eq!(trace[0]["status"], "error");
        let state = a.state();
        let records = affordance::scenario::log_records(&state);
        let answer = records.iter().find(|r| r["kind"] == "action").unwrap();
        assert_eq!(answer["timed_out"], true);
        assert_eq!(answer["seconds"], 1.25);
        assert_eq!(answer["exit"], 124);
        save(label, &trace);
    }
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn real_host_pause_resume_and_completed_resume_append_one_semantic_event() {
    use affordance::runtime::{FnModel, GenReply, GenRequest, InstanceSpec, Runtime, RuntimeConfig};
    use std::{
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        time::{Duration, Instant},
    };
    let e = Arc::new(enc());
    let dir = std::env::temp_dir().join(format!("aff-tooltrace-host-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let config = || {
        let mut c = RuntimeConfig::new(root().join("views/agents/session.tsx"));
        c.dir = Some(dir.clone());
        c.workers = 1;
        c.model_threads = 1;
        c.exec_threads = 1;
        c.strict_replay = true;
        c.max_wall = Some(Duration::from_secs(10));
        c
    };
    let model = || {
        let e = e.clone();
        let client = Arc::new(FnModel(move |req: &GenRequest| {
            let n = req
                .ids
                .iter()
                .filter(|&&t| t == e.token_id("<|assistant|>").unwrap())
                .count();
            let text = if n <= 1 {
                "</think><tool_call>shell<arg_key>command</arg_key><arg_value>private-command</arg_value></tool_call>"
            } else if n == 2 {
                "</think><tool_call>set_notes<arg_key>text</arg_key><arg_value>private-output</arg_value></tool_call>"
            } else {
                "</think><tool_call>send_message<arg_key>to</arg_key><arg_value>absent-peer</arg_value><arg_key>text</arg_key><arg_value>private-output</arg_value></tool_call>"
            };
            let mut ids = e.tok.encode(text, false).unwrap().get_ids().to_vec();
            ids.push(e.token_id("<|observation|>").unwrap());
            Ok(GenReply {
                ids,
                cut_off: false,
                ..Default::default()
            })
        }));
        affordance::runtime::model::with_template(
            client,
            affordance::runtime::model::ModelTemplate::new("glm53", json!({})).unwrap(),
        )
        .unwrap()
    };
    let started = Arc::new(AtomicBool::new(false));
    let flag = started.clone();
    let mut rt = Runtime::new(config(), e.clone(), model());
    rt.bind_machine(
        "home",
        Arc::new(move |ctx: &CallContext, _: &J| {
            flag.store(true, Ordering::SeqCst);
            while !ctx.cancel.is_cancelled() {
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(format!("cancelled: {}", ctx.cancel.reason()))
        }),
    );
    let mut spec = InstanceSpec::new("agent-0000", json!({"id":"agent-0000","task":"generic fixture"}));
    spec.max_turns = Some(3);
    rt.add_instance(spec).unwrap();
    let control = rt.control();
    let closer = std::thread::spawn(move || {
        let until = Instant::now() + Duration::from_secs(8);
        while !started.load(Ordering::SeqCst) {
            assert!(Instant::now() < until, "executor never started");
            std::thread::sleep(Duration::from_millis(5));
        }
        control.close(Duration::ZERO);
    });
    rt.run().unwrap();
    closer.join().unwrap();
    let read = || {
        std::fs::read_to_string(crate::shared::inst(&dir, "agent-0000/trace.jsonl"))
            .unwrap()
            .lines()
            .map(|s| serde_json::from_str::<J>(s).unwrap())
            .collect::<Vec<_>>()
    };
    assert!(
        read().iter().all(|e| e["t"] != "tool"),
        "controller cancellation has not completed the tool"
    );
    let mut rt = Runtime::restore(config(), e.clone(), model()).unwrap();
    rt.bind_machine(
        "home",
        Arc::new(|_: &CallContext, _: &J| {
            Ok(json!({"output":"private-output","exit":0,"timed_out":false,"seconds":0.01}))
        }),
    );
    let report = rt.run().unwrap();
    assert!(report.instances[0].error.is_none());
    let events = read();
    let completed: Vec<_> = events.iter().filter(|e| e["t"] == "tool").collect();
    assert_eq!(
        completed
            .iter()
            .map(|e| e["tool"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["shell", "set_notes", "send_message"]
    );
    assert!(completed.iter().all(|e| e["status"] == "ok" && e["timed_out"] == false));
    let before = std::fs::read(crate::shared::inst(&dir, "agent-0000/trace.jsonl")).unwrap();
    Runtime::restore(config(), e.clone(), model()).unwrap().run().unwrap();
    assert_eq!(
        read().iter().filter(|e| e["t"] == "tool").count(),
        3,
        "completed restore must not re-append prior completions"
    );
    assert!(
        std::fs::read(crate::shared::inst(&dir, "agent-0000/trace.jsonl"))
            .unwrap()
            .starts_with(&before)
    );
    save("host-restored", &events);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn concurrent_calls_keep_distinct_identities_without_counting_subcalls() {
    let e = enc();
    let v = fixture(
        &e,
        "const r=await harness.machine.call({cmd:'private-command'});return {result:r.output,exit:r.exit,timed_out:r.timed_out};",
    );
    let mut a = Instance::new(cfg()).unwrap();
    let cmds = a.start(&v, &e).unwrap();
    let task = cmds
        .iter()
        .find_map(|c| {
            if let Command::Generate { task, .. } = c {
                Some(*task)
            } else {
                None
            }
        })
        .unwrap();
    let text = "</think><tool_call>probe</tool_call><tool_call>probe</tool_call>";
    let mut ids = e.tok.encode(text, false).unwrap().get_ids().to_vec();
    ids.push(e.token_id("<|observation|>").unwrap());
    a.handle(
        &v,
        &e,
        Input::Reply {
            server: None,
            task,
            ids,
            cut_off: false,
            clock_ms: T0 + 100,
            prompt: None,
            aborted: false,
        },
    )
    .unwrap();
    let cmds = a.handle(&v, &e, Input::Tick { clock_ms: T0 + 150 }).unwrap();
    let tasks = cmds
        .iter()
        .filter_map(|c| {
            if let Command::Call { task, .. } = c {
                Some(*task)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(tasks.len(), 2);
    a.handle(
        &v,
        &e,
        Input::Result {
            task: tasks[1],
            outcome: Outcome::Ok(json!({"output":"private-output","exit":124,"timed_out":true})),
            clock_ms: T0 + 170,
        },
    )
    .unwrap();
    a.handle(
        &v,
        &e,
        Input::Result {
            task: tasks[0],
            outcome: Outcome::Ok(json!({"output":"private-output","exit":0,"timed_out":false})),
            clock_ms: T0 + 180,
        },
    )
    .unwrap();
    let trace = tools(&mut a);
    assert_eq!(trace.len(), 2);
    assert_eq!(trace[0]["index"], 1);
    assert_eq!(trace[1]["index"], 0);
    assert_eq!(trace[0]["dispatch"], trace[1]["dispatch"]);
    assert_eq!(trace[0]["ms"], 20);
    assert_eq!(trace[1]["ms"], 30);
    save("parallel", &trace);
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn long_names_are_bounded_without_colliding_or_retaining_payloads() {
    let e = enc();
    let mut names = vec![];
    for suffix in ["b", "c"] {
        let name = format!("{}{}", "a".repeat(160), suffix);
        let v = fixture_named(&e, "return 'private-output';", &name);
        let mut a = Instance::new(cfg()).unwrap();
        let cmds = a.start(&v, &e).unwrap();
        let task = cmds
            .iter()
            .find_map(|c| {
                if let Command::Generate { task, .. } = c {
                    Some(*task)
                } else {
                    None
                }
            })
            .unwrap();
        a.handle(
            &v,
            &e,
            Input::Reply {
                server: None,
                task,
                ids: reply(&e, &name),
                cut_off: false,
                clock_ms: T0 + 100,
                prompt: None,
                aborted: false,
            },
        )
        .unwrap();
        a.handle(&v, &e, Input::Tick { clock_ms: T0 + 150 }).unwrap();
        let trace = tools(&mut a);
        assert_eq!(trace.len(), 1);
        let bounded = trace[0]["tool"].as_str().unwrap();
        assert_eq!(bounded.chars().count(), 128);
        assert!(bounded.starts_with(&format!("{}#", "a".repeat(63))));
        assert!(trace[0].to_string().len() < 1024);
        assert!(!trace[0].to_string().contains("private-output"));
        names.push(bounded.to_owned());
    }
    assert_ne!(names[0], names[1]);
}
