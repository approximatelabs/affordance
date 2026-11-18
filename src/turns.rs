//! The pieces a turn strategy is written with (`views/strategies/turn_strategy.tsx`): a log
//! record as a turn (`logTurns`, native and memoized: it is in every render) and a turn plan
//! from turns and the laid-out screen (`turnPlan`). Which records, in what order, reduced how, is
//! the strategy's, in TSX. Only template apps translate a plan into their compatibility
//! `messages` input. Content remains a runtime value and may hold native token IDs and spans.
use crate::emit::{Capture, Item, Span};
use crate::interp::emit_child;
use crate::value::{Map, Value};
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct TurnPlan {
    pub turns: Vec<Value>,
    pub tools: Value,
    pub callable: bool,
    /// Per turn: the log record it came from (index, kind); `None` for the current screen.
    /// Render errors inside a template name the record (see `Interp::turn_origin`).
    pub origins: Vec<Option<(usize, Rc<str>)>>,
    /// The model-callable actions on the current screen, as zero-width spans (`screen_actions`).
    pub(crate) actions: Vec<Span>,
}

/// Label of the span around the current screen's tokens in the final prompt. The harness can
/// log `ids[span.start..span.end]` as `{"kind": "screen", "ids": [...]}` (see `Rendered::current_turn`).
pub const CURRENT_TURN: &str = "turn:current";

fn at(i: usize, kind: &str) -> String {
    format!("TurnStrategy log[{i}] (kind {kind:?})")
}

/// `ids` on a log record: the exact token ids of the record's body. Absent or null means
/// "render from the record's fields". A reduced record (a fold) drops them: `dict(e, ids=null)`.
fn record_ids(e: &Map, i: usize, kind: &str) -> Result<Option<Rc<[u32]>>, String> {
    match e.get("ids") {
        None | Some(Value::None) | Some(Value::Undef) => Ok(None),
        // Exact ids as the harness keeps them: a token fragment (no copy).
        Some(Value::Frag(f)) if f.len() == 1 && matches!(&f[0], Item::Tokens(_, spans) if spans.is_empty()) => {
            match &f[0] {
                Item::Tokens(ids, _) => Ok(Some(ids.clone())),
                _ => unreachable!(),
            }
        }
        Some(Value::List(l)) => l
            .read()
            .iter()
            .map(|v| match v {
                Value::Int(n) if (0..=u32::MAX as i64).contains(n) => Ok(*n as u32),
                o => Err(format!(
                    "{}: `ids` must be a list of token ids; got an element of type {}",
                    at(i, kind),
                    o.type_name()
                )),
            })
            .collect::<Result<Vec<_>, _>>()
            .map(|v| Some(v.into())),
        Some(o) => Err(format!(
            "{}: `ids` must be a list of token ids; got {}",
            at(i, kind),
            o.type_name()
        )),
    }
}

/// Exact ids as turn content: laid out already, so templates splice them without re-encoding.
fn ids_content(ids: &Rc<[u32]>) -> Value {
    Value::Frag(Rc::new(vec![Item::Tokens(ids.clone(), Rc::from(Vec::new()))]))
}

/// A screen record's `images` (`[{at, n, src}]`: the `<img>`s its ids hold) as the img spans the
/// screen was rendered with, so its images are sent again with every prompt that shows it.
fn image_spans(e: &Map, len: usize, i: usize, kind: &str) -> Result<Vec<Span>, String> {
    let Some(images) = e.get("images") else {
        return Ok(vec![]);
    };
    let images = images.to_wire();
    let bad = || {
        format!(
            "{}: `images` must be a list of {{at, n, src}} within the ids",
            at(i, kind)
        )
    };
    images
        .as_array()
        .ok_or_else(bad)?
        .iter()
        .map(|img| {
            let (start, n, src) = (
                img["at"].as_u64().ok_or_else(bad)? as usize,
                img["n"].as_u64().ok_or_else(bad)? as usize,
                img["src"].as_str().ok_or_else(bad)?,
            );
            if start + n > len {
                return Err(bad());
            }
            Ok(Span {
                label: "img".into(),
                kind: "label".into(),
                start,
                end: start + n,
                straddle: false,
                meta: serde_json::json!({"img": true, "tok_size": n, "src": src}),
            })
        })
        .collect()
}

