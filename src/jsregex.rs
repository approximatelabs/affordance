//! JavaScript regular expressions on the Rust `regex` crate.
//!
//! `/src/flags` literals and `new RegExp(src, flags)` compile once into a `Value::Regex`.
//! The JS pattern is translated to the `regex` crate's syntax so that the common
//! subset matches as in JavaScript: `.` excludes all four JS line terminators unless
//! `s`; `\d`, `\w`, `\b` are ASCII as in JS; `\s` is the JS whitespace set; `[]` and
//! `[^]` mean nothing and anything; `\uXXXX`, `\u{X}`, `\xHH`, `\cX`, `(?<name>..)` work.
//!
//! Supported flags: `g i m s u`. Not supported (load or construction error, never a
//! silent difference): lookahead `(?=` `(?!`, lookbehind `(?<=` `(?<!`, backreferences
//! `\1` `\k<name>`, and the `y` (sticky), `d` (indices) and `v` flags. The `regex` crate
//! is a finite-automaton engine; it has none of these by design.
//!
//! Other differences: regex values are immutable, so `lastIndex` is always 0 and `exec`
//! / `test` always search from the start (use `matchAll`, or `match` with `g`, for all
//! matches). Match arrays are plain arrays: they have no `index`, `input` or `groups`
//! properties (use `str.search(re)` for the index). Indices are code points, not UTF-16
//! units, like every other string index in the language. With `m`, `^`/`$` treat `\n`
//! and `\r` as line breaks but not U+2028/U+2029, and never match between `\r` and `\n`
//! (JS does both). With `i`, case folding is Unicode simple folding (as JS with `u`).
use crate::value::Value;
use std::rc::Rc;

#[derive(Debug)]
pub struct JsRegex {
    pub source: String,
    pub flags: String,
    pub global: bool,
    pub re: regex::Regex,
    /// Capture group names by index (index 0 is the whole match).
    pub names: Vec<Option<String>>,
    /// `lastIndex` (in characters): where a global regex's next `exec`/`test` starts.
    pub last_index: std::cell::Cell<usize>,
    /// The runtime froze it (it is kept): `lastIndex` cannot change.
    pub frozen: std::cell::Cell<bool>,
}

impl JsRegex {
    /// A new RegExp object with the same pattern (a literal makes one each time it is evaluated).
    pub fn fresh(&self) -> JsRegex {
        JsRegex {
            source: self.source.clone(),
            flags: self.flags.clone(),
            global: self.global,
            re: self.re.clone(),
            names: self.names.clone(),
            last_index: Default::default(),
            frozen: Default::default(),
        }
    }
}

const UNSUPPORTED: &str = "is not supported by the regex engine (Rust `regex` crate)";

pub fn compile(source: &str, flags: &str) -> Result<JsRegex, String> {
    let mut seen = String::new();
    for f in flags.chars() {
        if seen.contains(f) {
            return Err(format!("SyntaxError: invalid regular expression flags '{flags}'"));
        }
        match f {
            'g' | 'i' | 'm' | 's' | 'u' => seen.push(f),
            'y' | 'd' | 'v' => return Err(format!("SyntaxError: regex flag '{f}' {UNSUPPORTED}")),
            _ => return Err(format!("SyntaxError: invalid regular expression flags '{flags}'")),
        }
    }
    let body = translate(source, flags.contains('s'), flags.contains('u'))
        .map_err(|e| format!("SyntaxError: invalid regular expression /{source}/: {e}"))?;
    let mut pat = String::new();
    if flags.contains('i') {
        pat.push_str("(?i)");
    }
    if flags.contains('m') {
        // CRLF mode: `\r` is a line terminator for `^`/`$` too (JS also has U+2028/U+2029).
        pat.push_str("(?mR)");
    }
    pat.push_str(&body);
    let re = regex::Regex::new(&pat).map_err(|e| format!("SyntaxError: invalid regular expression /{source}/: {e}"))?;
    let names = re.capture_names().map(|n| n.map(str::to_owned)).collect();
    Ok(JsRegex {
        source: if source.is_empty() {
            "(?:)".into()
        } else {
            source.into()
        },
        flags: flags.into(),
        global: flags.contains('g'),
        re,
        names,
        last_index: Default::default(),
        frozen: Default::default(),
    })
}

const DIGIT: &str = "0-9";
const WORD: &str = "0-9A-Za-z_";
const SPACE: &str =
    "\\t\\n\\x0B\\f\\r \\x{A0}\\x{1680}\\x{2000}-\\x{200A}\\x{2028}\\x{2029}\\x{202F}\\x{205F}\\x{3000}\\x{FEFF}";

