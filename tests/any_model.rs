//! "An agent runs on any registered model." Every model in views/index.json with an `agent`
//! row is bound as the template of a small agent app (no template app of its own: the one
//! `ChatApp`, given the model's format), and a conversation is stepped as a run steps it: the
//! model's reply is logged with its exact token ids and what it said, and a new message follows.
//!
//! A past reply is written the way the model's publisher writes one. Where a publisher's
//! template writes it as the model generated it, the next prompt is the last prompt, the
//! reply's own tokens and what the format adds. Where it rewrites the reply (drops its
//! thinking, say), the next prompt is the template's. With `keep_replies` the reply's tokens
//! are always kept.
use crate::each;
use affordance::View;
use affordance::emit::{Encoder, RunCache};
use affordance::value::Value;
use serde_json::{Value as J, json};
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn load(p: PathBuf) -> J {
    serde_json::from_str(&std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))).unwrap()
}

/// The two-turn conversation on `model`: Ok(what was checked) or what went wrong.
fn steps(model: &str, e: &J, keep_replies: bool) -> Result<String, String> {
    let enc = Encoder::from_file(root().join(format!("ref/tokenizers/{model}.json")).to_str().unwrap())?;
    let name = model.replacen("__", "/", 1);
    let view = View::load_model(
        &root().join("tests/fixtures/any_model_agent.tsx"),
        "Main",
        &enc,
        &name,
        &json!({"keep_replies": keep_replies}),
    )?;
    let now = chrono::NaiveDate::from_ymd_opt(2026, 10, 5)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap();
    let render = |log: &J| -> Result<affordance::Rendered, String> {
        let Value::Map(state) = Value::from_json(&json!({"log": log})) else {
            unreachable!()
        };
        view.render(
            &enc,
            std::rc::Rc::new(state.into_data()),
            &mut RunCache::new(false),
            now,
        )
        .map_err(|e| {
            affordance::error_message(&e)
                .lines()
                .next()
                .unwrap_or_default()
                .to_string()
        })
    };
    // What the model would write: plain text, after closing the thinking its prompt opened.
    let protocols = load(root().join("views/reply_protocols.json"));
    let row = protocols
        .get(format!(
            "{}::{}",
            e["view"].as_str().unwrap(),
            e["entry"].as_str().unwrap()
        ))
        .unwrap_or(&e["agent"]);
    // The token that ends a turn. A view's own row lists it first; a measured row lists the
    // tokenizer's end of sequence first and the turn's end after it.
    let stops: Vec<&str> = row["stop"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(J::as_str)
        .collect();
    let own = protocols
        .get(format!(
            "{}::{}",
            e["view"].as_str().unwrap(),
            e["entry"].as_str().unwrap()
        ))
        .is_some();
    let stop = if own { stops.first() } else { stops.last() }
        .copied()
        .unwrap_or_default();
    let think = if row["in_think"] == true {
        "I should answer.</think>\n\n"
    } else {
        ""
    };
    let mut log = vec![json!({"kind": "input", "content": "Build it."})];
    let mut last = render(&J::Array(log.clone()))?;
    let mut checked = 0;
    for (reply, next) in [("Built.", "Ship it."), ("Shipped.", "Thanks.")] {
        let ids = enc
            .tok
            .encode(format!("{think}{reply}{stop}"), false)
            .map_err(|e| e.to_string())?
            .get_ids()
            .to_vec();
        let screen = last
            .current_turn_ids()
            .ok_or("the prompt has no current turn")?
            .to_vec();
        log.push(json!({"kind": "screen", "ids": screen}));
        log.push(json!({"kind": "model", "ids": ids, "content": reply, "reasoning": if think.is_empty() { J::Null } else { json!("I should answer.") }}));
        log.push(json!({"kind": "input", "content": next}));
        let now = render(&J::Array(log.clone()))?;
        let want: Vec<u32> = last.ids.iter().chain(&ids).copied().collect();
        if !now.ids.starts_with(&want) {
            let at = now.ids.iter().zip(&want).take_while(|(a, b)| a == b).count();
            let show = |ids: &[u32]| {
                enc.tok
                    .decode(&ids[at.saturating_sub(8)..(at + 10).min(ids.len())], false)
                    .unwrap_or_default()
            };
            return Err(format!(
                "turn {}: the prompt is not the last prompt and the reply: wanted {:?}, got {:?}",
                checked + 1,
                show(&want),
                show(&now.ids)
            ));
        }
        if now.spans.iter().all(|s| s.label != "protocol") {
            return Err("the prompt has no protocol span".into());
        }
        checked += 1;
        last = now;
    }
    Ok(format!("{checked} turns"))
}

/// Formats (a view and its entry) whose vendor writes a past turn differently from the prompt that produced it, so the
/// next prompt is not the last one extended. An agent runs on them; a server recomputes more.
const REWRITES_HISTORY: [(&str, &str); 0] = [];

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn an_agent_runs_on_every_registered_model_and_keeps_its_replies_when_asked() {
    let index = load(root().join("views/index.json"));
    let all = std::env::var_os("AFFORDANCE_ALL_MODELS").is_some();
    let models: Vec<(&String, &J)> = index
        .as_object()
        .unwrap()
        .iter()
        .filter(|(k, e)| !k.starts_with('_') && e["agent"].is_object())
        // In the suite: every twentieth model. tools/check-models.sh runs them all.
        .enumerate()
        .filter(|(i, _)| all || i % 20 == 0)
        .map(|(_, m)| m)
        .collect();
    let results = each::in_parallel(&models, |(model, e)| (model.to_string(), steps(model, e, true)));
    let format = |m: &str| {
        format!(
            "{}::{}",
            index[m]["view"].as_str().unwrap(),
            index[m]["entry"].as_str().unwrap()
        )
    };
    let known = |m: &str, e: &str| {
        e.contains("is not the last prompt") && REWRITES_HISTORY.iter().any(|(k, _)| *k == format(m))
    };
    let failed: Vec<String> = results
        .iter()
        .filter_map(|(m, r)| r.as_ref().err().filter(|e| !known(m, e)).map(|e| format!("{m}: {e}")))
        .collect();
    // A listed format that steps exactly on every one of its models no longer belongs in the list.
    for (key, _) in REWRITES_HISTORY {
        let of_it: Vec<_> = results.iter().filter(|(m, _)| format(m) == key).collect();
        assert!(
            of_it.is_empty() || of_it.iter().any(|(_, r)| r.is_err()),
            "{key} steps exactly: remove it from REWRITES_HISTORY"
        );
    }
    eprintln!(
        "{} of {} models stepped exactly",
        results.len() - failed.len(),
        results.len()
    );
    assert!(
        failed.is_empty(),
        "{} models failed:\n{}",
        failed.len(),
        failed.join("\n")
    );
}

/// "A model's tool call is read, whatever its format." For every model whose calls no parser
/// here was written for (`syntax: "pattern"`), the agent app's prompt carries how the format
/// writes a call, measured from the format; a call written by the format itself (the vendor's
/// template, given the call) is read back from that as the same tool and arguments.
#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn a_call_in_a_format_no_parser_was_written_for_is_read_from_the_format_itself() {
    use affordance::reply::{ActionSet, Syntax, parse_reply_typed};
    let index = load(root().join("views/index.json"));
    let models: Vec<(&String, &J)> = index
        .as_object()
        .unwrap()
        .iter()
        .filter(|(_, e)| e["agent"]["syntax"] == "pattern")
        .collect();
    assert!(
        models.len() >= 40,
        "{} models read their calls by pattern",
        models.len()
    );
    // In the suite: every fourth of them. tools/check-models.sh runs them all.
    let all = std::env::var_os("AFFORDANCE_ALL_MODELS").is_some();
    let models: Vec<(&String, &J)> = models
        .into_iter()
        .enumerate()
        .filter(|(i, _)| all || i % 4 == 0)
        .map(|(_, m)| m)
        .collect();
    let results = each::in_parallel(&models, |(model, e)| {
        let read = || -> Result<(), String> {
            let enc = Encoder::from_file(root().join(format!("ref/tokenizers/{model}.json")).to_str().unwrap())?;
            let name = model.replacen("__", "/", 1);
            let app = View::load_model(
                &root().join("tests/fixtures/any_model_agent.tsx"),
                "Main",
                &enc,
                &name,
                &json!({}),
            )?;
            let now = chrono::NaiveDate::from_ymd_opt(2026, 10, 5)
                .unwrap()
                .and_hms_opt(12, 0, 0)
                .unwrap();
            let Value::Map(state) = Value::from_json(&json!({"log": [{"kind": "input", "content": "Build it."}]}))
            else {
                unreachable!()
            };
            let prompt = app
                .render(
                    &enc,
                    std::rc::Rc::new(state.into_data()),
                    &mut RunCache::new(false),
                    now,
                )
                .map_err(|e| affordance::error_message(&e).to_string())?;
            let protocol = prompt
                .spans
                .iter()
                .find(|s| s.label == "protocol")
                .ok_or("no protocol span")?;
            let actions = ActionSet::from_spans(&prompt.spans)?.with_protocol(&protocol.meta)?;
            // The same call, written by the model's own format.
            let format = View::load(
                &root().join("views").join(e["view"].as_str().unwrap()),
                e["entry"].as_str().unwrap(),
                &enc,
            )?;
            let special = &e["agent"]["special"];
            let tools = json!([{"type": "function", "function": {"name": "shell", "description": "Run a shell command in your VM.",
                "parameters": {"type": "object", "properties": {"command": {"type": "string"}}, "required": ["command"]}}}]);
            let user = json!({"role": "user", "content": "Build it."});
            let id = e["agent"]["id_name"].as_array().map_or("callc0001".to_string(), |a| {
                format!("{}shell{}0", a[0].as_str().unwrap(), a[1].as_str().unwrap())
            });
            let call = json!({"id": id, "type": "function", "function": {"name": "shell", "arguments": {"command": "make -j 4"}}});
            let render = |messages: J, generate: bool| -> Option<Vec<u32>> {
                let case = json!({"messages": messages, "tools": tools, "kwargs": {"add_generation_prompt": generate}});
                format
                    .render_chat_opts(
                        &enc,
                        affordance::state_from_json(&case, special),
                        &mut RunCache::new(false),
                        now,
                        false,
                    )
                    .ok()
                    .map(|r| r.ids)
            };
            let before = render(json!([user]), true).ok_or("the format refuses the conversation")?;
            let with = [J::Null, json!("")]
                .into_iter()
                .filter_map(|content| {
                    render(
                        json!([user, {"role": "assistant", "content": content, "tool_calls": [call]}]),
                        false,
                    )
                })
                .min_by_key(Vec::len)
                .ok_or("the format refuses the call")?;
            let from = with.iter().zip(&before).take_while(|(a, b)| a == b).count();
            let stops: Vec<u32> = e["agent"]["stop"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|s| enc.token_id(s.as_str()?))
                .collect();
            let body = &with[from..];
            let body = &body[..body.iter().position(|t| stops.contains(t)).unwrap_or(body.len())];
            let text = enc.tok.decode(body, false).map_err(|e| e.to_string())?;
            let reply = parse_reply_typed(&text, Syntax::Pattern, &actions, false, false);
            let got: Vec<(&str, &J)> = reply.calls.iter().map(|c| (c.name.as_str(), &c.args)).collect();
            match got == [("shell", &json!({"command": "make -j 4"}))] {
                true => Ok(()),
                false => Err(format!(
                    "read {got:?} from {text:?} (problems {:?})",
                    reply.problem_names()
                )),
            }
        };
        (model.to_string(), read())
    });
    let failed: Vec<String> = results
        .iter()
        .filter_map(|(m, r)| r.as_ref().err().map(|e| format!("{m}: {e}")))
        .collect();
    assert!(
        failed.is_empty(),
        "{} of {} models failed:\n{}",
        failed.len(),
        results.len(),
        failed.join("\n")
    );
}

