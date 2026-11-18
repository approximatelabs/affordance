//! A hosted reply as a stream (server-sent events), rebuilt into the response the provider sends
//! when it does not stream, so the reply is read exactly as a non-streamed one is
//! (`hosted::chat_reply`, `anthropic_reply`, `responses_reply`). On the way, its text and reasoning
//! are handed out as they arrive (`Delta::Text`, `Delta::Reasoning`). A stream stopped by its
//! caller becomes a response cut off at its output limit: its text is kept, its calls are not.
use super::{Api, Delta};
use serde_json::{Map, Value as J, json};

/// The response being rebuilt from a stream's events.
pub(super) enum Assembler {
    Chat(ChatStream),
    Anthropic(AnthropicStream),
    Responses(ResponsesStream),
}

#[derive(Default)]
pub(super) struct ChatStream {
    id: J,
    model: J,
    content: String,
    /// The reasoning, and the field the provider sends it in (`reasoning_content` or `reasoning`).
    reasoning: Option<(String, String)>,
    /// Tool calls by index: id, name, arguments text.
    calls: Vec<(J, String, String)>,
    finish: Option<String>,
    usage: J,
}

#[derive(Default)]
pub(super) struct AnthropicStream {
    message: Map<String, J>,
    blocks: Vec<J>,
    /// A tool_use block's input as streamed JSON text, by block index.
    inputs: Vec<String>,
}

#[derive(Default)]
pub(super) struct ResponsesStream {
    /// Finished output items, by output index.
    items: Vec<(u64, J)>,
    /// Message text not yet in a finished item.
    partial: String,
    done: Option<J>,
    id: J,
    model: J,
    /// The output indexes of the function calls begun, in order: a call's place is its position here.
    calls: Vec<u64>,
}

/// The most tool calls or content blocks one streamed reply may name. A server's index sizes a
/// list here, so it is bounded before it is used.
const MAX_PARTS: usize = 1024;

impl Assembler {
    pub(super) fn new(api: &Api) -> Self {
        match api {
            Api::Chat => Assembler::Chat(Default::default()),
            Api::Anthropic { .. } => Assembler::Anthropic(Default::default()),
            Api::Responses { .. } => Assembler::Responses(Default::default()),
        }
    }

