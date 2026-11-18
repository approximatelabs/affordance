//! JSON Schema from a TypeScript type (`schemaOf(fn)`: a tool's arguments are its props
//! parameter's type). The mapping is fixed, so a schema's bytes, which are prompt bytes, are too:
//!
//! - `string`, `number`, `boolean`: that type; `T[]` and `Array<T>`: an array of T;
//! - a union of string literals: a string `enum`, in source order; one string literal: an
//!   `enum` of one;
//! - `T | null`: `type: [T, "null"]` (still required; optional is `x?: T`);
//! - an object type: `properties` in declaration order, `required` the members without `?`, in
//!   declaration order (omitted when empty);
//! - an alias (`type X = ...` or `interface X {...}`, in the defining module): its type.
//!
//! Keys come in the order `type`, `enum`, `items`, `properties`, `required`. Anything else
//! (generics, `any`, `unknown`, functions, an interface that extends another, recursion) is an
//! error naming the type: give the tool an explicit `schema`.
use crate::tsx::TsType;
use serde_json::{Map, Value as J, json};
use std::collections::HashMap;

pub fn schema_of(t: &TsType, aliases: &HashMap<String, TsType>) -> Result<J, String> {
    schema(t, aliases, 0)
}

fn describe(t: &TsType) -> String {
    match t {
        TsType::Ref(n, args) if args.is_empty() => n.clone(),
        TsType::Ref(n, _) => format!("{n}<...>"),
        TsType::Lit(v) => v.to_string(),
        TsType::Object(_) => "an object type".into(),
        TsType::Array(t) => format!("{}[]", describe(t)),
        TsType::Tuple(_) => "a tuple".into(),
        TsType::Union(ts) => ts.iter().map(describe).collect::<Vec<_>>().join(" | "),
        TsType::Intersection(_) => "an intersection".into(),
        TsType::Func => "a function type".into(),
        TsType::Other => "this type".into(),
    }
}

fn schema(t: &TsType, aliases: &HashMap<String, TsType>, depth: usize) -> Result<J, String> {
    if depth > 32 {
        return Err("a recursive type has no schema; give the tool an explicit schema".into());
    }
    let unsupported = || {
        Err(format!(
            "{} has no schema (types map: string, number, boolean, arrays, object types, string-literal unions, T | null); give the tool an explicit schema",
            describe(t)
        ))
    };
    Ok(match t {
        TsType::Ref(n, args) => match (n.as_str(), args.as_slice()) {
            ("string", []) => json!({"type": "string"}),
            ("number", []) => json!({"type": "number"}),
            ("boolean", []) => json!({"type": "boolean"}),
            ("Array" | "ReadonlyArray", [item]) => json!({"type": "array", "items": schema(item, aliases, depth + 1)?}),
            (name, []) => match aliases.get(name) {
                Some(TsType::Other) => {
                    return Err(format!(
                        "{name} has no schema (an interface that extends another); give the tool an explicit schema"
                    ));
                }
                Some(a) => schema(a, aliases, depth + 1)?,
                None => return unsupported(),
            },
            _ => return unsupported(),
        },
        TsType::Lit(v @ J::String(_)) => json!({"type": "string", "enum": [v]}),
        TsType::Array(item) => json!({"type": "array", "items": schema(item, aliases, depth + 1)?}),
        TsType::Union(ts) => {
            let nullable = ts
                .iter()
                .any(|x| matches!(x, TsType::Ref(n, a) if n == "null" && a.is_empty()));
            let rest: Vec<&TsType> = ts
                .iter()
                .filter(|x| !matches!(x, TsType::Ref(n, a) if n == "null" && a.is_empty()))
                .collect();
            let base = if rest.iter().all(|x| matches!(x, TsType::Lit(J::String(_)))) {
                json!({"type": "string", "enum": rest.iter().map(|x| match x { TsType::Lit(v) => v.clone(), _ => unreachable!() }).collect::<Vec<_>>()})
            } else if let [one] = rest.as_slice() {
                schema(one, aliases, depth + 1)?
            } else {
                return unsupported();
            };
            if !nullable {
                base
            } else {
                let mut m = base.as_object().cloned().unwrap_or_default();
                if let Some(J::Array(values)) = m.get_mut("enum") {
                    values.push(J::Null);
                }
                match m.get("type").cloned() {
                    Some(J::String(ty)) => {
                        m.insert("type".into(), json!([ty, "null"]));
                    }
                    _ => return unsupported(),
                }
                J::Object(m)
            }
        }
        TsType::Object(members) => {
            let mut props = Map::new();
            let mut required = vec![];
            for m in members {
                props.insert(m.name.clone(), schema(&m.ty, aliases, depth + 1)?);
                if !m.optional {
                    required.push(json!(m.name));
                }
            }
            let mut o = Map::new();
            o.insert("type".into(), json!("object"));
            o.insert("properties".into(), J::Object(props));
            if !required.is_empty() {
                o.insert("required".into(), J::Array(required));
            }
            J::Object(o)
        }
        _ => return unsupported(),
    })
}