/// Text content of a record: a string, a fragment, a list of content parts, or null. A map is
/// rejected here, where the record is known, rather than deep inside a template.
fn text_field(e: &Map, key: &str, i: usize, kind: &str) -> Result<Value, String> {
    let v = field(e, key);
    if let Value::Map(_) = v {
        return Err(format!(
            "{}: `{key}` must be a string (or a fragment / list of content parts); got dict. Serialize objects first, e.g. json(x)",
            at(i, kind)
        ));
    }
    snapshot(v)
}

/// A tool result's content as a `<result>` element when a sheet styles one, e.g.
/// `result { max-tokens: 1500tok; overflow: clip-middle; more: "\n[... {hidden} tokens hidden,
/// read_event {id}]\n"; }` (selectable by `[name="shell"]`, `#id`, `[error]`): the cap is
/// layout's, in tokens, and `{id}` is the record's id. Without such a rule the content is as
/// written, so every prompt is unchanged. A styled result is its own tokenization run (a box).
fn styled_result(
    rules: &[crate::style::Rule],
    ancestry: &crate::style::Ancestry,
    e: &Map,
    content: Value,
) -> Result<Value, String> {
    if !rules.iter().any(|r| r.sel.name.as_deref() == Some("result")) {
        return Ok(content);
    }
    let mut props: Vec<(Rc<str>, Value)> = vec![(Rc::from("kind"), Value::str("action"))];
    for key in ["name", "id", "error", "tool_call_id"] {
        if let Some(v) = e.get(key) {
            props.push((Rc::from(key), v.clone()));
        }
    }
    // The static style, and the one it takes once its position matches a `result:nth-last`
    // rule (resolved at layout). A result is boxed when either applies to it.
    let (style, graded) = crate::style::compute_graded(rules, "result", &props, ancestry)?;
    if !style.boxed() && graded.is_none() {
        return Ok(content);
    }
    let graded = graded.map(|(g, k, s)| {
        Rc::new(crate::emit::Graded {
            k,
            s,
            kind: crate::interp::document::box_kind(&g, None, None),
        })
    });
    let mut items = match &content {
        Value::Str(s) => vec![Item::Text(s.to_rc())],
        Value::Frag(f) => f.to_vec(),
        _ => return Ok(content),
    };
    let id = e.get("id").and_then(Value::as_str).unwrap_or("result");
    let meta = serde_json::json!({"name": e.get("name").map(Value::to_wire), "id": id});
    items.insert(
        0,
        Item::Open(Rc::new(crate::emit::SpanSpec {
            label: Rc::from(id),
            kind: crate::interp::document::box_kind_graded(&style, None, None, graded),
            meta: serde_json::json!({"component": "result"}),
        })),
    );
    items.insert(
        0,
        Item::Open(Rc::new(crate::emit::SpanSpec {
            label: Rc::from("result"),
            kind: crate::emit::SpanKind::Label,
            meta,
        })),
    );
    items.extend([Item::Close, Item::Close]);
    Ok(Value::Frag(Rc::new(items)))
}

fn turn(role: &str, content: Value) -> Map {
    let mut m = Map::default();
    m.insert(Rc::from("role"), Value::str(role));
    m.insert(Rc::from("content"), content);
    m
}

fn field(m: &Map, key: &str) -> Value {
    m.get(key).cloned().unwrap_or(Value::Undef)
}

/// A log observation retains token ownership and diagnostic ranges, but its
/// callbacks belong to a past render and must never be callable now.
fn snapshot(v: Value) -> Result<Value, String> {
    fn passive(meta: &mut serde_json::Value) {
        if let Some(m) = meta.as_object_mut() {
            m.remove("handler");
            m.remove("projected_tool");
        }
    }
    Ok(match v {
        Value::Frag(f) => {
            let mut items = Vec::with_capacity(f.len());
            for item in f.iter() {
                items.push(match item {
                    Item::Open(spec) => {
                        let mut spec = (**spec).clone();
                        passive(&mut spec.meta);
                        if let crate::emit::SpanKind::Box {
                            scroll: Some(scroll), ..
                        } = &mut spec.kind
                        {
                            let mut action = (*scroll.action).clone();
                            passive(&mut action.meta);
                            scroll.action = Rc::new(action);
                        }
                        Item::Open(Rc::new(spec))
                    }
                    Item::Tokens(ids, spans) => {
                        let mut spans = spans.to_vec();
                        for span in &mut spans {
                            passive(&mut span.meta);
                        }
                        Item::Tokens(ids.clone(), spans.into())
                    }
                    Item::Turns(_) => {
                        return Err("a log observation cannot contain a turn plan".into());
                    }
                    item => item.clone(),
                });
            }
            Value::Frag(Rc::new(items))
        }
        Value::List(v) => Value::list(v.read().iter().cloned().map(snapshot).collect::<Result<_, _>>()?),
        Value::Map(v) => Value::map(
            v.read()
                .iter()
                .map(|(k, v)| Ok((k.clone(), snapshot(v.clone())?)))
                .collect::<Result<_, String>>()?,
        ),
        v => v,
    })
}

