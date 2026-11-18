//! Every chat view as a template app in the harness: wrapped the way `views/template_apps/` wraps
//! its families (templateInput, the view, a protocol span from views/reply_protocols.json), it
//! runs a scripted three-turn tool loop through a real agent. The model's replies are the view's
//! own rendering of a tool call (as in tests/reply_coverage.rs), ending in its stop token; a family
//! without tool calls runs a text loop. Each turn must render, a call must parse and its result
//! come back, and each prompt must be an exact append of the last prompt plus the reply (the
//! prefix cache holds).
use crate::each;
use affordance::emit::{Encoder, RunCache};
use affordance::runtime::{Agent, AgentConfig, Command, Input};
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

fn tool() -> J {
    json!({"type": "function", "function": {"name": "get_weather", "description": "Get the current weather for a city.",
        "parameters": {"type": "object", "properties": {"city": {"type": "string"}}, "required": ["city"]}}})
}

/// What a model of this family generates after the generation prompt, for `assistant` (a tool call,
/// or plain text): the view's rendering of that turn past the shared prefix, closing the think
/// block the generation prompt opened if the history form drops it, ending in a stop token.
/// Also returns the template's own turn boundary: the text between the end of the turn (before its
/// stop token) and the start of `next`'s content.
fn native_reply(
    view: &View,
    enc: &Encoder,
    sp: &J,
    stop: &[u32],
    in_think: bool,
    assistant: &J,
    tools: &J,
    next: &J,
    marker: &str,
) -> Result<(Vec<u32>, String, bool), String> {
    let user = json!({"role": "user", "content": "Weather in Oslo?"});
    let render = |messages: J, agp: bool| -> Result<Vec<u32>, String> {
        let case = json!({"messages": messages, "tools": tools, "kwargs": {"add_generation_prompt": agp}});
        view.render_chat_opts(enc, state_from_json(&case, sp), &mut RunCache::new(false), now(), false)
            .map(|r| r.ids)
    };
    let a = render(json!([user]), true)?;
    // Templates differ in what they accept: arguments as an object or a JSON string, content null
    // or "". The first form that renders the turn is the one.
    let mut forms = vec![];
    for strargs in [false, true] {
        for content in [assistant["content"].clone(), json!("")] {
            let mut c = assistant.clone();
            c["content"] = content;
            if strargs {
                for call in c["tool_calls"].as_array_mut().into_iter().flatten() {
                    let s = call["function"]["arguments"].to_string();
                    call["function"]["arguments"] = json!(s);
                }
            }
            forms.push(c);
        }
    }
    let mut last = String::from("no form renders");
    let mut body = vec![];
    let mut boundary = None;
    for c in forms {
        match render(json!([user, c]), false) {
            Ok(b) => {
                let common = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
                let rest = b[common..].to_vec();
                if rest.iter().any(|t| !stop.contains(t)) {
                    // The same turn followed by `next`, as the template renders history.
                    let turn = &rest[..rest.iter().position(|t| stop.contains(t)).unwrap_or(rest.len())];
                    let turn_text = enc.tok.decode(turn, false).unwrap();
                    // A form must also render as history (followed by `next`), or the next is tried.
                    let Ok(h) = render(json!([user, c.clone(), next]), true) else {
                        last = "the turn does not render as history".into();
                        continue;
                    };
                    {
                        let h = enc.tok.decode(&h, false).unwrap();
                        // Located by its end: history may drop the turn's reasoning.
                        let t = turn_text.trim_end();
                        let tail: String = t.chars().rev().take(30).collect::<Vec<_>>().into_iter().rev().collect();
                        let after = h.rfind(&tail).map(|i| i + tail.len());
                        boundary = after.and_then(|i| h[i..].find(marker).map(|j| h[i..i + j].to_string()));
                        if boundary.is_none() {
                            last = format!(
                                "the turn ({tail:?}) is not in the template's history: {:?}",
                                h.chars()
                                    .rev()
                                    .take(300)
                                    .collect::<Vec<_>>()
                                    .into_iter()
                                    .rev()
                                    .collect::<String>()
                            );
                        }
                    }
                    body = rest;
                    break;
                }
                last = "the template renders nothing for this turn".into();
            }
            Err(e) => last = e,
        }
    }
    if body.is_empty() {
        return Err(last);
    }
    // `added`: the template writes no stop after the turn (the model does); it is appended.
    let added = match body.iter().position(|t| stop.contains(t)) {
        Some(end) => {
            body.truncate(end + 1);
            false
        }
        None => {
            body.push(stop[0]);
            true
        }
    };
    let text = enc.tok.decode(&body, false).unwrap();
    if in_think && !text.contains("</think>") {
        let mut close = enc
            .tok
            .encode("Checking.\n</think>\n\n", false)
            .map_err(|e| e.to_string())?
            .get_ids()
            .to_vec();
        close.extend(body);
        body = close;
    }
    Ok((body, boundary.ok_or(last)?, added))
}

