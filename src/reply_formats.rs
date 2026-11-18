//! Reply formats beyond the `<tool_call>` family: each model family writes thinking and tool calls
//! its own way. Each parser turns a decoded reply (special tokens kept) into the parts `reply.rs`
//! judges: thinking, prose, calls, trailing text, syntax errors.
//!
//! | Syntax | Families | Calls |
//! |---|---|---|
//! | `DeepSeekV3` | DeepSeek V2.5, V3, V3-0324, R1, R1-0528 | `<｜tool▁call▁begin｜>function<｜tool▁sep｜>name\n```json\n{..}\n```<｜tool▁call▁end｜>` |
//! | `DeepSeekV31` | DeepSeek V3.1 | `<｜tool▁call▁begin｜>name<｜tool▁sep｜>{..}<｜tool▁call▁end｜>` |
//! | `Dsml` | DeepSeek V3.2, V4, V4.1 | `<｜DSML｜invoke name="x"><｜DSML｜parameter name="k" string="true">v</｜DSML｜parameter>...` |
//! | `Llama31` | Llama 3.1, 3.2 | `{"name": "x", "parameters": {..}}` (one call, optional `<\|python_tag\|>`) |
//! | `Llama4` | Llama 4 | `[x(k="v"), y(k=1)]` after any prose |
//! | `MistralList` | Mistral 7B v0.3, Nemo | `[TOOL_CALLS] [{"name": "x", "arguments": {..}}, ...]` |
//! | `MistralArgs` | Ministral 3, Small 3.2/4, Medium 3.5, Magistral | `[THINK]..[/THINK]` then `[TOOL_CALLS]x[CALL_ID]id[ARGS]{..}` repeated |
//! | `Harmony` | gpt-oss | `<\|channel\|>analysis<\|message\|>..<\|end\|><\|start\|>assistant to=functions.x<\|channel\|>commentary json<\|message\|>{..}` |
//! | `Atem` | Muse Glimmer | messages `<\|start\|>assistant to=x<\|message\|><atem:function_calls>..</atem:function_calls><\|eom\|>` |
use crate::reply::Call;
use serde_json::{Value as J, json};

/// The parts of one reply.
#[derive(Debug, Default)]
pub struct Parts {
    pub think: Option<String>,
    pub provider_state: Option<String>,
    /// False when thinking was opened and never closed.
    pub think_closed: bool,
    pub prose: String,
    pub calls: Vec<Call>,
    pub trailing: String,
    pub errors: Vec<String>,
}

/// Split `<think>`-style thinking (`open`/`close` markers) off the front of a reply. `call` is the
/// marker that opens the reply's calls.
pub fn split_think<'a>(
    text: &'a str,
    open: &str,
    close: &str,
    in_think: bool,
    call: &str,
) -> (Option<String>, bool, &'a str) {
    let t = text.trim_start();
    if in_think || t.starts_with(open) {
        let body = if in_think { text } else { &t[open.len()..] };
        return match body.find(close) {
            Some(i) => (Some(body[..i].trim().to_string()), true, &body[i + close.len()..]),
            None => (Some(body.trim().to_string()), false, ""),
        };
    }
    // A close with no open: everything before it was thinking. A close after a call opened is
    // the call's argument text.
    let before_calls = &text[..text.find(call).unwrap_or(text.len())];
    match before_calls.find(close) {
        Some(i) => (Some(text[..i].trim().to_string()), true, &text[i + close.len()..]),
        None => (None, true, text),
    }
}

/// One JSON value at the start of `s`, and the rest after it.
fn json_prefix(s: &str) -> Result<(J, &str), String> {
    let mut it = serde_json::Deserializer::from_str(s).into_iter::<J>();
    match it.next() {
        Some(Ok(v)) => Ok((v, &s[it.byte_offset()..])),
        Some(Err(e)) => Err(format!("arguments are not JSON ({e})")),
        None => Err("missing arguments".into()),
    }
}

fn args_value(v: J) -> Result<J, String> {
    match v {
        J::String(s) => serde_json::from_str(&s).map_err(|_| "arguments is a string that is not JSON".to_string()),
        J::Null => Ok(json!({})),
        v => Ok(v),
    }
}

fn tagged(text: &str, start: &str) -> Option<(String, String)> {
    text.find(start)
        .map(|i| (text[..i].trim().to_string(), text[i..].to_string()))
}

