//! `async function` / async arrows with host completions, concurrent Promise.all,
//! and the JavaScript regex builtin (`/re/`, `new RegExp`, test/exec/match/matchAll/
//! search/replace/replaceAll/split).
use affordance::View;
use affordance::effects::{Effect, Outcome, Request};
use affordance::emit::Encoder;
use serde_json::{Value as J, json};
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn now() -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 10, 9)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap()
}
fn state(s: &J) -> std::rc::Rc<affordance::value::Map> {
    affordance::value::state(s)
}
fn enc() -> Encoder {
    Encoder::from_file(root().join("ref/tokenizers/Qwen__Qwen3-8B.json").to_str().unwrap()).unwrap()
}
fn view(enc: &Encoder, source: &str) -> Result<View, String> {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "aff-regex-async-{}-{}",
        std::process::id(),
        N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("test.tsx"), source).unwrap();
    View::load(&dir.join("test.tsx"), "Main", enc)
}
fn act(enc: &Encoder, v: &View, log: Vec<Effect>) -> affordance::EffectAction {
    v.act_effects(enc, state(&json!({})), now(), "go", 0, &json!({}), &json!({}), log)
        .unwrap()
}
fn complete(mut log: Vec<Effect>, i: usize, value: J) -> Vec<Effect> {
    let at = log.iter().filter_map(|e| e.settled).max().unwrap_or(0) + 1;
    log[i].outcome = Some(Outcome::Ok(value));
    log[i].settled = Some(at);
    log
}
fn call_arg(e: &Effect) -> J {
    let Request::Call { args, .. } = &e.request else {
        panic!()
    };
    args.clone()
}

/// Evaluate `expr` inside a handler and return its JSON value.
fn eval(enc: &Encoder, helpers: &str, expr: &str) -> Result<J, String> {
    let v = view(
        enc,
        &format!(
            r#"{helpers}
export function Main({{}}) {{ return <aff name="go" onGo={{() => {{ return {{ out: {expr} }}; }}}}/>; }}"#
        ),
    )?;
    let run = v.act_effects(enc, state(&json!({})), now(), "go", 0, &json!({}), &json!({}), vec![])?;
    Ok(run.writes["out"].clone())
}

