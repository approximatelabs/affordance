//! `affordance`: run apps (TSX) as agents, and render, test and lint them.
//!
//!   affordance run [PATH] [-f FILE]... [--set KEY=VALUE]... [--dir DIR] [--app FILE] [--restore | --fresh] [--keep-running]
//!   affordance config [PATH] [-f FILE]... [--set KEY=VALUE]...
//!   affordance init [DIR]
//!   affordance render APP.tsx --state FILE|- [--tokenizer FILE] [--entry NAME] [--json]
//!   affordance test APP.tsx SCENARIO.json
//!   affordance lint [PATH]...
//!   affordance check | render-case | bench   (development, in a source checkout)
mod cache_watch;
mod control;
mod deploy;
mod dev;
mod flags;
mod format;
mod host;
mod init;
mod run;
#[cfg(feature = "http")]
mod serve;

use std::path::{Path, PathBuf};

const USAGE: &str = "\
affordance: run apps (TSX) as agents against model servers and machines.

Usage:
  affordance run [PATH]       run the deployment: PATH, or affordance.yaml here or in a parent
      -f FILE                 another file over it (repeatable, in order)
      --set KEY=VALUE         one value over the files (repeatable; KEY dotted, VALUE YAML)
      --dir DIR               the journal directory (default .affordance beside the file)
      --app FILE              the app (overrides root.app or app.view)
      --restore                continue the run in the journal directory
      --fresh                 start over (the previous run's instances are kept under retired/)
      --keep-running          stay up while agents are idle (a chat waiting for its user)
      --control PATH          a control socket: post, status, close (also on serve and host)
      --flag NAME=VALUE       one of this machine's flags, for this run (repeatable; below)
  affordance serve [PATH]     run it behind HTTP (same options as run; --listen HOST:PORT): each
                              request is the event `request` to the root; the app answers on
                              the machine `client` (docs/RUNTIME.md)
  affordance host [PATH]      run it for a frontend over stdio (same options as run): JSON lines
                              in (event, answer, close, status) and out (deltas, output,
                              trace, questions on the machine `ui`) (docs/RUNTIME.md)
  affordance config [PATH]    print the resolved deployment (same -f and --set), secrets hidden
  affordance init [DIR]       write a first agent: app.tsx, affordance.yaml and .gitignore
  affordance render APP.tsx --state FILE|- [--tokenizer FILE] [--template NAME] [--entry NAME] [--json]
                              render an app to token ids and spans (default tokenizer: GPT-2);
                              --template: in that model's format (qwen36, llama31, messages, ..)
  affordance test APP.tsx SCENARIO.json [--entry NAME] [-v]
                              run an app against a scripted model and check what it did
  affordance lint [PATH]...   check TSX for errors and non-JavaScript idioms

Flags change what the command reports, never what a run does. They are read from
~/.config/affordance/flags (one `name = value` per line) and --flag:
  cache-warn = auto           warn when a prompt throws away most of the server's prefix cache:
                              on, off, or auto (on when stderr is a terminal)
  cache-warn-below = 0.2      ...when less than this share of the previous prompt starts the new one
  cache-warn-min-tokens = 1024  ...and the previous prompt had at least this many tokens

Development (in a source checkout): check, render-case, bench.
Deployment format: docs/RUNTIME.md. A deployment's local paths are relative to its file.
";

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut command = args.get(1).map(String::as_str).unwrap_or("--help");
    if args.iter().skip(2).any(|a| a == "--help" || a == "-h") {
        command = "--help";
    }
    // Commands that print to stdout end quietly when the reader goes away (`... | head`), as other
    // Unix tools do. Not the commands that run agents (`run`, `serve`, `host`, `test`): they write
    // to sockets and to the pipes of the commands their machines run, where a closed peer is an
    // error to handle, not a reason for the whole process to exit.
    #[cfg(unix)]
    if !matches!(command, "run" | "serve" | "host" | "test" | "init") {
        default_sigpipe();
    }
    let result = match command {
        "--help" | "-h" | "help" => {
            print!("{USAGE}");
            Ok(())
        }
        "--version" | "-V" => {
            println!("affordance {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        "run" => cmd_run(&args[2..]),
        "serve" => cmd_serve(&args[2..]),
        "host" => cmd_host(&args[2..]),
        "config" => cmd_config(&args[2..]),
        "init" => init::init(args.get(2).map(Path::new).unwrap_or(Path::new("."))),
        // `render` is `afr app`; the rest are the development commands, unchanged.
        "render" => {
            let mut a = vec![args[0].clone(), "app".into()];
            // The app is the first argument; `afr app` took it as --view.
            if args.get(2).is_some_and(|x| !x.starts_with('-')) {
                a.push("--view".into());
            }
            a.extend(args[2..].iter().cloned());
            dev::main(a);
            Ok(())
        }
        "test" | "lint" | "check" | "bench" => {
            dev::main(args.clone());
            Ok(())
        }
        "render-case" => {
            let mut a = vec![args[0].clone(), "render".into()];
            a.extend(args[2..].iter().cloned());
            dev::main(a);
            Ok(())
        }
        other => Err(format!("unknown command {other:?} (affordance --help)")),
    };
    if let Err(e) = result {
        eprintln!("affordance: {e}");
        std::process::exit(1);
    }
}

#[cfg(unix)]
fn default_sigpipe() {
    unsafe extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    // SAFETY: restores the default disposition of one signal before any thread starts.
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

/// The arguments `run` and `config` share: the deployment and its layers.
struct Selection {
    path: Option<PathBuf>,
    overrides: deploy::Overrides,
    flags: Vec<String>,
    listen: Option<String>,
    control: Option<PathBuf>,
    /// `--flag name=value` overrides of this machine's flags (`flags.rs`).
    machine_flags: Vec<String>,
}

fn select(args: &[String], allowed_flags: &[&str]) -> Result<Selection, String> {
    let mut s = Selection {
        path: None,
        overrides: deploy::Overrides::default(),
        flags: vec![],
        listen: None,
        control: None,
        machine_flags: vec![],
    };
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut value = |name: &str| it.next().cloned().ok_or_else(|| format!("{name} needs a value"));
        match a.as_str() {
            "-f" | "--file" => s.overrides.files.push(value(a)?.into()),
            "--set" => s.overrides.sets.push(value(a)?),
            "--dir" => s.overrides.dir = Some(value(a)?.into()),
            "--app" => s.overrides.app = Some(value(a)?.into()),
            "--flag" => s.machine_flags.push(value(a)?),
            "--listen" if allowed_flags.contains(&"--listen") => s.listen = Some(value(a)?),
            "--control" if allowed_flags.contains(&"--control") => s.control = Some(value(a)?.into()),
            f if allowed_flags.contains(&f) => s.flags.push(f.to_string()),
            f if f.starts_with('-') => return Err(format!("unknown option {f} (affordance --help)")),
            p if s.path.is_none() => s.path = Some(p.into()),
            p => return Err(format!("unexpected argument {p:?}")),
        }
    }
    Ok(s)
}

fn cmd_run(args: &[String]) -> Result<(), String> {
    let (s, d) = prepare_run(args, &["--restore", "--fresh", "--keep-running", "--control"])?;
    let has = |f: &str| s.flags.iter().any(|x| x == f);
    let warn = flags::Flags::load(&s.machine_flags)?.cache_warn()?;
    run::with_profile(|| {
        run::run(
            d.config,
            has("--restore"),
            has("--keep-running"),
            true,
            s.control.as_deref(),
            |rt| {
                rt.observe_traces(cache_watch::observer(warn));
                Ok(())
            },
        )
    })
}

/// `serve`: `run`, kept running, with requests posted to the first instance (the root, or the
/// one agent) and its answers on the machine `client`.
#[cfg(not(feature = "http"))]
fn cmd_serve(_args: &[String]) -> Result<(), String> {
    Err("serve: needs Affordance built with the http feature".into())
}

/// `serve` (see above).
#[cfg(feature = "http")]
fn cmd_serve(args: &[String]) -> Result<(), String> {
    let (s, d) = prepare_run(args, &["--restore", "--fresh", "--listen", "--control"])?;
    let mut settings = serve::Settings::from_config(&d.config["serve"])?;
    if let Some(listen) = &s.listen {
        settings.listen = listen.clone();
    }
    let restore = s.flags.iter().any(|x| x == "--restore");
    let warn = flags::Flags::load(&s.machine_flags)?.cache_warn()?;
    run::with_profile(|| {
        run::run(d.config, restore, true, true, s.control.as_deref(), |rt| {
            rt.observe_traces(cache_watch::observer(warn));
            let target = rt
                .specs()
                .first()
                .map(|s| s.id.clone())
                .ok_or("serve: the deployment has no instance")?;
            serve::start(rt, settings, target)
        })
    })
}

/// `host`: `run`, kept running, driven by a frontend over stdio (JSONL; see `host.rs`).
fn cmd_host(args: &[String]) -> Result<(), String> {
    let (s, d) = prepare_run(args, &["--restore", "--fresh", "--control"])?;
    let restore = s.flags.iter().any(|x| x == "--restore");
    let out = host::Out::new();
    let result = run::with_profile(|| {
        run::run(d.config, restore, true, false, s.control.as_deref(), |rt| {
            let target = rt
                .specs()
                .first()
                .map(|s| s.id.clone())
                .ok_or("host: the deployment has no instance")?;
            host::start(rt, out.clone(), target)
        })
    });
    out.send(match &result {
        Ok(()) => serde_json::json!({"type": "exit"}),
        Err(e) => serde_json::json!({"type": "exit", "error": e}),
    });
    result
}

/// The deployment for `run` or `serve`, refusing to run over an earlier run without a choice.
fn prepare_run(args: &[String], flags: &[&str]) -> Result<(Selection, deploy::Deployment), String> {
    let s = select(args, flags)?;
    let has = |f: &str| s.flags.iter().any(|x| x == f);
    if has("--restore") && has("--fresh") {
        return Err("--restore and --fresh are opposites; pick one".into());
    }
    let base = deploy::find(s.path.as_deref())?;
    let d = deploy::load(&base, &s.overrides)?;
    let dir = PathBuf::from(d.config["dir"].as_str().unwrap_or(deploy::STATE));
    if !has("--restore") && !has("--fresh") && holds_a_run(&dir) {
        return Err(format!(
            "{} holds a run: --restore to continue it, or --fresh to start over (its instances are kept under retired/)",
            dir.display()
        ));
    }
    Ok((s, d))
}

/// Whether `dir` holds an earlier run.
fn holds_a_run(dir: &Path) -> bool {
    affordance::runtime::layout::holds_a_run(dir)
}

fn cmd_config(args: &[String]) -> Result<(), String> {
    let s = select(args, &[])?;
    let base = deploy::find(s.path.as_deref())?;
    let d = deploy::load(&base, &s.overrides)?;
    let files: Vec<String> = d.files.iter().map(|f| f.display().to_string()).collect();
    println!("# resolved from: {}", files.join(", "));
    let text = serde_saphyr::to_string(&Plain(&deploy::redacted(&d.config))).map_err(|e| e.to_string())?;
    print!("{text}");
    Ok(())
}

/// A JSON value serialized as plain data (serde_json's own number type serializes as a private
/// map under arbitrary precision, which a YAML writer would print verbatim).
struct Plain<'a>(&'a serde_json::Value);

impl serde::Serialize for Plain<'_> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::{SerializeMap, SerializeSeq};
        use serde_json::Value as J;
        match self.0 {
            J::Null => s.serialize_unit(),
            J::Bool(b) => s.serialize_bool(*b),
            J::Number(n) => match (n.as_u64(), n.as_i64(), n.as_f64()) {
                (Some(u), _, _) => s.serialize_u64(u),
                (_, Some(i), _) => s.serialize_i64(i),
                (_, _, Some(f)) => s.serialize_f64(f),
                _ => s.serialize_str(&n.to_string()),
            },
            J::String(t) => s.serialize_str(t),
            J::Array(a) => {
                let mut seq = s.serialize_seq(Some(a.len()))?;
                for v in a {
                    seq.serialize_element(&Plain(v))?;
                }
                seq.end()
            }
            J::Object(o) => {
                let mut map = s.serialize_map(Some(o.len()))?;
                for (k, v) in o {
                    map.serialize_entry(k, &Plain(v))?;
                }
                map.end()
            }
        }
    }
}