fn hex(chars: &[char], i: usize, n: usize) -> Option<u32> {
    if i + n > chars.len() {
        return None;
    }
    let s: String = chars[i..i + n].iter().collect();
    if s.chars().all(|c| c.is_ascii_hexdigit()) {
        u32::from_str_radix(&s, 16).ok()
    } else {
        None
    }
}

fn lit(c: char, out: &mut String) {
    out.push_str(&format!("\\x{{{:X}}}", c as u32));
}

/// Translate a JS pattern body to `regex` crate syntax.
fn translate(src: &str, dot_all: bool, unicode: bool) -> Result<String, String> {
    let cs: Vec<char> = src.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    let mut in_class = false;
    let mut class_start = false;
    while i < cs.len() {
        let c = cs[i];
        let at_class_start = std::mem::take(&mut class_start);
        if c == '\\' {
            let e = *cs.get(i + 1).ok_or("\\ at end of pattern")?;
            i += 2;
            match e {
                'd' | 'w' | 's' => {
                    let set = match e {
                        'd' => DIGIT,
                        'w' => WORD,
                        _ => SPACE,
                    };
                    out.push('[');
                    out.push_str(set);
                    out.push(']');
                }
                'D' | 'W' | 'S' => {
                    let set = match e {
                        'D' => DIGIT,
                        'W' => WORD,
                        _ => SPACE,
                    };
                    out.push_str("[^");
                    out.push_str(set);
                    out.push(']');
                }
                'b' if in_class => out.push_str("\\x08"),
                'b' => out.push_str("(?-u:\\b)"),
                'B' => out.push_str("(?-u:\\B)"),
                'n' | 'r' | 't' | 'f' | 'v' => {
                    out.push('\\');
                    out.push(e);
                }
                '0' if !cs.get(i).is_some_and(|c| c.is_ascii_digit()) => out.push_str("\\x00"),
                '1'..='9' if !in_class => return Err(format!("backreference \\{e} {UNSUPPORTED}")),
                'k' if cs.get(i) == Some(&'<') => return Err(format!("named backreference \\k<..> {UNSUPPORTED}")),
                'c' if cs.get(i).is_some_and(|c| c.is_ascii_alphabetic()) => {
                    lit(char::from_u32(cs[i] as u32 % 32).unwrap(), &mut out);
                    i += 1;
                }
                'x' if hex(&cs, i, 2).is_some() => {
                    lit(char::from_u32(hex(&cs, i, 2).unwrap()).unwrap(), &mut out);
                    i += 2;
                }
                'u' if unicode && cs.get(i) == Some(&'{') => {
                    let end = cs[i..].iter().position(|&c| c == '}').ok_or("bad \\u{..} escape")? + i;
                    let s: String = cs[i + 1..end].iter().collect();
                    let cp = u32::from_str_radix(&s, 16)
                        .ok()
                        .and_then(char::from_u32)
                        .ok_or("bad \\u{..} escape")?;
                    lit(cp, &mut out);
                    i = end + 1;
                }
                'u' if hex(&cs, i, 4).is_some() => {
                    let hi = hex(&cs, i, 4).unwrap();
                    i += 4;
                    let cp =
                        if (0xD800..0xDC00).contains(&hi) && cs.get(i) == Some(&'\\') && cs.get(i + 1) == Some(&'u') {
                            match hex(&cs, i + 2, 4) {
                                Some(lo) if (0xDC00..0xE000).contains(&lo) => {
                                    i += 6;
                                    0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00)
                                }
                                _ => hi,
                            }
                        } else {
                            hi
                        };
                    lit(
                        char::from_u32(cp).ok_or("lone surrogate escapes cannot match UTF-8 text")?,
                        &mut out,
                    );
                }
                'p' | 'P' if unicode => {
                    out.push('\\');
                    out.push(e);
                }
                // Identity escapes (`\/`, `\<`, `\a`, ...) are the character itself.
                e => out.push_str(&regex::escape(e.encode_utf8(&mut [0; 4]))),
            }
            continue;
        }
        i += 1;
        if in_class {
            match c {
                ']' if at_class_start => {
                    // JS `[]` matches nothing; `[^]` matches anything.
                    let negated = out.ends_with("[^");
                    out.truncate(out.len() - if negated { 2 } else { 1 });
                    out.push_str(if negated { "(?s:.)" } else { "[^\\x{0}-\\x{10FFFF}]" });
                    in_class = false;
                }
                ']' => {
                    out.push(']');
                    in_class = false;
                }
                '[' | '&' | '~' => {
                    out.push('\\');
                    out.push(c);
                }
                '-' if cs.get(i) == Some(&'-') => out.push_str("\\-"),
                c => out.push(c),
            }
            continue;
        }
        match c {
            '[' => {
                in_class = true;
                out.push('[');
                if cs.get(i) == Some(&'^') {
                    out.push('^');
                    i += 1;
                }
                class_start = true;
            }
            '.' if dot_all => out.push_str("(?s:.)"),
            '.' => out.push_str("[^\\n\\r\\x{2028}\\x{2029}]"),
            '(' if cs.get(i) == Some(&'?') => {
                let rest: String = cs[i..cs.len().min(i + 3)].iter().collect();
                if rest.starts_with("?=") || rest.starts_with("?!") {
                    return Err(format!("lookahead {UNSUPPORTED}"));
                }
                if rest.starts_with("?<=") || rest.starts_with("?<!") {
                    return Err(format!("lookbehind {UNSUPPORTED}"));
                }
                if rest.starts_with("?<") {
                    out.push_str("(?P<");
                    i += 2;
                } else {
                    out.push('(');
                }
            }
            '{' => {
                // A quantifier `{n}`, `{n,}`, `{n,m}`; otherwise a literal brace, as in JS.
                let end = cs[i..].iter().position(|&c| c == '}').map(|p| p + i);
                let quant = end.is_some_and(|e| {
                    let body: String = cs[i..e].iter().collect();
                    let mut parts = body.splitn(2, ',');
                    let a = parts.next().unwrap_or("");
                    let b = parts.next();
                    !a.is_empty()
                        && a.chars().all(|c| c.is_ascii_digit())
                        && b.is_none_or(|b| b.chars().all(|c| c.is_ascii_digit()))
                });
                if quant {
                    let e = end.unwrap();
                    out.extend(&cs[i - 1..=e]);
                    i = e + 1;
                } else {
                    out.push_str("\\{");
                }
            }
            '}' => out.push_str("\\}"),
            c => out.push(c),
        }
    }
    if in_class {
        return Err("missing /".into());
    }
    Ok(out)
}

