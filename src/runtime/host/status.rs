//! The run's clock record, and a journal's last clock read from its tail.
use super::*;

/// Writes the run's clock (`clock.json`, private recovery state: where the clock stood, so a crash
/// during a long quiet wait does not give the wait its time again) on its own thread, the latest
/// value winning: a writer slowed by its disk skips values instead of queueing them, and neither
/// the control loop nor the workers wait for it. It is synced; the directory is synced by the
/// shutdown's own writes.
pub(super) struct StatusWriter {
    slot: Arc<Pending>,
    thread: Option<std::thread::JoinHandle<()>>,
}

/// The snapshot not yet written, and (under `open`'s lock) whether renames may still land.
#[derive(Default)]
pub(super) struct Pending {
    next: Mutex<(Option<J>, bool)>,
    wake: std::sync::Condvar,
    open: Mutex<bool>,
}

impl Pending {
    /// Run `rename` unless the writer was finished: a write still syncing when the run ends never
    /// replaces what the shutdown wrote after it.
    pub(super) fn commit(&self, rename: impl FnOnce()) {
        if *self.open.lock().unwrap() {
            rename()
        }
    }
}

impl StatusWriter {
    /// The status shows only committed work: each write waits for the commit of every journal
    /// line written before it.
    pub(super) fn start(dir: Option<PathBuf>, durable: Arc<Durable>) -> StatusWriter {
        StatusWriter::start_with(dir.map(|dir| {
            move |clock: J, p: &Pending| {
                if durable.wait(durable.written()).is_err() {
                    return;
                }
                // Its own temporary name: the shutdown writes `clock.tmp` while this may still run.
                let temp = dir.join(".clock.status");
                let written = std::fs::File::create(&temp).and_then(|mut f| {
                    f.write_all(clock.to_string().as_bytes())?;
                    f.sync_all()
                });
                if written.is_ok() {
                    p.commit(|| {
                        let _ = std::fs::rename(&temp, dir.join("clock.json"));
                    });
                }
            }
        }))
    }
    pub(super) fn start_with(write: Option<impl Fn(J, &Pending) + Send + 'static>) -> StatusWriter {
        let slot = Arc::new(Pending {
            open: Mutex::new(true),
            ..Default::default()
        });
        let thread = write.map(|write| {
            let slot = slot.clone();
            std::thread::Builder::new()
                .name("status".into())
                .spawn(move || {
                    loop {
                        let next = {
                            let mut g = slot.next.lock().unwrap();
                            while g.0.is_none() && !g.1 {
                                g = slot.wake.wait(g).unwrap();
                            }
                            if g.1 {
                                return;
                            }
                            g.0.take().unwrap()
                        };
                        write(next, &slot);
                    }
                })
                .expect("spawn status thread")
        });
        StatusWriter { slot, thread }
    }
    pub(super) fn put(&self, clock: J) {
        self.slot.next.lock().unwrap().0 = Some(clock);
        self.slot.wake.notify_one();
    }
    /// Stop: no rename lands after this returns. A write in progress is not waited for (its
    /// disk may be slow); its thread ends when the write does.
    pub(super) fn finish(mut self) {
        *self.slot.open.lock().unwrap() = false;
        self.slot.next.lock().unwrap().1 = true;
        self.slot.wake.notify_one();
        drop(self.thread.take());
    }
}

/// The clock of a journal's last record (None: no journal, or no record). Read from the tail
/// when the journal ends in a whole record; otherwise (a torn or unterminated tail) the
/// recovering reader runs over the journal as before. The prefix is validated by the agent's
/// own restore (`restore_instance`), on its worker, not here on the one restore thread.
pub(super) fn journal_last_clock(file: &std::path::Path) -> Result<Option<i64>, String> {
    if let Some(clock) = tail_clock(file) {
        return Ok(Some(clock));
    }
    let mut last_clock = None;
    if let Some(reader) = super::super::jsonl::JsonLines::<Input>::open(file, "journal")? {
        for row in reader {
            last_clock = Some(row?.value.clock_ms());
        }
    }
    Ok(last_clock)
}

/// The clock of the last record of a journal that ends with a newline, or None.
pub(super) fn tail_clock(file: &std::path::Path) -> Option<i64> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(file).ok()?;
    let len = f.metadata().ok()?.len();
    let mut tail: Vec<u8> = vec![];
    let mut start = len;
    let mut block = 64u64 << 10;
    loop {
        let from = start.saturating_sub(block);
        let mut chunk = vec![0u8; (start - from) as usize];
        f.seek(SeekFrom::Start(from)).ok()?;
        f.read_exact(&mut chunk).ok()?;
        chunk.extend_from_slice(&tail);
        tail = chunk;
        start = from;
        if tail.last() != Some(&b'\n') {
            return None;
        }
        let body = &tail[..tail.len() - 1];
        // The last line starts after the previous newline (or at the file's start).
        if let Some(i) = body.iter().rposition(|&b| b == b'\n') {
            return line_clock(&body[i + 1..]);
        }
        if start == 0 {
            return line_clock(body);
        }
        block = block.saturating_mul(2);
    }
}

