//! Run a check over many independent items (tokenizers, models, views) on every core. Each item
//! builds what it needs on its own thread; results come back in the items' order.
#![allow(dead_code)]

pub fn in_parallel<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let next = std::sync::atomic::AtomicUsize::new(0);
    let done: Vec<std::sync::Mutex<Option<R>>> = items.iter().map(|_| Default::default()).collect();
    // Several test files do this at once, each beside sixty other processes: a thread for every
    // core in each of them is more threads than cores many times over.
    let threads = std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .min(
            std::env::var("AFFORDANCE_EACH_THREADS")
                .ok()
                .and_then(|n| n.parse().ok())
                .unwrap_or(24),
        )
        .min(items.len().max(1));
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some(item) = items.get(i) else { break };
                    *done[i].lock().unwrap() = Some(f(item));
                }
            });
        }
    });
    done.into_iter()
        .map(|slot| slot.into_inner().unwrap().expect("every item was run"))
        .collect()
}

/// The tokenizers a test of every tokenizer walks: those of the models with hand-written
/// views, and, under `tools/check-models.sh` (`AFFORDANCE_ALL_MODELS`), every one in
/// `ref/tokenizers`.
pub fn reference_tokenizers() -> Vec<std::path::PathBuf> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = root.join("ref/tokenizers");
    let mut paths: Vec<std::path::PathBuf> = std::fs::read_dir(&dir)
        .expect("reference data: ref/tokenizers (docs/DEVELOPING.md)")
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            let name = path.file_name().unwrap().to_str().unwrap();
            name.ends_with(".json") && !name.ends_with(".special.json")
        })
        .collect();
    if std::env::var("AFFORDANCE_ALL_MODELS").is_err() {
        let index: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(root.join("views/index.json")).unwrap()).unwrap();
        paths.retain(|path| {
            let model = path.file_stem().unwrap().to_str().unwrap();
            index
                .get(model)
                .is_none_or(|entry| entry["converted"] != true && !entry["shared"].is_string())
        });
    }
    paths.sort();
    paths
}
