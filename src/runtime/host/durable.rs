//! Group commit: journal lines are written when their inputs are applied and made durable
//! together, by a barrier.
//!
//! A barrier closes the open epoch, syncs every journal written in it (in parallel, so their
//! syncs share the file system's commits), and then appends the cut, each such journal's
//! length, to `<dir>/commit.jsonl`. Stores (`store.jsonl`) are cut the same way, and blob files
//! are synced in the commit before the record covering the first line that refers to them. Only then is the epoch committed. Recovery truncates every
//! journal to the last committed cut. A crash therefore leaves the journals in a state the run
//! passed through. A line survives only together with every line written before it in any
//! journal, and so with everything that caused it.
//!
//! What waits for a commit is what leaves the process: model requests, machine calls, store
//! operations, a fresh mount's archiving of an old directory, the answer to an embedder's post and
//! the clock record. A delivery to another instance does not wait. Its line in the receiver's journal is
//! written after the sender's line that caused it, so no cut has the delivery without its cause,
//! and none has the sender's acknowledgement without the delivery.
use super::super::blobs::Unsynced;
use super::*;
use std::fs::File;
use std::path::Path;
use std::sync::{Condvar, Weak};

/// How long a barrier waits after the first line it will cover for more lines to join, unless
/// something waits on it (an effect, a post's answer, a clock write, a drained worker).
const GROUP: Duration = Duration::from_millis(2);
/// The commit log is rewritten as one complete cut when it grows past this.
const COMPACT_BYTES: u64 = 4 << 20;
/// Threads a barrier syncs journals on.
const PARALLEL: usize = 32;
pub(super) const LOG: &str = "commit.jsonl";

/// Each file's place in a cut, by instance id: the incarnation that writes it, and its bytes.
pub(super) type Files = BTreeMap<String, (u64, u64)>;

/// A committed state of the run's append-only files: every instance's journal and store.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct Cut {
    pub journals: Files,
    pub stores: Files,
}

/// The append-only files a cut covers, and their names in an instance's directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Kind {
    Journal,
    Store,
}
impl Kind {
    fn file(self) -> &'static str {
        match self {
            Kind::Journal => "journal.jsonl",
            Kind::Store => "store.jsonl",
        }
    }
    fn key(self) -> &'static str {
        match self {
            Kind::Journal => "journals",
            Kind::Store => "stores",
        }
    }
}
impl Cut {
    fn of(&self, kind: Kind) -> &Files {
        match kind {
            Kind::Journal => &self.journals,
            Kind::Store => &self.stores,
        }
    }
    fn of_mut(&mut self, kind: Kind) -> &mut Files {
        match kind {
            Kind::Journal => &mut self.journals,
            Kind::Store => &mut self.stores,
        }
    }
}

/// An instance's journal. Only its worker appends to it.
pub(super) struct Journal {
    file: Arc<File>,
    id: String,
    incarnation: u64,
    len: u64,
}

impl Journal {
    pub(super) fn new(file: File, id: &str, incarnation: u64) -> Result<Journal, String> {
        let len = file.metadata().map_err(|e| e.to_string())?.len();
        Ok(Journal {
            file: Arc::new(file),
            id: id.into(),
            incarnation,
            len,
        })
    }
}

/// A file written in the open epoch: a journal (its open handle) or a store (by path).
struct Entry {
    file: Result<Arc<File>, PathBuf>,
    incarnation: u64,
    len: u64,
    dir: Option<PathBuf>,
}

#[derive(Default)]
struct State {
    /// The epoch new lines join.
    open: u64,
    /// The epoch of the latest line written.
    written: u64,
    /// Every line of this epoch or earlier is durable.
    committed: u64,
    dirty: HashMap<(Kind, String), Entry>,
    since: Option<Instant>,
    requested: bool,
    stop: bool,
    /// At the stop, record that every line written is committed.
    close: bool,
    failed: Option<String>,
    /// Instances with effects waiting for a commit, by worker.
    waiting: BTreeMap<usize, BTreeSet<String>>,
}