/// DeepSeek V2.5 / V3 / R1 (`with_type`) and V3.1 (without): calls between tool-call tokens.
pub fn deepseek(text: &str, with_type: bool) -> Parts {
    const BEGIN: &str = "<｜tool▁calls▁begin｜>";
    const CALL: &str = "<｜tool▁call▁begin｜>";
    const SEP: &str = "<｜tool▁sep｜>";
    const CALL_END: &str = "<｜tool▁call▁end｜>";
    const END: &str = "<｜tool▁calls▁end｜>";
    let mut p = Parts {
        think_closed: true,
        ..Default::default()
    };
    let Some((prose, rest)) = tagged(text, BEGIN).or_else(|| tagged(text, CALL)) else {
        p.prose = text.trim().to_string();
        return p;
    };
    p.prose = prose;
    let mut rest = rest.strip_prefix(BEGIN).unwrap_or(&rest).to_string();
    while let Some(i) = rest.find(CALL) {
        let after = rest[i + CALL.len()..].to_string();
        let Some(j) = after.find(CALL_END) else {
            p.errors.push(format!("`{CALL}` is never closed"));
            rest.clear();
            break;
        };
        let body = &after[..j];
        let parsed = (|| -> Result<Call, String> {
            let (head, args) = body.split_once(SEP).ok_or("tool call without a separator")?;
            let (name, args) = if with_type {
                // `function<｜tool▁sep｜>name\n```json\n{..}\n````
                let (name, rest) = args.split_once('\n').ok_or("tool call without arguments")?;
                let rest = rest
                    .trim()
                    .strip_prefix("```json")
                    .ok_or("arguments are not in a ```json block")?;
                (
                    name.trim().to_string(),
                    rest.trim()
                        .strip_suffix("```")
                        .ok_or("unclosed ```json block")?
                        .trim()
                        .to_string(),
                )
            } else {
                (head.trim().to_string(), args.trim().to_string())
            };
            if with_type && head.trim() != "function" {
                return Err(format!("unknown tool call type {:?}", head.trim()));
            }
            let (v, tail) = json_prefix(&args)?;
            if !tail.trim().is_empty() {
                return Err("text after the arguments".into());
            }
            Ok(Call {
                name,
                args: args_value(v)?,
                id: None,
            })
        })();
        match parsed {
            Ok(c) => p.calls.push(c),
            Err(e) => p.errors.push(e),
        }
        rest = after[j + CALL_END.len()..].to_string();
    }
    // The V2.5/V3 template omits the closing token after a single call; accept it either way.
    let rest = rest.trim_start();
    let rest = rest.strip_prefix(END).unwrap_or(rest);
    p.trailing = rest.trim().to_string();
    p
}