/// "By default a past reply is written the way the model's publisher writes one." Most
/// publishers' templates write it as the model generated it, and the prompt then extends the
/// last one with the reply's own tokens. A template that rewrites it decides what the model
/// reads: DeepSeek R1's drops a past reply's thinking, so the next prompt has the answer and
/// not the thinking, and is not the last prompt extended.
#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn a_past_reply_is_written_the_way_the_publisher_writes_one() {
    let index = load(root().join("views/index.json"));
    let all = std::env::var_os("AFFORDANCE_ALL_MODELS").is_some();
    let models: Vec<(&String, &J)> = index
        .as_object()
        .unwrap()
        .iter()
        .filter(|(k, e)| !k.starts_with('_') && e["agent"].is_object() && e["converted"] == true)
        .enumerate()
        .filter(|(i, _)| all || i % 20 == 0)
        .map(|(_, m)| m)
        .collect();
    let results = each::in_parallel(&models, |(model, e)| (model.to_string(), steps(model, e, false)));
    // Either the prompt extends the last one, or the template rewrote the reply: nothing else
    // (no error, no missing protocol).
    let other: Vec<String> = results
        .iter()
        .filter_map(|(m, r)| {
            r.as_ref()
                .err()
                .filter(|e| !e.contains("is not the last prompt"))
                .map(|e| format!("{m}: {e}"))
        })
        .collect();
    assert!(other.is_empty(), "{}", other.join("\n"));
    let extended = results.iter().filter(|(_, r)| r.is_ok()).count();
    eprintln!(
        "{extended} of {} converted formats write a past reply as the model generated it",
        results.len()
    );
    assert!(extended * 2 > results.len(), "{extended} of {}", results.len());

    // A template that drops a past reply's thinking.
    let model = "deepseek-ai__DeepSeek-R1-Distill-Qwen-1.5B";
    let enc = Encoder::from_file(root().join(format!("ref/tokenizers/{model}.json")).to_str().unwrap()).unwrap();
    let said = "Thinking it over carefully.\n</think>\n\nBuilt.<｜end▁of▁sentence｜>";
    let ids = enc.tok.encode(said, false).unwrap().get_ids().to_vec();
    let prompt = |keep: bool| -> String {
        let view = View::load_model(
            &root().join("tests/fixtures/any_model_agent.tsx"),
            "Main",
            &enc,
            &model.replacen("__", "/", 1),
            &json!({"keep_replies": keep}),
        )
        .unwrap();
        let log = json!([
            {"kind": "input", "content": "Build it."},
            {"kind": "model", "ids": ids, "content": "Built.", "reasoning": "Thinking it over carefully."},
            {"kind": "input", "content": "Ship it."},
        ]);
        let Value::Map(state) = Value::from_json(&json!({"log": log})) else {
            unreachable!()
        };
        let now = chrono::NaiveDate::from_ymd_opt(2026, 10, 5)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let rendered = view
            .render(
                &enc,
                std::rc::Rc::new(state.into_data()),
                &mut RunCache::new(false),
                now,
            )
            .unwrap();
        enc.tok.decode(&rendered.ids, false).unwrap()
    };
    let (publisher, kept) = (prompt(false), prompt(true));
    assert!(
        publisher.contains("Built.") && !publisher.contains("Thinking it over"),
        "{publisher}"
    );
    assert!(
        kept.contains("Thinking it over carefully.\n</think>\n\nBuilt."),
        "{kept}"
    );
}

