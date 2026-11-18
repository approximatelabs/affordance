//! JavaScript's built-in methods and functions on the view language's values (strings, arrays,
//! numbers, `JSON`, `Math`, `parseInt`/`parseFloat`, `Object.*`), with JavaScript's results.
//! Supported operations follow `spec/LANGUAGE.md`; strings index by Unicode code points.
use crate::interp::{Interp, RResult};
use crate::jsvalue as js;
use crate::value::{Map, Text, Value};
use std::rc::Rc;

fn err<T>(m: impl Into<String>) -> RResult<T> {
    Err(m.into())
}

/// ECMAScript ToIntegerOrInfinity.
pub fn to_integer(v: &Value) -> f64 {
    if let Value::Int(i) = v {
        return *i as f64;
    }
    let f = js::to_number(v);
    if f.is_nan() { 0.0 } else { f.trunc() + 0.0 }
}

/// A relative position (`slice`, `splice`, `fill`, ...): negative counts from the end; clamped
/// to `0..=len`; `default` when absent or undefined.
pub fn rel_index(v: Option<&Value>, len: usize, default: usize) -> usize {
    match v {
        None | Some(Value::Undef) => default,
        Some(v) => {
            let n = to_integer(v);
            let len = len as f64;
            (if n < 0.0 { (len + n).max(0.0) } else { n.min(len) }) as usize
        }
    }
}

/// A position clamped to `0..=len` (`substring`, `startsWith`, ...).
fn clamp_index(v: Option<&Value>, len: usize, default: usize) -> usize {
    match v {
        None | Some(Value::Undef) => default,
        Some(v) => to_integer(v).clamp(0.0, len as f64) as usize,
    }
}

pub fn callable(v: &Value) -> bool {
    v.is_callable()
}

fn not_callable(v: &Value) -> String {
    format!(
        "TypeError: {} is not a function",
        match v {
            Value::Str(s) => format!("\"{}\"", s.chars().take(40).collect::<String>()),
            Value::List(_) | Value::Map(_) => js::type_of(v).to_string(),
            v => js::to_string(v),
        }
    )
}

/// `$$ $& $\` $'` in a replacement string (a string pattern has no groups: `$1` stays as is).
fn substitute(with: &str, matched: &str, before: &str, after: &str) -> String {
    let mut out = String::with_capacity(with.len());
    let mut chars = with.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '$' {
            out.push(c);
            continue;
        }
        match chars.peek() {
            Some('$') => {
                chars.next();
                out.push('$');
            }
            Some('&') => {
                chars.next();
                out.push_str(matched);
            }
            Some('`') => {
                chars.next();
                out.push_str(before);
            }
            Some('\'') => {
                chars.next();
                out.push_str(after);
            }
            _ => out.push('$'),
        }
    }
    out
}

/// ICU root-collation order for ASCII (`localeCompare`): punctuation and symbols, then digits,
/// then letters, comparing case-insensitively first and lower case before upper case on a tie.
fn collation_key(c: char) -> Option<(u32, u32)> {
    // The ASCII characters in CLDR root order (variable characters are non-ignorable in ICU's
    // default); the other control characters are ignorable.
    const ORDER: &str = "\t\n\u{b}\u{c}\r _-,;:!?.'\"()[]{}@*/\\&#%`^+<=>|~$0123456789aAbBcCdDeEfFgGhHiIjJkKlLmMnNoOpPqQrRsStTuUvVwWxXyYzZ";
    if c.is_ascii_alphabetic() {
        let base = ORDER.find(c.to_ascii_lowercase()).unwrap();
        return Some((base as u32, if c.is_ascii_uppercase() { 1 } else { 0 }));
    }
    ORDER.find(c).map(|i| (i as u32, 0))
}

/// `a.localeCompare(b)` for ASCII text (ICU's root collation); other text compares by code
/// points (a known difference: full Unicode collation is not built in).
pub fn locale_compare(a: &str, b: &str) -> i64 {
    use std::cmp::Ordering::*;
    let ord = if a.is_ascii() && b.is_ascii() {
        let pa: Vec<(u32, u32)> = a.chars().filter_map(collation_key).collect();
        let pb: Vec<(u32, u32)> = b.chars().filter_map(collation_key).collect();
        let primary = pa.iter().map(|k| k.0).cmp(pb.iter().map(|k| k.0));
        if primary != Equal {
            primary
        } else {
            pa.iter().map(|k| k.1).cmp(pb.iter().map(|k| k.1))
        }
    } else {
        a.cmp(b)
    };
    match ord {
        Less => -1,
        Equal => 0,
        Greater => 1,
    }
}

/// `Number.prototype.toFixed`: the exact decimal value rounded half up (JavaScript picks the
/// larger of two equally near results), not Rust's half-to-even.
pub fn to_fixed(x: f64, digits: f64) -> RResult<String> {
    if !(0.0..=100.0).contains(&digits) {
        return err("RangeError: toFixed() digits argument must be between 0 and 100");
    }
    let f = digits as usize;
    if x.is_nan() {
        return Ok("NaN".into());
    }
    if x.abs() >= 1e21 || x.is_infinite() {
        return Ok(js::number_string(x));
    }
    // Every double is a finite binary fraction: 1100 decimal places are its exact value.
    let exact = format!("{:.1100}", x.abs());
    let (int, frac) = exact.split_once('.').unwrap();
    let mut digits: Vec<u8> = int.bytes().chain(frac.bytes().take(f)).collect();
    if frac.as_bytes()[f] >= b'5' {
        let mut i = digits.len();
        loop {
            if i == 0 {
                digits.insert(0, b'1');
                break;
            }
            i -= 1;
            if digits[i] == b'9' {
                digits[i] = b'0';
            } else {
                digits[i] += 1;
                break;
            }
        }
    }
    let n_int = digits.len() - f;
    let mut s = String::from(if x < 0.0 { "-" } else { "" });
    s.push_str(std::str::from_utf8(&digits[..n_int]).unwrap());
    if f > 0 {
        s.push('.');
        s.push_str(std::str::from_utf8(&digits[n_int..]).unwrap());
    }
    Ok(s)
}

/// `Number.prototype.toString(radix)`: V8's `DoubleToRadixCString`.
pub fn radix_string(value: f64, radix: u32) -> String {
    if value.is_nan() {
        return "NaN".into();
    }
    if value.is_infinite() {
        return if value > 0.0 {
            "Infinity".into()
        } else {
            "-Infinity".into()
        };
    }
    if value == 0.0 {
        return "0".into();
    }
    const CHARS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let next_double = |d: f64| f64::from_bits(d.to_bits() + 1);
    let exponent = |d: f64| {
        let biased = ((d.to_bits() >> 52) & 0x7ff) as i32;
        if biased == 0 { 1 - 1075 } else { biased - 1075 }
    };
    let negative = value < 0.0;
    let value = value.abs();
    let mut integer = value.floor();
    let mut fraction = value - integer;
    let mut delta = 0.5 * (next_double(value) - value);
    delta = delta.max(next_double(0.0));
    let mut frac_digits: Vec<u8> = vec![];
    if fraction >= delta {
        loop {
            fraction *= radix as f64;
            delta *= radix as f64;
            let digit = fraction as u32;
            frac_digits.push(CHARS[digit as usize]);
            fraction -= digit as f64;
            if (fraction > 0.5 || (fraction == 0.5 && (digit & 1) == 1)) && fraction + delta > 1.0 {
                // Round up, carrying through the digits already written.
                loop {
                    match frac_digits.pop() {
                        None => {
                            integer += 1.0;
                            break;
                        }
                        Some(c) => {
                            let d = if c > b'9' {
                                (c - b'a' + 10) as u32
                            } else {
                                (c - b'0') as u32
                            };
                            if d + 1 < radix {
                                frac_digits.push(CHARS[(d + 1) as usize]);
                                break;
                            }
                        }
                    }
                }
                break;
            }
            if fraction < delta {
                break;
            }
        }
    }
    let mut int_digits: Vec<u8> = vec![];
    while exponent(integer / radix as f64) > 0 {
        integer /= radix as f64;
        int_digits.push(b'0');
    }
    loop {
        let remainder = integer % radix as f64;
        int_digits.push(CHARS[remainder as usize]);
        integer = (integer - remainder) / radix as f64;
        if integer <= 0.0 {
            break;
        }
    }
    let mut s = String::new();
    if negative {
        s.push('-');
    }
    s.extend(int_digits.iter().rev().map(|&c| c as char));
    if !frac_digits.is_empty() {
        s.push('.');
        s.extend(frac_digits.iter().map(|&c| c as char));
    }
    s
}

/// The global `parseInt(string, radix)`.
pub fn parse_int(s: &Value, radix: Option<&Value>) -> Value {
    let text = js::to_string(s);
    let mut t = text.trim_start_matches(js::is_js_space);
    let sign = if let Some(r) = t.strip_prefix('-') {
        t = r;
        -1.0
    } else {
        if let Some(r) = t.strip_prefix('+') {
            t = r;
        }
        1.0
    };
    let mut r = radix.map(js::to_int32).unwrap_or(0);
    let mut strip_prefix = true;
    if r != 0 {
        if !(2..=36).contains(&r) {
            return Value::Float(f64::NAN);
        }
        if r != 16 {
            strip_prefix = false;
        }
    } else {
        r = 10;
    }
    if strip_prefix && (t.starts_with("0x") || t.starts_with("0X")) {
        t = &t[2..];
        r = 16;
    }
    let digits: &str = &t[..t.find(|c: char| !c.is_digit(r as u32)).unwrap_or(t.len())];
    if digits.is_empty() {
        return Value::Float(f64::NAN);
    }
    let value = if r == 10 {
        // V8 reads decimal digits as a decimal literal (correctly rounded).
        digits.parse::<f64>().unwrap_or(f64::NAN)
    } else if (r as u32).is_power_of_two() {
        // Exact, rounded once (V8 rounds the dropped bits to even).
        match u128::from_str_radix(digits, r as u32) {
            Ok(n) => n as f64,
            Err(_) => digits
                .chars()
                .fold(0f64, |acc, d| acc * r as f64 + d.to_digit(r as u32).unwrap() as f64),
        }
    } else {
        // V8's `InternalStringToInt`: parts that keep the multiplier within 32 bits, each
        // folded in as `result * multiplier + part` (an approximation the spec allows).
        const MAX_MULTIPLIER: u32 = 0xFFFF_FFFF / 36;
        let ds: Vec<u32> = digits.chars().map(|c| c.to_digit(r as u32).unwrap()).collect();
        let mut result = 0f64;
        let mut i = 0;
        while i < ds.len() {
            let (mut part, mut multiplier) = (0u32, 1u32);
            while i < ds.len() {
                let m = multiplier * r as u32;
                if m > MAX_MULTIPLIER {
                    break;
                }
                part = part * r as u32 + ds[i];
                multiplier = m;
                i += 1;
            }
            result = result * multiplier as f64 + part as f64;
        }
        result
    };
    js::number(sign * value)
}

/// The global `parseFloat(string)`.
pub fn parse_float(s: &Value) -> Value {
    let text = js::to_string(s);
    let t = text.trim_start_matches(js::is_js_space);
    let (sign, body) = match t.as_bytes().first() {
        Some(b'-') => (-1.0, &t[1..]),
        Some(b'+') => (1.0, &t[1..]),
        _ => (1.0, t),
    };
    if body.starts_with("Infinity") {
        return Value::Float(sign * f64::INFINITY);
    }
    let n = js::decimal_prefix(body);
    if n == 0 {
        return Value::Float(f64::NAN);
    }
    let mut lit = body[..n].to_string();
    if lit.starts_with('.') {
        lit.insert(0, '0');
    }
    js::number(sign * lit.parse::<f64>().unwrap_or(f64::NAN))
}