/// `screen` records are prior observations, `input` is external user input,
/// `model` carries output/calls, and `action` carries a committed result. Timing
/// and other metadata may coexist on every record and stay in the source log.
/// Old screens are observations only: they never re-publish old handlers.
/// `memo`: an agent's store, which keeps each record's turn (a record is never changed in place, so
/// the same record gives the same turn, and later stages see the same value).
/// What a record's styled turn depends on besides the record: which rules apply, and the
/// ancestors a rule's selector can name (their element names and plain attribute values).
fn style_scope(rules: &[crate::style::Rule], ancestry: &crate::style::Ancestry) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (rules.as_ptr() as usize, rules.len()).hash(&mut h);
    for (name, props) in ancestry {
        name.hash(&mut h);
        for (key, value) in props.iter() {
            // Selectors compare attributes with plain values; a list or a function is not one.
            let plain = match value {
                Value::Str(s) => Some(s.to_string()),
                Value::Int(_) | Value::Float(_) | Value::Bool(_) => Some(crate::jsvalue::to_string(value)),
                _ => None,
            };
            if let Some(plain) = plain {
                (key, plain).hash(&mut h);
            }
        }
    }
    h.finish()
}

pub fn log_turns(
    log: &Value,
    skip: &[String],
    mut memo: Option<&mut crate::memo::MemoStore>,
    rules: &[crate::style::Rule],
    ancestry: &crate::style::Ancestry,
) -> Result<(Vec<Value>, Vec<(usize, Rc<str>)>), String> {
    let entries: Vec<Value> = match log {
        Value::Undef | Value::None => vec![],
        Value::List(v) => v.read().clone(),
        _ => return Err("logTurns(log): log must be a list of records".into()),
    };
    if let Some(memo) = memo.as_deref_mut() {
        memo.turn_scope(style_scope(rules, ancestry));
    }
    let mut turns = Vec::new();
    let mut origins = Vec::new();
    for (i, entry) in entries.iter().enumerate() {
        let Value::Map(record) = entry else {
            return Err(format!(
                "TurnStrategy log[{i}]: a log record must be an object; got {}",
                entry.type_name()
            ));
        };
        let guard = record.read();
        let e: &Map = &guard;
        let kind = e
            .get("kind")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("TurnStrategy log[{i}]: a log record needs a string `kind` (input, screen, model, action, metric, event)"))?;
        if skip.iter().any(|k| k == kind) {
            continue;
        }
        // The record's own string, shared (no allocation per record per render).
        let kind_rc: Rc<str> = match e.get("kind") {
            Some(Value::Str(t)) => t.to_rc(),
            _ => Rc::from(kind),
        };
        if let Some(t) = memo.as_deref_mut().and_then(|m| m.turn(entry)) {
            turns.push(t);
            origins.push((i, kind_rc.clone()));
            continue;
        }
        let ids = match kind {
            "metric" | "event" => None,
            _ => record_ids(e, i, kind)?,
        };
        let m = match kind {
            "screen" | "input" => turn(
                "user",
                match &ids {
                    Some(ids) if e.contains_key("images") => Value::Frag(Rc::new(vec![Item::Tokens(
                        ids.clone(),
                        image_spans(e, ids.len(), i, kind)?.into(),
                    )])),
                    Some(ids) => ids_content(ids),
                    None => text_field(e, "content", i, kind)?,
                },
            ),
            // Exact ids: the template emits its generation prompt, then these ids verbatim. The
            // calls ride along as `tool_calls` (metadata: a template pairs tool results with them,
            // and its state after a calling turn depends on them); `content` and `reasoning` stay
            // in the log.
            "model" if ids.is_some() => {
                // `ids` is the content itself: a token run, read like a list (interp::token_run).
                let content = ids_content(&ids.unwrap());
                let mut m = turn("assistant", content.clone());
                m.insert(Rc::from("generated"), Value::Bool(true));
                m.insert(Rc::from("ids"), content);
                if let Some(calls) = e
                    .get("calls")
                    .filter(|c| matches!(c, Value::List(l) if !l.read().is_empty()))
                {
                    m.insert(Rc::from("tool_calls"), calls.clone());
                }
                if let Some(state) = e.get("provider_state") {
                    m.insert(Rc::from("provider_state"), state.clone());
                }
                // What the reply said, as it was read: for a template app that writes a past
                // reply the way the model's publisher writes one, from its text.
                let mut reply = Map::default();
                for key in ["content", "reasoning"] {
                    if let Some(v) = e.get(key) {
                        reply.insert(Rc::from(key), v.clone());
                    }
                }
                m.insert(Rc::from("reply"), Value::map(reply));
                m
            }
            "model" => {
                let mut m = turn("assistant", text_field(e, "content", i, kind)?);
                for (src, dst) in [
                    ("calls", "tool_calls"),
                    ("reasoning", "reasoning_content"),
                    ("provider_state", "provider_state"),
                ] {
                    match e.get(src) {
                        // No calls is no `tool_calls`, as in the ids branch (Llama's template rejects an empty list).
                        Some(Value::List(l)) if src == "calls" && l.read().is_empty() => {}
                        Some(v) => {
                            m.insert(Rc::from(dst), v.clone());
                        }
                        None => {}
                    }
                }
                m
            }
            "action" => {
                let content = match &ids {
                    Some(ids) => ids_content(ids),
                    None => text_field(e, "result", i, kind)?,
                };
                let mut m = turn("tool", styled_result(rules, ancestry, e, content)?);
                for key in ["name", "tool_call_id", "error"] {
                    if let Some(v) = e.get(key) {
                        m.insert(Rc::from(key), v.clone());
                    }
                }
                m
            }
            // Metrics/events without a prompt-facing representation stay in the log.
            "metric" | "event" => continue,
            _ => {
                return Err(format!(
                    "TurnStrategy log[{i}]: unsupported log record kind {kind:?} (use input, screen, model, action, metric or event)"
                ));
            }
        };
        let t = Value::map(m);
        if let Some(memo) = memo.as_deref_mut() {
            memo.keep_turn(entry, t.clone());
        }
        turns.push(t);
        origins.push((i, kind_rc.clone()));
    }
    Ok((turns, origins))
}