/// "An agent runs on a model whose format has no tool calls: its reply is its answer." For
/// every registered model with no call syntax, the agent app's prompt says the reply is text,
/// and a reply is read as prose with no call (the loop ends an agent on such a reply, with the
/// prose as its answer).
#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn a_reply_from_a_format_with_no_tool_calls_is_read_as_text() {
    use affordance::reply::{ActionSet, Problem, Syntax, parse_reply_typed};
    let index = load(root().join("views/index.json"));
    let models: Vec<(&String, &J)> = index
        .as_object()
        .unwrap()
        .iter()
        .filter(|(_, e)| e["converted"] == true && e["agent"].is_object() && e["agent"]["syntax"].is_null())
        .collect();
    assert!(models.len() >= 100, "{} models have no call syntax", models.len());
    // In the suite: every eighth of them. tools/check-models.sh runs them all.
    let all = std::env::var_os("AFFORDANCE_ALL_MODELS").is_some();
    let models: Vec<(&String, &J)> = models
        .into_iter()
        .enumerate()
        .filter(|(i, _)| all || i % 8 == 0)
        .map(|(_, m)| m)
        .collect();
    let results = each::in_parallel(&models, |(model, _)| {
        let read = || -> Result<(), String> {
            let enc = Encoder::from_file(root().join(format!("ref/tokenizers/{model}.json")).to_str().unwrap())?;
            let app = View::load_model(
                &root().join("tests/fixtures/any_model_agent.tsx"),
                "Main",
                &enc,
                &model.replacen("__", "/", 1),
                &json!({}),
            )?;
            let now = chrono::NaiveDate::from_ymd_opt(2026, 10, 5)
                .unwrap()
                .and_hms_opt(12, 0, 0)
                .unwrap();
            let Value::Map(state) = Value::from_json(&json!({"log": [{"kind": "input", "content": "Build it."}]}))
            else {
                unreachable!()
            };
            let prompt = app
                .render(
                    &enc,
                    std::rc::Rc::new(state.into_data()),
                    &mut RunCache::new(false),
                    now,
                )
                .map_err(|e| affordance::error_message(&e).to_string())?;
            let protocol = prompt
                .spans
                .iter()
                .find(|s| s.label == "protocol")
                .ok_or("no protocol span")?;
            let name = protocol.meta["syntax"].as_str().ok_or("the protocol names no syntax")?;
            if name != "text" {
                return Err(format!("syntax {name:?}"));
            }
            let actions = ActionSet::from_spans(&prompt.spans)?.with_protocol(&protocol.meta)?;
            let in_think = protocol.meta["in_think"].as_bool().unwrap_or(false);
            let said = if in_think {
                "Plan it.</think>It is built."
            } else {
                "It is built."
            };
            let reply = parse_reply_typed(said, Syntax::parse(name)?, &actions, in_think, false);
            if reply.prose != "It is built." || !reply.calls.is_empty() || reply.problems != [Problem::NoAction] {
                return Err(format!(
                    "read as {:?}, {} calls, {} problems",
                    reply.prose,
                    reply.calls.len(),
                    reply.problems.len()
                ));
            }
            Ok(())
        };
        read().err().map(|e| format!("{model}: {e}"))
    });
    let failed: Vec<String> = results.into_iter().flatten().collect();
    assert!(
        failed.is_empty(),
        "{} of {} models failed:\n{}",
        failed.len(),
        models.len(),
        failed.join("\n")
    );
}