pub(super) fn line_clock(line: &[u8]) -> Option<i64> {
    let line = line.strip_suffix(b"\r").unwrap_or(line);
    serde_json::from_slice::<Input>(line).ok().map(|input| input.clock_ms())
}

#[cfg(test)]
mod journal_clock_tests {
    use super::*;

    pub(super) fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("aff-journal-clock-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }
    pub(super) fn tick(ms: i64) -> String {
        format!("{{\"input\":\"tick\",\"clock_ms\":{ms}}}\n")
    }

    /// Resume needs only each journal's last clock: it reads the journal's tail, not every
    /// record (parsing 1,219 journals of ~3.9 MB in full, one after another, once delayed every
    /// agent's restore). A damaged prefix is not read here; the
    /// agent's own restore validates its whole journal.
    #[test]
    pub(super) fn the_last_clock_comes_from_the_tail_alone() {
        let d = dir("tail");
        let file = d.join("journal.jsonl");
        let mut text = String::from("{\"not\":\"an input\"}\n");
        for ms in [5, 9, 12] {
            text.push_str(&tick(ms));
        }
        std::fs::write(&file, &text).unwrap();
        assert_eq!(journal_last_clock(&file).unwrap(), Some(12));
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            text,
            "the journal is unchanged"
        );
        // A last record longer than one read block is still found whole.
        let long = format!(
            "{{\"input\":\"event\",\"name\":\"m\",\"payload\":{{\"text\":\"{}\"}},\"clock_ms\":44}}\n",
            "x".repeat(200_000)
        );
        std::fs::write(&file, format!("{text}{long}")).unwrap();
        assert_eq!(journal_last_clock(&file).unwrap(), Some(44));
        let _ = std::fs::remove_dir_all(&d);
    }

    /// A torn tail is not trusted: the full reader recovers it as before (the torn bytes are
    /// saved aside and the journal ends at its last whole record).
    #[test]
    pub(super) fn a_torn_tail_falls_back_to_the_recovering_reader() {
        let d = dir("torn");
        let file = d.join("journal.jsonl");
        std::fs::write(&file, format!("{}{}{{\"input\":\"tick\",\"clo", tick(5), tick(9))).unwrap();
        assert_eq!(journal_last_clock(&file).unwrap(), Some(9));
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            format!("{}{}", tick(5), tick(9))
        );
        assert!(
            std::fs::read_dir(&d)
                .unwrap()
                .flatten()
                .any(|e| e.file_name().to_string_lossy().starts_with("torn-journal-"))
        );
        // An empty or missing journal has no clock.
        std::fs::write(&file, "").unwrap();
        assert_eq!(journal_last_clock(&file).unwrap(), None);
        assert_eq!(journal_last_clock(&d.join("missing.jsonl")).unwrap(), None);
        let _ = std::fs::remove_dir_all(&d);
    }
}

#[cfg(test)]
mod status_writer_tests {
    use super::*;

    /// A disk that takes 500 ms a write: the control loop's `put`s do not wait for it, snapshots
    /// it could not keep up with are skipped (the latest is written), and `finish` neither waits
    /// for a write in progress nor lets it land afterwards.
    #[test]
    pub(super) fn a_slow_disk_delays_no_put_and_the_latest_snapshot_wins() {
        let written: Arc<Mutex<Vec<J>>> = Default::default();
        let w = written.clone();
        let writer = StatusWriter::start_with(Some(move |clock: J, p: &Pending| {
            std::thread::sleep(Duration::from_millis(500));
            p.commit(|| w.lock().unwrap().push(clock));
        }));
        // Only the time inside `put` counts: how long a sleep really takes is the machine's.
        let mut putting = Duration::ZERO;
        for i in 0..20 {
            let started = Instant::now();
            writer.put(json!(i));
            putting += started.elapsed();
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(
            putting < Duration::from_millis(100),
            "puts waited on the disk: {putting:?}"
        );
        // The writer is at most two writes from the latest: the one in progress and the next.
        // Waited for, not slept for: when a thread next runs is the machine's.
        let deadline = Instant::now() + Duration::from_secs(20);
        while written.lock().unwrap().last() != Some(&json!(19)) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(
            written.lock().unwrap().last(),
            Some(&json!(19)),
            "the latest snapshot is written"
        );
        assert!(written.lock().unwrap().len() <= 3, "{:?}", written.lock().unwrap());
        writer.put(json!("late"));
        std::thread::sleep(Duration::from_millis(50));
        let finishing = Instant::now();
        writer.finish();
        assert!(
            finishing.elapsed() < Duration::from_millis(100),
            "finish waited on the disk"
        );
        std::thread::sleep(Duration::from_millis(700));
        assert_eq!(
            written.lock().unwrap().last(),
            Some(&json!(19)),
            "a write in progress at finish never lands"
        );
    }
}
