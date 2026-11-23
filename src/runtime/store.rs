//! Blob storage, per agent (the browser's IndexedDB): large payloads the app keeps out of the
//! prompt and reads back on demand (`read_event`, `search_history`). Optionally persisted as an
//! append-only JSONL file per agent; then the store keeps only where each text is in that file
//! and reads it back when asked, so a long run's payloads are not held in memory.
//!
//! - `store.put {text, key?}` -> `{key}` (keys default to `b1`, `b2`, ...)
//! - `store.get {key, offset?, limit?}` -> `{key, text, offset, total}` (offsets in characters)
//! - `store.search {query, limit?}` -> `{hits: [{key, at, snippet}], total}` (case-insensitive)
//! - `store.list {}` -> `{keys}`
use super::blobs::Blobs;
use serde_json::{Value as J, json};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(serde::Deserialize)]
struct Record {
    key: String,
    text: String,
    next: u64,
}

/// Where a text is: in memory (no store file), or the byte range of its line in the store file.
enum At {
    Heap(String),
    Line(u64, usize),
}

#[derive(Default)]
struct Space {
    blobs: BTreeMap<String, At>,
    order: Vec<String>,
    next: u64,
    write_error: Option<String>,
}

/// Blob texts are kept as given: a large one arrives as a blob reference (`blobs.rs`) and stays
/// one, so the store holds keys and references, and reads slices from the mapped blob.
#[derive(Default)]
pub struct Store {
    spaces: Mutex<BTreeMap<String, Arc<Mutex<Space>>>>,
    /// Where each instance's store file lives (in its own directory), with a run directory.
    layout: Option<Arc<super::layout::Layout>>,
    blobs: Option<Arc<Blobs>>,
    /// A put's record is synced by the runtime's journal commit, not by the put.
    deferred: bool,
}

impl Store {
    /// A store in the run directory `dir` (each instance's file in its own directory), or in
    /// memory without one.
    pub fn new(dir: Option<PathBuf>) -> Store {
        Store::with_blobs(dir, None)
    }
    /// A store whose large texts go to `blobs`.
    pub fn with_blobs(dir: Option<PathBuf>, blobs: Option<Arc<Blobs>>) -> Store {
        Store {
            spaces: Mutex::new(BTreeMap::new()),
            layout: dir.map(|d| Arc::new(super::layout::Layout::open(&d))),
            blobs,
            deferred: false,
        }
    }
    /// The run directory's layout this store uses (the runtime uses the same one).
    pub(crate) fn layout(&self) -> Option<Arc<super::layout::Layout>> {
        self.layout.clone()
    }
    /// Leave syncing a put's record to the caller (the runtime commits it with the journals,
    /// using `file_len`).
    pub fn deferring(mut self) -> Store {
        self.deferred = true;
        self
    }
    /// The store file of `agent`, and its length (0 when absent).
    pub fn file_len(&self, agent: &str) -> Option<(PathBuf, u64)> {
        self.file(agent).map(|path| {
            let len = std::fs::metadata(&path).map_or(0, |m| m.len());
            (path, len)
        })
    }
    /// The blob store its large texts go to, if any.
    pub fn blobs(&self) -> Option<&Arc<Blobs>> {
        self.blobs.as_ref()
    }

    fn space(&self, agent: &str) -> Arc<Mutex<Space>> {
        self.spaces.lock().unwrap().entry(agent.into()).or_default().clone()
    }

    /// A new mount is a new store, including any descendants of its old incarnation.
    /// The host serializes subtree retirement and calls on their owning worker.
    pub(crate) fn forget_tree(&self, agent: &str) {
        let prefix = format!("{agent}/");
        self.spaces
            .lock()
            .unwrap()
            .retain(|id, _| id != agent && !id.starts_with(&prefix));
    }

    fn with_text<R>(&self, s: &str, f: impl FnOnce(&str) -> R) -> Result<R, String> {
        match &self.blobs {
            Some(b) => b.with_text(s, f),
            None => Ok(f(s)),
        }
    }