/// A turn plan: `turns` (with the log records they came from, where known), then the current
/// screen (`app`: `asChatTools(children)`) as the final user turn, when there is anything on it.
pub fn plan(app: &Map, mut turns: Vec<Value>, mut origins: Vec<Option<(usize, Rc<str>)>>) -> Result<TurnPlan, String> {
    let callable = tools_callable(app)?;
    // The current screen is one laid-out subtree; a span over all of its tokens lets the
    // harness find its exact ids in the final prompt (no template knowledge, no decoding).
    let current = match field(app, "tree") {
        Value::Frag(f) => match f.as_slice() {
            [Item::Tokens(ids, spans)] => {
                let mut all = Vec::with_capacity(spans.len() + 1);
                all.push(Span {
                    label: CURRENT_TURN.into(),
                    kind: "label".into(),
                    start: 0,
                    end: ids.len(),
                    straddle: false,
                    meta: serde_json::json!({}),
                });
                all.extend(spans.iter().cloned());
                Value::Frag(Rc::new(vec![Item::Tokens(ids.clone(), all.into())]))
            }
            _ => Value::Frag(f),
        },
        v => v,
    };
    // An app with nothing on its screen (a plain conversation, a model layer for another
    // harness) has no current turn: nothing is laid out, so nothing is written, as an empty box
    // takes no space. The prompt then ends with the last turn of the log.
    let nothing = match &current {
        Value::Frag(f) => f.iter().all(|item| match item {
            Item::Tokens(ids, _) => ids.is_empty(),
            Item::Text(text) => text.is_empty(),
            _ => false,
        }),
        Value::Str(text) => text.is_empty(),
        Value::None | Value::Undef => true,
        _ => false,
    };
    // The screen's actions are read here, before the screen may turn out to take no turn: a
    // screen of tools with no text of their own writes nothing and still offers them.
    let mut actions = vec![];
    if let Value::Frag(items) = &current {
        for item in items.iter() {
            let Item::Tokens(_, spans) = item else { continue };
            actions.extend(spans.iter().filter(|s| crate::is_model_action(s)).map(|s| Span {
                start: 0,
                end: 0,
                straddle: false,
                ..s.clone()
            }));
        }
    }
    if !nothing {
        turns.push(Value::map(turn("user", current)));
        origins.push(None);
    }
    Ok(TurnPlan {
        turns,
        tools: field(app, "tools"),
        callable,
        origins,
        actions,
    })
}

