//! Large strings on disk (the browser's external strings). A string of at least `min_bytes` is
//! written once to `blobs/<sha256>` and travels through state, transcript, journal, store and
//! commands as a short reference string, `"\0blob:<sha256>:<bytes>:<chars>"`. The language reads
//! a reference as host text (`value::Text::Host`) backed by a read-only map of the file, so its
//! bytes are reclaimable page cache rather than heap. Apps see an ordinary string.
//!
//! References are content addresses: the same text always has the same reference, replay is
//! unchanged, and a reference compares equal exactly when the texts do. Any incoming string
//! that starts with the reference prefix is itself stored as a blob, so a reference is never
//! ambiguous. Blobs are written before anything that refers to them is journaled. In a runtime
//! they are synced by the journal commit that covers the first line referring to them
//! (`Unsynced`); without one, at once.
use crate::value::{HostResolver, HostText};
use memmap2::Mmap;
use serde_json::Value as J;
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::{BTreeSet, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

const PREFIX: &str = "\u{0}blob:";

/// Strings at least this long are stored as blobs unless the runtime configures otherwise.
pub const DEFAULT_MIN_BYTES: usize = 64 << 10;

/// Files written but not yet synced, with the directories whose entries are new. A journal
/// commit syncs them before it records lines: every line that refers to one was written after
/// it was added here, so the commit that covers the line covers the file.
#[derive(Debug, Default)]
pub struct Unsynced {
    pending: Mutex<(BTreeSet<PathBuf>, BTreeSet<PathBuf>)>,
}

impl Unsynced {
    /// Note a written file (and its directory) as not yet synced to disk.
    pub fn add(&self, file: &Path, dir: Option<&Path>) {
        let mut p = self.pending.lock().unwrap();
        p.0.insert(file.to_path_buf());
        if let Some(dir) = dir {
            p.1.insert(dir.to_path_buf());
        }
    }
    /// Whether nothing waits to be synced.
    pub fn is_empty(&self) -> bool {
        let p = self.pending.lock().unwrap();
        p.0.is_empty() && p.1.is_empty()
    }
    /// What was added so far, to be synced (files, then directories).
    pub fn take(&self) -> (Vec<PathBuf>, Vec<PathBuf>) {
        let mut p = self.pending.lock().unwrap();
        let (files, dirs) = std::mem::take(&mut *p);
        (files.into_iter().collect(), dirs.into_iter().collect())
    }
    /// Sync everything added so far, on this thread.
    pub fn sync_now(&self) -> std::io::Result<()> {
        let (files, dirs) = self.take();
        files.iter().chain(&dirs).try_for_each(|p| sync_path(p))
    }
}

/// `fdatasync` a file (or `fsync` a directory) by path.
pub fn sync_path(path: &Path) -> std::io::Result<()> {
    let f = std::fs::File::open(path)?;
    if f.metadata()?.is_dir() {
        f.sync_all()
    } else {
        f.sync_data()
    }
}

#[derive(Debug)]
/// Where the runtime keeps large strings: on disk, by content hash, so journals and state hold references instead of the bytes.
pub struct Blobs {
    dir: PathBuf,
    min_bytes: usize,
    unsynced: Option<Arc<Unsynced>>,
    /// Blobs whose files this process wrote or checked: a later put of the same text trusts them.
    known: Mutex<HashSet<String>>,
}

static TMP: AtomicU64 = AtomicU64::new(0);

struct Mapped {
    map: Mmap,
    json: String,
    chars: usize,
}

impl HostText for Mapped {
    // Validated as UTF-8 when mapped; blob files are never rewritten.
    fn as_str(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(&self.map) }
    }
    fn json(&self) -> &str {
        &self.json
    }
    fn char_count(&self) -> usize {
        self.chars
    }
}

/// `(sha256, bytes, chars)` of a well-formed reference.
fn parse(s: &str) -> Option<(&str, usize, usize)> {
    let mut it = s.strip_prefix(PREFIX)?.split(':');
    let (hash, bytes, chars) = (it.next()?, it.next()?.parse().ok()?, it.next()?.parse().ok()?);
    (it.next().is_none() && hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()))
        .then_some((hash, bytes, chars))
}

