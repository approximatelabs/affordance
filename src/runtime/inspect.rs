//! `harness.inspect`: a read-only copy of other instances' committed state.
//! The copy of each target is made here, on the target's worker, between its inputs; the host
//! gathers the copies and answers the caller's call. Nothing is written to the target's journal.
//!
//! A copy is what `select` names, encoded as JSON with a tag where JSON cannot say what the value
//! is (`{"$": "map", ...}`), so a `Map` does not arrive as `{}`. With a cursor from an earlier read
//! (`since`), an unchanged target with every selection covered answers `changed: false`; a hook that has not
//! changed is left out, and a list selected with `{key, path?, tail?}` sends only what it gained.
use super::agent::Agent;
use crate::value::{Map, Text, Value};
use serde_json::{Value as J, json};

/// One entry of `select`: a named hook (`useState(init, "name")`, `useLog()`'s `log`) or `props`, and for the object form, a path into it and
/// how many items of a list to send the first time.
pub struct Sel {
    key: String,
    path: Vec<String>,
    tail: Option<usize>,
    follow: bool,
}

impl Sel {
    /// Cursor coverage belongs to this exact selection, including its initial list window.
    fn name(&self) -> String {
        json!([self.key, self.path, self.follow, self.tail]).to_string()
    }
}

/// Parse an `inspect` call's `select`: which hooks, props and paths to copy.
pub fn parse_select(j: &J) -> Result<Vec<Sel>, String> {
    let items = match j {
        J::Null => return Ok(vec![]),
        J::Array(a) => a.clone(),
        other => vec![other.clone()],
    };
    items
        .iter()
        .map(|s| match s {
            J::String(k) => Ok(Sel {
                key: k.clone(),
                path: vec![],
                tail: None,
                follow: false,
            }),
            J::Object(o) => {
                let key = o
                    .get("key")
                    .and_then(J::as_str)
                    .ok_or("inspect: a select entry {key, path?, tail?} needs key")?
                    .to_string();
                let path = match o.get("path") {
                    None | Some(J::Null) => vec![],
                    Some(J::String(p)) => p.split('.').filter(|s| !s.is_empty()).map(str::to_string).collect(),
                    Some(J::Array(a)) => a
                        .iter()
                        .map(|x| {
                            x.as_str()
                                .map(str::to_string)
                                .ok_or("inspect: path entries are strings")
                        })
                        .collect::<Result<_, _>>()?,
                    Some(_) => return Err("inspect: path is a string (\"records\") or a list of keys".into()),
                };
                let tail = match o.get("tail") {
                    None | Some(J::Null) => None,
                    Some(t) => Some(t.as_u64().ok_or("inspect: tail is a count")? as usize),
                };
                Ok(Sel {
                    key,
                    path,
                    tail,
                    follow: true,
                })
            }
            _ => Err("inspect: select is a list of hook keys and {key, path?, tail?}".into()),
        })
        .collect()
}

/// The copy of one agent: `({version, changed, changes?}, its next cursor)`. `text`: blob
/// references become their text (the copy leaves the runtime as one string).
pub fn part(agent: &Agent, select: &[Sel], cursor: &J, text: bool) -> (J, J) {
    match part_checked(agent, select, cursor, text) {
        Ok(part) => part,
        Err(error) => (json!({"version": agent.journal_len(), "error": error}), cursor.clone()),
    }
}

fn part_checked(agent: &Agent, select: &[Sel], cursor: &J, text: bool) -> Result<(J, J), String> {
    let version = agent.journal_len();
    let since = cursor["v"].as_u64().map(|v| v as usize);
    let state = agent.kept_state();
    let previous = cursor["n"].as_object();
    let mut lengths = serde_json::Map::new();
    let mut changes = vec![];
    for sel in select {
        let hook = sel.key != "props";
        let name = sel.name();
        let old = previous.and_then(|n| n.get(&name));
        // Unchanged since the reader's copy (and the reader has this entry: `select` may grow).
        // A named hook, `useState(init, "name")`, is kept as `@name`.
        let kept = format!("@{}", sel.key);
        if since.is_some_and(|v| agent.changed_at(hook.then_some(kept.as_str())) <= v)
            && let Some(old) = old
        {
            lengths.insert(name, old.clone());
            continue;
        }
        let root = if hook {
            lookup(state.get("__ui"), &kept)
        } else {
            Some(Value::Map(std::rc::Rc::new(crate::value::Obj::new(props(state)))))
        };
        let value = sel.path.iter().fold(root, |v, k| lookup(v.as_ref(), k));
        let mut change = json!({"key": sel.key});
        if !sel.path.is_empty() {
            change["path"] = json!(sel.path);
        }
        match value {
            Some(Value::List(l)) if sel.follow => {
                let items = l.read();
                let len = items.len();
                // What the reader has: the length it last saw, or (the first time) all but the tail.
                let from = match old.and_then(J::as_u64) {
                    Some(n) if (n as usize) <= len => n as usize,
                    Some(_) => 0,
                    None => len - sel.tail.unwrap_or(len).min(len),
                };
                if old.is_some() && from == len {
                    lengths.insert(name, json!(len));
                    continue;
                }
                change["from"] = json!(from);
                change["items"] = J::Array(
                    items[from..]
                        .iter()
                        .map(|v| encode(v, text))
                        .collect::<Result<_, _>>()?,
                );
                change["length"] = json!(len);
                lengths.insert(name, json!(len));
            }
            Some(v) => {
                change["set"] = encode(&v, text)?;
                lengths.insert(name, J::Null);
            }
            None => {
                change["set"] = json!({"$": "undefined"});
                lengths.insert(name, J::Null);
            }
        }
        changes.push(change);
    }
    let changed = since != Some(version) || !changes.is_empty();
    let mut part = json!({"version": version, "changed": changed});
    if changed {
        part["changes"] = json!(changes);
    }
    Ok((part, json!({"v": version, "n": lengths})))
}