fn app_source(
    view: &str,
    entry: &str,
    syntax: &str,
    stop: &[u32],
    in_think: bool,
    tools: bool,
    sp: &J,
    alternate: bool,
    system: bool,
) -> String {
    let system = if system {
        r#""Use the available tools.""#
    } else {
        "null"
    };
    // The family's special-token strings, as a template app passes them (bos_token, eos_token, ..).
    let specials: String = sp
        .as_object()
        .unwrap()
        .iter()
        .filter_map(|(k, v)| {
            v.as_str()
                .map(|s| format!(" {k}={}", serde_json::to_string(s).unwrap()))
        })
        .collect();
    let tool = if tools {
        r#"<aff tool_name="get_weather" description="Get the current weather for a city."
                   args_schema={{type: "object", properties: {city: {type: "string"}}, required: ["city"]}}
                   onGet={({ city }) => `sunny in ${city}`}>{"[get_weather]\n"}</aff>"#
    } else {
        r#"{"Say something.\n"}"#
    };
    let lp = root().join("views/lib/loop.tsx");
    let lg = root().join("views/lib/log.tsx");
    format!(
        r#"import {{ TurnStrategy }} from {ts:?};
import {{ useLoop }} from {lp:?};
import {{ useLog }} from {lg:?};
import {{ {entry} }} from {view:?};
function WrapApp({{ children }}) {{
  const input = templateInput(children, {system}, true, {alternate});
  return [<{entry} messages={{input.turns}} tools={{input.tools}} add_generation_prompt={{true}}{specials}/>,
          <span label="protocol" syntax="{syntax}" stop={{{stop:?}}} in_think={{{in_think}}}/>];
}}
export default function Main({{ task }}) {{
  const log = useLog();
  useLoop(log, {{idle: () => true}});
  const app = <>{tool}</>;
  return <WrapApp><TurnStrategy strategy="unrolled" log={{[{{kind: "input", content: task}}, ...log.records]}}>{{app}}</TurnStrategy></WrapApp>;
}}
"#,
        ts = root().join("views/strategies/turn_strategy.tsx"),
        view = root().join("views").join(view)
    )
}

