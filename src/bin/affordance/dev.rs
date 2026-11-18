//! `affordance render`, `test` and `lint`, and the development commands that need a source
//! checkout and its reference data:
//!
//!   affordance check  [--set dev] [--only SUBSTR] [--tier hand|converted] [--threads N]
//!                                                      compare every model x case to the reference
//!   affordance render-case --model M --case C [--set dev]  print ids and decoded text
//!   affordance bench  ...                                  render speed against the references

use affordance::emit::{Encoder, RunCache};
use affordance::{View, state_from_json_exact};
use serde_json::{Value as J, json};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub fn root() -> PathBuf {
    std::env::var_os("AFFORDANCE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}

pub fn now() -> chrono::NaiveDateTime {
    // Same pinned clock as ref/render_ref.py.
    chrono::NaiveDate::from_ymd_opt(2026, 10, 5)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap()
}

struct Entry {
    model: String,
    view: PathBuf,
    entry: String,
    /// The view was converted from the model's official template (tools/jinja2tsx.py).
    converted: bool,
}

fn index() -> Vec<Entry> {
    let p = root().join("views/index.json");
    let j: J = serde_json::from_str(&std::fs::read_to_string(&p).expect("views/index.json")).unwrap();
    let mut v = vec![];
    for (model, e) in j.as_object().unwrap() {
        if model.starts_with('_') {
            continue;
        }
        v.push(Entry {
            model: model.clone(),
            view: root().join("views").join(e["view"].as_str().unwrap()),
            entry: e["entry"].as_str().unwrap().to_string(),
            // (A model registered with another model's view is in the converted tier too: the
            // hand tier is the models the hand-written views were written for.)
            converted: e["converted"] == true || e["shared"].is_string(),
        });
    }
    v
}

fn arg(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1).cloned())
}

/// A case file. Its integers past 64 bits keep their digits (a few conformance cases have
/// one, as Python does): see `parse_json_exact`.
fn load_cases(p: &Path) -> J {
    let text = std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{}: {}", p.display(), e));
    affordance::value::parse_json_exact(&text).unwrap_or_else(|e| panic!("{}: {}", p.display(), e))
}

fn load_json(p: &Path) -> J {
    serde_json::from_str(&std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{}: {}", p.display(), e))).unwrap()
}

/// Error kind: an explicit raise (Jinja raise_exception, encoder asserts/validation) vs a type /
/// undefined error from evaluating an expression.
fn ref_error_kind(e: &str) -> &'static str {
    let head = e.split(':').next().unwrap_or("");
    match head {
        // Incidental failures (a lookup, a parse), as against an explicit raise.
        "TypeError" | "UndefinedError" | "KeyError" | "AttributeError" | "IndexError" | "JSONDecodeError" => "type",
        // The sandbox refusing a call that changes a list or dict, and a filter or test the
        // template names that Jinja does not have: failures of the template, not its own raise.
        "SecurityError" => "type",
        // A template that recurses past Python's limit (ours stops at the interpreter's).
        "RecursionError" => "type",
        "TemplateRuntimeError" | "TemplateAssertionError"
            if e.contains(": No filter named") || e.contains(": No test named") =>
        {
            "type"
        }
        _ => "raise",
    }
}

fn our_error_kind(e: &str) -> &'static str {
    if e.starts_with("raise:") { "raise" } else { "type" }
}

#[derive(Default, Clone, Copy)]
struct Tally {
    kind_agree: usize,
    raises: usize,
    msg_agree: usize,
    /// Of the raises, the templates' own (`raise_exception`: a TemplateError) and how many agree.
    template_raises: usize,
    template_agree: usize,
    ok: usize,
    ok_err: usize,
    mismatch: usize,
    missing_err: usize,
    extra_err: usize,
}

fn first_diff(a: &[u32], b: &[u32]) -> usize {
    a.iter().zip(b.iter()).take_while(|(x, y)| x == y).count()
}

