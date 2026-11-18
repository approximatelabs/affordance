//! Hosted protocol conversion. No I/O or mutable conversation state.
use serde_json::{Value as J, json};

/// Inspect before normalization: even a truncated call violates a passive request.
pub(super) fn has_tool_call(response: &J, api: &super::Api) -> bool {
    match api {
        super::Api::Chat => response["choices"].as_array().into_iter().flatten().any(|c| {
            c["message"]["tool_calls"]
                .as_array()
                .is_some_and(|calls| !calls.is_empty())
                || c["message"].get("function_call").is_some_and(|call| !call.is_null())
                || matches!(c["finish_reason"].as_str(), Some("tool_calls" | "function_call"))
        }),
        super::Api::Anthropic { .. } => {
            response["stop_reason"] == "tool_use"
                || response["content"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .any(|b| matches!(b["type"].as_str(), Some("tool_use" | "server_tool_use")))
        }
        super::Api::Responses { .. } => response["output"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|item| item["type"].as_str().is_some_and(|kind| kind.ends_with("_call"))),
    }
}

/// The OpenAI message list from `MessagesApp`'s turns: assistant tool calls get the ids of
/// the tool results that follow them and string arguments.
pub(super) fn messages(doc: &J) -> Vec<J> {
    let turns = doc["messages"].as_array().cloned().unwrap_or_default();
    let mut out = vec![];
    for (i, t) in turns.iter().enumerate() {
        let mut m = json!({"role": t["role"], "content": t["content"].clone()});
        if m["content"].is_null() {
            m["content"] = json!("");
        }
        match t["role"].as_str() {
            Some("assistant") if t["tool_calls"].as_array().is_some_and(|c| !c.is_empty()) => {
                m["tool_calls"] = tool_calls(&turns, i)
                    .into_iter()
                    .map(|(id, name, args)| {
                        let args = match args {
                            J::String(s) => s,
                            a => a.to_string(),
                        };
                        json!({"id": id, "type": "function", "function": {"name": name, "arguments": args}})
                    })
                    .collect();
            }
            Some("tool") => {
                m["tool_call_id"] = t["tool_call_id"].clone();
            }
            _ => {}
        }
        out.push(m);
    }
    out
}

/// The Anthropic Messages fields (`system`, `messages`, `tools`) from `MessagesApp`'s turns.
/// The leading system turn is the top-level `system`; a later one is user text. Tool results
/// are `tool_result` blocks in the next user message, paired by id with the calls before them.
/// Signed thinking and all native blocks come from the opaque provider state when present.
pub(super) fn anthropic_messages(doc: &J, cache_ttl: &str, model: &str) -> Result<J, String> {
    let turns = doc["messages"].as_array().map(Vec::as_slice).unwrap_or_default();
    let (mut system, mut messages) = (vec![], Vec::<J>::new());
    // The last block before the current turn: the history every later turn repeats.
    let mut stable = None;
    let (mut caller, mut k) = (0, 0);
    for (i, t) in turns.iter().enumerate() {
        if i + 1 == turns.len() {
            stable = messages
                .len()
                .checked_sub(1)
                .map(|m| (m, messages[m]["content"].as_array().unwrap().len() - 1));
        }
        let (role, content) = match t["role"].as_str() {
            Some("system") if messages.is_empty() => {
                system.extend(blocks(&t["content"]));
                continue;
            }
            Some("assistant") => {
                (caller, k) = (i, 0);
                let content = if let Some(native) = native_state(t, "anthropic", model, "content")? {
                    native
                } else {
                    let mut content = blocks(&t["content"]);
                    for (id, name, args) in tool_calls(turns, i) {
                        let input = match args {
                            J::String(s) => serde_json::from_str(&s)
                                .map_err(|e| format!("anthropic: malformed tool arguments ({e})"))?,
                            a => a,
                        };
                        content.push(json!({"type": "tool_use", "id": id, "name": name, "input": input}));
                    }
                    content
                };
                ("assistant", content)
            }
            Some("tool") => {
                let id = t["tool_call_id"]
                    .as_str()
                    .map_or_else(|| format!("call_{caller}_{k}"), str::to_owned);
                k += 1;
                let mut result = json!({"type": "tool_result", "tool_use_id": id});
                let content = blocks(&t["content"]);
                if !content.is_empty() {
                    result["content"] = J::Array(content);
                }
                if t["error"] == true {
                    result["is_error"] = json!(true);
                }
                ("user", vec![result])
            }
            _ => ("user", blocks(&t["content"])),
        };
        // The API rejects empty messages and requires roles to alternate.
        if content.is_empty() {
            continue;
        }
        match messages.last_mut() {
            Some(m) if m["role"] == role => m["content"].as_array_mut().unwrap().extend(content),
            _ => messages.push(json!({"role": role, "content": content})),
        }
    }
    let tools: Vec<J> = doc["tools"].as_array().into_iter().flatten().map(|t| {
        let f = &t["function"];
        let mut tool = json!({"name": f["name"], "input_schema": if f["parameters"].is_object() { f["parameters"].clone() } else { json!({"type": "object"}) }});
        if f["description"].is_string() { tool["description"] = f["description"].clone(); }
        tool
    }).collect();
    let mark = json!({"type": "ephemeral", "ttl": cache_ttl});
    let mut out = json!({"messages": messages});
    if let Some((m, b)) = stable {
        'outer: for mi in (0..=m).rev() {
            let len = out["messages"][mi]["content"].as_array().unwrap().len();
            for bi in (0..if mi == m { b + 1 } else { len }).rev() {
                if matches!(
                    out["messages"][mi]["content"][bi]["type"].as_str(),
                    Some("text" | "image" | "tool_use" | "tool_result")
                ) {
                    out["messages"][mi]["content"][bi]["cache_control"] = mark.clone();
                    break 'outer;
                }
            }
        }
    }
    if !system.is_empty() {
        system.last_mut().unwrap()["cache_control"] = mark.clone();
        out["system"] = J::Array(system);
    }
    if !tools.is_empty() {
        out["tools"] = J::Array(tools);
        out["tools"].as_array_mut().unwrap().last_mut().unwrap()["cache_control"] = mark;
    }
    Ok(out)
}
/// An assistant turn's tool calls as `(id, name, arguments)`. A call takes the id of the tool
/// result at its position after the turn, or `call_{turn}_{position}`.
fn tool_calls(turns: &[J], i: usize) -> Vec<(String, J, J)> {
    let ids: Vec<&J> = turns[i + 1..]
        .iter()
        .take_while(|n| n["role"] == "tool")
        .map(|n| &n["tool_call_id"])
        .collect();
    turns[i]["tool_calls"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
        .map(|(k, c)| {
            let id = c["id"]
                .as_str()
                .or_else(|| ids.get(k).and_then(|id| id.as_str()))
                .map_or_else(|| format!("call_{i}_{k}"), str::to_owned);
            (id, c["function"]["name"].clone(), c["function"]["arguments"].clone())
        })
        .collect()
}

/// Anthropic content blocks from `MessagesApp` content: a string, or text and `image_url` parts.
/// Empty text is dropped (the API rejects it); images are data URLs (`images`), sent as base64.
fn blocks(content: &J) -> Vec<J> {
    let text = |t: &str| (!t.is_empty()).then(|| json!({"type": "text", "text": t}));
    match content {
        J::String(s) => text(s).into_iter().collect(),
        J::Array(parts) => parts
            .iter()
            .filter_map(|p| match p["type"].as_str() {
                Some("text") => text(p["text"].as_str().unwrap_or_default()),
                Some("image_url") => {
                    let url = p["image_url"]["url"].as_str().unwrap_or_default();
                    let (media_type, data) = url
                        .strip_prefix("data:")
                        .and_then(|u| u.split_once(";base64,"))
                        .unwrap_or_default();
                    Some(json!({"type": "image", "source": {"type": "base64", "media_type": media_type, "data": data}}))
                }
                _ => Some(p.clone()),
            })
            .collect(),
        _ => vec![],
    }
}

/// Every image, audio and video part of the turns as the data URL a request may carry
/// (`agent::image_source`, `agent::clip_source`): a provider never fetches a URL for an agent.
pub(super) fn images(doc: &mut J) -> Result<(), String> {
    for turn in doc["messages"].as_array_mut().into_iter().flatten() {
        for part in turn["content"].as_array_mut().into_iter().flatten() {
            match part["type"].as_str() {
                Some("image_url") => {
                    let url = &mut part["image_url"]["url"];
                    *url = J::String(crate::runtime::agent::image_source(url.as_str().unwrap_or_default())?);
                }
                Some(kind @ ("audio_url" | "video_url")) => {
                    let url = part[kind]["url"].as_str().unwrap_or_default();
                    crate::runtime::agent::clip_source(url, &kind[..5])?;
                }
                _ => {}
            }
        }
    }
    Ok(())
}

/// An Anthropic Messages response as the chat reply JSON, whether it was cut off, and its usage
/// with the chat client's fields added: `prompt_tokens` (all input, cached or not),
/// `prompt_tokens_details.cached_tokens` (cache reads) and `completion_tokens`.
pub(super) fn anthropic_reply(resp: &J, model: &str) -> Result<(J, bool, J), String> {
    let content = resp["content"]
        .as_array()
        .ok_or("anthropic: response has no content array")?;
    let (mut reasoning, mut text, mut calls) = (vec![], vec![], vec![]);
    for b in content {
        match b["type"].as_str() {
            Some("text") => text.push(b["text"].as_str().unwrap_or_default()),
            Some("thinking") => reasoning.push(b["thinking"].as_str().unwrap_or_default()),
            Some("tool_use") => {
                let name = b["name"]
                    .as_str()
                    .filter(|n| !n.is_empty())
                    .ok_or("anthropic: tool_use has no name")?;
                let id = b["id"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .ok_or("anthropic: tool_use has no id")?;
                calls.push(json!({"id": id, "name": name, "arguments": b["input"]}));
            }
            _ => {}
        }
    }
    let stop = resp["stop_reason"]
        .as_str()
        .ok_or("anthropic: response has no stop_reason")?;
    let mut usage = resp["usage"].clone();
    if let Some(u) = usage.as_object_mut() {
        let n = |k: &str| u.get(k).and_then(J::as_u64).unwrap_or(0);
        let read = n("cache_read_input_tokens");
        let input = n("input_tokens") + n("cache_creation_input_tokens") + read;
        let output = n("output_tokens");
        let write = n("cache_creation_input_tokens");
        u.insert("prompt_tokens".into(), json!(input));
        u.insert("completion_tokens".into(), json!(output));
        u.insert(
            "prompt_tokens_details".into(),
            json!({"cached_tokens": read, "cache_write_tokens": write}),
        );
        if let Some(mut details) = u.get("output_tokens_details").and_then(J::as_object).cloned() {
            if let Some(thinking) = details.get("thinking_tokens").cloned() {
                details.entry("reasoning_tokens").or_insert(thinking);
            }
            u.insert("completion_tokens_details".into(), J::Object(details));
        }
    }
    let reasoning: Vec<_> = reasoning.into_iter().filter(|r| !r.is_empty()).collect();
    let cut_off = stop == "max_tokens";
    // Aborted calls are never dispatched: do not replay native tool blocks lacking results.
    let saved: Vec<_> = content
        .iter()
        // A thinking block is kept only when it was signed: one cut short has an empty
        // signature, and the API refuses it on the next call.
        .filter(|b| {
            !cut_off
                || (b["type"] != "tool_use"
                    && (b["type"] != "thinking" || b["signature"].as_str().is_some_and(|s| !s.is_empty())))
        })
        .cloned()
        .collect();
    if cut_off {
        calls.clear();
    }
    Ok((
        json!({"reasoning": reasoning.join("\n\n"), "content": text.join("\n\n"), "tool_calls": calls, "provider_state": json!({"kind":"anthropic", "model":model, "content":saved}).to_string()}),
        cut_off,
        usage,
    ))
}

/// The chat completions reply JSON, whether it was cut off, and its usage.
pub(super) fn chat_reply(resp: &J) -> Result<(J, bool, J), String> {
    let choice = resp["choices"]
        .as_array()
        .and_then(|c| c.first())
        .ok_or("chat: response has no choices")?;
    let msg = choice["message"]
        .as_object()
        .ok_or("chat: choice has no message object")?;
    let content = match msg.get("content") {
        Some(J::String(s)) => s.as_str(),
        Some(J::Null) => "",
        _ => return Err("chat: message content must be a string or null".into()),
    };
    let calls = match msg.get("tool_calls") {
        None | Some(J::Null) => vec![],
        Some(J::Array(calls)) => calls
            .iter()
            .map(|c| {
                let function = &c["function"];
                let name = function["name"]
                    .as_str()
                    .filter(|n| !n.is_empty())
                    .ok_or("chat: tool call has no function name")?;
                let args = function["arguments"]
                    .as_str()
                    .ok_or("chat: tool arguments must be JSON text")?;
                let args: J = match serde_json::from_str(args) {
                    Ok(args) => args,
                    // A reply that ran out of tokens inside a call's arguments is cut off: its
                    // calls are dropped below, as the other APIs' clients drop them.
                    Err(_) if choice["finish_reason"] == "length" => J::Null,
                    Err(e) => return Err(format!("chat: malformed tool arguments ({e})")),
                };
                let mut call = json!({"name": name, "arguments": args});
                if let Some(id) = c["id"].as_str() {
                    call["id"] = json!(id);
                }
                Ok(call)
            })
            .collect::<Result<Vec<_>, String>>()?,
        _ => return Err("chat: tool_calls must be an array".into()),
    };
    let finish = choice["finish_reason"]
        .as_str()
        .ok_or("chat: response has no finish_reason")?;
    let reasoning = msg
        .get("reasoning_content")
        .or_else(|| msg.get("reasoning"))
        .and_then(J::as_str)
        .unwrap_or_default();
    Ok((
        json!({"reasoning": reasoning, "content": content, "tool_calls": if finish == "length" { vec![] } else { calls }}),
        finish == "length",
        resp["usage"].clone(),
    ))
}

// JSON text stays opaque in interpreter state: numbers, unknown fields and signed text are
// never coerced through the app language. A deliberate model/protocol change needs a new turn.
fn native_state(turn: &J, kind: &str, model: &str, field: &str) -> Result<Option<Vec<J>>, String> {
    let Some(state) = turn.get("provider_state").filter(|s| !s.is_null()) else {
        return Ok(None);
    };
    let state: J = serde_json::from_str(state.as_str().ok_or("provider_state must be opaque JSON text")?)
        .map_err(|e| format!("invalid provider_state: {e}"))?;
    if state["kind"] != kind {
        return Ok(None);
    }
    if state["model"] != model {
        return Err(format!(
            "{kind}: provider_state belongs to a different model; discard it explicitly when changing models"
        ));
    }
    Ok(Some(
        state[field]
            .as_array()
            .ok_or_else(|| format!("{kind}: provider_state has no {field} array"))?
            .clone(),
    ))
}

/// A thinking tool continuation needs the original assistant blocks, not reconstructed prose.
pub(super) fn validate_anthropic_history(doc: &J, model: &str) -> Result<(), String> {
    if let Some(turn) = doc["messages"]
        .as_array()
        .into_iter()
        .flatten()
        .rev()
        .find(|t| t["role"] == "assistant")
        && turn["tool_calls"].as_array().is_some_and(|calls| !calls.is_empty())
        && native_state(turn, "anthropic", model, "content")?.is_none()
    {
        return Err("anthropic: thinking tool continuation requires provider_state on the last assistant turn; retain the native model record".into());
    }
    Ok(())
}

fn response_parts(content: &J, assistant: bool) -> Result<Vec<J>, String> {
    let text = |s: &str| json!({"type": if assistant { "output_text" } else { "input_text" }, "text": s});
    match content {
        J::Null => Ok(vec![]),
        J::String(s) => Ok(if s.is_empty() { vec![] } else { vec![text(s)] }),
        J::Array(parts) => parts
            .iter()
            .map(|p| match p["type"].as_str() {
                Some("text") => Ok(text(p["text"].as_str().ok_or("responses: text part needs text")?)),
                Some("image_url") if !assistant => {
                    let mut image = json!({"type":"input_image", "image_url":p["image_url"]["url"]});
                    if let Some(detail) = p["image_url"].get("detail") {
                        image["detail"] = detail.clone();
                    }
                    Ok(image)
                }
                _ => Err("responses: unsupported content part (use text or user image_url)".into()),
            })
            .collect(),
        _ => Err("responses: content must be text or content parts".into()),
    }
}

pub(super) fn responses_messages(doc: &J, model: &str, strict: bool) -> Result<J, String> {
    let turns = doc["messages"].as_array().map(Vec::as_slice).unwrap_or_default();
    let (mut input, mut caller, mut k) = (vec![], 0, 0);
    for (i, t) in turns.iter().enumerate() {
        match t["role"].as_str() {
            Some("assistant") => {
                (caller, k) = (i, 0);
                if let Some(native) = native_state(t, "responses", model, "output")? {
                    input.extend(native);
                    continue;
                }
                let content = response_parts(&t["content"], true)?;
                if !content.is_empty() {
                    input.push(json!({"type":"message", "role":"assistant", "content":content}));
                }
                for (id, name, args) in tool_calls(turns, i) {
                    let arguments = match args {
                        J::String(s) => s,
                        v => v.to_string(),
                    };
                    input.push(json!({"type":"function_call", "call_id":id, "name":name, "arguments":arguments}));
                }
            }
            Some("tool") => {
                let id = t["tool_call_id"]
                    .as_str()
                    .map_or_else(|| format!("call_{caller}_{k}"), str::to_owned);
                k += 1;
                let output = match &t["content"] {
                    J::String(s) => json!(s),
                    J::Null => json!(""),
                    content => json!(response_parts(content, false)?),
                };
                input.push(json!({"type":"function_call_output", "call_id":id, "output":output}));
            }
            Some(role @ ("system" | "developer" | "user")) => {
                let content = response_parts(&t["content"], false)?;
                if !content.is_empty() {
                    input.push(json!({"role":if role == "system" {"developer"} else {role}, "content":content}));
                }
            }
            _ => return Err("responses: unsupported message role".into()),
        }
    }
    let tools: Vec<J> = doc["tools"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|t| {
            let mut f = t["function"].clone();
            f["type"] = json!("function");
            f["strict"] = json!(strict);
            f
        })
        .collect();
    let mut body = json!({"input":input});
    if !tools.is_empty() {
        body["tools"] = json!(tools);
    }
    Ok(body)
}

pub(super) fn responses_reply(resp: &J, model: &str) -> Result<(J, bool, J), String> {
    let status = resp["status"].as_str().ok_or("responses: response has no status")?;
    let cut_off = status == "incomplete" && resp["incomplete_details"]["reason"] == "max_output_tokens";
    if status != "completed" && !cut_off {
        return Err(format!(
            "responses: unsuccessful response status {status}: {}",
            super::brief(&resp["incomplete_details"].to_string())
        ));
    }
    let output = resp["output"]
        .as_array()
        .ok_or("responses: response has no output array")?;
    let (mut text, mut reasoning, mut calls) = (vec![], vec![], vec![]);
    for item in output {
        match item["type"].as_str() {
            Some("message") => {
                for p in item["content"].as_array().into_iter().flatten() {
                    match p["type"].as_str() {
                        Some("output_text") => text.push(p["text"].as_str().unwrap_or_default()),
                        Some("refusal") => text.push(p["refusal"].as_str().unwrap_or_default()),
                        _ => {}
                    }
                }
            }
            Some("reasoning") => {
                for p in item["summary"].as_array().into_iter().flatten() {
                    if let Some(s) = p["text"].as_str() {
                        reasoning.push(s);
                    }
                }
            }
            Some("function_call") if !cut_off => {
                let id = item["call_id"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .ok_or("responses: function_call has no call_id")?;
                let name = item["name"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .ok_or("responses: function_call has no name")?;
                let args = item["arguments"]
                    .as_str()
                    .ok_or("responses: arguments must be JSON text")?;
                let args: J =
                    serde_json::from_str(args).map_err(|e| format!("responses: malformed tool arguments ({e})"))?;
                calls.push(json!({"id":id, "name":name, "arguments":args}));
            }
            _ => {}
        }
    }
    let mut usage = resp["usage"].clone();
    if let Some(u) = usage.as_object_mut() {
        u.insert("prompt_tokens".into(), resp["usage"]["input_tokens"].clone());
        u.insert("completion_tokens".into(), resp["usage"]["output_tokens"].clone());
        u.insert(
            "prompt_tokens_details".into(),
            resp["usage"]["input_tokens_details"].clone(),
        );
        u.insert(
            "completion_tokens_details".into(),
            resp["usage"]["output_tokens_details"].clone(),
        );
    }
    let saved: Vec<_> = output
        .iter()
        .filter(|item| !cut_off || item["type"] != "function_call")
        .cloned()
        .collect();
    Ok((
        json!({"reasoning":reasoning.join("\n\n"), "content":text.join("\n\n"), "tool_calls":calls,
        "provider_state":json!({"kind":"responses", "model":model, "output":saved}).to_string()}),
        cut_off,
        usage,
    ))
}

/// Retry-After is a minimum, even when it exceeds the per-attempt request timeout. OpenAI
/// reset headers are durations; Anthropic reset headers are RFC3339 timestamps.
pub(super) fn retry_delay(after: &str, resets: &[&str]) -> Option<std::time::Duration> {
    use std::time::Duration;
    let seconds = |s: &str| {
        s.parse::<f64>()
            .ok()
            .filter(|n| n.is_finite() && *n >= 0.0)
            .and_then(|n| Duration::try_from_secs_f64(n).ok())
    };
    let date = |s: &str| {
        chrono::DateTime::parse_from_rfc2822(s)
            .or_else(|_| chrono::DateTime::parse_from_rfc3339(s))
            .ok()
            .map(|d| {
                (d.with_timezone(&chrono::Utc) - chrono::Utc::now())
                    .to_std()
                    .unwrap_or_default()
            })
    };
    let after = seconds(after).or_else(|| date(after));
    fn duration(mut s: &str) -> Option<Duration> {
        let mut total = 0.0;
        if s.is_empty() {
            return None;
        }
        while !s.is_empty() {
            let n = s.bytes().take_while(|b| b.is_ascii_digit() || *b == b'.').count();
            if n == 0 {
                return None;
            }
            let value: f64 = s[..n].parse().ok()?;
            s = &s[n..];
            let (unit, factor) = [("ms", 0.001), ("h", 3600.0), ("m", 60.0), ("s", 1.0)]
                .into_iter()
                .find(|(unit, _)| s.starts_with(unit))?;
            total += value * factor;
            s = &s[unit.len()..];
        }
        Duration::try_from_secs_f64(total).ok()
    }
    resets
        .iter()
        .filter_map(|s| duration(s).or_else(|| date(s)))
        .chain(after)
        .max()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn provider_wait_forms_and_missing_native_continuation() {
        use std::time::Duration;
        assert_eq!(retry_delay("180", &[]), Some(Duration::from_secs(180)));
        assert_eq!(retry_delay("", &["6m0s", "100ms"]), Some(Duration::from_secs(360)));
        assert_eq!(retry_delay("0.25", &["6m0s"]), Some(Duration::from_secs(360)));
        assert_eq!(retry_delay("Fri, 30 Oct 2020 20:00:00 GMT", &[]), Some(Duration::ZERO));
        assert_eq!(retry_delay("invalid", &["bad"]), None);
        let doc = json!({"messages":[{"role":"assistant","tool_calls":[{}]}]});
        assert!(
            validate_anthropic_history(&doc, "model")
                .unwrap_err()
                .contains("provider_state")
        );
    }
}