/// Char (code point) index of a byte offset.
fn char_index(s: &str, byte: usize) -> i64 {
    s[..byte].chars().count() as i64
}

/// The next byte offset after `at` (one code point on), as JS AdvanceStringIndex.
fn advance(s: &str, at: usize) -> usize {
    at + s[at..].chars().next().map_or(1, char::len_utf8)
}

/// One match as a JS match array: [whole, group1, ...]; unmatched groups are undefined.
fn match_array(c: &regex::Captures) -> Value {
    Value::list(
        c.iter()
            .map(|m| m.map_or(Value::Undef, |m| Value::str(m.as_str())))
            .collect(),
    )
}

/// All matches in JS global order (lastIndex advanced one code point past empty matches).
pub fn all_captures<'h>(re: &JsRegex, s: &'h str) -> impl Iterator<Item = regex::Captures<'h>> {
    let mut at = 0;
    std::iter::from_fn(move || {
        if at > s.len() {
            return None;
        }
        let c = re.re.captures_at(s, at)?;
        let m = c.get(0).unwrap();
        at = if m.end() == m.start() {
            advance(s, m.end())
        } else {
            m.end()
        };
        Some(c)
    })
}

/// The matches as an array, under the bulk array limit.
fn match_list(re: &JsRegex, s: &str, item: impl Fn(&regex::Captures) -> Value) -> Result<Vec<Value>, String> {
    let mut out = vec![];
    for c in all_captures(re, s) {
        out.push(item(&c));
        crate::jsvalue::bulk_array(out.len())?;
    }
    Ok(out)
}

/// `re.exec(s)`: a global regex starts at its `lastIndex` and moves it past the match (back to 0
/// when nothing matches).
pub fn exec(re: &JsRegex, s: &str) -> Result<Value, String> {
    if !re.global {
        return Ok(re.re.captures(s).map_or(Value::None, |c| match_array(&c)));
    }
    if re.frozen.get() {
        return Err("TypeError: Cannot assign to read only property 'lastIndex' of object (this RegExp is kept by the runtime, frozen)".into());
    }
    let start = re.last_index.get();
    let chars = s.chars().count();
    if start > chars {
        re.last_index.set(0);
        return Ok(Value::None);
    }
    let at = s.char_indices().nth(start).map_or(s.len(), |(i, _)| i);
    match re.re.captures_at(s, at) {
        Some(c) => {
            let end = c.get(0).unwrap().end();
            re.last_index.set(char_index(s, end) as usize);
            Ok(match_array(&c))
        }
        None => {
            re.last_index.set(0);
            Ok(Value::None)
        }
    }
}

