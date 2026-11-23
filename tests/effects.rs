use affordance::View;
use affordance::effects::{Effect, Outcome, Request};
use affordance::emit::Encoder;
use serde_json::{Value as J, json};
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn now() -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 10, 8)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap()
}
fn state(s: &J) -> std::rc::Rc<affordance::value::Map> {
    affordance::value::state(s)
}
fn view(source: &str) -> (Encoder, View) {
    let enc = Encoder::from_file(root().join("ref/tokenizers/Qwen__Qwen3-8B.json").to_str().unwrap()).unwrap();
    // One directory per view: tests run in parallel and would otherwise overwrite each other's file.
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "affwindow-effects-{}-{}",
        std::process::id(),
        N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("test.tsx"), source).unwrap();
    let view = View::load(&dir.join("test.tsx"), "Main", &enc).unwrap();
    (enc, view)
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn spike_two_awaits_replay_and_reject_changed_request() {
    let (enc, v) = view(
        r#"export function Main({q}) {
        const [answer, setAnswer] = useState("");
        return <aff name="ask" onAsk={async ({x}) => {
            setAnswer("asking");
            const a = await harness.machine.get("model").call({prompt:q});
            const b = await harness.machine.get("calc").call({x:a.n});
            setAnswer(`${a.text} / ${b}`);
        }}>{answer}</aff>;
    }"#,
    );
    let s = json!({"q":"2+2?"});
    let start = || {
        v.act_effects(&enc, state(&s), now(), "ask", 0, &json!({}), &json!({}), vec![])
            .unwrap()
    };
    let first = start();
    assert!(!first.done);
    assert_eq!(first.blocked, vec![0]);
    assert!(
        first.writes["__ui"]
            .as_object()
            .unwrap()
            .values()
            .any(|v| v == "asking")
    );
    let mut log = first.effects;
    log[0].outcome = Some(Outcome::Ok(json!({"text":"four","n":4})));
    let second = v
        .act_effects(&enc, state(&s), now(), "ask", 0, &json!({}), &json!({}), log)
        .unwrap();
    assert_eq!(second.blocked, vec![1]);
    let mut log = second.effects;
    log[1].outcome = Some(Outcome::Ok(json!(40)));
    let final_run = v
        .act_effects(&enc, state(&s), now(), "ask", 0, &json!({}), &json!({}), log.clone())
        .unwrap();
    assert!(final_run.done);
    assert!(
        final_run.writes["__ui"]
            .as_object()
            .unwrap()
            .values()
            .any(|v| v == "four / 40")
    );
    let replay = v
        .act_effects(&enc, state(&s), now(), "ask", 0, &json!({}), &json!({}), log.clone())
        .unwrap();
    assert_eq!(final_run.writes, replay.writes);
    log[0].request = Request::Call {
        app: "machine:model".into(),
        args: json!({"prompt":"wrong"}),
        signal: None,
    };
    assert!(
        v.act_effects(&enc, state(&s), now(), "ask", 0, &json!({}), &json!({}), log)
            .unwrap_err()
            .contains("diverged")
    );
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn all_starts_every_call_before_parking_and_keeps_order() {
    let (enc, v) = view(
        r#"export function Main({}) { return <aff name="go" onGo={async () => {
        const results = await Promise.all([harness.machine.get("a").call({}),harness.machine.get("b").call({}),7]);
        return {results:results};
    }}/>; }"#,
    );
    let run = v
        .act_effects(&enc, state(&json!({})), now(), "go", 0, &json!({}), &json!({}), vec![])
        .unwrap();
    assert_eq!(run.blocked, vec![0, 1]);
    let log = run
        .effects
        .into_iter()
        .enumerate()
        .map(|(i, e)| Effect {
            outcome: Some(Outcome::Ok(json!(i))),
            ..e
        })
        .collect();
    let done = v
        .act_effects(&enc, state(&json!({})), now(), "go", 0, &json!({}), &json!({}), log)
        .unwrap();
    assert!(done.done);
    assert_eq!(done.writes["results"], json!([0, 1, 7]));
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn fire_and_forget_is_recorded_without_await() {
    let (enc, v) = view(
        r#"export function Main({}) { return <aff name="go" onGo={() => {
        harness.machine.get("summary").call({text:"content"});
        return {status:"working"};
    }}/>; }"#,
    );
    let run = v
        .act_effects(&enc, state(&json!({})), now(), "go", 0, &json!({}), &json!({}), vec![])
        .unwrap();
    assert!(run.done);
    assert_eq!(run.effects.len(), 1);
    assert_eq!(run.blocked, vec![0]);
    assert_eq!(
        run.effects[0].request,
        Request::Call {
            app: "machine:summary".into(),
            args: json!({"text":"content"}),
            signal: None
        }
    );
}
