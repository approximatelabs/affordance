//! For every model in views/index.json, measure from its view what an agent needs beyond a
//! rendered prompt: the generation prompt, the token that ends a turn, whether the prompt ends
//! inside open thinking, whether a reply appended to the prompt is what the view renders for
//! the next turn, and which reply parser (src/reply.rs) reads back the tool calls it renders.
//!
//!     cargo run --release --example protocol_survey > target/protocol_survey.json
//!
//! Needs the reference tokenizers (ref/tokenizers). Development program, not a product binary.
use affordance::emit::{Encoder, RunCache};
use affordance::reply::{ActionDef, ActionSet, Syntax, parse_reply_typed};
use affordance::{View, state_from_json};
use serde_json::{Value as J, json};
use std::path::PathBuf;

const SYNTAXES: [&str; 13] = [
    "hermes",
    "xml",
    "glm",
    "deepseek_v3",
    "deepseek_v31",
    "dsml",
    "llama31",
    "llama4",
    "mistral_list",
    "mistral_args",
    "harmony",
    "atem",
    "aff",
];
const MARK: &str = "Zqxj reply text.";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn load(p: PathBuf) -> Option<J> {
    serde_json::from_str(&std::fs::read_to_string(p).ok()?).ok()
}
fn tools() -> J {
    json!([
        {"type": "function", "function": {"name": "get_weather", "description": "Get the current weather for a city.",
            "parameters": {"type": "object", "properties": {"city": {"type": "string"}, "unit": {"type": "string", "enum": ["celsius", "fahrenheit"]}}, "required": ["city"]}}},
        {"type": "function", "function": {"name": "search_docs", "description": "Search the document store.",
            "parameters": {"type": "object", "properties": {"query": {"type": "string"}, "top_k": {"type": "integer"}}, "required": ["query"]}}}
    ])
}
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