    /// The agent's store file, open for reading (none without a store directory).
    fn open(&self, agent: &str) -> Result<Option<std::fs::File>, String> {
        let Some(path) = self.file(agent).filter(|p| p.exists()) else {
            return Ok(None);
        };
        std::fs::File::open(&path)
            .map(Some)
            .map_err(|e| format!("{}: {e}", path.display()))
    }

    /// The text of `at`, read from the store file `f` when it lives there.
    fn read(f: Option<&std::fs::File>, at: &At) -> Result<String, String> {
        match at {
            At::Heap(s) => Ok(s.clone()),
            &At::Line(offset, len) => {
                use std::os::unix::fs::FileExt;
                let f = f.ok_or("the store file is missing")?;
                let mut buf = vec![0; len];
                f.read_exact_at(&mut buf, offset).map_err(|e| format!("store: {e}"))?;
                let record: Record = serde_json::from_slice(&buf).map_err(|e| format!("store: invalid record: {e}"))?;
                Ok(record.text)
            }
        }
    }

    fn file(&self, agent: &str) -> Option<PathBuf> {
        self.layout.as_ref()?.dir(agent).map(|d| d.join("store.jsonl"))
    }

    /// Load an agent's persisted blobs (on restore).
    pub fn load(&self, agent: &str) -> Result<(), String> {
        let handle = self.space(agent);
        let mut current = handle.lock().unwrap();
        let mut space = Space::default();
        if let Some(path) = self.file(agent)
            && let Some(reader) = super::jsonl::JsonLines::<Record>::open(&path, "store")?
        {
            for row in reader {
                let row = row?;
                let record = row.value;
                if record.next < space.next {
                    return Err(format!(
                        "{} at byte {}: store counter moved backwards",
                        path.display(),
                        row.offset
                    ));
                }
                if !space.blobs.contains_key(&record.key) {
                    space.order.push(record.key.clone());
                }
                space.blobs.insert(record.key, At::Line(row.offset, row.len));
                space.next = record.next;
            }
        }
        *current = space;
        Ok(())
    }

    /// Serve one `harness.store.*` call (`put`, `get`, `search`, `list`) for `agent`.
    pub fn call(&self, agent: &str, op: &str, args: &J) -> Result<J, String> {
        let handle = self.space(agent);
        let mut space = handle.lock().unwrap();
        match op {
            "store.put" => {
                if let Some(error) = &space.write_error {
                    return Err(format!("store writes disabled until reload: {error}"));
                }
                let text = args["text"].as_str().ok_or("store.put needs text")?.to_string();
                let mut next = space.next;
                let key = match args["key"].as_str() {
                    Some(k) => k.to_string(),
                    None => loop {
                        next = next.checked_add(1).ok_or("store key counter exhausted")?;
                        let key = format!("b{next}");
                        if !space.blobs.contains_key(&key) {
                            break key;
                        }
                    },
                };
                let file = match &self.layout {
                    Some(layout) => Some(layout.dir_or_new(agent)?.join("store.jsonl")),
                    None => None,
                };
                let at = match file {
                    Some(path) => {
                        let mut f = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(&path)
                            .map_err(|e| e.to_string())?;
                        let mut line = json!({"key": key, "text": text, "next": next}).to_string();
                        line.push('\n');
                        let deferred = self.deferred;
                        let offset = append_or_rollback(&mut f, |f| {
                            f.write_all(line.as_bytes())?;
                            if deferred {
                                return Ok(());
                            }
                            f.sync_all()?;
                            std::fs::File::open(path.parent().unwrap())?.sync_all()
                        })
                        .map_err(|failure| {
                            let error = format!("{}: {}", path.display(), failure.message);
                            if failure.unknown_tail {
                                space.write_error = Some(error.clone());
                            }
                            error
                        })?;
                        At::Line(offset, line.len() - 1)
                    }
                    None => At::Heap(text),
                };
                if !space.blobs.contains_key(&key) {
                    space.order.push(key.clone());
                }
                space.blobs.insert(key.clone(), at);
                space.next = next;
                Ok(json!({"key": key}))
            }
            "store.get" => {
                let key = args["key"].as_str().ok_or("store.get needs key")?;
                let Some(at) = space.blobs.get(key) else {
                    return Ok(json!({"key": key, "missing": true}));
                };
                let text = &Store::read(self.open(agent)?.as_ref(), at)?;
                let (offset, limit) = (
                    args["offset"].as_u64().unwrap_or(0) as usize,
                    args["limit"].as_u64().map(|l| l as usize),
                );
                self.with_text(text, |text| {
                    let total = text.chars().count();
                    let offset = offset.min(total);
                    let end = limit.map(|l| offset.saturating_add(l).min(total)).unwrap_or(total);
                    json!({"key": key, "text": char_range(text, offset, end), "offset": offset, "total": total})
                })
            }
            "store.search" => {
                let q = args["query"].as_str().ok_or("store.search needs query")?.to_lowercase();
                let limit = args["limit"].as_u64().unwrap_or(5) as usize;
                let mut hits = vec![];
                let f = self.open(agent)?;
                for key in &space.order {
                    let text = Store::read(f.as_ref(), &space.blobs[key])?;
                    let hit = self.with_text(&text, |text| {
                        let at = find_ci(text, &q)?;
                        let start = at.saturating_sub(80);
                        Some(json!({"key": key, "at": at, "snippet": char_range(text, start, at + q.chars().count() + 80)}))
                    })?;
                    hits.extend(hit);
                }
                let total = hits.len();
                hits.truncate(limit);
                Ok(json!({"hits": hits, "total": total}))
            }
            "store.list" => Ok(json!({"keys": space.order})),
            other => Err(format!("unknown store operation {other}")),
        }
    }
}

