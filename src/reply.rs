//! The reply shape: every model reply is optional thinking, then exactly one action,
//! then the end. There is no reply without an action.
//!
//! ```text
//! reply  := think? action END
//! think  := "<think>" text "</think>"        (up to a token budget; early exit by closing it)
//! action := one affordance call, in the model's own tool syntax (Hermes JSON, Qwen XML, or <aff name args>)
//! ```
//!
//! This module has the set of actions a render offers (`ActionSet`, derived from the affordance
//! spans), a small JSON-schema checker, and a strict reply parser that names every way a reply can
//! break the shape.

use crate::emit::Span;
use serde_json::{Value as J, json};

/// How an action is written in the reply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Syntax {
    /// `<tool_call>\n{"name": "x", "arguments": {...}}\n</tool_call>` (Qwen3, Hermes)
    Hermes,
    /// `<tool_call>\n<function=x>\n<parameter=k>\nv\n</parameter>\n</function>\n</tool_call>` (Qwen3.5, Qwen3.6)
    Xml,
    /// `<aff x {"k": v}>` (the native form; no chat model is trained on it)
    Aff,
    /// `<tool_call>x<arg_key>k</arg_key><arg_value>v</arg_value>...</tool_call>` (GLM-4.5 .. GLM-5.3).
    /// Values are text (non-strings JSON-encoded by the template), typed by the schema like XML.
    Glm,
    /// DeepSeek V2.5 / V3 / R1: `<｜tool▁call▁begin｜>function<｜tool▁sep｜>x` + a ```json block.
    DeepSeekV3,
    /// DeepSeek V3.1: `<｜tool▁call▁begin｜>x<｜tool▁sep｜>{..}<｜tool▁call▁end｜>`.
    DeepSeekV31,
    /// DeepSeek V3.2 / V4 / V4.1 DSML invoke blocks.
    Dsml,
    /// Llama 3.1 / 3.2: one JSON object `{"name", "parameters"}`.
    Llama31,
    /// Llama 4: a Python call list `[x(k="v")]`.
    Llama4,
    /// Mistral 7B v0.3 / Nemo: `[TOOL_CALLS] [{..}, ..]`.
    MistralList,
    /// Ministral 3, Mistral Small 3.2 / 4, Medium 3.5, Magistral: `[THINK]`, `[TOOL_CALLS]x[ARGS]{..}`.
    MistralArgs,
    /// gpt-oss Harmony channels.
    Harmony,
    /// Muse Glimmer ATEM messages.
    Atem,
    /// A hosted chat API's reply as JSON: `{reasoning?, content, tool_calls: [{name, arguments}]}`
    /// (`MessagesApp`, `runtime::model::Chat`).
    Chat,
    /// A decision model's reply: one option code (`A`), which names the action the template app
    /// listed under it (the protocol span's `codes`), called with no arguments.
    Choice,
    /// The format's own way of writing a call, measured from the format (the protocol span's
    /// `calls`; `crate::reply_pattern`): for a model no parser here was written for.
    Pattern,
    /// A reply that is text and nothing else, with its thinking split off: for a model whose
    /// format has no way to write a tool call.
    Text,
}

impl Syntax {
    /// Every syntax's name, as a template app's protocol span gives it.
    pub const NAMES: [&'static str; 17] = [
        "hermes",
        "xml",
        "glm",
        "aff",
        "deepseek_v3",
        "deepseek_v31",
        "dsml",
        "llama31",
        "llama4",
        "mistral_list",
        "mistral_args",
        "harmony",
        "atem",
        "chat",
        "choice",
        "pattern",
        "text",
    ];

    /// Types text arguments by the action's schema (XML-style values).
    fn text_args(self) -> bool {
        matches!(
            self,
            Syntax::Xml | Syntax::Glm | Syntax::Atem | Syntax::Llama4 | Syntax::Pattern
        )
    }

    /// By the name a template adapter declares (`<span label="protocol" syntax="glm" .../>`).
    pub fn parse(name: &str) -> Result<Syntax, String> {
        match name {
            "hermes" => Ok(Syntax::Hermes),
            "xml" | "qwen_xml" => Ok(Syntax::Xml),
            "aff" => Ok(Syntax::Aff),
            "glm" => Ok(Syntax::Glm),
            "deepseek_v3" => Ok(Syntax::DeepSeekV3),
            "deepseek_v31" => Ok(Syntax::DeepSeekV31),
            "dsml" => Ok(Syntax::Dsml),
            "llama31" => Ok(Syntax::Llama31),
            "llama4" => Ok(Syntax::Llama4),
            "mistral_list" => Ok(Syntax::MistralList),
            "mistral_args" => Ok(Syntax::MistralArgs),
            "harmony" => Ok(Syntax::Harmony),
            "atem" => Ok(Syntax::Atem),
            "chat" => Ok(Syntax::Chat),
            "choice" => Ok(Syntax::Choice),
            "pattern" => Ok(Syntax::Pattern),
            "text" => Ok(Syntax::Text),
            other => Err(format!(
                "unknown reply syntax {other:?} (one of {})",
                Syntax::NAMES.join(", ")
            )),
        }
    }
}