    /// One event. False when the caller stopped the stream.
    pub(super) fn event(
        &mut self,
        name: &str,
        data: &str,
        on: &mut dyn FnMut(Delta<'_>) -> bool,
    ) -> Result<bool, String> {
        if data == "[DONE]" {
            return Ok(true);
        }
        let ev: J = serde_json::from_str(data).map_err(|e| format!("stream event is not JSON ({e})"))?;
        if ev.get("error").is_some_and(|e| !e.is_null()) && !matches!(self, Assembler::Responses(_)) {
            return Err(format!("stream error: {}", super::brief(&ev["error"].to_string())));
        }
        let mut go = true;
        let mut text = |delta: Delta<'_>, go: &mut bool| {
            if *go && !matches!(delta, Delta::Text("") | Delta::Reasoning("")) {
                *go = on(delta);
            }
        };
        match self {
            Assembler::Chat(s) => {
                if s.id.is_null() {
                    s.id = ev["id"].clone();
                    s.model = ev["model"].clone();
                }
                if !ev["usage"].is_null() {
                    s.usage = ev["usage"].clone();
                }
                let Some(choice) = ev["choices"].as_array().and_then(|c| c.first()) else {
                    return Ok(true);
                };
                let delta = &choice["delta"];
                if let Some(t) = delta["content"].as_str() {
                    s.content.push_str(t);
                    text(Delta::Text(t), &mut go);
                }
                // One of the two names a server may use, the first present: a server that sends
                // both as aliases says it once, as the reply that is not streamed reads it.
                for key in ["reasoning_content", "reasoning"] {
                    if let Some(t) = delta[key].as_str() {
                        s.reasoning
                            .get_or_insert_with(|| (String::new(), key.into()))
                            .0
                            .push_str(t);
                        text(Delta::Reasoning(t), &mut go);
                        break;
                    }
                }
                for call in delta["tool_calls"].as_array().into_iter().flatten() {
                    let i = call["index"].as_u64().unwrap_or(0) as usize;
                    if i >= MAX_PARTS {
                        return Err(format!("stream names tool call {i}: a reply holds at most {MAX_PARTS}"));
                    }
                    if s.calls.len() <= i {
                        s.calls.resize(i + 1, (J::Null, String::new(), String::new()));
                    }
                    let slot = &mut s.calls[i];
                    if call["id"].is_string() {
                        slot.0 = call["id"].clone();
                    }
                    if let Some(n) = call["function"]["name"].as_str() {
                        slot.1.push_str(n);
                    }
                    if let Some(a) = call["function"]["arguments"].as_str() {
                        slot.2.push_str(a);
                    }
                    let piece = Delta::Call {
                        index: i,
                        id: call["id"].as_str(),
                        name: call["function"]["name"].as_str(),
                        arguments: call["function"]["arguments"].as_str().unwrap_or_default(),
                    };
                    text(piece, &mut go);
                }
                if let Some(f) = choice["finish_reason"].as_str() {
                    s.finish = Some(f.into());
                }
            }
            Assembler::Anthropic(s) => match ev["type"].as_str().unwrap_or(name) {
                "message_start" => {
                    s.message = ev["message"].as_object().cloned().unwrap_or_default();
                }
                "content_block_start" => {
                    let i = ev["index"].as_u64().unwrap_or(0) as usize;
                    if i >= MAX_PARTS {
                        return Err(format!(
                            "stream names content block {i}: a reply holds at most {MAX_PARTS}"
                        ));
                    }
                    if s.blocks.len() <= i {
                        s.blocks.resize(i + 1, J::Null);
                        s.inputs.resize(i + 1, String::new());
                    }
                    // Fields are written into the block as its events arrive, so it must be an
                    // object: anything else from a server is a broken stream, not a panic.
                    if !ev["content_block"].is_object() {
                        return Err("anthropic: a content block is not an object".into());
                    }
                    s.blocks[i] = ev["content_block"].clone();
                    if s.blocks[i]["type"] == "tool_use" {
                        let piece = Delta::Call {
                            index: s.blocks[..i].iter().filter(|b| b["type"] == "tool_use").count(),
                            id: s.blocks[i]["id"].as_str(),
                            name: s.blocks[i]["name"].as_str(),
                            arguments: "",
                        };
                        text(piece, &mut go);
                    }
                }
                "content_block_delta" => {
                    let i = ev["index"].as_u64().unwrap_or(0) as usize;
                    let Some(block) = s.blocks.get_mut(i) else {
                        return Err("anthropic stream: a delta before its block".into());
                    };
                    let delta = &ev["delta"];
                    let append = |block: &mut J, key: &str, more: &str| {
                        let now = block[key].as_str().unwrap_or_default().to_string();
                        block[key] = json!(now + more);
                    };
                    match delta["type"].as_str() {
                        Some("text_delta") => {
                            let t = delta["text"].as_str().unwrap_or_default();
                            append(block, "text", t);
                            text(Delta::Text(t), &mut go);
                        }
                        Some("thinking_delta") => {
                            let t = delta["thinking"].as_str().unwrap_or_default();
                            append(block, "thinking", t);
                            text(Delta::Reasoning(t), &mut go);
                        }
                        Some("signature_delta") => {
                            append(block, "signature", delta["signature"].as_str().unwrap_or_default())
                        }
                        Some("input_json_delta") => {
                            let more = delta["partial_json"].as_str().unwrap_or_default();
                            s.inputs[i].push_str(more);
                            let index = s.blocks[..i].iter().filter(|b| b["type"] == "tool_use").count();
                            text(
                                Delta::Call {
                                    index,
                                    id: None,
                                    name: None,
                                    arguments: more,
                                },
                                &mut go,
                            );
                        }
                        _ => {}
                    }
                }
                "content_block_stop" => {
                    let i = ev["index"].as_u64().unwrap_or(0) as usize;
                    if let Some(input) = s.inputs.get(i).filter(|t| !t.is_empty())
                        && s.blocks.get(i).is_some_and(J::is_object)
                    {
                        s.blocks[i]["input"] = serde_json::from_str(input)
                            .map_err(|e| format!("anthropic: malformed tool input ({e})"))?;
                    }
                }
                "message_delta" => {
                    for (k, v) in ev["delta"].as_object().into_iter().flatten() {
                        s.message.insert(k.clone(), v.clone());
                    }
                    let usage = s.message.entry("usage").or_insert_with(|| json!({}));
                    // A usage that is not an object (a server's mistake) is replaced, not indexed.
                    if !usage.is_object() {
                        *usage = json!({});
                    }
                    for (k, v) in ev["usage"].as_object().into_iter().flatten() {
                        if !v.is_null() {
                            usage[k] = v.clone();
                        }
                    }
                }
                _ => {}
            },
            Assembler::Responses(s) => match ev["type"].as_str().unwrap_or(name) {
                "response.created" | "response.in_progress" => {
                    s.id = ev["response"]["id"].clone();
                    s.model = ev["response"]["model"].clone();
                }
                "response.output_text.delta" | "response.refusal.delta" => {
                    let t = ev["delta"].as_str().unwrap_or_default();
                    s.partial.push_str(t);
                    text(Delta::Text(t), &mut go);
                }
                "response.reasoning_summary_text.delta" => {
                    text(Delta::Reasoning(ev["delta"].as_str().unwrap_or_default()), &mut go);
                }
                "response.output_item.added" if ev["item"]["type"] == "function_call" => {
                    s.calls.push(ev["output_index"].as_u64().unwrap_or(0));
                    let piece = Delta::Call {
                        index: s.calls.len() - 1,
                        id: ev["item"]["call_id"].as_str(),
                        name: ev["item"]["name"].as_str(),
                        arguments: "",
                    };
                    text(piece, &mut go);
                }
                "response.function_call_arguments.delta" => {
                    let at = ev["output_index"].as_u64().unwrap_or(0);
                    if let Some(index) = s.calls.iter().position(|&o| o == at) {
                        let piece = Delta::Call {
                            index,
                            id: None,
                            name: None,
                            arguments: ev["delta"].as_str().unwrap_or_default(),
                        };
                        text(piece, &mut go);
                    }
                }
                "response.output_item.done" => {
                    if ev["item"]["type"] == "message" {
                        s.partial.clear();
                    }
                    s.items
                        .push((ev["output_index"].as_u64().unwrap_or(0), ev["item"].clone()));
                }
                "response.completed" | "response.incomplete" | "response.failed" => {
                    s.done = Some(ev["response"].clone());
                }
                "error" => {
                    return Err(format!(
                        "responses stream error: {}",
                        super::brief(&ev.get("error").unwrap_or(&ev).to_string())
                    ));
                }
                _ => {}
            },
        }
        Ok(go)
    }