impl State {
    fn put(&mut self, kind: Kind, id: &str, entry: Entry) {
        match self.dirty.get_mut(&(kind, id.to_string())) {
            Some(old) if old.incarnation == entry.incarnation => {
                old.len = entry.len;
                old.file = entry.file;
                if entry.dir.is_some() {
                    old.dir = entry.dir;
                }
            }
            // A remount under the same id: the old incarnation's file is synced now, since the
            // cut will name only the new one.
            Some(old) => {
                let _ = sync_one(old);
                *old = entry;
            }
            None => {
                self.dirty.insert((kind, id.to_string()), entry);
            }
        }
    }
}

pub(super) enum Ready {
    Now,
    Later,
    Failed(String),
}

#[derive(Default)]
pub(super) struct Durable {
    state: Mutex<State>,
    work: std::sync::Condvar,
    done: Condvar,
    failed: AtomicBool,
    thread: Mutex<Option<std::thread::JoinHandle<()>>>,
    unsynced: Mutex<Option<Arc<Unsynced>>>,
}

impl Durable {
    pub(super) fn new() -> Durable {
        Durable {
            state: Mutex::new(State {
                open: 1,
                ..State::default()
            }),
            ..Durable::default()
        }
    }

    /// Write a line. Returns the epoch whose commit makes it durable.
    pub(super) fn append(&self, journal: &mut Journal, line: &[u8]) -> Result<u64, String> {
        if let Some(error) = self.failed() {
            return Err(error);
        }
        (&*journal.file).write_all(line).map_err(|e| e.to_string())?;
        journal.len += line.len() as u64;
        Ok(self.mark(journal, None))
    }

    /// A journal created by this run: the next commit records it (and syncs its directory).
    pub(super) fn register(&self, journal: &Journal, dir: &Path) {
        self.mark(journal, Some(dir.to_path_buf()));
    }

    fn mark(&self, journal: &Journal, dir: Option<PathBuf>) -> u64 {
        let line = dir.is_none();
        let mut st = self.state.lock().unwrap();
        let e = st.open;
        st.put(
            Kind::Journal,
            &journal.id,
            Entry {
                file: Ok(journal.file.clone()),
                incarnation: journal.incarnation,
                len: journal.len,
                dir,
            },
        );
        // A registration is no line: nothing waits on it.
        if line {
            st.written = e;
        }
        if st.since.is_none() {
            st.since = Some(Instant::now());
            self.work.notify_one();
        }
        e
    }

    /// An instance's store file (`path`) now ends at `len`: the next commit syncs it and records
    /// the length, which recovery truncates the store to. `created`: its directory entry is new.
    pub(super) fn stored(&self, id: &str, incarnation: u64, path: &Path, len: u64, created: bool) {
        let mut st = self.state.lock().unwrap();
        let dir = created.then(|| path.parent().unwrap().to_path_buf());
        st.put(
            Kind::Store,
            id,
            Entry {
                file: Err(path.to_path_buf()),
                incarnation,
                len,
                dir,
            },
        );
        if st.since.is_none() {
            st.since = Some(Instant::now());
            self.work.notify_one();
        }
    }

    /// The epoch of the latest line written anywhere: its commit covers everything before now.
    pub(super) fn written(&self) -> u64 {
        self.state.lock().unwrap().written
    }

    pub(super) fn failed(&self) -> Option<String> {
        if !self.failed.load(Ordering::Acquire) {
            return None;
        }
        self.state.lock().unwrap().failed.clone()
    }

    /// Whether epoch `e` is committed. If not yet, instance `id` on worker `w` is woken
    /// (`Msg::Durable`) when it is, and a barrier starts at once.
    pub(super) fn watch(&self, w: usize, id: &str, e: u64) -> Ready {
        let mut st = self.state.lock().unwrap();
        if let Some(error) = &st.failed {
            return Ready::Failed(error.clone());
        }
        if st.committed >= e {
            return Ready::Now;
        }
        st.waiting.entry(w).or_default().insert(id.to_string());
        st.requested = true;
        self.work.notify_one();
        Ready::Later
    }

