//! `affordance run`, `config` and `init`: finding a deployment, layering files and overrides,
//! `${VAR}`s, paths relative to the file that names them, the journal directory, and an app
//! outside any checkout importing the bundled library.
use serde_json::json;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn affordance(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_affordance"))
        .args(args)
        .current_dir(dir)
        .env_remove("DEPLOY_TEST_URL")
        .output()
        .unwrap()
}

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("aff-cli-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // The real path: on macOS the temp directory is behind a symlink, and a resolved deployment
    // names real paths.
    dir.canonicalize().unwrap()
}

fn stdout(out: &Output) -> String {
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    assert!(!out.status.success(), "{}", String::from_utf8_lossy(&out.stdout));
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// Layers apply in order: the file, affordance.local.yaml, each -f, each --set; maps merge, lists
/// and scalars replace, null removes. Paths are relative to the file that names them, `${VAR}`s
/// come from the environment, and secrets are hidden.
#[test]
fn config_layers_files_and_overrides_in_order() {
    let dir = temp("layers");
    std::fs::create_dir_all(dir.join("app")).unwrap();
    std::fs::write(
        dir.join("affordance.yaml"),
        "app: {view: app/main.tsx}\nmodel: {kind: chat, url: http://base/v1, model: m, context: 100}\n\
         instances: [{id: a, max_turns: 2}, {id: b}]\nworkers: 2\nenv: {KEY: \"${DEPLOY_TEST_KEY:-unset}\"}\nsecrets: [KEY]\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("affordance.local.yaml"),
        "model: {url: http://local/v1}\nworkers: 3\n",
    )
    .unwrap();
    std::fs::create_dir_all(dir.join("over")).unwrap();
    std::fs::write(dir.join("over/more.yaml"), "model: {context: 200}\nlibrary: lib\n").unwrap();
    let out = affordance(
        &dir,
        &[
            "config",
            "-f",
            "over/more.yaml",
            "--set",
            "instances[1].max_turns=9",
            "--set",
            "workers=null",
            "--set",
            "model.model=\"0.10\"",
        ],
    );
    let text = stdout(&out);
    let body = text.split_once('\n').unwrap().1;
    let config: serde_json::Value = serde_saphyr::from_str(body).unwrap();
    assert_eq!(config["model"]["url"], "http://local/v1", "{text}");
    assert_eq!(config["model"]["context"], 200);
    assert_eq!(config["model"]["model"], "0.10", "a quoted value stays a string");
    assert_eq!(
        config["instances"],
        json!([{"id": "a", "max_turns": 2}, {"id": "b", "max_turns": 9}])
    );
    assert!(config.get("workers").is_none(), "null removes a key");
    // Paths: relative to the file that names them.
    let canon = |p: &Path| p.to_string_lossy().into_owned();
    assert_eq!(config["app"]["view"], canon(&dir.join("app/main.tsx")));
    assert_eq!(config["library"], canon(&dir.join("over/lib")));
    assert_eq!(config["dir"], canon(&dir.join(".affordance")));
    assert_eq!(config["env"]["KEY"], "<redacted>");
    assert!(text.starts_with("# resolved from: "), "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// `affordance run` finds affordance.yaml from a subdirectory; a misnamed file, an unset
/// variable and a rerun over an existing run each say what to do.
#[test]
fn run_finds_its_file_and_explains_what_it_will_not_do() {
    let dir = temp("find");
    std::fs::create_dir_all(dir.join("deep/er")).unwrap();
    std::fs::write(dir.join("affordance.yaml"), "app: {view: x.tsx}\n").unwrap();
    let found = stdout(&affordance(&dir.join("deep/er"), &["config"]));
    assert!(
        found.contains(&dir.join("affordance.yaml").display().to_string()),
        "{found}"
    );

    let misnamed = temp("misnamed");
    std::fs::write(misnamed.join("affordance.yml"), "app: {view: x.tsx}\n").unwrap();
    assert!(stderr(&affordance(&misnamed, &["run"])).contains("rename it to affordance.yaml"));

    std::fs::write(dir.join("affordance.yaml"), "model: {url: \"${DEPLOY_TEST_URL}\"}\n").unwrap();
    let unset = stderr(&affordance(&dir, &["config"]));
    assert!(
        unset.contains("${DEPLOY_TEST_URL} is not set") && unset.contains("model.url"),
        "{unset}"
    );

    std::fs::write(dir.join("affordance.yaml"), "app: {view: x.tsx}\n").unwrap();
    // A directory that holds an earlier run (its record).
    std::fs::create_dir_all(dir.join(".affordance")).unwrap();
    std::fs::write(dir.join(".affordance/run.json"), "{}").unwrap();
    let rerun = stderr(&affordance(&dir, &["run"]));
    assert!(rerun.contains("--restore") && rerun.contains("--fresh"), "{rerun}");
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&misnamed);
}

/// An OpenAI-compatible stand-in that answers with `replies` in order.
fn server(replies: Vec<serde_json::Value>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for reply in replies {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let (mut length, mut line) = (0, String::new());
            while reader.read_line(&mut line).unwrap() > 0 && line != "\r\n" {
                if let Some(n) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    length = n.trim().parse().unwrap();
                }
                line.clear();
            }
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();
            let text = reply.to_string();
            write!(
                &stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}",
                text.len()
            )
            .unwrap();
        }
    });
    url
}

