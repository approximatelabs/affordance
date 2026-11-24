//! Group commit and crash recovery: journals are committed together, and a crash between two
//! commits resumes from a consistent cut of every journal.
use crate::tokenizer;
use serde_json::{Value as J, json};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Askers send numbered messages, one at a time, to a hub on its own worker, each carrying the
/// clock it was sent at; the hub records every message it takes and echoes it back.
const APP: &str = r#"export default function Main({role, hub, n}) {
  const [k, setK] = useState(0, 'k');
  const [sent, setSent] = useState([], 'sent');
  const [got, setGot] = useState([], 'got');
  useEvent('op', p => { setGot(g => [...g, [p.from, p.seq, p.at]]); harness.send(p.from, {seq: p.seq}, {event: 'echo'}); });
  useEvent('echo', p => setK(p.seq + 1));
  useEffect(() => {
    if (role !== 'asker') return;
    if (k < n) { const at = harness.clock_ms; setSent(s => [...s, at]); harness.send(hub, {seq: k, at}, {event: 'op'}); }
    else harness.stop({done: n});
  }, [k]);
  return 'echo';
}"#;
const ASKERS: usize = 3;
const N: usize = 1500;

fn commit_cut(dir: &Path) -> serde_json::Map<String, J> {
    let mut cut = serde_json::Map::new();
    for line in std::fs::read_to_string(dir.join("commit.jsonl")).unwrap().lines() {
        let Ok(v) = serde_json::from_str::<J>(line) else { break };
        for (id, at) in v["journals"].as_object().unwrap() {
            cut.insert(id.clone(), at.clone());
        }
    }
    cut
}
fn len(path: &Path) -> u64 {
    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}
fn state(dir: &Path, id: &str) -> J {
    let snap: J =
        serde_json::from_slice(&std::fs::read(crate::shared::instance_dir(dir, id).join("snapshot.json")).unwrap())
            .unwrap();
    assert!(snap["error"].is_null(), "{id}: {}", snap["error"]);
    snap["state"]["__ui"].clone()
}