    /// Wait until epoch `e` is committed.
    pub(super) fn wait(&self, e: u64) -> Result<(), String> {
        let mut st = self.state.lock().unwrap();
        loop {
            if let Some(error) = &st.failed {
                return Err(error.clone());
            }
            if st.committed >= e {
                return Ok(());
            }
            if self.thread.lock().unwrap().is_none() {
                return Err("the journal barrier is not running".into());
            }
            st.requested = true;
            self.work.notify_one();
            st = self.done.wait(st).unwrap();
        }
    }

    /// Sync the blob and store files written so far, on this thread.
    pub(super) fn sync_files(&self) -> Result<(), String> {
        let unsynced = self.unsynced.lock().unwrap().clone();
        unsynced.map_or(Ok(()), |u| {
            u.sync_now().map_err(|e| format!("syncing blob and store files: {e}"))
        })
    }

    /// A worker has nothing queued: commit what it wrote now rather than after `GROUP`.
    pub(super) fn request(&self) {
        let mut st = self.state.lock().unwrap();
        if !st.dirty.is_empty() && !st.requested {
            st.requested = true;
            self.work.notify_one();
        }
    }

    /// Start the barrier thread over the run directory, whose journals are the recovered `cut`.
    pub(super) fn start(
        self: &Arc<Self>,
        dir: &Path,
        cut: Cut,
        unsynced: Arc<Unsynced>,
        shared: Weak<Shared>,
    ) -> Result<(), String> {
        *self.unsynced.lock().unwrap() = Some(unsynced.clone());
        let mut log = Log {
            path: dir.join(LOG),
            dir: dir.to_path_buf(),
            file: None,
            bytes: 0,
            cut,
            unsynced,
        };
        log.snapshot(0).map_err(|e| format!("{}: {e}", log.path.display()))?;
        let me = self.clone();
        let thread = std::thread::Builder::new()
            .name("journal-commit".into())
            .spawn(move || me.run(log, shared))
            .map_err(|e| e.to_string())?;
        *self.thread.lock().unwrap() = Some(thread);
        Ok(())
    }

    /// Commit everything written, then stop the barrier thread. With `close` (no line can be
    /// written after this), the log records that the journals are complete as they stand: a
    /// restore after a clean shutdown truncates nothing, so the journals stay authoritative.
    pub(super) fn finish(&self, close: bool) -> Result<(), String> {
        {
            let mut st = self.state.lock().unwrap();
            st.stop = true;
            st.close |= close;
            st.requested = true;
            self.work.notify_one();
        }
        let thread = self.thread.lock().unwrap().take();
        if let Some(thread) = thread {
            thread.join().map_err(|_| "the journal barrier panicked".to_string())?;
        }
        self.failed().map_or(Ok(()), Err)
    }

    fn run(&self, mut log: Log, shared: Weak<Shared>) {
        loop {
            let (epoch, batch) = {
                let mut st = self.state.lock().unwrap();
                loop {
                    if !st.dirty.is_empty() {
                        if st.requested || st.stop {
                            break;
                        }
                        let since = st.since.map_or(Duration::ZERO, |s| s.elapsed());
                        if since >= GROUP {
                            break;
                        }
                        st = self.work.wait_timeout(st, GROUP - since).unwrap().0;
                    } else if st.stop {
                        if st.close
                            && st.failed.is_none()
                            && let Err(error) = log.close()
                        {
                            eprintln!("affordance: journal commit failed: {error}");
                            st.failed = Some(format!("journal durability failed: {error}"));
                            self.failed.store(true, Ordering::Release);
                        }
                        return;
                    } else {
                        st = self.work.wait(st).unwrap();
                    }
                }
                let e = st.open;
                st.open += 1;
                st.requested = false;
                st.since = None;
                (e, std::mem::take(&mut st.dirty))
            };
            let result = log.commit(epoch, batch);
            let failed = result.is_err();
            let waiting = {
                let mut st = self.state.lock().unwrap();
                match result {
                    Ok(()) => st.committed = epoch,
                    Err(error) => {
                        eprintln!("affordance: journal commit failed: {error}");
                        st.failed = Some(format!("journal durability failed: {error}"));
                        self.failed.store(true, Ordering::Release);
                    }
                }
                self.done.notify_all();
                std::mem::take(&mut st.waiting)
            };
            if let Some(shared) = shared.upgrade() {
                for (w, ids) in waiting {
                    shared.post(w, Msg::Durable(ids.into_iter().collect()));
                }
            }
            if failed {
                return;
            }
        }
    }
}

