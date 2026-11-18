//! The language surface is JavaScript, and removing syntax fails the tree.
//! - every view in the tree (and in the out-of-tree consumers listed in a local tools/view_consumers.txt) parses and is free of Python/Jinja idioms; so are the docs' code examples;
//! - a Python idiom gets an error that names its JavaScript replacement, at parse time or at run
//!   time;
//! - the migration table in spec/LANGUAGE.md covers every name the code knows.
use affordance::View;
use affordance::emit::{Encoder, RunCache};
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn now() -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 10, 14)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap()
}

#[test]
fn views_docs_and_consumers_have_no_python_idioms() {
    let paths = affordance::lint::default_paths(&root());
    let found = affordance::lint::lint_paths(&paths);
    let list: Vec<String> = found.iter().map(|f| f.to_string()).collect();
    assert!(
        found.is_empty(),
        "{} finding(s) (run `afr lint`):\n{}",
        found.len(),
        list.join("\n")
    );
    let consumers = affordance::lint::consumers(&root());
    eprintln!(
        "linted {} paths, {} of them out-of-tree consumers: {:?}",
        paths.len(),
        consumers.len(),
        consumers
    );
}

fn write(name: &str, src: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("aff-lint-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join(name);
    std::fs::write(&p, src).unwrap();
    p
}

#[test]
fn lint_flags_each_kind_of_idiom() {
    let p = write(
        "idioms.tsx",
        r#"function first(xs) { return xs[0]; }
export default function Main({ xs, s, o }) {
  const a = len(xs) + str(s);
  const b = s.strip().startswith("x") ? True : None;
  const c = xs + [1];
  const d = first(xs);
  return `${a} ${b} ${c} ${d} ${o.get("k")} ${xs.count(1)}`;
}
"#,
    );
    let found: Vec<String> = affordance::lint::lint_file(&p)
        .iter()
        .map(|f| f.message.clone())
        .collect();
    for want in [
        "`len(..)`",
        "`str(..)`",
        "`.strip(..)`",
        "`.startswith(..)`",
        "`True`",
        "`None`",
        "[...a, ...b]",
    ] {
        assert!(found.iter().any(|f| f.contains(want)), "missing {want} in {found:#?}");
    }
    // A view's own `first` is not the Jinja builtin; `.get` / `.count` may be an object's own
    // members, so they are left to the run-time error.
    assert!(
        !found
            .iter()
            .any(|f| f.contains("`first(") || f.contains(".get") || f.contains(".count")),
        "{found:#?}"
    );
    assert_eq!(found.len(), 7, "{found:#?}");
}

#[test]
fn python_syntax_is_a_parse_error_that_names_the_migration() {
    let cases = [
        ("const t = s[0:160];", "use x.slice(a, b)"),
        ("const t = xs[-5:];", "use x.slice(a, b)"),
        ("const t = s[:2];", "use x.slice(a, b)"),
        (
            "const t = f(r, reasoning=\"\");",
            "keyword arguments `f(k=v)` are not supported",
        ),
        ("const t = k not in o;", "use !(k in obj)"),
        ("const t = a and b;", "use && / || / !"),
    ];
    for (stmt, want) in cases {
        let p = write(
            "syntax.tsx",
            &format!(
                "function f() {{ return 1; }}\nexport default function Main({{ s, xs, r, k, o, a, b }}) {{\n  {stmt}\n  return t;\n}}\n"
            ),
        );
        let found = affordance::lint::lint_file(&p);
        assert!(
            found.len() == 1 && found[0].message.contains(want) && found[0].line == 3,
            "{stmt}: {found:?}"
        );
    }
    // Not keyword arguments: comparisons and arrow functions in argument position.
    let p = write(
        "ok.tsx",
        "export default function Main({ xs, a }) { return String(xs.map(x => x == a).filter((y = 1) => y)); }\n",
    );
    assert!(affordance::lint::lint_file(&p).is_empty());
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn python_builtins_and_methods_fail_with_the_javascript_to_write() {
    let enc = Encoder::from_file(
        root()
            .join("ref/tokenizers/Qwen__Qwen3-4B-Instruct-2507.json")
            .to_str()
            .unwrap(),
    )
    .unwrap();
    let run = |body: &str| {
        let p = write("run.tsx", &format!("export default function Main() {{ {body} }}\n"));
        let v = View::load(&p, "Main", &enc).unwrap();
        affordance::error_message(
            &v.render(
                &enc,
                std::rc::Rc::new(Default::default()),
                &mut RunCache::new(false),
                now(),
            )
            .err()
            .expect("an error"),
        )
        .to_string()
    };
    let e = run("let s = \"\"; for (const [i, x] of enumerate([1])) s += x; return s;");
    assert!(
        e.starts_with("ReferenceError: enumerate is not defined") && e.contains("xs.entries()"),
        "{e}"
    );
    let e = run("return \" a \".strip();");
    assert!(
        e.starts_with("TypeError: string.strip is not a function") && e.contains(".trim()"),
        "{e}"
    );
    let e = run("return \"a\\nb\".splitlines().length;");
    assert!(e.contains(".split(\"\\n\")"), "{e}");
    let e = run("return String(True);");
    assert!(
        e.starts_with("ReferenceError: True is not defined") && e.contains("use true"),
        "{e}"
    );
}
