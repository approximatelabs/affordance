//! Render a TSX app, invoke a visible action, and render the resulting state.
//! cargo run --release --example hello -- /path/to/tokenizer.json
use affordance::{
    View,
    emit::{Encoder, RunCache},
    value::state,
};
use serde_json::json;
use std::path::Path;

fn main() -> Result<(), String> {
    let path = std::env::args().nth(1).ok_or("usage: hello /path/to/tokenizer.json")?;
    let encoder = Encoder::from_file(&path)?;
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("views/demo/hello.tsx");
    let view = View::load(&source, "Hello", &encoder)?;
    let now = chrono::Utc::now().naive_utc();
    let mut cache = RunCache::new(true);
    let initial = state(&json!({"name": "world"}));

    let before = view.render(&encoder, initial.clone(), &mut cache, now)?;
    println!("{}", encoder.tok.decode(&before.ids, false).map_err(|e| e.to_string())?);
    println!("{} token IDs; action: greet", before.ids.len());

    // A host normally obtains this action and its arguments from a model's reply.
    let acted = view.act(&encoder, initial, now, "greet", 0, &json!({"name": "reader"}))?;
    let after = view.render(&encoder, state(&acted.state), &mut cache, now)?;
    println!("{}", encoder.tok.decode(&after.ids, false).map_err(|e| e.to_string())?);
    Ok(())
}
