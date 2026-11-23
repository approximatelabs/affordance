//! A run driven from outside over its control socket (`affordance run --control PATH`) and by
//! signals, as a fleet manager and a terminal drive one: the real command, a real socket.
use crate::controller::Controller;
use serde_json::{Value as J, json};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// A deployment in a fresh directory: `app` as its one agent (no model), kept running.
fn deployment(name: &str, app: &str, home: Option<&Path>) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("aff-control-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("app.tsx"), app).unwrap();
    let mut d = json!({
        "app": {"view": "app.tsx", "entry": "Main"},
        "model": {"kind": "chat", "url": "http://127.0.0.1:9/v1", "model": "x", "context": 1000},
        "agents": [{"id": "a", "model": null}],
        "dir": "run",
        "keep_running": true,
        "max_wall_s": 60
    });
    if let Some(home) = home {
        d["machines"] = json!({"home": {"kind": "local", "home": home}});
    }
    std::fs::write(dir.join("deployment.json"), d.to_string()).unwrap();
    dir
}

fn start(dir: &Path, restore: bool) -> Child {
    let mut command = Command::new(env!("CARGO_BIN_EXE_affordance"));
    command
        .arg("run")
        .arg("deployment.json")
        .arg("--control")
        .arg(dir.join("control.sock"))
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if restore {
        command.arg("--restore");
    }
    command.spawn().unwrap()
}

fn summary(child: Child) -> J {
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "{:?}", out.status);
    serde_json::from_slice(&out.stdout).unwrap()
}

