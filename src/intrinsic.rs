//! Native functions retain their defining prototype independently of their call receiver.
use crate::{ast::Sym, value::Value};
use std::{fmt, ops::Deref, rc::Rc};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Prototype {
    Object,
    Array,
    Map,
    Set,
    Error,
    String,
    Number,
    Boolean,
    RegExp,
    Function,
    Fragment,
    Promise,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Native {
    pub owner: Option<Prototype>,
    pub name: Sym,
}
impl From<Sym> for Native {
    fn from(name: Sym) -> Self {
        Self { owner: None, name }
    }
}
impl Deref for Native {
    type Target = str;
    fn deref(&self) -> &str {
        &self.name
    }
}
impl fmt::Display for Native {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.name.fmt(f)
    }
}
impl Native {
    pub(crate) fn is_namespace(&self) -> bool {
        self.owner.is_none() && matches!(&**self, "Math" | "JSON")
    }
    pub fn method(owner: Prototype, name: &str) -> Value {
        let name = match (owner, name) {
            (Prototype::String, "trimLeft") => "trimStart",
            (Prototype::String, "trimRight") => "trimEnd",
            (Prototype::Set, "keys") => "values",
            _ => name,
        };
        Value::Native(Self {
            owner: Some(owner),
            name: Rc::from(name),
        })
    }
}

pub const OBJECT: &[&str] = &[
    "hasOwnProperty",
    "toString",
    "valueOf",
    "isPrototypeOf",
    "propertyIsEnumerable",
    "toLocaleString",
];
pub const ARRAY: &[&str] = &[
    "at",
    "concat",
    "entries",
    "every",
    "fill",
    "filter",
    "find",
    "findIndex",
    "findLast",
    "findLastIndex",
    "flat",
    "flatMap",
    "forEach",
    "includes",
    "indexOf",
    "join",
    "keys",
    "lastIndexOf",
    "map",
    "pop",
    "push",
    "reduce",
    "reduceRight",
    "reverse",
    "shift",
    "slice",
    "some",
    "sort",
    "splice",
    "toReversed",
    "toSorted",
    "toSpliced",
    "toString",
    "unshift",
    "values",
    "with",
];
const MAP: &[&str] = &[
    "get", "set", "has", "delete", "clear", "forEach", "keys", "values", "entries",
];
const SET: &[&str] = &["add", "has", "delete", "clear", "forEach", "keys", "values", "entries"];
const STRING: &[&str] = &[
    "trim",
    "trimStart",
    "trimEnd",
    "trimLeft",
    "trimRight",
    "toLowerCase",
    "toLocaleLowerCase",
    "toUpperCase",
    "toLocaleUpperCase",
    "toString",
    "valueOf",
    "startsWith",
    "endsWith",
    "includes",
    "indexOf",
    "lastIndexOf",
    "charAt",
    "charCodeAt",
    "codePointAt",
    "at",
    "substring",
    "substr",
    "slice",
    "padStart",
    "padEnd",
    "repeat",
    "concat",
    "split",
    "replace",
    "replaceAll",
    "match",
    "matchAll",
    "search",
    "localeCompare",
];

pub fn member(value: &Value, name: &str) -> Option<Value> {
    use Prototype::*;
    let specific = match value {
        Value::Promise(_) if ["then", "catch", "finally"].contains(&name) => Some(Promise),
        Value::Map(m) if m.class().is_some_and(|c| c.error_kind().is_some()) && name == "toString" => Some(Error),
        Value::List(_) if ARRAY.contains(&name) => Some(Array),
        Value::Coll(c) if (if c.read().set { SET } else { MAP }).contains(&name) => {
            Some(if c.read().set { Set } else { Map })
        }
        Value::Str(_) if STRING.contains(&name) => Some(String),
        Value::Int(_) | Value::Float(_) | Value::BigInt(_) if ["toString", "toFixed", "valueOf"].contains(&name) => {
            Some(Number)
        }
        Value::Bool(_) if ["toString", "valueOf"].contains(&name) => Some(Boolean),
        Value::Regex(_) if ["toString", "exec", "test"].contains(&name) => Some(RegExp),
        Value::Frag(_)
            if [
                "slice",
                "at",
                "includes",
                "trim",
                "trimStart",
                "trimEnd",
                "startsWith",
                "endsWith",
                "indexOf",
                "lastIndexOf",
            ]
            .contains(&name) =>
        {
            Some(Fragment)
        }
        v if v.is_callable() && ["call", "apply", "bind", "toString"].contains(&name) => Some(Function),
        _ => None,
    };
    specific
        .or_else(|| (!matches!(value, Value::None | Value::Undef) && OBJECT.contains(&name)).then_some(Object))
        .map(|owner| Native::method(owner, name))
}

