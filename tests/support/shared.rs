//! One loaded tokenizer per file for a whole test program. Loading one takes most of a second
//! of processor time, and a program whose tests each load their own spends its time there.
#![allow(dead_code)] // Each integration-test binary uses a different subset.
use affordance::emit::Encoder;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

type Slot = Arc<OnceLock<Arc<Encoder>>>;

pub fn tokenizer(path: &Path) -> Arc<Encoder> {
    static LOADED: Mutex<Option<HashMap<PathBuf, Slot>>> = Mutex::new(None);
    let slot = LOADED
        .lock()
        .unwrap()
        .get_or_insert_with(HashMap::new)
        .entry(path.to_path_buf())
        .or_default()
        .clone();
    slot.get_or_init(|| Arc::new(Encoder::from_file(path.to_str().unwrap()).unwrap()))
        .clone()
}

/// An instance's directory in a run directory, found by its name (instances live under
/// `instances/<id>/`, each naming itself in its `agent.json`).
pub fn instance_dir(run: &Path, name: &str) -> PathBuf {
    find_instance(run, name).unwrap_or_else(|| panic!("no instance {name:?} in {}", run.display()))
}

pub fn find_instance(run: &Path, name: &str) -> Option<PathBuf> {
    std::fs::read_dir(run.join("instances"))
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            std::fs::read(p.join("agent.json"))
                .ok()
                .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
                .is_some_and(|v| v["id"] == name)
        })
}

/// A file of an instance, written as the path it had when instances were named directories:
/// `inst(&dir, "run/a/kid/journal.jsonl")` is the journal of instance `a/kid` in run `dir/run`.
pub fn inst(base: &Path, rel: &str) -> PathBuf {
    let (path, file) = rel
        .rsplit_once('/')
        .unwrap_or_else(|| panic!("{rel}: no instance in the path"));
    let parts: Vec<&str> = path.split('/').collect();
    for k in 0..parts.len() {
        let run = parts[..k].iter().fold(base.to_path_buf(), |p, s| p.join(s));
        if run.join("instances").is_dir()
            && let Some(d) = find_instance(&run, &parts[k..].join("/"))
        {
            return d.join(file);
        }
    }
    panic!("{rel}: no such instance under {}", base.display())
}

/// A new instance directory in a run directory, named `name` (for a test that writes its files).
pub fn new_instance(run: &Path, name: &str) -> PathBuf {
    let d = run
        .join("instances")
        .join(affordance::runtime::exec::instance_component(name));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("agent.json"), serde_json::json!({"id": name}).to_string()).unwrap();
    d
}