/// The DSML forms of DeepSeek V3.2 (`function_calls`), V4 (`tool_calls`) and V4.1 (` calls`, a
/// space after the DSML token). Parameters with `string="false"` are JSON.
pub fn dsml(text: &str) -> Parts {
    const D: &str = "｜DSML｜";
    let mut p = Parts {
        think_closed: true,
        ..Default::default()
    };
    let open = ["function_calls>", "tool_calls>", " calls>"]
        .iter()
        .find_map(|b| text.find(&format!("<{D}{b}")).map(|i| (i, *b)));
    let Some((start, block)) = open else {
        p.prose = text.trim().to_string();
        return p;
    };
    p.prose = text[..start].trim().to_string();
    let body = &text[start + 1 + D.len() + block.len()..];
    let close = format!("</{D}{block}");
    let (inner, trailing) = match body.find(&close) {
        Some(i) => (&body[..i], &body[i + close.len()..]),
        None => {
            p.errors.push("the tool-call block is never closed".into());
            (body, "")
        }
    };
    p.trailing = trailing.trim().to_string();
    let sp = if block.starts_with(' ') { " " } else { "" };
    let invoke = format!("<{D}{sp}invoke name=\"");
    let invoke_end = format!("</{D}{sp}invoke>");
    let param = format!("<{D}{sp}parameter name=\"");
    let param_end = format!("</{D}{sp}parameter>");
    let mut rest = inner;
    while let Some(i) = rest.find(&invoke) {
        let after = &rest[i + invoke.len()..];
        let Some(q) = after.find("\">") else {
            p.errors.push("bad invoke header".into());
            break;
        };
        let name = after[..q].to_string();
        let Some(e) = after.find(&invoke_end) else {
            p.errors.push(format!("invoke of {name} is never closed"));
            break;
        };
        // A malformed header can close after its invoke end tag; reject before slicing.
        if q + 2 > e {
            p.errors.push(format!("malformed invoke of {name}"));
            break;
        }
        let mut params = &after[q + 2..e];
        let mut args = serde_json::Map::new();
        let mut ok = true;
        while let Some(k) = params.find(&param) {
            let a = &params[k + param.len()..];
            let Some(gt) = a.find('>') else {
                ok = false;
                break;
            };
            let head = &a[..gt];
            let key = head.split('"').next().unwrap_or("").to_string();
            let is_string = !head.contains("string=\"false\"");
            let Some(ve) = a[gt + 1..].find(&param_end) else {
                ok = false;
                break;
            };
            let raw = &a[gt + 1..gt + 1 + ve];
            let value = if is_string {
                J::String(raw.to_string())
            } else {
                serde_json::from_str(raw.trim()).unwrap_or_else(|_| J::String(raw.to_string()))
            };
            args.insert(key, value);
            params = &a[gt + 1 + ve + param_end.len()..];
        }
        // Arguments the template could not split into parameters arrive as one `arguments`
        // parameter holding the whole object (JSON, or JSON text).
        if args.len() == 1 {
            let whole = match args.get("arguments") {
                Some(J::Object(o)) => Some(J::Object(o.clone())),
                Some(J::String(s)) => serde_json::from_str::<J>(s).ok().filter(J::is_object),
                _ => None,
            };
            if let Some(w) = whole {
                args = w.as_object().unwrap().clone();
            }
        }
        if ok {
            p.calls.push(Call {
                name,
                args: J::Object(args),
                id: None,
            })
        } else {
            p.errors.push(format!("bad parameters in the call to {name}"))
        }
        rest = &after[e + invoke_end.len()..];
    }
    p
}

/// Llama 3.1/3.2: the whole reply is one JSON call (`parameters`), optionally after `<|python_tag|>`.
/// A hosted chat API's reply, as `runtime::model::Chat` writes it:
/// `{"reasoning": ..., "content": ..., "tool_calls": [{"name": ..., "arguments": {...}}]}`.
pub fn chat(text: &str) -> Parts {
    let mut p = Parts {
        think_closed: true,
        ..Default::default()
    };
    let v: J = match serde_json::from_str(text.trim()) {
        Ok(v) => v,
        Err(e) => {
            p.errors.push(format!("chat reply is not JSON: {e}"));
            return p;
        }
    };
    p.think = v["reasoning"].as_str().filter(|s| !s.is_empty()).map(str::to_owned);
    p.provider_state = v["provider_state"].as_str().map(str::to_owned);
    p.prose = v["content"].as_str().unwrap_or_default().to_string();
    for c in v["tool_calls"].as_array().into_iter().flatten() {
        match args_value(c["arguments"].clone()) {
            Ok(args) => p.calls.push(Call {
                name: c["name"].as_str().unwrap_or_default().to_string(),
                args,
                id: c["id"].as_str().map(str::to_owned),
            }),
            Err(e) => p.errors.push(e),
        }
    }
    p
}

pub fn llama31(text: &str) -> Parts {
    let mut p = Parts {
        think_closed: true,
        ..Default::default()
    };
    let t = text.trim();
    let body = t.strip_prefix("<|python_tag|>").unwrap_or(t).trim();
    if !body.starts_with('{') {
        p.prose = t.to_string();
        return p;
    }
    match json_prefix(body) {
        Ok((v, tail)) if v.get("name").is_some() => {
            let args = v
                .get("parameters")
                .or_else(|| v.get("arguments"))
                .cloned()
                .unwrap_or(J::Null);
            match args_value(args) {
                Ok(args) => p.calls.push(Call {
                    name: v["name"].as_str().unwrap_or("").to_string(),
                    args,
                    id: None,
                }),
                Err(e) => p.errors.push(e),
            }
            p.trailing = tail.trim().to_string();
        }
        Ok(_) => p.prose = t.to_string(),
        Err(e) => p.errors.push(format!("tool call: {e}")),
    }
    p
}

