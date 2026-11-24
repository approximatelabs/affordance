//! Log policy, independent official protocols, and token/handler ownership.
use affordance::emit::{Encoder, RunCache};
use affordance::value::{Map, Value};
use affordance::{Rendered, View};
use serde_json::{Value as J, json};
use std::{path::PathBuf, rc::Rc};

use crate::shared;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn now() -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 10, 8)
        .unwrap()
        .and_hms_opt(18, 0, 0)
        .unwrap()
}
fn encoder(model: &str) -> std::sync::Arc<Encoder> {
    let path = if model.starts_with("nvidia__") {
        root().join(format!("ref/middle_models/{model}/tokenizer.json"))
    } else {
        root().join(format!("ref/tokenizers/{model}.json"))
    };
    shared::tokenizer(&path)
}
fn state(j: &J) -> Rc<Map> {
    let Value::Map(m) = Value::from_json(j) else {
        panic!("expected state object")
    };

    std::rc::Rc::new(m.into_data())
}
fn render(v: &View, e: &Encoder, j: &J) -> Result<Rendered, String> {
    v.render(e, state(j), &mut RunCache::new(true), now())
}
/// The find-big-number demo for one model format: one app file, the format chosen as a binding does.
fn find_big_number(e: &Encoder, template: &str) -> View {
    View::load_model(
        &root().join("views/demo/find_big_number.tsx"),
        "Main",
        e,
        template,
        // The references for Qwen3.6 were made with its template's `preserve_thinking` on.
        &if template == "qwen36" {
            json!({"preserve_thinking": true})
        } else {
            json!({})
        },
    )
    .unwrap()
}
fn temp_view(name: &str, src: &str, e: &Encoder) -> View {
    let dir = std::env::temp_dir().join(format!("afr_middle_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{name}.tsx"));
    std::fs::write(&path, src).unwrap();
    View::load(&path, "Main", e).unwrap()
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn seven_template_apps_match_independent_hf_references_and_keep_live_spans() {
    let cases: J =
        serde_json::from_str(&std::fs::read_to_string(root().join("ref/middle_out/strategies.json")).unwrap()).unwrap();
    assert_eq!(cases.as_array().unwrap().len(), 56);
    let mut encoders = std::collections::HashMap::new();
    let mut generic_views = std::collections::HashMap::new();
    for case in cases.as_array().unwrap() {
        let family = case["family"].as_str().unwrap();
        let e = encoders
            .entry(family.to_string())
            .or_insert_with(|| encoder(case["model"].as_str().unwrap()));
        let view = View::load(&root().join(format!("tests/fixtures/turns_{family}.tsx")), "Main", e).unwrap();
        let r = render(&view, e, &case["state"]).unwrap_or_else(|err| panic!("{family}: {err}"));
        let expected: Vec<u32> = serde_json::from_value(case["ids"].clone()).unwrap();
        assert_eq!(r.ids, expected, "{family}: {}", case["state"]);
        let generic = generic_views.entry(family.to_owned()).or_insert_with(|| {
            temp_view(
                &format!("generic-{family}"),
                &format!(
                    r#"
import {{ModelContext}} from '{}/views/lib/model.tsx';
export default function Main({{strategy,log,text='Live screen.'}}){{
  return <ModelContext system='Strategy test.' strategy={{strategy}} log={{log}}>
    <aff label='live' description='Choose the live action.'
      args_schema={{{{type:'object',properties:{{text:{{type:'string'}}}},required:['text']}}}}
      onChange={{({{text}})=>({{selected:text}})}}>{{text}}</aff>
  </ModelContext>;
}}"#,
                    root().display()
                ),
                e,
            )
        });
        let generic_render = generic
            .render_with_harness(
                e,
                state(&case["state"]),
                &mut RunCache::new(true),
                now(),
                Value::from_json(&json!({"model":{"name":"test","template":family,"template_options": if family == "qwen36" { json!({"preserve_thinking": true}) } else { json!({}) }}})),
            )
            .unwrap();
        assert_eq!(
            generic_render.ids, expected,
            "binding-selected {family}: {}",
            case["state"]
        );
        let actions: Vec<_> = r.spans.iter().filter(|s| s.meta["projected_tool"] == true).collect();
        assert_eq!(actions.len(), 1, "historical actions must not become callable");
        let span = actions[0];
        assert_eq!(span.label, "change__live");
        assert!(span.start < span.end && span.end <= r.ids.len());
        assert_eq!(
            &r.ids[span.start..span.end],
            e.encode_run(case["state"]["text"].as_str().unwrap(), false, false)
        );
        let acted = view
            .act(
                e,
                state(&case["state"]),
                now(),
                "change__live",
                0,
                &json!({"text":"chosen"}),
            )
            .unwrap();
        assert_eq!(acted.state["selected"], "chosen");
    }
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn model_context_registry_matches_every_registered_template() {
    let registry: J = serde_json::from_str(include_str!("../views/template_apps/index.json")).unwrap();
    for (family, entry) in registry.as_object().unwrap() {
        let e = encoder(entry["model"].as_str().unwrap());
        let generic = temp_view(
            &format!("registry-{family}"),
            &format!(
                r#"
import {{ModelContext,templateApps}} from '{}/views/lib/model.tsx';
export default function Main({{inspect}}){{
  if(inspect)return JSON.stringify(Object.fromEntries(Object.entries(templateApps).map(([key,fn])=>[key,nameOf(fn)])));
  return <ModelContext system='Registry.'>hello</ModelContext>;
}}"#,
                root().display()
            ),
            &e,
        );
        let names: J = serde_json::from_str(
            &e.tok
                .decode(&render(&generic, &e, &json!({"inspect":true})).unwrap().ids, false)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(names.as_object().unwrap().len(), registry.as_object().unwrap().len());
        assert_eq!(names[family], entry["app"]);
        let rendered = generic
            .render_with_harness(
                &e,
                state(&json!({})),
                &mut RunCache::new(true),
                now(),
                Value::from_json(&json!({"model":{"name":"test","template":family,"template_options": if family == "qwen36" { json!({"preserve_thinking": true}) } else { json!({}) }}})),
            )
            .unwrap();
        let explicit = temp_view(
            &format!("explicit-{family}"),
            &format!(
                r#"
import {{{}}} from '{}/views/template_apps/{}.tsx';
import {{TurnStrategy}} from '{}/views/strategies/turn_strategy.tsx';
export default function Main(){{return <{} system='Registry.'><TurnStrategy>hello</TurnStrategy></{}>;}}
"#,
                entry["app"].as_str().unwrap(),
                root().display(),
                family,
                root().display(),
                entry["app"].as_str().unwrap(),
                entry["app"].as_str().unwrap()
            ),
            &e,
        );
        assert_eq!(rendered.ids, render(&explicit, &e, &json!({})).unwrap().ids, "{family}");
    }
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn find_big_number_is_token_exact_in_two_official_templates() {
    let cases: J =
        serde_json::from_str(&std::fs::read_to_string(root().join("ref/middle_out/find_big_number.json")).unwrap())
            .unwrap();
    assert_eq!(cases.as_array().unwrap().len(), 16);
    let mut encoders = std::collections::HashMap::new();
    for case in cases.as_array().unwrap() {
        let family = case["family"].as_str().unwrap();
        let e = encoders
            .entry(family.to_string())
            .or_insert_with(|| encoder(case["model"].as_str().unwrap()));
        assert_eq!(case["state"]["numbers"].as_array().unwrap().len(), 1024);
        let v = find_big_number(e, family);
        let r = render(&v, e, &case["state"]).unwrap();
        let expected: Vec<u32> = serde_json::from_value(case["ids"].clone()).unwrap();
        let mismatch = r.ids.iter().zip(&expected).position(|(a, b)| a != b);
        assert!(
            r.ids == expected,
            "{family}: first mismatch {mismatch:?}, lengths {} / {}",
            r.ids.len(),
            expected.len()
        );
        assert_eq!(r.spans.iter().filter(|s| s.meta["projected_tool"] == true).count(), 4);
    }
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn unrolling_and_last_turn_have_distinct_observation_history() {
    let e = encoder("Qwen__Qwen3.6-27B");
    let v = View::load(&root().join("tests/fixtures/turns_qwen36.tsx"), "Main", &e).unwrap();
    let mut s = json!({"strategy":"unrolled", "log":[
        {"kind":"input","content":"persistent goal"},
        {"kind":"screen","content":"obsolete screen"},
        {"kind":"model","content":"I observed it"},
        {"kind":"metric","prefill_ms":8}
    ]});
    let a = render(&v, &e, &s).unwrap();
    s["strategy"] = json!("last-turn");
    let b = render(&v, &e, &s).unwrap();
    for (r, unrolled) in [(&a, true), (&b, false)] {
        let text = e.tok.decode(&r.ids, false).unwrap();
        assert_eq!(text.contains("obsolete screen"), unrolled);
        assert!(text.contains("persistent goal") && text.contains("I observed it") && text.contains("Live screen."));
        assert!(!text.contains("prefill_ms"));
    }
    assert!(a.ids.len() > b.ids.len());
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn invalid_plans_and_unconsumed_strategy_fail_explicitly() {
    let e = encoder("Qwen__Qwen3.6-27B");
    let v = View::load(&root().join("tests/fixtures/turns_qwen36.tsx"), "Main", &e).unwrap();
    for s in [
        json!({"strategy":"guess","log":[]}),
        json!({"strategy":"last-turn","log":{}}),
        json!({"strategy":"last-turn","log":[{"kind":"unknown"}]}),
    ] {
        assert!(render(&v, &e, &s).is_err());
    }
    let naked = temp_view(
        "naked",
        r#"export default function Main() { return turnPlan(asChatTools("screen"), []); }"#,
        &e,
    );
    assert!(
        render(&naked, &e, &json!({}))
            .err()
            .unwrap()
            .contains("model-template app")
    );
    let mixed = temp_view(
        "mixed",
        r#"export default function Main() { return templateInput(turnPlan(asChatTools("screen"), []) + "lost child", "sys"); }"#,
        &e,
    );
    assert!(render(&mixed, &e, &json!({})).err().unwrap().contains("exactly one"));
}

fn numbers() -> Vec<i64> {
    let mut seed = 20261008u64;
    (0..1024)
        .map(|_| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % 1_000_000) as i64
        })
        .collect()
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn find_big_number_scroll_edit_and_submit_survive_model_switch() {
    let families = [
        ("qwen3", "Qwen__Qwen3-4B-Instruct-2507"),
        ("qwen36", "Qwen__Qwen3.6-27B"),
        ("llama31", "meta-llama__Llama-3.1-8B-Instruct"),
        ("glm53", "zai-org__GLM-5.3"),
        ("mimo25", "XiaomiMiMo__MiMo-V2.5"),
        ("nemotron3", "nvidia__NVIDIA-Nemotron-3-Nano-30B-A3B-BF16"),
    ];
    let ns = numbers();
    let mut s = json!({"numbers":ns,"budget":96,"strategy":"last-turn","log":[]});
    for (family, model) in families {
        let e = encoder(model);
        let v = find_big_number(&e, family);
        let r = render(&v, &e, &s).unwrap();
        let text = e.tok.decode(&r.ids, false).unwrap();
        assert!(
            text.contains("Goal: Find the biggest")
                && text.contains("Scratchpad:")
                && text.contains("Answer:")
                && text.contains("Submit")
        );
        let labels: Vec<_> = r
            .spans
            .iter()
            .filter(|s| s.meta["projected_tool"] == true)
            .map(|s| s.label.as_str())
            .collect();
        assert_eq!(
            labels,
            [
                "change__scratchpad",
                "scroll__notebook",
                "change__answer",
                "click__submit"
            ]
        );
        s = v
            .act(&e, state(&s), now(), "scroll__notebook", 0, &json!({"by":100}))
            .unwrap()
            .state;
        let next = render(&v, &e, &s).unwrap();
        assert!(e.tok.decode(&next.ids, false).unwrap().contains("starts at row 100"));
        s = v
            .act(
                &e,
                state(&s),
                now(),
                "change__scratchpad",
                0,
                &json!({"text":"best so far: 123"}),
            )
            .unwrap()
            .state;
        s = v
            .act(&e, state(&s), now(), "change__answer", 0, &json!({"text":"123"}))
            .unwrap()
            .state;
        s = v
            .act(&e, state(&s), now(), "click__submit", 0, &json!({}))
            .unwrap()
            .state;
        assert_eq!(s["submitted"], "123");
        assert!(s.get("messages").is_none());
        // Same app hook path persists through the next template app.
        s = v
            .act(&e, state(&s), now(), "scroll__notebook", 0, &json!({"by":-100}))
            .unwrap()
            .state;
        assert!(
            e.tok
                .decode(&render(&v, &e, &s).unwrap().ids, false)
                .unwrap()
                .contains("Scratchpad: best so far: 123")
        );
    }
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn projected_actions_cannot_be_cropped_after_strategy() {
    let e = encoder("Qwen__Qwen3.6-27B");
    let path = root().join("views/demo/find_big_number.tsx");
    let src = format!(
        "import Main from {:?}; export default function Outer() {{ return <Main numbers={{[1,2,3]}} style={{{{maxTokens:1}}}}/>; }}",
        path.to_str().unwrap()
    );
    let dir = std::env::temp_dir().join(format!("afr_middle_crop_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("crop.tsx");
    std::fs::write(&file, src).unwrap();
    let v = View::load_model(&file, "Outer", &e, "qwen36", &json!({"preserve_thinking": true})).unwrap();
    let error = render(&v, &e, &json!({}))
        .err()
        .expect("cropped actions must be refused");
    assert!(!error.contains("model binding"), "{error}");
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn historical_fragments_keep_tokens_but_never_publish_past_handlers() {
    let e = encoder("Qwen__Qwen3.6-27B");
    let src = format!(
        r#"import {{ Qwen36App }} from {:?};
import {{ TurnStrategy }} from {:?};
export default function Main() {{
  const old = asChatTools(<aff label="old" onChange={{({{text}}) => ({{selected:"old"}})}}>{{"Old snapshot."}}</aff>);
  return <Qwen36App><TurnStrategy strategy="unrolled" log={{[{{kind:"screen",content:old.tree}}]}}>
    <aff label="live" onChange={{({{text}}) => ({{selected:text}})}}>{{"Current snapshot."}}</aff>
  </TurnStrategy></Qwen36App>;
}}
"#,
        root().join("views/template_apps/qwen36.tsx").to_str().unwrap(),
        root().join("views/strategies/turn_strategy.tsx").to_str().unwrap()
    );
    let v = temp_view("historical_fragment", &src, &e);
    let r = render(&v, &e, &json!({})).unwrap();
    let text = e.tok.decode(&r.ids, false).unwrap();
    assert!(text.contains("Old snapshot.") && text.contains("Current snapshot."));
    assert_eq!(r.spans.iter().filter(|s| s.meta["projected_tool"] == true).count(), 1);
    assert!(
        v.act(&e, state(&json!({})), now(), "change__old", 0, &json!({"text":"past"}))
            .is_err()
    );
    assert_eq!(
        v.act(
            &e,
            state(&json!({})),
            now(),
            "change__live",
            0,
            &json!({"text":"current"})
        )
        .unwrap()
        .state["selected"],
        "current"
    );
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn nemotron_port_matches_independent_thinking_tools_and_history_audit() {
    let e = encoder("nvidia__NVIDIA-Nemotron-3-Nano-30B-A3B-BF16");
    let v = View::load(&root().join("views/nemotron/nemotron3.tsx"), "Nemotron3", &e).unwrap();
    let cases: J =
        serde_json::from_str(&std::fs::read_to_string(root().join("ref/middle_out/nemotron_audit.json")).unwrap())
            .unwrap();
    assert_eq!(cases.as_array().unwrap().len(), 120);
    for (i, case) in cases.as_array().unwrap().iter().enumerate() {
        let s = affordance::state_from_json(&case["case"], &json!({}));
        let r = v
            .render_chat_opts(&e, s, &mut RunCache::new(true), now(), false)
            .unwrap();
        let expected: Vec<u32> = serde_json::from_value(case["ids"].clone()).unwrap();
        let mismatch = r.ids.iter().zip(&expected).position(|(a, b)| a != b);
        assert!(
            r.ids == expected,
            "Nemotron audit {i}, mismatch {mismatch:?}, lengths {} / {}",
            r.ids.len(),
            expected.len()
        );
    }
}

/// A conversation with nothing on the screen is the official template's prompt for the same
/// messages: no empty user turn after the last message, and the generation prompt follows it.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn an_empty_screen_adds_no_turn() {
    let e = encoder("Qwen__Qwen3.6-27B");
    let views = root().join("views");
    let app = temp_view(
        "screenless",
        &format!(
            r#"import {{ TurnStrategy }} from "{0}/strategies/turn_strategy.tsx";
import {{ Qwen36App }} from "{0}/template_apps/qwen36.tsx";
export default function Main({{ log }}) {{
  return <Qwen36App system="Be brief."><TurnStrategy strategy="unrolled" log={{log}}>{{""}}</TurnStrategy></Qwen36App>;
}}"#,
            views.display()
        ),
        &e,
    );
    let official = View::load(&views.join("qwen/qwen35.tsx"), "Qwen36", &e).unwrap();
    let conversations = [
        // Ends with the user's message.
        (
            json!([{"kind":"input","content":"hello"}, {"kind":"model","content":"hi","reasoning":"","calls":[]}, {"kind":"input","content":"again"}]),
            json!([{"role":"system","content":"Be brief."}, {"role":"user","content":"hello"}, {"role":"assistant","content":"hi"}, {"role":"user","content":"again"}]),
        ),
        // Only the first message.
        (
            json!([{"kind":"input","content":"hello"}]),
            json!([{"role":"system","content":"Be brief."}, {"role":"user","content":"hello"}]),
        ),
    ];
    for (log, messages) in conversations {
        let ours = render(&app, &e, &json!({"log": log})).unwrap();
        let theirs = render(
            &official,
            &e,
            &json!({"messages": messages, "tools": null, "add_generation_prompt": true, "enable_thinking": false}),
        )
        .unwrap();
        assert_eq!(
            e.tok.decode(&ours.ids, false).unwrap(),
            e.tok.decode(&theirs.ids, false).unwrap()
        );
        assert_eq!(ours.ids, theirs.ids);
        assert!(ours.current_turn().is_none());
    }
}