impl Blobs {
    /// A blob store in `dir` for strings of at least `min_bytes`.
    pub fn new(dir: PathBuf, min_bytes: usize) -> Result<Blobs, String> {
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        Ok(Blobs {
            dir,
            min_bytes: min_bytes.max(1),
            unsynced: None,
            known: Mutex::new(HashSet::new()),
        })
    }

    /// Leave syncing to whoever drains `unsynced` (the runtime's journal commit).
    pub fn deferring(mut self, unsynced: Arc<Unsynced>) -> Blobs {
        self.unsynced = Some(unsynced);
        self
    }

    /// The file for `hash` is in place: sync it now, or record it for the next commit.
    fn placed(&self, path: &Path, new_entry: bool) -> std::io::Result<()> {
        match &self.unsynced {
            Some(u) => {
                u.add(path, new_entry.then_some(self.dir.as_path()));
                Ok(())
            }
            None => {
                sync_path(path)?;
                if new_entry {
                    sync_path(&self.dir)?;
                }
                Ok(())
            }
        }
    }

    /// Whether the existing file for `hash` holds exactly `bytes`. A crash may leave a blob file
    /// named for its content without that content (renamed before it was synced, and no
    /// committed line refers to it): a later put of the same text must not trust it unchecked.
    fn holds(&self, path: &Path, bytes: &[u8]) -> bool {
        let Ok(file) = std::fs::File::open(path) else {
            return false;
        };
        if file.metadata().map_or(true, |m| m.len() != bytes.len() as u64) {
            return false;
        }
        if bytes.is_empty() {
            return true;
        }
        // Safety: blob files are immutable once renamed into place.
        unsafe { Mmap::map(&file) }.is_ok_and(|map| map[..] == *bytes)
    }

    /// Strings at least this long are stored as blobs.
    pub fn min_bytes(&self) -> usize {
        self.min_bytes
    }

    /// A fresh path in the blob directory for writing a blob in place (see `put_file`).
    pub fn temp_path(&self) -> PathBuf {
        self.dir.join(format!(
            ".tmp.{}.{}",
            std::process::id(),
            TMP.fetch_add(1, Ordering::Relaxed)
        ))
    }

    /// Store the file at `tmp` (written by the caller, e.g. a spooled capture) as a blob and
    /// return its reference; `None` (file left in place) if it is not UTF-8.
    pub fn put_file(&self, tmp: &std::path::Path) -> Result<Option<String>, String> {
        let err = |e: std::io::Error| format!("blob {}: {e}", tmp.display());
        let file = std::fs::File::open(tmp).map_err(err)?;
        let len = file.metadata().map_err(err)?.len();
        // Safety: the caller has finished writing `tmp` and nothing else writes it.
        let map = if len == 0 {
            None
        } else {
            Some(unsafe { Mmap::map(&file) }.map_err(err)?)
        };
        let bytes: &[u8] = map.as_deref().unwrap_or(b"");
        let Ok(text) = std::str::from_utf8(bytes) else {
            return Ok(None);
        };
        let hash: String = Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect();
        let chars = text.chars().count();
        let path = self.dir.join(&hash);
        if self.known.lock().unwrap().contains(&hash) {
            let _ = std::fs::remove_file(tmp);
        } else if self.holds(&path, bytes) {
            let _ = std::fs::remove_file(tmp);
            self.placed(&path, false).map_err(err)?;
        } else {
            std::fs::rename(tmp, &path).map_err(err)?;
            self.placed(&path, true).map_err(err)?;
        }
        self.known.lock().unwrap().insert(hash.clone());
        Ok(Some(format!("{PREFIX}{hash}:{len}:{chars}")))
    }