/// Llama 4: `[name(k="v", n=1), other()]` at the end of the reply, after any prose.
pub fn llama4(text: &str) -> Parts {
    let mut p = Parts {
        think_closed: true,
        ..Default::default()
    };
    let t = text.trim_end();
    let t = t.strip_suffix("<|python_end|>").unwrap_or(t);
    // The call list is the first `[ident(` that parses to the end of the reply; one mentioned in
    // the prose before it is not. When none does, the first is judged as written.
    let starts: Vec<usize> = t
        .char_indices()
        .filter(|(_, c)| *c == '[')
        .map(|(i, _)| i)
        .filter(|&i| {
            let r = t[i + 1..].trim_start();
            let name_len = r
                .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '.'))
                .unwrap_or(r.len());
            name_len > 0 && r[name_len..].starts_with('(')
        })
        .collect();
    let Some(&first) = starts.first() else {
        p.prose = t.trim().to_string();
        return p;
    };
    let (start, parsed) = starts
        .iter()
        .map(|&i| (i, pythonic_calls(&t[i..])))
        .find(|(_, r)| r.as_ref().is_ok_and(|(_, tail)| tail.trim().is_empty()))
        .unwrap_or_else(|| (first, pythonic_calls(&t[first..])));
    p.prose = t[..start]
        .trim()
        .trim_end_matches("<|python_start|>")
        .trim()
        .to_string();
    match parsed {
        Ok((calls, tail)) => (p.calls, p.trailing) = (calls, tail.trim().to_string()),
        Err(e) => p.errors.push(e),
    }
    p
}

/// `[f(a="x", b=1), g()]`: names, keyword arguments with Python literals; and the text after the
/// list.
fn pythonic_calls(s: &str) -> Result<(Vec<Call>, String), String> {
    let chars: Vec<char> = s.trim().chars().collect();
    let mut i = 1;
    let mut calls = vec![];
    let skip = |i: &mut usize| {
        while *i < chars.len() && chars[*i].is_whitespace() {
            *i += 1
        }
    };
    loop {
        skip(&mut i);
        if i < chars.len() && chars[i] == ']' {
            break;
        }
        let ns = i;
        while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '.') {
            i += 1
        }
        let name: String = chars[ns..i].iter().collect();
        if name.is_empty() || i >= chars.len() || chars[i] != '(' {
            return Err("expected name(...) in the call list".into());
        }
        i += 1;
        let mut args = serde_json::Map::new();
        loop {
            skip(&mut i);
            if i < chars.len() && chars[i] == ')' {
                i += 1;
                break;
            }
            let ks = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1
            }
            let key: String = chars[ks..i].iter().collect();
            skip(&mut i);
            if key.is_empty() || i >= chars.len() || chars[i] != '=' {
                return Err(format!("expected key=value in the call to {name}"));
            }
            i += 1;
            skip(&mut i);
            let (value, n) =
                py_literal(&chars[i..]).ok_or_else(|| format!("bad value for `{key}` in the call to {name}"))?;
            args.insert(key, value);
            i += n;
            skip(&mut i);
            if i < chars.len() && chars[i] == ',' {
                i += 1
            }
        }
        calls.push(Call {
            name,
            args: J::Object(args),
            id: None,
        });
        skip(&mut i);
        if i < chars.len() && chars[i] == ',' {
            i += 1
        }
    }
    Ok((calls, chars[i + 1..].iter().collect()))
}

/// A Python literal written as text (`{'a': [1, True]}`) as JSON, if it is one.
pub fn python_value(s: &str) -> Option<J> {
    let chars: Vec<char> = s.trim().chars().collect();
    py_literal(&chars).filter(|(_, n)| *n == chars.len()).map(|(v, _)| v)
}

/// Literals nested deeper than this are not ones a model wrote (serde_json's limit).
const PY_DEPTH: usize = 128;

/// A Python literal (string, number, True/False/None, list, tuple, dict) and its length in chars.
fn py_literal(c: &[char]) -> Option<(J, usize)> {
    py_nested(c, PY_DEPTH)
}