impl crate::interp::Interp<'_> {
    pub(crate) fn object_method(
        &mut self,
        receiver: &Value,
        name: &str,
        args: &[Value],
    ) -> crate::interp::RResult<Value> {
        if name == "toString" {
            let tag = match receiver {
                Value::Undef => "Undefined",
                Value::None => "Null",
                Value::List(_) => "Array",
                Value::Coll(c) => {
                    if c.read().set {
                        "Set"
                    } else {
                        "Map"
                    }
                }
                Value::Map(m) if m.class().is_some_and(|c| c.error_kind().is_some()) => "Error",
                Value::Str(_) => "String",
                Value::Bool(_) => "Boolean",
                Value::Int(_) | Value::Float(_) | Value::BigInt(_) => "Number",
                Value::Regex(_) => "RegExp",
                Value::Promise(_) => "Promise",
                Value::Native(n) if n.owner.is_none() && matches!(&**n, "Math" | "JSON") => &n.name,
                v if v.is_callable() => "Function",
                _ => "Object",
            };
            return Ok(Value::string(format!("[object {tag}]")));
        }
        if matches!(receiver, Value::None | Value::Undef) {
            return Err("TypeError: Cannot convert undefined or null to object".into());
        }
        if name == "valueOf" {
            return if !receiver.is_object() {
                Err("primitive wrapper objects are not supported".into())
            } else {
                Ok(receiver.clone())
            };
        }
        if name == "toLocaleString" {
            let function = self.get_prop(receiver, "toString")?;
            return self.call_with_receiver(&function, receiver.clone(), vec![]);
        }
        if name == "isPrototypeOf" {
            let mut at = match args.first() {
                Some(Value::Class(c)) => c.parent.clone(),
                _ => None,
            };
            while let Some(c) = at {
                if matches!(receiver,Value::Class(owner) if Rc::ptr_eq(owner,&c)) {
                    return Ok(Value::Bool(true));
                }
                at = c.parent.clone();
            }
            return Ok(Value::Bool(false));
        }
        let key = crate::jsvalue::try_to_string(args.first().unwrap_or(&Value::Undef))?;
        let enumerable = name == "propertyIsEnumerable";
        let own = match receiver {
            Value::Map(m) => {
                m.read().contains_key(key.as_str())
                    || !enumerable && m.ext().is_some_and(|e| e.hidden.borrow().contains_key(key.as_str()))
            }
            Value::List(l) => {
                key == "length" && !enumerable
                    || crate::jsvalue::array_index_of(&Value::str(&key)).is_some_and(|i| i < l.read().len())
            }
            Value::Str(s) => {
                key == "length" && !enumerable
                    || crate::jsvalue::array_index_of(&Value::str(&key)).is_some_and(|i| i < s.char_count())
            }
            Value::Class(c) => {
                c.statics.read().contains_key(key.as_str())
                    && (!enumerable || !c.static_methods.contains_key(key.as_str()))
            }
            Value::Regex(_) => key == "lastIndex" && !enumerable,
            Value::Native(n) if n.owner.is_none() => {
                crate::jsbuiltins::is_global_function(n, &key) || crate::jsbuiltins::global_constant(n, &key).is_some()
            }
            _ => false,
        };
        Ok(Value::Bool(own))
    }
}