/// `Math.round`: halves round up (toward +Infinity); `-0.4` rounds to `-0`.
pub fn math_round(x: f64) -> f64 {
    if !x.is_finite() || x == 0.0 {
        return x;
    }
    let f = x.floor();
    let r = if x - f >= 0.5 { f + 1.0 } else { f };
    if r == 0.0 && x < 0.0 { -0.0 } else { r }
}

/// `Math.hypot` as V8 computes it (scaled, with Kahan summation).
pub fn math_hypot(xs: &[f64]) -> f64 {
    let mut max = 0f64;
    let mut nan = false;
    for &x in xs {
        let a = x.abs();
        if a.is_infinite() {
            return f64::INFINITY;
        }
        if a.is_nan() {
            nan = true;
        } else if a > max {
            max = a;
        }
    }
    if nan {
        return f64::NAN;
    }
    if max == 0.0 {
        return 0.0;
    }
    let (mut sum, mut compensation) = (0f64, 0f64);
    for &x in xs {
        let n = x.abs() / max;
        let summand = n * n - compensation;
        let preliminary = sum + summand;
        compensation = (preliminary - sum) - summand;
        sum = preliminary;
    }
    sum.sqrt() * max
}

/// `Math.*` functions of numbers.
pub fn math(name: &str, a: &[Value]) -> Option<Value> {
    let n = |i: usize| a.get(i).map(js::to_number).unwrap_or(f64::NAN);
    let x = n(0);
    let f = match name {
        "Math.max" | "Math.min" => {
            let max = name == "Math.max";
            let mut m = if max { f64::NEG_INFINITY } else { f64::INFINITY };
            for v in a {
                let x = js::to_number(v);
                if x.is_nan() || m.is_nan() {
                    m = f64::NAN;
                } else if (max && (x > m || (x == 0.0 && m == 0.0 && m.is_sign_negative())))
                    || (!max && (x < m || (x == 0.0 && m == 0.0 && x.is_sign_negative())))
                {
                    m = x;
                }
            }
            m
        }
        "Math.floor" => x.floor(),
        "Math.ceil" => x.ceil(),
        "Math.round" => math_round(x),
        "Math.trunc" => x.trunc(),
        "Math.abs" => x.abs(),
        "Math.sign" => {
            if x > 0.0 {
                1.0
            } else if x < 0.0 {
                -1.0
            } else {
                x
            }
        }
        "Math.sqrt" => x.sqrt(),
        "Math.cbrt" => crate::jsmath::cbrt(x),
        "Math.pow" => js::pow(x, n(1)),
        "Math.exp" => crate::jsmath::exp(x),
        "Math.expm1" => crate::jsmath::expm1(x),
        "Math.log" => crate::jsmath::log(x),
        "Math.log1p" => crate::jsmath::log1p(x),
        "Math.log2" => crate::jsmath::log2(x),
        "Math.log10" => crate::jsmath::log10(x),
        "Math.sin" => crate::jsmath::sin(x),
        "Math.cos" => crate::jsmath::cos(x),
        "Math.tan" => crate::jsmath::tan(x),
        "Math.asin" => crate::jsmath::asin(x),
        "Math.acos" => crate::jsmath::acos(x),
        "Math.atan" => crate::jsmath::atan(x),
        "Math.atan2" => crate::jsmath::atan2(x, n(1)),
        "Math.sinh" => crate::jsmath::sinh(x),
        "Math.cosh" => crate::jsmath::cosh(x),
        "Math.tanh" => crate::jsmath::tanh(x),
        "Math.asinh" => crate::jsmath::asinh(x),
        "Math.acosh" => crate::jsmath::acosh(x),
        "Math.atanh" => crate::jsmath::atanh(x),
        "Math.hypot" => math_hypot(&a.iter().map(js::to_number).collect::<Vec<_>>()),
        "Math.fround" => x as f32 as f64,
        "Math.clz32" => {
            return Some(Value::Int(
                a.first().map(js::to_uint32).unwrap_or(0).leading_zeros() as i64
            ));
        }
        "Math.imul" => {
            let (p, q) = (
                a.first().map(js::to_int32).unwrap_or(0),
                a.get(1).map(js::to_int32).unwrap_or(0),
            );
            return Some(Value::Int(p.wrapping_mul(q) as i64));
        }
        _ => return None,
    };
    Some(js::number(f))
}

/// JSON text for a string (`JSON.stringify`'s quoting).
pub fn json_quote(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// `JSON.parse`'s grammar (ECMA-404): numbers are doubles (`-0` too), object keys in
/// JavaScript's order (a repeated key keeps its first place and its last value).
struct JsonParser<'a> {
    s: &'a [u8],
    pos: usize,
}

impl JsonParser<'_> {
    fn fail<T>(&self) -> RResult<T> {
        match self.s.get(self.pos) {
            None => err("SyntaxError: Unexpected end of JSON input"),
            Some(_) => {
                let rest = String::from_utf8_lossy(&self.s[self.pos..]);
                let c = rest.chars().next().unwrap();
                err(format!(
                    "SyntaxError: Unexpected token '{c}', \"{}\" is not valid JSON",
                    String::from_utf8_lossy(&self.s[..self.s.len().min(30)])
                ))
            }
        }
    }

    fn ws(&mut self) {
        while matches!(self.s.get(self.pos), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    /// A nested value: the walk recurses once per level, so it grows the stack as the other
    /// native walks do (the 5000 levels need more than `eval`'s guaranteed headroom).
    fn nested(&mut self, depth: usize) -> RResult<Value> {
        stacker::maybe_grow(128 * 1024, 2 * 1024 * 1024, || self.value(depth))
    }

    fn value(&mut self, depth: usize) -> RResult<Value> {
        if depth > 5000 {
            return err("RangeError: Maximum call stack size exceeded (JSON nesting)");
        }
        self.ws();
        let Some(&c) = self.s.get(self.pos) else {
            return self.fail();
        };
        match c {
            b'{' => {
                self.pos += 1;
                let mut m = Map::default();
                self.ws();
                if self.s.get(self.pos) == Some(&b'}') {
                    self.pos += 1;
                    return Ok(Value::map(m));
                }
                loop {
                    self.ws();
                    if self.s.get(self.pos) != Some(&b'"') {
                        return self.fail();
                    }
                    let k = self.string()?;
                    self.ws();
                    if self.s.get(self.pos) != Some(&b':') {
                        return self.fail();
                    }
                    self.pos += 1;
                    let v = self.nested(depth + 1)?;
                    js::obj_insert(&mut m, Rc::from(k), v);
                    self.ws();
                    match self.s.get(self.pos) {
                        Some(b',') => self.pos += 1,
                        Some(b'}') => {
                            self.pos += 1;
                            return Ok(Value::map(m));
                        }
                        _ => return self.fail(),
                    }
                }
            }
            b'[' => {
                self.pos += 1;
                let mut v = vec![];
                self.ws();
                if self.s.get(self.pos) == Some(&b']') {
                    self.pos += 1;
                    return Ok(Value::list(v));
                }
                loop {
                    v.push(self.nested(depth + 1)?);
                    self.ws();
                    match self.s.get(self.pos) {
                        Some(b',') => self.pos += 1,
                        Some(b']') => {
                            self.pos += 1;
                            return Ok(Value::list(v));
                        }
                        _ => return self.fail(),
                    }
                }
            }
            b'"' => Ok(Value::string(self.string()?)),
            b't' if self.s[self.pos..].starts_with(b"true") => {
                self.pos += 4;
                Ok(Value::Bool(true))
            }
            b'f' if self.s[self.pos..].starts_with(b"false") => {
                self.pos += 5;
                Ok(Value::Bool(false))
            }
            b'n' if self.s[self.pos..].starts_with(b"null") => {
                self.pos += 4;
                Ok(Value::None)
            }
            b'-' | b'0'..=b'9' => {
                let start = self.pos;
                if c == b'-' {
                    self.pos += 1;
                }
                let digits = |p: &mut Self| {
                    let s = p.pos;
                    while p.s.get(p.pos).is_some_and(u8::is_ascii_digit) {
                        p.pos += 1;
                    }
                    p.pos - s
                };
                match self.s.get(self.pos) {
                    Some(b'0') => self.pos += 1,
                    Some(b'1'..=b'9') => {
                        digits(self);
                    }
                    _ => return self.fail(),
                }
                if self.s.get(self.pos) == Some(&b'.') {
                    self.pos += 1;
                    if digits(self) == 0 {
                        return self.fail();
                    }
                }
                if matches!(self.s.get(self.pos), Some(b'e' | b'E')) {
                    self.pos += 1;
                    if matches!(self.s.get(self.pos), Some(b'+' | b'-')) {
                        self.pos += 1;
                    }
                    if digits(self) == 0 {
                        return self.fail();
                    }
                }
                let text = std::str::from_utf8(&self.s[start..self.pos]).unwrap();
                // As the host's JSON reads (`Value::from_json`): `1.0` stays a float and a large
                // integer keeps its digits; both behave as the double JavaScript reads, and the
                // Python formatting helpers print them as Python would.
                if !text.contains(['.', 'e', 'E']) {
                    return Ok(match text.parse::<i64>() {
                        Ok(0) if text.starts_with('-') => Value::Float(-0.0),
                        Ok(i) => Value::Int(i),
                        Err(_) => Value::BigInt(Rc::from(text)),
                    });
                }
                Ok(Value::Float(text.parse::<f64>().unwrap()))
            }
            _ => self.fail(),
        }
    }

    fn string(&mut self) -> RResult<String> {
        self.pos += 1;
        let mut out: Vec<u8> = vec![];
        loop {
            let Some(&c) = self.s.get(self.pos) else {
                return err("SyntaxError: Unterminated string in JSON");
            };
            self.pos += 1;
            match c {
                b'"' => return Ok(String::from_utf8(out).unwrap()),
                b'\\' => {
                    let Some(&e) = self.s.get(self.pos) else {
                        return self.fail();
                    };
                    self.pos += 1;
                    match e {
                        b'"' => out.push(b'"'),
                        b'\\' => out.push(b'\\'),
                        b'/' => out.push(b'/'),
                        b'b' => out.push(8),
                        b'f' => out.push(12),
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'u' => {
                            let unit = |p: &mut Self| -> RResult<u32> {
                                let h =
                                    p.s.get(p.pos..p.pos + 4)
                                        .and_then(|h| std::str::from_utf8(h).ok())
                                        // Four hex digits: `from_str_radix` alone would take a sign.
                                        .filter(|h| h.bytes().all(|b| b.is_ascii_hexdigit()))
                                        .and_then(|h| u32::from_str_radix(h, 16).ok());
                                match h {
                                    Some(u) => {
                                        p.pos += 4;
                                        Ok(u)
                                    }
                                    None => err("SyntaxError: Bad Unicode escape in JSON"),
                                }
                            };
                            let mut cp = unit(self)?;
                            if (0xD800..0xDC00).contains(&cp) && self.s[self.pos..].starts_with(b"\\u") {
                                let save = self.pos;
                                self.pos += 2;
                                let lo = unit(self)?;
                                if (0xDC00..0xE000).contains(&lo) {
                                    cp = 0x10000 + ((cp - 0xD800) << 10) + (lo - 0xDC00);
                                } else {
                                    self.pos = save;
                                }
                            }
                            let Some(ch) = char::from_u32(cp) else {
                                return err(
                                    "a lone surrogate (\\uD800-\\uDFFF) in JSON is not supported: strings are Unicode text",
                                );
                            };
                            let mut b = [0u8; 4];
                            out.extend_from_slice(ch.encode_utf8(&mut b).as_bytes());
                        }
                        _ => {
                            self.pos -= 1;
                            return self.fail();
                        }
                    }
                }
                c if c < 0x20 => {
                    self.pos -= 1;
                    return err("SyntaxError: Bad control character in string literal in JSON");
                }
                c => out.push(c),
            }
        }
    }
}

/// `JSON.parse(text)` without a reviver.
pub fn json_parse_text(text: &str) -> RResult<Value> {
    let mut p = JsonParser {
        s: text.as_bytes(),
        pos: 0,
    };
    let v = p.nested(0)?;
    p.ws();
    if p.pos < p.s.len() {
        return p.fail();
    }
    Ok(v)
}

/// The comparator result as V8 reads it: ToNumber, NaN as 0.
type Cmp<'a, 'p> = &'a mut dyn FnMut(&mut Interp<'p>, &Value, &Value) -> RResult<f64>;