pub fn test(re: &JsRegex, s: &str) -> Result<bool, String> {
    if !re.global {
        return Ok(re.re.is_match(s));
    }
    Ok(!matches!(exec(re, s)?, Value::None))
}

/// `str.match(re)`
pub fn str_match(re: &JsRegex, s: &str) -> Result<Value, String> {
    if !re.global {
        return Ok(re.re.captures(s).map_or(Value::None, |c| match_array(&c)));
    }
    // A global match starts from 0 and leaves `lastIndex` 0, as JavaScript's does.
    if !re.frozen.get() {
        re.last_index.set(0)
    }
    let all = match_list(re, s, |c| Value::str(c.get(0).unwrap().as_str()))?;
    Ok(if all.is_empty() { Value::None } else { Value::list(all) })
}

/// `str.matchAll(re)` (as an array of match arrays).
pub fn match_all(re: &JsRegex, s: &str) -> Result<Value, String> {
    if !re.global {
        return Err("TypeError: String.prototype.matchAll called with a non-global RegExp argument".into());
    }
    Ok(Value::list(match_list(re, s, match_array)?))
}

/// `str.search(re)`
pub fn search(re: &JsRegex, s: &str) -> Value {
    Value::Int(re.re.find(s).map_or(-1, |m| char_index(s, m.start())))
}

/// The arguments a JS replacer function receives: match, groups..., offset, string[, groups].
/// `offset`: the match's position in characters; `subject`: the string as one shared value. The
/// caller keeps both across matches, so a replace over many matches does not copy and rescan the
/// whole string for each.
pub fn replacer_args(re: &JsRegex, c: &regex::Captures, offset: i64, subject: &Value) -> Vec<Value> {
    let mut args: Vec<Value> = c
        .iter()
        .map(|m| m.map_or(Value::Undef, |m| Value::str(m.as_str())))
        .collect();
    args.push(Value::Int(offset));
    args.push(subject.clone());
    if re.names.iter().any(Option::is_some) {
        let mut g = crate::value::Map::default();
        for (i, n) in re.names.iter().enumerate() {
            if let Some(n) = n {
                g.insert(
                    Rc::from(n.as_str()),
                    c.get(i).map_or(Value::Undef, |m| Value::str(m.as_str())),
                );
            }
        }
        args.push(Value::map(g));
    }
    args
}

/// JS GetSubstitution: `$$`, `$&`, `` $` ``, `$'`, `$n`, `$nn`, `$<name>`.
pub fn expand(re: &JsRegex, c: &regex::Captures, s: &str, tpl: &str, out: &mut String) {
    let m = c.get(0).unwrap();
    let n = c.len() - 1;
    let t: Vec<char> = tpl.chars().collect();
    let mut i = 0;
    let group = |k: usize, out: &mut String| {
        if let Some(g) = c.get(k) {
            out.push_str(g.as_str());
        }
    };
    while i < t.len() {
        if t[i] != '$' || i + 1 == t.len() {
            out.push(t[i]);
            i += 1;
            continue;
        }
        let d = |k: usize| t.get(k).and_then(|c| c.to_digit(10)).map(|v| v as usize);
        match t[i + 1] {
            '$' => {
                out.push('$');
                i += 2;
            }
            '&' => {
                out.push_str(m.as_str());
                i += 2;
            }
            '`' => {
                out.push_str(&s[..m.start()]);
                i += 2;
            }
            '\'' => {
                out.push_str(&s[m.end()..]);
                i += 2;
            }
            '<' if re.names.iter().any(Option::is_some) => match t[i + 2..].iter().position(|&c| c == '>') {
                Some(p) => {
                    let name: String = t[i + 2..i + 2 + p].iter().collect();
                    if let Some(k) = re.names.iter().position(|x| x.as_deref() == Some(name.as_str())) {
                        group(k, out);
                    }
                    i += 3 + p;
                }
                None => {
                    out.push('$');
                    i += 1;
                }
            },
            _ => match (d(i + 1), d(i + 2)) {
                (Some(a), Some(b)) if (1..=n).contains(&(a * 10 + b)) => {
                    group(a * 10 + b, out);
                    i += 3;
                }
                (Some(a), _) if (1..=n).contains(&a) => {
                    group(a, out);
                    i += 2;
                }
                _ => {
                    out.push('$');
                    i += 1;
                }
            },
        }
    }
}