fn tools_callable(input: &Map) -> Result<bool, String> {
    match input.get("callable") {
        None | Some(Value::Undef) => Ok(true),
        Some(Value::Bool(b)) => Ok(*b),
        _ => Err("callable must be a boolean".into()),
    }
}

pub fn fragment(plan: TurnPlan) -> Value {
    Value::Frag(Rc::new(vec![Item::Turns(Rc::new(plan))]))
}

/// Consume exactly one strategy child. Reject mixed content instead of silently
/// losing an app or publishing tools that have no corresponding action spans.
/// `generated_ok`: the template renders `model` records that carry exact `ids` (a turn with
/// `generated: true`, `ids`, and the ids as `content`); other template apps reject them.
/// `alternate`: the template requires user and assistant turns to alternate (Llama 2, Mistral
/// v0.3; calls and tool results do not count): a user turn right after another user turn (the
/// task, then a screen) or after a tool result (the screen after a call) joins it, after a blank
/// line, its laid-out content kept as it is.
pub fn template_input(
    children: &Value,
    system: Value,
    generated_ok: bool,
    alternate: bool,
) -> Result<(Value, Rc<TurnPlan>), String> {
    let mut cap = Capture::default();
    emit_child(children, &mut cap)?;
    let mut found = None;
    for item in &cap.items {
        match item {
            Item::Turns(plan) if found.is_none() => found = Some(plan),
            Item::Text(s) if s.is_empty() => (),
            Item::Segment => (),
            _ => return Err("template app requires exactly one TurnStrategy child".into()),
        }
    }
    let plan = found.ok_or("template app requires a TurnStrategy child")?;
    if !generated_ok {
        for (t, o) in plan.turns.iter().zip(&plan.origins) {
            if let (Value::Map(m), Some((i, kind))) = (t, o)
                && m.read().contains_key("generated")
            {
                return Err(format!(
                    "{}: this template app does not render model records with exact `ids` (MessagesApp sends text to hosted APIs); drop `ids`",
                    at(*i, kind)
                ));
            }
        }
    }
    // A null system: no system turn (a format that moves the system prompt every turn).
    let mut turns = if matches!(system, Value::None) {
        vec![]
    } else {
        vec![Value::map(turn("system", system))]
    };
    for t in plan.turns.iter() {
        let role = |v: Option<&Value>| match v {
            Some(Value::Map(m)) => m.read().get("role").and_then(Value::as_str).map(str::to_owned),
            _ => None,
        };
        let joins = alternate
            && role(Some(t)).as_deref() == Some("user")
            && matches!(role(turns.last()).as_deref(), Some("user" | "tool"));
        let joined = match (joins, turns.last(), t) {
            (true, Some(Value::Map(a)), Value::Map(b)) => match (
                content_items(a.read().get("content")),
                content_items(b.read().get("content")),
            ) {
                (Some(mut x), Some(y)) => {
                    x.push(Item::Text(Rc::from("\n\n")));
                    x.extend(y);
                    let mut m = a.read().clone();
                    m.insert(Rc::from("content"), Value::Frag(Rc::new(x)));
                    Some(Value::map(m))
                }
                _ => None,
            },
            _ => None,
        };
        match joined {
            Some(m) => *turns.last_mut().unwrap() = m,
            None => turns.push(t.clone()),
        }
    }
    let mut input = Map::default();
    input.insert(Rc::from("turns"), Value::list(turns));
    input.insert(Rc::from("tools"), plan.tools.clone());
    if !plan.callable {
        input.insert(Rc::from("callable"), Value::Bool(false));
    }
    Ok((Value::map(input), plan.clone()))
}