fn py_nested(c: &[char], depth: usize) -> Option<(J, usize)> {
    let first = *c.first()?;
    if first == '"' || first == '\'' {
        return py_str(c);
    }
    if let Some(close) = match first {
        '[' => Some(']'),
        '(' => Some(')'),
        '{' => Some('}'),
        _ => None,
    } {
        let depth = depth.checked_sub(1)?;
        let skip = |mut i: usize| {
            while c.get(i).is_some_and(|ch| ch.is_whitespace()) {
                i += 1
            }
            i
        };
        let (mut items, mut dict, mut commas, mut i) = (vec![], serde_json::Map::new(), 0, skip(1));
        while *c.get(i)? != close {
            let (v, n) = py_nested(&c[i..], depth)?;
            i = skip(i + n);
            if first == '{' {
                // Keys are strings, as in JSON.
                let (J::String(k), ':') = (v, *c.get(i)?) else {
                    return None;
                };
                i = skip(i + 1);
                let (v, n) = py_nested(&c[i..], depth)?;
                dict.insert(k, v);
                i = skip(i + n);
            } else {
                items.push(v);
            }
            match *c.get(i)? {
                ',' => {
                    commas += 1;
                    i = skip(i + 1)
                }
                ch if ch == close => {}
                _ => return None,
            }
        }
        let v = match first {
            '{' => J::Object(dict),
            // `(x)` is x; a tuple has a comma.
            '(' if items.len() == 1 && commas == 0 => items.remove(0),
            _ => J::Array(items),
        };
        return Some((v, i + 1));
    }
    let end = c
        .iter()
        .position(|ch| matches!(ch, ',' | ':' | ')' | ']' | '}'))
        .unwrap_or(c.len());
    let word: String = c[..end].iter().collect::<String>().trim().to_string();
    let v = match word.as_str() {
        "True" => json!(true),
        "False" => json!(false),
        "None" => J::Null,
        w => serde_json::from_str(w).ok()?,
    };
    Some((v, end))
}

/// A quoted Python string with its escapes, and its length in chars.
fn py_str(c: &[char]) -> Option<(J, usize)> {
    let (quote, mut s, mut i) = (c[0], String::new(), 1);
    loop {
        let ch = *c.get(i)?;
        i += 1;
        if ch == quote {
            return Some((J::String(s), i));
        }
        if ch != '\\' {
            s.push(ch);
            continue;
        }
        let e = *c.get(i)?;
        i += 1;
        match e {
            'n' => s.push('\n'),
            't' => s.push('\t'),
            'r' => s.push('\r'),
            'a' => s.push('\x07'),
            'b' => s.push('\x08'),
            'f' => s.push('\x0c'),
            'v' => s.push('\x0b'),
            '\\' | '\'' | '"' => s.push(e),
            'x' | 'u' | 'U' => {
                let hex = c.get(
                    i..i + match e {
                        'x' => 2,
                        'u' => 4,
                        _ => 8,
                    },
                )?;
                if !hex.iter().all(char::is_ascii_hexdigit) {
                    return None;
                }
                s.push(char::from_u32(
                    u32::from_str_radix(&hex.iter().collect::<String>(), 16).ok()?,
                )?);
                i += hex.len();
            }
            // Python keeps the backslash of an escape it does not know.
            _ => {
                s.push('\\');
                s.push(e)
            }
        }
    }
}

/// Mistral 7B v0.3 / Nemo: `[TOOL_CALLS] [{"name", "arguments", "id"?}, ...]`.
pub fn mistral_list(text: &str) -> Parts {
    let mut p = Parts {
        think_closed: true,
        ..Default::default()
    };
    let Some((prose, rest)) = tagged(text, "[TOOL_CALLS]") else {
        p.prose = text.trim().to_string();
        return p;
    };
    p.prose = prose;
    match json_prefix(rest["[TOOL_CALLS]".len()..].trim_start()) {
        Ok((J::Array(items), tail)) => {
            for v in items {
                match (
                    v["name"].as_str(),
                    args_value(v.get("arguments").cloned().unwrap_or(J::Null)),
                ) {
                    (Some(n), Ok(args)) => p.calls.push(Call {
                        name: n.to_string(),
                        args,
                        id: v["id"].as_str().map(str::to_owned),
                    }),
                    (None, _) => p.errors.push("tool call without a name".into()),
                    (_, Err(e)) => p.errors.push(e),
                }
            }
            p.trailing = tail.trim().to_string();
        }
        Ok(_) => p.errors.push("[TOOL_CALLS] must be followed by a JSON list".into()),
        Err(e) => p.errors.push(format!("tool calls: {e}")),
    }
    p
}