    /// Store `text` (once per distinct text) and return its reference.
    pub fn put(&self, text: &str) -> Result<String, String> {
        let hash: String = Sha256::digest(text.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let path = self.dir.join(&hash);
        if !self.known.lock().unwrap().contains(&hash) {
            let err = |e: std::io::Error| format!("blob {}: {e}", path.display());
            if self.holds(&path, text.as_bytes()) {
                self.placed(&path, false).map_err(err)?;
            } else {
                let tmp = self.temp_path();
                let write = || -> std::io::Result<()> {
                    let mut f = std::fs::File::create(&tmp)?;
                    f.write_all(text.as_bytes())?;
                    std::fs::rename(&tmp, &path)
                };
                write().map_err(|e| {
                    let _ = std::fs::remove_file(&tmp);
                    err(e)
                })?;
                self.placed(&path, true).map_err(err)?;
            }
            self.known.lock().unwrap().insert(hash.clone());
        }
        Ok(format!("{PREFIX}{hash}:{}:{}", text.len(), text.chars().count()))
    }

    /// True if `s` is a reference to a stored blob, its sizes the blob's own. A string from
    /// outside the runtime is one only if this holds: a forged char count would have the
    /// language index the blob's bytes as chars.
    pub fn is_ref(&self, s: &str) -> bool {
        self.present(s) && self.open(s).is_ok()
    }

    /// Whether `s` is a reference whose blob is stored with its byte size (references the
    /// runtime wrote itself: a journal, an image).
    fn present(&self, s: &str) -> bool {
        parse(s).is_some_and(|(hash, bytes, _)| {
            std::fs::metadata(self.dir.join(hash)).is_ok_and(|m| m.len() == bytes as u64)
        })
    }

    /// The first reference in `j` whose blob is missing or has the wrong size: a journal copied
    /// without its blob directory must not replay reference text as if it were the content.
    pub fn missing(&self, j: &J) -> Option<String> {
        match j {
            J::String(s) => self.missing_str(s),
            J::Array(a) => a.iter().find_map(|x| self.missing(x)),
            J::Object(o) => o.values().find_map(|x| self.missing(x)),
            _ => None,
        }
    }

    /// `missing` for one string.
    pub fn missing_str(&self, s: &str) -> Option<String> {
        (parse(s).is_some() && !self.present(s)).then(|| s.trim_start_matches('\0').to_string())
    }

    /// Replace every string of at least `min_bytes`, and any string that looks like a reference
    /// without being one, with a reference. References already present are left as they are.
    pub fn lower(&self, j: &mut J) -> Result<(), String> {
        match j {
            J::String(s) if s.starts_with(PREFIX) => {
                if !self.is_ref(s) {
                    *s = self.put(s)?;
                }
            }
            J::String(s) if s.len() >= self.min_bytes => *s = self.put(s)?,
            J::Array(a) => {
                for x in a {
                    self.lower(x)?;
                }
            }
            J::Object(o) => {
                for (_, x) in o.iter_mut() {
                    self.lower(x)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// `lower` for a language value: its long strings become blob text (host text backed by the
    /// blob file). Unchanged parts keep their identity (the same `Rc`), so a caller can tell what
    /// changed by pointer.
    pub fn lower_value(&self, v: &crate::value::Value) -> Result<crate::value::Value, String> {
        Ok(self.lower_changed(v)?.unwrap_or_else(|| v.clone()))
    }

    fn lower_changed(&self, v: &crate::value::Value) -> Result<Option<crate::value::Value>, String> {
        use crate::value::{Text, Value};
        Ok(match v {
            Value::Str(Text::Heap(s)) if (s.starts_with(PREFIX) && !self.is_ref(s)) || s.len() >= self.min_bytes => {
                let reference = self.put(s)?;
                Some(Value::Str(Text::Host(Rc::new(self.open(&reference)?))))
            }
            // A frozen subtree already lowered cannot have changed.
            Value::List(l) if l.is_lowered() => None,
            Value::Map(m) if m.is_lowered() => None,
            Value::List(l) => {
                let mut out: Option<Vec<Value>> = None;
                for (i, x) in l.read().iter().enumerate() {
                    if let Some(y) = self.lower_changed(x)? {
                        out.get_or_insert_with(|| l.read().to_vec())[i] = y;
                    }
                }
                if out.is_none() {
                    l.mark_lowered();
                }
                out.map(Value::list)
            }
            Value::Map(m) => {
                let mut out: Option<crate::value::Map> = None;
                for (k, x) in m.read().iter() {
                    if let Some(y) = self.lower_changed(x)? {
                        out.get_or_insert_with(|| m.read().clone()).insert(k.clone(), y);
                    }
                }
                if out.is_none() {
                    m.mark_lowered();
                }
                out.map(Value::map)
            }
            _ => None,
        })
    }

    fn open(&self, reference: &str) -> Result<Mapped, String> {
        let (hash, bytes, chars) = parse(reference).ok_or("not a blob reference")?;
        let path = self.dir.join(hash);
        let file = std::fs::File::open(&path).map_err(|e| format!("blob {}: {e}", path.display()))?;
        // Safety: blob files are immutable once renamed into place.
        let map = unsafe { Mmap::map(&file) }.map_err(|e| format!("blob {}: {e}", path.display()))?;
        if map.len() != bytes {
            return Err(format!(
                "blob {}: {} bytes, reference says {bytes}",
                path.display(),
                map.len()
            ));
        }
        let text = std::str::from_utf8(&map).map_err(|e| format!("blob {}: {e}", path.display()))?;
        // The count is the text's, never the reference's: the language takes a count equal to
        // the byte size to mean ASCII, and indexes bytes as chars.
        let counted = text.chars().count();
        if counted != chars {
            return Err(format!(
                "blob {}: {counted} chars, reference says {chars}",
                path.display()
            ));
        }
        Ok(Mapped {
            map,
            json: reference.to_owned(),
            chars,
        })
    }

    /// The text of `s`: a reference's blob, or `s` itself.
    pub fn text<'a>(&self, s: &'a str) -> Result<std::borrow::Cow<'a, str>, String> {
        if !self.present(s) {
            return Ok(s.into());
        }
        Ok(self.open(s)?.as_str().to_owned().into())
    }

    /// Call `f` with the text of `s` without copying a blob onto the heap.
    pub fn with_text<R>(&self, s: &str, f: impl FnOnce(&str) -> R) -> Result<R, String> {
        if !self.present(s) {
            return Ok(f(s));
        }
        Ok(f(self.open(s)?.as_str()))
    }

    /// A copy of `j` with every reference replaced by its text (for whatever leaves the runtime).
    pub fn expand(&self, j: &J) -> Result<J, String> {
        Ok(match j {
            J::String(s) => J::String(self.text(s)?.into_owned()),
            J::Array(a) => J::Array(a.iter().map(|x| self.expand(x)).collect::<Result<_, _>>()?),
            J::Object(o) => J::Object(
                o.iter()
                    .map(|(k, x)| Ok((k.clone(), self.expand(x)?)))
                    .collect::<Result<_, String>>()?,
            ),
            other => other.clone(),
        })
    }

    /// `expand`, but a reference to a blob over `max_bytes` stays a reference (its text is in the
    /// blob directory): a snapshot of a state holding large outputs stays small.
    pub fn expand_up_to(&self, j: &J, max_bytes: usize) -> Result<J, String> {
        Ok(match j {
            J::String(s) if parse(s).is_some_and(|(_, bytes, _)| bytes > max_bytes) && self.present(s) => j.clone(),
            J::String(s) => J::String(self.text(s)?.into_owned()),
            J::Array(a) => J::Array(
                a.iter()
                    .map(|x| self.expand_up_to(x, max_bytes))
                    .collect::<Result<_, _>>()?,
            ),
            J::Object(o) => J::Object(
                o.iter()
                    .map(|(k, x)| Ok((k.clone(), self.expand_up_to(x, max_bytes)?)))
                    .collect::<Result<_, String>>()?,
            ),
            other => other.clone(),
        })
    }

    /// How the language reads references: as host text mapped from the blob file. One resolver
    /// per thread shares maps between every value that refers to the same blob.
    pub fn resolver(self: &Arc<Self>) -> HostResolver {
        let blobs = self.clone();
        let open: RefCell<HashMap<String, Weak<Mapped>>> = RefCell::new(HashMap::new());
        Rc::new(move |s: &str| {
            parse(s)?;
            if let Some(m) = open.borrow().get(s).and_then(Weak::upgrade) {
                return Some(m as Rc<dyn HostText>);
            }
            let m = match blobs.open(s) {
                Ok(m) => Rc::new(m),
                // The text then stands as its reference. That is wrong content, so it is said.
                Err(e) => {
                    eprintln!("affordance: a blob could not be read, its reference is shown in its place: {e}");
                    return None;
                }
            };
            let mut open = open.borrow_mut();
            if open.len() > 1024 {
                open.retain(|_, w| w.strong_count() > 0);
            }
            open.insert(s.to_owned(), Rc::downgrade(&m));
            Some(m as Rc<dyn HostText>)
        })
    }
}