/// V8's TimSort (`third_party/v8/builtins/array-sort.tq`): the same comparator calls in the same
/// order, so even an inconsistent comparator sorts as in Node.
struct TimSort<'a, 'b, 'p> {
    it: &'b mut Interp<'p>,
    cmp: Cmp<'a, 'p>,
    work: Vec<Value>,
    runs: Vec<(usize, usize)>,
    min_gallop: usize,
}

const MIN_GALLOP: usize = 7;

// A line-by-line port of V8's merge loops: index copies are kept as the source writes them.
#[allow(clippy::manual_memcpy, clippy::needless_range_loop)]
impl TimSort<'_, '_, '_> {
    fn compare(&mut self, x: &Value, y: &Value) -> RResult<f64> {
        (self.cmp)(self.it, x, y)
    }

    /// `f` with the work array lent out (the gallops read it while comparing).
    fn with_work<R>(&mut self, f: impl FnOnce(&mut Self, &[Value]) -> RResult<R>) -> RResult<R> {
        let work = std::mem::take(&mut self.work);
        let r = f(self, &work);
        self.work = work;
        r
    }

    fn min_run_length(mut n: usize) -> usize {
        let mut r = 0;
        while n >= 64 {
            r |= n & 1;
            n >>= 1;
        }
        n + r
    }

    fn count_and_make_run(&mut self, low_arg: usize, high: usize) -> RResult<usize> {
        let low = low_arg + 1;
        if low == high {
            return Ok(1);
        }
        let mut run = 2;
        let (el, pre) = (self.work[low].clone(), self.work[low - 1].clone());
        let descending = self.compare(&el, &pre)? < 0.0;
        let mut previous = el;
        for idx in low + 1..high {
            let current = self.work[idx].clone();
            let order = self.compare(&current, &previous)?;
            if descending {
                if order >= 0.0 {
                    break;
                }
            } else if order < 0.0 {
                break;
            }
            previous = current;
            run += 1;
        }
        if descending {
            self.work[low_arg..low_arg + run].reverse();
        }
        Ok(run)
    }

    fn binary_insertion_sort(&mut self, low: usize, start_arg: usize, high: usize) -> RResult<()> {
        let mut start = if low == start_arg { start_arg + 1 } else { start_arg };
        while start < high {
            let (mut left, mut right) = (low, start);
            let pivot = self.work[start].clone();
            while left < right {
                let mid = left + ((right - left) >> 1);
                let m = self.work[mid].clone();
                if self.compare(&pivot, &m)? < 0.0 {
                    right = mid;
                } else {
                    left = mid + 1;
                }
            }
            let mut p = start;
            while p > left {
                self.work[p] = self.work[p - 1].clone();
                p -= 1;
            }
            self.work[left] = pivot;
            start += 1;
        }
        Ok(())
    }

    fn gallop_left(&mut self, array: &[Value], key: &Value, base: usize, length: usize, hint: usize) -> RResult<usize> {
        let (mut last_ofs, mut offset) = (0usize, 1usize);
        let order = self.compare(&array[base + hint], key)?;
        if order < 0.0 {
            let max_ofs = length - hint;
            while offset < max_ofs {
                if self.compare(&array[base + hint + offset], key)? >= 0.0 {
                    break;
                }
                last_ofs = offset;
                offset = (offset << 1) + 1;
            }
            offset = offset.min(max_ofs);
            last_ofs += hint;
            offset += hint;
        } else {
            let max_ofs = hint + 1;
            while offset < max_ofs {
                if self.compare(&array[base + hint - offset], key)? < 0.0 {
                    break;
                }
                last_ofs = offset;
                offset = (offset << 1) + 1;
            }
            offset = offset.min(max_ofs);
            let tmp = last_ofs;
            last_ofs = (hint + 1).wrapping_sub(offset);
            offset = hint - tmp;
            // (`last_ofs` is `hint - offset` plus the 1 added below; it may wrap to -1 + 1.)
            return self.gallop_finish(array, key, base, last_ofs, offset, true);
        }
        self.gallop_finish(array, key, base, last_ofs + 1, offset, true)
    }

    fn gallop_right(
        &mut self,
        array: &[Value],
        key: &Value,
        base: usize,
        length: usize,
        hint: usize,
    ) -> RResult<usize> {
        let (mut last_ofs, mut offset) = (0usize, 1usize);
        let order = self.compare(key, &array[base + hint])?;
        if order < 0.0 {
            let max_ofs = hint + 1;
            while offset < max_ofs {
                if self.compare(key, &array[base + hint - offset])? >= 0.0 {
                    break;
                }
                last_ofs = offset;
                offset = (offset << 1) + 1;
            }
            offset = offset.min(max_ofs);
            let tmp = last_ofs;
            last_ofs = (hint + 1).wrapping_sub(offset);
            offset = hint - tmp;
            return self.gallop_finish(array, key, base, last_ofs, offset, false);
        }
        let max_ofs = length - hint;
        while offset < max_ofs {
            if self.compare(key, &array[base + hint + offset])? < 0.0 {
                break;
            }
            last_ofs = offset;
            offset = (offset << 1) + 1;
        }
        offset = offset.min(max_ofs);
        last_ofs += hint;
        offset += hint;
        self.gallop_finish(array, key, base, last_ofs + 1, offset, false)
    }

    /// The binary search that ends a gallop: `a[lo - 1] < key <= a[hi]` (left) or
    /// `a[lo - 1] <= key < a[hi]` (right).
    fn gallop_finish(
        &mut self,
        array: &[Value],
        key: &Value,
        base: usize,
        mut lo: usize,
        mut hi: usize,
        left: bool,
    ) -> RResult<usize> {
        while lo < hi {
            let m = lo + ((hi - lo) >> 1);
            if left {
                if self.compare(&array[base + m], key)? < 0.0 {
                    lo = m + 1;
                } else {
                    hi = m;
                }
            } else if self.compare(key, &array[base + m])? < 0.0 {
                hi = m;
            } else {
                lo = m + 1;
            }
        }
        Ok(hi)
    }

    fn merge_at(&mut self, i: usize) -> RResult<()> {
        let (mut base_a, mut length_a) = self.runs[i];
        let (base_b, mut length_b) = self.runs[i + 1];
        self.runs[i].1 = length_a + length_b;
        self.runs.remove(i + 1);
        let k = self.with_work(|s, w| s.gallop_right(w, &w[base_b], base_a, length_a, 0))?;
        base_a += k;
        length_a -= k;
        if length_a == 0 {
            return Ok(());
        }
        length_b =
            self.with_work(|s, w| s.gallop_left(w, &w[base_a + length_a - 1], base_b, length_b, length_b - 1))?;
        if length_b == 0 {
            return Ok(());
        }
        if length_a <= length_b {
            self.merge_low(base_a, length_a, base_b, length_b)
        } else {
            self.merge_high(base_a, length_a, base_b, length_b)
        }
    }

    fn merge_low(&mut self, base_a: usize, mut length_a: usize, base_b: usize, mut length_b: usize) -> RResult<()> {
        let temp: Vec<Value> = self.work[base_a..base_a + length_a].to_vec();
        let (mut dest, mut cursor_temp, mut cursor_b) = (base_a, 0usize, base_b);
        self.work[dest] = self.work[cursor_b].clone();
        dest += 1;
        cursor_b += 1;
        enum End {
            Succeed,
            CopyB,
        }
        let end = 'merge: {
            length_b -= 1;
            if length_b == 0 {
                break 'merge End::Succeed;
            }
            if length_a == 1 {
                break 'merge End::CopyB;
            }
            let mut min_gallop = self.min_gallop;
            loop {
                let (mut wins_a, mut wins_b) = (0usize, 0usize);
                loop {
                    let (b, t) = (self.work[cursor_b].clone(), temp[cursor_temp].clone());
                    if self.compare(&b, &t)? < 0.0 {
                        self.work[dest] = b;
                        dest += 1;
                        cursor_b += 1;
                        wins_b += 1;
                        length_b -= 1;
                        wins_a = 0;
                        if length_b == 0 {
                            break 'merge End::Succeed;
                        }
                        if wins_b >= min_gallop {
                            break;
                        }
                    } else {
                        self.work[dest] = t;
                        dest += 1;
                        cursor_temp += 1;
                        wins_a += 1;
                        length_a -= 1;
                        wins_b = 0;
                        if length_a == 1 {
                            break 'merge End::CopyB;
                        }
                        if wins_a >= min_gallop {
                            break;
                        }
                    }
                }
                min_gallop += 1;
                let mut first = true;
                while wins_a >= MIN_GALLOP || wins_b >= MIN_GALLOP || first {
                    first = false;
                    min_gallop = min_gallop.saturating_sub(1).max(1);
                    self.min_gallop = min_gallop;
                    let key = self.work[cursor_b].clone();
                    wins_a = self.gallop_right(&temp, &key, cursor_temp, length_a, 0)?;
                    if wins_a > 0 {
                        for k in 0..wins_a {
                            self.work[dest + k] = temp[cursor_temp + k].clone();
                        }
                        dest += wins_a;
                        cursor_temp += wins_a;
                        length_a -= wins_a;
                        if length_a == 1 {
                            break 'merge End::CopyB;
                        }
                        if length_a == 0 {
                            break 'merge End::Succeed;
                        }
                    }
                    self.work[dest] = self.work[cursor_b].clone();
                    dest += 1;
                    cursor_b += 1;
                    length_b -= 1;
                    if length_b == 0 {
                        break 'merge End::Succeed;
                    }
                    let key = temp[cursor_temp].clone();
                    wins_b = self.with_work(|s, w| s.gallop_left(w, &key, cursor_b, length_b, 0))?;
                    if wins_b > 0 {
                        for k in 0..wins_b {
                            self.work[dest + k] = self.work[cursor_b + k].clone();
                        }
                        dest += wins_b;
                        cursor_b += wins_b;
                        length_b -= wins_b;
                        if length_b == 0 {
                            break 'merge End::Succeed;
                        }
                    }
                    self.work[dest] = temp[cursor_temp].clone();
                    dest += 1;
                    cursor_temp += 1;
                    length_a -= 1;
                    if length_a == 1 {
                        break 'merge End::CopyB;
                    }
                }
                min_gallop += 1;
                self.min_gallop = min_gallop;
            }
        };
        match end {
            End::Succeed => {
                for k in 0..length_a {
                    self.work[dest + k] = temp[cursor_temp + k].clone();
                }
            }
            End::CopyB => {
                for k in 0..length_b {
                    self.work[dest + k] = self.work[cursor_b + k].clone();
                }
                self.work[dest + length_b] = temp[cursor_temp].clone();
            }
        }
        Ok(())
    }

    fn merge_high(&mut self, base_a: usize, mut length_a: usize, base_b: usize, mut length_b: usize) -> RResult<()> {
        let temp: Vec<Value> = self.work[base_b..base_b + length_b].to_vec();
        // Cursors move down; they are kept one above their position (`dest` is `dest + 1`) so
        // they never go below zero.
        let mut dest = base_b + length_b; // one past
        let mut cursor_temp = length_b; // one past
        let mut cursor_a = base_a + length_a; // one past
        self.work[dest - 1] = self.work[cursor_a - 1].clone();
        dest -= 1;
        cursor_a -= 1;
        enum End {
            Succeed,
            CopyA,
        }
        let end = 'merge: {
            length_a -= 1;
            if length_a == 0 {
                break 'merge End::Succeed;
            }
            if length_b == 1 {
                break 'merge End::CopyA;
            }
            let mut min_gallop = self.min_gallop;
            loop {
                let (mut wins_a, mut wins_b) = (0usize, 0usize);
                loop {
                    let (t, a) = (temp[cursor_temp - 1].clone(), self.work[cursor_a - 1].clone());
                    if self.compare(&t, &a)? < 0.0 {
                        self.work[dest - 1] = a;
                        dest -= 1;
                        cursor_a -= 1;
                        wins_a += 1;
                        length_a -= 1;
                        wins_b = 0;
                        if length_a == 0 {
                            break 'merge End::Succeed;
                        }
                        if wins_a >= min_gallop {
                            break;
                        }
                    } else {
                        self.work[dest - 1] = t;
                        dest -= 1;
                        cursor_temp -= 1;
                        wins_b += 1;
                        length_b -= 1;
                        wins_a = 0;
                        if length_b == 1 {
                            break 'merge End::CopyA;
                        }
                        if wins_b >= min_gallop {
                            break;
                        }
                    }
                }
                min_gallop += 1;
                let mut first = true;
                while wins_a >= MIN_GALLOP || wins_b >= MIN_GALLOP || first {
                    first = false;
                    min_gallop = min_gallop.saturating_sub(1).max(1);
                    self.min_gallop = min_gallop;
                    let key = temp[cursor_temp - 1].clone();
                    let k = self.with_work(|s, w| s.gallop_right(w, &key, base_a, length_a, length_a - 1))?;
                    wins_a = length_a - k;
                    if wins_a > 0 {
                        dest -= wins_a;
                        cursor_a -= wins_a;
                        for k in (0..wins_a).rev() {
                            self.work[dest + k] = self.work[cursor_a + k].clone();
                        }
                        length_a -= wins_a;
                        if length_a == 0 {
                            break 'merge End::Succeed;
                        }
                    }
                    self.work[dest - 1] = temp[cursor_temp - 1].clone();
                    dest -= 1;
                    cursor_temp -= 1;
                    length_b -= 1;
                    if length_b == 1 {
                        break 'merge End::CopyA;
                    }
                    let key = self.work[cursor_a - 1].clone();
                    let k = self.gallop_left(&temp, &key, 0, length_b, length_b - 1)?;
                    wins_b = length_b - k;
                    if wins_b > 0 {
                        dest -= wins_b;
                        cursor_temp -= wins_b;
                        for k in 0..wins_b {
                            self.work[dest + k] = temp[cursor_temp + k].clone();
                        }
                        length_b -= wins_b;
                        if length_b == 1 {
                            break 'merge End::CopyA;
                        }
                        if length_b == 0 {
                            break 'merge End::Succeed;
                        }
                    }
                    self.work[dest - 1] = self.work[cursor_a - 1].clone();
                    dest -= 1;
                    cursor_a -= 1;
                    length_a -= 1;
                    if length_a == 0 {
                        break 'merge End::Succeed;
                    }
                }
                min_gallop += 1;
                self.min_gallop = min_gallop;
            }
        };
        match end {
            End::Succeed => {
                for k in 0..length_b {
                    self.work[dest - length_b + k] = temp[k].clone();
                }
            }
            End::CopyA => {
                dest -= length_a;
                cursor_a -= length_a;
                for k in (0..length_a).rev() {
                    self.work[dest + k] = self.work[cursor_a + k].clone();
                }
                self.work[dest - 1] = temp[cursor_temp - 1].clone();
            }
        }
        Ok(())
    }

    fn run_invariant(&self, n: usize) -> bool {
        if n < 2 {
            return true;
        }
        self.runs[n - 2].1 > self.runs[n - 1].1 + self.runs[n].1
    }

    fn merge_collapse(&mut self) -> RResult<()> {
        while self.runs.len() > 1 {
            let mut n = self.runs.len() - 2;
            if !self.run_invariant(n + 1) || !self.run_invariant(n) {
                if n > 0 && self.runs[n - 1].1 < self.runs[n + 1].1 {
                    n -= 1;
                }
                self.merge_at(n)?;
            } else if self.runs[n].1 <= self.runs[n + 1].1 {
                self.merge_at(n)?;
            } else {
                break;
            }
        }
        Ok(())
    }

    fn merge_force_collapse(&mut self) -> RResult<()> {
        while self.runs.len() > 1 {
            let mut n = self.runs.len() - 2;
            if n > 0 && self.runs[n - 1].1 < self.runs[n + 1].1 {
                n -= 1;
            }
            self.merge_at(n)?;
        }
        Ok(())
    }

    fn sort(mut self) -> RResult<Vec<Value>> {
        let length = self.work.len();
        if length < 2 {
            return Ok(self.work);
        }
        let (mut remaining, mut low) = (length, 0);
        let min_run = Self::min_run_length(remaining);
        while remaining != 0 {
            let mut run = self.count_and_make_run(low, low + remaining)?;
            if run < min_run {
                let forced = min_run.min(remaining);
                self.binary_insertion_sort(low, low + run, low + forced)?;
                run = forced;
            }
            self.runs.push((low, run));
            self.merge_collapse()?;
            low += run;
            remaining -= run;
        }
        self.merge_force_collapse()?;
        Ok(self.work)
    }
}

