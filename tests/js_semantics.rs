//! Views have JavaScript value semantics. Each case is a JS expression and what JavaScript itself
//! prints for it.
use affordance::View;
use affordance::emit::{Encoder, RunCache};
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn now() -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 10, 13)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap()
}
fn enc() -> Encoder {
    Encoder::from_file(
        root()
            .join("ref/tokenizers/Qwen__Qwen3-4B-Instruct-2507.json")
            .to_str()
            .unwrap(),
    )
    .unwrap()
}

/// Render `src` (a module whose `Main` returns a string) and return the text.
fn run(enc: &Encoder, name: &str, src: &str) -> Result<String, String> {
    let dir = std::env::temp_dir().join(format!("aff-js-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{name}.tsx"));
    std::fs::write(&path, src).unwrap();
    let v = View::load(&path, "Main", enc)?;
    let r = v.render(
        enc,
        std::rc::Rc::new(Default::default()),
        &mut RunCache::new(false),
        now(),
    )?;
    Ok(enc.tok.decode(&r.ids, false).unwrap())
}

fn js(enc: &Encoder, expr: &str) -> String {
    // A file per call: tests run in parallel.
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let name = format!("case{}", N.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
    run(
        enc,
        &name,
        &format!("export default function Main() {{ return String({expr}); }}\n"),
    )
    .unwrap_or_else(|e| format!("ERROR: {}", affordance::error_message(&e)))
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn javascript_expressions_evaluate_as_in_javascript() {
    let enc = enc();
    let cases: &[(&str, &str)] = &[
        // errors stringify as Error.prototype.toString does
        ("new Error(\"boom\")", "Error: boom"),
        ("new TypeError(\"bad\").toString()", "TypeError: bad"),
        ("new Error()", "Error"),
        ("{name: \"x\", message: \"y\", extra: 1}", "[object Object]"),
        // array callbacks get (value, index, array), as in JavaScript
        ("[1, 2, 3].filter((x, i, a) => a[i + 1] !== undefined).join()", "1,2"),
        ("[1, 2].map((x, i, a) => a.length).join()", "2,2"),
        ("[1, 2].some((x, i, a) => a[i - 1]?.toString() == \"1\")", "true"),
        ("[1, 2, 3].reduce((s, x, i, a) => s + a.length, 0)", "9"),
        ("[1, 2].find((x, i, a) => a.at(-1) == x)", "2"),
        // + and stringification
        (r#""a" + 1"#, "a1"),
        (r#"1 + "2""#, "12"),
        ("1 + 2", "3"),
        (r#""x" + true"#, "xtrue"),
        (r#""v" + null"#, "vnull"),
        (r#""u" + undefined"#, "uundefined"),
        (r#"[1, 2] + """#, "1,2"),
        (r#"[1, [2, 3]] + """#, "1,2,3"),
        ("0.1 + 0.2", "0.30000000000000004"),
        (
            "`${1.5}|${2.0}|${true}|${null}|${undefined}|${[1, [2, 3]]}`",
            "1.5|2|true|null|undefined|1,2,3",
        ),
        // truthiness and logic
        (r#"[] ? "t" : "f""#, "t"),
        (r#""" ? "t" : "f""#, "f"),
        (r#"0 || "d""#, "d"),
        (r#"[] && "y""#, "y"),
        ("!{}", "false"),
        ("!!\"0\"", "true"),
        (r#"null ?? "n""#, "n"),
        (r#"0 ?? "n""#, "0"),
        // equality
        (r#"1 == "1""#, "true"),
        (r#"1 === "1""#, "false"),
        ("null == undefined", "true"),
        ("null === undefined", "false"),
        ("[] === []", "false"),
        ("1 === 1.0", "true"),
        (r#"0 == """#, "true"),
        (r#""a" != "a""#, "false"),
        // comparisons
        ("undefined < 5", "false"),
        (r#""10" < "9""#, "true"),
        (r#""10" < 9"#, "false"),
        ("null >= 0", "true"),
        // arithmetic
        ("7 / 2", "3.5"),
        ("6 / 3", "2"),
        ("-7 % 3", "-1"),
        (r#""6" * "2""#, "12"),
        ("1 / 0", "Infinity"),
        (r#""a" - 1"#, "NaN"),
        // An integer remainder that overflows (`i64::MIN % -1`) is the float one, -0 (it panicked).
        (r#"1 / (JSON.parse("-9223372036854775808") % -1)"#, "-Infinity"),
        // indexing
        ("[1, 2, 3][-1]", "undefined"),
        (r#""abc"[1]"#, "b"),
        (r#"({a: 1})["a"]"#, "1"),
        ("[1, 2, 3].length", "3"),
        // typeof
        ("typeof 1", "number"),
        (r#"typeof "x""#, "string"),
        ("typeof undefined", "undefined"),
        ("typeof notDeclared", "undefined"),
        ("typeof [1]", "object"),
        ("typeof null", "object"),
        ("typeof (x => x)", "function"),
        // methods
        (r#""a-b-c".replace("-", "+")"#, "a+b-c"),
        (r#""a-b-c".replaceAll("-", "+")"#, "a+b+c"),
        (r#""abc".split("").join("|")"#, "a|b|c"),
        ("[3, 1, 10].sort()", "1,10,3"),
        ("[3, 1, 10].sort((a, b) => a - b)", "1,3,10"),
        ("[1, 2, 3].reduce((a, b) => a + b, 0)", "6"),
        ("[1, 2, 3].indexOf(2)", "1"),
        ("[1, 2, 3].find(x => x > 1)", "2"),
        ("[1, 2, 3].findIndex(x => x > 5)", "-1"),
        (r#""abc".padStart(5, "*")"#, "**abc"),
        ("(3.14159).toFixed(2)", "3.14"),
        (r#""x".repeat(3)"#, "xxx"),
        (r#""hello".indexOf("l")"#, "2"),
        (r#""hello".substring(1, 3)"#, "el"),
        ("[[1], [2, 3]].flat()", "1,2,3"),
        ("[1, 2].map(x => x * 2)", "2,4"),
        // globals
        ("Math.max(1, 5, 3)", "5"),
        ("Math.floor(2.7)", "2"),
        ("Math.round(2.5)", "3"),
        (r#"Object.keys({a: 1, b: 2}).join(",")"#, "a,b"),
        (r#"JSON.stringify({a: [1, "x"], b: null})"#, r#"{"a":[1,"x"],"b":null}"#),
        (r#"JSON.parse("[1,2]")[1]"#, "2"),
        (r#"parseInt("42px")"#, "42"),
        (r#"parseFloat("3.5kg")"#, "3.5"),
        (r#"Number("3.5")"#, "3.5"),
        (r#"Number("x")"#, "NaN"),
        ("String(12)", "12"),
        ("Array.isArray([])", "true"),
        (r#"Object.entries({a: 1}).length"#, "1"),
    ];
    let failures: Vec<String> = cases
        .iter()
        .filter_map(|(e, want)| {
            let got = js(&enc, e);
            (got != *want).then(|| format!("{e}: got {got:?}, want {want:?}"))
        })
        .collect();
    assert!(
        failures.is_empty(),
        "{} of {} cases differ:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn statements_in_a_js_file() {
    let enc = enc();
    let src = r#"function keysOf(o) { let s = ""; for (const k in o) s += k; return s; }
function count(n) { let i = 0; let s = 0; while (i < n) { i += 1; if (i % 2) continue; s += i; } return s; }
export default function Main() {
  const xs = [];
  xs.push(1, 2);
  const empty = [];
  const label = empty ? "arrays are truthy" : "arrays are falsy";
  return `${keysOf({a: 1, b: 2})} ${keysOf(["x", "y"])} ${count(6)} ${xs} ${label} ${xs.length > 1 && "many"}`;
}
"#;
    assert_eq!(run(&enc, "stmts", src).unwrap(), "ab 01 12 1,2 arrays are truthy many");
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn destructuring_spread_and_updates() {
    let enc = enc();
    let src = r#"function sum(...xs) { return 0; }
function add(a, b = 10) { return a + b; }
export default function Main() {
  const { a, b = 5, ...rest } = { a: 1, c: 3, d: 4 };
  const xs = [1, 2, 3];
  let n = 3;
  let down = "";
  while (n > 0) { down += n; n--; }
  for (let i = 0; i < 2; i++) down += "|";
  return `${a} ${b} ${Object.keys(rest).join("")} ${Math.max(...xs)} ${add(1)} ${add(1, undefined)} ${add(1, 2)} ${down}`;
}
"#;
    assert_eq!(run(&enc, "destructure", src).unwrap(), "1 5 cd 3 11 11 3 321||");
}

/// What JavaScript rejects, the views reject: no Python leniency.
#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn javascript_errors() {
    let enc = enc();
    let fails = |body: &str| {
        let e = run(&enc, "fails", &format!("export default function Main() {{ {body} }}\n")).expect_err(body);
        affordance::error_message(&e).to_string()
    };
    assert!(
        fails("const x = null; return x.a;").starts_with("TypeError: Cannot read properties of null (reading 'a')")
    );
    assert!(
        fails("const x = undefined; return x.a;")
            .starts_with("TypeError: Cannot read properties of undefined (reading 'a')")
    );
    assert!(
        fails("let s = \"\"; for (const k of {a: 1}) s += k; return s;")
            .starts_with("TypeError: object is not iterable")
    );
    assert!(
        fails("let s = \"\"; for (const k of undefined) s += k; return s;")
            .starts_with("TypeError: undefined is not iterable")
    );
    assert!(fails("return \"abc\".strip();").starts_with("TypeError: string.strip is not a function"));
    assert!(fails("return len([1]);").starts_with("ReferenceError: len is not defined"));
    assert!(fails("return [1].count(1);").starts_with("TypeError: array.count is not a function"));
    assert!(fails("throw new TypeError(\"bad\");").starts_with("TypeError: bad"));
    assert!(fails("throw new Error(\"bad\");").starts_with("raise: bad"));
    // A token fragment only joins with text: other values need an explicit conversion.
    assert!(fails("return p`a` + [1];").starts_with("TypeError: cannot add 1 to a token fragment"));
    // An element without a name is a load error (it panicked).
    for body in ["const s = < ></>; return s;", "return < />;"] {
        assert!(
            fails(body).contains("expected an element name after `<`"),
            "{body}: {}",
            fails(body)
        );
    }
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn try_catch_throw_and_optional_chaining() {
    let enc = enc();
    let src = r#"function risky(n) { if (n > 1) throw new_error("too big: " + n); return n; }
function new_error(m) { return {message: m, code: 7}; }
function attempt(n) {
  let log = "";
  try { log += risky(n); } catch (e) { log += `caught(${e.message},${e.code})`; } finally { log += "+done"; }
  return log;
}
export default function Main() {
  const r = {a: {b: 2}, f: null};
  let fallthrough = "";
  try { throw "plain"; } catch (e) { fallthrough = e; }
  return `${attempt(1)} ${attempt(5)} ${fallthrough} ${r?.a?.b} ${r.f?.x} ${r.missing?.y} ${r.a?.toString()}`;
}
"#;
    assert_eq!(
        run(&enc, "trycatch", src).unwrap(),
        "1+done caught(too big: 5,7)+done plain 2 undefined undefined [object Object]"
    );
}

/// `catch` never catches a pending effect: the handler still parks on it, and a failed effect
/// (an error outcome in the transcript) is caught on replay.
#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn catch_handles_failed_effects_but_not_pending_ones() {
    use affordance::effects::Outcome;
    let enc = enc();
    let dir = std::env::temp_dir().join(format!("aff-js-effects-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("app.tsx"),
        r#"export default function Main() {
  return <aff label="go" onGo={async () => {
    try {
      const r = await harness.send("x", {});
      return `sent ${r.ok}`;
    } catch (e) {
      return `send failed: ${e.message}`;
    }
  }}>{"[go]"}</aff>;
}
"#,
    )
    .unwrap();
    let v = View::load(&dir.join("app.tsx"), "Main", &enc).unwrap();
    let st = || std::rc::Rc::new(Default::default());
    let first = v
        .act_effects(
            &enc,
            st(),
            now(),
            "go",
            0,
            &serde_json::json!({}),
            &serde_json::json!({}),
            vec![],
        )
        .unwrap();
    assert!(!first.done, "parked on the pending call, not caught");
    let mut log = first.effects;
    log[0].outcome = Some(Outcome::Error("no agent named x".into()));
    let done = v
        .act_effects(
            &enc,
            st(),
            now(),
            "go",
            0,
            &serde_json::json!({}),
            &serde_json::json!({}),
            log,
        )
        .unwrap();
    assert_eq!(
        (done.done, done.value),
        (true, serde_json::json!("send failed: no agent named x"))
    );
}

/// `await` is an operator only in an async function's own body, as in JavaScript: anywhere else
/// it is a load error (it parsed, and the frame, which cannot suspend, dropped its task silently).
#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn await_outside_an_async_function_is_a_syntax_error() {
    let enc = enc();
    let sync = [
        "function f() { await g(); }",
        "async function f() { [1].map(x => await x); }",
        "async function f() { return function () { return await g(); }; }",
        "async function f() { return [1].map((x) => { await x; }); }",
        "const o = {m() { return await g(); }};",
        "class C { m() { return await g(); } }",
        "class C { x = await g(); }",
        "const x = await g();",
    ];
    for (i, decl) in sync.iter().enumerate() {
        let e = run(
            &enc,
            &format!("await_sync{i}"),
            &format!(
                "async function g() {{ return 1; }}\n{decl}\nexport default function Main() {{ return \"ok\"; }}\n"
            ),
        )
        .unwrap_err();
        assert!(
            e.contains(":2: `await` is only valid in an async function"),
            "{decl}: {e}"
        );
    }
    let async_ = [
        "async function f() { return await g(); }",
        "const f = async () => await g();",
        "const f = async x => { return await g() + x; };",
        "const o = {async m() { return await g(); }};",
        "class C { async m() { return await g(); } }",
        "function f() { return async () => await g(); }",
        "async function f() { const h = async () => await g(); return await h(); }",
    ];
    for (i, decl) in async_.iter().enumerate() {
        let out = run(
            &enc,
            &format!("await_async{i}"),
            &format!(
                "async function g() {{ return 1; }}\n{decl}\nexport default function Main() {{ return \"ok\"; }}\n"
            ),
        );
        assert_eq!(out.as_deref(), Ok("ok"), "{decl}");
    }
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn a_runaway_view_runs_out_of_fuel_instead_of_hanging() {
    let enc = enc();
    let started = std::time::Instant::now();
    let e = run(
        &enc,
        "runaway",
        "export default function Main() { let n = 0; while (true) { n = n + 1; } return \"never\"; }\n",
    )
    .unwrap_err();
    assert!(e.contains("out of fuel"), "{e}");
    let nested = run(&enc, "nested", "export default function Main() { let n = 0; for (let i = 0; i < 100000; i++) { for (let j = 0; j < 100000; j++) { n = n + 1; } } return String(n); }\n").unwrap_err();
    assert!(
        nested.contains("out of fuel"),
        "nested loops share one budget: {nested}"
    );
    eprintln!("two runaway renders stopped in {:?}", started.elapsed());
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn compound_assignment_operators() {
    let enc = enc();
    let out = run(&enc, "compound", "export default function Main() { let x = 6; x *= 7; x -= 2; x /= 4; let y = 5; for (let i = 0; i < 3; i *= 1) { y += 1; i += 1; } return `${x} ${y} ${3 * 4}`; }\n").unwrap();
    assert_eq!(out, "10 8 12");
}

/// JSX spread attributes: `{...obj}` passes every entry; a later attribute of the same name wins,
/// as in React; null spreads nothing; a non-object is a TypeError.
#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn jsx_spread_attributes_pass_an_objects_entries() {
    let enc = enc();
    let src = |main: &str| {
        format!(
            "function C({{ a = \"-\", b = \"-\", c = \"-\" }}) {{ return a + b + c; }}\nexport default function Main() {{ const o = {{a: \"x\", b: \"y\"}}; return {main}; }}\n"
        )
    };
    assert_eq!(run(&enc, "spread1", &src("<C {...o}/>")).unwrap(), "xy-");
    assert_eq!(run(&enc, "spread2", &src("<C {...o} b=\"z\"/>")).unwrap(), "xz-");
    assert_eq!(
        run(&enc, "spread3", &src("<C b=\"z\" {...o} c=\"w\"/>")).unwrap(),
        "xyw"
    );
    assert_eq!(run(&enc, "spread4", &src("<C {...null} a=\"q\"/>")).unwrap(), "q--");
    assert!(
        run(&enc, "spread5", &src("<C {...3}/>"))
            .unwrap_err()
            .contains("needs an object")
    );
}

/// `x = [...x, more]` and `x = [...x.slice(0, -k), more]` grow a list in place when nothing else
/// holds it, with JavaScript's results: an alias keeps the old array, the new items see the old
/// value, and a non-list is spread as written.
#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn growing_a_list_by_spreading_it_keeps_javascript_semantics() {
    let enc = enc();
    let run1 =
        |name: &str, body: &str| run(&enc, name, &format!("export default function Main() {{ {body} }}\n")).unwrap();
    assert_eq!(
        run1(
            "grow1",
            "let out = []; for (const i of [1, 2, 3]) { out = [...out, i]; } return out.join();"
        ),
        "1,2,3"
    );
    assert_eq!(
        run1(
            "grow2",
            "let out = [1, 2]; const alias = out; out = [...out, 3]; return alias.join() + '|' + out.join();"
        ),
        "1,2|1,2,3"
    );
    assert_eq!(
        run1(
            "grow3",
            "let out = [1, 2, 3]; out = [...out.slice(0, -1), out.length, ...out]; return out.join();"
        ),
        "1,2,3,1,2,3"
    );
    assert_eq!(
        run1(
            "grow4",
            "let out = [1]; out = [...out.slice(0, -2), 9]; return out.join();"
        ),
        "9"
    );
    assert_eq!(
        run1("grow5", "let s = 'ab'; s = [...s, 'c']; return s.join('-');"),
        "a-b-c"
    );
    assert_eq!(
        run1(
            "grow6",
            "let out = [1, 2]; out = [...out, out.length, ...out]; return out.join();"
        ),
        "1,2,2,1,2"
    );
    assert_eq!(
        run1(
            "grow7",
            "let out = 'xy'; out = [...out.slice(0, -1), 'z']; return out.join();"
        ),
        "x,z"
    );
}

/// A model turn's `ids` is a token run (one laid-out fragment, no value per id) that reads like
/// a list of ids; `tokens()` passes it through.
#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn a_token_run_reads_like_a_list_of_ids() {
    let enc = enc();
    let cases: &[(&str, &str)] = &[
        ("tokens([5, 6, 7]).length", "3"),
        ("tokens([5, 6, 7]).at(-1)", "7"),
        ("tokens([5, 6, 7]).at(3)", "undefined"),
        ("tokens([5, 6, 7])[1]", "6"),
        ("tokens([5, 6, 7]).slice(0, -1).length", "2"),
        ("tokens([5, 6, 7]).slice(1).at(0)", "6"),
        ("tokens([5, 6, 7]).includes(6)", "true"),
        ("[...tokens([5, 6])].join()", "5,6"),
        ("tokens([5, 6])?.length > 0", "true"),
        ("tokens(tokens([5, 6]).slice(0, -1)).length", "1"),
    ];
    for (expr, want) in cases {
        assert_eq!(js(&enc, expr), *want, "{expr}");
    }
}

/// Integer-like keys keep JavaScript's order (ascending, before the other keys) without a scan
/// per insert: building an object of 4n index keys, in a loop or by `JSON.parse`, costs about 4
/// times n's (a scan per insert made it 16 times).
#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn integer_keys_insert_in_linear_time() {
    let enc = enc();
    let time = |name: &str, n: usize| {
        let src = format!(
            "export default function Main() {{ const o = {{}}; o.first = 0; for (let i = 0; i < {n}; i++) o[i] = i; \
            const p = JSON.parse('{{' + Object.keys(o).map(k => `\"${{k}}\":1`).join(',') + '}}'); \
            return Object.keys(p).length + ':' + Object.keys(o).at(-1); }}\n"
        );
        // The fastest of three, so a scheduling hiccup does not decide it.
        (0..3)
            .map(|_| {
                let start = std::time::Instant::now();
                assert_eq!(run(&enc, name, &src).unwrap(), format!("{}:first", n + 1));
                start.elapsed()
            })
            .min()
            .unwrap()
    };
    let (small, large) = (time("keys_n", 20_000), time("keys_4n", 80_000));
    eprintln!("integer keys: 20k {small:?}, 80k {large:?}");
    assert!(large < small * 8, "20k integer keys took {small:?}, 80k took {large:?}");
}