    /// The response as the provider sends it unstreamed. `stopped`: the caller stopped the stream,
    /// so the response is cut off where it was.
    pub(super) fn finish(self, stopped: bool) -> Result<J, String> {
        const ENDED: &str = "stream ended without a finish reason";
        match self {
            Assembler::Chat(s) => {
                let finish = match (stopped, s.finish) {
                    (true, _) => "length".to_string(),
                    (false, Some(f)) => f,
                    (false, None) => return Err(ENDED.into()),
                };
                let mut message = json!({"role": "assistant", "content": s.content});
                if let Some((text, key)) = s.reasoning {
                    message[key] = json!(text);
                }
                if !stopped && !s.calls.is_empty() {
                    message["tool_calls"] = s
                        .calls
                        .into_iter()
                        .map(|(id, name, args)| json!({"id": id, "type": "function", "function": {"name": name, "arguments": args}}))
                        .collect();
                }
                Ok(
                    json!({"id": s.id, "model": s.model, "choices": [{"index": 0, "message": message, "finish_reason": finish}], "usage": s.usage}),
                )
            }
            Assembler::Anthropic(s) => {
                let mut message = s.message;
                if stopped {
                    message.insert("stop_reason".into(), json!("max_tokens"));
                } else if !message.get("stop_reason").is_some_and(J::is_string) {
                    return Err(ENDED.into());
                }
                message.insert(
                    "content".into(),
                    J::Array(s.blocks.into_iter().filter(|b| !b.is_null()).collect()),
                );
                Ok(J::Object(message))
            }
            Assembler::Responses(mut s) => match s.done {
                Some(response) if !stopped || response["status"] == "completed" => Ok(response),
                _ if stopped => {
                    s.items.sort_by_key(|(i, _)| *i);
                    let mut output: Vec<J> = s.items.into_iter().map(|(_, item)| item).collect();
                    if !s.partial.is_empty() {
                        output.push(json!({"type": "message", "role": "assistant", "status": "incomplete",
                            "content": [{"type": "output_text", "text": s.partial, "annotations": []}]}));
                    }
                    Ok(json!({"id": s.id, "model": s.model, "status": "incomplete",
                        "incomplete_details": {"reason": "max_output_tokens"}, "output": output, "usage": {}}))
                }
                _ => Err(ENDED.into()),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A recorded stream through the client's own event parser (`http::SseParser`), cut into
    /// seven-byte pieces as a network would cut it, then the assembler.
    fn run(api: Api, stream: &str, stop_after: Option<usize>) -> (Result<J, String>, Vec<String>) {
        use crate::runtime::http::{HttpError, SseParser};
        let mut parser = SseParser::default();
        let mut assembler = Assembler::new(&api);
        let mut seen = vec![];
        let mut stopped = false;
        let mut failed = None;
        let pieces: Vec<&[u8]> = stream.as_bytes().chunks(7).collect();
        for (n, piece) in pieces.iter().enumerate() {
            let mut on_event = |name: &str, data: &str| -> Result<bool, HttpError> {
                let mut on = |d: Delta<'_>| {
                    seen.push(format!("{d:?}"));
                    stop_after.is_none_or(|n| seen.len() < n)
                };
                match assembler.event(name, data, &mut on) {
                    Ok(true) => Ok(true),
                    Ok(false) => {
                        stopped = true;
                        Ok(false)
                    }
                    Err(e) => {
                        failed = Some(e);
                        Ok(false)
                    }
                }
            };
            if !parser.feed(piece, n + 1 == pieces.len(), &mut on_event).unwrap() {
                break;
            }
        }
        if let Some(e) = failed {
            return (Err(e), seen);
        }
        (assembler.finish(stopped), seen)
    }

    #[test]
    fn chat_chunks_rebuild_the_completion() {
        let stream = r#"data: {"id":"c1","model":"m","choices":[{"index":0,"delta":{"role":"assistant","reasoning_content":"Think."}}]}

data: {"id":"c1","model":"m","choices":[{"index":0,"delta":{"content":"Hel"}}]}

data: {"id":"c1","model":"m","choices":[{"index":0,"delta":{"content":"lo"}}]}

data: {"id":"c1","model":"m","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"t1","type":"function","function":{"name":"ls","arguments":"{\"p"}}]}}]}