/// A turn's text content as fragment items (a string or laid-out text; not content parts).
fn content_items(v: Option<&Value>) -> Option<Vec<Item>> {
    match v? {
        Value::Frag(items) => Some((**items).clone()),
        Value::Str(s) => Some(vec![Item::Text(s.to_rc())]),
        _ => None,
    }
}

/// Hosted content from already-laid-out tokens; also used to save the screen verbatim.
pub(crate) fn token_content(ids: &[u32], spans: &[Span], enc: &crate::emit::Encoder) -> Result<Value, String> {
    let dec = |a: usize, b: usize| {
        enc.tok
            .decode(ids.get(a..b).ok_or("an <img> span lies outside its turn")?, false)
            .map_err(|e| e.to_string())
    };
    let mut imgs: Vec<_> = spans.iter().filter(|s| s.meta.get("img").is_some()).collect();
    if imgs.is_empty() {
        Ok(Value::string(dec(0, ids.len())?))
    } else {
        // Hosted APIs take pixels as content parts; placeholder tokens carry none.
        imgs.sort_by_key(|s| s.start);
        let mut parts = Vec::new();
        let mut at = 0;
        for s in imgs {
            let text = dec(at, s.start)?;
            if !text.is_empty() {
                parts.push(serde_json::json!({"type": "text", "text": text}));
            }
            let element = s.meta.get("media").and_then(|v| v.as_str()).unwrap_or("img");
            let url = s
                .meta
                .get("src")
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("<{element}> in a messages turn needs src="))?;
            // The part a chat API takes for each kind: an image and a video by URL (a data URL
            // here), a sound as its bytes and format where that is wav or mp3 (the form every
            // OpenAI-compatible server takes) and by URL otherwise.
            parts.push(match element {
                "audio" => match url
                    .strip_prefix("data:audio/")
                    .and_then(|rest| rest.split_once(";base64,"))
                    .filter(|(format, _)| matches!(*format, "wav" | "mp3"))
                {
                    Some((format, data)) => {
                        serde_json::json!({"type": "input_audio", "input_audio": {"data": data, "format": format}})
                    }
                    None => serde_json::json!({"type": "audio_url", "audio_url": {"url": url}}),
                },
                "video" => serde_json::json!({"type": "video_url", "video_url": {"url": url}}),
                _ => serde_json::json!({"type": "image_url", "image_url": {"url": url}}),
            });
            at = s.end;
        }
        let text = dec(at, ids.len())?;
        if !text.is_empty() {
            parts.push(serde_json::json!({"type": "text", "text": text}));
        }
        Ok(Value::from_json(&serde_json::Value::Array(parts)))
    }
}

/// The hosted history after its `history` rule (see `messages_json`).
fn drop_history(
    mut messages: Vec<serde_json::Value>,
    enc: &crate::emit::Encoder,
    window: Option<usize>,
    rules: &[crate::style::Rule],
    ancestry: &crate::style::Ancestry,
) -> Result<serde_json::Value, String> {
    let style = crate::style::compute(rules, "history", &[], false, ancestry)?;
    let drop = match style.overflow {
        crate::style::Overflow::DropStart | crate::style::Overflow::DropChildren => crate::emit::Drop::Start,
        _ => return Ok(serde_json::Value::Array(messages)),
    };
    let Some(lim) = style.max_tokens.and_then(|s| match s {
        crate::emit::Size::Tokens(n) => Some(n),
        crate::emit::Size::Window(p) => window.map(|w| (w as u64 * p as u64 / 10_000) as usize),
    }) else {
        return Ok(serde_json::Value::Array(messages));
    };
    let start = usize::from(messages.first().is_some_and(|m| m["role"] == "system"));
    let end = messages.len().saturating_sub(1).max(start);
    // Units of the history [start, end): a message, or an assistant's calls with their results.
    let mut units: Vec<(usize, usize)> = vec![];
    for (i, m) in messages.iter().enumerate().take(end).skip(start) {
        let joins = m["role"] == "tool" && units.last().is_some();
        if joins {
            units.last_mut().unwrap().1 = i + 1
        } else {
            units.push((i, i + 1))
        }
    }
    let mut bounds = vec![0usize];
    for &(a, b) in &units {
        let tokens: usize = messages[a..b]
            .iter()
            .map(|m| enc.encode_run(&m.to_string(), false, false).len())
            .sum();
        bounds.push(bounds.last().unwrap() + tokens);
    }
    if units.is_empty() || *bounds.last().unwrap() <= lim {
        return Ok(serde_json::Value::Array(messages));
    }
    let step = style.drop_step.map_or(0, |s| s.tokens(lim));
    let (lo, _) = crate::emit::drop_cut(&bounds, lim, 0, drop, step);
    let cut = if lo < units.len() { units[lo].0 } else { end };
    messages.drain(start..cut);
    // What is left may begin with an assistant turn (an agent's one user message was the oldest).
    // Providers and chat templates expect the user first, and the model should know that earlier
    // turns are gone: a user message says so (the rule's `more`, or a default).
    if cut > start && messages.get(start).is_some_and(|m| m["role"] != "user") {
        let marker = style
            .more
            .as_deref()
            .map(str::trim_end)
            .filter(|m| !m.is_empty())
            .unwrap_or("[Earlier messages were dropped to fit the context window.]");
        messages.insert(start, serde_json::json!({"role": "user", "content": marker}));
    }
    Ok(serde_json::Value::Array(messages))
}