fn survey(model: &str, e: &J) -> J {
    let Ok(enc) = Encoder::from_file(root().join(format!("ref/tokenizers/{model}.json")).to_str().unwrap()) else {
        return json!({"error": "no tokenizer"});
    };
    let sp = load(root().join(format!("ref/tokenizers/{model}.special.json"))).unwrap_or(json!({}));
    let view = match View::load(
        &root().join("views").join(e["view"].as_str().unwrap()),
        e["entry"].as_str().unwrap(),
        &enc,
    ) {
        Ok(v) => v,
        Err(error) => return json!({"error": format!("view: {}", affordance::error_message(&error))}),
    };
    let now = chrono::NaiveDate::from_ymd_opt(2026, 10, 5)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap();
    let render = |messages: J, tools: J, agp: bool| -> Result<Vec<u32>, String> {
        let case = json!({"messages": messages, "tools": tools, "kwargs": {"add_generation_prompt": agp}});
        view.render_chat_opts(&enc, state_from_json(&case, &sp), &mut RunCache::new(false), now, false)
            .map(|r| r.ids)
            .map_err(|e| affordance::error_message(&e).chars().take(120).collect())
    };
    let text = |ids: &[u32]| enc.tok.decode(ids, false).unwrap_or_default();
    let kimi = model.starts_with("moonshotai__Kimi");
    let mut out = json!({});
    // A format that reads content as a list of parts ignores a plain string.
    let plain = json!({"role": "user", "content": "Weather in Oslo?"});
    let parts = json!({"role": "user", "content": [{"type": "text", "text": "Weather in Oslo?"}]});
    let shows = |m: &J| render(json!([m]), J::Null, true).is_ok_and(|ids| text(&ids).contains("Weather in Oslo?"));
    let as_parts = !shows(&plain) && shows(&parts);
    out["content_parts"] = json!(as_parts);
    let user = if as_parts { parts } else { plain };
    // The generation prompt: what add_generation_prompt adds.
    let (a, a0) = match (
        render(json!([user]), J::Null, true),
        render(json!([user]), J::Null, false),
    ) {
        (Ok(a), Ok(a0)) => (a, a0),
        (Err(e), _) | (_, Err(e)) => return json!({"error": format!("plain chat: {e}")}),
    };
    let common = a.iter().zip(&a0).take_while(|(x, y)| x == y).count();
    let prompt = text(&a[common..]);
    out["generation_prompt"] = json!(prompt);
    out["prompt_extends_the_conversation"] = json!(common == a0.len());
    let open = prompt
        .rfind("<think>")
        .or(prompt.rfind("[THINK]"))
        .or(prompt.rfind("<|channel|>analysis"));
    let closed = prompt.rfind("</think>").or(prompt.rfind("[/THINK]"));
    out["in_think"] = json!(open.is_some() && closed.is_none_or(|c| c < open.unwrap()));
    // Media: the token the format writes where an image, an audio clip or a video goes (the
    // server expands it). A content part of that kind adds it: a token of the tokenizer's added
    // vocabulary that a text-only message does not have.
    let added_vocabulary = enc.tok.get_added_tokens_decoder();
    let specials = |m: J| -> Option<Vec<u32>> {
        let ids = render(json!([m]), J::Null, true).ok()?;
        Some(ids.into_iter().filter(|t| added_vocabulary.contains_key(t)).collect())
    };
    // The model's own configuration names the token for some kinds (`image_token_id`,
    // `video_token_index`, `audio_context_token_id`).
    let config = load(root().join(format!("ref/models/{model}/config.json"))).unwrap_or(json!({}));
    let declared = |kind: &str| -> Vec<String> {
        let mut found: Vec<String> = sp[format!("{kind}_token")]
            .as_str()
            .map(str::to_string)
            .into_iter()
            .collect();
        for key in ["token_id", "token_index", "context_token_id"] {
            if let Some(id) = config[format!("{kind}_{key}")].as_u64() {
                found.extend(enc.tok.id_to_token(id as u32));
            }
        }
        found
    };
    let words = json!({"type": "text", "text": "Weather in Oslo?"});
    if let Some(without) = specials(json!({"role": "user", "content": [words]})) {
        let mut media = serde_json::Map::new();
        for kind in ["image", "audio", "video"] {
            let Some(with) = specials(json!({"role": "user", "content": [{"type": kind}, words]})) else {
                continue;
            };
            let mut names: Vec<String> = Vec::new();
            for t in with.iter().filter(|t| !without.contains(t)) {
                let name = enc.tok.id_to_token(*t).unwrap_or_default();
                if !names.contains(&name) {
                    names.push(name);
                }
            }
            // The declared token; or the one named as a placeholder; or the only one; or, of a
            // begin, a placeholder and an end named for the kind, the one in the middle.
            let words_for = match kind {
                "image" => vec!["image", "img", "vision"],
                "video" => vec!["video", "vision"],
                other => vec![other],
            };
            let of_kind: Vec<&String> = names
                .iter()
                .filter(|n| words_for.iter().any(|w| n.to_lowercase().contains(w)))
                .collect();
            let pick = declared(kind)
                .into_iter()
                .find(|n| names.contains(n))
                .or_else(|| {
                    names
                        .iter()
                        .find(|n| {
                            let n = n.to_lowercase();
                            ["pad", "context", "patch", "unused", "placeholder", "soft", "embedding"]
                                .iter()
                                .any(|w| n.contains(w))
                        })
                        .cloned()
                })
                .or_else(|| (names.len() == 1).then(|| names[0].clone()))
                .or_else(|| of_kind.get(of_kind.len() / 2).map(|n| n.to_string()));
            if let Some(token) = pick {
                media.insert(kind.to_string(), json!(token));
            }
        }
        if !media.is_empty() {
            out["media"] = J::Object(media);
        }
    }
    // Two user turns in a row (an app's task, then its screen), and a system turn: some formats
    // refuse them.
    out["alternate"] = json!(render(json!([user, user]), J::Null, true).is_err());
    out["system_role"] =
        json!(render(json!([{"role": "system", "content": "Be brief."}, user]), J::Null, true).is_ok());
    // The tokenizer's named tokens, which a template reads by name (bos_token, eos_token, ..).
    out["special"] = J::Object(
        sp.as_object()
            .into_iter()
            .flatten()
            .filter(|(_, v)| v.is_string())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
    );
    // The end of a turn: the first token after a reply's text.
    let reply = if as_parts {
        json!({"role": "assistant", "content": [{"type": "text", "text": MARK}]})
    } else {
        json!({"role": "assistant", "content": MARK})
    };
    let mut stops: Vec<u32> = vec![];
    if let Some(eos) = sp["eos_token"].as_str().and_then(|s| enc.token_id(s)) {
        stops.push(eos);
    }
    match render(json!([user, reply]), J::Null, false) {
        Ok(b) => {
            out["a_reply_appends_exactly"] = json!(b.starts_with(&a));
            let from = b.iter().zip(&a).take_while(|(x, y)| x == y).count();
            let rest = &b[from..];
            let end = (1..=rest.len()).find(|n| text(&rest[..*n]).contains(MARK));
            if let Some(n) = end {
                // What the view writes before the reply's text that the prompt did not have.
                let lead = text(&rest[..n]);
                out["before_the_reply"] = json!(lead[..lead.find(MARK).unwrap()].to_string());
                if let Some(t) = rest.get(n) {
                    out["turn_end"] = json!(enc.tok.id_to_token(*t));
                    out["turn_end_is_special"] = json!(enc.tok.decode(&[*t], true).unwrap_or_default().is_empty());
                    if !stops.contains(t) {
                        stops.push(*t);
                    }
                } else {
                    out["turn_end"] = J::Null;
                }
            }
        }
        Err(e) => out["reply_error"] = json!(e),
    }
    out["stop"] = json!(stops.iter().filter_map(|t| enc.tok.id_to_token(*t)).collect::<Vec<_>>());
    // Tools: does the view render them, and which parser reads a call back.
    let with_tools = render(json!([user]), tools(), true);
    out["renders_tools"] = json!(matches!(&with_tools, Ok(t) if *t != a));
    if let Err(e) = &with_tools {
        out["tools_error"] = json!(e);
    }
    if let Ok(at) = with_tools {
        let mut found: Vec<&str> = vec![];
        let mut sample = String::new();
        let mut appended = false;
        for strargs in [false, true] {
            let args = json!({"city": "Oslo"});
            // Kimi writes a call's id and no name: its ids are `functions.{name}:{index}`.
            let id = if kimi { "functions.get_weather:0" } else { "callc0001" };
            let call = json!({"id": id, "type": "function", "function": {"name": "get_weather",
                "arguments": if strargs { json!(args.to_string()) } else { args.clone() }}});
            let b = [J::Null, json!("")]
                .into_iter()
                .filter_map(|content| {
                    render(
                        json!([user, {"role": "assistant", "content": content, "tool_calls": [call]}]),
                        tools(),
                        false,
                    )
                    .ok()
                })
                .min_by_key(|ids| ids.len());
            let Some(b) = b else { continue };
            let from = b.iter().zip(&at).take_while(|(x, y)| x == y).count();
            appended |= from == at.len();
            let body = &b[from..];
            let body = &body[..body.iter().position(|t| stops.contains(t)).unwrap_or(body.len())];
            let body_text = text(body);
            if body_text.trim().is_empty() {
                continue;
            }
            if sample.is_empty() {
                sample = body_text.clone();
            }
            for name in SYNTAXES {
                let r = parse_reply_typed(
                    &body_text,
                    Syntax::parse(name).unwrap(),
                    &actions(),
                    out["in_think"] == true && from == at.len(),
                    false,
                );
                if r.calls.len() == 1
                    && r.calls[0].name == "get_weather"
                    && r.calls[0].args == args
                    && !found.contains(&name)
                {
                    found.push(name);
                }
            }
        }
        // No parser reads it: the format's own way of writing a call (src/reply_pattern.rs),
        // measured as `callProbe` in views/template_apps/chat.tsx measures it, and kept if it
        // reads the call back.
        if found.is_empty() && !sample.is_empty() {
            let piece = |kind: u32, i: u32| format!("{}{i}", char::from_u32(kind).unwrap());
            let call = |i: u32, args: J| json!({"id": piece(0xE040, i - 1), "type": "function", "function": {"name": piece(0xE010, i), "arguments": args}});
            let arg = |i: u32| json!({piece(0xE020, i): piece(0xE030, i)});
            let both = json!({piece(0xE020, 1): piece(0xE030, 1), piece(0xE020, 2): piece(0xE030, 2)});
            let written = |turn: J| -> J {
                let Ok(prompt) = render(json!([user]), tools(), true) else {
                    return J::Null;
                };
                let full = [J::Null, json!("")]
                    .into_iter()
                    .filter_map(|content| {
                        let mut t = turn.clone();
                        if t.get("content").is_none() {
                            t["content"] = content;
                        }
                        t["role"] = json!("assistant");
                        render(json!([user, t]), tools(), false).ok()
                    })
                    .min_by_key(|ids| ids.len());
                let Some(full) = full else { return J::Null };
                let from = full.iter().zip(&prompt).take_while(|(x, y)| x == y).count();
                let body = &full[from..];
                json!(text(
                    &body[..body.iter().position(|t| stops.contains(t)).unwrap_or(body.len())]
                ))
            };
            let calls = |list: J| written(json!({"tool_calls": list}));
            let thought = written(json!({"reasoning_content": piece(0xE030, 8), "content": piece(0xE030, 9)}));
            let think = thought.as_str().and_then(|t| {
                let (a, b) = (t.find(&piece(0xE030, 8))?, t.find(&piece(0xE030, 9))?);
                (b > a).then(|| json!([t[..a], t[a + piece(0xE030, 8).len()..b]]))
            });
            let id_name = if kimi { json!(["functions.", ":"]) } else { J::Null };
            let probe = json!({
                "one": calls(json!([call(1, arg(1))])), "pair": calls(json!([call(1, both)])),
                "number": calls(json!([call(1, json!({piece(0xE020, 1): 7}))])), "none": calls(json!([call(1, json!({}))])),
                "two": calls(json!([call(1, arg(1)), call(2, arg(2))])), "think": think, "id_name": id_name,
            });
            if std::env::var("SURVEY_ONLY").is_ok() {
                eprintln!(
                    "probe {probe:#}\nsample {sample:?}\npattern {:#?}",
                    affordance::reply_pattern::Pattern::derive(&probe)
                );
            }
            match affordance::reply_pattern::Pattern::derive(&probe) {
                Ok(pattern) => {
                    let set = ActionSet {
                        actions: actions().actions,
                        pattern: Some(std::sync::Arc::new(pattern)),
                    };
                    let r = parse_reply_typed(&sample, Syntax::Pattern, &set, false, false);
                    if r.calls.len() == 1
                        && r.calls[0].name == "get_weather"
                        && r.calls[0].args == json!({"city": "Oslo"})
                    {
                        found.push("pattern");
                        if !id_name.is_null() {
                            out["id_name"] = id_name;
                        }
                    } else {
                        out["pattern_error"] = json!(format!(
                            "read {:?} (problems {:?})",
                            r.calls.iter().map(|c| (&c.name, &c.args)).collect::<Vec<_>>(),
                            r.problem_names()
                        ));
                    }
                }
                Err(error) => out["pattern_error"] = json!(error),
            }
        }
        out["a_call_appends_exactly"] = json!(appended);
        out["call_text"] = json!(sample);
        out["parsers"] = json!(found);
    }
    out
}

