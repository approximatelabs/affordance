//! What an embedding program relies on (docs/HARNESS_GUIDE.md):
//! - Divergence probe: with `ids` on log records, `prev_prompt ++ generated` is a prefix of
//!   the next render on every turn, including whitespace before `</think>`, non-canonical
//!   generated ids and adjacent `<aff>` labels on one line; text records do not have this property.
//! - One `Encoder` shared by 16 render threads gives the same ids as one thread.
//! - Render errors carry the view stack and the TurnStrategy record.
//! - The `harness.call` example: effects served by name, a slow one asynchronously.
use affordance::effects::{Outcome, Request};
use affordance::emit::{Encoder, RunCache};
use affordance::value::{Map, Value};
use affordance::{Rendered, View};
use serde_json::{Value as J, json};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn now() -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 10, 13)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap()
}
fn encoder(model: &str) -> Encoder {
    Encoder::from_file(root().join(format!("ref/tokenizers/{model}.json")).to_str().unwrap()).unwrap()
}
fn state(j: &J) -> Rc<Map> {
    affordance::value::state(j)
}
fn view(entry_file: &str, enc: &Encoder) -> View {
    View::load(&root().join("views/demo/consumer").join(entry_file), "Main", enc).unwrap()
}
fn render(v: &View, enc: &Encoder, cache: &mut RunCache, s: &J) -> Result<Rendered, String> {
    v.render(enc, state(s), cache, now())
}
fn id(enc: &Encoder, s: &str) -> u32 {
    enc.token_id(s).unwrap_or_else(|| panic!("no token {s}"))
}
/// Model output as a real server returns it: not necessarily the canonical tokenization.
/// Each piece is encoded on its own and concatenated, and the stop token is appended.
fn generate(enc: &Encoder, pieces: &[&str], stop: u32) -> Vec<u32> {
    let mut ids: Vec<u32> = pieces
        .iter()
        .flat_map(|p| enc.tok.encode(*p, false).unwrap().get_ids().to_vec())
        .collect();
    ids.push(stop);
    ids
}

struct Family {
    file: &'static str,
    model: &'static str,
    /// Generated pieces for (reasoning, tool name, argument) and the stop token for a tool call.
    call: fn(&str, &str, &str, &str) -> Vec<String>,
    tool_stop: &'static str,
    text_stop: &'static str,
}

fn glm_call(reasoning: &str, name: &str, key: &str, val: &str) -> Vec<String> {
    vec![
        reasoning.into(),
        "</think>".into(),
        format!("<tool_call>{name}<arg_key>{key}</arg_key><arg_value>{val}</arg_value></tool_call>"),
    ]
}
fn qwen_call(reasoning: &str, name: &str, key: &str, val: &str) -> Vec<String> {
    vec![
        reasoning.into(),
        "</think>\n\n".into(),
        format!("<tool_call>\n<function={name}>\n<parameter={key}>\n{val}\n</parameter>\n</function>\n</tool_call>"),
    ]
}
fn qwen2507_call(_reasoning: &str, name: &str, key: &str, val: &str) -> Vec<String> {
    vec![format!(
        "<tool_call>\n{{\"name\": \"{name}\", \"arguments\": {{\"{key}\": \"{val}\"}}}}\n</tool_call>"
    )]
}
fn llama31_call(_reasoning: &str, name: &str, key: &str, val: &str) -> Vec<String> {
    vec![format!(
        "{{\"name\": \"{name}\", \"parameters\": {{\"{key}\": \"{val}\"}}}}"
    )]
}
fn mimo_call(_reasoning: &str, name: &str, key: &str, val: &str) -> Vec<String> {
    vec![format!(
        "<tool_call>\n<function={name}>\n<parameter={key}>{val}</parameter>\n</function>\n</tool_call>"
    )]
}