struct AppendFailure {
    message: String,
    unknown_tail: bool,
}

/// Failure leaves the previous durable prefix intact, or explicitly makes further appends unsafe.
fn append_or_rollback(
    file: &mut std::fs::File,
    append: impl FnOnce(&mut std::fs::File) -> std::io::Result<()>,
) -> Result<u64, AppendFailure> {
    let offset = file
        .metadata()
        .map_err(|e| AppendFailure {
            message: e.to_string(),
            unknown_tail: false,
        })?
        .len();
    if let Err(error) = append(file) {
        let rollback = file.set_len(offset).and_then(|_| file.sync_all());
        return Err(match rollback {
            Ok(()) => AppendFailure {
                message: format!("append failed and was rolled back: {error}"),
                unknown_tail: false,
            },
            Err(failed) => AppendFailure {
                message: format!("append failed: {error}; rollback failed: {failed}"),
                unknown_tail: true,
            },
        });
    }
    Ok(offset)
}

/// Characters `a..b` of `s` (clamped), without collecting its characters.
fn char_range(s: &str, a: usize, b: usize) -> &str {
    if b <= a {
        return "";
    }
    let mut bounds = s.char_indices().map(|(i, _)| i).chain(std::iter::once(s.len()));
    let start = bounds.nth(a).unwrap_or(s.len());
    let end = bounds.nth(b - a - 1).unwrap_or(s.len());
    &s[start..end]
}