/// The Hermes-style markers a reply's call and thinking are written between.
pub const TOOL_OPEN: &str = "<tool_call>";
/// Closes a call.
pub const TOOL_CLOSE: &str = "</tool_call>";
/// Opens the thinking block.
pub const THINK_OPEN: &str = "<think>";
/// Closes the thinking block.
pub const THINK_CLOSE: &str = "</think>";

#[derive(Debug, Clone)]
/// One call a reply makes.
pub struct Call {
    /// The action it names.
    pub name: String,
    /// Its arguments.
    pub args: J,
    /// The call's id when the model writes one (Mistral's `"id"` / `[CALL_ID]`): its tool result
    /// must cite it.
    pub id: Option<String>,
}

/// One action the current render offers.
#[derive(Debug, Clone)]
pub struct ActionDef {
    pub name: String,
    /// JSON schema of the arguments object.
    pub schema: J,
    /// The app declared this action (with these arguments) as its default (`<aff default ...>`).
    pub default: Option<J>,
    /// The option code a decision model answers with to call it (`Syntax::Choice`), as the
    /// template app listed it.
    pub code: Option<String>,
}

#[derive(Debug, Clone, Default)]
/// The actions a render offers: what a reply may call.
pub struct ActionSet {
    /// The actions, in document order.
    pub actions: Vec<ActionDef>,
    /// How the model's format writes a call, for a reply in `Syntax::Pattern`.
    pub pattern: Option<std::sync::Arc<crate::reply_pattern::Pattern>>,
}

impl ActionSet {
    /// The model-callable affordances of a render, by their span label (the projected tool name
    /// when the app went through `asChatTools`). Harness callbacks are not offered. Duplicate
    /// labels are one action (the harness dispatches the first).
    pub fn from_spans(spans: &[Span]) -> Result<ActionSet, String> {
        let mut actions: Vec<ActionDef> = vec![];
        for s in spans {
            if !crate::is_model_action(s) {
                continue;
            }
            if actions.iter().any(|a| a.name == s.label) {
                continue;
            }
            let props = &s.meta["props"];
            let properties: serde_json::Map<String, J> = s.meta["params"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|p| p.as_str().map(|p| (p.to_string(), json!({}))))
                .collect();
            let schema = s.meta.get("args_schema").cloned().unwrap_or_else(|| {
                if s.meta["object_param"] == true {
                    json!({"type": "object"})
                } else {
                    json!({"type": "object", "properties": properties, "required": s.meta["required_params"].as_array().cloned().unwrap_or_default()})
                }
            });
            // `default` (bare) declares the action with no arguments; `default={{k: v}}` with these arguments
            let default = match &props["default"] {
                J::Bool(true) => Some(json!({})),
                J::Object(o) => Some(J::Object(o.clone())),
                _ => None,
            };
            if let Some(args) = &default {
                validate(&schema, args)
                    .map_err(|e| format!("default arguments of `{}` do not fit its schema: {}", s.label, e))?;
            }
            actions.push(ActionDef {
                name: s.label.clone(),
                schema,
                default,
                code: None,
            });
        }
        if actions.iter().filter(|a| a.default.is_some()).count() > 1 {
            return Err("more than one affordance is declared `default`".into());
        }
        Ok(ActionSet { actions, pattern: None })
    }

    /// Take what a template app's protocol span says about reading replies: the option codes
    /// (`codes`, for `Syntax::Choice`) and how the format writes a call (`calls`, for
    /// `Syntax::Pattern`).
    pub fn with_protocol(self, meta: &J) -> Result<ActionSet, String> {
        let mut set = self.with_codes(&meta["codes"])?;
        if meta["calls"].is_object() {
            set.pattern = Some(std::sync::Arc::new(crate::reply_pattern::Pattern::derive(
                &meta["calls"],
            )?));
        }
        Ok(set)
    }

    /// Give actions the option codes a template app listed them under (`{code: action}`, its
    /// protocol span's `codes`), for a reply in `Syntax::Choice`.
    pub fn with_codes(mut self, codes: &J) -> Result<ActionSet, String> {
        let Some(codes) = codes.as_object() else {
            return Ok(self);
        };
        for (code, name) in codes {
            let name = name
                .as_str()
                .ok_or_else(|| format!("option code {code:?} names no action"))?;
            let action = self
                .actions
                .iter_mut()
                .find(|a| a.name == name)
                .ok_or_else(|| format!("option code {code:?} names {name:?}, which the render does not offer"))?;
            action.code = Some(code.clone());
        }
        Ok(self)
    }

    /// The action named `name`, if the render offers it.
    pub fn get(&self, name: &str) -> Option<&ActionDef> {
        self.actions.iter().find(|a| a.name == name)
    }
}

pub(crate) fn type_of(schema: &J) -> Option<&str> {
    match schema.get("type") {
        Some(J::String(s)) => Some(s.as_str()),
        _ => None,
    }
}

/// Every type a schema allows: its `type`, or each member of a type array.
fn types_of(schema: &J) -> Vec<&str> {
    match schema.get("type") {
        Some(J::String(s)) => vec![s.as_str()],
        Some(J::Array(a)) => a.iter().filter_map(|t| t.as_str()).collect(),
        _ => vec![],
    }
}

