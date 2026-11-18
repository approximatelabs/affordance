//! Python and Jinja idioms that are not in the view language, and what to write instead.
//!
//! One table drives the parser's errors (`x[a:b]`, `f(k=v)`, `not in`), the runtime's errors
//! (`enumerate is not defined`, `string.strip is not a function`) and `affordance lint`, which flags the
//! idioms in source files before they fail a render. `spec/LANGUAGE.md` §11 lists the same table
//! (a test keeps them in step).

/// Python/Jinja builtins that are not functions here, with the JavaScript to write instead.
pub const FUNCTIONS: &[(&str, &str)] = &[
    (
        "str",
        "String(x), or a template literal `${x}` (formatPythonValue(x) for Python's text)",
    ),
    ("len", "x.length (Object.keys(o).length for an object)"),
    ("int", "parseInt(s) or Math.trunc(x)"),
    ("float", "Number(x)"),
    ("bool", "Boolean(x)"),
    ("repr", "formatPythonRepr(x)"),
    (
        "json",
        "JSON.stringify(x), or formatJSON(x, {indent, separators, ascii, sortKeys}) for Python's json.dumps text",
    ),
    (
        "tojson",
        "JSON.stringify(x), or formatJSON(x) for Python's json.dumps text",
    ),
    ("parse_json", "JSON.parse(s)"),
    ("try_parse_json", "JSON.parse(s) inside try { } catch { }"),
    ("enumerate", "xs.entries() (for (const [i, x] of xs.entries()))"),
    (
        "range",
        "for (let i = 0; i < n; i++), or Array.from({length: n}, (_, i) => i)",
    ),
    ("items", "Object.entries(o)"),
    ("keys", "Object.keys(o)"),
    ("values", "Object.values(o)"),
    ("dict", "{...o, k: v}"),
    ("list", "[...x] or Array.from(x)"),
    ("reversed", "[...xs].reverse()"),
    ("sorted", "[...xs].sort((a, b) => ...)"),
    ("sort_by", "[...xs].sort((a, b) => ...)"),
    ("join", "xs.join(sep)"),
    ("first", "xs[0]"),
    ("last", "xs.at(-1)"),
    ("default", "x ?? d"),
    ("merge", "{...a, ...b}"),
    ("omit", "const {k, ...rest} = o"),
    ("min", "Math.min(...xs)"),
    ("max", "Math.max(...xs)"),
    ("abs", "Math.abs(x)"),
    ("sum", "xs.reduce((a, b) => a + b, 0)"),
    ("any", "xs.some(x => ...)"),
    ("all", "xs.every(x => ...)"),
    ("raise", "throw new Error(msg)"),
    ("trim", "s.trim()"),
    ("strftime_now", "formatNow(fmt)"),
    ("count_tokens", "countTokens(s)"),
    ("is_fragment", "isFragment(x)"),
    ("is_string", "typeof x === \"string\""),
    ("is_number", "typeof x === \"number\""),
    ("is_int", "Number.isInteger(x)"),
    ("is_bool", "typeof x === \"boolean\""),
    ("is_map", "isPlainObject(x)"),
    ("is_list", "Array.isArray(x)"),
    ("is_sequence", "Array.isArray(x)"),
    ("is_none", "x === null"),
    ("is_defined", "x !== undefined"),
    ("is_undefined", "x === undefined"),
];

/// Python string/list/dict methods that are not methods here.
pub const METHODS: &[(&str, &str)] = &[
    ("strip", ".trim() (trimChars(s, chars) to strip given characters)"),
    ("lstrip", ".trimStart() (trimStartChars(s, chars))"),
    ("rstrip", ".trimEnd() (trimEndChars(s, chars))"),
    ("startswith", ".startsWith(x)"),
    ("endswith", ".endsWith(x)"),
    ("lower", ".toLowerCase()"),
    ("upper", ".toUpperCase()"),
    ("title", "titleCase(s)"),
    ("splitlines", ".split(\"\\n\") (or .split(/\\r?\\n/))"),
    ("rsplit", ".split(sep) and take from the end"),
    ("rfind", ".lastIndexOf(x)"),
    ("isdigit", "/^\\d+$/.test(s)"),
    ("isalpha", "/^[a-zA-Z]+$/.test(s)"),
    ("isalnum", "/^[a-zA-Z0-9]+$/.test(s)"),
    ("isspace", "/^\\s+$/.test(s)"),
    ("format", "a template literal `${x}`"),
    ("items", "Object.entries(o)"),
    ("keys", "Object.keys(o)"),
    ("values", "Object.values(o)"),
    ("get", "o[k] ?? d"),
    ("append", ".push(x)"),
    ("extend", ".push(...xs)"),
    ("count", "s.split(x).length - 1, or xs.filter(y => y === x).length"),
    ("index", ".indexOf(x)"),
];