data: {"id":"c1","model":"m","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"\":1}"}}]},"finish_reason":"tool_calls"}]}

data: {"id":"c1","model":"m","choices":[],"usage":{"prompt_tokens":3,"completion_tokens":4}}

data: [DONE]

"#;
        let (resp, seen) = run(Api::Chat, stream, None);
        let resp = resp.unwrap();
        // The call arrives in pieces too: its id and name with the first, its arguments' text as sent.
        assert_eq!(
            seen,
            [
                "Reasoning(\"Think.\")",
                "Text(\"Hel\")",
                "Text(\"lo\")",
                "Call { index: 0, id: Some(\"t1\"), name: Some(\"ls\"), arguments: \"{\\\"p\" }",
                "Call { index: 0, id: None, name: None, arguments: \"\\\":1}\" }"
            ]
        );
        let (reply, cut_off, usage) = crate::runtime::model::hosted::chat_reply(&resp).unwrap();
        assert_eq!(
            reply,
            json!({"reasoning": "Think.", "content": "Hello", "tool_calls": [{"id": "t1", "name": "ls", "arguments": {"p": 1}}]})
        );
        assert!(!cut_off);
        assert_eq!(usage["completion_tokens"], 4);
        // Stopped in the middle: the text so far, cut off, with no call.
        let (resp, _) = run(Api::Chat, stream, Some(2));
        let (reply, cut_off, _) = crate::runtime::model::hosted::chat_reply(&resp.unwrap()).unwrap();
        assert_eq!(
            reply,
            json!({"reasoning": "Think.", "content": "Hel", "tool_calls": []})
        );
        assert!(cut_off);
        // A stream that ends early is an error, never a reply.
        let cut = stream
            .split("data: [DONE]")
            .next()
            .unwrap()
            .replace(r#","finish_reason":"tool_calls""#, "");
        assert!(run(Api::Chat, &cut, None).0.is_err());
    }

    #[test]
    fn anthropic_events_rebuild_the_message() {
        let stream = r#"event: message_start
data: {"type":"message_start","message":{"id":"msg_1","type":"message","role":"assistant","model":"claude","content":[],"stop_reason":null,"usage":{"input_tokens":10,"output_tokens":1}}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":"","signature":""}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"Hmm."}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"signature_delta","signature":"sig"}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

event: content_block_start
data: {"type":"content_block_start","index":1,"content_block":{"type":"text","text":""}}

event: content_block_delta
data: {"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"Sure."}}

event: content_block_stop
data: {"type":"content_block_stop","index":1}

event: content_block_start
data: {"type":"content_block_start","index":2,"content_block":{"type":"tool_use","id":"tu_1","name":"ls","input":{}}}

event: content_block_delta
data: {"type":"content_block_delta","index":2,"delta":{"type":"input_json_delta","partial_json":"{\"p\":"}}

event: content_block_delta
data: {"type":"content_block_delta","index":2,"delta":{"type":"input_json_delta","partial_json":"1}"}}

event: content_block_stop
data: {"type":"content_block_stop","index":2}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"tool_use","stop_sequence":null},"usage":{"output_tokens":20}}

event: message_stop
data: {"type":"message_stop"}

"#;
        let (resp, seen) = run(Api::Anthropic { cache_ttl: "5m".into() }, stream, None);
        let resp = resp.unwrap();
        assert_eq!(
            seen,
            [
                "Reasoning(\"Hmm.\")",
                "Text(\"Sure.\")",
                "Call { index: 0, id: Some(\"tu_1\"), name: Some(\"ls\"), arguments: \"\" }",
                "Call { index: 0, id: None, name: None, arguments: \"{\\\"p\\\":\" }",
                "Call { index: 0, id: None, name: None, arguments: \"1}\" }"
            ]
        );
        assert_eq!(
            resp["content"][0],
            json!({"type":"thinking","thinking":"Hmm.","signature":"sig"})
        );
        assert_eq!(resp["content"][2]["input"], json!({"p": 1}));
        assert_eq!(resp["usage"], json!({"input_tokens": 10, "output_tokens": 20}));
        let (reply, cut_off, _) = crate::runtime::model::hosted::anthropic_reply(&resp, "claude").unwrap();
        assert_eq!(reply["content"], "Sure.");
        assert_eq!(
            reply["tool_calls"],
            json!([{"id": "tu_1", "name": "ls", "arguments": {"p": 1}}])
        );
        assert!(!cut_off);
        // Stopped after the thinking: no text, the signed thinking kept, cut off.
        let (resp, _) = run(Api::Anthropic { cache_ttl: "5m".into() }, stream, Some(1));
        let (reply, cut_off, _) = crate::runtime::model::hosted::anthropic_reply(&resp.unwrap(), "claude").unwrap();
        assert!(cut_off);
        assert_eq!(reply["reasoning"], "Hmm.");
        assert_eq!(reply["tool_calls"], json!([]));
    }

    #[test]
    fn responses_events_end_in_the_response() {
        let done = json!({"id":"r1","model":"gpt","status":"completed","output":[{"type":"message","role":"assistant","content":[{"type":"output_text","text":"Hi there","annotations":[]}]}],"usage":{"input_tokens":3,"output_tokens":2}});
        let stream = format!(
            "event: response.created\ndata: {}\n\nevent: response.output_text.delta\ndata: {}\n\nevent: response.output_text.delta\ndata: {}\n\nevent: response.completed\ndata: {}\n\n",
            json!({"type":"response.created","response":{"id":"r1","model":"gpt","status":"in_progress"}}),
            json!({"type":"response.output_text.delta","delta":"Hi"}),
            json!({"type":"response.output_text.delta","delta":" there"}),
            json!({"type":"response.completed","response":done}),
        );
        let api = || Api::Responses { strict_tools: false };
        let (resp, seen) = run(api(), &stream, None);
        assert_eq!(resp.unwrap(), done);
        assert_eq!(seen, ["Text(\"Hi\")", "Text(\" there\")"]);
        let (resp, _) = run(api(), &stream, Some(1));
        let (reply, cut_off, _) = crate::runtime::model::hosted::responses_reply(&resp.unwrap(), "gpt").unwrap();
        assert!(cut_off);
        assert_eq!(reply["content"], "Hi");
    }
    #[test]
    fn a_responses_function_call_streams_in_pieces() {
        let stream = r#"
data: {"type":"response.created","response":{"id":"r1","model":"m"}}

data: {"type":"response.output_item.added","output_index":1,"item":{"type":"function_call","call_id":"c1","name":"ls","arguments":""}}

data: {"type":"response.function_call_arguments.delta","output_index":1,"delta":"{\"p\":"}

data: {"type":"response.function_call_arguments.delta","output_index":1,"delta":"1}"}

"#;
        let (_, seen) = run(Api::Responses { strict_tools: false }, stream, None);
        assert_eq!(
            seen,
            [
                "Call { index: 0, id: Some(\"c1\"), name: Some(\"ls\"), arguments: \"\" }",
                "Call { index: 0, id: None, name: None, arguments: \"{\\\"p\\\":\" }",
                "Call { index: 0, id: None, name: None, arguments: \"1}\" }"
            ]
        );
    }
}