fn wait_for(what: &str, mut ok: impl FnMut() -> bool) {
    let until = Instant::now() + Duration::from_secs(30);
    while !ok() {
        assert!(Instant::now() < until, "{what}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

const COUNTER: &str = r#"export function Main() {
  const [n, setN] = useState(0, "n");
  useEvent("add", ({by}) => setN(x => x + by));
  return <box>{`n=${n}`}</box>;
}"#;

#[test]
fn a_controller_posts_reads_the_status_and_closes_a_run_over_its_socket() {
    let dir = deployment("socket", COUNTER, None);
    let child = start(&dir, false);
    let mut c = Controller::connect(&dir.join("control.sock"));
    wait_for("the run starts", || {
        c.call(json!({"type": "status"}))["value"]["state"] == "running"
    });
    // A post is answered once its line is committed; one to no such instance says so.
    let posted = c.call(json!({"type": "event", "name": "add", "payload": {"by": 2}, "key": "op-1"}));
    assert_eq!(posted["type"], "result", "{posted}");
    let missing = c.call(json!({"type": "event", "target": "nobody", "name": "add"}));
    assert_eq!(
        (missing["type"].as_str(), missing["message"].as_str()),
        (Some("error"), Some("no instance nobody"))
    );
    let unknown = c.call(json!({"type": "pause"}));
    assert_eq!(unknown["type"], "error", "{unknown}");
    assert_eq!(c.call(json!({"type": "set_model_limit", "limit": 0}))["type"], "result");
    let status = c.call(json!({"type": "status"}))["value"].clone();
    assert_eq!(status["agents"][0]["id"], "a", "{status}");
    // A second controller sees the same run; the first disconnecting leaves it running.
    drop(c);
    let mut c = Controller::connect(&dir.join("control.sock"));
    assert_eq!(c.call(json!({"type": "status"}))["value"]["state"], "running");
    c.close(10_000);
    let exit = std::iter::from_fn(|| c.line())
        .find(|l| l["type"] == "exit")
        .expect("told the run ended");
    assert_eq!(exit["ended"], "closed");
    let report = summary(child);
    assert_eq!(report["ended"], "closed");
    assert!(!dir.join("control.sock").exists(), "the socket is removed with the run");
    // Restored, the same key is taken once: posted again, it is journaled but changes nothing.
    let child = start(&dir, true);
    let mut c = Controller::connect(&dir.join("control.sock"));
    let again = c.call(json!({"type": "event", "name": "add", "payload": {"by": 2}, "key": "op-1"}));
    assert_eq!(again["type"], "result", "{again}");
    let more = c.call(json!({"type": "event", "name": "add", "payload": {"by": 5}, "key": "op-2"}));
    assert_eq!(more["type"], "result", "{more}");
    c.close(0);
    summary(child);
    let snapshot: J = serde_json::from_slice(&std::fs::read(dir.join("run/a/snapshot.json")).unwrap()).unwrap();
    assert_eq!(
        snapshot["state"]["__ui"]["@n"], 7,
        "2, then 5; the repeated key changed nothing: {snapshot}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Ctrl-C (SIGINT) or SIGTERM closes the run: running calls get time to finish, a second signal
/// closes it at once, and either way the command the agent was running is stopped and the run can
/// be restored.
#[test]
fn a_signal_closes_the_run_and_stops_its_commands() {
    let marker = format!("aff-signal-{}", std::process::id());
    let home = std::env::temp_dir().join(format!("{marker}-home"));
    std::fs::create_dir_all(&home).unwrap();
    let app = format!(
        r#"export function Main() {{
  const [done, setDone] = useState(false, "done");
  useEffect(() => {{ (async () => {{
    await harness.machine.get("home").call({{cmd: "touch started; exec -a {marker} sleep 60"}});
    setDone(true);
  }})(); }}, []);
  return <box>{{`done=${{done}}`}}</box>;
}}"#
    );
    let dir = deployment("signal", &app, Some(&home));
    let child = start(&dir, false);
    wait_for("the command starts", || home.join("started").exists());
    let running = || {
        Command::new("pgrep")
            .arg("-f")
            .arg(&marker)
            .output()
            .unwrap()
            .status
            .success()
    };
    assert!(running());
    let pid = child.id().to_string();
    let signal = |name: &str| {
        assert!(
            Command::new("kill")
                .args(["-s", name, &pid])
                .status()
                .unwrap()
                .success()
        )
    };
    signal("TERM");
    std::thread::sleep(Duration::from_millis(500));
    assert!(running(), "the first signal gives the command time to finish");
    signal("INT");
    let report = summary(child);
    assert_eq!(report["ended"], "closed");
    wait_for("the command is stopped", || !running());
    // Not lost: restored, the command is issued again.
    std::fs::remove_file(home.join("started")).unwrap();
    let child = start(&dir, true);
    wait_for("the restored run issues the command again", || {
        home.join("started").exists()
    });
    signal_twice(&child);
    summary(child);
    wait_for("the command is stopped", || !running());
    std::fs::remove_dir_all(&dir).unwrap();
    let _ = std::fs::remove_dir_all(&home);
}

fn signal_twice(child: &Child) {
    let pid = child.id().to_string();
    for _ in 0..2 {
        assert!(
            Command::new("kill")
                .args(["-s", "TERM", &pid])
                .status()
                .unwrap()
                .success()
        );
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// "A restored agent keeps its own secret." Two agents override the deployment's secret `KEY` with
/// values of their own. Secret values are never saved in the run directory; restored, each agent
/// is given its own value again from the deployment, not the deployment-wide one.
#[test]
fn a_restored_agent_keeps_its_own_secret() {
    let dir = std::env::temp_dir().join(format!("aff-control-secret-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("app.tsx"),
        r#"export function Main() {
  const [seen, setSeen] = useState("", "seen");
  useEvent("check", () => { setSeen(harness.env.KEY); harness.stop({length: harness.env.KEY.length}); });
  return <box>waiting</box>;
}"#,
    )
    .unwrap();
    let (wide, a, b) = (
        "deployment-wide-value",
        "alpha-agent-secret-1",
        "bravo-agent-secret-value-22",
    );
    let d = json!({
        "app": {"view": "app.tsx", "entry": "Main"},
        "model": {"kind": "chat", "url": "http://127.0.0.1:9/v1", "model": "x", "context": 1000},
        "env": {"KEY": wide},
        "secrets": ["KEY"],
        "agents": [{"id": "a", "model": null, "env": {"KEY": a}}, {"id": "b", "model": null, "env": {"KEY": b}}],
        "dir": "run",
        "keep_running": true,
        "max_wall_s": 60
    });
    std::fs::write(dir.join("deployment.json"), d.to_string()).unwrap();
    let child = start(&dir, false);
    let mut c = Controller::connect(&dir.join("control.sock"));
    wait_for("the run starts", || {
        c.call(json!({"type": "status"}))["value"]["state"] == "running"
    });
    c.close(0);
    summary(child);
    let saved = std::fs::read_to_string(dir.join("run/a/agent.json")).unwrap();
    assert!(!saved.contains(a), "a secret value is never saved: {saved}");
    let child = start(&dir, true);
    let mut c = Controller::connect(&dir.join("control.sock"));
    for id in ["a", "b"] {
        let posted = c.call(json!({"type": "event", "target": id, "name": "check"}));
        assert_eq!(posted["type"], "result", "{posted}");
    }
    let report = summary(child);
    let stopped = |id: &str| {
        report["per_agent"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["id"] == id)
            .unwrap()["stopped"]["length"]
            .clone()
    };
    assert_eq!((stopped("a"), stopped("b")), (json!(a.len()), json!(b.len())));
    // The app kept its secret in its state; the snapshot, which people read, shows it redacted.
    let snapshot = std::fs::read_to_string(dir.join("run/a/snapshot.json")).unwrap();
    assert!(!snapshot.contains(a) && snapshot.contains("[secret:KEY]"), "{snapshot}");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// "A journal missing committed inputs is not replayed as if they never happened." After a clean
/// close, one agent's journal loses its last line (a disk that lost what it said it had written).
/// Restored, that agent fails with what is missing and runs nothing again; the other agent is
/// restored and goes on.
#[test]
fn a_journal_missing_committed_inputs_fails_its_own_restore() {
    let dir = std::env::temp_dir().join(format!("aff-control-short-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("app.tsx"), COUNTER).unwrap();
    let d = json!({
        "app": {"view": "app.tsx", "entry": "Main"},
        "model": {"kind": "chat", "url": "http://127.0.0.1:9/v1", "model": "x", "context": 1000},
        "agents": [{"id": "a", "model": null}, {"id": "b", "model": null}],
        "dir": "run",
        "keep_running": true,
        "max_wall_s": 60
    });
    std::fs::write(dir.join("deployment.json"), d.to_string()).unwrap();
    let child = start(&dir, false);
    let mut c = Controller::connect(&dir.join("control.sock"));
    for id in ["a", "b"] {
        let posted = c.call(json!({"type": "event", "target": id, "name": "add", "payload": {"by": 3}}));
        assert_eq!(posted["type"], "result", "{posted}");
    }
    c.close(0);
    summary(child);
    // The last committed line of a's journal is gone.
    let journal = dir.join("run/a/journal.jsonl");
    let bytes = std::fs::read(&journal).unwrap();
    let body = &bytes[..bytes.len() - 1];
    let cut = body.iter().rposition(|&b| b == b'\n').map_or(0, |at| at + 1);
    std::fs::write(&journal, &bytes[..cut]).unwrap();
    let _ = std::fs::remove_file(dir.join("run/a/checkpoint.json"));
    let child = start(&dir, true);
    let mut c = Controller::connect(&dir.join("control.sock"));
    let status = c.call(json!({"type": "status"}));
    wait_for("the run starts", || {
        c.call(json!({"type": "status"}))["value"]["state"] == "running"
    });
    let posted = c.call(json!({"type": "event", "target": "b", "name": "add", "payload": {"by": 4}}));
    assert_eq!(posted["type"], "result", "{posted} {status}");
    c.close(0);
    let out = child.wait_with_output().unwrap();
    let report: J = serde_json::from_slice(&out.stdout).unwrap();
    let agent = |id: &str| {
        report["per_agent"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["id"] == id)
            .unwrap()
            .clone()
    };
    let a = agent("a");
    assert!(
        a["error"]
            .as_str()
            .is_some_and(|e| e.contains("shorter than its last commit")),
        "{a}"
    );
    assert!(agent("b")["error"].is_null(), "{}", agent("b"));
    let snapshot: J = serde_json::from_slice(&std::fs::read(dir.join("run/b/snapshot.json")).unwrap()).unwrap();
    assert_eq!(snapshot["state"]["__ui"]["@n"], 7, "{snapshot}");
    assert_eq!(
        std::fs::read(&journal).unwrap().len(),
        cut,
        "a's journal is left as it was"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
