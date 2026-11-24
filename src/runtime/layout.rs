//! Where a run keeps each instance's files. An instance's name is a label for addressing it
//! (`a`, `a/kid`) and never becomes a path: each instance lives in `instances/<id>/`, under an id
//! the runtime assigns when it is first created (a UUIDv7: random, ordered by creation), and its
//! `instance.json` records its name. The run's own files (`run.json`, `commit.jsonl`, `blobs/`,
//! `clock.json`, the lock) sit beside `instances/`, where no name can reach them. An instance
//! replaced by a fresh mount moves to `retired/<id>/`; the new one gets a new id.
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The directory of every instance, in a run directory.
pub(crate) const INSTANCES: &str = "instances";
const RETIRED: &str = "retired";
/// The run's own record: it says the directory holds a run.
pub const RUN: &str = "run.json";

/// The instances of a run directory, by name.
pub(crate) struct Layout {
    root: PathBuf,
    by_name: Mutex<HashMap<String, PathBuf>>,
}

impl Layout {
    /// The layout of the run directory `root`, as its instances' records say.
    pub(crate) fn open(root: &Path) -> Layout {
        Layout {
            root: root.to_path_buf(),
            by_name: Mutex::new(scan(root).into_iter().collect()),
        }
    }

    /// An existing instance's directory.
    pub(crate) fn dir(&self, name: &str) -> Option<PathBuf> {
        self.by_name.lock().unwrap().get(name).cloned()
    }

    /// An instance's directory: its own if it has one, else a new one under a new id (created).
    pub(crate) fn dir_or_new(&self, name: &str) -> Result<PathBuf, String> {
        let mut by_name = self.by_name.lock().unwrap();
        if let Some(d) = by_name.get(name) {
            return Ok(d.clone());
        }
        let d = self.root.join(INSTANCES).join(uuid_v7());
        std::fs::create_dir_all(&d).map_err(|e| format!("{}: {e}", d.display()))?;
        // Named from the start: a directory a crash leaves before its full record (the runtime
        // writes that next) is still found by its name.
        let record = d.join("instance.json");
        std::fs::write(&record, serde_json::json!({"id": name}).to_string())
            .map_err(|e| format!("{}: {e}", record.display()))?;
        by_name.insert(name.to_string(), d.clone());
        Ok(d)
    }

    /// Move an instance's files, and its descendants', to `retired/<id>/` (a fresh mount over
    /// it): the names are free for new instances.
    pub(crate) fn retire_tree(&self, name: &str) -> Result<(), String> {
        let gone: Vec<PathBuf> = {
            let mut by_name = self.by_name.lock().unwrap();
            let names: Vec<String> = by_name
                .keys()
                .filter(|n| *n == name || n.strip_prefix(name).is_some_and(|rest| rest.starts_with('/')))
                .cloned()
                .collect();
            names.iter().filter_map(|n| by_name.remove(n)).collect()
        };
        if gone.is_empty() {
            return Ok(());
        }
        std::fs::create_dir_all(self.root.join(RETIRED)).map_err(|e| e.to_string())?;
        for d in &gone {
            let to = self.root.join(RETIRED).join(d.file_name().unwrap_or_default());
            std::fs::rename(d, &to).map_err(|e| format!("{}: {e}", d.display()))?;
        }
        // The moves are durable before the new instance's first line is.
        for dir in [self.root.join(INSTANCES), self.root.join(RETIRED)] {
            std::fs::File::open(&dir)
                .and_then(|f| f.sync_all())
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

/// Every instance of the run directory `root`, by name: each `instances/<id>/` whose `instance.json`
/// names it. A directory whose record cannot be read is not listed (its own restore reports it).
pub(crate) fn scan(root: &Path) -> Vec<(String, PathBuf)> {
    let mut found = vec![];
    let Ok(entries) = std::fs::read_dir(root.join(INSTANCES)) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = std::fs::read(path.join("instance.json"))
            .ok()
            .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
            .and_then(|v| v["id"].as_str().map(str::to_owned));
        if let Some(name) = name {
            found.push((name, path));
        }
    }
    found.sort();
    found
}

/// Whether `root` holds a run (its record, or any instance).
pub fn holds_a_run(root: &Path) -> bool {
    root.join(RUN).is_file() || !scan(root).is_empty()
}

/// A UUIDv7: 48 bits of Unix milliseconds, then random bits (RFC 9562), as lowercase hex.
fn uuid_v7() -> String {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64);
    let mut random = [0u8; 10];
    let filled = std::fs::File::open("/dev/urandom").and_then(|mut f| {
        use std::io::Read;
        f.read_exact(&mut random)
    });
    if filled.is_err() {
        // No system randomness: a process-unique fallback (ids need only be unique, not secret).
        use std::hash::{BuildHasher, Hasher};
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u64(NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
        h.write_u128(std::time::Instant::now().elapsed().as_nanos());
        random[..8].copy_from_slice(&h.finish().to_le_bytes());
    }
    let mut b = [0u8; 16];
    b[..6].copy_from_slice(&ms.to_be_bytes()[2..]);
    b[6..].copy_from_slice(&random);
    b[6] = 0x70 | (b[6] & 0x0f);
    b[8] = 0x80 | (b[8] & 0x3f);
    let h: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &h[..8], &h[8..12], &h[12..16], &h[16..20], &h[20..])
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_uuid_v7_has_its_version_and_variant_and_is_ordered_by_time() {
        let a = super::uuid_v7();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let b = super::uuid_v7();
        assert_eq!(a.len(), 36);
        assert_eq!(&a[14..15], "7");
        assert!(matches!(&a[19..20], "8" | "9" | "a" | "b"));
        assert!(a < b, "{a} {b}");
    }

    #[test]
    fn names_that_look_like_the_runs_own_files_are_only_names() {
        let root = std::env::temp_dir().join(format!("aff-layout-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let layout = super::Layout::open(&root);
        for name in ["blobs", "commit.jsonl", "instances", "a b", "..x"] {
            let d = layout.dir_or_new(name).unwrap();
            assert!(d.starts_with(root.join("instances")), "{name}: {}", d.display());
            std::fs::write(d.join("instance.json"), serde_json::json!({"id": name}).to_string()).unwrap();
        }
        assert!(!root.join("blobs").exists());
        let found: Vec<String> = super::scan(&root).into_iter().map(|(n, _)| n).collect();
        assert_eq!(found.len(), 5);
        let _ = std::fs::remove_dir_all(&root);
    }
}
