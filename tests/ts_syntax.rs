//! TypeScript syntax is parsed and erased (spec/LANGUAGE.md §2): annotations of every shape,
//! `type` / `interface` declarations, generics on functions, arrows and calls. Each case is a
//! module whose `Main` returns a string.
use affordance::View;
use affordance::emit::{Encoder, RunCache};
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn now() -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 10, 18)
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

fn run(enc: &Encoder, src: &str) -> Result<String, String> {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!("aff-ts-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("c{}.tsx", N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
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

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs the test tokenizers: tools/fetch-test-tokenizers.sh")]
fn type_syntax_is_parsed_and_erased() {
    let enc = enc();
    let cases: &[(&str, &str)] = &[
        // inline object types on a destructured parameter (a tool handler's usual shape)
        (
            r#"function f({ q, n = 2 }: { q: string; n?: number }) { return q + n; }
            export default function Main() { return f({q: "a"}); }"#,
            "a2",
        ),
        // object, union, array, tuple, function and literal types on parameters and returns
        (
            r#"function g(a: { x: number, y?: string[] }, b: "p" | "q" = "p", c: [number, string?] = [1], h?: (x: number) => void): { out: string } { return {out: b + a.x}; }
            export default function Main() { return g({x: 3}).out; }"#,
            "p3",
        ),
        // declarations, exported or not, including interfaces without semicolons
        (
            r#"interface A { x: number }
            const Y = "y";
            export interface B<T> extends A, Readonly<T> { items: T[]; f(a: number): string; [k: string]: unknown }
            type U<T = string> = { kind: "a"; v: T } | { kind: "b" } | null;
            export type K = keyof typeof Y;
            export default function Main() { return Y; }"#,
            "y",
        ),
        // generics: declarations, call sites, arrows (`<T,>` in TSX)
        (
            r#"function id<T extends object = {}>(x: T): T { return x; }
            const pair = <T,>(a: T, b: T): T[] => [a, b];
            export default function Main() { const xs = pair<string>("m", "n"); return id<string>(xs.join("")); }"#,
            "mn",
        ),
        // a ternary with a parenthesized branch is not a typed arrow
        (
            r#"export default function Main() { const t = true; const Y = "k"; return t ? (Y) : "no"; }"#,
            "k",
        ),
        // comparisons still compare
        (
            r#"export default function Main() { const a = 1, b = 2, c = 3; return String(a < b && b > a && (c < 4)); }"#,
            "true",
        ),
        // casts and non-null assertions
        (
            r#"export default function Main() { const o = {a: "z"} as { a: string }; const xs = [1, 2] as const; const v = o!.a satisfies string; return (v as string) + xs.length + String(!false) + String(1 !== 2); }"#,
            "z2truetrue",
        ),
        // conditional, indexed, template-literal and nested generic types
        (
            r#"type C<T> = T extends string ? "s" : "o";
            type I = { a: { b: number } }["a"];
            type L = `id-${string}`;
            function m(x: Map<string, Array<Array<number>>> | undefined): string { return "ok"; }
            export default function Main() { return m(undefined); }"#,
            "ok",
        ),
    ];
    let mut failed = vec![];
    for (src, want) in cases {
        match run(&enc, src) {
            Ok(got) if got == *want => {}
            other => failed.push(format!("{src}\n  expected {want:?}, got {other:?}")),
        }
    }
    assert!(failed.is_empty(), "{}", failed.join("\n\n"));
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs the test tokenizers: tools/fetch-test-tokenizers.sh")]
fn a_package_import_is_an_error_not_silently_dropped() {
    let e = run(
        &enc(),
        "import { useChat } from \"affordance\";\nexport default function Main() { return \"x\"; }",
    )
    .unwrap_err();
    assert!(e.contains("cannot import \"affordance\""), "{e}");
}
