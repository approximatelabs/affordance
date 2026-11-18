//! How long loading a tokenizer takes, and each stage of it: the cost every program that renders
//! pays once per model.
//!
//!     cargo run --release --example load_bench -- ref/tokenizers/Qwen__Qwen3-8B.json ...
use std::time::Instant;
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--threads") {
        for n in args[1].split(',') {
            together(n.parse().unwrap(), &args[2]);
        }
        return;
    }
    for path in args {
        let t = Instant::now();
        let text = std::fs::read_to_string(&path).unwrap();
        let read = t.elapsed();
        let t = Instant::now();
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        let parse = t.elapsed();
        drop(json);
        let t = Instant::now();
        let tok: tokenizers::Tokenizer = text.parse().unwrap();
        let hf = t.elapsed();
        drop(tok);
        let t = Instant::now();
        let enc = affordance::emit::Encoder::from_file(&path).unwrap();
        let ours = t.elapsed();
        let t = Instant::now();
        let n = enc
            .encode_run("Hello, world. This is a test of 1234 things.", true, false)
            .len();
        let first = t.elapsed();
        println!(
            "{:<46} {:>5.1} MB  read {:>4.0} ms  json {:>4.0} ms  tokenizers crate {:>5.0} ms  Encoder::from_file {:>5.0} ms  first encode {:>5.1} ms ({n} ids)",
            std::path::Path::new(&path).file_name().unwrap().to_str().unwrap(),
            text.len() as f64 / 1e6,
            read.as_secs_f64() * 1e3,
            parse.as_secs_f64() * 1e3,
            hf.as_secs_f64() * 1e3,
            ours.as_secs_f64() * 1e3,
            first.as_secs_f64() * 1e3
        );
    }
}

/// `load_bench --threads N PATH`: the same file loaded on N threads at once, by the tokenizers
/// crate alone and by `Encoder::from_file`. Loading that scales is N loads in the time of one.
fn together(n: usize, path: &str) {
    let text = std::fs::read_to_string(path).unwrap();
    let wall = |f: &(dyn Fn() + Sync)| {
        let t = Instant::now();
        std::thread::scope(|s| {
            for _ in 0..n {
                s.spawn(f);
            }
        });
        t.elapsed().as_secs_f64() * 1e3
    };
    let json = wall(&|| drop(serde_json::from_str::<serde_json::Value>(&text).unwrap()));
    let crate_only = wall(&|| drop(text.parse::<tokenizers::Tokenizer>().unwrap()));
    let ours = wall(&|| drop(affordance::emit::Encoder::from_file(path).unwrap()));
    println!(
        "{n:>3} threads: json {json:>6.0} ms   tokenizers crate {crate_only:>6.0} ms   Encoder::from_file {ours:>6.0} ms"
    );
}