/// The commit log: one line per barrier with the journals it covered, after a first line with
/// the complete cut.
struct Log {
    path: PathBuf,
    dir: PathBuf,
    file: Option<File>,
    bytes: u64,
    cut: Cut,
    unsynced: Arc<Unsynced>,
}

fn record(epoch: u64, cut: &Cut) -> String {
    format!(
        "{}\n",
        json!({"epoch": epoch, "journals": cut.journals, "stores": cut.stores})
    )
}

impl Log {
    /// Replace the log with one line: the complete cut.
    fn snapshot(&mut self, epoch: u64) -> std::io::Result<()> {
        let line = record(epoch, &self.cut);
        let temp = self.path.with_extension("tmp");
        let mut file = File::create(&temp)?;
        file.write_all(line.as_bytes())?;
        file.sync_all()?;
        std::fs::rename(&temp, &self.path)?;
        File::open(&self.dir)?.sync_all()?;
        self.file = Some(std::fs::OpenOptions::new().append(true).open(&self.path)?);
        self.bytes = line.len() as u64;
        Ok(())
    }

    fn close(&mut self) -> Result<(), String> {
        self.unsynced
            .sync_now()
            .map_err(|e| format!("syncing blob and store files: {e}"))?;
        let file = self.file.as_mut().expect("the commit log is open");
        file.write_all(b"{\"closed\":true}\n")
            .and_then(|_| file.sync_data())
            .map_err(|e| format!("{}: {e}", self.path.display()))
    }

    fn commit(&mut self, epoch: u64, batch: HashMap<(Kind, String), Entry>) -> Result<(), String> {
        // Every blob a line in this epoch refers to was added before the line was written, so
        // before the epoch closed: it is in this set, synced with the journals and stores,
        // before the record that commits them.
        let (files, dirs) = self.unsynced.take();
        let mut jobs: Vec<Sync> = batch.values().map(Sync::Journal).collect();
        jobs.extend(files.iter().map(|p| Sync::Path(p)));
        sync(&jobs).map_err(|e| e.to_string())?;
        dirs.iter()
            .try_for_each(|d| super::super::blobs::sync_path(d))
            .map_err(|e| e.to_string())?;
        let mut changes = Cut::default();
        for ((kind, id), e) in &batch {
            changes.of_mut(*kind).insert(id.clone(), (e.incarnation, e.len));
            self.cut.of_mut(*kind).insert(id.clone(), (e.incarnation, e.len));
        }
        let written = if self.bytes > COMPACT_BYTES {
            self.snapshot(epoch)
        } else {
            let line = record(epoch, &changes);
            let file = self.file.as_mut().expect("the commit log is open");
            let done = file.write_all(line.as_bytes()).and_then(|_| file.sync_data());
            self.bytes += line.len() as u64;
            done
        };
        written.map_err(|e| format!("{}: {e}", self.path.display()))
    }
}

fn sync_one(e: &Entry) -> std::io::Result<()> {
    match &e.file {
        Ok(file) => file.sync_data()?,
        Err(path) => File::open(path)?.sync_data()?,
    }
    // A journal this run created: its directory entry, and its directory's, are made durable too.
    if let Some(dir) = &e.dir {
        File::open(dir)?.sync_all()?;
        if let Some(parent) = dir.parent() {
            File::open(parent)?.sync_all()?;
        }
    }
    Ok(())
}