/// `affordance init` writes an agent that runs outside any checkout: its `@affordance/` imports
/// come from the library bundled in the binary, its tokenizer is the default. A second init into
/// the same directory refuses.
#[test]
fn an_initialized_agent_runs_outside_a_checkout() {
    let dir = temp("init");
    let made = stdout(&affordance(&dir, &["init", "agent"]));
    assert!(made.contains("affordance run"), "{made}");
    assert!(dir.join("agent/affordance.d.ts").is_file() && dir.join("agent/tsconfig.json").is_file());
    assert!(stderr(&affordance(&dir, &["init", "agent"])).contains("already exists"));
    let answer = json!({"choices": [{"message": {"content": null, "tool_calls": [{"id": "c1", "type": "function",
        "function": {"name": "answer", "arguments": "{\"text\": \"done\"}"}}]}, "finish_reason": "tool_calls"}],
        "usage": {"prompt_tokens": 10, "completion_tokens": 5}});
    let url = server(vec![answer]);
    let out = affordance(&dir.join("agent"), &["run", "--set", &format!("model.url={url}")]);
    let report: serde_json::Value = serde_json::from_str(&stdout(&out)).unwrap();
    assert_eq!(
        report["per_instance"][0]["stopped"],
        json!({"answer": "done"}),
        "{report}"
    );
    assert!(crate::shared::inst(&dir, "agent/.affordance/first/journal.jsonl").is_file());
    let _ = std::fs::remove_dir_all(&dir);
}

/// Every deployment the repository ships (examples and demos) loads: it parses, and its layers and
/// paths resolve.
#[test]
fn every_shipped_deployment_loads() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut found = 0;
    for dir in ["examples", "views"] {
        let mut stack = vec![root.join(dir)];
        while let Some(d) = stack.pop() {
            for entry in std::fs::read_dir(&d).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.file_name().is_some_and(|n| n == "affordance.yaml") {
                    found += 1;
                    let out = affordance(&root, &["config", path.to_str().unwrap()]);
                    assert!(
                        out.status.success(),
                        "{}: {}",
                        path.display(),
                        String::from_utf8_lossy(&out.stderr)
                    );
                }
            }
        }
    }
    assert!(found >= 5, "found {found} deployments");
}