/// `args` as the process received them, with the command name second (as `afr` took them).
pub fn main(args: Vec<String>) {
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("help");
    let set = arg(&args, "--set").unwrap_or_else(|| "dev".into());
    let only = arg(&args, "--only");
    let verbose = args.iter().any(|a| a == "-v");
    match cmd {
        "app" => {
            if let Err(error) = render_app(&args) {
                eprintln!("ERROR {error}");
                std::process::exit(1);
            }
        }
        // affordance lint [PATH..]: parse errors and non-JavaScript idioms in views. Default: in a
        // source checkout, its views, fixtures and docs (lint::default_paths); elsewhere, here.
        "lint" => {
            let paths: Vec<PathBuf> = args[2..]
                .iter()
                .filter(|a| !a.starts_with("--"))
                .map(PathBuf::from)
                .collect();
            let paths = if !paths.is_empty() {
                paths
            } else if Path::new("views/index.json").is_file() {
                affordance::lint::default_paths(Path::new("."))
            } else {
                vec![PathBuf::from(".")]
            };
            let found = affordance::lint::lint_paths(&paths);
            for f in &found {
                println!("{f}");
            }
            eprintln!("{} finding(s)", found.len());
            if !found.is_empty() {
                std::process::exit(1);
            }
        }
        "check" => {
            let references = arg(&args, "--ref-dir")
                .map(PathBuf::from)
                .unwrap_or_else(|| root().join("ref/out"));
            // `--set a,b,c`: the sets one after another in this process.
            let mut ok = true;
            for one in set.split(',') {
                if set.contains(',') {
                    println!("== {one}");
                }
                ok &= check(
                    one,
                    only.as_deref(),
                    verbose,
                    args.iter().any(|a| a == "--raw"),
                    args.iter().any(|a| a == "--pieces"),
                    args.iter().any(|a| a == "--msg"),
                    &references,
                );
            }
            if !ok {
                std::process::exit(1);
            }
        }
        "render" => {
            let model = arg(&args, "--model").expect("--model");
            let case = arg(&args, "--case").expect("--case");
            let e = index()
                .into_iter()
                .find(|e| e.model == model)
                .expect("model not in views/index.json");
            let enc =
                Encoder::from_file(root().join(format!("ref/tokenizers/{}.json", model)).to_str().unwrap()).unwrap();
            let view = View::load(&e.view, &e.entry, &enc).unwrap_or_else(|er| panic!("{}", er));
            let cases = load_cases(&root().join(format!("ref/cases_{}.json", set)));
            let c = cases
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["id"] == case.as_str())
                .expect("case");
            let sp = load_json(&root().join(format!("ref/tokenizers/{}.special.json", model)));
            let mut cache = RunCache::new(false);
            match view.render(&enc, state_from_json_exact(c, &sp), &mut cache, now()) {
                Ok(r) => {
                    println!("{:?}", r.ids);
                    println!("{}", enc.tok.decode(&r.ids, false).unwrap());
                    for s in &r.spans {
                        println!("{}", span_line(s));
                    }
                }
                Err(er) => println!("ERROR {}", er),
            }
            let refj = load_json(&root().join(format!("ref/out/{}/{}.json", set, model)));
            println!("--- reference:");
            let r = &refj[&case];
            if let Some(t) = r.get("text") {
                println!("{:?}", r["ids"]);
                println!("{}", t.as_str().unwrap());
            } else {
                println!("ERROR {}", r["error"]);
            }
        }
        // affordance test APP.tsx SCENARIO.json [--entry Main] [--tokenizer FILE|NAME] [-v]: run an
        // app against a scripted model and check what it did (src/scenario.rs). A NAME is a cached
        // tokenizer of a source checkout (ref/tokenizers/NAME.json).
        "test" => {
            // The two files are the arguments that are not an option or an option's value.
            let mut files: Vec<&String> = vec![];
            let mut rest = args[2..].iter();
            while let Some(a) = rest.next() {
                if a == "--tokenizer" || a == "--entry" {
                    rest.next();
                } else if !a.starts_with('-') {
                    files.push(a);
                }
            }
            let [app, scenario] = files[..] else {
                eprintln!("usage: affordance test APP.tsx SCENARIO.json [--entry Main] [--tokenizer FILE] [-v]");
                std::process::exit(2)
            };
            let sc = load_json(Path::new(scenario));
            // The tokenizer the scenario or the command names; with neither, the bundled default,
            // as `render` uses.
            let named = arg(&args, "--tokenizer").or_else(|| sc["tokenizer"].as_str().map(str::to_owned));
            let tok = named.clone().unwrap_or_else(|| "the bundled default".into());
            let file = match &named {
                Some(tok) if Path::new(tok).is_file() => PathBuf::from(tok),
                Some(tok) => root().join(format!("ref/tokenizers/{tok}.json")),
                None => affordance::library::default_tokenizer().unwrap_or_else(|e| {
                    eprintln!("ERROR {e}");
                    std::process::exit(2)
                }),
            };
            let enc = Encoder::from_file(file.to_str().unwrap()).unwrap_or_else(|e| {
                eprintln!("ERROR tokenizer {tok}: {e}");
                std::process::exit(2)
            });
            let entry = arg(&args, "--entry")
                .or_else(|| sc["entry"].as_str().map(str::to_owned))
                .unwrap_or_else(|| "Main".into());
            match affordance::scenario::run(Path::new(app), &entry, &enc, &sc) {
                Err(e) => {
                    eprintln!("ERROR {}", affordance::error_message(&e));
                    std::process::exit(1)
                }
                Ok(r) => {
                    if verbose || !r.ok {
                        for l in &r.log {
                            eprintln!("  {l}");
                        }
                    }
                    for d in &r.dumps {
                        println!("DUMP\n{d}");
                    }
                    match r.failure {
                        None => println!("PASS {} ({} records)", app, r.records.len()),
                        Some(f) => {
                            println!(
                                "FAIL {app}
{f}"
                            );
                            std::process::exit(1)
                        }
                    }
                }
            }
        }
        "bench" => bench(
            &set,
            only.as_deref(),
            arg(&args, "--threads")
                .map(|t| t.split(',').map(|x| x.parse().unwrap()).collect())
                .unwrap_or(vec![1]),
            arg(&args, "--output")
                .map(PathBuf::from)
                .unwrap_or_else(|| root().join("target/bench/results_rust.json")),
        ),
        _ => {
            eprintln!("unknown command: {cmd} (affordance --help)");
            std::process::exit(2);
        }
    }
}