fn main() {
    let index = load(root().join("views/index.json")).unwrap();
    let only = std::env::var("SURVEY_ONLY").ok();
    let models: Vec<(&String, &J)> = index
        .as_object()
        .unwrap()
        .iter()
        .filter(|(k, _)| !k.starts_with('_') && only.as_ref().is_none_or(|o| *k == o))
        .collect();
    let models = &models[..];
    let threads = std::thread::available_parallelism().map_or(8, |n| n.get()).min(64);
    let rows: Vec<(String, J)> = std::thread::scope(|scope| {
        let parts: Vec<_> = models
            .chunks(models.len().div_ceil(threads))
            .map(|part| {
                scope.spawn(move || {
                    part.iter()
                        .map(|(model, e)| {
                            let row = std::panic::catch_unwind(|| survey(model, e))
                                .unwrap_or_else(|_| json!({"error": "the survey panicked"}));
                            ((*model).clone(), row)
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        parts.into_iter().flat_map(|p| p.join().unwrap()).collect()
    });
    let mut all = serde_json::Map::new();
    for (model, mut row) in rows {
        row["converted"] = json!(index[&model]["converted"] == true);
        all.insert(model, row);
    }
    println!("{}", serde_json::to_string_pretty(&J::Object(all)).unwrap());
}