enum Sync<'a> {
    Journal(&'a Entry),
    Path(&'a Path),
}
impl Sync<'_> {
    fn run(&self) -> std::io::Result<()> {
        match self {
            Sync::Journal(e) => sync_one(e),
            Sync::Path(p) => super::super::blobs::sync_path(p),
        }
    }
}

fn sync(jobs: &[Sync]) -> std::io::Result<()> {
    if jobs.len() <= 1 {
        return jobs.iter().try_for_each(Sync::run);
    }
    let chunk = jobs.len().div_ceil(PARALLEL);
    std::thread::scope(|s| {
        let handles: Vec<_> = jobs
            .chunks(chunk)
            .map(|part| s.spawn(move || part.iter().try_for_each(Sync::run)))
            .collect();
        handles.into_iter().try_for_each(|h| {
            h.join()
                .unwrap_or_else(|_| Err(std::io::Error::other("a journal sync panicked")))
        })
    })
}

/// The last committed cut in a commit log (`None`: there is no log, or the run that wrote it
/// shut down with everything committed). Only the final line may be torn: it is the barrier a
/// crash interrupted, whose cut was never committed. A bad line before it is damage, not an
/// older cut, and is refused (as `jsonl.rs` refuses a bad journal line).
fn read_log(path: &Path) -> Result<Option<Cut>, String> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    let mut cut = Cut::default();
    let mut lines = bytes.split_inclusive(|&b| b == b'\n').peekable();
    let mut at = 0;
    while let Some(line) = lines.next() {
        let Some(value) = serde_json::from_slice::<J>(line)
            .ok()
            .filter(|v| v.is_object() && line.ends_with(b"\n"))
        else {
            if lines.peek().is_none() {
                break;
            }
            return Err(format!(
                "{}: byte {at}: not a commit record, and lines follow it; refusing to cut the journals",
                path.display()
            ));
        };
        if value["closed"] == true {
            return Ok(None);
        }
        for kind in [Kind::Journal, Kind::Store] {
            for (id, file) in value[kind.key()].as_object().into_iter().flatten() {
                let (Some(incarnation), Some(len)) = (file[0].as_u64(), file[1].as_u64()) else {
                    return Err(format!("{}: byte {at}: invalid entry for {id}", path.display()));
                };
                cut.of_mut(kind).insert(id.clone(), (incarnation, len));
            }
        }
        at += line.len();
    }
    Ok(Some(cut))
}

/// The lock on a run directory, held until dropped. Recovery truncates the journals and the
/// barrier replaces the commit log, so only one runtime may run over a directory: a second is
/// refused, not queued (a Web Lock taken with `ifAvailable`). The kernel drops the lock with its
/// process, so a crash leaves nothing to clear.
pub(super) struct Owner(File);

/// Own the run directory: an exclusive lock on `<dir>/runtime.lock`.
pub(super) fn own(dir: &Path) -> Result<Owner, String> {
    let path = dir.join("runtime.lock");
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    match file.try_lock() {
        Ok(()) => Ok(Owner(file)),
        Err(std::fs::TryLockError::WouldBlock) => Err(format!(
            "{}: the run directory is in use by another runtime",
            dir.display()
        )),
        Err(std::fs::TryLockError::Error(e)) => Err(format!("{}: {e}", path.display())),
    }
}

/// Unlocked, not just closed: a process forked meanwhile (by any thread) holds a copy of the
/// descriptor until it execs, and the lock lasts until its last copy closes.
impl Drop for Owner {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}

/// Every instance directory under the run directory (children inside their parents'), by id.
fn instances(dir: &Path) -> Vec<(String, PathBuf)> {
    let mut found = vec![];
    let mut stack = vec![(String::new(), dir.to_path_buf())];
    while let Some((prefix, at)) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&at) else { continue };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') || !entry.file_type().is_ok_and(|t| t.is_dir()) {
                continue;
            }
            if prefix.is_empty() && (name == "blobs" || name == "control") {
                continue;
            }
            let path = entry.path();
            if !path.join("agent.json").exists() {
                continue;
            }
            let id = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            found.push((id.clone(), path.clone()));
            stack.push((id, path));
        }
    }
    found.sort();
    found
}