fn props(state: &Map) -> Map {
    state
        .iter()
        .filter(|(k, _)| &***k != "__ui")
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

fn lookup(v: Option<&Value>, key: &str) -> Option<Value> {
    match v? {
        Value::Map(m) => m.read().get(key).cloned(),
        Value::List(l) => key.parse::<usize>().ok().and_then(|i| l.read().get(i).cloned()),
        _ => None,
    }
}

/// A kept value as JSON, tagged where JSON has no word for it. A plain object with a `$` key is
/// itself tagged, so a tag is never ambiguous.
pub fn encode(v: &Value, text: bool) -> Result<J, String> {
    encode_inner(v, text, &mut Vec::new())
}

fn encode_inner(v: &Value, text: bool, path: &mut Vec<usize>) -> Result<J, String> {
    if path.len() >= 128 {
        return Err("inspect: value nesting exceeds 128 levels".into());
    }
    let id = match v {
        Value::List(l) => Some(std::rc::Rc::as_ptr(l) as usize),
        Value::Map(m) => Some(std::rc::Rc::as_ptr(m) as usize),
        Value::Coll(c) => Some(std::rc::Rc::as_ptr(c) as usize),
        _ => None,
    };
    if let Some(id) = id {
        if path.contains(&id) {
            return Err("inspect: circular value".into());
        }
        path.push(id);
    }
    let result = stacker::maybe_grow(128 * 1024, 2 * 1024 * 1024, || encode_one(v, text, path));
    if id.is_some() {
        path.pop();
    }
    result
}

fn encode_one(v: &Value, text: bool, path: &mut Vec<usize>) -> Result<J, String> {
    Ok(match v {
        Value::Undef => json!({"$": "undefined"}),
        Value::None => J::Null,
        Value::Bool(b) => json!(b),
        Value::Int(i) => json!(i),
        Value::Float(f) if f.is_finite() => json!(f),
        Value::Float(f) => {
            json!({"$": "number", "value": if f.is_nan() { "NaN" } else if *f > 0.0 { "Infinity" } else { "-Infinity" }})
        }
        Value::Str(Text::Host(h)) if text => J::String(h.as_str().to_string()),
        Value::Str(_) | Value::BigInt(_) | Value::Frag(_) => v.to_wire(),
        Value::List(l) => J::Array(
            l.read()
                .iter()
                .map(|x| encode_inner(x, text, path))
                .collect::<Result<_, _>>()?,
        ),
        Value::Map(m) => {
            let fields: serde_json::Map<String, J> = m
                .read()
                .iter()
                .map(|(k, x)| Ok((k.to_string(), encode_inner(x, text, path)?)))
                .collect::<Result<_, String>>()?;
            match m.class() {
                Some(c) => json!({"$": "instance", "class": &*c.name, "fields": fields}),
                None if fields.contains_key("$") => json!({"$": "object", "fields": fields}),
                None => J::Object(fields),
            }
        }
        Value::Coll(c) => {
            let c = c.read();
            let live = c.entries.iter().flatten();
            if c.set {
                json!({"$": "set", "values": live.map(|(k, _)| encode_inner(k, text, path)).collect::<Result<Vec<_>, _>>()?})
            } else {
                json!({"$": "map", "entries": live.map(|(k, x)| Ok(json!([encode_inner(k, text, path)?, encode_inner(x, text, path)?]))).collect::<Result<Vec<_>, String>>()?})
            }
        }
        Value::Regex(r) => json!({"$": "regexp", "source": r.source, "flags": r.flags}),
        Value::Token(_, name) => json!({"$": "token", "name": &**name}),
        Value::Promise(_) => json!({"$": "promise"}),
        Value::Native(n) if n.is_namespace() => json!({"$": "object", "fields": {}}),
        Value::Method(_)
        | Value::Native(_)
        | Value::Lambda(..)
        | Value::Func(_)
        | Value::Setter(_)
        | Value::Bound(_)
        | Value::Class(_) => json!({"$": "function"}),
    })
}