/// The library's file tools (`@affordance/tools/files.tsx`) read, write and edit text files through
/// a machine's shell: a path with a space and a quote, content with quotes and newlines, a range of
/// lines, an exact single replacement, and failures a model can act on.
#[test]
fn the_file_tools_read_write_and_edit_through_the_shell() {
    let dir = temp("files");
    std::fs::write(
        dir.join("app.tsx"),
        r#"import { fileTools } from "@affordance/tools/files.tsx";
export default function Main() {
  useEffect(() => {
    const go = async () => {
      const {read, write, edit} = fileTools(harness.machine.get("home"));
      const path = "notes/it's a file.txt";
      const out = [];
      const failing = async f => { try { await f(); return "no error"; } catch (e) { return e.message; } };
      out.push(await write({path, content: "one\ntwo 'q' \"d\" $HOME\nthree\nthree\n"}));
      out.push(await read({path}));
      out.push(await edit({path, old_string: "two 'q' \"d\" $HOME", new_string: "2"}));
      out.push(await read({path, offset: 2, limit: 1}));
      out.push(await failing(() => edit({path, old_string: "three", new_string: "3"})));
      out.push(await edit({path, old_string: "three", new_string: "3", replace_all: true}));
      out.push(await read({path}));
      out.push(await failing(() => edit({path, old_string: "absent", new_string: "x"})));
      out.push((await failing(() => read({path: "missing.txt"}))).includes("No such file"));
      harness.stop(out);
    };
    go();
  }, []);
  return "";
}
"#,
    )
    .unwrap();
    std::fs::write(
        dir.join("affordance.yaml"),
        "instances: [{id: root, app: app.tsx}]\nmodel: {kind: chat, url: \"http://127.0.0.1:9/v1\", model: none, context: 4096}\nmachines:\n  home: {kind: local, home: work}\n",
    )
    .unwrap();
    std::fs::create_dir_all(dir.join("work")).unwrap();
    let out = affordance(&dir, &["run"]);
    let report: serde_json::Value =
        serde_json::from_str(&stdout(&out)).unwrap_or_else(|e| panic!("{e}: {}", stderr(&out)));
    assert_eq!(
        report["per_instance"][0]["stopped"],
        json!([
            "wrote notes/it's a file.txt",
            "one\ntwo 'q' \"d\" $HOME\nthree\nthree\n",
            "edited notes/it's a file.txt (1 replacement)",
            "2\n",
            "old_string matches 2 places in notes/it's a file.txt: give more of the surrounding text, or set replace_all",
            "edited notes/it's a file.txt (2 replacements)",
            "one\n2\n3\n3\n",
            "old_string was not found in notes/it's a file.txt",
            true
        ]),
        "{report}"
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("work/notes/it's a file.txt")).unwrap(),
        "one\n2\n3\n3\n"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// With no model server running, `affordance run` says so on the terminal, keeps trying for the
/// binding's `retry.for_s`, then gives the app the error and ends with a failure. Nothing listens on port 9.
#[test]
fn a_run_with_no_model_server_says_so_and_stops_when_its_retry_time_has_passed() {
    let dir = temp("unreachable");
    stdout(&affordance(&dir, &["init", "agent"]));
    let started = std::time::Instant::now();
    let out = affordance(
        &dir.join("agent"),
        &[
            "run",
            "--set",
            "model.url=http://127.0.0.1:9/v1",
            "--set",
            "model.retry.for_s=2",
        ],
    );
    assert!(started.elapsed() < std::time::Duration::from_secs(30));
    let said = String::from_utf8_lossy(&out.stderr);
    assert!(
        said.contains("first: the model call failed (chat: unreachable"),
        "{said}"
    );
    assert!(said.contains("Trying again in 1 s"), "{said}");
    assert!(said.contains("is not tried again: the app gets the error"), "{said}");
    // The report is printed, and the run fails: a script can tell the agent never got an answer.
    assert!(!out.status.success());
    assert!(said.contains("1 of 1 instances failed"), "{said}");
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["per_instance"][0]["stopped"]["reason"], "loop_error", "{report}");
}

/// Render a one-file app with the command and return the text it prints (or its error).
fn render(dir: &Path, source: &str, state: &str) -> String {
    std::fs::write(dir.join("app.tsx"), source).unwrap();
    std::fs::write(dir.join("state.json"), state).unwrap();
    let out = affordance(dir, &["render", "app.tsx", "--state", "state.json"]);
    let all = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    all.lines()
        .find_map(|l| l.strip_prefix("text    ").map(str::to_owned))
        .unwrap_or(all)
}