/// Render app props directly, without routing them through the chat-case schema.
fn render_app(args: &[String]) -> Result<(), String> {
    use affordance::value::Value;
    use std::io::Read;
    let required = |name: &str| arg(args, name).ok_or_else(|| format!("missing {name}"));
    let tokenizer = match (arg(args, "--tokenizer"), arg(args, "--model")) {
        (Some(path), None) => PathBuf::from(path),
        (None, Some(model)) => root().join(format!("ref/tokenizers/{model}.json")),
        (Some(_), Some(_)) => return Err("use either --tokenizer or --model".into()),
        // No tokenizer named: the bundled default (GPT-2), enough for an app with no model format.
        (None, None) => affordance::library::default_tokenizer()?,
    };
    let path = PathBuf::from(required("--view")?);
    let entry = arg(args, "--entry").unwrap_or_else(|| "Main".into());
    let source = required("--state")?;
    let mut input = String::new();
    if source == "-" {
        std::io::stdin().read_to_string(&mut input).map_err(|e| e.to_string())?;
    } else {
        input = std::fs::read_to_string(&source).map_err(|e| format!("{source}: {e}"))?;
    }
    let state: J = affordance::value::parse_json_exact(&input).map_err(|e| format!("invalid app state: {e}"))?;
    let batch = args.iter().any(|a| a == "--batch");
    let states = if batch {
        state
            .as_array()
            .ok_or("--batch requires an array of app states")?
            .clone()
    } else {
        vec![state]
    };
    if states.is_empty() {
        return Err("app state batch is empty".into());
    }
    let enc = Encoder::from_file(tokenizer.to_str().ok_or("tokenizer path is not UTF-8")?)?;
    let library = match arg(args, "--library") {
        Some(dir) => PathBuf::from(dir),
        None => affordance::library::for_app(&path)?,
    };
    let templates: Vec<String> = arg(args, "--template").into_iter().collect();
    let view = View::load_for(&path, &entry, &enc, &[library], &templates)?;
    let mut cache = RunCache::new(args.iter().any(|a| a == "--cache"));
    // `--template NAME`: render in that model's format, as a run does when its binding names the
    // template. The app is the same file for every model.
    let template = match arg(args, "--template") {
        Some(name) => {
            let options = match arg(args, "--template-options") {
                Some(text) => serde_json::from_str(&text).map_err(|e| format!("--template-options: {e}"))?,
                None => serde_json::json!({}),
            };
            Some((name, options))
        }
        None if arg(args, "--template-options").is_some() => return Err("--template-options needs --template".into()),
        None => None,
    };
    let view = match &template {
        Some((name, options)) => view.for_model(name, options),
        None => view,
    };
    let machine = args.iter().any(|a| a == "--json");
    let mut outputs = vec![];
    let mut failed = false;
    for state in states {
        let Value::Map(state) = Value::from_json_exact(&state) else {
            return Err("app state must be a JSON object".into());
        };
        let state = std::rc::Rc::new(state.into_data());
        let rendered = view.render(&enc, state, &mut cache, now());
        // The app's console, as a browser shows a page's: beside the output, not in it.
        for (level, text) in affordance::console::take() {
            eprintln!("console.{level}: {text}");
        }
        match rendered {
            Ok(r) => {
                if !machine {
                    println!("ids     {:?}", r.ids);
                    println!("text    {}", enc.tok.decode(&r.ids, false).map_err(|e| e.to_string())?);
                    for s in &r.spans {
                        println!("{}", span_line(s));
                    }
                }
                outputs.push(json!({"ids":r.ids,"spans":r.spans}));
            }
            Err(error) => {
                failed = true;
                if !machine {
                    println!("ERROR {error}");
                }
                outputs.push(json!({"error":error}));
            }
        }
    }
    if machine {
        let output = if batch { J::Array(outputs) } else { outputs.remove(0) };
        println!("{}", serde_json::to_string(&output).map_err(|e| e.to_string())?);
    }
    if failed {
        Err("app rendering failed".into())
    } else {
        Ok(())
    }
}