pub(crate) fn flatten(
    v: &Value,
    enc: &crate::emit::Encoder,
    window: Option<usize>,
    position: &mut Option<(usize, usize)>,
) -> Result<Value, String> {
    use crate::emit::{Live, Out, RunCache};
    Ok(match v {
        Value::Frag(_) => {
            let mut cap = Capture::default();
            emit_child(v, &mut cap)?;
            // Nothing in it is laid out differently from its text: the text as written, as a
            // browser's Node.textContent reads it. Laying it out through the tokenizer and back
            // would lose what the tokenizer cannot write (a private-use character a template app
            // marks held content with), and the model's own template hands its tokenizer the
            // raw string too.
            if let Some(text) = written_text(&cap.items, enc)? {
                return Ok(Value::string(text));
            }
            let mut cache = RunCache::new(false);
            let mut live = Live::new(enc, &mut cache);
            live.window = window;
            // Graded results count across the history's messages (see `messages_json`).
            if let Some(p) = *position {
                live.positions.push(p);
            }
            live.start_in_prompt();
            live.replay(&cap.items)?;
            if position.is_some() {
                *position = live.positions.pop();
            }
            let (ids, spans) = live.finish()?;
            token_content(&ids, &spans, enc)?
        }
        v => v.clone(),
    })
}

/// The text a fragment is written as, or `None` when something in it is laid out differently
/// from its text: a box (a limit, a size, a drop), something hidden, a picture, a turn plan.
fn written_text(items: &[crate::emit::Item], enc: &crate::emit::Encoder) -> Result<Option<String>, String> {
    use crate::emit::{Item, SpanKind};
    let decode = |ids: &[u32]| enc.tok.decode(ids, false).map_err(|e| e.to_string());
    let mut out = String::new();
    for item in items {
        match item {
            Item::Text(t) => out.push_str(t),
            Item::Tok(id) => out.push_str(&decode(&[*id])?),
            Item::Open(spec) => {
                if !matches!(spec.kind, SpanKind::Label | SpanKind::Affordance) || spec.meta.get("img").is_some() {
                    return Ok(None);
                }
            }
            Item::Close | Item::Segment => {}
            Item::Tokens(ids, spans) => {
                if spans.iter().any(|s| s.meta.get("img").is_some()) {
                    return Ok(None);
                }
                out.push_str(&decode(ids)?);
            }
            Item::Turns(_) => return Ok(None),
        }
    }
    Ok(Some(out))
}

/// The model-callable actions on a plan's screen, as zero-width spans. They belong to the screen,
/// however a template writes it: a template that writes the screen as text (a decision model's
/// state, a hosted API's JSON) still offers them, as a page keeps its listeners in any rendering.
/// The template app element lays out those its output does not already carry (see
/// `Interp::template_actions`).
pub(crate) fn screen_actions(plan: &TurnPlan) -> Vec<Span> {
    plan.actions.clone()
}