impl<'p> Interp<'p> {
    /// `Array.prototype.sort`: undefined last; without a comparator by string order (UTF-16 code
    /// units), stable; with one, V8's TimSort with the comparator.
    pub(crate) fn sort_values(&mut self, items: Vec<Value>, cmp: Option<&Value>) -> RResult<Vec<Value>> {
        let cmp = cmp.filter(|c| !matches!(c, Value::Undef));
        if let Some(c) = cmp
            && !callable(c)
        {
            return err("TypeError: The comparison function must be either a function or undefined");
        }
        let (mut defined, undefined): (Vec<Value>, Vec<Value>) =
            items.into_iter().partition(|v| !matches!(v, Value::Undef));
        match cmp {
            None => {
                let mut keyed: Vec<(String, Value)> = defined.into_iter().map(|v| (js::to_string(&v), v)).collect();
                keyed.sort_by(|x, y| js::cmp_strings(&x.0, &y.0));
                defined = keyed.into_iter().map(|(_, v)| v).collect();
            }
            Some(c) => {
                let c = c.clone();
                let mut call = |it: &mut Interp<'p>, x: &Value, y: &Value| -> RResult<f64> {
                    let r = it.call_value(&c, vec![x.clone(), y.clone()])?;
                    let n = js::to_number(&r);
                    Ok(if n.is_nan() { 0.0 } else { n })
                };
                defined = TimSort {
                    it: self,
                    cmp: &mut call,
                    work: defined,
                    runs: vec![],
                    min_gallop: MIN_GALLOP,
                }
                .sort()?;
            }
        }
        defined.extend(undefined);
        Ok(defined)
    }

    /// Shared indexed access for arrays and borrowed array methods. Reads stay live across
    /// callbacks, without holding a RefCell borrow while app code runs.
    pub(crate) fn array_like_len(&mut self, recv: &Value) -> RResult<usize> {
        if matches!(recv, Value::None | Value::Undef) {
            return err("TypeError: Cannot convert undefined or null to object");
        }
        if let Value::List(l) = recv {
            return Ok(l.read().len());
        }
        let n = match recv {
            Value::Str(s) => s.char_count() as f64,
            _ => to_integer(&self.get_prop(recv, "length")?).max(0.0),
        };
        if n > js::MAX_BULK_ARRAY as f64 {
            return err("RangeError: array-like length exceeds iteration limit");
        }
        Ok(n as usize)
    }
    fn array_like_at(&mut self, recv: &Value, i: usize) -> RResult<Option<Value>> {
        match recv {
            Value::List(l) => Ok(l.read().get(i).cloned()),
            Value::Str(s) => Ok(s.chars().nth(i).map(|c| Value::string(c.to_string()))),
            Value::Map(_) | Value::Class(_) => {
                let key = Value::string(i.to_string());
                if !js::has(recv, &key)? {
                    return Ok(None);
                }
                self.get_prop(recv, &i.to_string()).map(Some)
            }
            _ => Ok(None),
        }
    }
    pub(crate) fn array_like_values(&mut self, recv: &Value) -> RResult<Vec<Value>> {
        let len = self.array_like_len(recv)?;
        (0..len)
            .map(|i| self.array_like_at(recv, i).map(|v| v.unwrap_or(Value::Undef)))
            .collect()
    }
    fn array_like_set(&mut self, recv: &Value, i: usize, value: Value) -> RResult<()> {
        if let Value::List(l) = recv {
            crate::value::note_write(recv, &value);
            let mut data = l.write()?;
            if i >= data.len() {
                data.resize(i + 1, Value::Undef)
            }
            data[i] = value;
            Ok(())
        } else {
            self.set_member(recv, &Value::string(i.to_string()), value)
        }
    }
    fn array_like_delete(&mut self, recv: &Value, i: usize) -> RResult<()> {
        match recv {
            Value::List(l) => {
                let mut data = l.write()?;
                if i < data.len() {
                    if i + 1 != data.len() {
                        return err("sparse arrays are not supported");
                    }
                    data.pop();
                }
                Ok(())
            }
            Value::Map(m) => {
                m.write()?.shift_remove(i.to_string().as_str());
                Ok(())
            }
            _ => err("TypeError: Cannot delete indexed property"),
        }
    }
    fn array_like_length(&mut self, recv: &Value, len: usize) -> RResult<()> {
        self.set_member(recv, &Value::str("length"), Value::Int(len as i64))
    }
    fn array_like_move(&mut self, recv: &Value, from: usize, to: usize) -> RResult<()> {
        match self.array_like_at(recv, from)? {
            Some(v) => self.array_like_set(recv, to, v),
            None => self.array_like_delete(recv, to),
        }
    }

    pub(crate) fn array_method(&mut self, recv: &Value, name: &str, a: Vec<Value>) -> RResult<Value> {
        let a0 = a.first().cloned().unwrap_or(Value::Undef);
        if name == "toString" {
            let join = self.get_prop(recv, "join")?;
            return if join.is_callable() {
                self.call_with_receiver(&join, recv.clone(), vec![])
            } else {
                self.object_method(recv, "toString", &[])
            };
        }
        if name == "concat" {
            let mut out = if matches!(recv, Value::List(_)) {
                self.array_like_values(recv)?
            } else {
                vec![recv.clone()]
            };
            for value in a {
                if let Value::List(l) = value {
                    js::bulk_array(out.len() + l.read().len())?;
                    out.extend(l.read().iter().cloned())
                } else {
                    out.push(value)
                }
            }
            return Ok(Value::list(out));
        }
        let len = self.array_like_len(recv)?;
        if matches!(
            name,
            "filter"
                | "map"
                | "forEach"
                | "every"
                | "some"
                | "find"
                | "findIndex"
                | "findLast"
                | "findLastIndex"
                | "reduce"
                | "reduceRight"
                | "flatMap"
        ) && !callable(&a0)
        {
            return err(not_callable(&a0));
        }
        let callback = |it: &mut Self, x: Value, i: usize| {
            it.call_with_receiver(
                &a0,
                a.get(1).cloned().unwrap_or(Value::Undef),
                vec![x, Value::Int(i as i64), recv.clone()],
            )
        };
        Ok(match name {
            "filter" | "map" | "flatMap" | "forEach" | "every" | "some" => {
                let mut out = vec![];
                for i in 0..len {
                    let Some(x) = self.array_like_at(recv, i)? else {
                        if name == "map" {
                            return err("sparse array results are not supported");
                        }
                        continue;
                    };
                    let result = callback(self, x.clone(), i)?;
                    match name {
                        "filter" if js::truthy(&result) => out.push(x),
                        "map" => out.push(result),
                        "flatMap" => {
                            if let Value::List(l) = result {
                                out.extend(l.read().iter().cloned())
                            } else {
                                out.push(result)
                            }
                        }
                        "every" if !js::truthy(&result) => return Ok(Value::Bool(false)),
                        "some" if js::truthy(&result) => return Ok(Value::Bool(true)),
                        _ => {}
                    }
                }
                match name {
                    "every" => Value::Bool(true),
                    "some" => Value::Bool(false),
                    "forEach" => Value::Undef,
                    _ => Value::list(out),
                }
            }
            "find" | "findIndex" | "findLast" | "findLastIndex" => {
                for offset in 0..len {
                    let i = if name.starts_with("findLast") {
                        len - 1 - offset
                    } else {
                        offset
                    };
                    let x = self.array_like_at(recv, i)?.unwrap_or(Value::Undef);
                    if js::truthy(&callback(self, x.clone(), i)?) {
                        return Ok(if name.ends_with("Index") {
                            Value::Int(i as i64)
                        } else {
                            x
                        });
                    }
                }
                if name.ends_with("Index") {
                    Value::Int(-1)
                } else {
                    Value::Undef
                }
            }
            "reduce" | "reduceRight" => {
                let mut acc = a.get(1).cloned();
                for offset in 0..len {
                    let i = if name == "reduceRight" {
                        len - 1 - offset
                    } else {
                        offset
                    };
                    let Some(x) = self.array_like_at(recv, i)? else {
                        continue;
                    };
                    acc = Some(match acc {
                        None => x,
                        Some(acc) => self.call_with_receiver(
                            &a0,
                            Value::Undef,
                            vec![acc, x, Value::Int(i as i64), recv.clone()],
                        )?,
                    });
                }
                acc.ok_or("TypeError: Reduce of empty array with no initial value")?
            }
            "flat" => {
                let depth = if matches!(a0, Value::Undef) {
                    1.0
                } else {
                    to_integer(&a0)
                };
                // One level over a plain array (the common case: a list of lists): copy directly,
                // charging the fuel the general walk below would.
                if depth == 1.0
                    && let Value::List(l) = &recv
                {
                    let items = l.read();
                    let mut out = Vec::with_capacity(items.len());
                    let mut steps = 1;
                    for x in items.iter() {
                        steps += 1;
                        match x {
                            Value::List(inner) => {
                                let inner = inner.read();
                                steps += inner.len() + 1;
                                // Bounded as it grows, before the work is paid for below.
                                js::bulk_array(out.len() + inner.len())?;
                                out.extend(inner.iter().cloned());
                            }
                            other => out.push(other.clone()),
                        }
                    }
                    drop(items);
                    for _ in 0..steps {
                        self.spend()?;
                    }
                    return Ok(Value::list(out));
                }
                let mut stack = vec![(recv.clone(), 0, len, depth, None)];
                let mut out = vec![];
                let mut active = std::collections::HashSet::new();
                while let Some((source, index, length, depth, id)) = stack.last_mut() {
                    self.spend()?;
                    if *index == *length {
                        if let Some(id) = id {
                            active.remove(id);
                        }
                        stack.pop();
                        continue;
                    }
                    let next = *index;
                    *index += 1;
                    let depth = *depth;
                    let Some(value) = self.array_like_at(source, next)? else {
                        continue;
                    };
                    match value {
                        Value::List(l) if depth >= 1.0 => {
                            let id = if depth - 1.0 == depth {
                                let id = Rc::as_ptr(&l) as usize;
                                if !active.insert(id) {
                                    return err("RangeError: circular array cannot be flattened to infinite depth");
                                }
                                Some(id)
                            } else {
                                None
                            };
                            let length = l.read().len();
                            stack.push((Value::List(l), 0, length, depth - 1.0, id));
                        }
                        other => out.push(other),
                    }
                }
                Value::list(out)
            }
            "includes" | "indexOf" | "lastIndexOf" => {
                let start = if name == "lastIndexOf" {
                    match a.get(1) {
                        None => len as f64 - 1.0,
                        Some(v) => {
                            let n = to_integer(v);
                            if n < 0.0 {
                                len as f64 + n
                            } else {
                                n.min(len as f64 - 1.0)
                            }
                        }
                    }
                } else {
                    rel_index(a.get(1), len, 0) as f64
                };
                for offset in 0..len {
                    let index = if name == "lastIndexOf" {
                        start - offset as f64
                    } else {
                        start + offset as f64
                    };
                    if index < 0.0 || index >= len as f64 {
                        break;
                    }
                    let item = self.array_like_at(recv, index as usize)?;
                    if (name == "includes" || item.is_some())
                        && if name == "includes" {
                            js::same_value_zero(&item.unwrap_or(Value::Undef), &a0)
                        } else {
                            js::strict_eq(&item.unwrap_or(Value::Undef), &a0)
                        }
                    {
                        return Ok(if name == "includes" {
                            Value::Bool(true)
                        } else {
                            Value::Int(index as i64)
                        });
                    }
                }
                if name == "includes" {
                    Value::Bool(false)
                } else {
                    Value::Int(-1)
                }
            }
            "at" => {
                let n = to_integer(&a0);
                let i = if n < 0.0 { len as f64 + n } else { n };
                if i >= 0.0 && i < len as f64 {
                    self.array_like_at(recv, i as usize)?.unwrap_or(Value::Undef)
                } else {
                    Value::Undef
                }
            }
            "slice" => {
                let (start, end) = (rel_index(a.first(), len, 0), rel_index(a.get(1), len, len));
                let mut out = vec![];
                for i in start..end {
                    out.push(
                        self.array_like_at(recv, i)?
                            .ok_or("sparse array results are not supported")?,
                    )
                }
                Value::list(out)
            }
            "join" => {
                let separator = if matches!(a0, Value::Undef) {
                    ",".to_string()
                } else {
                    js::try_to_string(&a0)?
                };
                if matches!(recv, Value::List(_)) {
                    Value::string(js::array_join(recv, &separator)?)
                } else {
                    let mut out = js::StrBuf::default();
                    for i in 0..len {
                        if i > 0 {
                            out.append(&separator)?
                        }
                        let value = self.array_like_at(recv, i)?.unwrap_or(Value::Undef);
                        if !matches!(value, Value::None | Value::Undef) {
                            out.append(&js::try_to_string(&value)?)?;
                        }
                    }
                    Value::string(out.into_string())
                }
            }
            "keys" => Value::list((0..len).map(|i| Value::Int(i as i64)).collect()),
            "values" => Value::list(self.array_like_values(recv)?),
            "entries" => Value::list(
                self.array_like_values(recv)?
                    .into_iter()
                    .enumerate()
                    .map(|(i, v)| Value::list(vec![Value::Int(i as i64), v]))
                    .collect(),
            ),
            "toSorted" => {
                let values = self.array_like_values(recv)?;
                Value::list(self.sort_values(values, a.first())?)
            }
            "toReversed" => Value::list(self.array_like_values(recv)?.into_iter().rev().collect()),
            "toSpliced" => {
                let copy = Value::list(self.array_like_values(recv)?);
                self.array_method(&copy, "splice", a)?;
                copy
            }
            "with" => {
                let i = to_integer(&a0);
                let i = if i < 0.0 { len as f64 + i } else { i };
                if !(i >= 0.0 && i < len as f64) {
                    return err("RangeError: Invalid index");
                }
                let mut out = vec![];
                for j in 0..len {
                    out.push(if j == i as usize {
                        a.get(1).cloned().unwrap_or(Value::Undef)
                    } else {
                        self.array_like_at(recv, j)?.unwrap_or(Value::Undef)
                    })
                }
                Value::list(out)
            }
            "push" | "unshift" => {
                let count = a.len();
                if count > 1 {
                    js::bulk_array(len + count)?
                }
                if name == "unshift" && count > 0 {
                    for i in (0..len).rev() {
                        self.array_like_move(recv, i, i + count)?
                    }
                }
                for (i, v) in a.into_iter().enumerate() {
                    self.array_like_set(recv, if name == "push" { len + i } else { i }, v)?
                }
                self.array_like_length(recv, len + count)?;
                Value::Int((len + count) as i64)
            }
            "pop" | "shift" => {
                let value = if len == 0 {
                    Value::Undef
                } else {
                    self.array_like_at(recv, if name == "pop" { len - 1 } else { 0 })?
                        .unwrap_or(Value::Undef)
                };
                if len > 0 {
                    if name == "shift" {
                        for i in 1..len {
                            self.array_like_move(recv, i, i - 1)?
                        }
                    }
                    self.array_like_delete(recv, len - 1)?
                }
                self.array_like_length(recv, len.saturating_sub(1))?;
                value
            }
            "reverse" => {
                for i in 0..len / 2 {
                    let j = len - i - 1;
                    let left = self.array_like_at(recv, i)?;
                    let right = self.array_like_at(recv, j)?;
                    if let Some(v) = right {
                        self.array_like_set(recv, i, v)?
                    } else {
                        self.array_like_delete(recv, i)?
                    }
                    if let Some(v) = left {
                        self.array_like_set(recv, j, v)?
                    } else {
                        self.array_like_delete(recv, j)?
                    }
                }
                recv.clone()
            }
            "fill" => {
                let (start, end) = (rel_index(a.get(1), len, 0), rel_index(a.get(2), len, len));
                for i in start..end {
                    self.array_like_set(recv, i, a0.clone())?
                }
                recv.clone()
            }
            "sort" => {
                let mut values = vec![];
                for i in 0..len {
                    if let Some(v) = self.array_like_at(recv, i)? {
                        values.push(v)
                    }
                }
                let values = self.sort_values(values, a.first())?;
                let count = values.len();
                for (i, v) in values.into_iter().enumerate() {
                    self.array_like_set(recv, i, v)?
                }
                for i in (count..len).rev() {
                    self.array_like_delete(recv, i)?
                }
                recv.clone()
            }
            "splice" => {
                let start = rel_index(a.first(), len, 0);
                let delete = match a.len() {
                    0 => 0,
                    1 => len - start,
                    _ => to_integer(&a[1]).clamp(0.0, (len - start) as f64) as usize,
                };
                let insert = a.len().saturating_sub(2);
                if insert > 1 {
                    js::bulk_array(len - delete + insert)?
                }
                let mut removed = vec![];
                for i in start..start + delete {
                    removed.push(
                        self.array_like_at(recv, i)?
                            .ok_or("sparse array results are not supported")?,
                    )
                }
                if insert < delete {
                    for i in start..len - delete {
                        self.array_like_move(recv, i + delete, i + insert)?
                    }
                    for i in (len - delete + insert..len).rev() {
                        self.array_like_delete(recv, i)?
                    }
                } else if insert > delete {
                    for i in (start..len - delete).rev() {
                        self.array_like_move(recv, i + delete, i + insert)?
                    }
                }
                for (i, v) in a.into_iter().skip(2).enumerate() {
                    self.array_like_set(recv, start + i, v)?
                }
                self.array_like_length(recv, len - delete + insert)?;
                Value::list(removed)
            }
            _ => return err(crate::interp::not_a_function("array", name)),
        })
    }

    pub(crate) fn string_method(&mut self, s: &Text, name: &str, a: &[Value]) -> RResult<Value> {
        let a0 = a.first().cloned().unwrap_or(Value::Undef);
        let a1 = a.get(1);
        let text = |v: &Value| -> RResult<String> {
            if matches!(v, Value::Regex(_)) {
                return err(format!(
                    "TypeError: First argument to String.prototype.{name} must not be a regular expression"
                ));
            }
            Ok(js::to_string(v))
        };
        let len = || s.char_count();
        Ok(match name {
            "trim" | "trimStart" | "trimEnd" | "trimLeft" | "trimRight" => {
                let (l, r) = (
                    name != "trimEnd" && name != "trimRight",
                    name != "trimStart" && name != "trimLeft",
                );
                let mut t: &str = s;
                if l {
                    t = t.trim_start_matches(js::is_js_space);
                }
                if r {
                    t = t.trim_end_matches(js::is_js_space);
                }
                Value::str(t)
            }
            "toLowerCase" | "toLocaleLowerCase" => Value::string(s.to_lowercase()),
            "toUpperCase" | "toLocaleUpperCase" => Value::string(s.to_uppercase()),
            "toString" | "valueOf" => Value::Str(s.clone()),
            "startsWith" => {
                let needle = text(&a0)?;
                let start = clamp_index(a1, len(), 0);
                Value::Bool(s[s.char_to_byte(start)..].starts_with(needle.as_str()))
            }
            "endsWith" => {
                let needle = text(&a0)?;
                let end = clamp_index(a1, len(), len());
                Value::Bool(s[..s.char_to_byte(end)].ends_with(needle.as_str()))
            }
            "includes" => {
                let needle = text(&a0)?;
                let start = clamp_index(a1, len(), 0);
                Value::Bool(s[s.char_to_byte(start)..].contains(needle.as_str()))
            }
            "indexOf" => {
                let needle = js::to_string(&a0);
                let from = s.char_to_byte(clamp_index(a1, len(), 0));
                Value::Int(
                    s[from..]
                        .find(needle.as_str())
                        .map_or(-1, |b| s.byte_to_char(from + b) as i64),
                )
            }
            "lastIndexOf" => {
                let needle = js::to_string(&a0);
                let n = a1.map(js::to_number).unwrap_or(f64::NAN);
                let start = if n.is_nan() {
                    len()
                } else {
                    n.trunc().clamp(0.0, len() as f64) as usize
                };
                let mut end = (s.char_to_byte(start) + needle.len()).min(s.len());
                while !s.is_char_boundary(end) {
                    end -= 1;
                }
                Value::Int(s[..end].rfind(needle.as_str()).map_or(-1, |b| s.byte_to_char(b) as i64))
            }
            "charAt" | "charCodeAt" | "codePointAt" | "at" => {
                let mut i = to_integer(&a0);
                if name == "at" && i < 0.0 {
                    i += len() as f64;
                }
                let c = if i >= 0.0 { s.chars().nth(i as usize) } else { None };
                match (name, c) {
                    ("charAt", c) => Value::string(c.map(String::from).unwrap_or_default()),
                    ("charCodeAt", Some(c)) => Value::Int(c as i64),
                    ("charCodeAt", None) => Value::Float(f64::NAN),
                    (_, Some(c)) if name == "codePointAt" => Value::Int(c as i64),
                    (_, Some(c)) => Value::string(c.to_string()),
                    (_, None) => Value::Undef,
                }
            }
            "substring" => {
                let n = len();
                let (x, y) = (clamp_index(Some(&a0), n, 0), clamp_index(a1, n, n));
                Value::str(s.char_slice(x.min(y), x.max(y)))
            }
            "substr" => {
                let n = len();
                let start = rel_index(Some(&a0), n, 0);
                let count = match a1 {
                    None | Some(Value::Undef) => n - start,
                    Some(v) => to_integer(v).clamp(0.0, (n - start) as f64) as usize,
                };
                Value::str(s.char_slice(start, start + count))
            }
            "slice" => {
                let n = len();
                let (x, y) = (rel_index(Some(&a0), n, 0), rel_index(a1, n, n));
                Value::str(s.char_slice(x, y))
            }
            "padStart" | "padEnd" => {
                let width = to_integer(&a0).max(0.0);
                let fill = match a1 {
                    None | Some(Value::Undef) => " ".to_string(),
                    Some(v) => js::to_string(v),
                };
                if width > js::MAX_STRING_LENGTH as f64 && width > len() as f64 && !fill.is_empty() {
                    return err(js::INVALID_STRING_LENGTH);
                }
                let width = width as usize;
                let n = len();
                if n >= width || fill.is_empty() {
                    Value::Str(s.clone())
                } else {
                    let pad: String = fill.chars().cycle().take(width - n).collect();
                    Value::string(if name == "padStart" {
                        pad + s
                    } else {
                        s.to_string() + &pad
                    })
                }
            }
            "repeat" => {
                let n = to_integer(&a0);
                if n < 0.0 || n.is_infinite() {
                    return err(format!("RangeError: Invalid count value: {}", js::to_string(&a0)));
                }
                if s.chars().count() as f64 * n > js::MAX_STRING_LENGTH as f64 {
                    return err(js::INVALID_STRING_LENGTH);
                }
                Value::string(s.repeat(n as usize))
            }
            "concat" => {
                let mut out = js::StrBuf::from(s.to_string());
                for x in a {
                    out.append(&js::try_to_string(x)?)?;
                }
                Value::string(out.into_string())
            }
            "split" => {
                let limit = match a1 {
                    None | Some(Value::Undef) => u32::MAX,
                    Some(v) => js::to_uint32(v),
                } as usize;
                let parts: Vec<Value> = match &a0 {
                    Value::Undef => vec![Value::Str(s.clone())],
                    sep => {
                        let sep = js::to_string(sep);
                        // Counted only when there can be too many parts.
                        let parts = match sep.len() {
                            0 => s.char_count(),
                            n if s.len() / n < js::MAX_BULK_ARRAY => 0,
                            _ => s.matches(sep.as_str()).count() + 1,
                        };
                        js::bulk_array(parts.min(limit))?;
                        if sep.is_empty() {
                            s.chars().take(limit).map(|c| Value::string(c.to_string())).collect()
                        } else {
                            s.split(sep.as_str()).take(limit).map(Value::str).collect()
                        }
                    }
                };
                Value::list(parts.into_iter().take(limit).collect())
            }
            "replace" | "replaceAll" => {
                let pat = js::to_string(&a0);
                let with = a1.cloned().unwrap_or(Value::Undef);
                let all = name == "replaceAll";
                // Match positions (bytes), left to right, not overlapping; an empty pattern
                // matches before every character and at the end.
                let mut at = vec![];
                if pat.is_empty() {
                    at.push(0);
                    if all {
                        at.extend(s.char_indices().skip(1).map(|(i, _)| i));
                        if !s.is_empty() {
                            at.push(s.len());
                        }
                    }
                } else {
                    let mut from = 0;
                    while let Some(p) = s[from..].find(pat.as_str()) {
                        at.push(from + p);
                        from += p + pat.len();
                        if !all {
                            break;
                        }
                    }
                }
                let mut out = js::StrBuf::from(String::with_capacity(s.len()));
                let mut last = 0;
                for p in at {
                    out.push_str(&s[last..p]);
                    let rep = if callable(&with) {
                        let r = self.call_value(
                            &with,
                            vec![
                                Value::str(&pat),
                                Value::Int(s.byte_to_char(p) as i64),
                                Value::Str(s.clone()),
                            ],
                        )?;
                        js::to_string(&r)
                    } else {
                        substitute(&js::to_string(&with), &pat, &s[..p], &s[p + pat.len()..])
                    };
                    out.append(&rep)?;
                    last = p + pat.len();
                }
                out.append(&s[last..])?;
                Value::string(out.into_string())
            }
            "localeCompare" => Value::Int(locale_compare(s, &js::to_string(&a0))),
            "isWellFormed" => Value::Bool(true),
            "toWellFormed" => Value::Str(s.clone()),
            _ => return err(crate::interp::not_a_function("string", name)),
        })
    }

    /// `n.toFixed(d)`, `n.toString(radix)`, `b.toString()`, `o.toString()`; `None`: not one of these.
    pub(crate) fn primitive_method(&mut self, recv: &Value, name: &str, a: &[Value]) -> RResult<Option<Value>> {
        let a0 = a.first().cloned().unwrap_or(Value::Undef);
        Ok(Some(match (recv, name) {
            (Value::Int(_) | Value::Float(_) | Value::BigInt(_), "toFixed") => {
                Value::string(to_fixed(js::to_number(recv), to_integer(&a0))?)
            }
            (Value::Int(_) | Value::Float(_) | Value::BigInt(_), "toString") => match a0 {
                Value::Undef => Value::string(js::to_string(recv)),
                r => {
                    let radix = to_integer(&r);
                    if !(2.0..=36.0).contains(&radix) {
                        return err("RangeError: toString() radix must be between 2 and 36");
                    }
                    if radix == 10.0 {
                        Value::string(js::to_string(recv))
                    } else {
                        Value::string(radix_string(js::to_number(recv), radix as u32))
                    }
                }
            },
            (Value::Int(_) | Value::Float(_) | Value::BigInt(_) | Value::Bool(_), "valueOf") => recv.clone(),
            (Value::Bool(_), "toString") => Value::string(js::to_string(recv)),
            _ => return Ok(None),
        }))
    }

    /// `JSON.stringify(value, replacer, space)`.
    pub(crate) fn json_stringify(&mut self, a: &[Value]) -> RResult<Value> {
        let v = a.first().cloned().unwrap_or(Value::Undef);
        let replacer = a.get(1).cloned().unwrap_or(Value::Undef);
        let gap = match a.get(2) {
            Some(Value::Int(_) | Value::Float(_)) => " ".repeat(to_integer(&a[2]).clamp(0.0, 10.0) as usize),
            Some(Value::Str(s)) => s.chars().take(10).collect(),
            _ => String::new(),
        };
        let fields: Option<Vec<String>> = match &replacer {
            Value::List(l) => {
                let mut f: Vec<String> = vec![];
                for x in l.read().iter() {
                    if matches!(x, Value::Str(_) | Value::Int(_) | Value::Float(_)) {
                        let k = js::to_string(x);
                        if !f.contains(&k) {
                            f.push(k);
                        }
                    }
                }
                Some(f)
            }
            _ => None,
        };
        let replacer = if callable(&replacer) { Some(replacer) } else { None };
        let mut out = js::StrBuf::default();
        let mut wrapper = Map::default();
        wrapper.insert(Rc::from(""), v.clone());
        let ok = self.json_value(
            &Value::map(wrapper),
            &Value::str(""),
            v,
            &replacer,
            fields.as_deref(),
            &gap,
            "",
            &mut out,
        )?;
        Ok(if ok {
            Value::string(out.into_string())
        } else {
            Value::Undef
        })
    }

    /// One value of `JSON.stringify`; false if it serializes to nothing (undefined, a function).
    #[allow(clippy::too_many_arguments)]
    fn json_value(
        &mut self,
        holder: &Value,
        key: &Value,
        v: Value,
        replacer: &Option<Value>,
        fields: Option<&[String]>,
        gap: &str,
        indent: &str,
        out: &mut js::StrBuf,
    ) -> RResult<bool> {
        if self.json_path.len() >= 400 {
            return err("RangeError: JSON nesting exceeds 400 levels");
        }
        let wrote = stacker::maybe_grow(128 * 1024, 2 * 1024 * 1024, || {
            self.json_value_inner(holder, key, v, replacer, fields, gap, indent, out)
        })?;
        out.check()?;
        Ok(wrote)
    }

    #[allow(clippy::too_many_arguments)]
    fn json_value_inner(
        &mut self,
        holder: &Value,
        key: &Value,
        mut v: Value,
        replacer: &Option<Value>,
        fields: Option<&[String]>,
        gap: &str,
        indent: &str,
        out: &mut js::StrBuf,
    ) -> RResult<bool> {
        if matches!(v, Value::Map(_) | Value::List(_) | Value::Class(_)) {
            let function = self.get_prop(&v, "toJSON")?;
            if function.is_callable() {
                v = self.call_with_receiver(&function, v, vec![key.clone()])?;
            }
        }
        if let Some(r) = replacer {
            v = self.call_with_receiver(r, holder.clone(), vec![key.clone(), v])?;
        }
        let p = match &v {
            Value::List(l) => Rc::as_ptr(l) as *const () as usize,
            Value::Map(m) => Rc::as_ptr(m) as *const () as usize,
            _ => 0,
        };
        if p != 0 {
            if self.json_path.contains(&p) {
                return err("TypeError: Converting circular structure to JSON");
            }
            self.json_path.push(p);
        }
        let r = self.json_one(key, &v, replacer, fields, gap, indent, out);
        if p != 0 {
            self.json_path.pop();
        }
        r
    }

    #[allow(clippy::too_many_arguments)]
    fn json_one(
        &mut self,
        key: &Value,
        v: &Value,
        replacer: &Option<Value>,
        fields: Option<&[String]>,
        gap: &str,
        indent: &str,
        out: &mut js::StrBuf,
    ) -> RResult<bool> {
        let _ = key;
        match v {
            Value::None => out.push_str("null"),
            Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Value::Int(i) => out.push_str(&i.to_string()),
            // A host integer beyond 2^53 keeps its digits (it never was a double).
            Value::BigInt(s) => out.push_str(s),
            Value::Float(f) => out.push_str(&if f.is_finite() {
                js::number_string(*f)
            } else {
                "null".into()
            }),
            Value::Str(s) => {
                out.fits(s)?;
                json_quote(s, out)
            }
            Value::Native(n) if n.is_namespace() => out.push_str("{}"),
            Value::Class(_)
            | Value::Method(_)
            | Value::Native(_)
            | Value::Undef
            | Value::Lambda(..)
            | Value::Func(_)
            | Value::Bound(_)
            | Value::Setter(_) => return Ok(false),
            Value::List(l) => {
                if l.read().is_empty() {
                    out.push_str("[]");
                    return Ok(true);
                }
                let inner = format!("{indent}{gap}");
                out.push('[');
                let len = l.read().len();
                for i in 0..len {
                    let x = l.read().get(i).cloned().unwrap_or(Value::Undef);
                    if i > 0 {
                        out.push(',');
                    }
                    if !gap.is_empty() {
                        out.push('\n');
                        out.push_str(&inner);
                    }
                    if !self.json_value(v, &Value::string(i.to_string()), x, replacer, fields, gap, &inner, out)? {
                        out.push_str("null");
                    }
                }
                if !gap.is_empty() {
                    out.push('\n');
                    out.push_str(indent);
                }
                out.push(']');
            }
            Value::Map(m) => {
                let inner = format!("{indent}{gap}");
                let keys: Vec<Rc<str>> = match fields {
                    Some(f) => f
                        .iter()
                        .filter(|k| m.read().contains_key(k.as_str()))
                        .map(|k| Rc::from(k.as_str()))
                        .collect(),
                    None => m.read().keys().cloned().collect(),
                };
                out.push('{');
                let mut any = false;
                for k in keys {
                    let x = m.read().get(&*k).cloned().unwrap_or(Value::Undef);
                    let mark = out.len();
                    if any {
                        out.push(',');
                    }
                    if !gap.is_empty() {
                        out.push('\n');
                        out.push_str(&inner);
                    }
                    json_quote(&k, out);
                    out.push(':');
                    if !gap.is_empty() {
                        out.push(' ');
                    }
                    if self.json_value(v, &Value::Str(k.into()), x, replacer, fields, gap, &inner, out)? {
                        any = true;
                    } else {
                        out.truncate(mark);
                    }
                }
                if any && !gap.is_empty() {
                    out.push('\n');
                    out.push_str(indent);
                }
                out.push('}');
            }
            Value::Regex(_) | Value::Promise(_) | Value::Coll(_) => out.push_str("{}"),
            // A model turn's exact token ids write out as the id list.
            Value::Frag(f) if f.iter().all(|i| matches!(i, crate::emit::Item::Tokens(..))) && !f.is_empty() => {
                let j = v.to_json();
                out.push_str(&serde_json::to_string(&j).unwrap());
            }
            _ => out.push_str("null"),
        }
        Ok(true)
    }

    /// `JSON.parse(text, reviver)`.
    pub(crate) fn json_parse(&mut self, a: &[Value]) -> RResult<Value> {
        let text = js::to_string(a.first().unwrap_or(&Value::Undef));
        let v = json_parse_text(&text)?;
        match a.get(1) {
            Some(r) if callable(r) => {
                let r = r.clone();
                let mut holder = Map::default();
                holder.insert(Rc::from(""), v);
                self.revive(&Value::map(holder), &Value::str(""), &r)
            }
            _ => Ok(v),
        }
    }

    /// `JSON.parse`'s reviver: children first; an undefined result deletes the member.
    fn revive(&mut self, holder: &Value, key: &Value, function: &Value) -> RResult<Value> {
        stacker::maybe_grow(128 * 1024, 2 * 1024 * 1024, || self.revive_inner(holder, key, function))
    }
    fn revive_inner(&mut self, holder: &Value, key: &Value, function: &Value) -> RResult<Value> {
        let value = crate::jsvalue::index(holder, key)?;
        match &value {
            Value::List(list) => {
                let len = list.read().len();
                for i in 0..len {
                    let item = self.revive(&value, &Value::string(i.to_string()), function)?;
                    crate::value::note_write(&value, &item);
                    let mut data = list.write()?;
                    if !matches!(item, Value::Undef) && i >= data.len() {
                        data.resize(i + 1, Value::Undef);
                    }
                    if i < data.len() {
                        data[i] = item;
                    }
                }
            }
            Value::Map(map) => {
                let keys = map.read().keys().cloned().collect::<Vec<_>>();
                for key in keys {
                    let item = self.revive(&value, &Value::Str((&key).into()), function)?;
                    if matches!(item, Value::Undef) {
                        map.write()?.shift_remove(&*key);
                    } else {
                        crate::value::note_write(&value, &item);
                        map.write()?.insert(key, item);
                    }
                }
            }
            _ => {}
        }
        self.call_with_receiver(function, holder.clone(), vec![key.clone(), value])
    }

    /// `Array.from(items, mapFn)`.
    pub(crate) fn array_from(&mut self, a: &[Value]) -> RResult<Value> {
        let a0 = a.first().cloned().unwrap_or(Value::Undef);
        let items = match &a0 {
            Value::Str(s) => s.chars().map(|c| Value::string(c.to_string())).collect(),
            // `Array.from({length: n})`: n undefined values (pair with `(_, i) => i`).
            Value::Map(m) => {
                let n = m.read().get("length").map(to_integer).unwrap_or(0.0).clamp(0.0, 1e7) as usize;
                (0..n)
                    .map(|i| m.read().get(i.to_string().as_str()).cloned().unwrap_or(Value::Undef))
                    .collect()
            }
            Value::Undef | Value::None => return err(format!("TypeError: {} is not iterable", js::to_string(&a0))),
            v => crate::interp::iterate(v).unwrap_or_default(),
        };
        match a.get(1) {
            Some(f) if !matches!(f, Value::Undef) => {
                if !callable(f) {
                    return err(not_callable(f));
                }
                let f = f.clone();
                let mut out = Vec::with_capacity(items.len());
                for (i, x) in items.into_iter().enumerate() {
                    out.push(self.call_value(&f, vec![x, Value::Int(i as i64)])?);
                }
                Ok(Value::list(out))
            }
            _ => Ok(Value::list(items)),
        }
    }
}

