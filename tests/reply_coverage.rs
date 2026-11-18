//! Reply-parser coverage: every chat view in views/index.json has
//! a row in views/reply_protocols.json, and every parser reads back what its view renders.
//!
//! Round trip: render a conversation without and with an assistant turn holding tool calls (the
//! views are token-exact with the official templates). Where the first render is a prefix of the
//! second, the difference, cut at the first stop token, is exactly what the model would generate
//! for those calls; parsing it must give the same calls back.
use crate::each;
use affordance::emit::{Encoder, RunCache};
use affordance::reply::{ActionDef, ActionSet, Syntax, parse_reply_typed};
use affordance::{View, state_from_json};
use serde_json::{Value as J, json};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn now() -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 10, 5)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap()
}
fn load(p: PathBuf) -> J {
    serde_json::from_str(&std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))).unwrap()
}

fn tools() -> J {
    json!([
        {"type": "function", "function": {"name": "get_weather", "description": "Get the current weather for a city.",
            "parameters": {"type": "object", "properties": {"city": {"type": "string"}, "unit": {"type": "string", "enum": ["celsius", "fahrenheit"]}}, "required": ["city"]}}},
        {"type": "function", "function": {"name": "search_docs", "description": "Search the document store.",
            "parameters": {"type": "object", "properties": {"query": {"type": "string"}, "top_k": {"type": "integer"},
                "filters": {"type": "object", "properties": {"tags": {"type": "array", "items": {"type": "string"}}}}}, "required": ["query"]}}}
    ])
}

/// Conversations to round-trip: (name, calls as (name, args)), with dict or string arguments and
/// with or without reasoning.
fn cases() -> Vec<(&'static str, Vec<(&'static str, J)>, bool, bool)> {
    let oslo = ("get_weather", json!({"city": "Oslo"}));
    let docs = (
        "search_docs",
        json!({"query": "a, b [c]", "top_k": 3, "filters": {"tags": ["x", "y"]}}),
    );
    vec![
        ("one_call", vec![oslo.clone()], false, false),
        ("one_call_strargs", vec![oslo.clone()], true, false),
        ("two_calls", vec![oslo.clone(), docs.clone()], false, false),
        ("two_calls_strargs", vec![oslo.clone(), docs.clone()], true, false),
        ("reasoning_call", vec![oslo], false, true),
    ]
}

/// Template renderings that cannot be read back: what the model would learn there is not the call.
const KNOWN_LOSSY: [(&str, &str, &str); 2] = [
    (
        "llama/llama.tsx::Llama4",
        "one_call_strargs",
        "the Llama 4 template iterates a JSON-string argument character by character",
    ),
    (
        "llama/llama.tsx::Llama4",
        "two_calls_strargs",
        "the Llama 4 template iterates a JSON-string argument character by character",
    ),
];