/// The handler references of the affordances a value carries, laid out or not.
pub(crate) fn handlers_in(v: &Value, out: &mut std::collections::HashSet<u64>) {
    match v {
        Value::Frag(items) => {
            for item in items.iter() {
                match item {
                    Item::Tokens(_, spans) => out.extend(spans.iter().filter_map(|s| s.meta["handler"].as_u64())),
                    Item::Open(spec) => out.extend(spec.meta["handler"].as_u64()),
                    _ => {}
                }
            }
        }
        Value::List(l) => l.read().iter().for_each(|x| handlers_in(x, out)),
        _ => {}
    }
}

/// Generic messages protocol for hosted APIs: the same turn plan a template app
/// consumes, with fragment content flattened to text and the tool table as given.
///
/// Layout runs before serialization: a `history`
/// rule drops whole units from the start, as it does for a token template. A unit is a message,
/// or an assistant's calls with all their results; the system message and the current screen
/// are never dropped. Units are measured with the bound tokenizer on their JSON: the provider's
/// own count differs, so the cap is approximate here.
pub fn messages_json(
    input: &Value,
    enc: &crate::emit::Encoder,
    window: Option<usize>,
    rules: &[crate::style::Rule],
    ancestry: &crate::style::Ancestry,
) -> Result<Value, String> {
    let Value::Map(m) = input else {
        return Err("messagesJson needs templateInput output".into());
    };
    let Some(Value::List(turns)) = m.read().get("turns").cloned() else {
        return Err("messagesJson input has no turns".into());
    };
    let mut out = Vec::with_capacity(turns.read().len());
    // Graded results (`result:nth-last`) are counted among the history's messages: every turn
    // but the screen's.
    let count = |v: &Value| {
        match v { Value::Frag(f) => f.iter().filter(|it| matches!(it, Item::Open(s) if matches!(&s.kind, crate::emit::SpanKind::Box { graded: Some(_), .. }))).count(), _ => 0 }
    };
    let all = turns.read();
    // The current screen, when there is one, is the last turn: its content as sent is what the
    // log records for it. A screen with nothing on it takes no turn: every turn is then history.
    let screen = all.last().is_some_and(|t| match t {
        Value::Map(t) => match t.read().get("content") {
            Some(Value::Frag(f)) => f.iter().any(|i| match i {
                Item::Tokens(_, spans) => spans.iter().any(|s| s.label == CURRENT_TURN),
                _ => false,
            }),
            _ => false,
        },
        _ => false,
    });
    let history = all.len() - usize::from(screen);
    let graded: usize = all
        .iter()
        .take(history)
        .filter_map(|t| match t {
            Value::Map(m) => Some(m.read().values().map(count).sum::<usize>()),
            _ => None,
        })
        .sum();
    let mut running = (graded > 0).then_some((graded, 0));
    for (j, t) in all.iter().enumerate() {
        let Value::Map(t) = t else {
            return Err("turn must be an object".into());
        };
        let mut none = None;
        let position = if j < history { &mut running } else { &mut none };
        let mut n = Map::default();
        for (k, v) in t.read().iter() {
            n.insert(k.clone(), flatten(v, enc, window, position)?);
        }
        out.push(Value::map(n));
    }
    let sent = match out.last() {
        Some(Value::Map(t)) if screen => t.read().get("content").map(Value::to_json),
        _ => None,
    };
    drop(all);
    let messages = drop_history(out.iter().map(Value::to_json).collect(), enc, window, rules, ancestry)?;
    let mut doc = serde_json::json!({
        "messages": messages,
        "tools": m.read().get("tools").map(Value::to_json).unwrap_or(serde_json::Value::Null),
    });
    if !tools_callable(&m.read())? {
        doc["callable"] = serde_json::json!(false);
    }
    // The JSON line is what a hosted API is sent: a `hosted` span marks it, so the window check
    // and the layout event count it, not the screen's actions after it, and carries the screen.
    Ok(Value::Frag(Rc::new(vec![
        Item::Open(Rc::new(crate::emit::SpanSpec {
            label: Rc::from("hosted"),
            kind: crate::emit::SpanKind::Label,
            meta: sent.map_or(serde_json::Value::Null, |c| serde_json::json!({"screen": c})),
        })),
        Item::Text(Rc::from(format!("{}\n", doc))),
        Item::Close,
    ])))
}
