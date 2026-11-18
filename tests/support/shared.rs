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