/// Truncate every journal and store under `dir` to the last committed cut and return the cut
/// the run starts from. A file the cut does not name (or names for another incarnation) was
/// created after the last commit: it is emptied. Bytes past the cut are kept aside in
/// `uncommitted-journal-<offset>.bin` (or `-store-`). Without a commit log, nothing is truncated.
/// An instance whose `agent.json` cannot be read is left as it is, and keeps its place in the cut:
/// its own restore fails, not the run.
pub(super) fn recover(dir: &Path) -> Result<Cut, String> {
    let committed = read_log(&dir.join(LOG))?;
    let mut cut = Cut::default();
    for (id, path) in instances(dir) {
        let saved = std::fs::read(path.join("agent.json"))
            .map_err(|e| e.to_string())
            .and_then(|b| serde_json::from_slice::<J>(&b).map_err(|e| e.to_string()));
        let incarnation = match saved {
            Ok(v) => v["incarnation"].as_u64().unwrap_or(0),
            Err(e) => {
                eprintln!("{id}: agent.json is unreadable ({e}); its journal and store are left as they are");
                for kind in [Kind::Journal, Kind::Store] {
                    if let Some(&at) = committed.as_ref().and_then(|c| c.of(kind).get(&id)) {
                        cut.of_mut(kind).insert(id.clone(), at);
                    }
                }
                continue;
            }
        };
        for kind in [Kind::Journal, Kind::Store] {
            let file = path.join(kind.file());
            let Ok(meta) = std::fs::metadata(&file) else { continue };
            let len = meta.len();
            let keep = match &committed {
                None => len,
                Some(c) => match c.of(kind).get(&id) {
                    Some(&(i, n)) if i == incarnation => n,
                    _ => 0,
                },
            };
            if keep < len {
                let saved = set_aside(&file, keep).map_err(|e| format!("{}: {e}", file.display()))?;
                eprintln!(
                    "{id}: {} cut to its last commit, {len} -> {keep} bytes (the rest is in {saved})",
                    kind.file()
                );
            } else if keep > len {
                eprintln!(
                    "{id}: {} ({len} bytes) is shorter than its last commit ({keep} bytes)",
                    kind.file()
                );
            }
            cut.of_mut(kind).insert(id.clone(), (incarnation, keep.min(len)));
        }
    }
    Ok(cut)
}

