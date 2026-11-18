//! Readout prompts: views whose render a scoring model reads. Options are `<aff option>` spans and
//! the question a `question` span; the ids are token-exact against each publisher's own prompt.
use affordance::View;
use affordance::emit::{Encoder, RunCache};
use serde_json::{Value as J, json};
use std::path::PathBuf;

use crate::shared;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn now() -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 10, 16)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap()
}

fn view(name: &str, src: &str, enc: &Encoder) -> View {
    let dir = std::env::temp_dir().join(format!("aff-decision-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("main.tsx"), src).unwrap();
    View::load(&dir.join("main.tsx"), "Main", enc).unwrap()
}

/// A readout render: ids, options `{value, span, meta}` in token order, and the question span.
fn readout(v: &View, enc: &Encoder, state: &J) -> (Vec<u32>, Vec<J>, J) {
    let m = affordance::value::state(state);
    let r = v.render(enc, m, &mut RunCache::new(false), now()).unwrap();
    let mut opts: Vec<&affordance::emit::Span> = r
        .spans
        .iter()
        .filter(|s| s.kind == "affordance" && s.meta.get("option").is_some())
        .collect();
    opts.sort_by_key(|s| s.start);
    let options = opts
        .iter()
        .map(|s| {
            let mut meta = s.meta.clone();
            meta.as_object_mut().unwrap().remove("option");
            json!({"value": s.meta["option"], "span": [s.start, s.end], "meta": meta})
        })
        .collect();
    let question = r
        .spans
        .iter()
        .find(|s| s.label == "question")
        .map(|s| json!([s.start, s.end]))
        .unwrap_or(J::Null);
    (r.ids, options, question)
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn d1_view_matches_the_publisher_prompt_on_every_fixture_screen() {
    let enc = shared::tokenizer(&root().join("ref/tokenizers/LiquidAI__d1-3B.json"));
    let fx: J =
        serde_json::from_str(&std::fs::read_to_string(root().join("tests/fixtures/d1_prompts.json")).unwrap()).unwrap();
    let q = &fx["question"];
    let options: Vec<J> = q["criteria"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| json!({"name": k, "desc": v}))
        .collect();
    let v = view(
        "d1",
        &format!(
            r#""use strict_tokens";
import {{ D1Choice }} from "{d1}";
export function Main({{ plan = "", recent_buttons = [], screen = "frame" }}) {{
  return <D1Choice state={{{{plan: plan, recent_buttons: recent_buttons}}}} images={{[screen]}} instructions={{{instr}}} options={{{options}}}/>;
}}
"#,
            d1 = root().join("views/decision/d1.tsx").display(),
            options = J::Array(options),
            instr = q["instructions"]
        ),
        &enc,
    );
    let image = enc.special_strings.iter().find(|(s, _)| s == "<image>").unwrap().1;
    for p in fx["prompts"].as_array().unwrap() {
        let (ids, opts, question) = readout(&v, &enc, &p["state"]);
        assert_eq!(json!(ids), p["ids"], "screen {}", p["screen"]);
        assert_eq!(ids.iter().filter(|&&i| i == image).count(), 1);
        assert_eq!(question, J::Null);
        // Options in prompt order, each span exactly its line, with d1's code letter.
        assert_eq!(
            opts.iter().map(|o| o["value"].clone()).collect::<Vec<_>>(),
            q["criteria"]
                .as_object()
                .unwrap()
                .keys()
                .map(|k| json!(k))
                .collect::<Vec<_>>()
        );
        for (o, code) in opts.iter().zip(p["codes"].as_array().unwrap()) {
            assert_eq!(&o["meta"]["code"], code);
            let (a, b) = (
                o["span"][0].as_u64().unwrap() as usize,
                o["span"][1].as_u64().unwrap() as usize,
            );
            assert_eq!(
                enc.tok.decode(&ids[a..b], false).unwrap(),
                format!(
                    "{} {}",
                    code.as_str().unwrap(),
                    q["criteria"][o["value"].as_str().unwrap()].as_str().unwrap()
                )
            );
        }
    }
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn clef_view_matches_encode_record_ids_question_and_option_spans_on_every_fixture_screen() {
    let enc = shared::tokenizer(&root().join("ref/tokenizers/Cloudflare__clef-flash.json"));
    let fx: J =
        serde_json::from_str(&std::fs::read_to_string(root().join("tests/fixtures/clef_prompts.json")).unwrap())
            .unwrap();
    let options: Vec<J> = fx["question"]["criteria"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| json!({"name": k, "desc": v}))
        .collect();
    let v = view(
        "clef",
        &format!(
            r#""use strict_tokens";
import {{ ClefChoice }} from "{clef}";
export function Main({{ plan = "", recent_buttons = [] }}) {{
  return <ClefChoice state={{{{plan: plan, recent_buttons: recent_buttons}}}} images={{["frame"]}} field="buttons" instructions={{{instr}}} options={{{options}}}/>;
}}
"#,
            clef = root().join("views/decision/clef.tsx").display(),
            options = J::Array(options),
            instr = fx["question"]["instructions"]
        ),
        &enc,
    );
    for p in fx["prompts"].as_array().unwrap() {
        let (ids, opts, question) = readout(&v, &enc, &p["state"]);
        assert_eq!(json!(ids), p["ids"], "screen {}", p["screen"]);
        assert_eq!(question, p["question_span"]);
        assert_eq!(
            opts.iter().map(|o| o["value"].clone()).collect::<Vec<_>>(),
            p["option_ids"].as_array().unwrap().clone()
        );
        assert_eq!(
            opts.iter().map(|o| o["span"].clone()).collect::<Vec<_>>(),
            p["option_spans"].as_array().unwrap().clone()
        );
    }
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn clm_view_gives_clm_state_and_candidate_inputs_token_for_token_on_256_courier_states() {
    let enc = shared::tokenizer(&root().join("ref/tokenizers/Qwen__Qwen3-8B.json"));
    let v = View::load(&root().join("views/apps/courier_clm.tsx"), "Main", &enc).unwrap();
    let fx: J = serde_json::from_str(&std::fs::read_to_string(root().join("tests/fixtures/clm_prompts.json")).unwrap())
        .unwrap();
    let span =
        |ids: &[u32], s: &J| -> J { json!(ids[s[0].as_u64().unwrap() as usize..s[1].as_u64().unwrap() as usize]) };
    for p in fx["prompts"].as_array().unwrap() {
        let (ids, opts, question) = readout(&v, &enc, &p["observe"]);
        assert_eq!(span(&ids, &question), p["state_ids"], "{}", p["id"]);
        assert_eq!(
            J::Array(opts.iter().map(|o| o["value"].clone()).collect()),
            p["options"]
        );
        for (o, want) in opts.iter().zip(p["candidate_ids"].as_array().unwrap()) {
            assert_eq!(&span(&ids, &o["span"]), want);
        }
    }
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn a_const_that_needs_a_missing_special_token_fails_only_where_it_is_used() {
    // The d1 tokenizer has no tok.tool_call: a view that defines a const with it still loads and
    // renders what does not use it, and the part that does is an error naming the const.
    let enc = shared::tokenizer(&root().join("ref/tokenizers/LiquidAI__d1-3B.json"));
    let src = |body: &str| {
        format!(
            "\"use strict_tokens\";\nconst CALL = [tok.tool_call, \"x\"];\nexport function Main({{ use }}) {{ return {body}; }}\n"
        )
    };
    let v = view("const", &src("use ? CALL : \"plain\""), &enc);
    let off = affordance::value::state(&json!({"use": false}));
    assert!(v.render(&enc, off, &mut RunCache::new(false), now()).is_ok());
    let on = affordance::value::state(&json!({"use": true}));
    let Err(e) = v.render(&enc, on, &mut RunCache::new(false), now()) else {
        panic!("rendered a missing special token")
    };
    assert!(e.contains("tok.tool_call") && e.contains("needed by const"), "{e}");
}

/// d1 as a template app: an app's screen and tools render as d1's own prompt (the state the screen,
/// the options the tools), token for token what `D1Choice` renders for that state, and a reply of
/// one option code calls the tool listed under it.
#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn d1_template_app_renders_tools_as_options_and_reads_a_code_as_their_call() {
    use affordance::reply::{ActionSet, Syntax, parse_reply_typed};
    let enc = shared::tokenizer(&root().join("ref/tokenizers/LiquidAI__d1-3B.json"));
    let r = root().display().to_string();
    let app = view(
        "app",
        &format!(
            r#"import {{ D1App }} from '{r}/views/template_apps/d1.tsx';
import {{ TurnStrategy }} from '{r}/views/strategies/turn_strategy.tsx';
import {{ Tool }} from '{r}/views/lib/tool.tsx';
export default function Main({{ message }}) {{
  /** Orders: placing or tracking an order */
  function orders({{}}: {{}}) {{ return "orders"; }}
  /** Refunds: asking for a refund */
  function refunds({{}}: {{}}) {{ return "refunds"; }}
  return <D1App system="Which team should handle this message?"><TurnStrategy strategy="last-turn" log={{[]}}>
    {{"Customer message:\n" + message}}
    <Tool fn={{orders}}/>
    <Tool fn={{refunds}}/>
  </TurnStrategy></D1App>;
}}"#
        ),
        &enc,
    );
    let direct = view(
        "direct",
        &format!(
            r#"import {{ D1Choice }} from '{r}/views/decision/d1.tsx';
export default function Main({{ message }}) {{
  return <D1Choice state={{"Customer message:\n" + message}} instructions="Which team should handle this message?"
    options={{[{{name: "orders", desc: "Orders: placing or tracking an order"}}, {{name: "refunds", desc: "Refunds: asking for a refund"}}]}}/>;
}}"#
        ),
        &enc,
    );
    let state = json!({"message": "I was charged twice and want my money back."});
    let m = affordance::value::state(&state);
    let a = app.render(&enc, m.clone(), &mut RunCache::new(false), now()).unwrap();
    let d = direct.render(&enc, m, &mut RunCache::new(false), now()).unwrap();
    assert_eq!(
        a.ids,
        d.ids,
        "{}\n---\n{}",
        enc.tok.decode(&a.ids, false).unwrap(),
        enc.tok.decode(&d.ids, false).unwrap()
    );
    let protocol = a.spans.iter().find(|s| s.label == "protocol").unwrap();
    assert_eq!(protocol.meta["syntax"], "choice");
    assert_eq!(protocol.meta["codes"], json!({"A": "orders", "B": "refunds"}));
    let actions = ActionSet::from_spans(&a.spans)
        .unwrap()
        .with_codes(&protocol.meta["codes"])
        .unwrap();
    let reply = parse_reply_typed(" B", Syntax::Choice, &actions, false, false);
    assert_eq!(reply.calls.len(), 1);
    assert_eq!(reply.calls[0].name, "refunds");
    assert!(reply.problems.is_empty(), "{:?}", reply.problem_names());
    let wrong = parse_reply_typed("Z", Syntax::Choice, &actions, false, false);
    assert!(wrong.calls.is_empty() && !wrong.problems.is_empty());
}

/// A template app offers each of the screen's actions once, however it writes the screen: in
/// place (Qwen3), as text (d1), or as a hosted API's JSON (MessagesApp).
#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn every_template_app_offers_each_screen_action_once() {
    use affordance::reply::ActionSet;
    let r = root().display().to_string();
    for ((app, file, tokenizer), screen) in [
        ("Qwen3App", "qwen3", "Qwen__Qwen3-8B"),
        ("D1App", "d1", "LiquidAI__d1-3B"),
        ("MessagesApp", "messages", "Qwen__Qwen3-8B"),
    ]
    .into_iter()
    // A screen with text, and a screen of tools alone (it writes no turn and still offers them).
    .flat_map(|case| {
        [
            (case, r#"{"A customer wrote in."}<Tool fn={orders}>orders</Tool>"#),
            (case, "<Tool fn={orders}/>"),
        ]
    }) {
        let enc = shared::tokenizer(&root().join(format!("ref/tokenizers/{tokenizer}.json")));
        let v = view(
            app,
            &format!(
                r#"import {{ {app} }} from '{r}/views/template_apps/{file}.tsx';
import {{ TurnStrategy }} from '{r}/views/strategies/turn_strategy.tsx';
import {{ Tool }} from '{r}/views/lib/tool.tsx';
export default function Main() {{
  /** Orders */
  function orders({{}}: {{}}) {{ return "orders"; }}
  /** Refunds */
  function refunds({{}}: {{}}) {{ return "refunds"; }}
  return <{app}><TurnStrategy strategy="last-turn" log={{[]}}>
    {screen}
    <Tool fn={{refunds}}/>
  </TurnStrategy></{app}>;
}}"#
            ),
            &enc,
        );
        let out = v
            .render(
                &enc,
                affordance::value::state(&json!({})),
                &mut RunCache::new(false),
                now(),
            )
            .unwrap();
        let offered: Vec<&str> = out
            .spans
            .iter()
            .filter(|s| s.kind == "affordance" && s.meta.get("handler").is_some())
            .map(|s| s.label.as_str())
            .collect();
        assert_eq!(offered, ["orders", "refunds"], "{app} {screen}");
        let actions = ActionSet::from_spans(&out.spans).unwrap();
        assert_eq!(actions.actions.len(), 2, "{app} {screen}");
    }
}