/// The process is killed between two commits. The askers then lose everything after the last
/// commit (as a power loss would) while the hub keeps all it wrote: the hub's journal holds
/// messages whose sends the askers no longer have. Recovery cuts every journal back to the last
/// commit, so on restore each message is taken exactly once, with the clock of the send the
/// asker's journal kept (a message from the lost timeline carries another clock).
#[test]
fn a_crash_between_commits_resumes_from_a_consistent_cut() {
    let root = std::env::temp_dir().join(format!("aff-group-commit-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let app = root.join("echo.tsx");
    std::fs::write(&app, APP).unwrap();
    let dir: PathBuf = root.join("run");
    let mut agents = vec![json!({"id": "hub", "model": null, "props": {"role": "hub"}, "dedicated_worker": true})];
    for i in 0..ASKERS {
        agents
            .push(json!({"id": format!("asker-{i}"), "model": null, "props": {"role": "asker", "hub": "hub", "n": N}}));
    }
    let cfg = json!({"app": {"view": app, "tokenizer": tokenizer::path()}, "model": {"kind": "sglang", "url": "http://127.0.0.1:1", "context": "none"},
        "instances": agents, "dir": dir, "workers": 2});
    let cfg_path = root.join("run.json");
    std::fs::write(&cfg_path, cfg.to_string()).unwrap();

    // The hub's journal, once the run has made its instance (empty until then).
    let hub_journal = || {
        crate::shared::find_instance(&dir, "hub")
            .map(|d| d.join("journal.jsonl"))
            .unwrap_or_default()
    };
    // A commit can land between seeing uncommitted lines and the kill arriving, and then there
    // is nothing for the power loss to take: the run is started again.
    let hub_uncommitted = 'run: {
        for _attempt in 0..20 {
            let _ = std::fs::remove_dir_all(&dir);
            let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_affordance"))
                .arg("run")
                .arg(&cfg_path)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .unwrap();
            let deadline = Instant::now() + Duration::from_secs(60);
            // Kill once a few commits are behind it and the hub has written past the last one.
            loop {
                assert!(Instant::now() < deadline, "the run made no progress");
                assert!(
                    child.try_wait().unwrap().is_none(),
                    "the run finished before it could be killed"
                );
                let commits = std::fs::read_to_string(dir.join("commit.jsonl"))
                    .map(|t| t.lines().count())
                    .unwrap_or(0);
                // And the hub's journal is longer than its last commit covers: that is what the kill
                // must leave uncommitted.
                if commits >= 4 && len(&hub_journal()) > 20_000 {
                    let committed = commit_cut(&dir).get("hub").map_or(0, |at| at[1].as_u64().unwrap());
                    if len(&hub_journal()) > committed {
                        break;
                    }
                }
                std::thread::yield_now();
            }
            child.kill().unwrap();
            child.wait().unwrap();

            let cut = commit_cut(&dir);
            let mut lost = 0;
            for i in 0..ASKERS {
                let journal = crate::shared::inst(&dir, &format!("asker-{i}/journal.jsonl"));
                let keep = cut.get(&format!("asker-{i}")).map_or(0, |at| at[1].as_u64().unwrap());
                lost += len(&journal) - keep;
                std::fs::OpenOptions::new()
                    .write(true)
                    .open(&journal)
                    .unwrap()
                    .set_len(keep)
                    .unwrap();
            }
            let hub_uncommitted = len(&hub_journal()) - cut["hub"][1].as_u64().unwrap();
            if lost + hub_uncommitted > 0 {
                break 'run hub_uncommitted;
            }
        }
        panic!("the kill fell on a commit 20 times; nothing was uncommitted");
    };

    let out = std::process::Command::new(env!("CARGO_BIN_EXE_affordance"))
        .arg("run")
        .arg(&cfg_path)
        .arg("--restore")
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    if hub_uncommitted > 0 {
        assert!(
            std::fs::read_dir(crate::shared::instance_dir(&dir, "hub"))
                .unwrap()
                .flatten()
                .any(|e| e.file_name().to_string_lossy().starts_with("uncommitted-journal-")),
            "the hub's uncommitted lines are set aside"
        );
    }
    let hub = state(&dir, "hub");
    let got = hub["@got"].as_array().unwrap();
    assert_eq!(got.len(), ASKERS * N, "every message is taken exactly once");
    for i in 0..ASKERS {
        let id = format!("asker-{i}");
        let asker = state(&dir, &id);
        assert_eq!(asker["@k"], json!(N));
        let sent = asker["@sent"].as_array().unwrap();
        assert_eq!(sent.len(), N);
        let taken: Vec<&J> = got.iter().filter(|m| m[0] == json!(id)).collect();
        assert_eq!(taken.len(), N);
        for (seq, m) in taken.iter().enumerate() {
            assert_eq!(m[1], json!(seq), "{id}: in order, once each");
            assert_eq!(m[2], sent[seq], "{id} #{seq}: the hub took the send the asker kept");
        }
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// Each step, an asker puts a store entry and sends the hub a 70 KiB text (a blob in the hub's
/// journal, and in its state); the hub echoes. Blob and store files are synced by the commits,
/// not by each write.
const FILES_APP: &str = r#"export default function Main({role, hub, n, me}) {
  const [k, setK] = useState(0, 'k');
  const [keys, setKeys] = useState([], 'keys');
  const [got, setGot] = useState([], 'got');
  const [last, setLast] = useState('', 'last');
  useEvent('op', p => { setGot(g => [...g, [p.from, p.seq, p.text.length, p.text.slice(-12)]]); setLast(p.text); harness.send(p.from, {seq: p.seq}, {event: 'echo'}); });
  useEvent('echo', p => setK(p.seq + 1));
  useEffect(() => {
    if (role !== 'asker') return;
    if (k >= n) { harness.stop({done: n}); return; }
    harness.store.put({text: me + ' note ' + k}).then(({key}) => {
      setKeys(ks => [...ks, [key, k]]);
      harness.send(hub, {seq: k, text: (me + ':' + k + ':').padEnd(70 * 1024, '.') + me + '#' + String(k).padStart(5, '0')}, {event: 'op'});
    });
  }, [k]);
  return 'files';
}"#;

/// Every blob reference in a JSON text: (sha256, bytes).
fn blob_refs(text: &str) -> Vec<(String, u64)> {
    let mut out = vec![];
    let mut rest = text;
    while let Some(i) = rest.find("\\u0000blob:") {
        let tail = &rest[i + "\\u0000blob:".len()..];
        let mut parts = tail.split(':');
        let (Some(hash), Some(bytes)) = (parts.next(), parts.next()) else {
            break;
        };
        out.push((hash.to_string(), bytes.parse().unwrap()));
        rest = &tail[64..];
    }
    out
}
fn prefix(path: &Path, len: u64) -> String {
    let bytes = std::fs::read(path).unwrap_or_default();
    String::from_utf8_lossy(&bytes[..(len as usize).min(bytes.len())]).into_owned()
}
fn cut_of(dir: &Path, kind: &str) -> serde_json::Map<String, J> {
    let mut cut = serde_json::Map::new();
    for line in std::fs::read_to_string(dir.join("commit.jsonl")).unwrap().lines() {
        let Ok(v) = serde_json::from_str::<J>(line) else { break };
        for (id, at) in v[kind].as_object().into_iter().flatten() {
            cut.insert(id.clone(), at.clone());
        }
    }
    cut
}

/// Killed between commits, then a power loss simulated: every journal and store loses what
/// the last commit did not cover, and every blob file no committed line (or synced recovery
/// image) refers to is left empty, as a file renamed into place before its data reached the
/// disk can be. On restore, every blob any journal, store or image refers to exists with its
/// content, every key an asker learned reads back its text, and the hub took each text once.
#[test]
fn a_crash_between_commits_keeps_every_blob_and_store_entry_a_committed_line_refers_to() {
    const N: usize = 40;
    let root = std::env::temp_dir().join(format!("aff-group-commit-files-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let app = root.join("files.tsx");
    std::fs::write(&app, FILES_APP).unwrap();
    let dir: PathBuf = root.join("run");
    let mut agents = vec![json!({"id": "hub", "model": null, "props": {"role": "hub"}, "dedicated_worker": true})];
    for i in 0..ASKERS {
        agents.push(json!({"id": format!("asker-{i}"), "model": null, "props": {"role": "asker", "hub": "hub", "n": N, "me": format!("asker-{i}")}}));
    }
    let cfg = json!({"app": {"view": app, "tokenizer": tokenizer::path()}, "model": {"kind": "sglang", "url": "http://127.0.0.1:1", "context": "none"},
        "instances": agents, "dir": dir, "workers": 2});
    let cfg_path = root.join("run.json");
    std::fs::write(&cfg_path, cfg.to_string()).unwrap();
    let mut ids = vec!["hub".to_string()];
    ids.extend((0..ASKERS).map(|i| format!("asker-{i}")));
    // A kill that falls right on a commit leaves nothing uncommitted to lose: run again.
    for attempt in 1.. {
        let _ = std::fs::remove_dir_all(&dir);
        let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_affordance"))
            .arg("run")
            .arg(&cfg_path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        // Kill it when a blob has appeared since the last commit: that blob is what the power
        // loss takes.
        let deadline = Instant::now() + Duration::from_secs(60);
        let mut at_commit = (0, 0);
        loop {
            assert!(Instant::now() < deadline, "the run made no progress");
            assert!(
                child.try_wait().unwrap().is_none(),
                "the run finished before it could be killed"
            );
            let commits = len(&dir.join("commit.jsonl"));
            let blobs = std::fs::read_dir(dir.join("blobs")).map(|d| d.count()).unwrap_or(0);
            if commits != at_commit.0 {
                at_commit = (commits, blobs);
            } else if blobs > at_commit.1 && blobs >= 40 {
                break;
            }
            std::thread::yield_now();
        }
        child.kill().unwrap();
        child.wait().unwrap();

        // The power loss.
        let (journals, stores) = (cut_of(&dir, "journals"), cut_of(&dir, "stores"));
        let mut keep = std::collections::BTreeSet::new();
        for id in &ids {
            for (file, cut) in [("journal.jsonl", &journals), ("store.jsonl", &stores)] {
                let path = crate::shared::instance_dir(&dir, id).join(file);
                let at = cut.get(id).map_or(0, |at| at[1].as_u64().unwrap());
                keep.extend(blob_refs(&prefix(&path, at)).into_iter().map(|(h, _)| h));
                if path.exists() {
                    std::fs::OpenOptions::new()
                        .write(true)
                        .open(&path)
                        .unwrap()
                        .set_len(at)
                        .unwrap();
                }
            }
            if let Ok(image) = std::fs::read_to_string(crate::shared::instance_dir(&dir, id).join("checkpoint.json")) {
                keep.extend(blob_refs(&image).into_iter().map(|(h, _)| h));
            }
        }
        let mut emptied = 0;
        for entry in std::fs::read_dir(dir.join("blobs")).unwrap().flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.len() == 64 && !keep.contains(&name) {
                std::fs::File::create(entry.path()).unwrap();
                emptied += 1;
            }
        }
        if emptied > 0 {
            if attempt > 1 {
                eprintln!("the kill fell on a commit {} times before this one", attempt - 1);
            }
            break;
        }
        assert!(
            attempt < 20,
            "the kill fell on a commit {attempt} times; every blob was committed"
        );
    }

    let out = std::process::Command::new(env!("CARGO_BIN_EXE_affordance"))
        .arg("run")
        .arg(&cfg_path)
        .arg("--restore")
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    // Every reference anywhere reads back its content.
    for id in &ids {
        for file in ["journal.jsonl", "store.jsonl", "checkpoint.json", "snapshot.json"] {
            let text = std::fs::read_to_string(crate::shared::instance_dir(&dir, id).join(file)).unwrap_or_default();
            for (hash, bytes) in blob_refs(&text) {
                let content = std::fs::read(dir.join("blobs").join(&hash))
                    .unwrap_or_else(|e| panic!("{id}/{file}: blob {hash}: {e}"));
                assert_eq!(content.len() as u64, bytes, "{id}/{file}: blob {hash} has its length");
                assert_eq!(
                    affordance::sha256_hex(&content),
                    hash,
                    "{id}/{file}: blob {hash} has its content"
                );
            }
        }
    }
    let hub = state(&dir, "hub");
    let got = hub["@got"].as_array().unwrap();
    assert_eq!(got.len(), ASKERS * N, "every text is taken exactly once");
    for i in 0..ASKERS {
        let id = format!("asker-{i}");
        let taken: Vec<&J> = got.iter().filter(|m| m[0] == json!(id)).collect();
        for (seq, m) in taken.iter().enumerate() {
            assert_eq!(m[1], json!(seq));
            assert_eq!(m[2], json!(70 * 1024 + id.len() + 6), "{id} #{seq}");
            assert_eq!(m[3], json!(format!("{}#{seq:05}", &id[id.len() - 6..])), "{id} #{seq}");
        }
        // Every key the asker learned reads back the text it put.
        let records: Vec<J> = std::fs::read_to_string(crate::shared::instance_dir(&dir, &id).join("store.jsonl"))
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        let keys = state(&dir, &id)["@keys"].as_array().unwrap().clone();
        assert_eq!(keys.len(), N);
        for pair in keys {
            let (key, k) = (pair[0].as_str().unwrap(), pair[1].as_u64().unwrap());
            let text = records
                .iter()
                .rev()
                .find(|r| r["key"] == key)
                .unwrap_or_else(|| panic!("{id}: key {key} is in its store"))["text"]
                .clone();
            assert_eq!(text, json!(format!("{id} note {k}")), "{id}: key {key}");
        }
    }
    let _ = std::fs::remove_dir_all(&root);
}