/// Data never changes what code means: a state key or prop named after a platform function or a
/// language global does not replace it.
#[test]
fn a_state_key_does_not_replace_a_platform_name() {
    let dir = temp("names");
    let text = render(
        &dir,
        "export default function Main({model}) { const [n] = useState(3, 'n'); \
         return `${Math.max(1, 2)} ${JSON.stringify({a: n})} ${model}`; }",
        r#"{"model": "gpt", "JSON": 5, "Math": {"max": 7}, "useState": 1}"#,
    );
    assert_eq!(text, r#"2 {"a":3} gpt"#);
}

/// Control flow and evaluation order as JavaScript has them.
#[test]
fn break_ternary_arrows_and_spread_order_follow_javascript() {
    let dir = temp("language");
    let text = render(
        &dir,
        "export default function Main({flag}) { const out = []; \
         for (let i = 0; i < 3; i++) { blk: { out.push('in' + i); if (i === 1) break; } out.push('after' + i); } \
         const id = v => v; const f = flag ? id : x => x * 2; \
         let x = [1]; const g = () => { x.push(2); return 3; }; x = [...x, g()]; \
         return `${out.join(',')} ${f(4)} ${JSON.stringify(x)}`; }",
        r#"{"flag": false}"#,
    );
    assert_eq!(text, "in0,after0,in1 8 [1,3]");
}

/// Values and formats that used to end the process are errors an app can catch.
#[test]
fn a_cyclic_value_a_huge_indent_and_a_bad_date_format_are_errors() {
    let dir = temp("bounds");
    let text = render(
        &dir,
        "export default function Main() { const a = [1]; a.push(a); const said = []; \
         for (const f of [() => formatJSON(a), () => formatJSON({a: 1}, {indent: 4000000000000}), () => formatNow('%Q %')]) \
         { try { f(); said.push('no error'); } catch (e) { said.push(e.message); } } return said.join(' | '); }",
        "{}",
    );
    assert_eq!(
        text,
        "Converting circular structure to JSON | formatJSON indent is at most 64 | formatNow: `%Q %` is not a strftime format"
    );
}

/// An action written after a box's last token is still offered when the box overflows and keeps
/// its end.
#[test]
fn an_action_at_the_end_of_an_overflowing_box_is_kept() {
    let dir = temp("box-end");
    std::fs::write(
        dir.join("app.tsx"),
        "export default function Main() { return <box max_tokens={4} overflow=\"clip-start\">\
         {\"one two three four five six seven eight\"}<aff label=\"t\" name=\"go\" onGo={() => 1}></aff></box>; }",
    )
    .unwrap();
    std::fs::write(dir.join("state.json"), "{}").unwrap();
    let shown = stdout(&affordance(&dir, &["render", "app.tsx", "--state", "state.json"]));
    assert!(shown.contains(" five six seven eight"), "{shown}");
    assert!(shown.contains("action     go()"), "{shown}");
}

/// A machine command of several lines runs every line, also after one that reads standard input,
/// and a fractional `timeout_s` is a time limit.
#[test]
fn a_command_runs_its_later_lines_after_one_that_reads_standard_input() {
    let dir = temp("stdin");
    std::fs::write(
        dir.join("app.tsx"),
        "export default function Main() { useEffect(() => { const go = async () => { \
         const r = await harness.machine.get('home').call({cmd: 'echo first\\ncat\\necho AFTER\\nexit 7', timeout_s: 5.5}); \
         harness.stop({output: r.output, exit: r.exit}); }; go(); }, []); return 'x'; }",
    )
    .unwrap();
    std::fs::write(
        dir.join("affordance.yaml"),
        "instances: [{id: root, app: app.tsx}]\nmodel: {kind: chat, url: \"http://127.0.0.1:9/v1\", model: x, context: 1000}\nmachines: {home: {kind: local}}\n",
    )
    .unwrap();
    let report: serde_json::Value = serde_json::from_str(&stdout(&affordance(&dir, &["run"]))).unwrap();
    assert_eq!(
        report["per_instance"][0]["stopped"],
        json!({"output": "first\nAFTER\n", "exit": 7}),
        "{report}"
    );
}

/// `affordance config` hides a secret's value wherever the deployment puts it.
#[test]
fn config_hides_a_secret_value_in_props_and_headers() {
    let dir = temp("secret");
    std::fs::write(
        dir.join("affordance.yaml"),
        "app: {view: app.tsx}\nenv: {API: \"${AUDIT_SECRET}\"}\nsecrets: [API]\n\
         model: {kind: chat, url: http://base/v1, model: m, context: 100, headers: {X-Gateway: \"token ${AUDIT_SECRET}\"}}\n\
         instances: [{id: a, props: {key: \"${AUDIT_SECRET}\"}}]\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_affordance"))
        .args(["config"])
        .current_dir(&dir)
        .env("AUDIT_SECRET", "sk-very-secret-value")
        .output()
        .unwrap();
    let shown = stdout(&out);
    assert!(!shown.contains("sk-very-secret-value"), "{shown}");
    assert_eq!(shown.matches("<redacted>").count(), 3, "{shown}");
}

/// The app `affordance init` writes can be tested with a scripted model, and a reply with no tool
/// call ends the agent with that reply as its answer. A scenario with no steps is an error.
#[test]
fn an_initialized_agent_is_testable_and_a_prose_reply_is_its_answer() {
    let dir = temp("test-init");
    stdout(&affordance(&dir, &["init", "agent"]));
    let agent = dir.join("agent");
    std::fs::write(
        agent.join("prose.json"),
        r#"{"template": "messages", "props": {"task": "count"}, "steps": [
            {"model": {"text": "There are 3 files."}},
            {"expect": {"stopped": {"answer": "There are 3 files."}, "model_calls": 0}}]}"#,
    )
    .unwrap();
    assert!(stdout(&affordance(&agent, &["test", "app.tsx", "prose.json"])).contains("PASS"));
    std::fs::write(agent.join("none.json"), r#"{"stepz": []}"#).unwrap();
    let refused = affordance(&agent, &["test", "app.tsx", "none.json"]);
    assert!(!refused.status.success());
}