/// Names Python spells differently from JavaScript.
pub const NAMES: &[(&str, &str)] = &[("True", "true"), ("False", "false"), ("None", "null")];

pub fn function(name: &str) -> Option<&'static str> {
    FUNCTIONS.iter().find(|(n, _)| *n == name).map(|(_, h)| *h)
}

pub fn method(name: &str) -> Option<&'static str> {
    METHODS.iter().find(|(n, _)| *n == name).map(|(_, h)| *h)
}

pub fn name(name: &str) -> Option<&'static str> {
    NAMES.iter().find(|(n, _)| *n == name).map(|(_, h)| *h)
}

/// Methods that are certainly Python (`.strip()`): lint flags them on any receiver. The others in
/// `METHODS` (`get`, `count`, `index`, ...) may be an object's own function member, so lint
/// leaves them to the runtime error.
pub fn certain_method(name: &str) -> bool {
    !matches!(
        name,
        "get" | "count" | "index" | "format" | "append" | "extend" | "items" | "keys" | "values"
    )
}

pub const SLICE: &str = "Python slice syntax `x[a:b]` is not supported; use x.slice(a, b) (x.slice(-n) for the last n, [...x].reverse() for x[::-1])";
pub const KWARG: &str = "keyword arguments `f(k=v)` are not supported; pass positional arguments or an object: f(x, {k: v}) ({...o, k: v} for Python's dict(o, k=v))";
pub const NOT_IN: &str = "Python `not in` is not supported; use !(k in obj), or !xs.includes(x) / !s.includes(x)";
pub const AND_OR: &str = "Python `and` / `or` / `not` are not operators here; use && / || / !";
pub const LIST_ADD: &str =
    "`+` with an array joins strings in JavaScript ([1] + [2] is \"12\"); use [...a, ...b] or xs.concat(ys)";

/// A browser or Node global this platform does not have, and what an app uses instead.
pub fn platform(name: &str) -> Option<&'static str> {
    Some(match name {
        "Date" => {
            "there is no Date object: Date.now() is the clock in milliseconds, and formatNow(\"%Y-%m-%d\") prints it"
        }
        "setTimeout" | "setInterval" => "there are no timers: await harness.wait({ms}) in a handler",
        "process" => "there is no process: the instance's environment is harness.env",
        "window" | "document" | "globalThis" => {
            "there is no DOM: an app renders elements and keeps state with useState"
        }
        "require" => "modules are imported: import {x} from \"./file.tsx\"",
        "localStorage" | "sessionStorage" => "state is useState/useRef; durable text is harness.store",
        "XMLHttpRequest" => "use fetch",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    /// The spec's migration table (§11) names every removed function and method the code knows, so
    /// the documented surface and the implementation cannot drift apart.
    #[test]
    fn the_spec_migration_table_covers_the_code() {
        let spec =
            std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("spec/LANGUAGE.md")).unwrap();
        let section = &spec[spec.find("## 11. Python idioms").expect("spec §11")..];
        let mut missing = vec![];
        for (name, _) in super::FUNCTIONS {
            if !section.contains(&format!("`{name}(")) {
                missing.push(format!("{name}(..)"));
            }
        }
        for (name, _) in super::METHODS {
            if !section.contains(&format!("`.{name}(")) {
                missing.push(format!(".{name}(..)"));
            }
        }
        assert!(missing.is_empty(), "spec/LANGUAGE.md §11 lacks: {missing:?}");
    }
}