/// A tokenizer, loaded once per process and shared by the threads and sets that use it
/// (loading one costs far more than rendering a set's cases with it).
fn shared_encoder(path: &str) -> std::sync::Arc<Encoder> {
    static LOADED: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, std::sync::Arc<Encoder>>>> =
        std::sync::OnceLock::new();
    let loaded = LOADED.get_or_init(Default::default);
    if let Some(enc) = loaded.lock().unwrap().get(path) {
        return enc.clone();
    }
    let enc = std::sync::Arc::new(Encoder::from_file(path).unwrap());
    loaded.lock().unwrap().entry(path.to_string()).or_insert(enc).clone()
}

fn check(
    set: &str,
    only: Option<&str>,
    verbose: bool,
    also_raw: bool,
    pieces: bool,
    msg_log: bool,
    references: &Path,
) -> bool {
    let cases = load_cases(&root().join(format!("ref/cases_{}.json", set)));
    let cases = cases.as_array().unwrap();
    let mut table: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    let mut total = Tally::default();
    let mut invalid = 0usize;
    // `--only`: a part that is a model's whole name selects that model alone; any other part
    // selects every model whose name contains it.
    // `--tier hand` or `--tier converted`: the hand-written views alone, or the converted ones.
    let tier = std::env::args().skip_while(|a| a != "--tier").nth(1);
    let all: Vec<Entry> = index()
        .into_iter()
        .filter(|e| match tier.as_deref() {
            Some("hand") => !e.converted,
            Some("converted") => e.converted,
            _ => true,
        })
        .collect();
    let names: std::collections::BTreeSet<String> = all.iter().map(|e| e.model.clone()).collect();
    let entries = all
        .into_iter()
        .filter(|e| {
            only.is_none_or(|o| {
                o.split(',').any(|part| {
                    if names.contains(part) {
                        e.model == part
                    } else {
                        e.model.contains(part)
                    }
                })
            })
        })
        .collect::<Vec<_>>();
    let mut references_ready = vec![];
    let mut case_ids = std::collections::BTreeSet::new();
    for case in cases {
        let Some(id) = case["id"].as_str() else {
            eprintln!("INVALID CASE: missing string id");
            return false;
        };
        if !case_ids.insert(id) {
            eprintln!("INVALID CASE: duplicate id {id}");
            return false;
        }
    }
    if entries.is_empty() || cases.is_empty() {
        eprintln!("conformance selection contains no models or cases");
        return false;
    }
    // Validate the declared inventory before rendering. A partially downloaded oracle is
    // not evidence for the models that happened to remain on disk.
    for e in entries {
        let path = references.join(set).join(format!("{}.json", e.model));
        let reference = std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|text| serde_json::from_str::<J>(&text).map_err(|e| e.to_string()));
        let reference = match reference {
            Ok(value) => value,
            Err(error) => {
                eprintln!("MISSING OR INVALID REFERENCE {}: {error}", path.display());
                invalid += 1;
                continue;
            }
        };
        let valid_ids = |ids: &J| {
            ids.as_array()
                .is_some_and(|xs| xs.iter().all(|x| x.as_u64().is_some_and(|n| n <= u32::MAX as u64)))
        };
        for id in &case_ids {
            let row = &reference[*id];
            let valid = match (row.get("ids"), row.get("error")) {
                (Some(ids), None) => valid_ids(ids),
                (None, Some(error)) => error.is_string(),
                _ => false,
            };
            if !valid || row.get("ids_raw").is_some_and(|ids| !valid_ids(ids)) {
                eprintln!(
                    "INVALID REFERENCE {} / {id}: expected token IDs or an error string",
                    e.model
                );
                invalid += 1;
            }
        }
        references_ready.push((e, reference));
    }
    if invalid != 0 {
        return false;
    }
    println!(
        "Reference coverage: {} selected models, {} cases per model",
        references_ready.len(),
        cases.len()
    );
    // Each model is independent (its own tokenizer, view and cases): they run on every core, and
    // their lines are printed in the index's order.
    macro_rules! outln {
        ($out:expr, $($arg:tt)*) => {{
            use std::fmt::Write as _;
            let _ = writeln!($out, $($arg)*);
        }};
    }
    type Done = (String, Option<(Tally, BTreeMap<String, String>, usize)>);
    let work = |e: &Entry, refj: &J| -> Done {
        let mut out = String::new();
        let mut bad = 0usize;
        let tokp = root().join(format!("ref/tokenizers/{}.json", e.model));
        // One process may check several sets (`--set a,b,c`): each tokenizer is loaded once.
        let enc = shared_encoder(tokp.to_str().unwrap());
        let enc: &Encoder = &enc;
        let sp = load_json(&root().join(format!("ref/tokenizers/{}.special.json", e.model)));
        let view = match View::load(&e.view, &e.entry, enc) {
            Ok(v) => v,
            Err(er) => {
                outln!(out, "{:50} LOAD ERROR {}", e.model, er);
                return (out, None);
            }
        };
        // Second tokenizer: the repo's own tokenizer.json, when it differs from HF's.
        let raw_enc = if also_raw {
            let p = root().join(format!("ref/models/{}/tokenizer.json", e.model));
            if cases
                .iter()
                .any(|c| refj[c["id"].as_str().unwrap()].get("ids_raw").is_some())
                && p.exists()
            {
                Some(Encoder::from_file(p.to_str().unwrap()).unwrap())
            } else {
                None
            }
        } else {
            None
        };
        let mut t = Tally::default();
        let mut row = BTreeMap::new();
        let mut cache = RunCache::new(false);
        for c in cases {
            let id = c["id"].as_str().unwrap();
            let r = &refj[id];
            if r.get("ids").is_none() && r.get("error").is_none() {
                eprintln!("INVALID REFERENCE {} / {}: no ids or error", e.model, id);
                bad += 1;
                continue;
            }
            let state = state_from_json_exact(c, &sp);
            // Compare the message only; the view stack (`\n    at ...`) is diagnostic.
            let got = view
                .render_chat_opts(enc, state.clone(), &mut cache, now(), pieces)
                .map_err(|e| affordance::error_message(&e).to_string());
            let verdict = match (r.get("ids"), &got) {
                (Some(want), Ok(g)) => {
                    let want: Vec<u32> = want
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|x| x.as_u64().unwrap() as u32)
                        .collect();
                    if want == g.ids {
                        t.ok += 1;
                        "match".to_string()
                    } else {
                        t.mismatch += 1;
                        let d = first_diff(&want, &g.ids);
                        if verbose {
                            let lo = d.saturating_sub(8);
                            outln!(
                                out,
                                "  MISMATCH {} / {} at token {}:\n    want ...{:?}\n    got  ...{:?}",
                                e.model,
                                id,
                                d,
                                enc.tok.decode(&want[lo..(d + 8).min(want.len())], false).unwrap(),
                                enc.tok.decode(&g.ids[lo..(d + 8).min(g.ids.len())], false).unwrap()
                            );
                        }
                        format!("MISMATCH@{}", d)
                    }
                }
                (None, Err(er)) => {
                    t.ok_err += 1;
                    let (rk, ok) = (ref_error_kind(r["error"].as_str().unwrap_or("")), our_error_kind(er));
                    if rk == "raise" && ok == "raise" {
                        t.raises += 1;
                        let refmsg = r["error"]
                            .as_str()
                            .unwrap_or("")
                            .split_once(": ")
                            .map(|x| x.1)
                            .unwrap_or("");
                        let ours = er.trim_start_matches("raise: ");
                        let agree = !refmsg.is_empty()
                            && (refmsg == ours || refmsg.starts_with(ours) || ours.starts_with(refmsg));
                        let template = r["error"].as_str().unwrap_or("").starts_with("TemplateError");
                        t.template_raises += template as usize;
                        t.template_agree += (template && agree) as usize;
                        if agree {
                            t.msg_agree += 1;
                        } else if msg_log {
                            outln!(
                                out,
                                "  MSG {} / {}\n    ref  {}\n    ours {}",
                                e.model,
                                id,
                                refmsg.chars().take(220).collect::<String>(),
                                ours.chars().take(220).collect::<String>()
                            );
                        }
                    }
                    if rk == ok {
                        t.kind_agree += 1;
                    } else if verbose {
                        outln!(
                            out,
                            "  ERROR-KIND {} / {}: reference {} ({}), view {} ({})",
                            e.model,
                            id,
                            rk,
                            r["error"].as_str().unwrap_or("").chars().take(90).collect::<String>(),
                            ok,
                            er.chars().take(90).collect::<String>()
                        );
                    }
                    "both-error".to_string()
                }
                (Some(_), Err(er)) => {
                    t.extra_err += 1;
                    if verbose {
                        outln!(out, "  ERROR {} / {}: {}", e.model, id, er);
                    }
                    format!("ERROR {}", er.chars().take(80).collect::<String>())
                }
                (None, Ok(_)) => {
                    t.missing_err += 1;
                    if verbose {
                        outln!(out, "  NO-ERROR {} / {}: reference raised {}", e.model, id, r["error"]);
                    }
                    "missing-error".to_string()
                }
            };
            row.insert(id.to_string(), verdict.clone());
            if let (Some(re), true) = (&raw_enc, verdict == "match") {
                let want = r.get("ids_raw").unwrap_or(&r["ids"]);
                let want: Vec<u32> = want
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|x| x.as_u64().unwrap() as u32)
                    .collect();
                let mut c2 = RunCache::new(false);
                let rv = View::load(&e.view, &e.entry, re).unwrap();
                let g = rv.render(re, state, &mut c2, now()).map(|r| r.ids).unwrap_or_default();
                row.insert(
                    format!("{}#raw", id),
                    if g == want {
                        "match".into()
                    } else {
                        format!("MISMATCH@{}", first_diff(&want, &g))
                    },
                );
            }
        }
        let n = cases.len();
        outln!(
            out,
            "{:50} {:3}/{:3} exact ({} match, {} both-error, {} same kind)  mismatch {}  unexpected-error {}  missing-error {}",
            e.model,
            t.ok + t.ok_err,
            n,
            t.ok,
            t.ok_err,
            t.kind_agree,
            t.mismatch,
            t.extra_err,
            t.missing_err
        );
        (out, Some((t, row, bad)))
    };
    let next = std::sync::atomic::AtomicUsize::new(0);
    let done: Vec<std::sync::Mutex<Option<Done>>> = references_ready.iter().map(|_| Default::default()).collect();
    // `--threads N`; by default at most 32 at once, since the test suite runs several checks
    // side by side, beside tests that measure time.
    let asked = std::env::args()
        .skip_while(|a| a != "--threads")
        .nth(1)
        .and_then(|n| n.parse::<usize>().ok());
    let threads = asked
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, |n| n.get()).min(32))
        .clamp(1, references_ready.len().max(1));
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some((e, refj)) = references_ready.get(i) else {
                        break;
                    };
                    *done[i].lock().unwrap() = Some(work(e, refj));
                }
            });
        }
    });
    for ((e, _), slot) in references_ready.iter().zip(done) {
        let (out, result) = slot.into_inner().unwrap().expect("every model was run");
        print!("{out}");
        let Some((t, row, bad)) = result else {
            invalid += 1;
            continue;
        };
        invalid += bad;
        total.ok += t.ok;
        total.ok_err += t.ok_err;
        total.kind_agree += t.kind_agree;
        total.raises += t.raises;
        total.msg_agree += t.msg_agree;
        total.template_raises += t.template_raises;
        total.template_agree += t.template_agree;
        total.mismatch += t.mismatch;
        total.extra_err += t.extra_err;
        total.missing_err += t.missing_err;
        table.insert(e.model.clone(), row);
    }
    println!(
        "TOTAL exact {} ({} match + {} both-error [{} same error kind]), mismatch {}, unexpected-error {}, missing-error {}",
        total.ok + total.ok_err,
        total.ok,
        total.ok_err,
        total.kind_agree,
        total.mismatch,
        total.extra_err,
        total.missing_err
    );
    eprintln!(
        "explicit raises: {} of {} have the same message text",
        total.msg_agree, total.raises
    );
    let suffix = only
        .map(|o| {
            if o.contains(',') {
                "_selected".into()
            } else {
                format!("_{}", o.replace('/', "_"))
            }
        })
        .unwrap_or_default();
    let outp = root().join(format!(
        "target/conformance/check_{}{}{}.json",
        set,
        if pieces { "_pieces" } else { "" },
        suffix
    ));
    std::fs::create_dir_all(outp.parent().unwrap()).unwrap();
    std::fs::write(&outp, serde_json::to_string_pretty(&json!(table)).unwrap()).unwrap();
    // Exact means the same ids, or the same error: of the same kind, and a template's own raise
    // (`raise_exception`) with the same message. A library's error text (pydantic's) is not ours.
    invalid == 0
        && total.ok + total.ok_err > 0
        && total.mismatch + total.extra_err + total.missing_err == 0
        && total.kind_agree == total.ok_err
        && total.template_agree == total.template_raises
}