/// `Object.keys` / `values` / `entries` / `assign` / `fromEntries` / `is`, `Number.*`,
/// `String.fromCharCode`, `Array(..)`; `None`: not one of these.
pub fn object_global(name: &str, a: &[Value]) -> Option<RResult<Value>> {
    let a0 = a.first().cloned().unwrap_or(Value::Undef);
    let to_object = |v: &Value| -> RResult<Vec<(Rc<str>, Value)>> {
        Ok(match v {
            Value::Map(m) => m.read().iter().map(|(k, x)| (k.clone(), x.clone())).collect(),
            Value::List(l) => l
                .read()
                .iter()
                .enumerate()
                .map(|(i, x)| (Rc::from(i.to_string()), x.clone()))
                .collect(),
            Value::Str(s) => s
                .chars()
                .enumerate()
                .map(|(i, c)| (Rc::from(i.to_string()), Value::string(c.to_string())))
                .collect(),
            Value::Undef | Value::None => return err("TypeError: Cannot convert undefined or null to object"),
            _ => vec![],
        })
    };
    Some((|| -> RResult<Value> {
        Ok(match name {
            // Shallow, as in JavaScript (the runtime's own freeze is deep: value::freeze).
            "Object.freeze" => {
                match &a0 { Value::List(l) => l.freeze_shallow(), Value::Map(m) => m.freeze_shallow(), _ => {} }
                a0.clone()
            }
            "Object.isFrozen" => Value::Bool(match &a0 { Value::List(l) => l.is_frozen(), Value::Map(m) => m.is_frozen(), _ => true }),
            "Object.keys" => Value::list(to_object(&a0)?.into_iter().map(|(k, _)| Value::Str(k.into())).collect()),
            "Object.values" => Value::list(to_object(&a0)?.into_iter().map(|(_, v)| v).collect()),
            "Object.entries" => Value::list(to_object(&a0)?.into_iter().map(|(k, v)| Value::list(vec![Value::Str(k.into()), v])).collect()),
            "Object.fromEntries" => {
                let mut m = Map::default();
                for e in crate::interp::iterate(&a0)? {
                    if !matches!(e, Value::List(_) | Value::Map(_)) {
                        return err(format!("TypeError: Iterator value {} is not an entry object", js::to_string(&e)));
                    }
                    let k = js::index(&e, &Value::Int(0))?;
                    let v = js::index(&e, &Value::Int(1))?;
                    js::obj_insert(&mut m, Rc::from(js::to_string(&k)), v);
                }
                Value::map(m)
            }
            // The target changes in place and is returned.
            "Object.assign" => {
                let Value::Map(m) = &a0 else {
                    return err(if matches!(a0, Value::Undef | Value::None) { "TypeError: Cannot convert undefined or null to object" } else { "Object.assign: the target must be an object" });
                };
                for x in &a[1..] {
                    if !matches!(x, Value::Undef | Value::None) {
                        let entries = to_object(x)?;
                        for (_, v) in &entries { crate::value::note_write(&a0, v) }
                        let mut w = m.write()?;
                        for (k, v) in entries {
                            js::obj_insert(&mut w, k, v);
                        }
                    }
                }
                a0.clone()
            }
            "Object.is" => {
                let b = a.get(1).cloned().unwrap_or(Value::Undef);
                let (x, y) = (js::to_number(&a0), js::to_number(&b));
                let num = |v: &Value| matches!(v, Value::Int(_) | Value::Float(_));
                Value::Bool(if num(&a0) && num(&b) { (x.is_nan() && y.is_nan()) || (x == y && x.is_sign_negative() == y.is_sign_negative()) } else { js::strict_eq(&a0, &b) })
            }
            "Number.isSafeInteger" => Value::Bool(match a0 {
                Value::Int(i) => i.unsigned_abs() < 1 << 53,
                Value::Float(f) => f.trunc() == f && f.abs() <= 9007199254740991.0,
                _ => false,
            }),
            "String.fromCharCode" | "String.fromCodePoint" => {
                if name == "String.fromCodePoint"
                    && let Some(bad) = a.iter().find(|v| {
                        let n = js::to_number(v);
                        !(0.0..=1_114_111.0).contains(&n) || n.fract() != 0.0
                    })
                {
                    return err(format!("RangeError: Invalid code point {}", js::to_string(bad)));
                }
                let units: Vec<u32> = a.iter().map(|v| if name == "String.fromCharCode" { js::to_uint32(v) & 0xffff } else { js::to_number(v) as u32 }).collect();
                let mut s = String::new();
                let mut i = 0;
                while i < units.len() {
                    let u = units[i];
                    let cp = if (0xD800..0xDC00).contains(&u) && units.get(i + 1).is_some_and(|l| (0xDC00..0xE000).contains(l)) {
                        i += 1;
                        0x10000 + ((u - 0xD800) << 10) + (units[i] - 0xDC00)
                    } else {
                        u
                    };
                    match char::from_u32(cp) {
                        Some(c) => s.push(c),
                        None => return err("a lone surrogate is not supported: strings are Unicode text"),
                    }
                    i += 1;
                }
                Value::string(s)
            }
            "Array" => match a {
                [Value::Int(_) | Value::Float(_)] => {
                    let n = js::to_number(&a0);
                    if n.trunc() != n || !(0.0..4294967296.0).contains(&n) {
                        return err("RangeError: Invalid array length");
                    }
                    if n != 0.0 {
                        return err("Array(n) makes a sparse array (not supported): use Array.from({length: n}, ..) or Array(n).fill(x)");
                    }
                    Value::list(vec![])
                }
                items => Value::list(items.to_vec()),
            },
            // `Array(n).fill(x, start, end)` / `new Array(n).fill(x)` (written as one call).
            "__arrayFill" => {
                let n = js::to_number(&a0);
                if n.trunc() != n || !(0.0..4294967296.0).contains(&n) {
                    return err("RangeError: Invalid array length");
                }
                let n = n as usize;
                js::bulk_array(n)?;
                let x = a.get(1).cloned().unwrap_or(Value::Undef);
                let (start, end) = (rel_index(a.get(2), n, 0), rel_index(a.get(3), n, n));
                if start > 0 || end < n {
                    return err("Array(n).fill(x, start, end) leaves holes (sparse arrays are not supported)");
                }
                Value::list(vec![x; n])
            }
            _ => return Err(String::new()),
        })
    })())
    .filter(|r| !matches!(r, Err(e) if e.is_empty()))
}

