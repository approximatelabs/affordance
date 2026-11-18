//! Tools from functions (views/lib/tool.tsx): a function's name, its JSDoc summary and its props
//! parameter's type are a tool's name, description and argument schema (`nameOf`, `docOf`,
//! `schemaOf`, src/schema.rs). The schema's bytes are prompt bytes, so the mapping is pinned here.
use affordance::View;
use affordance::emit::{Encoder, RunCache};
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn now() -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 10, 19)
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
    let dir = std::env::temp_dir().join(format!("aff-tools-{}", std::process::id()));
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
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn a_functions_type_is_its_tools_schema() {
    let enc = enc();
    let cases: &[(&str, &str)] = &[
        // primitives, optional, declaration order
        (
            r#"function f({ q, n, ok }: { q: string; n?: number; ok: boolean }) {}"#,
            r#"{"type":"object","properties":{"q":{"type":"string"},"n":{"type":"number"},"ok":{"type":"boolean"}},"required":["q","ok"]}"#,
        ),
        // arrays, string-literal enums, nested objects, nullable (still required)
        (
            r#"function f({ tags, unit, at, note }: { tags: string[]; unit: "c" | "f"; at: { x: number; y?: number }; note: string | null }) {}"#,
            r#"{"type":"object","properties":{"tags":{"type":"array","items":{"type":"string"}},"unit":{"type":"string","enum":["c","f"]},"at":{"type":"object","properties":{"x":{"type":"number"},"y":{"type":"number"}},"required":["x"]},"note":{"type":["string","null"]}},"required":["tags","unit","at","note"]}"#,
        ),
        // aliases (type and interface), Array<T>
        (
            r#"type Unit = "c" | "f";
interface Point { x: number; y: number }
function f({ unit, pts }: { unit?: Unit; pts: Array<Point> }) {}"#,
            r#"{"type":"object","properties":{"unit":{"type":"string","enum":["c","f"]},"pts":{"type":"array","items":{"type":"object","properties":{"x":{"type":"number"},"y":{"type":"number"}},"required":["x","y"]}}},"required":["pts"]}"#,
        ),
        // no arguments
        (r#"function f({}: {}) {}"#, r#"{"type":"object","properties":{}}"#),
    ];
    let mut failed = vec![];
    for (decl, want) in cases {
        let src = format!("{decl}\nexport default function Main() {{ return JSON.stringify(schemaOf(f)); }}\n");
        match run(&enc, &src) {
            Ok(got) if got == *want => {}
            other => failed.push(format!("{decl}\n  expected {want}\n  got      {other:?}")),
        }
    }
    assert!(failed.is_empty(), "{}", failed.join("\n\n"));
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn names_and_docs_come_from_the_declaration() {
    let src = r#"
/**
 * Look something up.
 * Searches the index.
 * @param q what to look up
 */
function lookup({ q }: { q: string }) { return q; }

export default function Main() {
  /** Add a note. */
  const add_note = ({ text }: { text: string }) => text;
  function inner({ x }: { x: number }) { return x; }
  const anon = [({ y }: { y: string }) => y][0];
  return [nameOf(lookup), docOf(lookup), nameOf(add_note), docOf(add_note), nameOf(inner), String(docOf(inner)), String(nameOf(anon)), JSON.stringify(schemaOf(anon))].join("|");
}"#;
    assert_eq!(
        run(&enc(), src).unwrap(),
        r#"lookup|Look something up. Searches the index.|add_note|Add a note.|inner|undefined|undefined|{"type":"object","properties":{"y":{"type":"string"}},"required":["y"]}"#
    );
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn a_type_with_no_schema_is_an_error_naming_it() {
    let enc = enc();
    let e = run(
        &enc,
        "function f({ q }) {}\nexport default function Main() { return JSON.stringify(schemaOf(f)); }",
    )
    .unwrap_err();
    assert!(e.contains("schemaOf(f): its parameter has no type"), "{e}");
    let e = run(&enc, "function f({ m }: { m: Map<string, number> }) {}\nexport default function Main() { return JSON.stringify(schemaOf(f)); }").unwrap_err();
    assert!(e.contains("schemaOf(f): Map<...> has no schema"), "{e}");
    let e = run(&enc, "interface A { x: number }\ninterface B extends A { y: number }\nfunction f({ b }: { b: B }) {}\nexport default function Main() { return JSON.stringify(schemaOf(f)); }").unwrap_err();
    assert!(e.contains("B has no schema (an interface that extends another)"), "{e}");
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn log_text_handles_model_calls_and_multimodal_results_without_losing_text() {
    let src = format!(
        r#"import {{recordText}} from '{}/views/lib/log.tsx';
export default function Main() {{
  const records=[{{kind:'input',content:'hello'}},
    {{kind:'model',reasoning:'plan',content:'answer',calls:[{{function:{{name:'lookup',arguments:{{q:'x'}}}}}}]}},
    {{kind:'action',result:[{{type:'text',text:'first'}},{{type:'image_url',image_url:{{url:'data:image/png;base64,private'}}}},{{type:'text',text:'last'}}]}},
    {{kind:'action',result:{{ok:true}}}},{{kind:'metric'}}];
  let error='';try{{recordText({{kind:'input',released:true}});}}catch(e){{error=e.message;}}
  return JSON.stringify({{text:records.map(recordText),error}});
}}"#,
        root().display()
    );
    let result: serde_json::Value = serde_json::from_str(&run(&enc(), &src).unwrap()).unwrap();
    assert_eq!(
        result["text"],
        serde_json::json!([
            "hello",
            "[reasoning] plan\nanswer\n[call] lookup {\"q\":\"x\"}",
            "first\n[image]\nlast",
            "{\"ok\":true}",
            ""
        ])
    );
    assert!(result["error"].as_str().unwrap().contains("released record"));
}
