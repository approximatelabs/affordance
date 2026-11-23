//! A close is answered promptly while the runtime publishes its status (every 100 ms) and its
//! agents keep working: the clock record is written off the control loop, so a slow disk delays
//! neither closing nor the workers (the slow-disk case itself is `host.rs`'s `status_writer_tests`).
use crate::shared;
use affordance::runtime::{AgentSpec, FnModel, GenReply, GenRequest, Runtime, RuntimeConfig};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs the test tokenizers: tools/fetch-test-tokenizers.sh")]
fn a_close_is_answered_while_status_is_published() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dir = std::env::temp_dir().join(format!("aff-close-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // Agents that keep working (a send to themselves every few ms), so workers take the status lock.
    std::fs::write(dir.join("app.tsx"), r#"export default function Main() {
  const [n, setN] = useState(0, "n");
  useEvent("tick", () => setN(x => x + 1));
  useEffect(() => { (async () => { for (;;) { await harness.wait({ms: 5}); await harness.send(harness.agent == "a" ? "b" : "a", {}, {event: "tick"}); } })(); }, []);
  return <text>{`n=${n}`}</text>;
}"#).unwrap();
    let enc = shared::tokenizer(&root.join("ref/tokenizers/Qwen__Qwen3-8B.json"));
    let mut cfg = RuntimeConfig::new(dir.join("app.tsx"));
    cfg.dir = Some(dir.join("run"));
    cfg.serve = true;
    cfg.max_wall = Some(Duration::from_secs(60));
    let mut rt = Runtime::new(cfg, enc, Arc::new(FnModel(|_: &GenRequest| Ok(GenReply::default()))));
    for id in ["a", "b"] {
        let mut s = AgentSpec::new(id, json!({}));
        s.model = None;
        rt.add_agent(s).unwrap();
    }
    let control = rt.control();
    let asked = Arc::new(std::sync::Mutex::new(None));
    let (asked2, closer_control) = (asked.clone(), control.clone());
    let closer = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(1500));
        let status = closer_control.status();
        *asked2.lock().unwrap() = Some(Instant::now());
        closer_control.close(Duration::ZERO);
        status
    });
    let report = rt.run().unwrap();
    let answered = asked.lock().unwrap().expect("closed").elapsed();
    let status = closer.join().unwrap();
    eprintln!("close answered in {} ms", answered.as_millis());
    assert!(!report.complete);
    assert_eq!(status["state"], "running", "the status was published: {status}");
    assert_eq!(control.status()["state"], "closed");
    assert!(answered < Duration::from_millis(1000), "closing took {answered:?}");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Closing does not wait behind a backlog of queued `send`s: a queued delivery the sender will
/// issue again is not applied while the run closes. Its sender gets no answer (not a
/// `delivered: false`), so the send stays outstanding in its journal and the restore delivers it,
/// once, to the receiver that had not journaled it.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs the test tokenizers: tools/fetch-test-tokenizers.sh")]
fn closing_skips_queued_sends_and_the_restore_delivers_them_once() {
    for batch in [1, 16] {
        close_behind_a_backlog(batch);
    }
}