/// `structuredClone(v)`: a deep copy of data (arrays, objects); the copy is the program's own,
/// mutable. Shared parts stay shared in the copy, and a cycle stays a cycle. Functions and
/// promises cannot be cloned (a DataCloneError, as in JavaScript).
pub fn structured_clone(v: &Value) -> RResult<Value> {
    fn go(v: &Value, seen: &mut std::collections::HashMap<*const (), Value>) -> RResult<Value> {
        stacker::maybe_grow(128 * 1024, 2 * 1024 * 1024, || go_inner(v, seen))
    }
    fn go_inner(v: &Value, seen: &mut std::collections::HashMap<*const (), Value>) -> RResult<Value> {
        Ok(match v {
            Value::List(l) => {
                let p = Rc::as_ptr(l) as *const ();
                if let Some(c) = seen.get(&p) {
                    return Ok(c.clone());
                }
                let out = Value::list(vec![]);
                seen.insert(p, out.clone());
                let items: Vec<Value> = l.read().clone();
                let copied = items.iter().map(|x| go(x, seen)).collect::<RResult<Vec<_>>>()?;
                for value in &copied {
                    crate::value::note_write(&out, value);
                }
                if let Value::List(o) = &out {
                    *o.write()? = copied
                }
                out
            }
            Value::Map(m) => {
                let p = Rc::as_ptr(m) as *const ();
                if let Some(c) = seen.get(&p) {
                    return Ok(c.clone());
                }
                let out = Value::map(Map::default());
                seen.insert(p, out.clone());
                let entries: Vec<(Rc<str>, Value)> = m.read().iter().map(|(k, x)| (k.clone(), x.clone())).collect();
                let mut copied = Map::default();
                for (k, x) in entries {
                    copied.insert(k, go(&x, seen)?);
                }
                for value in copied.values() {
                    crate::value::note_write(&out, value);
                }
                if let Value::Map(o) = &out {
                    *o.write()? = copied
                }
                out
            }
            Value::Coll(c) => {
                let p = Rc::as_ptr(c) as *const ();
                if let Some(x) = seen.get(&p) {
                    return Ok(x.clone());
                }
                let set = c.read().set;
                let out = Value::Coll(Rc::new(crate::value::Obj::new(crate::value::Coll::new(set))));
                seen.insert(p, out.clone());
                let entries: Vec<(Value, Value)> = c.read().iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                let mut copied = crate::value::Coll::new(set);
                for (k, v) in entries {
                    copied.insert(go(&k, seen)?, go(&v, seen)?);
                }
                for (key, value) in copied.iter() {
                    crate::value::note_write(&out, key);
                    crate::value::note_write(&out, value);
                }
                if let Value::Coll(o) = &out {
                    *o.data_mut().map_err(|_| "structuredClone: busy".to_string())? = copied
                }
                out
            }
            Value::Class(_)
            | Value::Method(_)
            | Value::Native(_)
            | Value::Lambda(..)
            | Value::Func(_)
            | Value::Bound(_)
            | Value::Setter(_)
            | Value::Promise(_) => return err("DataCloneError: a function cannot be cloned"),
            // A regex of its own: the copy's `lastIndex` moves without the original's.
            Value::Regex(re) => Value::Regex(Rc::new(crate::jsregex::JsRegex {
                source: re.source.clone(),
                flags: re.flags.clone(),
                global: re.global,
                re: re.re.clone(),
                names: re.names.clone(),
                last_index: std::cell::Cell::new(0),
                frozen: Default::default(),
            })),
            other => other.clone(),
        })
    }
    go(v, &mut std::collections::HashMap::new())
}

