//! Project a laid-out app's model actions into tool schemas, retaining native
//! tokens, visible spans and the interpreter's original handler references.
use crate::emit::{Capture, Encoder, Live, Out, RunCache};
use crate::interp::emit_child;
use crate::value::{Map, Value};
use serde_json::{Value as J, json};
use std::{collections::HashSet, rc::Rc};

/// Preserve readable names when possible. Lossy names use the full UTF-8 input
/// as identity, so sanitization/truncation cannot swap targets after a reorder.
fn base_name(raw: &str) -> String {
    let clean = raw.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-');
    // Reserve the hash delimiter: a literal label cannot impersonate a hashed one.
    if clean && raw.len() <= 40 && !raw.contains("__id_") {
        return raw.to_string();
    }
    let mut hash = 0xcbf29ce484222325u64;
    for byte in raw.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    let prefix: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .take(40)
        .collect();
    format!("{prefix}__id_{hash:016x}")
}

pub fn project(
    children: &Value,
    enc: &Encoder,
    safe: bool,
    cache: &mut RunCache,
    callable: bool,
    window: Option<usize>,
) -> Result<Value, String> {
    let mut capture = Capture::default();
    emit_child(children, &mut capture)?;
    let mut live = Live::new(enc, cache);
    live.window = window;
    live.start_in_prompt();
    live.safe = safe;
    live.replay(&capture.items)?;
    let (ids, mut spans) = live.finish()?;
    let mut tools = vec![];
    let mut used = HashSet::new();
    for span in &mut spans {
        if span.kind != "affordance" || span.meta.get("handler").is_none() {
            continue;
        }
        let original = span.meta["original_name"].as_str().unwrap_or(&span.label).to_string();
        if crate::is_harness_callback(&original) {
            continue;
        }
        let target = span.meta["target"].as_str().unwrap_or("");
        let handler = span.meta["handler"]
            .as_u64()
            .ok_or("affordance has an invalid handler reference")?;
        let suffix = if target.is_empty() {
            format!("h{handler}")
        } else {
            target.to_string()
        };
        let base = match span.meta["props"]["tool_name"].as_str() {
            // These names are the harness's own callbacks: a tool that took one would be listed
            // for the model and then refused on every call.
            Some(n) if crate::is_harness_callback(n) => {
                return Err(format!(
                    "a tool cannot be named `{n}`: the harness uses that name for its own callbacks"
                ));
            }
            Some(n) => n.to_string(),
            None => base_name(&format!("{original}__{suffix}")),
        };
        let mut alias = base.clone();
        if !used.insert(alias.clone()) {
            let prefix: String = base.chars().take(40).collect();
            alias = format!("{prefix}__h{handler}");
            let mut duplicate = 0usize;
            while !used.insert(alias.clone()) {
                duplicate += 1;
                let prefix: String = base.chars().take(20).collect();
                alias = format!("{prefix}__h{handler}_{duplicate}");
            }
        }
        let properties: serde_json::Map<String, J> = span.meta["params"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|p| p.as_str().map(|p| (p.to_string(), json!({}))))
            .collect();
        let parameters = span.meta.get("args_schema").cloned().unwrap_or_else(|| {
            if span.meta["object_param"] == true { json!({"type":"object"}) }
            else { json!({"type":"object","properties":properties,"required":span.meta["required_params"].as_array().cloned().unwrap_or_default()}) }
        });
        let description = span.meta["props"]["description"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| {
                if target.is_empty() {
                    original.clone()
                } else {
                    format!("{original} on {target}")
                }
            });
        tools.push(
            json!({"type":"function","function":{"name":alias,"description":description,"parameters":parameters}}),
        );
        span.meta["original_name"] = json!(original);
        span.meta["name"] = json!(alias);
        span.meta["projected_tool"] = json!(true);
        span.label = alias;
    }
    if !callable {
        for span in &mut spans {
            if let Some(meta) = span.meta.as_object_mut() {
                meta.remove("handler");
            }
        }
    }
    let mut result = Map::default();
    result.insert(Rc::from("tree"), Value::tokens(ids.into(), spans.into()));
    result.insert(Rc::from("tools"), Value::from_json(&J::Array(tools)));
    if !callable {
        result.insert(Rc::from("callable"), Value::Bool(false));
    }
    Ok(Value::map(result))
}