fn close_behind_a_backlog(batch: usize) {
    const SENDERS: usize = 8;
    const EACH: usize = 64;
    let total = SENDERS * EACH;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dir = std::env::temp_dir().join(format!("aff-close-backlog-{}-{batch}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // Each sender sends EACH messages at once; the receiver spends a while on each.
    std::fs::write(
        dir.join("app.tsx"),
        format!(
            r#"export default function Main() {{
  const [n, setN] = useState(0, "n");
  useEvent("m", () => {{ let x = 0; for (let i = 0; i < 20000; i++) x += i; setN(k => k + 1); }});
  useEffect(() => {{ if (harness.agent == "r" && n >= {total}) harness.stop({{n}}); }}, [n]);
  useEffect(() => {{ if (harness.agent != "r") (async () => {{
    const sent = await Promise.all(Array.from({{length: {EACH}}}, (_, i) => harness.send("r", {{i}}, {{event: "m"}})));
    harness.stop({{delivered: sent.filter(s => s.delivered === true).length}});
  }})(); }}, []);
  return <text>{{`n=${{n}}`}}</text>;
}}"#
        ),
    )
    .unwrap();
    let enc = shared::tokenizer(&root.join("ref/tokenizers/Qwen__Qwen3-8B.json"));
    let config = || {
        let mut cfg = RuntimeConfig::new(dir.join("app.tsx"));
        cfg.dir = Some(dir.join("run"));
        cfg.workers = 2;
        cfg.input_batch = batch;
        cfg.max_wall = Some(Duration::from_secs(120));
        cfg
    };
    let model = || Arc::new(FnModel(|_: &GenRequest| Ok(GenReply::default())));
    let add = |rt: &mut Runtime| {
        let mut r = AgentSpec::new("r", json!({}));
        r.model = None;
        r.dedicated_worker = true;
        rt.add_agent(r).unwrap();
        for i in 0..SENDERS {
            let mut s = AgentSpec::new(&format!("s{i}"), json!({}));
            s.model = None;
            rt.add_agent(s).unwrap();
        }
    };
    let journal =
        |id: &str| std::fs::read_to_string(dir.join("run").join(id).join("journal.jsonl")).unwrap_or_default();
    let received = || {
        let text = journal("r");
        let mut ids: Vec<String> = text
            .match_indices("\"id\":\"s")
            .map(|(at, m)| {
                let rest = &text[at + m.len() - 1..];
                rest[..rest.find('"').unwrap()].to_string()
            })
            .collect();
        ids.sort();
        ids
    };
    // Close once the receiver has started on the backlog.
    let mut rt = Runtime::new(config(), enc.clone(), model());
    add(&mut rt);
    let control = rt.control();
    let asked = Arc::new(std::sync::Mutex::new(None));
    let (asked2, r_journal) = (asked.clone(), dir.join("run/r/journal.jsonl"));
    let closer = std::thread::spawn(move || {
        while !std::fs::read_to_string(&r_journal).is_ok_and(|t| t.contains("\"id\":\"s")) {
            std::thread::sleep(Duration::from_millis(2))
        }
        *asked2.lock().unwrap() = Some(Instant::now());
        control.close(Duration::from_secs(150));
    });
    let report = rt.run().unwrap();
    let answered = asked.lock().unwrap().expect("closed").elapsed();
    closer.join().unwrap();
    let before = received();
    eprintln!(
        "batch {batch}: close answered in {} ms with {} of {total} messages journaled",
        answered.as_millis(),
        before.len()
    );
    assert!(!report.complete);
    assert!(answered < Duration::from_millis(1500), "closing took {answered:?}");
    assert!(
        before.len() < total / 2,
        "the queued sends were applied: {} of {total}",
        before.len()
    );
    for i in 0..SENDERS {
        assert!(
            !journal(&format!("s{i}")).contains("\"delivered\":false"),
            "s{i} journaled a failed delivery"
        );
    }
    // Restore: every send the receiver had not journaled is still outstanding and is issued again.
    let rt = Runtime::restore(config(), enc.clone(), model()).unwrap();
    let report = rt.run().unwrap();
    assert!(report.complete, "every agent finished");
    let after = received();
    let mut unique = after.clone();
    unique.dedup();
    assert_eq!(unique.len(), total, "every message reached the receiver");
    assert_eq!(after.len(), total, "each message was journaled once");
    for agent in &report.agents {
        if agent.id == "r" {
            assert_eq!(agent.stopped, Some(json!({"n": total})));
        } else {
            assert_eq!(agent.stopped, Some(json!({"delivered": EACH})), "{}", agent.id);
        }
    }
    for i in 0..SENDERS {
        assert!(!journal(&format!("s{i}")).contains("\"delivered\":false"), "s{i}");
    }
    std::fs::remove_dir_all(&dir).unwrap();
}