/// `new Map(entries)` / `new Set(values)`.
pub fn new_coll(set: bool, init: &Value) -> RResult<Value> {
    let mut c = crate::value::Coll::new(set);
    if !matches!(init, Value::Undef | Value::None) {
        for e in crate::interp::iterate(init)? {
            if set {
                c.insert(e.clone(), e);
                continue;
            }
            if !matches!(e, Value::List(_) | Value::Map(_) | Value::Coll(_)) {
                return err(format!(
                    "TypeError: Iterator value {} is not an entry object",
                    js::to_string(&e)
                ));
            }
            let k = js::index(&e, &Value::Int(0))?;
            let v = js::index(&e, &Value::Int(1))?;
            c.insert(k, v);
        }
    }
    Ok(Value::Coll(Rc::new(crate::value::Obj::new(c))))
}

/// A Map's or Set's iteration in progress (no compaction while it lasts).
pub struct CollWalk(Rc<crate::value::Obj<crate::value::Coll>>);

impl CollWalk {
    pub fn new(c: &Rc<crate::value::Obj<crate::value::Coll>>) -> CollWalk {
        if let Ok(mut w) = c.data_mut() {
            w.iterating += 1
        }
        CollWalk(c.clone())
    }
}

impl Drop for CollWalk {
    fn drop(&mut self) {
        if let Ok(mut w) = self.0.data_mut() {
            w.iterating -= 1
        }
    }
}