/// Ministral 3, Mistral Small 3.2 / 4, Medium 3.5, Magistral: `[THINK]..[/THINK]`, prose, then
/// `[TOOL_CALLS]name([CALL_ID]id)?[ARGS]{json}` per call.
pub fn mistral_args(text: &str, in_think: bool) -> Parts {
    let (think, closed, rest) = split_think(text, "[THINK]", "[/THINK]", in_think, "[TOOL_CALLS]");
    let mut p = Parts {
        think,
        think_closed: closed,
        ..Default::default()
    };
    let Some((prose, mut rest)) = tagged(rest, "[TOOL_CALLS]") else {
        p.prose = rest.trim().to_string();
        return p;
    };
    p.prose = prose;
    while let Some(after) = rest.strip_prefix("[TOOL_CALLS]") {
        let Some(a) = after.find("[ARGS]") else {
            p.errors.push("tool call without [ARGS]".into());
            rest = String::new();
            break;
        };
        let mut head = after[..a].split("[CALL_ID]");
        let name = head.next().unwrap_or("").trim().to_string();
        let id = head.next().map(|i| i.trim().to_string()).filter(|i| !i.is_empty());
        match json_prefix(&after[a + "[ARGS]".len()..]) {
            Ok((v, tail)) => {
                match args_value(v) {
                    Ok(args) => p.calls.push(Call { name, args, id }),
                    Err(e) => p.errors.push(e),
                }
                rest = tail.trim_start().to_string();
            }
            Err(e) => {
                p.errors.push(format!("{name}: {e}"));
                rest = String::new();
                break;
            }
        }
    }
    p.trailing = rest.trim().to_string();
    p
}

/// One Harmony message header: `[ to=R]<|channel|>C[ to=R][ <|constrain|>json]<|message|>`.
fn harmony_header(h: &str) -> (String, Option<String>) {
    let recipient = h.find("to=").map(|i| {
        h[i + 3..]
            .split(|c: char| c.is_whitespace() || c == '<')
            .next()
            .unwrap_or("")
            .to_string()
    });
    let channel = h
        .split("<|channel|>")
        .nth(1)
        .unwrap_or("")
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_string();
    (channel, recipient)
}

/// gpt-oss: messages on channels. `analysis` is thinking, `final` the answer, a message with a
/// `to=functions.x` recipient a call. The reply continues after `<|start|>assistant`.
pub fn harmony(text: &str) -> Parts {
    let mut p = Parts {
        think_closed: true,
        ..Default::default()
    };
    let mut thinking = vec![];
    for (i, msg) in text.split("<|start|>").enumerate() {
        let msg = if i == 0 {
            msg
        } else {
            msg.strip_prefix("assistant").unwrap_or(msg)
        };
        if msg.trim().is_empty() {
            continue;
        }
        let Some((header, body)) = msg.split_once("<|message|>") else {
            p.errors.push(format!(
                "message without <|message|>: {:?}",
                msg.chars().take(60).collect::<String>()
            ));
            continue;
        };
        let (channel, recipient) = harmony_header(header);
        let ends = ["<|end|>", "<|call|>", "<|return|>"];
        let cut = ends.iter().filter_map(|e| body.find(e)).min();
        let content = &body[..cut.unwrap_or(body.len())];
        // The reply's stop token (`<|call|>`, `<|return|>`) was cut off; only an analysis message
        // without `<|end|>` is unfinished.
        let closed = cut.is_some() || i + 1 < text.split("<|start|>").count() || channel != "analysis";
        match (recipient.as_deref(), channel.as_str()) {
            (Some(r), _) if r.starts_with("functions.") || !matches!(r, "user" | "all") => {
                match serde_json::from_str::<J>(content.trim())
                    .map_err(|e| e.to_string())
                    .and_then(args_value)
                {
                    Ok(args) => p.calls.push(Call {
                        name: r.trim_start_matches("functions.").to_string(),
                        args,
                        id: None,
                    }),
                    Err(e) => p.errors.push(format!("arguments of {r}: {e}")),
                }
            }
            (_, "analysis") => {
                thinking.push(content.trim().to_string());
                if !closed {
                    p.think_closed = false
                }
            }
            (_, "final") | (_, "commentary") => {
                if !p.prose.is_empty() {
                    p.prose.push('\n')
                }
                p.prose.push_str(content.trim())
            }
            (_, other) => p.errors.push(format!("unknown channel {other:?}")),
        }
    }
    if !thinking.is_empty() {
        p.think = Some(thinking.join("\n"))
    }
    p
}