/// The reference prompt text for a case (from ref/out), falling back to the decoded ids.
fn load_text_cache(key: &str, model: &str, fallback: &str) -> String {
    thread_local! {
        static CACHE: std::cell::RefCell<std::collections::HashMap<String, J>> = Default::default();
    }
    let (set, id) = key.split_once('/').unwrap();
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        let k = format!("{}/{}", set, model);
        let j = c
            .entry(k)
            .or_insert_with(|| load_json(&root().join(format!("ref/out/{}/{}.json", set, model))));
        j[id]
            .get("text")
            .and_then(|t| t.as_str())
            .map(String::from)
            .unwrap_or_else(|| fallback.to_string())
    })
}

fn median(mut v: Vec<u128>) -> u128 {
    v.sort();
    v[v.len() / 2]
}

/// Timing of the view renderer on the cases whose reference has ids (same filter as bench_hf.py).
///  cold: no run cache; incr: cache warmed by rendering the conversation without its last message
///  (the agent loop: re-render after one new message). Also multi-thread throughput (cold).
fn bench(sets: &str, only: Option<&str>, threads: Vec<usize>, output: PathBuf) {
    use std::time::Instant;
    const REPS: usize = 5;
    let mut out = serde_json::Map::new();
    for e in index() {
        if let Some(o) = only
            && !o.split(',').any(|x| e.model == x || e.model.contains(x))
        {
            continue;
        }
        let tokp = root().join(format!("ref/tokenizers/{}.json", e.model));
        let enc = Encoder::from_file(tokp.to_str().unwrap()).unwrap();
        let sp = load_json(&root().join(format!("ref/tokenizers/{}.special.json", e.model)));
        let view = View::load(&e.view, &e.entry, &enc).unwrap();
        let mut work: Vec<(String, serde_json::Value, Vec<u32>)> = vec![];
        for set in sets.split(',') {
            let cases = load_cases(&root().join(format!("ref/cases_{}.json", set)));
            let refj = load_json(&root().join(format!("ref/out/{}/{}.json", set, e.model)));
            for c in cases.as_array().unwrap() {
                let id = c["id"].as_str().unwrap();
                if let Some(ids) = refj[id].get("ids") {
                    let ids: Vec<u32> = ids
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|x| x.as_u64().unwrap() as u32)
                        .collect();
                    work.push((format!("{}/{}", set, id), c.clone(), ids));
                }
            }
        }
        let mut per = serde_json::Map::new();
        let (mut sum_cold, mut sum_incr, mut ok, mut sum_tok) = (0u128, 0u128, 0usize, 0u128);
        for (key, c, want) in &work {
            let state = state_from_json_exact(c, &sp);
            let mut cold = vec![];
            let mut got = vec![];
            for _ in 0..REPS {
                let mut cache = RunCache::new(false);
                let t = Instant::now();
                let r = view.render(&enc, state.clone(), &mut cache, now()).unwrap();
                cold.push(t.elapsed().as_nanos());
                got = r.ids;
            }
            if &got == want {
                ok += 1;
            }
            // previous turn: same conversation without its last message
            let mut prev = c.clone();
            let msgs = prev["messages"].as_array_mut().unwrap();
            if msgs.len() > 1 {
                msgs.pop();
            }
            let prev_state = state_from_json_exact(&prev, &sp);
            let mut incr = vec![];
            for _ in 0..REPS {
                let mut cache = RunCache::new(true);
                let _ = view.render(&enc, prev_state.clone(), &mut cache, now());
                let t = Instant::now();
                let r = view.render(&enc, state.clone(), &mut cache, now()).unwrap();
                incr.push(t.elapsed().as_nanos());
                assert_eq!(&r.ids, want, "cached render differs on {}", key);
            }
            // floor: tokenizers crate encoding the finished reference string in one call
            let text = enc.tok.decode(want, false).unwrap_or_default();
            let ref_text = load_text_cache(key, &e.model, &text);
            let mut tokf = vec![];
            for _ in 0..REPS {
                let t = Instant::now();
                let _ = enc.tok.encode_fast(ref_text.as_str(), false).unwrap();
                tokf.push(t.elapsed().as_nanos());
            }
            let (mc, mi) = (median(cold), median(incr));
            sum_tok += median(tokf);
            sum_cold += mc;
            sum_incr += mi;
            per.insert(
                key.clone(),
                json!({"tokens": want.len(), "cold_ns": mc as u64, "incr_ns": mi as u64}),
            );
        }
        let n = work.len().max(1) as f64;
        let mut tp = serde_json::Map::new();
        for &t in &threads {
            // throughput: t threads, each its own encoder + view, all cases, cold cache
            let barrier = std::sync::Arc::new(std::sync::Barrier::new(t + 1));
            let mut hs = vec![];
            for _ in 0..t {
                let b = barrier.clone();
                let (view_path, entry, tokp, sp, work) = (
                    e.view.clone(),
                    e.entry.clone(),
                    tokp.clone(),
                    sp.clone(),
                    work.iter().map(|w| w.1.clone()).collect::<Vec<_>>(),
                );
                hs.push(std::thread::spawn(move || {
                    let enc = Encoder::from_file(tokp.to_str().unwrap()).unwrap();
                    let view = View::load(&view_path, &entry, &enc).unwrap();
                    let states: Vec<_> = work.iter().map(|c| state_from_json_exact(c, &sp)).collect();
                    b.wait();
                    let mut renders = 0usize;
                    for _ in 0..3 {
                        for s in &states {
                            let mut cache = RunCache::new(false);
                            let _ = view.render(&enc, s.clone(), &mut cache, now());
                            renders += 1;
                        }
                    }
                    renders
                }));
            }
            barrier.wait();
            let t0 = Instant::now();
            let total: usize = hs.into_iter().map(|h| h.join().unwrap()).sum();
            let secs = t0.elapsed().as_secs_f64();
            tp.insert(t.to_string(), json!(total as f64 / secs));
        }
        println!(
            "{:45} {:4} cases ({} exact)  cold {:7.1} us/render  incr {:7.1} us/render  tokenize-only {:7.1} us  throughput {:?}",
            e.model,
            work.len(),
            ok,
            sum_cold as f64 / n / 1000.0,
            sum_incr as f64 / n / 1000.0,
            sum_tok as f64 / n / 1000.0,
            tp.iter()
                .map(|(k, v)| format!("{}t: {:.0}/s", k, v.as_f64().unwrap()))
                .collect::<Vec<_>>()
        );
        out.insert(e.model.clone(), json!({"cases": per, "throughput": tp}));
    }
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(output, serde_json::to_string(&serde_json::Value::Object(out)).unwrap()).unwrap();
}

/// A span as one line: its token range, kind and label, and what it does. An `<aff>` element has
/// one span, and each of its `on*` handlers another (`action`, with the handler's parameters).
fn span_line(s: &affordance::emit::Span) -> String {
    let (kind, detail) = match s.kind.as_str() {
        "affordance" if s.meta.get("handler").is_some() => {
            let params: Vec<&str> = s.meta["params"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|p| p.as_str())
                .collect();
            ("action", format!("{}({})", s.label, params.join(", ")))
        }
        "box" => {
            let m = &s.meta;
            let hidden = m["hidden_tokens"].as_u64().unwrap_or(0);
            (
                "box",
                format!(
                    "{}{}",
                    s.label,
                    if hidden > 0 {
                        format!(" ({hidden} tokens hidden)")
                    } else {
                        String::new()
                    }
                ),
            )
        }
        kind => (kind, s.label.clone()),
    };
    format!("span    {:>5}..{:<5} {kind:<10} {detail}", s.start, s.end)
}