/// The next item of a Map (a [key, value] entry) or a Set (a value) from `*at`, skipping holes.
pub fn coll_next(c: &Rc<crate::value::Obj<crate::value::Coll>>, at: &mut usize) -> Option<Value> {
    let c = c.read();
    while *at < c.entries.len() {
        let i = *at;
        *at += 1;
        if let Some((k, v)) = c.entry_at(i) {
            return Some(if c.set {
                k.clone()
            } else {
                Value::list(vec![k.clone(), v.clone()])
            });
        }
    }
    None
}

impl Interp<'_> {
    /// A Map's or a Set's method.
    pub(crate) fn coll_method(
        &mut self,
        recv: &Value,
        c: &Rc<crate::value::Obj<crate::value::Coll>>,
        name: &str,
        a: Vec<Value>,
    ) -> RResult<Value> {
        let a0 = a.first().cloned().unwrap_or(Value::Undef);
        let set = c.read().set;
        let what = if set { "Set" } else { "Map" };
        // A frozen Map or Set is the runtime's (state, a kept render): JavaScript's freeze would
        // not stop these writes, so the error names the boundary.
        let write = || -> RResult<std::cell::RefMut<'_, crate::value::Coll>> {
            if c.is_deep() {
                return err(format!(
                    "TypeError: this {what} is kept by the runtime (state, props, a kept render or a cached result) and cannot change: copy it with new {what}(x) to change it"
                ));
            }
            c.data_mut()
                .map_err(|_| format!("TypeError: this {what} cannot change while it is being read"))
        };
        Ok(match (name, set) {
            ("get", false) => c.read().get(&a0).cloned().unwrap_or(Value::Undef),
            ("has", _) => Value::Bool(c.read().has(&a0)),
            ("set", false) => {
                let v = a.get(1).cloned().unwrap_or(Value::Undef);
                crate::value::note_write(recv, &a0);
                crate::value::note_write(recv, &v);
                write()?.insert(a0, v);
                recv.clone()
            }
            ("add", true) => {
                crate::value::note_write(recv, &a0);
                write()?.insert(a0.clone(), a0);
                recv.clone()
            }
            ("delete", _) => Value::Bool(write()?.remove(&a0)),
            ("clear", _) => {
                write()?.clear();
                Value::Undef
            }
            ("forEach", _) => {
                if !a0.is_callable() {
                    return err(not_callable(&a0));
                }
                let _walk = CollWalk::new(c);
                let mut at = 0usize;
                loop {
                    let next = {
                        let r = c.read();
                        let mut found = None;
                        while at < r.entries.len() {
                            at += 1;
                            if let Some((k, v)) = r.entry_at(at - 1) {
                                found = Some((k.clone(), v.clone()));
                                break;
                            }
                        }
                        found
                    };
                    let Some((k, v)) = next else { break };
                    self.call_with_receiver(&a0, a.get(1).cloned().unwrap_or(Value::Undef), vec![v, k, recv.clone()])?;
                }
                Value::Undef
            }
            // The keys, values and entries, as arrays (JavaScript's are iterators: `.next()` is
            // not there, so a use of one is an error, never a different value).
            ("keys", _) => Value::list(c.read().iter().map(|(k, _)| k.clone()).collect()),
            ("values", _) => Value::list(c.read().iter().map(|(_, v)| v.clone()).collect()),
            ("entries", _) => Value::list(
                c.read()
                    .iter()
                    .map(|(k, v)| Value::list(vec![k.clone(), v.clone()]))
                    .collect(),
            ),
            ("toString", _) => Value::string(js::to_string(recv)),
            _ => return err(format!("TypeError: {} is not a function", name)),
        })
    }
}

/// `Math.max`, `Object.keys`, ...: a built-in function under a namespace.
pub(crate) fn is_global_function(ns: &str, name: &str) -> bool {
    match ns {
        "Math" => crate::jsbuiltins::math(&format!("Math.{name}"), &[]).is_some() || name == "random",
        "Number" => matches!(
            name,
            "isInteger" | "isFinite" | "isNaN" | "isSafeInteger" | "parseFloat" | "parseInt"
        ),
        "Object" => matches!(
            name,
            "keys" | "values" | "entries" | "fromEntries" | "assign" | "is" | "freeze" | "isFrozen"
        ),
        "Map" => name == "groupBy",
        "JSON" => matches!(name, "stringify" | "parse"),
        "Array" => matches!(name, "isArray" | "of" | "from"),
        "Promise" => matches!(name, "all" | "race" | "any" | "allSettled" | "resolve" | "reject"),
        "String" => matches!(name, "fromCharCode" | "fromCodePoint"),
        "Date" => name == "now",
        "console" => crate::console::level(name).is_some(),
        _ => false,
    }
}

/// `Math.PI`, `Number.MAX_SAFE_INTEGER`, ...
pub(crate) fn global_constant(ns: &str, name: &str) -> Option<Value> {
    let f = match (ns, name) {
        ("Math", "PI") => std::f64::consts::PI,
        ("Math", "E") => std::f64::consts::E,
        ("Math", "LN2") => std::f64::consts::LN_2,
        ("Math", "LN10") => std::f64::consts::LN_10,
        ("Math", "LOG2E") => std::f64::consts::LOG2_E,
        ("Math", "LOG10E") => std::f64::consts::LOG10_E,
        ("Math", "SQRT2") => std::f64::consts::SQRT_2,
        ("Math", "SQRT1_2") => std::f64::consts::FRAC_1_SQRT_2,
        ("Number", "MAX_SAFE_INTEGER") => return Some(Value::Int((1 << 53) - 1)),
        ("Number", "MIN_SAFE_INTEGER") => return Some(Value::Int(-((1 << 53) - 1))),
        ("Number", "EPSILON") => f64::EPSILON,
        ("Number", "MAX_VALUE") => f64::MAX,
        ("Number", "MIN_VALUE") => 5e-324,
        ("Number", "POSITIVE_INFINITY") => f64::INFINITY,
        ("Number", "NEGATIVE_INFINITY") => f64::NEG_INFINITY,
        ("Number", "NaN") => f64::NAN,
        _ => return None,
    };
    Some(Value::Float(f))
}