/// The character position of the first case-insensitive match of `q` (already lowercase).
fn find_ci(text: &str, q: &str) -> Option<usize> {
    if q.is_ascii() {
        let (t, n) = (text.as_bytes(), q.len());
        if n == 0 {
            return Some(0);
        }
        let b = (0..t.len().checked_sub(n)? + 1).find(|&i| t[i..i + n].eq_ignore_ascii_case(q.as_bytes()))?;
        return Some(text[..b].chars().count());
    }
    let lower = text.to_lowercase();
    let pos = lower.find(q)?;
    Some(lower[..pos].chars().count())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct Scratch(PathBuf);
    impl Scratch {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "aff-store-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn an_interrupted_append_can_be_followed_by_a_put_and_reload() {
        let record = "{\"key\":\"b2\",\"text\":\"😀payload\",\"next\":2}\n".as_bytes();
        for cut in 0..=record.len() {
            let dir = Scratch::new();
            let store = Store::new(Some(dir.0.clone()));
            store.call("a", "store.put", &json!({"text":"saved"})).unwrap();
            let home = store.layout().unwrap().dir("a").unwrap();
            // In a run, every instance's directory has its record, which names it.
            std::fs::write(home.join("agent.json"), json!({"id": "a"}).to_string()).unwrap();
            let path = home.join("store.jsonl");
            let prefix = std::fs::read(&path).unwrap();
            let mut file = std::fs::OpenOptions::new().append(true).open(&path).unwrap();
            let failed = append_or_rollback(&mut file, |file| {
                file.write_all(&record[..cut])?;
                Err(std::io::Error::other("injected append or sync failure"))
            })
            .err()
            .unwrap();
            assert!(!failed.unknown_tail);
            assert_eq!(std::fs::read(&path).unwrap(), prefix);
            assert_eq!(
                store.call("a", "store.put", &json!({"text":"next"})).unwrap()["key"],
                "b2"
            );
            let restored = Store::new(Some(dir.0.clone()));
            restored.load("a").unwrap();
            assert_eq!(
                restored.call("a", "store.list", &json!({})).unwrap()["keys"],
                json!(["b1", "b2"])
            );
            assert_eq!(
                restored.call("a", "store.get", &json!({"key":"b2"})).unwrap()["text"],
                "next"
            );
        }
        let mut read_only = std::fs::File::open("/dev/null").unwrap();
        assert!(
            append_or_rollback(&mut read_only, |f| f.write_all(b"no"))
                .err()
                .unwrap()
                .unknown_tail
        );
    }
    #[test]
    fn store_operations_share_only_their_own_lock_and_failed_puts_do_not_burn_keys() {
        let dir = Scratch::new();
        let store = Arc::new(Store::new(Some(dir.0.clone())));
        let a = store.space("a");
        let held = a.lock().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let other = store.clone();
        let worker = std::thread::spawn(move || {
            other.call("b", "store.put", &json!({"text":"independent"})).unwrap();
            other.load("b").unwrap();
            tx.send(other.call("b", "store.get", &json!({"key":"b1"})).unwrap())
                .unwrap();
        });
        let answer = rx.recv_timeout(std::time::Duration::from_secs(2));
        drop(held);
        worker.join().unwrap();
        assert_eq!(answer.unwrap()["text"], "independent");
        // An open failure must leave the automatic key/counter uncommitted.
        let path = store.layout().unwrap().dir_or_new("a").unwrap().join("store.jsonl");
        std::fs::create_dir_all(&path).unwrap();
        assert!(store.call("a", "store.put", &json!({"text":"failed"})).is_err());
        std::fs::remove_dir(&path).unwrap();
        assert_eq!(
            store.call("a", "store.put", &json!({"text":"saved"})).unwrap()["key"],
            "b1"
        );
        std::fs::remove_file(&path).unwrap();
        store.load("a").unwrap();
        assert_eq!(store.call("a", "store.list", &json!({})).unwrap()["keys"], json!([]));
        // A write and its rollback both fail on this device (Linux's always-full device).
        // Quarantine only this space.
        if cfg!(target_os = "linux") {
            std::os::unix::fs::symlink("/dev/full", &path).unwrap();
            assert!(store.call("a", "store.put", &json!({"text":"failed"})).is_err());
            std::fs::remove_file(&path).unwrap();
            std::fs::File::create(&path).unwrap();
            assert!(
                store
                    .call("a", "store.put", &json!({"text":"blocked"}))
                    .unwrap_err()
                    .contains("disabled")
            );
        }
        store
            .call("b", "store.put", &json!({"text":"still independent"}))
            .unwrap();
        store.load("a").unwrap();
        assert_eq!(
            store.call("a", "store.put", &json!({"text":"recovered"})).unwrap()["key"],
            "b1"
        );
    }
}