/// One view's loop: Ok(exact appends of 2) or the first problem.
fn run_loop(
    model: &str,
    view_file: &str,
    entry: &str,
    p: &J,
    dir: &std::path::Path,
    alternate: bool,
    system: bool,
) -> Result<String, String> {
    let enc = Encoder::from_file(root().join(format!("ref/tokenizers/{model}.json")).to_str().unwrap())?;
    let sp = load(root().join(format!("ref/tokenizers/{model}.special.json")));
    let stop: Vec<u32> = p["stop"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|s| enc.token_id(s.as_str().unwrap()))
        .collect();
    // A family without tool calls runs a text loop: its replies are plain text.
    let tools_mode = p["syntax"].is_string();
    let syntax = p["syntax"].as_str().unwrap_or("hermes");
    let in_think = p["in_think"] == true;
    let view = View::load(&root().join("views").join(view_file), entry, &enc)?;
    let assistant = if tools_mode {
        json!({"role": "assistant", "content": null, "tool_calls": [{"id": "callc0001", "type": "function", "function": {"name": "get_weather", "arguments": {"city": "Oslo"}}}]})
    } else {
        json!({"role": "assistant", "content": "It is sunny in Oslo."})
    };
    let (next, marker) = if tools_mode {
        (
            json!({"role": "tool", "tool_call_id": "callc0001", "name": "get_weather", "content": "sunny in Oslo"}),
            "sunny in Oslo",
        )
    } else {
        (json!({"role": "user", "content": "Say something."}), "Say something.")
    };
    // The marker as this tokenizer decodes it (some decoders drop whitespace).
    let marker_ids = enc
        .tok
        .encode(marker, false)
        .map_err(|e| e.to_string())?
        .get_ids()
        .to_vec();
    let marker = enc.tok.decode(&marker_ids, false).unwrap();
    let (reply, boundary, added) = native_reply(
        &view,
        &enc,
        &sp,
        &stop,
        in_think,
        &assistant,
        &if tools_mode { json!([tool()]) } else { J::Null },
        &next,
        &marker,
    )?;
    let path = dir.join(format!("{}.tsx", model.replace(['.', '-'], "_")));
    std::fs::write(
        &path,
        app_source(
            view_file, entry, syntax, &stop, in_think, tools_mode, &sp, alternate, system,
        ),
    )
    .unwrap();
    let app = View::load(&path, "Main", &enc)?;
    let cfg = AgentConfig {
        context: None,
        retry: Default::default(),
        template: None,
        id: "a".into(),
        props: json!({"task": "Weather in Oslo?"}),
        env: BTreeMap::new(),
        max_new: 512,
        max_turns: Some(3),
        max_prompt: None,
        start_ms: 1_791_000_000_000,
        seed: 7,
        machines: vec![],
        model: Some("default".into()),
    };
    let mut agent = Agent::new(cfg)?;
    let mut queue: Vec<Command> = agent.start(&app, &enc)?;
    let mut clock = 1_791_000_000_000;
    let mut prompts: Vec<Vec<u32>> = vec![];
    while let Some(cmd) = queue.pop() {
        clock += 10;
        if let Command::Generate { task, ids, .. } = cmd {
            prompts.push(ids.to_vec());
            queue.extend(agent.handle(
                &app,
                &enc,
                Input::Reply {
                    server: None,
                    task,
                    ids: reply.clone(),
                    cut_off: false,
                    clock_ms: clock,
                    prompt: None,
                    aborted: false,
                },
            )?);
        }
    }
    // Where a prompt is not the last one plus the reply: the text around the first difference.
    let rewrite = prompts.windows(2).find_map(|w| {
        let want: Vec<u32> = [w[0].as_slice(), reply.as_slice()].concat();
        let at = want.iter().zip(&w[1]).take_while(|(a, b)| a == b).count();
        (at < want.len()).then(|| {
            let ctx = |ids: &[u32]| {
                enc.tok
                    .decode(&ids[at.saturating_sub(12)..(at + 12).min(ids.len())], false)
                    .unwrap()
            };
            format!("expected {:?}, rendered {:?}", ctx(&want), ctx(&w[1]))
        })
    });
    let t = affordance::scenario::log_records(&agent.state());
    let calls = t
        .iter()
        .filter(|r| r["kind"] == "action" && r["name"] == "get_weather")
        .count();
    let results_ok = t
        .iter()
        .filter(|r| r["kind"] == "action")
        .all(|r| r["result"] == "sunny in Oslo");
    if tools_mode && (calls != 3 || !results_ok) {
        let problems: Vec<&J> = t
            .iter()
            .filter(|r| r["kind"] == "model")
            .map(|r| &r["problems"])
            .collect();
        return Err(format!(
            "the reply did not parse to the call ({calls} of 3 ran; problems {problems:?})"
        ));
    }
    let m = &agent.metrics;
    if m.exact_appends != 2 {
        return Err(format!(
            "{} of 2 turns were exact appends ({} rewrites): {}",
            m.exact_appends,
            m.rewrites,
            rewrite.unwrap_or_default()
        ));
    }
    // The turn boundary after a generated turn is the template's own (no doubled stop, no missing
    // separator): from the reply's stop token to the next content, against the template's history
    // from its stop token after the turn (or from the turn's end, where it writes none there).
    if let [first, second, ..] = prompts.as_slice() {
        let text = enc.tok.decode(&second[first.len() + reply.len() - 1..], false).unwrap();
        let got = text
            .find(marker.as_str())
            .map(|i| text[..i].to_string())
            .unwrap_or_default();
        let stop_text = enc.tok.decode(&reply[reply.len() - 1..], false).unwrap();
        let want = match boundary.find(&stop_text) {
            Some(i) => boundary[i..].to_string(),
            // The template writes no stop after the turn but the model does (DeepSeek V2.5/V3's
            // single call): the model's stop, then the template's boundary.
            None if added => format!("{stop_text}{boundary}"),
            None => boundary.clone(),
        };
        if got != want {
            return Err(format!("turn boundary {got:?}, the template's is {want:?}"));
        }
    }
    Ok("exact".into())
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn every_chat_view_runs_as_a_template_app_with_exact_appends() {
    let index = load(root().join("views/index.json"));
    let protocols = load(root().join("views/reply_protocols.json"));
    let dir = std::env::temp_dir().join(format!("aff-sweep-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut seen = std::collections::BTreeSet::new();
    let mut views = vec![];
    for (model, e) in index.as_object().unwrap() {
        // A converted view renders a list of messages to the official tokens; it is not a
        // template app (no turn spans, no reply protocol), so it has no loop to run here.
        if model.starts_with('_') || e["converted"] == true {
            continue;
        }
        let (view, entry) = (e["view"].as_str().unwrap(), e["entry"].as_str().unwrap());
        let key = format!("{view}::{entry}");
        if !seen.insert(key.clone()) {
            continue;
        }
        if std::env::var("SWEEP_ONLY").is_ok_and(|only| !key.contains(&only)) {
            continue;
        }
        views.push((model.as_str(), view, entry, key));
    }
    // Each view's loop is independent: they run side by side.
    let runs = each::in_parallel(&views, |(model, view, entry, key)| {
        let (model, view, entry) = (*model, *view, *entry);
        let p = &protocols[key];
        // How a template app for this family must call templateInput: the first configuration that
        // holds, among plainly; `alternate` (roles must alternate, or a format that places tools by
        // the last user turn); no system turn (a format that moves the system prompt every turn).
        let configs = [
            (false, true, ""),
            (true, true, " (alternate)"),
            (false, false, " (no system)"),
            (true, false, " (alternate, no system)"),
        ];
        let mut run = Err(String::new());
        for (alternate, system, how) in configs {
            run = run_loop(model, view, entry, p, &dir, alternate, system).map(|_| how);
            if run.is_ok() {
                break;
            }
            if run_loop(model, view, entry, p, &dir, false, true)
                .is_err_and(|e| !e.contains("exact appends") && !e.contains("must alternate"))
            {
                break;
            }
        }
        if run.is_err() {
            run = run_loop(model, view, entry, p, &dir, false, true).map(|_| "")
        }
        match run {
            Ok(how) => Ok(format!("{key}{how}")),
            Err(e) => Err(format!("{key} ({model}): {}", e.chars().take(900).collect::<String>())),
        }
    });
    let (mut ok, mut table) = (vec![], vec![]);
    for run in runs {
        match run {
            Ok(line) => ok.push(line),
            Err(line) => table.push(line),
        }
    }
    eprintln!(
        "exact ({}): {ok:?}\n\nnot exact ({}):\n{}",
        ok.len(),
        table.len(),
        table.join("\n")
    );
    assert!(
        table.is_empty(),
        "{} views do not run as template apps with exact appends:\n{}",
        table.len(),
        table.join("\n")
    );
}

/// A text-only template app renders a tool result made of content parts (text and an image) as
/// its text with `[image]`, never as an empty result or the parts as JSON objects.
#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn text_only_template_apps_render_content_part_results_as_text() {
    let dir = std::env::temp_dir().join(format!("aff-parts-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for (file, app, tokenizer) in [
        ("qwen3", "Qwen3App", "Qwen__Qwen3-8B"),
        ("qwen3_base", "Qwen3BaseApp", "Qwen__Qwen3-8B"),
        ("llama31", "Llama31App", "meta-llama__Llama-3.1-8B-Instruct"),
    ] {
        let views = root().join("views");
        let source = format!(
            r#"import {{ {app} }} from "{v}/template_apps/{file}.tsx";
import {{ TurnStrategy }} from "{v}/strategies/turn_strategy.tsx";
const LOG = [
  {{kind: "input", content: "look"}},
  {{kind: "model", ids: null, reasoning: "", content: "ok", calls: [{{type: "function", id: "c1", function: {{name: "look", arguments: {{at: "top"}}}}}}]}},
  {{kind: "action", name: "look", tool_call_id: "c1", result: [{{type: "text", text: "a picture of a cat"}}, {{type: "image_url", image_url: {{url: "data:image/png;base64,AAAA"}}}}]}},
];
export default function Main() {{ return <{app}><TurnStrategy strategy="unrolled" log={{LOG}}/></{app}>; }}
"#,
            v = views.display()
        );
        let path = dir.join(format!("{file}.tsx"));
        std::fs::write(&path, source).unwrap();
        let enc = Encoder::from_file(
            root()
                .join(format!("ref/tokenizers/{tokenizer}.json"))
                .to_str()
                .unwrap(),
        )
        .unwrap();
        let view = View::load_within(&path, "Main", &enc, &[views]).unwrap();
        let ids = view
            .render(
                &enc,
                state_from_json(&json!({}), &json!({})),
                &mut RunCache::new(false),
                now(),
            )
            .unwrap()
            .ids;
        let text = enc.tok.decode(&ids, false).unwrap();
        assert!(
            text.contains("a picture of a cat") && text.contains("[image]"),
            "{app}: {text}"
        );
        assert!(
            !text.contains("image_url") && !text.contains("[object Object]"),
            "{app}: {text}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}