/// Move a file's bytes past `keep` into a new file beside it, and return that file's name.
fn set_aside(journal: &Path, keep: u64) -> std::io::Result<String> {
    let kind = journal.file_stem().and_then(|s| s.to_str()).unwrap_or("journal");
    use std::io::{Read, Seek, SeekFrom};
    let parent = journal.parent().unwrap();
    let mut file = std::fs::OpenOptions::new().read(true).write(true).open(journal)?;
    let mut tail = vec![];
    file.seek(SeekFrom::Start(keep))?;
    file.read_to_end(&mut tail)?;
    let mut number = 0;
    let (mut saved, name) = loop {
        let name = if number == 0 {
            format!("uncommitted-{kind}-{keep}.bin")
        } else {
            format!("uncommitted-{kind}-{keep}-{number}.bin")
        };
        match std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(parent.join(&name))
        {
            Ok(file) => break (file, name),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => number += 1,
            Err(e) => return Err(e),
        }
    };
    saved.write_all(&tail)?;
    saved.sync_all()?;
    File::open(parent)?.sync_all()?;
    file.set_len(keep)?;
    file.sync_all()?;
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("aff-durable-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }
    fn instance(root: &Path, id: &str, incarnation: u64, journal: &str) {
        let d = root.join(id);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(
            d.join("agent.json"),
            json!({"id": id, "incarnation": incarnation}).to_string(),
        )
        .unwrap();
        std::fs::write(d.join("journal.jsonl"), journal).unwrap();
    }

    /// Recovery cuts every journal to the last whole commit line: past it, lines are set aside;
    /// a journal of another incarnation (a remount after the commit) or one the cut does not
    /// name (created after it) is emptied; children are found inside their parents.
    #[test]
    fn recovery_truncates_every_journal_to_the_last_committed_cut() {
        let d = dir("cut");
        instance(&d, "a", 7, "1\n2\n3\n");
        instance(&d, "a/kid", 8, "k1\nk2\n");
        instance(&d, "b", 9, "x\n");
        instance(&d, "c", 3, "new\n");
        std::fs::write(d.join("a/store.jsonl"), "s1\ns2\n").unwrap();
        std::fs::write(
            d.join(LOG),
            concat!(
                "{\"epoch\":0,\"journals\":{\"a\":[7,2],\"b\":[5,2]}}\n",
                "{\"epoch\":1,\"journals\":{\"a\":[7,4],\"a/kid\":[8,3]},\"stores\":{\"a\":[7,3]}}\n",
                "{\"epoch\":2,\"journals\":{\"a\":[7,6]"
            ),
        )
        .unwrap();
        let cut = recover(&d).unwrap();
        assert_eq!(
            cut.journals,
            Files::from([
                ("a".into(), (7, 4)),
                ("a/kid".into(), (8, 3)),
                ("b".into(), (9, 0)),
                ("c".into(), (3, 0))
            ])
        );
        assert_eq!(cut.stores, Files::from([("a".into(), (7, 3))]));
        assert_eq!(std::fs::read_to_string(d.join("a/store.jsonl")).unwrap(), "s1\n");
        assert_eq!(
            std::fs::read_to_string(d.join("a/uncommitted-store-3.bin")).unwrap(),
            "s2\n"
        );
        assert_eq!(std::fs::read_to_string(d.join("a/journal.jsonl")).unwrap(), "1\n2\n");
        assert_eq!(
            std::fs::read_to_string(d.join("a/uncommitted-journal-4.bin")).unwrap(),
            "3\n"
        );
        assert_eq!(std::fs::read_to_string(d.join("a/kid/journal.jsonl")).unwrap(), "k1\n");
        assert_eq!(std::fs::read_to_string(d.join("b/journal.jsonl")).unwrap(), "");
        assert_eq!(std::fs::read_to_string(d.join("c/journal.jsonl")).unwrap(), "");
        // Recovering again changes nothing; without a log nothing is cut.
        assert_eq!(recover(&d).unwrap(), cut);
        std::fs::remove_file(d.join(LOG)).unwrap();
        std::fs::write(d.join("a/journal.jsonl"), "1\n2\n3\n").unwrap();
        assert_eq!(recover(&d).unwrap().journals["a"], (7, 6));
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Only the last line of the log may be torn. A bad line with lines after it is damage, not
    /// an older cut: recovery refuses it and truncates nothing (reading it as a cut emptied `b`
    /// and cut `a` back to its first commit, silently).
    #[test]
    fn a_corrupt_line_before_the_last_is_refused_not_read_as_a_cut() {
        let d = dir("corrupt");
        instance(&d, "a", 7, "1\n2\n3\n");
        instance(&d, "b", 9, "x\ny\n");
        let first = "{\"epoch\":0,\"journals\":{\"a\":[7,2]}}\n";
        std::fs::write(
            d.join(LOG),
            format!("{first}{{\"epoch\":1,\"jour\n{{\"epoch\":2,\"journals\":{{\"a\":[7,6],\"b\":[9,4]}}}}\n"),
        )
        .unwrap();
        let error = recover(&d).unwrap_err();
        assert!(error.contains(&format!("byte {}", first.len())), "{error}");
        assert_eq!(std::fs::read_to_string(d.join("a/journal.jsonl")).unwrap(), "1\n2\n3\n");
        assert_eq!(std::fs::read_to_string(d.join("b/journal.jsonl")).unwrap(), "x\ny\n");
        // A record of the wrong shape is refused the same way.
        std::fs::write(d.join(LOG), format!("{first}[1]\n{first}")).unwrap();
        assert!(recover(&d).is_err());
        // The same bad bytes as the last line are a torn barrier: the cut before them holds.
        std::fs::write(d.join(LOG), format!("{first}{{\"epoch\":1,\"jour\n")).unwrap();
        assert_eq!(
            recover(&d).unwrap().journals,
            Files::from([("a".into(), (7, 2)), ("b".into(), (9, 0))])
        );
        assert_eq!(std::fs::read_to_string(d.join("a/journal.jsonl")).unwrap(), "1\n");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// An instance whose `agent.json` cannot be read is not taken for incarnation 0 (and
    /// emptied): its files are left as they are, and it keeps its place in the cut.
    #[test]
    fn an_unreadable_agent_json_leaves_its_instance_untouched() {
        let d = dir("unreadable");
        instance(&d, "a", 7, "1\n2\n3\n");
        instance(&d, "b", 9, "x\ny\n");
        std::fs::write(d.join("a/agent.json"), "{\"id\":\"a\",\"incarn").unwrap();
        std::fs::write(d.join(LOG), "{\"epoch\":0,\"journals\":{\"a\":[7,2],\"b\":[9,2]}}\n").unwrap();
        let cut = recover(&d).unwrap();
        assert_eq!(cut.journals, Files::from([("a".into(), (7, 2)), ("b".into(), (9, 2))]));
        assert_eq!(std::fs::read_to_string(d.join("a/journal.jsonl")).unwrap(), "1\n2\n3\n");
        assert_eq!(std::fs::read_to_string(d.join("b/journal.jsonl")).unwrap(), "x\n");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// One owner at a time. Dropping the owner releases the lock even while a copy of its
    /// descriptor lives on (a process another thread forked, before it execs): closing alone
    /// left the next runtime in this process refused.
    #[test]
    fn the_owner_releases_the_directory_when_dropped() {
        let d = dir("owner");
        let owner = own(&d).unwrap();
        assert!(own(&d).err().is_some_and(|e| e.contains("in use by another runtime")));
        let forked = owner.0.try_clone().unwrap();
        drop(owner);
        let again = own(&d).unwrap();
        drop((forked, again));
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Lines join the open epoch; a commit covers them, wakes the waiting instances' workers
    /// and is recorded in the log, which recovery reads back.
    #[test]
    fn a_barrier_commits_what_was_written_and_records_the_cut() {
        let d = dir("barrier");
        instance(&d, "a", 1, "");
        let durable = Arc::new(Durable::new());
        durable
            .start(&d, recover(&d).unwrap(), Default::default(), Weak::new())
            .unwrap();
        let file = std::fs::OpenOptions::new()
            .append(true)
            .open(d.join("a/journal.jsonl"))
            .unwrap();
        let mut journal = Journal::new(file, "a", 1).unwrap();
        assert!(matches!(durable.watch(0, "a", 0), Ready::Now));
        let e = durable.append(&mut journal, b"one\n").unwrap();
        assert_eq!(durable.written(), e);
        durable.wait(e).unwrap();
        assert!(matches!(durable.watch(0, "a", e), Ready::Now));
        let e2 = durable.append(&mut journal, b"two\n").unwrap();
        assert!(e2 > e);
        durable.finish(false).unwrap();
        assert_eq!(read_log(&d.join(LOG)).unwrap().unwrap().journals["a"], (1, 8));
        // A clean shutdown closes the log: the journals are complete as they stand.
        let durable = Arc::new(Durable::new());
        durable
            .start(&d, recover(&d).unwrap(), Default::default(), Weak::new())
            .unwrap();
        durable.finish(true).unwrap();
        assert_eq!(read_log(&d.join(LOG)).unwrap(), None);
        let _ = std::fs::remove_dir_all(&d);
    }
}
