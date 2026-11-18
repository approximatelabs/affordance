//! JSON adapter for the Rust app/strategy/template stack. No model inference or
//! answer oracle lives here. Callers supply selected actions and own their log.
use affordance::View;
use affordance::emit::{Encoder, RunCache};
use affordance::value::Value;
use serde_json::{Value as J, json};
use std::{io::Read, path::PathBuf};

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let arg = |key: &str| args.windows(2).find(|w| w[0] == key).map(|w| w[1].as_str());
    let family = arg("--family").unwrap_or("qwen36");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let index: J = serde_json::from_str(
        &std::fs::read_to_string(root.join("views/template_apps/index.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let model = if family == "qwen3_base" {
        "Qwen__Qwen3-8B"
    } else {
        index[family]["model"].as_str().ok_or("unknown family")?
    };
    let path = arg("--tokenizer").map(PathBuf::from).unwrap_or_else(|| {
        if model.starts_with("nvidia__") {
            root.join(format!("ref/middle_models/{model}/tokenizer.json"))
        } else {
            root.join(format!("ref/tokenizers/{model}.json"))
        }
    });
    let enc = Encoder::from_file(path.to_str().ok_or("non-UTF8 path")?)?;
    // One app file for every model: the family names the template the binding would.
    let view = View::load(
        &root.join(format!("views/demo/{}.tsx", arg("--app").unwrap_or("find_big_number"))),
        "Main",
        &enc,
    )?
    .for_model(family, &serde_json::json!({}));
    let now = chrono::NaiveDate::from_ymd_opt(2026, 10, 8)
        .unwrap()
        .and_hms_opt(18, 0, 0)
        .unwrap();
    let mut text = String::new();
    let request = if args.iter().any(|s| s == "--sample") {
        let mut rng = arg("--seed")
            .unwrap_or("20261008")
            .parse::<u64>()
            .map_err(|e| e.to_string())?;
        if rng == 0 {
            return Err("seed must be nonzero".into());
        }
        let numbers: Vec<u64> = (0..1024)
            .map(|_| {
                rng ^= rng << 13;
                rng ^= rng >> 7;
                rng ^= rng << 17;
                rng % 1_000_000
            })
            .collect();
        let budget = arg("--budget")
            .unwrap_or("256")
            .parse::<usize>()
            .map_err(|e| e.to_string())?;
        json!({"states":[{"numbers":numbers,"budget":budget,"strategy":arg("--strategy").unwrap_or("last-turn"),"log":[]}]})
    } else {
        std::io::stdin().read_to_string(&mut text).map_err(|e| e.to_string())?;
        serde_json::from_str::<J>(&text).map_err(|e| e.to_string())?
    };
    let states = request["states"].as_array().ok_or("request needs states array")?;
    let mut cache = RunCache::new(true);
    let mut output = Vec::new();
    for (i, input) in states.iter().enumerate() {
        let mut state = input.clone();
        if let Some(calls) = request["calls"][i].as_array() {
            for call in calls {
                let name = call["name"].as_str().ok_or("call needs name")?;
                let Value::Map(m) = Value::from_json(&state) else {
                    return Err("state must be object".into());
                };
                let m = std::rc::Rc::new(m.into_data());
                state = view.act(&enc, m, now, name, 0, &call["arguments"])?.state;
            }
        }
        let Value::Map(m) = Value::from_json(&state) else {
            return Err("state must be object".into());
        };
        let m = std::rc::Rc::new(m.into_data());
        let r = view.render(&enc, m, &mut cache, now)?;
        output.push(json!({"state":state,"ids":r.ids,"spans":r.spans,"text":enc.tok.decode(&r.ids,false).map_err(|e|e.to_string())?}));
    }
    println!("{}", serde_json::to_string(&output).map_err(|e| e.to_string())?);
    Ok(())
}