/// Muse Glimmer: messages with a recipient. `to=self` is thinking, `to=user` the answer, any
/// other recipient a call written as `<atem:function_calls>`.
pub fn atem(text: &str) -> Parts {
    let mut p = Parts {
        think_closed: true,
        ..Default::default()
    };
    let mut thinking = vec![];
    let n = text.split("<|start|>").count();
    for (i, msg) in text.split("<|start|>").enumerate() {
        let msg = if i == 0 {
            msg
        } else {
            msg.strip_prefix("assistant").unwrap_or(msg)
        };
        if msg.trim().is_empty() {
            continue;
        }
        let Some((header, body)) = msg.split_once("<|message|>") else {
            p.errors.push("message without <|message|>".into());
            continue;
        };
        let recipient = header
            .split_whitespace()
            .find_map(|w| w.strip_prefix("to="))
            .unwrap_or("user")
            .to_string();
        let cut = ["<|eom|>", "<|eot|>"].iter().filter_map(|e| body.find(e)).min();
        let closed = cut.is_some() || i + 1 < n || recipient != "self";
        let content = body[..cut.unwrap_or(body.len())].trim();
        match recipient.as_str() {
            "self" => {
                thinking.push(content.to_string());
                if !closed {
                    p.think_closed = false
                }
            }
            "user" => {
                if !p.prose.is_empty() {
                    p.prose.push('\n')
                }
                p.prose.push_str(content)
            }
            _ => {
                let Some(inner) = content
                    .strip_prefix("<atem:function_calls>")
                    .and_then(|c| c.trim_end().strip_suffix("</atem:function_calls>"))
                else {
                    p.errors
                        .push(format!("the call to {recipient} is not an <atem:function_calls> block"));
                    continue;
                };
                let mut rest = inner;
                while let Some(k) = rest.find("<atem:invoke name=\"") {
                    let a = &rest[k + "<atem:invoke name=\"".len()..];
                    let Some(q) = a.find("\">") else {
                        p.errors.push("bad atem:invoke".into());
                        break;
                    };
                    let name = a[..q].to_string();
                    let Some(e) = a.find("</atem:invoke>") else {
                        p.errors.push(format!("{name}: atem:invoke never closed"));
                        break;
                    };
                    // A malformed header can close after its invoke end tag; reject before slicing.
                    if q + 2 > e {
                        p.errors.push(format!("malformed atem:invoke of {name}"));
                        break;
                    }
                    let mut params = &a[q + 2..e];
                    let (mut args, mut ok) = (serde_json::Map::new(), true);
                    while let Some(j) = params.find("<atem:parameter name=\"") {
                        let b = &params[j + "<atem:parameter name=\"".len()..];
                        // A parameter that opens and never closes makes the call incomplete: it
                        // must not run with that argument missing.
                        let (Some(q2), Some(ve)) = (b.find("\">"), b.find("</atem:parameter>")) else {
                            ok = false;
                            break;
                        };
                        if q2 + 2 > ve {
                            ok = false;
                            break;
                        }
                        // The text as written: the action's schema gives it its type
                        // (`coerce_xml_args`), so a string parameter whose text is `true` or
                        // `{"a": 1}` stays a string.
                        let raw = &b[q2 + 2..ve];
                        args.insert(b[..q2].to_string(), J::String(raw.to_string()));
                        params = &b[ve + "</atem:parameter>".len()..];
                    }
                    if ok {
                        p.calls.push(Call {
                            name,
                            args: J::Object(args),
                            id: None,
                        })
                    } else {
                        p.errors.push(format!("malformed atem:parameter in the call to {name}"))
                    }
                    rest = &a[e + "</atem:invoke>".len()..];
                }
            }
        }
    }
    if !thinking.is_empty() {
        p.think = Some(thinking.join("\n"))
    }
    p
}

#[cfg(test)]
#[path = "reply_formats_tests.rs"]
mod tests;