/// JS `split` with a regex separator (captures are spliced into the result).
pub fn split(re: &JsRegex, s: &str, limit: Option<usize>) -> Result<Value, String> {
    let lim = limit.unwrap_or(u32::MAX as usize);
    let mut out: Vec<Value> = vec![];
    if lim == 0 {
        return Ok(Value::list(out));
    }
    if s.is_empty() {
        return Ok(Value::list(if re.re.is_match(s) {
            vec![]
        } else {
            vec![Value::str(s)]
        }));
    }
    let (mut p, mut q) = (0usize, 0usize);
    while q < s.len() {
        // The first position >= q where a match starts, with that match (as the spec's per-q sticky test).
        let Some(c) = re.re.captures_at(s, q) else { break };
        let m = c.get(0).unwrap();
        if m.start() >= s.len() {
            break;
        }
        if m.end() == p || (m.end() == m.start() && m.start() == p) {
            q = advance(s, m.start().max(q));
            continue;
        }
        out.push(Value::str(&s[p..m.start()]));
        if out.len() == lim {
            return Ok(Value::list(out));
        }
        for g in c.iter().skip(1) {
            out.push(g.map_or(Value::Undef, |g| Value::str(g.as_str())));
            if out.len() == lim {
                return Ok(Value::list(out));
            }
        }
        crate::jsvalue::bulk_array(out.len())?;
        p = m.end();
        q = if m.end() == m.start() { advance(s, p) } else { p };
    }
    out.push(Value::str(&s[p..]));
    Ok(Value::list(out))
}

impl crate::interp::Interp<'_> {
    pub(crate) fn regex_method(&mut self, re: &Rc<JsRegex>, name: &str, a: &[Value]) -> Result<Value, String> {
        let arg = |i: usize| {
            a.get(i)
                .map_or_else(|| "undefined".to_string(), crate::jsvalue::to_string)
        };
        Ok(match name {
            "test" => Value::Bool(test(re, &arg(0))?),
            "exec" => exec(re, &arg(0))?,
            "toString" => Value::string(format!("/{}/{}", re.source, re.flags)),
            _ => return Err(format!("'RegExp' object has no method '{name}'")),
        })
    }

    /// `match`, `matchAll`, `search`, `replace`, `replaceAll` and `split` with a regex argument.
    pub(crate) fn str_regex_method(&mut self, s: &str, name: &str, a: &[Value]) -> Result<Value, String> {
        let re = match &a[0] {
            Value::Regex(r) => r.clone(),
            // JS: a non-regex argument to match/search/matchAll is compiled as a pattern.
            other => Rc::new(compile(
                &crate::jsvalue::to_string(other),
                if name == "matchAll" { "g" } else { "" },
            )?),
        };
        Ok(match name {
            "match" => str_match(&re, s)?,
            "matchAll" => match_all(&re, s)?,
            "search" => search(&re, s),
            "split" => split(
                &re,
                s,
                // The limit as JavaScript reads it (ToUint32): `2.0` and `2.7` are 2.
                match a.get(1) {
                    Some(Value::Int(n)) => Some((*n).clamp(0, u32::MAX as i64) as usize),
                    Some(v @ Value::Float(_)) => Some(crate::jsvalue::to_uint32(v) as usize),
                    _ => None,
                },
            )?,
            _ => {
                if name == "replaceAll" && !re.global {
                    return Err("TypeError: replaceAll must be called with a global RegExp".into());
                }
                let with = a.get(1).cloned().unwrap_or(Value::Undef);
                // Matches stream: a global replace makes no list of them.
                let caps: Box<dyn Iterator<Item = regex::Captures>> = if re.global {
                    Box::new(all_captures(&re, s))
                } else {
                    Box::new(re.re.captures(s).into_iter())
                };
                let mut out = crate::jsvalue::StrBuf::default();
                let mut last = 0;
                // For a replacer function: the subject once, and the offset counted on from the
                // previous match.
                let subject = Value::str(s);
                let (mut counted_to, mut chars) = (0usize, 0i64);
                for c in caps {
                    let c = &c;
                    let m = c.get(0).unwrap();
                    chars += s[counted_to..m.start()].chars().count() as i64;
                    counted_to = m.start();
                    out.push_str(&s[last..m.start()]);
                    match &with {
                        // Any callable is a replacer (a bound function, a method, `String`), as
                        // with a string pattern.
                        replacer if replacer.is_callable() => {
                            let r = self.call_value(&with, replacer_args(&re, c, chars, &subject))?;
                            out.append(&crate::jsvalue::to_string(&r))?;
                        }
                        other => {
                            expand(&re, c, s, &crate::jsvalue::to_string(other), &mut out);
                            out.check()?;
                        }
                    }
                    last = m.end();
                }
                out.append(&s[last..])?;
                Value::string(out.into_string())
            }
        })
    }
}