/// A JSON-schema subset: type, enum, properties, required, additionalProperties (false or a schema), items.
pub fn validate(schema: &J, v: &J) -> Result<(), String> {
    if let Some(e) = schema.get("enum").and_then(|e| e.as_array())
        && !e.contains(v)
    {
        return Err(format!("{} is not one of {}", v, J::Array(e.clone())));
    }
    let types = types_of(schema);
    if !types.is_empty() {
        let ok = types.iter().any(|t| match *t {
            "string" => v.is_string(),
            "integer" => v.is_i64() || v.is_u64() || v.as_f64().is_some_and(|f| f.fract() == 0.0 && !v.is_string()),
            "number" => v.is_number(),
            "boolean" => v.is_boolean(),
            "null" => v.is_null(),
            "array" => v.is_array(),
            "object" => v.is_object(),
            _ => true,
        });
        if !ok {
            return Err(format!("{} is not of type {}", v, types.join("|")));
        }
    }
    if let (Some(o), true) = (v.as_object(), types.is_empty() || types.contains(&"object")) {
        let props = schema.get("properties").and_then(|p| p.as_object());
        for r in schema
            .get("required")
            .and_then(|r| r.as_array())
            .into_iter()
            .flatten()
            .filter_map(|r| r.as_str())
        {
            if !o.contains_key(r) {
                return Err(format!("missing required argument `{}`", r));
            }
        }
        for (k, val) in o {
            match props.and_then(|p| p.get(k)) {
                Some(ps) => validate(ps, val).map_err(|e| format!("`{}`: {}", k, e))?,
                None if schema.get("additionalProperties") == Some(&J::Bool(false)) => {
                    return Err(format!("unknown argument `{}`", k));
                }
                None => {
                    if let Some(extra @ J::Object(_)) = schema.get("additionalProperties") {
                        validate(extra, val).map_err(|e| format!("`{k}`: {e}"))?;
                    }
                }
            }
        }
    }
    if let (Some(a), Some(items)) = (v.as_array(), schema.get("items")) {
        for (i, x) in a.iter().enumerate() {
            validate(items, x).map_err(|e| format!("[{}]: {}", i, e))?;
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
/// What is wrong with a reply.
pub enum Problem {
    /// No action in the reply.
    NoAction,
    /// More than one action in one reply.
    MultipleActions(usize),
    TextAfterAction,
    /// Text outside the think block before the action.
    ProseBeforeAction,
    /// The think block never closed.
    UnclosedThink,
    /// The action is not written in the syntax.
    BadSyntax(String),
    /// The action is not offered by the current render.
    UnknownAction(String),
    /// The arguments do not fit the action's schema.
    BadArgs(String),
    /// The reply hit the token limit before it was complete.
    CutOff,
}

/// A reply, parsed.
#[derive(Debug, Clone, Default)]
pub struct Reply {
    /// The thinking, if the reply has a thinking block.
    pub think: Option<String>,
    /// Opaque hosted-provider continuation state, kept out of visible reasoning.
    pub provider_state: Option<String>,
    /// The text outside the thinking and the calls.
    pub prose: String,
    /// The calls, in the order written.
    pub calls: Vec<Call>,
    /// Text after the last call.
    pub trailing: String,
    /// What is wrong with it; empty for a well-formed reply.
    pub problems: Vec<Problem>,
}

impl Reply {
    /// Optional thinking, exactly one known action with valid arguments, nothing else.
    pub fn valid(&self) -> bool {
        self.problems.is_empty() && self.calls.len() == 1
    }

    /// What the old loop would accept: a first action that exists and fits its schema, whatever surrounds it.
    pub fn lenient_valid(&self) -> bool {
        !self.calls.is_empty()
            && !self.problems.iter().any(|p| {
                matches!(
                    p,
                    Problem::UnknownAction(_) | Problem::BadArgs(_) | Problem::BadSyntax(_) | Problem::CutOff
                )
            })
    }

    /// The action to run: the first call, if it is a known one with valid arguments.
    pub fn action(&self) -> Option<&Call> {
        if self.lenient_valid() { self.calls.first() } else { None }
    }

    /// Each problem's name (`no_action`, `bad_args`, ...), for traces and outputs.
    pub fn problem_names(&self) -> Vec<&'static str> {
        self.problems
            .iter()
            .map(|p| match p {
                Problem::NoAction => "no_action",
                Problem::MultipleActions(_) => "multiple_actions",
                Problem::TextAfterAction => "text_after_action",
                Problem::ProseBeforeAction => "prose_before_action",
                Problem::UnclosedThink => "unclosed_think",
                Problem::BadSyntax(_) => "bad_syntax",
                Problem::UnknownAction(_) => "unknown_action",
                Problem::BadArgs(_) => "bad_args",
                Problem::CutOff => "cut_off",
            })
            .collect()
    }
}

/// Parse a decoded reply. `in_think`: the prompt ended inside an open think block, so the text
/// starts with thinking. `cut_off`: generation stopped at the token limit.
pub fn parse_reply(text: &str, syntax: Syntax, actions: &ActionSet, in_think: bool, cut_off: bool) -> Reply {
    use crate::reply_formats as f;
    let mut r = Reply::default();
    let parts = match syntax {
        Syntax::Hermes | Syntax::Xml | Syntax::Aff | Syntax::Glm => {
            let call = if syntax == Syntax::Aff { "<aff " } else { TOOL_OPEN };
            let (think, closed, rest) = f::split_think(text, THINK_OPEN, THINK_CLOSE, in_think, call);
            let (prose, calls, trailing, errors) = scan_calls(rest, syntax);
            f::Parts {
                think,
                think_closed: closed,
                prose,
                calls,
                trailing,
                errors,
                ..Default::default()
            }
        }
        Syntax::Text => {
            // No call opens a reply here, so an unclosed think block runs to the end.
            let (think, closed, rest) = f::split_think(text, THINK_OPEN, THINK_CLOSE, in_think, "\u{0}");
            f::Parts {
                think,
                think_closed: closed,
                prose: rest.trim().to_string(),
                ..Default::default()
            }
        }
        Syntax::MistralArgs => f::mistral_args(text, in_think),
        Syntax::Harmony => f::harmony(text),
        Syntax::Atem => f::atem(text),
        Syntax::Chat => f::chat(text),
        Syntax::Pattern => match &actions.pattern {
            Some(pattern) => pattern.read(text, in_think),
            None => f::Parts {
                think_closed: true,
                prose: text.trim().to_string(),
                errors: vec!["this model's format wrote no tool call to learn its calls from".into()],
                ..Default::default()
            },
        },
        Syntax::Choice => {
            let code = text.trim();
            let mut p = f::Parts {
                think_closed: true,
                ..Default::default()
            };
            match actions.actions.iter().find(|a| a.code.as_deref() == Some(code)) {
                Some(a) => p.calls.push(Call {
                    name: a.name.clone(),
                    args: json!({}),
                    id: None,
                }),
                None if code.is_empty() => {}
                None => p.errors.push(format!("{code:?} is not one of the option codes")),
            }
            p
        }
        _ => {
            // Each parser, with the marker that opens its calls (DeepSeek's is the prefix of both
            // `<｜tool▁calls▁begin｜>` and a bare `<｜tool▁call▁begin｜>`).
            let (call, parse): (&str, fn(&str) -> f::Parts) = match syntax {
                Syntax::DeepSeekV3 => ("<｜tool▁call", |t| f::deepseek(t, true)),
                Syntax::DeepSeekV31 => ("<｜tool▁call", |t| f::deepseek(t, false)),
                Syntax::Dsml => ("<｜DSML｜", f::dsml),
                Syntax::Llama31 => ("<|python_tag|>", f::llama31),
                Syntax::Llama4 => ("<|python_start|>", f::llama4),
                Syntax::MistralList => ("[TOOL_CALLS]", f::mistral_list),
                _ => unreachable!(),
            };
            let (think, closed, rest) = f::split_think(text, THINK_OPEN, THINK_CLOSE, in_think, call);
            let mut p = parse(rest);
            (p.think, p.think_closed) = (think, closed);
            p
        }
    };
    r.think = parts.think;
    r.provider_state = parts.provider_state;
    let closed = parts.think_closed;
    if !closed {
        r.problems.push(Problem::UnclosedThink);
    }
    let (prose, calls, trailing, syntax_errors) = (parts.prose, parts.calls, parts.trailing, parts.errors);
    r.prose = prose;
    r.trailing = trailing;
    for e in syntax_errors {
        r.problems.push(Problem::BadSyntax(e));
    }
    for c in &calls {
        match actions.get(&c.name) {
            None => r.problems.push(Problem::UnknownAction(c.name.clone())),
            Some(def) => {
                if let Err(e) = validate(&def.schema, &c.args) {
                    r.problems.push(Problem::BadArgs(format!("{}: {}", c.name, e)));
                }
            }
        }
    }
    r.calls = calls;
    match r.calls.len() {
        0 if closed => r.problems.push(Problem::NoAction),
        0 => {}
        1 => {}
        n => r.problems.push(Problem::MultipleActions(n)),
    }
    if !r.prose.is_empty() && !r.calls.is_empty() {
        r.problems.push(Problem::ProseBeforeAction);
    }
    if !r.trailing.is_empty() {
        r.problems.push(Problem::TextAfterAction);
    }
    if cut_off {
        r.problems.push(Problem::CutOff);
    }
    r
}

/// (prose before the first call, calls, text after the last call, syntax errors)
fn scan_calls(text: &str, syntax: Syntax) -> (String, Vec<Call>, String, Vec<String>) {
    let mut calls = vec![];
    let mut errors = vec![];
    let (open, close) = match syntax {
        Syntax::Aff => ("<aff ", ">"),
        _ => (TOOL_OPEN, TOOL_CLOSE),
    };
    let Some(first) = text.find(open) else {
        // Hermes models sometimes write the bare object without tags; that is not an action
        if syntax != Syntax::Aff && text.trim_start().starts_with('{') && text.contains("\"name\"") {
            errors.push("call without <tool_call> tags".to_string());
        }
        return (text.trim().to_string(), calls, String::new(), errors);
    };
    let prose = text[..first].trim().to_string();
    let mut rest = &text[first..];
    let mut trailing_start = text.len();
    while let Some(i) = rest.find(open) {
        let after = &rest[i + open.len()..];
        let (body, next, closed) = match syntax {
            Syntax::Aff => match aff_end(after) {
                Some(j) => (&after[..j], &after[j + 1..], true),
                None => (after, "", false),
            },
            // A Hermes body is JSON: its closing tag is the first one outside a JSON string (an
            // argument may hold the tag's text). A body that is not well formed falls back to
            // the first tag, and fails as bad JSON.
            Syntax::Hermes => match close_outside_strings(after, close).or_else(|| after.find(close)) {
                Some(j) => (&after[..j], &after[j + close.len()..], true),
                None => (after, "", false),
            },
            _ => match after.find(close) {
                Some(j) => (&after[..j], &after[j + close.len()..], true),
                None => (after, "", false),
            },
        };
        if !closed {
            errors.push(format!("`{}` is never closed", open.trim()));
        } else {
            match parse_body(body, syntax) {
                Ok(c) => calls.push(c),
                Err(e) => errors.push(e),
            }
        }
        trailing_start = text.len() - next.len();
        rest = next;
    }
    (prose, calls, text[trailing_start..].trim().to_string(), errors)
}

/// Index of the `>` that closes an `<aff name {json}>`: the first `>` outside a JSON string.
fn aff_end(s: &str) -> Option<usize> {
    let (mut in_str, mut esc) = (false, false);
    for (i, c) in s.char_indices() {
        if in_str {
            if esc {
                esc = false;
            } else if c == '\\' {
                esc = true;
            } else if c == '"' {
                in_str = false;
            }
        } else if c == '"' {
            in_str = true;
        } else if c == '>' {
            return Some(i);
        }
    }
    None
}

/// Index of the first `close` in `s` that is not inside a JSON string.
fn close_outside_strings(s: &str, close: &str) -> Option<usize> {
    let (mut in_str, mut esc) = (false, false);
    for (i, c) in s.char_indices() {
        if in_str {
            if esc {
                esc = false;
            } else if c == '\\' {
                esc = true;
            } else if c == '"' {
                in_str = false;
            }
        } else if c == '"' {
            in_str = true;
        } else if s[i..].starts_with(close) {
            return Some(i);
        }
    }
    None
}

fn parse_body(body: &str, syntax: Syntax) -> Result<Call, String> {
    let body = body.trim();
    match syntax {
        Syntax::Hermes => {
            let v: J = serde_json::from_str(body).map_err(|e| format!("tool call is not JSON ({}): {}", e, body))?;
            let name = v["name"].as_str().ok_or("tool call without a name")?.to_string();
            let args = match &v["arguments"] {
                J::String(s) => {
                    serde_json::from_str(s).map_err(|_| "arguments is a string that is not JSON".to_string())?
                }
                J::Null => json!({}),
                a => a.clone(),
            };
            Ok(Call { name, args, id: None })
        }
        Syntax::Aff => {
            let (name, args) = match body.find(|c: char| c.is_whitespace()) {
                Some(i) => (&body[..i], body[i..].trim()),
                None => (body, ""),
            };
            if name.is_empty() {
                return Err("<aff> without a name".into());
            }
            let args = if args.is_empty() {
                json!({})
            } else {
                serde_json::from_str(args).map_err(|e| format!("<aff> arguments are not JSON ({})", e))?
            };
            Ok(Call {
                name: name.to_string(),
                args,
                id: None,
            })
        }
        Syntax::Xml => {
            let f = body
                .strip_prefix("<function=")
                .ok_or("tool call without <function=NAME>")?;
            let gt = f.find('>').ok_or("unterminated <function=")?;
            let name = f[..gt].trim().to_string();
            let inner = f[gt + 1..].strip_suffix("</function>").ok_or("missing </function>")?;
            // MiMo V2.6 writes string arguments as a JSON body: `<function=x>{"k": "v"}</function>`.
            if inner.trim_start().starts_with('{') && !inner.contains("<parameter=") {
                let args: J =
                    serde_json::from_str(inner.trim()).map_err(|e| format!("function body is not JSON ({e})"))?;
                return Ok(Call { name, args, id: None });
            }
            // Otherwise the body is `<parameter=>` elements and nothing else.
            let stray = |t: &str| {
                if t.trim().is_empty() {
                    Ok(())
                } else {
                    Err(format!("unexpected text in the call to {name}: {:?}", t.trim()))
                }
            };
            let mut args = serde_json::Map::new();
            let mut rest = inner;
            while let Some(i) = rest.find("<parameter=") {
                stray(&rest[..i])?;
                let after = &rest[i + "<parameter=".len()..];
                let gt = after.find('>').ok_or("unterminated <parameter=")?;
                let key = after[..gt].trim().to_string();
                let val = &after[gt + 1..];
                let end = val.find("</parameter>").ok_or("missing </parameter>")?;
                let raw = val[..end].strip_prefix('\n').unwrap_or(&val[..end]);
                let raw = raw.strip_suffix('\n').unwrap_or(raw);
                args.insert(key, J::String(raw.to_string()));
                rest = &val[end + "</parameter>".len()..];
            }
            stray(rest)?;
            Ok(Call {
                name,
                args: J::Object(args),
                id: None,
            })
        }
        Syntax::Glm => {
            let name_end = body.find("<arg_key>").unwrap_or(body.len());
            let name = body[..name_end].trim().to_string();
            if name.is_empty() {
                return Err("tool call without a name".into());
            }
            let mut args = serde_json::Map::new();
            let mut rest = &body[name_end..];
            while let Some(i) = rest.find("<arg_key>") {
                let after = &rest[i + "<arg_key>".len()..];
                let k_end = after.find("</arg_key>").ok_or("missing </arg_key>")?;
                let key = after[..k_end].trim().to_string();
                let after = after[k_end + "</arg_key>".len()..].trim_start();
                let val = after
                    .strip_prefix("<arg_value>")
                    .ok_or_else(|| format!("argument `{key}` has no <arg_value>"))?;
                let v_end = val.find("</arg_value>").ok_or("missing </arg_value>")?;
                args.insert(key, J::String(val[..v_end].to_string()));
                rest = &val[v_end + "</arg_value>".len()..];
            }
            if !rest.trim().is_empty() {
                return Err(format!("unexpected text in tool call: {:?}", rest.trim()));
            }
            Ok(Call {
                name,
                args: J::Object(args),
                id: None,
            })
        }
        other => Err(format!("{other:?} calls are not <tool_call> bodies")),
    }
}

/// XML parameters arrive as text); give them the type their schema asks for.
pub fn coerce_xml_args(call: &mut Call, actions: &ActionSet) {
    let Some(def) = actions.get(&call.name) else { return };
    let props = def.schema.get("properties").and_then(|p| p.as_object());
    if let J::Object(o) = &mut call.args {
        for (k, v) in o.iter_mut() {
            let J::String(s) = v else { continue };
            let schema = props.and_then(|p| p.get(k));
            let parsed = match schema.and_then(type_of) {
                Some("string") => None,
                Some("integer") => s.trim().parse::<i64>().ok().map(J::from),
                Some("number") => s.trim().parse::<f64>().ok().map(J::from),
                Some("boolean") => match s.trim() {
                    "true" | "True" => Some(J::Bool(true)),
                    "false" | "False" => Some(J::Bool(false)),
                    _ => None,
                },
                // JSON, or a Python literal (Llama 4 writes `str(v)`: `{'a': True}`).
                _ => serde_json::from_str::<J>(s.trim())
                    .ok()
                    .or_else(|| crate::reply_formats::python_value(s)),
            };
            // A schema that allows a string keeps the text when it rejects the parsed value
            // (`2026` for `["string", "null"]` is the string "2026").
            let keeps_text =
                |p: &J| schema.is_some_and(|sc| types_of(sc).contains(&"string") && validate(sc, p).is_err());
            if let Some(p) = parsed.filter(|p| !keeps_text(p)) {
                *v = p;
            }
        }
    }
}

/// Parse a reply and, for the XML syntax, type its parameters.
pub fn parse_reply_typed(text: &str, syntax: Syntax, actions: &ActionSet, in_think: bool, cut_off: bool) -> Reply {
    if !syntax.text_args() {
        return parse_reply(text, syntax, actions, in_think, cut_off);
    }
    // type the parameters first so validation sees numbers, not text
    let untyped = ActionSet {
        actions: vec![],
        pattern: actions.pattern.clone(),
    };
    let mut r = parse_reply(text, syntax, &untyped, in_think, cut_off);
    r.problems
        .retain(|p| !matches!(p, Problem::UnknownAction(_) | Problem::BadArgs(_)));
    for c in &mut r.calls {
        coerce_xml_args(c, actions);
        match actions.get(&c.name) {
            None => r.problems.push(Problem::UnknownAction(c.name.clone())),
            Some(def) => {
                if let Err(e) = validate(&def.schema, &c.args) {
                    r.problems.push(Problem::BadArgs(format!("{}: {}", c.name, e)));
                }
            }
        }
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chat_reply_is_json_with_reasoning_content_and_calls() {
        let r = parse_reply_typed(
            r#"{"reasoning": "count first", "content": "", "tool_calls": [{"name": "scroll", "arguments": {"by": 3}}]}"#,
            Syntax::Chat,
            &actions(),
            false,
            false,
        );
        assert_eq!(
            (r.think.as_deref(), r.calls.len(), r.problems.len()),
            (Some("count first"), 1, 0)
        );
        assert_eq!(r.calls[0].args, json!({"by": 3}));
        let bad = parse_reply_typed("not json", Syntax::Chat, &actions(), false, false);
        assert!(matches!(bad.problems[0], Problem::BadSyntax(_)));
    }

    fn actions() -> ActionSet {
        ActionSet {
            pattern: None,
            actions: vec![
                ActionDef {
                    code: None,
                    name: "scroll".into(),
                    schema: json!({"type":"object","properties":{"by":{"type":"integer"}},"required":["by"]}),
                    default: None,
                },
                ActionDef {
                    code: None,
                    name: "wait".into(),
                    schema: json!({"type":"object","properties":{}}),
                    default: Some(json!({})),
                },
                ActionDef {
                    code: None,
                    name: "answer".into(),
                    schema: json!({"type":"object","properties":{"value":{"type":"string"}},"required":["value"]}),
                    default: None,
                },
            ],
        }
    }

    #[test]
    fn one_action_is_valid_with_or_without_think() {
        let a = actions();
        let r = parse_reply(
            "<think>\nhmm\n</think>\n\n<tool_call>\n{\"name\": \"scroll\", \"arguments\": {\"by\": 3}}\n</tool_call>",
            Syntax::Hermes,
            &a,
            false,
            false,
        );
        assert!(r.valid(), "{:?}", r.problems);
        assert_eq!(r.think.as_deref(), Some("hmm"));
        let r = parse_reply(
            "<tool_call>{\"name\":\"wait\",\"arguments\":{}}</tool_call>",
            Syntax::Hermes,
            &a,
            false,
            false,
        );
        assert!(r.valid(), "{:?}", r.problems);
    }

    #[test]
    fn every_way_to_break_the_shape_is_named() {
        let a = actions();
        let p = |t: &str| parse_reply(t, Syntax::Hermes, &a, false, false).problem_names();
        assert_eq!(p("The answer is 5."), vec!["no_action"]);
        assert_eq!(p(""), vec!["no_action"]);
        assert_eq!(
            p("<tool_call>{\"name\":\"fly\",\"arguments\":{}}</tool_call>"),
            vec!["unknown_action"]
        );
        assert_eq!(
            p("<tool_call>{\"name\":\"scroll\",\"arguments\":{\"by\":\"x\"}}</tool_call>"),
            vec!["bad_args"]
        );
        assert_eq!(
            p("<tool_call>{\"name\":\"scroll\",\"arguments\":{}}</tool_call>"),
            vec!["bad_args"]
        );
        assert_eq!(p("<tool_call>{broken</tool_call>"), vec!["bad_syntax", "no_action"]);
        assert_eq!(
            p("Let me scroll.<tool_call>{\"name\":\"wait\",\"arguments\":{}}</tool_call>"),
            vec!["prose_before_action"]
        );
        assert_eq!(
            p("<tool_call>{\"name\":\"wait\",\"arguments\":{}}</tool_call> done"),
            vec!["text_after_action"]
        );
        assert_eq!(
            p(
                "<tool_call>{\"name\":\"wait\",\"arguments\":{}}</tool_call><tool_call>{\"name\":\"wait\",\"arguments\":{}}</tool_call>"
            ),
            vec!["multiple_actions"]
        );
        assert_eq!(p("<think>still thinking"), vec!["unclosed_think"]);
        assert!(
            parse_reply(
                "Let me scroll.<tool_call>{\"name\":\"wait\",\"arguments\":{}}</tool_call>",
                Syntax::Hermes,
                &a,
                false,
                false
            )
            .lenient_valid()
        );
    }

    #[test]
    fn reply_that_starts_inside_an_open_think_block() {
        let a = actions();
        let r = parse_reply(
            "I should wait.\n</think>\n\n<tool_call>\n{\"name\":\"wait\",\"arguments\":{}}\n</tool_call>",
            Syntax::Hermes,
            &a,
            true,
            false,
        );
        assert!(r.valid(), "{:?}", r.problems);
        assert_eq!(r.think.as_deref(), Some("I should wait."));
    }

    #[test]
    fn xml_and_native_syntaxes() {
        let a = actions();
        let r = parse_reply_typed(
            "</think>\n\n<tool_call>\n<function=scroll>\n<parameter=by>\n-4\n</parameter>\n</function>\n</tool_call>",
            Syntax::Xml,
            &a,
            true,
            false,
        );
        assert!(r.valid(), "{:?}", r.problems);
        assert_eq!(r.calls[0].args, json!({"by": -4}));
        let r = parse_reply_typed(
            "<tool_call>\n<function=answer>\n<parameter=value>\n12\n</parameter>\n</function>\n</tool_call>",
            Syntax::Xml,
            &a,
            false,
            false,
        );
        assert!(r.valid(), "{:?}", r.problems);
        assert_eq!(r.calls[0].args, json!({"value": "12"}));
        let r = parse_reply("<aff scroll {\"by\": 2}>", Syntax::Aff, &a, false, false);
        assert!(r.valid(), "{:?}", r.problems);
        let r = parse_reply("<aff wait>", Syntax::Aff, &a, false, false);
        assert!(r.valid(), "{:?}", r.problems);
        let r = parse_reply("<aff answer {\"value\": \"a>b\"}>", Syntax::Aff, &a, false, false);
        assert_eq!(r.calls[0].args, json!({"value": "a>b"}));
    }

    #[test]
    fn glm_syntax_and_cut_off_replies() {
        let a = actions();
        // The GLM-5.3 prompt ends inside <think>; non-string values arrive JSON-encoded as text.
        let r = parse_reply_typed(
            "plan it  \n</think><tool_call>scroll<arg_key>by</arg_key><arg_value>-4</arg_value></tool_call>",
            Syntax::Glm,
            &a,
            true,
            false,
        );
        assert!(r.valid(), "{:?}", r.problems);
        assert_eq!(r.think.as_deref(), Some("plan it"));
        assert_eq!(r.calls[0].args, json!({"by": -4}));
        let r = parse_reply_typed(
            "</think><tool_call>answer<arg_key>value</arg_key><arg_value>12</arg_value></tool_call>",
            Syntax::Glm,
            &a,
            true,
            false,
        );
        assert_eq!(r.calls[0].args, json!({"value": "12"}));
        let r = parse_reply_typed("</think><tool_call>wait</tool_call>", Syntax::Glm, &a, true, false);
        assert!(r.valid(), "{:?}", r.problems);
        // Text-only reply: no action, the prose is the content.
        let r = parse_reply_typed("hmm</think>Nothing to do.", Syntax::Glm, &a, true, false);
        assert_eq!(
            (r.prose.as_str(), r.problem_names()),
            ("Nothing to do.", vec!["no_action"])
        );
        // Cut off before </think>: the fragment is thinking, never prose or a call.
        let r = parse_reply_typed(
            "the puzzle has three parts, and the second",
            Syntax::Glm,
            &a,
            true,
            true,
        );
        assert_eq!(r.problem_names(), vec!["unclosed_think", "cut_off"]);
        assert!(r.prose.is_empty() && r.calls.is_empty());
        // Cut off inside a tool call: no call is invented from the partial arguments.
        let r = parse_reply_typed(
            "ok</think><tool_call>answer<arg_key>value</arg_key><arg_value>12",
            Syntax::Glm,
            &a,
            true,
            true,
        );
        assert!(r.calls.is_empty(), "{:?}", r.calls);
        assert_eq!(r.problem_names(), vec!["bad_syntax", "no_action", "cut_off"]);
        assert!(!r.lenient_valid());
        // A malformed body names the problem.
        let r = parse_reply_typed(
            "</think><tool_call>answer<arg_key>value</arg_key>12</tool_call>",
            Syntax::Glm,
            &a,
            true,
            false,
        );
        assert_eq!(r.problem_names(), vec!["bad_syntax", "no_action"]);
        assert_eq!(Syntax::parse("glm"), Ok(Syntax::Glm));
    }

    #[test]
    fn family_formats_cut_off_and_edge_cases() {
        let a = ActionSet {
            pattern: None,
            actions: vec![ActionDef {
                code: None,
                name: "scroll".into(),
                schema: json!({"type":"object","properties":{"by":{"type":"integer"}},"required":["by"]}),
                default: None,
            }],
        };
        let p = |t: &str, s: Syntax, think: bool, cut: bool| parse_reply_typed(t, s, &a, think, cut);
        // gpt-oss: analysis cut off before <|end|> is unfinished thinking.
        let r = p(
            "<|channel|>analysis<|message|>Let me think about",
            Syntax::Harmony,
            false,
            true,
        );
        assert_eq!(r.problem_names(), vec!["unclosed_think", "cut_off"]);
        let r = p(
            "<|channel|>analysis<|message|>Scroll down.<|end|><|start|>assistant<|channel|>commentary to=functions.scroll <|constrain|>json<|message|>{\"by\": 3}",
            Syntax::Harmony,
            false,
            false,
        );
        assert!(r.valid(), "{:?}", r.problems);
        assert_eq!(r.think.as_deref(), Some("Scroll down."));
        // Mistral: [THINK] cut off; a call after [THINK]..[/THINK].
        assert_eq!(
            p("[THINK]hmm, the", Syntax::MistralArgs, false, true).problem_names(),
            vec!["unclosed_think", "cut_off"]
        );
        let r = p(
            "[THINK]Down.[/THINK][TOOL_CALLS]scroll[CALL_ID]abcdefghi[ARGS]{\"by\": 2}",
            Syntax::MistralArgs,
            false,
            false,
        );
        assert!(r.valid() && r.think.as_deref() == Some("Down."), "{:?}", r.problems);
        // DSML block never closed (cut off mid-call): no call is invented.
        let r = p(
            "<｜DSML｜tool_calls>\n<｜DSML｜invoke name=\"scroll\">\n<｜DSML｜parameter name=\"by\" string=\"false\">3",
            Syntax::Dsml,
            false,
            true,
        );
        assert!(
            r.calls.is_empty() && r.problem_names().contains(&"cut_off"),
            "{:?}",
            r.calls
        );
        // DeepSeek V3 single call without the closing token.
        let r = p(
            "<｜tool▁calls▁begin｜><｜tool▁call▁begin｜>function<｜tool▁sep｜>scroll\n```json\n{\"by\": 1}\n```<｜tool▁call▁end｜>",
            Syntax::DeepSeekV3,
            false,
            false,
        );
        assert!(r.valid(), "{:?}", r.problems);
        // Llama 3.1 with <|python_tag|>; plain prose is no call.
        assert!(
            p(
                "<|python_tag|>{\"name\": \"scroll\", \"parameters\": {\"by\": 4}}",
                Syntax::Llama31,
                false,
                false
            )
            .valid()
        );
        assert_eq!(
            p("It is sunny.", Syntax::Llama31, false, false).problem_names(),
            vec!["no_action"]
        );
        // Llama 4 after prose, values written as str(v).
        let r = p("Scrolling now.[scroll(by=\"5\")]", Syntax::Llama4, false, false);
        assert_eq!(
            (r.prose.as_str(), r.calls[0].args.clone()),
            ("Scrolling now.", json!({"by": 5}))
        );
    }
}
