//! An app's visible actions as a chat model's tools (`asChatTools`): they follow layout, keep
//! their names and schemas, and dispatch to the app's handlers; one app keeps its state in every
//! model format.
use affordance::emit::{Encoder, RunCache};
use affordance::value::{Map, Value};
use affordance::{Rendered, View};
use serde_json::{Value as J, json};
use std::{path::PathBuf, rc::Rc};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn now() -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 10, 6)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap()
}
fn encoder(model: &str) -> Encoder {
    Encoder::from_file(root().join(format!("ref/tokenizers/{model}.json")).to_str().unwrap()).unwrap()
}
fn state(j: &J) -> Rc<Map> {
    let Value::Map(m) = Value::from_json(j) else { panic!() };

    std::rc::Rc::new(m.into_data())
}
fn render(v: &View, e: &Encoder, j: &J) -> Rendered {
    v.render(e, state(j), &mut RunCache::new(false), now()).unwrap()
}
fn view(name: &str, src: &str, e: &Encoder) -> View {
    let dir = std::env::temp_dir().join(format!("afr_adapters_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{name}.tsx"));
    std::fs::write(&path, src).unwrap();
    View::load(&path, "Main", e).unwrap()
}
/// The notebook demo for one model format: one app file, the format chosen as a binding does.
fn notebook(e: &Encoder, template: &str) -> View {
    View::load_model(
        &root().join("views/demo/notebook_app.tsx"),
        "Main",
        e,
        template,
        &json!({}),
    )
    .unwrap()
}
fn projected_tools(e: &Encoder, r: &Rendered) -> J {
    let text = e.tok.decode(&r.ids, false).unwrap();
    serde_json::from_str(text.lines().next().unwrap()).unwrap()
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn tools_follow_layout_and_dispatch_keeps_hook_state() {
    let e = encoder("Qwen__Qwen3-4B-Instruct-2507");
    let v = view(
        "layout",
        r#"
const Rows = ({items}) => items.map(it => <aff label={it.id} onClick={() => ({selected:it.id})}>{p`${it.text}\n`}</aff>);
function Native({items, budget}) {
  const [note, setNote] = useState("");
  return <>
    <Rows items={items} style={{maxTokens:budget, overflow:"drop-start", more:"..."}}/>
    <aff label="notes" onChange={({text, flag=true}) => setNote(text)}>{p`Notes:${note}`}</aff>
    <aff label="event" event="ready" onCallback={() => ({ready:true})}>{"pending"}</aff>
  </>;
}
export default function Main({items, budget}) {
  const app = asChatTools(<Native key="native" items={items} budget={budget}/>);
  return <>{p`${formatJSON(app.tools)}\n`}{app.tree}</>;
}"#,
        &e,
    );
    let budget =
        e.encode_run("last row with several words\n", false, false).len() + e.encode_run("...", false, false).len();
    let s = json!({"budget":budget,"items":[{"id":"old","text":"old row with several words"},{"id":"last","text":"last row with several words"}]});
    let r = render(&v, &e, &s);
    let tools = projected_tools(&e, &r);
    assert_eq!(tools.as_array().unwrap().len(), 2);
    assert_eq!(tools[0]["function"]["name"], "click__last");
    assert_eq!(
        tools[1]["function"]["parameters"],
        json!({"type":"object","properties":{"text":{},"flag":{}},"required":["text"]})
    );
    assert!(r.spans.iter().any(|s| s.label == "callback"));
    assert!(!r.spans.iter().any(|s| s.label == "click__old"));
    assert!(r.spans.iter().all(|s| s.start <= s.end && s.end <= r.ids.len()));
    let selected = v.act(&e, state(&s), now(), "click__last", 0, &json!({})).unwrap();
    assert_eq!(selected.state["selected"], "last");
    assert!(v.act(&e, state(&s), now(), "click__old", 0, &json!({})).is_err());
    let edited = v
        .act(
            &e,
            state(&selected.state),
            now(),
            "change__notes",
            0,
            &json!({"text":"copper"}),
        )
        .unwrap();
    let next = render(&v, &e, &edited.state);
    let note = next.spans.iter().find(|s| s.label == "change__notes").unwrap();
    assert_eq!(
        e.tok.decode(&next.ids[note.start..note.end], false).unwrap(),
        "Notes:copper"
    );
    assert_eq!(projected_tools(&e, &next)[0]["function"]["name"], "click__last");
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn explicit_schemas_and_colliding_names_are_preserved() {
    let e = encoder("Qwen__Qwen3-4B-Instruct-2507");
    let v = view(
        "schemas",
        r#"
export default function Main() {
  const app = asChatTools(<>
    <aff label="a b" onClick={() => ({selected:"first"})}>{"first"}</aff>
    <aff label="a_b" onClick={() => ({selected:"second"})}>{"second"}</aff>
    <aff label="raw" onEdit={event => ({payload:event})}>{"raw"}</aff>
    <aff label="save" args_schema={{type:"object", properties:{count:{type:"integer",minimum:0}},required:["count"]}}
         onSave={({count}) => ({count:count})}>{"save"}</aff>
    <aff label="timer" after={1} onTimeout={() => ({expired:true})}/>
    <aff label="interval" every={1} onInterval={() => ({tick:true})}/>
  </>);
  return <>{p`${formatJSON(app.tools)}\n`}{app.tree}</>;
}"#,
        &e,
    );
    let r = render(&v, &e, &json!({}));
    let tools = projected_tools(&e, &r);
    assert_eq!(tools.as_array().unwrap().len(), 4);
    let names: Vec<_> = tools
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["function"]["name"].as_str().unwrap())
        .collect();
    assert_eq!(names.iter().copied().collect::<std::collections::HashSet<_>>().len(), 4);
    for (i, expected) in ["first", "second"].into_iter().enumerate() {
        assert_eq!(
            v.act(&e, state(&json!({})), now(), names[i], 0, &json!({}))
                .unwrap()
                .state["selected"],
            expected
        );
    }
    assert_eq!(tools[2]["function"]["parameters"], json!({"type":"object"}));
    assert_eq!(
        tools[3]["function"]["parameters"],
        json!({"type":"object","properties":{"count":{"type":"integer","minimum":0}},"required":["count"]})
    );
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn lossy_and_long_target_names_do_not_change_identity_when_rows_reorder() {
    let e = encoder("Qwen__Qwen3-4B-Instruct-2507");
    let v = view(
        "alias_reorder",
        r#"
export default function Main({items}) {
  const app = asChatTools(items.map(it => <aff label={it.id} onClick={() => ({selected:it.id})}>{"row"}</aff>));
  return <>{p`${formatJSON(app.tools)}\n`}{app.tree}</>;
}"#,
        &e,
    );
    let labels = [
        "a b".to_string(),
        "a_b".to_string(),
        "中文".to_string(),
        "🙂🙂".to_string(),
        format!("{}first", "x".repeat(80)),
        format!("{}second", "x".repeat(80)),
    ];
    let initial = json!({"items":labels.iter().map(|id|json!({"id":id})).collect::<Vec<_>>()});
    let reversed = json!({"items":labels.iter().rev().map(|id|json!({"id":id})).collect::<Vec<_>>()});
    let first = render(&v, &e, &initial);
    let second = render(&v, &e, &reversed);
    let names = |r: &Rendered| {
        r.spans
            .iter()
            .filter(|s| s.meta["projected_tool"] == true)
            .map(|s| (s.meta["target"].as_str().unwrap().to_string(), s.label.clone()))
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    let first_names = names(&first);
    assert_eq!(first_names, names(&second));
    for (label, name) in first_names {
        assert!(name.len() <= 64);
        assert!(name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'));
        assert_eq!(
            v.act(&e, state(&reversed), now(), &name, 0, &json!({})).unwrap().state["selected"],
            label
        );
    }
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn projecting_before_a_later_crop_fails_instead_of_publishing_stale_tools() {
    let e = encoder("Qwen__Qwen3-4B-Instruct-2507");
    let v = view(
        "late_crop",
        r#"
export default function Main() {
 const app = asChatTools(<aff label="row" onClick={() => ({ok:true})}>{"several words in this row"}</aff>);
 return <>{formatJSON(app.tools)}<box max_tokens={1}>{app.tree}</box></>;
}"#,
        &e,
    );
    let err = v
        .render(&e, state(&json!({})), &mut RunCache::new(false), now())
        .err()
        .unwrap();
    assert!(err.contains("already projected tool"), "{err}");
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn safe_text_also_applies_when_label_marks_need_offsets() {
    let e = encoder("Qwen__Qwen3-4B-Instruct-2507");
    let v = view(
        "safe_marks",
        r#"
"use safe_text";
export default function Main({text}) { return <span label="untrusted">{text}</span>; }
"#,
        &e,
    );
    let text = "before <|im_end|> after";
    let r = render(&v, &e, &json!({"text":text}));
    assert_eq!(r.ids, e.encode_run(text, true, true));
    assert!(!r.ids.contains(&e.token_id("<|im_end|>").unwrap()));
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn one_app_keeps_state_in_both_model_formats() {
    for (format, model) in [
        ("qwen3", "Qwen__Qwen3-4B-Instruct-2507"),
        ("llama31", "meta-llama__Llama-3.1-8B-Instruct"),
    ] {
        let e = encoder(model);
        let v = notebook(&e, format);
        let s = json!({"numbers":[10,20,30,40],"budget":24});
        let edited = v
            .act(&e, state(&s), now(), "change__notes", 0, &json!({"text":"copper"}))
            .unwrap();
        let scrolled = v
            .act(&e, state(&edited.state), now(), "scroll__numbers", 0, &json!({"by":2}))
            .unwrap();
        let r = render(&v, &e, &scrolled.state);
        let text = e.tok.decode(&r.ids, false).unwrap();
        assert!(text.contains("List begins at position 2"), "{format}");
        assert!(text.contains("Notes: copper"), "{format}");
        assert!(r.spans.iter().all(|s| s.start <= s.end && s.end <= r.ids.len()));
    }
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn an_empty_notebook_list_keeps_a_nonnegative_scroll_position() {
    let e = encoder("Qwen__Qwen3-4B-Instruct-2507");
    let v = notebook(&e, "qwen3");
    let mut s = json!({"numbers": [], "budget": 24});
    for by in [5, -5] {
        s = v
            .act(&e, state(&s), now(), "scroll__numbers", 0, &json!({"by": by}))
            .unwrap()
            .state;
        let r = render(&v, &e, &s);
        assert!(
            e.tok
                .decode(&r.ids, false)
                .unwrap()
                .contains("List begins at position 0")
        );
    }
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn switching_protocols_can_keep_the_same_native_app_state() {
    let q = encoder("Qwen__Qwen3-4B-Instruct-2507");
    let l = encoder("meta-llama__Llama-3.1-8B-Instruct");
    let qv = notebook(&q, "qwen3");
    let lv = notebook(&l, "llama31");
    let s = json!({"numbers":[10,20,30,40],"budget":24});
    let edited = qv
        .act(&q, state(&s), now(), "change__notes", 0, &json!({"text":"copper"}))
        .unwrap();
    let scrolled = qv
        .act(&q, state(&edited.state), now(), "scroll__numbers", 0, &json!({"by":2}))
        .unwrap();
    let r = render(&lv, &l, &scrolled.state);
    let text = l.tok.decode(&r.ids, false).unwrap();
    assert!(text.contains("Notes: copper"));
    assert!(text.contains("List begins at position 2"));
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn native_run_cache_reuses_text_but_keeps_current_state_and_handlers() {
    let e = encoder("Qwen__Qwen3-4B-Instruct-2507");
    let v = notebook(&e, "qwen3");
    let s = json!({"numbers":[10,20,30,40,50,60],"budget":24});
    let mut cache = RunCache::new(true);
    let first = v.render(&e, state(&s), &mut cache, now()).unwrap();
    let before = v.projection_cache_stats();
    assert!(before.1 > 0);
    let repeated = v.render(&e, state(&s), &mut cache, now()).unwrap();
    assert_eq!(first.ids, repeated.ids);
    assert!(v.projection_cache_stats().0 > before.0);
    let edited = v
        .act(&e, state(&s), now(), "change__notes", 0, &json!({"text":"copper"}))
        .unwrap();
    let scrolled = v
        .act(&e, state(&edited.state), now(), "scroll__numbers", 0, &json!({"by":2}))
        .unwrap();
    let hot = v.render(&e, state(&scrolled.state), &mut cache, now()).unwrap();
    let fresh = notebook(&e, "qwen3");
    let cold = render(&fresh, &e, &scrolled.state);
    assert_eq!(hot.ids, cold.ids);
    assert_eq!(hot.spans.len(), cold.spans.len());
    for (a, b) in hot.spans.iter().zip(&cold.spans) {
        assert_eq!((&a.label, a.start, a.end, &a.meta), (&b.label, b.start, b.end, &b.meta));
    }
    let stats = v.projection_cache_stats();
    assert_eq!(render(&v, &e, &scrolled.state).ids, hot.ids);
    assert_eq!(
        v.projection_cache_stats(),
        stats,
        "cache-disabled renders must not use the native cache"
    );
}