const FAMILIES: [Family; 6] = [
    Family {
        file: "glm53.tsx",
        model: "zai-org__GLM-5.3",
        call: glm_call,
        tool_stop: "<|observation|>",
        text_stop: "<|user|>",
    },
    Family {
        file: "qwen36.tsx",
        model: "Qwen__Qwen3.6-27B",
        call: qwen_call,
        tool_stop: "<|im_end|>",
        text_stop: "<|im_end|>",
    },
    Family {
        file: "qwen3.tsx",
        model: "Qwen__Qwen3-4B-Instruct-2507",
        call: qwen2507_call,
        tool_stop: "<|im_end|>",
        text_stop: "<|im_end|>",
    },
    Family {
        file: "llama31.tsx",
        model: "meta-llama__Llama-3.1-8B-Instruct",
        call: llama31_call,
        tool_stop: "<|eot_id|>",
        text_stop: "<|eot_id|>",
    },
    Family {
        file: "mimo25.tsx",
        model: "XiaomiMiMo__MiMo-V2.5",
        call: mimo_call,
        tool_stop: "<|im_end|>",
        text_stop: "<|im_end|>",
    },
    Family {
        file: "qwen3_base.tsx",
        model: "Qwen__Qwen3-8B",
        call: qwen2507_call,
        tool_stop: "<|im_end|>",
        text_stop: "<|im_end|>",
    },
];

/// The consumer's session loop. With `exact`, screens and model turns are logged as ids;
/// otherwise as text, the way the consumer had to before (decoded screen, parsed reply fields).
/// Returns (turns checked, turns where `prev_prompt ++ generated` was not a prefix).
fn session(f: &Family, exact: bool) -> (usize, usize) {
    let enc = encoder(f.model);
    let v = view(f.file, &enc);
    let mut cache = RunCache::new(true);
    let mut log: Vec<J> =
        vec![json!({"kind": "input", "content": "Task: keep /srv/kv healthy.", "pinned": true, "id": "e1", "seq": 1})];
    let mut prev_full: Option<Vec<u32>> = None;
    let (mut checked, mut breaks) = (0, 0);
    for turn in 0..10 {
        let s = json!({"log": log, "handle": "agent-0001", "unread": turn % 3, "notes": if turn > 4 { "port 48213" } else { "" }});
        let r = render(&v, &enc, &mut cache, &s).unwrap_or_else(|e| panic!("{}: turn {turn}: {e}", f.file));
        if let Some(prev) = &prev_full {
            checked += 1;
            if !r.ids.starts_with(prev) {
                breaks += 1;
                if exact {
                    let at = prev.iter().zip(&r.ids).take_while(|(a, b)| a == b).count();
                    panic!(
                        "{}: turn {turn}: diverged at {at}/{}\n prev: {:?}\n next: {:?}",
                        f.file,
                        prev.len(),
                        enc.tok
                            .decode(&prev[at.saturating_sub(8)..(at + 12).min(prev.len())], false)
                            .unwrap(),
                        enc.tok
                            .decode(&r.ids[at.saturating_sub(8)..(at + 12).min(r.ids.len())], false)
                            .unwrap()
                    );
                }
            }
        }
        // The current screen's exact ids, found by span (no template tokens, no decoding).
        let screen = r.current_turn_ids().expect("turn:current span");
        assert!(!screen.is_empty());
        log.push(if exact {
            json!({"kind": "screen", "ids": screen})
        } else {
            json!({"kind": "screen", "content": enc.tok.decode(screen, false).unwrap()})
        });
        // Model output: whitespace before </think> and pieces split mid-word (non-canonical ids).
        let reasoning = format!("Turn {turn}: check the serv ice log first.  \n");
        let text_only = turn == 3 || turn == 7;
        let generated = if text_only {
            let thinks = matches!(f.file, "glm53.tsx" | "qwen36.tsx");
            generate(
                &enc,
                &[&reasoning, if thinks { "</think>" } else { "" }, "Nothing to do yet."],
                id(&enc, f.text_stop),
            )
        } else {
            let pieces = (f.call)(
                &reasoning,
                "shell",
                "command",
                &format!("journalctl -u kv --since -{turn}m"),
            );
            generate(
                &enc,
                &pieces.iter().map(String::as_str).collect::<Vec<_>>(),
                id(&enc, f.tool_stop),
            )
        };
        let mut full = r.ids.clone();
        full.extend(&generated);
        prev_full = Some(full);
        log.push(if exact {
            json!({"kind": "model", "ids": generated, "content": "", "reasoning": reasoning.trim(), "seq": 100 + turn})
        } else if text_only {
            json!({"kind": "model", "content": "Nothing to do yet.", "reasoning": reasoning.trim()})
        } else {
            json!({"kind": "model", "content": "", "reasoning": reasoning.trim(),
                   "calls": [{"type": "function", "function": {"name": "shell", "arguments": {"command": format!("journalctl -u kv --since -{turn}m")}}}]})
        });
        if !text_only {
            log.push(json!({"kind": "action", "name": "shell", "result": format!("kv[{turn}]: ok\nlisten 48213\n[exit 0]"), "exit": 0}));
        }
        if turn == 5 {
            log.push(json!({"kind": "input", "content": "[message from agent-0042] which port?"}));
        }
        log.push(json!({"kind": "metric", "render_ms": 1.5}));
    }
    (checked, breaks)
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn exact_id_records_make_every_turn_an_exact_append() {
    for f in &FAMILIES {
        let (checked, breaks) = session(f, true);
        assert_eq!((checked, breaks), (9, 0), "{}", f.file);
    }
}

/// The control: the same session logged as text (decoded screens, trimmed reasoning) does not
/// re-render to the same ids, which is what `ids` records fix.
#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn text_records_do_not_reproduce_generated_ids() {
    for f in &FAMILIES {
        let (checked, breaks) = session(f, false);
        assert_eq!(checked, 9);
        assert!(breaks > 0, "{}: text records unexpectedly exact", f.file);
    }
}