fn actions() -> ActionSet {
    ActionSet {
        pattern: None,
        actions: tools()
            .as_array()
            .unwrap()
            .iter()
            .map(|t| ActionDef {
                name: t["function"]["name"].as_str().unwrap().into(),
                schema: t["function"]["parameters"].clone(),
                default: None,
                code: None,
            })
            .collect(),
    }
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn every_chat_view_has_a_reply_protocol_and_every_parser_reads_back_its_view() {
    let index = load(root().join("views/index.json"));
    let protocols = load(root().join("views/reply_protocols.json"));
    let mut seen: BTreeMap<String, String> = BTreeMap::new();
    let mut report: Vec<String> = vec![];
    let mut failures: Vec<String> = vec![];
    let mut views = vec![];
    for (model, e) in index.as_object().unwrap() {
        // A converted view renders a list of messages to the official tokens; it is not a
        // template app and has no reply protocol.
        if model.starts_with('_') || e["converted"] == true {
            continue;
        }
        let key = format!("{}::{}", e["view"].as_str().unwrap(), e["entry"].as_str().unwrap());
        let Some(p) = protocols.get(&key) else {
            failures.push(format!("{key}: no row in views/reply_protocols.json"));
            continue;
        };
        if seen.contains_key(&key) {
            continue;
        }
        seen.insert(key.clone(), model.clone());
        views.push((model.as_str(), key, p, e));
    }
    // Each view is independent (its own tokenizer and view): they run side by side.
    let results = each::in_parallel(&views, |(model, key, p, e)| {
        let (model, key, p, e) = (*model, key.clone(), *p, *e);
        let mut report: Vec<String> = vec![];
        let mut failures: Vec<String> = vec![];
        let enc = Encoder::from_file(root().join(format!("ref/tokenizers/{model}.json")).to_str().unwrap()).unwrap();
        let stop: Vec<u32> = p["stop"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| {
                enc.token_id(s.as_str().unwrap())
                    .unwrap_or_else(|| panic!("{key}: stop token {s} not in {model}'s vocabulary"))
            })
            .collect();
        let Some(syntax) = p["syntax"].as_str() else {
            report.push(format!("{key}: no parser ({})", p["reason"].as_str().unwrap_or("?")));
            return (report, failures);
        };
        let syntax = Syntax::parse(syntax).unwrap();
        let view = View::load(
            &root().join("views").join(e["view"].as_str().unwrap()),
            e["entry"].as_str().unwrap(),
            &enc,
        )
        .unwrap();
        let sp = load(root().join(format!("ref/tokenizers/{model}.special.json")));
        let (mut checked, mut skipped) = (vec![], vec![]);
        for (name, calls, strargs, reasoning) in cases() {
            let tool_calls: Vec<J> = calls
                .iter()
                .enumerate()
                .map(|(i, (n, a))| {
                    json!({"id": format!("callc000{}", i + 1), "type": "function",
                "function": {"name": n, "arguments": if strargs { json!(a.to_string()) } else { a.clone() }}})
                })
                .collect();
            let mut assistant = json!({"role": "assistant", "content": null, "tool_calls": tool_calls});
            if reasoning {
                assistant["reasoning_content"] = json!("I should call the weather tool.");
            }
            let user = json!({"role": "user", "content": "Weather in Oslo? Also search the docs."});
            let render = |messages: J, agp: bool| -> Result<Vec<u32>, String> {
                let case = json!({"messages": messages, "tools": tools(), "kwargs": {"add_generation_prompt": agp}});
                view.render_chat_opts(
                    &enc,
                    state_from_json(&case, &sp),
                    &mut RunCache::new(false),
                    now(),
                    false,
                )
                .map(|r| r.ids)
            };
            if let Some(why) = KNOWN_LOSSY
                .iter()
                .find(|(k, c, _)| *k == key && *c == name)
                .map(|(_, _, w)| w)
            {
                skipped.push(format!("{name}: {why}"));
                continue;
            }
            // Some templates need `content: ""` rather than null next to tool calls.
            let with_empty = {
                let mut a = assistant.clone();
                a["content"] = json!("");
                a
            };
            let second = render(json!([user, assistant]), false).or_else(|_| render(json!([user, with_empty]), false));
            let (a, b) = match (render(json!([user]), true), second) {
                (Ok(a), Ok(b)) => (a, b),
                (Err(e), _) | (_, Err(e)) => {
                    skipped.push(format!(
                        "{name}: the template rejects it ({})",
                        affordance::error_message(&e).chars().take(90).collect::<String>()
                    ));
                    continue;
                }
            };
            // Where history is rendered differently from a fresh generation (thinking dropped, a
            // different header), check the calls in their history form: what follows the shared prefix.
            let (body, in_think, form) = if b.starts_with(&a) {
                (&b[a.len()..], p["in_think"] == true, "")
            } else {
                let a0 = render(json!([user]), false).unwrap_or_default();
                let common = a0.iter().zip(&b).take_while(|(x, y)| x == y).count();
                (&b[common..], false, " (history form)")
            };
            let body = &body[..body.iter().position(|t| stop.contains(t)).unwrap_or(body.len())];
            let text = enc.tok.decode(body, false).unwrap();
            if text.trim().is_empty() || text.trim() == "<｜Assistant｜>" {
                skipped.push(format!("{name}: the template renders no calls for this message"));
                continue;
            }
            let r = parse_reply_typed(&text, syntax, &actions(), in_think, false);
            let got: Vec<(String, J)> = r.calls.iter().map(|c| (c.name.clone(), c.args.clone())).collect();
            let want: Vec<(String, J)> = calls.iter().map(|(n, a)| (n.to_string(), a.clone())).collect();
            // Some templates keep only the first call (Llama 3.1, gpt-oss): then that one must come back.
            let ok = got == want
                || (got.len() == 1 && got[0] == want[0] && matches!(syntax, Syntax::Llama31 | Syntax::Harmony));
            if ok {
                checked.push(format!("{name}{form}"))
            } else {
                failures.push(format!(
                    "{key} {name}: parsed {got:?} from {text:?} (problems {:?})",
                    r.problem_names()
                ))
            }
        }
        if checked.is_empty() {
            failures.push(format!("{key}: no round trip could be checked ({skipped:?})"))
        }
        report.push(format!(
            "{key} [{syntax:?}]: checked {checked:?}{}",
            if skipped.is_empty() {
                String::new()
            } else {
                format!(", skipped {skipped:?}")
            }
        ));
        (report, failures)
    });
    for (r, f) in results {
        report.extend(r);
        failures.extend(f);
    }
    eprintln!("{}", report.join("\n"));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    // Every row belongs to a view that exists (no stale rows).
    for key in protocols.as_object().unwrap().keys().filter(|k| !k.starts_with('_')) {
        assert!(
            seen.contains_key(key),
            "views/reply_protocols.json row {key} matches no entry in views/index.json"
        );
    }
}

/// Each template app declares the same reply protocol as the chat view it wraps.
#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn template_app_protocol_spans_match_the_protocol_table() {
    let apps = load(root().join("views/template_apps/index.json"));
    let index = load(root().join("views/index.json"));
    let protocols = load(root().join("views/reply_protocols.json"));
    let dir = std::env::temp_dir().join(format!("aff-protocol-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut checked = 0;
    for (family, a) in apps.as_object().unwrap() {
        let model = a["model"].as_str().unwrap();
        let tokenizer = root().join(format!("ref/tokenizers/{model}.json"));
        // A decision template (d1) wraps no chat view: its reply is an option code, and
        // tests/decision.rs checks its prompt and its replies.
        if !tokenizer.exists() || family == "messages" || a["app"] == "D1App" {
            continue;
        }
        let e = &index[model];
        // A view's own row, or, for a model registered from its publisher's template, what was
        // measured for it.
        let row = protocols
            .get(format!(
                "{}::{}",
                e["view"].as_str().unwrap(),
                e["entry"].as_str().unwrap()
            ))
            .unwrap_or(&e["agent"]);
        let enc = Encoder::from_file(tokenizer.to_str().unwrap()).unwrap();
        let app_name = a["app"].as_str().unwrap();
        let src = format!(
            "import {{ TurnStrategy }} from {:?};\nimport {{ {app_name} }} from {:?};\nimport {{ ConsumerAgent }} from {:?};\nexport default function Main({{}}) {{\n  const app = <ConsumerAgent label=\"agent\" handle=\"a\"/>;\n  return <{}><TurnStrategy strategy=\"unrolled\" log={{[]}}>{{app}}</TurnStrategy></{}>;\n}}\n",
            root().join("views/strategies/turn_strategy.tsx"),
            root().join(format!("views/template_apps/{family}.tsx")),
            root().join("views/demo/consumer/agent.tsx"),
            a["app"].as_str().unwrap(),
            a["app"].as_str().unwrap()
        );
        let path = dir.join(format!("{family}.tsx"));
        std::fs::write(&path, src).unwrap();
        let v = View::load(&path, "Main", &enc).unwrap();
        let r = v
            .render(
                &enc,
                std::rc::Rc::new(Default::default()),
                &mut RunCache::new(false),
                now(),
            )
            .unwrap();
        let span = r
            .spans
            .iter()
            .find(|s| s.label == "protocol")
            .unwrap_or_else(|| panic!("{family}: no protocol span"));
        let stop: Vec<J> = row["stop"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|s| enc.token_id(s.as_str().unwrap()).map(|t| json!(t)))
            .collect();
        assert_eq!(span.meta["syntax"], row["syntax"], "{family}");
        assert!(
            span.meta["stop"].as_array().unwrap().iter().all(|t| stop.contains(t)),
            "{family}: stop {} vs table {:?}",
            span.meta["stop"],
            stop
        );
        checked += 1;
    }
    assert!(checked >= 6, "{checked}");
}
