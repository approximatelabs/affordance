use crate::shared;
use affordance::{
    View,
    emit::{Encoder, RunCache},
    value::Value,
};
use serde_json::{Value as J, json};
use std::{collections::BTreeSet, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn encoder() -> std::sync::Arc<Encoder> {
    shared::tokenizer(&root().join("ref/tokenizers/Qwen__Qwen3-8B.json"))
}
fn now() -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 10, 6)
        .unwrap()
        .and_hms_opt(15, 0, 0)
        .unwrap()
}
fn state() -> J {
    serde_json::from_str(&std::fs::read_to_string(root().join("views/apps/courier_state.json")).unwrap()).unwrap()
}
fn load(enc: &Encoder) -> View {
    View::load(&root().join("views/apps/courier.tsx"), "CourierApp", enc).unwrap()
}
fn actions(view: &View, enc: &Encoder, s: &J) -> BTreeSet<String> {
    let Value::Map(props) = Value::from_json(s) else {
        panic!()
    };
    let props = std::rc::Rc::new(props.into_data());
    let r = view.render(enc, props, &mut RunCache::new(true), now()).unwrap();
    assert!(r.spans.iter().all(|sp| sp.start <= sp.end && sp.end <= r.ids.len()));
    r.spans
        .into_iter()
        .filter(|sp| sp.meta.get("handler").is_some())
        .map(|sp| sp.label)
        .collect()
}
fn act(view: &View, enc: &Encoder, s: &J, action: &str) -> J {
    let Value::Map(props) = Value::from_json(s) else {
        panic!()
    };
    let props = std::rc::Rc::new(props.into_data());
    view.act(enc, props, now(), action, 0, &json!({})).unwrap().state
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs the test tokenizers: tools/fetch-test-tokenizers.sh")]
fn layouts_preserve_actions_and_state_transitions() {
    let enc = encoder();
    let view = load(&enc);
    for layout in ["map", "coordinates", "rows"] {
        let mut s = state();
        s["layout"] = json!(layout);
        assert_eq!(actions(&view, &enc, &s), ["move".to_string()].into_iter().collect());
        // Native handlers share a name and dispatch with the `which` occurrence.
        let Value::Map(props) = Value::from_json(&s) else {
            panic!()
        };
        let props = std::rc::Rc::new(props.into_data());
        for (index, agent) in [(0, json!({"x":2,"y":1})), (1, json!({"x":1,"y":2}))] {
            let after = view
                .act(&enc, props.clone(), now(), "move", index, &json!({}))
                .unwrap()
                .state;
            assert_eq!(after["agent"], agent);
            assert_eq!(after["carrying"], false);
        }
    }
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs the test tokenizers: tools/fetch-test-tokenizers.sh")]
fn entering_parcel_picks_it_up_and_entering_depot_finishes() {
    let enc = encoder();
    let view = load(&enc);
    let mut s = state();
    s["agent"] = json!({"x":5,"y":2});
    let after = act(&view, &enc, &s, "move");
    // North appears first here. Dispatch the south occurrence explicitly.
    let Value::Map(props) = Value::from_json(&s) else {
        panic!()
    };
    let props = std::rc::Rc::new(props.into_data());
    let after_south = view.act(&enc, props, now(), "move", 1, &json!({})).unwrap().state;
    assert_eq!(after_south["agent"], s["parcel"]);
    assert_eq!(after_south["carrying"], true);
    assert_eq!(after["agent"], json!({"x":5,"y":1}));
    s["agent"] = json!({"x":1,"y":4});
    s["carrying"] = json!(true);
    let Value::Map(props) = Value::from_json(&s) else {
        panic!()
    };
    let props = std::rc::Rc::new(props.into_data());
    let complete = view.act(&enc, props, now(), "move", 1, &json!({})).unwrap().state;
    assert_eq!(complete["agent"], s["depot"]);
    assert!(actions(&view, &enc, &complete).is_empty());
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn projected_actions_keep_distinct_targets_in_both_protocols() {
    for (model, template) in [
        ("Qwen__Qwen3-8B", "qwen3"),
        ("meta-llama__Llama-3.1-8B-Instruct", "llama31"),
    ] {
        let enc = shared::tokenizer(&root().join(format!("ref/tokenizers/{model}.json")));
        let view = View::load_model(
            &root().join("views/apps/courier.tsx"),
            "CourierChat",
            &enc,
            template,
            &json!({}),
        )
        .unwrap();
        let s = state();
        assert_eq!(
            actions(&view, &enc, &s),
            ["move__east", "move__south"].into_iter().map(str::to_string).collect()
        );
        assert_eq!(act(&view, &enc, &s, "move__south")["agent"], json!({"x":1,"y":2}));
    }
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs the test tokenizers: tools/fetch-test-tokenizers.sh")]
fn history_display_choices_are_causal_and_native_handlers_append() {
    let enc = encoder();
    let view = load(&enc);
    let mut s = state();
    let moves = [(1, 2, "east"), (2, 3, "east"), (3, 2, "west"), (2, 1, "west")];
    s["history"] = json!(
        moves
            .iter()
            .map(|(from, to, action)| json!({
                "from":{"x":from,"y":1}, "to":{"x":to,"y":1}, "action":action,
                "carrying_before":false,"carrying_after":false
            }))
            .collect::<Vec<_>>()
    );
    s["history_n"] = json!(2);
    for mode in ["none", "pinned", "all", "last_n"] {
        s["history_mode"] = json!(mode);
        let Value::Map(props) = Value::from_json(&s) else {
            panic!()
        };
        let props = std::rc::Rc::new(props.into_data());
        let rendered = view
            .render(&enc, props.clone(), &mut RunCache::new(true), now())
            .unwrap();
        let text = enc.tok.decode(&rendered.ids, false).unwrap();
        assert_eq!(text.contains("Completed moves"), mode != "none");
        assert_eq!(text.contains("1,1 east 2,1;"), mode == "all" || mode == "pinned");
        assert_eq!(text.contains("3,1 west 2,1;"), mode == "all" || mode == "last_n");
        let after = view.act(&enc, props, now(), "move", 0, &json!({})).unwrap().state;
        let history = after["history"].as_array().unwrap();
        assert_eq!(history.len(), if mode == "none" { 4 } else { 5 });
        if mode != "none" {
            assert_eq!(
                history[4],
                json!({"from":s["agent"],"to":after["agent"],"action":"east",
                "carrying_before":false,"carrying_after":false})
            );
        }
    }
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs the test tokenizers: tools/fetch-test-tokenizers.sh")]
fn pinned_history_retains_the_public_pickup_event() {
    let enc = encoder();
    let view = load(&enc);
    let mut s = state();
    s["agent"] = json!({"x":5,"y":4});
    s["carrying"] = json!(true);
    s["history_mode"] = json!("pinned");
    let route = [
        (1, 1, 2, 1, "east", false, false),
        (2, 1, 3, 1, "east", false, false),
        (3, 1, 4, 1, "east", false, false),
        (4, 1, 5, 1, "east", false, false),
        (5, 1, 5, 2, "south", false, false),
        (5, 2, 5, 3, "south", false, true),
        (5, 3, 5, 4, "south", true, true),
    ];
    s["history"] = json!(
        route
            .iter()
            .map(|(x, y, nx, ny, action, before, after)| json!({
                "from":{"x":x,"y":y},"to":{"x":nx,"y":ny},"action":action,
                "carrying_before":before,"carrying_after":after
            }))
            .collect::<Vec<_>>()
    );
    let Value::Map(props) = Value::from_json(&s) else {
        panic!()
    };
    let props = std::rc::Rc::new(props.into_data());
    let rendered = view.render(&enc, props, &mut RunCache::new(true), now()).unwrap();
    let text = enc.tok.decode(&rendered.ids, false).unwrap();
    assert!(text.contains("1,1 east 2,1;") && text.contains("5,2 south 5,3*;"));
    assert!(!text.contains("5,3 south 5,4*;"));
}