/// A screen logged as decoded text re-encodes across the adjacent `<aff>` runs.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn decoded_screen_text_differs_from_screen_ids() {
    let enc = encoder("zai-org__GLM-5.3");
    let v = view("glm53.tsx", &enc);
    let r = render(
        &v,
        &enc,
        &mut RunCache::new(false),
        &json!({"log": [], "handle": "agent-0001"}),
    )
    .unwrap();
    let screen = r.current_turn_ids().unwrap();
    let text = enc.tok.decode(screen, false).unwrap();
    assert!(text.contains("[shell][send_message] [set_notes]"), "{text:?}");
    assert_ne!(enc.tok.encode(text.as_str(), false).unwrap().get_ids(), screen);
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn model_ids_are_metadata_free_and_reducible() {
    let enc = encoder("zai-org__GLM-5.3");
    let v = view("glm53.tsx", &enc);
    let mut cache = RunCache::new(false);
    let generated = generate(&enc, &["think  \n", "</think>", "done"], id(&enc, "<|user|>"));
    let with_ids = json!({"log": [{"kind": "model", "ids": generated, "content": "IGNORED", "reasoning": "IGNORED"}], "handle": "a"});
    let text = enc
        .tok
        .decode(&render(&v, &enc, &mut cache, &with_ids).unwrap().ids, false)
        .unwrap();
    assert!(
        text.contains("<|assistant|><think>think  \n</think>done<|user|>") && !text.contains("IGNORED"),
        "{text}"
    );
    // A fold reduces a record by dropping its ids: the fields render as usual.
    let reduced = json!({"log": [{"kind": "model", "ids": null, "content": "short", "reasoning": ""}], "handle": "a"});
    let text = enc
        .tok
        .decode(&render(&v, &enc, &mut cache, &reduced).unwrap().ids, false)
        .unwrap();
    assert!(text.contains("<|assistant|><think></think>short<|user|>"), "{text}");
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn template_apps_without_exact_model_turns_name_the_record() {
    let enc = encoder("Qwen__Qwen3-4B-Instruct-2507");
    let dir = std::env::temp_dir().join(format!("aff-consumer-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let agent = root().join("views/demo/consumer/agent.tsx");
    let strategies = root().join("views/strategies/turn_strategy.tsx");
    let messages = root().join("views/template_apps/messages.tsx");
    std::fs::write(dir.join("m.tsx"), format!(
        "import {{ TurnStrategy }} from {:?};\nimport {{ MessagesApp }} from {:?};\nimport {{ ConsumerAgent }} from {:?};\nexport default function Main({{ log }}) {{\n  const app = <ConsumerAgent label=\"agent\" handle=\"a\"/>;\n  return <MessagesApp><TurnStrategy strategy=\"unrolled\" log={{log}}>{{app}}</TurnStrategy></MessagesApp>;\n}}\n",
        strategies, messages, agent)).unwrap();
    let v = View::load(&dir.join("m.tsx"), "Main", &enc).unwrap();
    let err = render(
        &v,
        &enc,
        &mut RunCache::new(false),
        &json!({"log": [{"kind": "input", "content": "hi"}, {"kind": "model", "ids": [1, 2]}]}),
    )
    .err()
    .unwrap();
    assert!(
        err.contains("TurnStrategy log[1] (kind \"model\")") && err.contains("exact `ids`"),
        "{err}"
    );
    let err = render(
        &v,
        &enc,
        &mut RunCache::new(false),
        &json!({"log": [{"kind": "input", "content": "hi"}, {"kind": "note"}]}),
    )
    .err()
    .unwrap();
    assert!(
        err.contains("TurnStrategy log[1]: unsupported log record kind \"note\""),
        "{err}"
    );
    let err = render(
        &v,
        &enc,
        &mut RunCache::new(false),
        &json!({"log": [{"kind": "action", "name": "x", "result": {"out": 1}}]}),
    )
    .err()
    .unwrap();
    assert!(
        err.contains("TurnStrategy log[0] (kind \"action\"): `result` must be a string"),
        "{err}"
    );
}

/// The consumer's crash: a `p``...``` fragment reached a template slot that indexes its content.
/// A general fragment rejects numeric indexing explicitly; the error says where (file:line per frame) and which log record.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn fragment_in_a_string_slot_names_file_line_and_record() {
    let enc = encoder("Qwen__Qwen3-4B-Instruct-2507");
    let dir = std::env::temp_dir().join(format!("aff-consumer-frag-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let strategies = root().join("views/strategies/turn_strategy.tsx");
    std::fs::write(
        dir.join("t.tsx"),
        format!(
            r#"import {{ TurnStrategy }} from {strategies:?};
function first_output(m) {{
  const c = m.content;
  return c[0].output;
}}
function ToolTurn({{ m }}) {{
  return "tool: " + String(first_output(m));
}}
export function TinyApp({{ children }}) {{
  const input = templateInput(children, "sys");
  return input.turns.map(m => m.role === "tool" ? <ToolTurn m={{m}}/> : "");
}}
export default function Main({{ log }}) {{
  const app = <aff label="go" onGo={{() => ({{}})}}>{{"[go]"}}</aff>;
  const withFragment = [...log, {{kind: "action", result: <>out</>}}];
  return <TinyApp><TurnStrategy strategy="unrolled" log={{withFragment}}>{{app}}</TurnStrategy></TinyApp>;
}}
"#
        ),
    )
    .unwrap();
    let v = View::load(&dir.join("t.tsx"), "Main", &enc).unwrap();
    // A fragment as an action result (what the consumer built with p``), appended by the app.
    let log = Value::list(vec![
        Value::from_json(&json!({"kind": "input", "content": "hi"})),
        Value::from_json(&json!({"kind": "model", "content": ""})),
    ]);
    let mut s = Map::default();
    s.insert(Rc::from("log"), log);
    let err = v
        .render(&enc, Rc::new(s), &mut RunCache::new(false), now())
        .err()
        .unwrap();
    let file = dir.join("t.tsx").display().to_string();
    assert!(err.contains(&format!("at first_output ({file}:4)")), "{err}");
    assert!(err.contains(&format!("at ToolTurn ({file}:7)")), "{err}");
    assert!(
        err.contains("while rendering TurnStrategy log[2] (kind \"action\")"),
        "{err}"
    );
    let msg = affordance::error_message(&err);
    assert!(
        msg.contains("a fragment cannot be indexed") && msg.contains("did you use p`...`?"),
        "{msg}"
    );
}

/// The encoder is loaded once and shared; each render thread binds its own `View` to it.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn one_encoder_serves_sixteen_render_threads() {
    let enc = Arc::new(encoder("zai-org__GLM-5.3"));
    let state_json = json!({"log": [{"kind": "input", "content": "Task."}, {"kind": "model", "ids": [1, 2, 3]}], "handle": "agent-0001"});
    let expected = {
        let v = view("glm53.tsx", &enc);
        render(&v, &enc, &mut RunCache::new(true), &state_json).unwrap().ids
    };
    let handles: Vec<_> = (0..16)
        .map(|_| {
            let enc = Arc::clone(&enc);
            let s = state_json.clone();
            std::thread::spawn(move || {
                let v = view("glm53.tsx", &enc);
                let mut cache = RunCache::new(true);
                (0..20)
                    .map(|_| render(&v, &enc, &mut cache, &s).unwrap().ids)
                    .collect::<Vec<_>>()
            })
        })
        .collect();
    for h in handles {
        for ids in h.join().unwrap() {
            assert_eq!(ids, expected);
        }
    }
}

/// Effects for external actions. The app's handlers await `harness.machine.call` and `harness.send`;
/// the host serves those effect names (never the tool names) on worker threads, one of them slow,
/// replays each handler when its effect completes, and logs the result as an `action` record.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn harness_api_effects_with_a_slow_external_call() {
    use std::sync::mpsc;
    use std::time::{Duration, Instant};
    let enc = encoder("zai-org__GLM-5.3");
    let v = view("glm53.tsx", &enc);
    // Two pending tool calls from two agents' replies (as parsed from the model output).
    let calls = [
        ("agent-0001", "shell", json!({"command": "sleep 0.3; echo built"})),
        ("agent-0002", "send_message", json!({"to": "agent-0001", "text": "hi"})),
    ];
    let mut logs: Vec<Vec<J>> = vec![vec![json!({"kind": "input", "content": "Task."})]; 2];
    let (tx, rx) = mpsc::channel::<(usize, usize, Outcome)>();
    let mut transcripts = vec![];
    let start = Instant::now();
    for (agent, (handle, tool, args)) in calls.iter().enumerate() {
        let s = json!({"log": logs[agent], "handle": handle});
        // Run the handler named by the model's tool name until its first pending effect.
        let step = v
            .act_effects(&enc, state(&s), now(), tool, 0, args, &json!({}), vec![])
            .unwrap();
        assert!(!step.done);
        for &i in &step.blocked {
            let Request::Call { app: effect, args, .. } = step.effects[i].request.clone() else {
                panic!()
            };
            let tx = tx.clone();
            // The host's effect executors, registered by effect name.
            std::thread::spawn(move || {
                let outcome = match effect.as_str() {
                    "machine:" => {
                        std::thread::sleep(Duration::from_millis(300)); // a slow command
                        Outcome::Ok(json!({"output": format!("ran {:?}", args["cmd"].as_str().unwrap()), "exit": 0}))
                    }
                    "send" => Outcome::Ok(json!({"seq": 7})),
                    other => Outcome::Error(format!("no executor for effect {other}")),
                };
                tx.send((agent, i, outcome)).unwrap();
            });
        }
        transcripts.push(step.effects);
    }
    // Complete effects in arrival order: the fast message lands while the shell call still runs.
    let mut order = vec![];
    for _ in 0..2 {
        let (agent, i, outcome) = rx.recv().unwrap();
        transcripts[agent][i].outcome = Some(outcome);
        let (handle, tool, args) = &calls[agent];
        let s = json!({"log": logs[agent], "handle": handle});
        let done = v
            .act_effects(
                &enc,
                state(&s),
                now(),
                tool,
                0,
                args,
                &json!({}),
                transcripts[agent].clone(),
            )
            .unwrap();
        assert!(done.done);
        logs[agent].push(
            json!({"kind": "action", "name": tool, "result": done.writes["result"], "effects": transcripts[agent]}),
        );
        order.push((agent, start.elapsed()));
    }
    assert_eq!(
        order.iter().map(|o| o.0).collect::<Vec<_>>(),
        vec![1, 0],
        "the fast effect completes first"
    );
    assert!(order[0].1 < Duration::from_millis(250));
    assert_eq!(logs[0][1]["result"], "ran \"sleep 0.3; echo built\"\n[exit 0]");
    assert_eq!(logs[1][1]["result"], "delivered to agent-0001 (#7)");
    // The next render shows the result as a tool turn.
    let r = render(
        &v,
        &enc,
        &mut RunCache::new(false),
        &json!({"log": logs[1], "handle": "agent-0002"}),
    )
    .unwrap();
    assert!(
        enc.tok
            .decode(&r.ids, false)
            .unwrap()
            .contains("<|observation|><tool_response>delivered to agent-0001 (#7)</tool_response>")
    );
}