const SUB: &str = r#"
async function sub(q) {
  const a = await harness.machine.get("llm").call({ q: q, step: 1 });
  const b = await harness.machine.get("llm").call({ q: q, step: 2, prev: a });
  return q + ":" + a + "," + b;
}"#;

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs the test tokenizers: tools/fetch-test-tokenizers.sh")]
fn async_functions_issue_concurrent_requests_and_recover_in_completion_order() {
    let enc = enc();
    let v = view(
        &enc,
        &format!(
            r#"{SUB}
export async function both(qs) {{ return await Promise.all(qs.map(q => sub(q))); }}
export function Main({{}}) {{ return <aff name="go" onGo={{async () => {{
  const r = await both(["x", "y", "z"]);
  return {{ out: r }};
}}}}/>; }}"#
        ),
    )
    .unwrap();
    // First run: all three sub-model calls start before anything parks.
    let r1 = act(&enc, &v, vec![]);
    assert!(!r1.done);
    assert_eq!(r1.blocked, vec![0, 1, 2]);
    assert_eq!(
        r1.effects.iter().map(|e| call_arg(e)["q"].clone()).collect::<Vec<_>>(),
        vec![json!("x"), json!("y"), json!("z")]
    );
    // Complete out of order: z first. Only z's second step is added.
    let r2 = act(&enc, &v, complete(r1.effects, 2, json!("Z1")));
    assert_eq!(r2.effects.len(), 4);
    assert_eq!(call_arg(&r2.effects[3]), json!({"q": "z", "step": 2, "prev": "Z1"}));
    assert_eq!(r2.blocked, vec![0, 1, 3]);
    let r3 = act(&enc, &v, complete(complete(r2.effects, 0, json!("X1")), 1, json!("Y1")));
    assert_eq!(r3.effects.len(), 6);
    let mut log = r3.effects;
    for (i, val) in [(3, "Z2"), (4, "X2"), (5, "Y2")] {
        log = complete(log, i, json!(val));
    }
    let done = act(&enc, &v, log.clone());
    assert!(done.done);
    assert_eq!(done.writes["out"], json!(["x:X1,X2", "y:Y1,Y2", "z:Z1,Z2"]));
    // The log replays to the same result, and a changed request is still a divergence.
    assert_eq!(act(&enc, &v, log.clone()).writes, done.writes);
    let mut bad = log;
    bad[4].request = Request::Call {
        app: "machine:llm".into(),
        args: json!({"q": "x", "step": 2, "prev": "other"}),
        signal: None,
    };
    assert!(
        v.act_effects(&enc, state(&json!({})), now(), "go", 0, &json!({}), &json!({}), bad)
            .unwrap_err()
            .contains("diverged")
    );
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs the test tokenizers: tools/fetch-test-tokenizers.sh")]
fn async_arrow_and_root_requests_share_one_completion_transcript() {
    let enc = enc();
    let v = view(
        &enc,
        r#"export function Main({}) { return <aff name="go" onGo={async () => {
  const a = await harness.machine.get("bash").call({ c: 1 });
  const ask = async (p) => (await harness.machine.get("llm").call({ p: p })).text;
  const rs = await Promise.all([ask("u"), ask("v")]);
  const b = await harness.machine.get("bash").call({ c: 2 });
  return { out: [a, rs, b] };
}}/>; }"#,
    )
    .unwrap();
    let r = act(&enc, &v, vec![]);
    assert_eq!(r.effects.len(), 1);
    assert!(!serde_json::to_string(&r.effects[0]).unwrap().contains("key"));
    let r = act(&enc, &v, complete(r.effects, 0, json!("A")));
    assert_eq!(r.blocked, vec![1, 2]);
    let r = act(
        &enc,
        &v,
        complete(complete(r.effects, 2, json!({"text": "V"})), 1, json!({"text": "U"})),
    );
    assert_eq!(call_arg(&r.effects[3]), json!({"c":2}));
    let done = act(&enc, &v, complete(r.effects, 3, json!("B")));
    assert_eq!(done.writes["out"], json!(["A", ["U", "V"], "B"]));
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs the test tokenizers: tools/fetch-test-tokenizers.sh")]
fn unawaited_async_call_returns_a_promise_and_sync_body_settles() {
    let enc = enc();
    let v = view(
        &enc,
        r#"async function twice(x) { return x * 2; }
export function Main({}) { return <aff name="go" onGo={async () => {
  const p = twice(4);
  const v = await p;
  return { out: [v, await twice(5), typeof_ok(p)] };
}}/>; }
function typeof_ok(p) { return String(p); }"#,
    )
    .unwrap();
    let r = act(&enc, &v, vec![]);
    assert!(r.done);
    assert_eq!(r.writes["out"], json!([8, 10, "<promise>"]));
    // Outside a handler (render), async functions run synchronously.
    let v = view(
        &enc,
        r#"async function twice(x) { return x * 2; }
export function Main({}) { return <>{String(twice(21))}</>; }"#,
    )
    .unwrap();
    let mut cache = affordance::emit::RunCache::new(false);
    let out = v.render(&enc, state(&json!({})), &mut cache, now()).unwrap();
    assert_eq!(enc.tok.decode(&out.ids, false).unwrap(), "42");
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs the test tokenizers: tools/fetch-test-tokenizers.sh")]
fn regex_literals_and_constructor() {
    let e = enc();
    let ev = |x: &str| eval(&e, "", x).unwrap();
    assert_eq!(ev(r#"/a+b/.test("xaab")"#), json!(true));
    assert_eq!(ev(r#"new RegExp("A+B", "i").test("xaab")"#), json!(true));
    assert_eq!(ev(r#"RegExp("^b").test("ab")"#), json!(false));
    assert_eq!(
        ev(r#"[/a\/b/g.source, /x/gi.flags, /x/g.global, /x/.global, String(/a.b/m)]"#),
        json!(["a\\/b", "gi", true, false, "/a.b/m"])
    );
    assert_eq!(ev(r#"/(\d+)-(\d+)?/.exec("tel 12- x")"#), json!(["12-", "12", null]));
    assert_eq!(ev(r#"/z/.exec("abc")"#), json!(null));
    assert_eq!(
        ev(r#"/(?<y>\d{4})-(?<m>\d\d)/.exec("on 2026-10")"#),
        json!(["2026-10", "2026", "10"])
    );
    // `/` after an operand is still division (a JS number: 6 / 3 is 2).
    assert_eq!(ev("[6 / 3, (8) / 2 / 2]"), json!([2, 2]));
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs the test tokenizers: tools/fetch-test-tokenizers.sh")]
fn regex_string_methods_follow_js() {
    let e = enc();
    let ev = |x: &str| eval(&e, "", x).unwrap();
    assert_eq!(ev(r#""a1b22c333".match(/\d+/g)"#), json!(["1", "22", "333"]));
    assert_eq!(ev(r#""abc".match(/\d+/g)"#), json!(null));
    assert_eq!(ev(r#""baaab".match(/a*/g)"#), json!(["", "aaa", "", ""]));
    assert_eq!(
        ev(r#""k=v; x=y".matchAll(/(\w)=(\w)/g)"#),
        json!([["k=v", "k", "v"], ["x=y", "x", "y"]])
    );
    assert_eq!(ev(r#""xyz".search(/z/)"#), json!(2));
    assert_eq!(ev(r#""üüx".search(/x/)"#), json!(2));
    assert_eq!(ev(r#""a-b-c".replace(/-/, "+")"#), json!("a+b-c"));
    assert_eq!(ev(r#""a-b-c".replace(/-/g, "+")"#), json!("a+b+c"));
    assert_eq!(
        ev(r#""John Smith".replace(/(\w+)\s(\w+)/, "$2, $1 [$&] $$")"#),
        json!("Smith, John [John Smith] $")
    );
    assert_eq!(
        ev(r#""2026-10".replace(/(?<y>\d+)-(?<m>\d+)/, "$<m>/$<y>")"#),
        json!("10/2026")
    );
    assert_eq!(
        ev(r#""a1b2".replaceAll(/\d/g, (m, off) => "<" + m + String(off) + ">")"#),
        json!("a<11>b<23>")
    );
    assert_eq!(ev(r#""x.y.z".replaceAll(".", "-")"#), json!("x-y-z"));
    assert_eq!(ev(r#""a, b ,c".split(/\s*,\s*/)"#), json!(["a", "b", "c"]));
    assert_eq!(ev(r#""a1b2c".split(/(\d)/)"#), json!(["a", "1", "b", "2", "c"]));
    assert_eq!(ev(r#""abc".split(/(?:)/)"#), json!(["a", "b", "c"]));
    assert_eq!(ev(r#""".split(/,/)"#), json!([""]));
    assert_eq!(ev(r#""a,b,c".split(/,/, 2)"#), json!(["a", "b"]));
    assert_eq!(ev(r#"",a,".split(/,/)"#), json!(["", "a", ""]));
    // JS `.`, `\s`, `\d`, `\w`, `\b`, `[]` and `[^]`.
    assert_eq!(
        ev(
            r#"[/^.$/.test("\r"), /^.$/s.test("\n"), /\d/.test("٣"), /\w/.test("é"), /\s/.test(" "), /[]/.test("a"), /^[^]$/.test("\n")]"#
        ),
        json!([false, true, false, false, true, false, true])
    );
    assert_eq!(
        ev(r#"[/a{/.test("a{"), /x{2}/.test("xx"), /[a-c\]]+/.exec("z]ab")]"#),
        json!([true, true, ["]ab"]])
    );
    assert_eq!(ev(r#"/^b/m.test("a\nb")"#), json!(true));
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs the test tokenizers: tools/fetch-test-tokenizers.sh")]
fn unsupported_js_regex_features_are_errors() {
    let e = enc();
    for (src, what) in [
        (r#"/a(?=b)/"#, "lookahead"),
        (r#"/(?<=a)b/"#, "lookbehind"),
        (r#"/(?<!a)b/"#, "lookbehind"),
        (r#"/(a)\1/"#, "backreference"),
        (r#"/a/y"#, "flag 'y'"),
    ] {
        let err = eval(&e, "", &format!("{src}.test(\"ab\")")).unwrap_err();
        assert!(err.contains(what), "{src}: {err}");
    }
    let err = eval(&e, "", r#"new RegExp("(?=x)").test("x")"#).unwrap_err();
    assert!(err.contains("lookahead"), "{err}");
    assert!(
        eval(&e, "", r#""a".replaceAll(/a/, "b")"#)
            .unwrap_err()
            .contains("global")
    );
}
