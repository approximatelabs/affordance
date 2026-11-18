//! Differential testing of the view language against Node: each case is a JavaScript function
//! body; it runs as a view (`Main` returns a printout of the body's result) and under Node
//! (`new Function` in strict mode), and the two printouts must be equal.
//!
//! - `tests/js_corpus/*.js`: the checked-in corpus. A case starts at a `// case: <name>` line.
//!   A program Node rejects (a SyntaxError) must fail to load as a view. Two markers:
//!   - `// expect: unsupported`: a construct the language does not support by design; the view
//!     must fail with an error (thrown or at load), never print a value Node does not print.
//!   - `// expect: divergent <reason>`: a documented difference (`spec/LANGUAGE.md`, "Differences
//!     from JavaScript"); the case must still differ (if it starts to match, drop the marker).
//! - `generated_programs_match_node`: cases from the seeded generator `tools/jsgen.mjs`
//!   (`JS_FUZZ_SEED`, `JS_FUZZ_COUNT` choose the run; the default run is part of the suite).
//!
//! Both tests skip (and say so) when `node` is not installed.
use crate::tokenizer;
use affordance::View;
use affordance::emit::{Encoder, RunCache};
use std::path::{Path, PathBuf};
use std::process::Command;

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
    tokenizer::encoder()
}

/// The printout of a value, written in the language itself so it runs the same in both engines:
/// `-0` is distinguished, strings are quoted, arrays and objects are printed in key order.
const PRELUDE: &str = r#"function __show(v) {
  if (v === undefined) return "undefined";
  if (v === null) return "null";
  const t = typeof v;
  if (t === "number") { if (v === 0 && 1 / v < 0) return "-0"; return String(v); }
  if (t === "string") return "\"" + v + "\"";
  if (t === "boolean") return String(v);
  if (t === "function") return "function";
  if (Array.isArray(v)) return "[" + v.map(x => __show(x)).join(",") + "]";
  if (t === "object") {
    let s = "{";
    let first = true;
    for (const k of Object.keys(v)) { s += (first ? "" : ",") + k + ":" + __show(v[k]); first = false; }
    return s + "}";
  }
  return t + ":" + String(v);
}
function __errname(e) {
  if (e !== null && typeof e === "object" && typeof e.name === "string") return e.name;
  return "value " + __show(e);
}
"#;

#[derive(Debug, Clone, Default)]
struct Case {
    name: String,
    body: String,
    unsupported: bool,
    /// A documented difference from JavaScript (the reason).
    divergent: Option<String>,
    /// The only CPU architecture whose Node is this case's oracle (`// oracle: x86_64 <why>`):
    /// V8 itself rounds a few floating-point results differently by platform.
    oracle: Option<String>,
}

fn parse_corpus(file: &str, text: &str) -> Vec<Case> {
    let mut cases: Vec<Case> = vec![];
    for line in text.lines() {
        let t = line.trim();
        if let Some(name) = t.strip_prefix("// case:") {
            cases.push(Case {
                name: format!("{file}:{}", name.trim()),
                ..Default::default()
            });
        } else if t == "// expect: unsupported" && !cases.is_empty() {
            cases.last_mut().unwrap().unsupported = true;
        } else if let (Some(reason), false) = (t.strip_prefix("// expect: divergent"), cases.is_empty()) {
            cases.last_mut().unwrap().divergent = Some(reason.trim().to_string());
        } else if let (Some(rest), false) = (t.strip_prefix("// oracle:"), cases.is_empty()) {
            cases.last_mut().unwrap().oracle = rest.split_whitespace().next().map(str::to_string);
        } else if let Some(c) = cases.last_mut() {
            c.body.push_str(line);
            c.body.push('\n');
        }
    }
    // A case whose oracle is another architecture's Node is not compared here.
    cases.retain(|c| c.oracle.as_deref().is_none_or(|arch| arch == std::env::consts::ARCH));
    cases
}

fn corpus() -> Vec<Case> {
    let dir = root().join("tests/js_corpus");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "js"))
        .collect();
    files.sort();
    files
        .iter()
        .flat_map(|p| {
            parse_corpus(
                &p.file_name().unwrap().to_string_lossy(),
                &std::fs::read_to_string(p).unwrap(),
            )
        })
        .collect()
}

fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("aff-jsdiff-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Node's printout of every case, in order.
fn node_outputs(cases: &[Case], dir: &Path) -> Vec<String> {
    let bodies: Vec<&str> = cases.iter().map(|c| c.body.as_str()).collect();
    let input = dir.join("bodies.json");
    std::fs::write(&input, serde_json::to_string(&bodies).unwrap()).unwrap();
    let script = format!(
        r#""use strict";
{PRELUDE}
const bodies = JSON.parse(require("fs").readFileSync(process.argv[2], "utf8"));
const out = bodies.map(b => {{
  let f;
  try {{ f = new Function('"use strict";\n' + b); }} catch (e) {{ return "SYNTAX " + e.message; }}
  try {{ return "OK " + __show(f()); }} catch (e) {{ return "THROW " + __errname(e); }}
}});
process.stdout.write(JSON.stringify(out));
"#
    );
    let js = dir.join("run.js");
    std::fs::write(&js, script).unwrap();
    let o = Command::new("node")
        .arg("--stack-size=4000")
        .arg(&js)
        .arg(&input)
        .output()
        .expect("run node");
    assert!(
        o.status.success(),
        "node failed: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    serde_json::from_slice(&o.stdout).unwrap()
}

/// The view's printout of one case: `OK ..`/`THROW ..` as in Node, `ERROR ..` when the view
/// does not load or fails outside the case's own `try`, `PANIC ..` if the interpreter panics.
fn ours(enc: &Encoder, dir: &Path, i: usize, case: &Case) -> String {
    let src = format!(
        "{PRELUDE}\nfunction __case() {{\n{}\n}}\nexport default function Main() {{\n  try {{ return \"OK \" + __show(__case()); }} catch (e) {{ return \"THROW \" + __errname(e); }}\n}}\n",
        case.body
    );
    let path = dir.join(format!("case{i}.tsx"));
    std::fs::write(&path, src).unwrap();
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        View::load(&path, "Main", enc).and_then(|v| {
            v.render(
                enc,
                std::rc::Rc::new(Default::default()),
                &mut RunCache::new(false),
                now(),
            )
        })
    }));
    match r {
        Ok(Ok(r)) => enc.tok.decode(&r.ids, false).unwrap(),
        Ok(Err(e)) => format!("ERROR {}", affordance::error_message(&e).lines().next().unwrap_or("")),
        Err(p) => format!(
            "PANIC {}",
            p.downcast_ref::<String>()
                .cloned()
                .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default()
        ),
    }
}

struct Report {
    /// Cases that differ (unexpectedly), described.
    bad: Vec<String>,
    /// Documented differences that still differ.
    known: usize,
}

/// Run the cases in both engines.
fn compare(cases: &[Case], tag: &str) -> Report {
    let dir = scratch(tag);
    let want = node_outputs(cases, &dir);
    let enc = enc();
    let mut report = Report { bad: vec![], known: 0 };
    for (i, (c, want)) in cases.iter().zip(&want).enumerate() {
        let got = ours(&enc, &dir, i, c);
        let same = if want.starts_with("SYNTAX ") {
            // A program JavaScript rejects must not load either.
            got.starts_with("ERROR ")
        } else if c.unsupported {
            got.starts_with("ERROR ") || got.starts_with("THROW ")
        } else {
            &got == want
        };
        let note = match (&c.divergent, same) {
            (Some(_), false) => {
                report.known += 1;
                continue;
            }
            (Some(reason), true) => format!(" (marked divergent \"{reason}\" but now matches Node: drop the marker)"),
            (None, true) => continue,
            (None, false) if c.unsupported => " (marked unsupported: must fail, not print a value)".to_string(),
            (None, false) => String::new(),
        };
        report.bad.push(format!(
            "--- {}{note}\n{}  node: {want}\n  ours: {got}",
            c.name,
            c.body
                .trim_end()
                .lines()
                .map(|l| format!("  | {l}\n"))
                .collect::<String>()
        ));
    }
    let _ = std::fs::remove_dir_all(&dir);
    report
}

#[test]
fn corpus_matches_node() {
    if !node_available() {
        eprintln!(
            "SKIPPED corpus_matches_node: `node` is not installed (the differential corpus needs Node as its oracle)"
        );
        return;
    }
    let cases = corpus();
    assert!(cases.len() > 100, "corpus has {} cases", cases.len());
    let r = compare(&cases, "corpus");
    eprintln!(
        "js corpus: {} cases, {} documented differences, {} unexpected",
        cases.len(),
        r.known,
        r.bad.len()
    );
    assert!(
        r.bad.is_empty(),
        "{} of {} corpus cases differ from Node:\n{}",
        r.bad.len(),
        cases.len(),
        r.bad.join("\n")
    );
}

#[test]
fn generated_programs_match_node() {
    if !node_available() {
        eprintln!("SKIPPED generated_programs_match_node: `node` is not installed");
        return;
    }
    let seed: u64 = std::env::var("JS_FUZZ_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    let count: usize = std::env::var("JS_FUZZ_COUNT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(400);
    let o = Command::new("node")
        .arg(root().join("tools/jsgen.mjs"))
        .arg(seed.to_string())
        .arg(count.to_string())
        .output()
        .expect("run node");
    assert!(
        o.status.success(),
        "generator failed: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    let generated: Vec<serde_json::Value> = serde_json::from_slice(&o.stdout).unwrap();
    let cases: Vec<Case> = generated
        .iter()
        .map(|c| Case {
            name: c["name"].as_str().unwrap().to_string(),
            body: c["body"].as_str().unwrap().to_string(),
            ..Default::default()
        })
        .collect();
    let r = compare(&cases, &format!("gen{seed}"));
    eprintln!(
        "js generator (seed {seed}): {} cases, {} differ",
        cases.len(),
        r.bad.len()
    );
    assert!(
        r.bad.is_empty(),
        "{} of {} generated cases (seed {seed}) differ from Node:\n{}",
        r.bad.len(),
        cases.len(),
        r.bad.join("\n")
    );
}
