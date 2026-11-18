//! Independent app composition contracts, also runnable on cheap-tier snapshots.
use affordance::{
    Rendered, View,
    emit::{Encoder, RunCache},
    value::Value,
};
use serde_json::{Value as J, json};
use std::{collections::BTreeMap, path::PathBuf, rc::Rc};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn enc() -> Encoder {
    Encoder::from_file(root().join("ref/tokenizers/Qwen__Qwen3-8B.json").to_str().unwrap()).unwrap()
}
fn now() -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 10, 6)
        .unwrap()
        .and_hms_opt(14, 0, 0)
        .unwrap()
}
fn state(j: &J) -> Rc<affordance::value::Map> {
    let Value::Map(m) = Value::from_json(j) else { panic!() };

    std::rc::Rc::new(m.into_data())
}
fn view(name: &str, src: &str, e: &Encoder) -> View {
    let dir = std::env::temp_dir().join(format!("afr_contract_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{name}.tsx"));
    std::fs::write(&path, src).unwrap();
    View::load(&path, "Main", e).unwrap()
}
fn render(v: &View, e: &Encoder, j: &J) -> Result<Rendered, String> {
    v.render(e, state(j), &mut RunCache::new(false), now())
}
fn tools(e: &Encoder, r: &Rendered) -> J {
    serde_json::from_str(e.tok.decode(&r.ids, false).unwrap().lines().next().unwrap()).unwrap()
}
fn names(t: &J) -> Vec<String> {
    t.as_array()
        .unwrap()
        .iter()
        .map(|t| t["function"]["name"].as_str().unwrap().to_string())
        .collect()
}
fn model_name(s: &affordance::emit::Span) -> &str {
    s.meta["tool"].as_str().unwrap_or(&s.label)
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn reusing_a_captured_tree_keeps_every_declared_tool_dispatchable() {
    let e = enc();
    let v = view(
        "reused",
        r#"
export default function Main() {
  const row=<aff label="row" onClick={() => ({picked:true})}>{"row"}</aff>;
  const app=asChatTools(<>{row}{row}</>);
  return <>{p`${formatJSON(app.tools)}\n`}{app.tree}</>;
}"#,
        &e,
    );
    let r = render(&v, &e, &json!({})).unwrap();
    let tool_names = names(&tools(&e, &r));
    assert!(!tool_names.is_empty());
    for name in tool_names {
        assert!(
            r.spans
                .iter()
                .any(|s| model_name(s) == name && s.meta.get("handler").is_some()),
            "declared tool {name} has no handler span"
        );
        assert_eq!(
            v.act(&e, state(&json!({})), now(), &name, 0, &json!({})).unwrap().state["picked"],
            true
        );
    }
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn later_parent_layout_cannot_publish_hidden_tools() {
    let e = enc();
    let v = view(
        "later_crop",
        r#"
export default function Main() {
  const app=asChatTools(<aff label="row" onClick={() => ({picked:true})}>{"several words in a row"}</aff>);
  return <>{p`${formatJSON(app.tools)}\n`}<box max_tokens={0}>{app.tree}</box></>;
}"#,
        &e,
    );
    match render(&v, &e, &json!({})) {
        Err(error) => assert!(
            error.contains("crop") || error.contains("budget") || error.contains("project"),
            "{error}"
        ),
        Ok(r) => {
            for name in names(&tools(&e, &r)) {
                assert!(
                    r.spans
                        .iter()
                        .any(|s| model_name(s) == name && s.meta.get("handler").is_some()),
                    "declared tool {name} was hidden by final layout"
                );
            }
        }
    }
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn named_targets_keep_the_same_alias_after_reordering() {
    let e = enc();
    let v = view(
        "reorder",
        r#"
export default function Main({items}) {
  const app=asChatTools(<>{items.map(it => <aff label={it} onClick={() => ({picked:it})}>{it}</aff>)}</>);
  return <>{p`${formatJSON(app.tools)}\n`}{app.tree}</>;
}"#,
        &e,
    );
    let first = json!({"items":["left","right"]});
    let second = json!({"items":["right","left"]});
    let mapping = |r: &Rendered| {
        r.spans
            .iter()
            .filter(|s| s.meta.get("handler").is_some())
            .map(|s| {
                (
                    s.meta["target"].as_str().unwrap().to_string(),
                    model_name(s).to_string(),
                )
            })
            .collect::<BTreeMap<_, _>>()
    };
    let before = mapping(&render(&v, &e, &first).unwrap());
    assert_eq!(before, mapping(&render(&v, &e, &second).unwrap()));
    for (target, name) in before {
        assert_eq!(
            v.act(&e, state(&second), now(), &name, 0, &json!({})).unwrap().state["picked"],
            target
        );
    }
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn one_object_parameter_does_not_invent_a_property_named_after_the_variable() {
    let e = enc();
    let v = view(
        "object_arg",
        r#"
export default function Main() {
  const app=asChatTools(<aff label="input" onEdit={event => ({payload:event})}>{"input"}</aff>);
  return <>{p`${formatJSON(app.tools)}\n`}{app.tree}</>;
}"#,
        &e,
    );
    let r = render(&v, &e, &json!({})).unwrap();
    let t = tools(&e, &r);
    assert!(t[0]["function"]["parameters"]["properties"].get("event").is_none());
    let name = names(&t).remove(0);
    let args = json!({"text":"copper"});
    assert_eq!(
        v.act(&e, state(&json!({})), now(), &name, 0, &args).unwrap().state["payload"],
        args
    );
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn ordinary_nested_array_and_number_children_stay_valid_in_an_adapter() {
    let e = enc();
    let v = view(
        "nested",
        r#"
export default function Main() {
  const children=[["hello"],[3],null,false];
  const app=asChatTools(<>{children}</>);
  return <>{p`${formatJSON(app.tools)}\n`}{app.tree}</>;
}"#,
        &e,
    );
    let r = render(&v, &e, &json!({})).unwrap();
    assert_eq!(tools(&e, &r), json!([]));
    assert!(e.tok.decode(&r.ids, false).unwrap().ends_with("hello3"));
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn passive_projection_keeps_prompt_and_schemas_without_callable_spans() {
    let e = enc();
    let v = view(
        "passive",
        r#"
export default function Main({callable}) {
  const screen=<><aff onRun={({q})=>q}>first</aff><aff onRun={()=>'ok'}>second</aff>
    <aff style={{display:'none'}} onRun={()=>'hidden'}>hidden</aff>
    <aff event='message' onCallback={()=>'event'}/></>;
  const app=asChatTools(screen,{callable});
  return <>{formatJSON(app.tools)}{"\n"}{app.tree}</>;
}"#,
        &e,
    );
    let mut cache = RunCache::new(true);
    let mut active = None;
    for callable in [true, false, true, false] {
        let r = v
            .render(&e, state(&json!({"callable":callable})), &mut cache, now())
            .unwrap();
        let schemas = tools(&e, &r);
        assert_eq!(schemas.as_array().unwrap().len(), 2);
        let actions = affordance::reply::ActionSet::from_spans(&r.spans).unwrap();
        assert_eq!(actions.actions.len(), if callable { 2 } else { 0 });
        if !callable {
            assert!(
                r.spans
                    .iter()
                    .filter(|s| !affordance::is_harness_callback(&s.label))
                    .all(|s| s.meta.get("handler").is_none())
            );
        }
        match &active {
            None => active = Some((r.ids, schemas)),
            Some((ids, tools)) => {
                assert_eq!(&r.ids, ids);
                assert_eq!(&schemas, tools);
            }
        }
    }
    for bad in [
        json!(null),
        json!({"callable":1}),
        json!({"callable":false,"typo":true}),
    ] {
        assert!(render(&v, &e, &json!({"callable":bad})).is_err());
    }
}
