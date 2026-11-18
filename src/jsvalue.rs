//! JavaScript value semantics: the view language's operators, conversions and comparisons.
use crate::value::{Map, Value};
use std::rc::Rc;

/// ECMAScript ToBoolean.
pub fn truthy(v: &Value) -> bool {
    match v {
        Value::Bool(b) => *b,
        Value::Int(i) => *i != 0,
        Value::BigInt(s) => s.bytes().any(|b| b.is_ascii_digit() && b != b'0'),
        Value::Float(f) => *f != 0.0 && !f.is_nan(),
        Value::Str(s) => !s.is_empty(),
        Value::None | Value::Undef => false,
        // Arrays, objects, fragments, functions: always truthy (even when empty).
        _ => true,
    }
}

/// ECMAScript white space and line terminators (`trim`, `Number(s)`, `parseInt`).
pub fn is_js_space(c: char) -> bool {
    (c.is_whitespace() && c != '\u{85}') || c == '\u{feff}'
}

/// ECMAScript Number::toString(10): the shortest digits that round-trip, in plain notation for
/// decimal exponents -7 < n < 21 and in exponent notation (`1e+21`, `1.5e-7`) otherwise.
pub fn number_string(f: f64) -> String {
    if f.is_nan() {
        return "NaN".into();
    }
    if f == 0.0 {
        return "0".into();
    }
    if f.is_infinite() {
        return if f > 0.0 { "Infinity".into() } else { "-Infinity".into() };
    }
    if f == f.trunc() && f.abs() < 9.0e15 {
        return format!("{}", f as i64);
    }
    let sci = format!("{:e}", f.abs());
    let (mantissa, exp) = sci.split_once('e').unwrap();
    let mut digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let e10 = exp.parse::<i32>().unwrap();
    if let Some(even) = even_tie(f.abs(), &digits, e10) {
        digits = even;
    }
    let k = digits.len() as i32;
    let n = e10 + 1;
    let mut s = String::from(if f < 0.0 { "-" } else { "" });
    if k <= n && n <= 21 {
        s.push_str(&digits);
        s.extend(std::iter::repeat_n('0', (n - k) as usize));
    } else if 0 < n && n <= 21 {
        s.push_str(&digits[..n as usize]);
        s.push('.');
        s.push_str(&digits[n as usize..]);
    } else if -6 < n && n <= 0 {
        s.push_str("0.");
        s.extend(std::iter::repeat_n('0', (-n) as usize));
        s.push_str(&digits);
    } else {
        s.push_str(&digits[..1]);
        if k > 1 {
            s.push('.');
            s.push_str(&digits[1..]);
        }
        s.push('e');
        s.push(if n >= 1 { '+' } else { '-' });
        s.push_str(&(n - 1).abs().to_string());
    }
    s
}

/// When the double lies exactly halfway between two shortest digit strings that both read back
/// as it, JavaScript prints the even one (`2 ** -25` is `2.9802322387695312e-8`); Rust's
/// shortest formatting may pick the odd one. The even string, if it is not `digits`.
fn even_tie(a: f64, digits: &str, e10: i32) -> Option<String> {
    // A tie needs two neighbours within one ulp: only at 16 or more digits; the odd one only.
    if digits.len() < 16 || digits.as_bytes()[digits.len() - 1].is_multiple_of(2) {
        return None;
    }
    // Every double is a finite binary fraction: 1100 places are its exact decimal value.
    let exact = format!("{:.1100e}", a);
    let (m, e) = exact.split_once('e').unwrap();
    if e.parse::<i32>().ok()? != e10 {
        return None;
    }
    let all: Vec<u8> = m.bytes().filter(u8::is_ascii_digit).collect();
    let k = digits.len();
    if all[k] != b'5' || all[k + 1..].iter().any(|&d| d != b'0') {
        return None;
    }
    // The candidates: the exact value truncated to k digits (P) and P + 1.
    let mut p = all[..k].to_vec();
    let truncated = p.clone();
    let mut i = k;
    while i > 0 {
        i -= 1;
        if p[i] == b'9' {
            p[i] = b'0';
        } else {
            p[i] += 1;
            break;
        }
    }
    let other = if digits.as_bytes() == truncated.as_slice() {
        p
    } else {
        truncated
    };
    if other.len() != k || other[0] == b'0' || other[k - 1] % 2 != 0 {
        return None;
    }
    let s = String::from_utf8(other).unwrap();
    let back: f64 = format!("{}.{}e{}", &s[..1], &s[1..], e10).parse().ok()?;
    if back != a {
        return None;
    }
    Some(s.trim_end_matches('0').to_string())
}

/// Infallible diagnostic spelling. Language operations that can reject use try_to_string.
pub fn to_string(v: &Value) -> String {
    try_to_string(v).unwrap_or_else(|e| e)
}

/// ECMAScript ToString, with an iterative path shared by String and array join. Re-entering
/// an array contributes an empty string (as JS join does), not unbounded native recursion.
pub fn try_to_string(v: &Value) -> Result<String, String> {
    stringify(v, None)
}

pub fn array_join(v: &Value, separator: &str) -> Result<String, String> {
    stringify(v, Some(separator))
}

/// V8's longest string (`String::kMaxLength` on 64-bit platforms), counted as `length` counts
/// here (code points). Every way to build a longer one throws, as in V8.
pub const MAX_STRING_LENGTH: usize = 536_870_888;
pub const INVALID_STRING_LENGTH: &str = "RangeError: Invalid string length";
/// The longest array a single native step makes (`Array(n).fill`, `[...s]`, `split`, borrowed
/// array-likes): fuel counts steps, so a bulk allocator is bounded by size instead.
pub const MAX_BULK_ARRAY: usize = 10_000_000;

/// `Err` (V8's "Invalid array length") for an array of `n` items made in one native step.
pub fn bulk_array(n: usize) -> Result<(), String> {
    if n > MAX_BULK_ARRAY {
        Err("RangeError: Invalid array length".into())
    } else {
        Ok(())
    }
}

/// A string being built under `MAX_STRING_LENGTH`. Code points are counted only once the bytes
/// could pass it, and each byte once: the first `counted` bytes hold `chars` code points, so the
/// text cannot be too long while its bytes are within the limit plus `counted - chars`.
#[derive(Default)]
pub struct StrBuf {
    s: String,
    counted: usize,
    chars: usize,
}

impl StrBuf {
    /// Append `piece`, or fail (before allocating) if the result would be too long.
    pub fn append(&mut self, piece: &str) -> Result<(), String> {
        self.fits(piece)?;
        self.s.push_str(piece);
        Ok(())
    }
    /// `Err` if appending `piece` (or as much text, written another way) would pass the limit.
    pub fn fits(&mut self, piece: &str) -> Result<(), String> {
        if !self.s.is_char_boundary(self.counted) {
            (self.counted, self.chars) = (0, 0)
        }
        let len = self.s.len() + piece.len();
        if len > MAX_STRING_LENGTH + (self.counted - self.chars) {
            let chars = self.chars + self.s[self.counted..].chars().count();
            if chars + piece.chars().count() > MAX_STRING_LENGTH {
                return Err(INVALID_STRING_LENGTH.into());
            }
            (self.counted, self.chars) = (self.s.len(), chars);
        }
        Ok(())
    }
    /// Check what unchecked appends (`DerefMut`) added.
    pub fn check(&mut self) -> Result<(), String> {
        self.append("")
    }
    /// Shrinking forgets the count (these shadow `String`'s, which `DerefMut` would reach).
    pub fn truncate(&mut self, n: usize) {
        self.s.truncate(n);
        if n < self.counted {
            (self.counted, self.chars) = (0, 0)
        }
    }
    pub fn split_off(&mut self, at: usize) -> String {
        if at < self.counted {
            (self.counted, self.chars) = (0, 0)
        }
        self.s.split_off(at)
    }
    pub fn into_string(self) -> String {
        self.s
    }
}

impl From<String> for StrBuf {
    fn from(s: String) -> StrBuf {
        StrBuf { s, ..StrBuf::default() }
    }
}

impl std::ops::Deref for StrBuf {
    type Target = String;
    fn deref(&self) -> &String {
        &self.s
    }
}

impl std::ops::DerefMut for StrBuf {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.s
    }
}

fn stringify(v: &Value, separator: Option<&str>) -> Result<String, String> {
    enum Work {
        Value(Value, Option<String>),
        Text(String),
        Leave(usize),
        ErrorName(usize, Value),
        ErrorMessage(usize, String),
    }
    let mut work = vec![Work::Value(v.clone(), separator.map(str::to_string))];
    let mut path = std::collections::HashSet::new();
    let mut out = StrBuf::default();
    while let Some(task) = work.pop() {
        out.check()?;
        let (v, separator) = match task {
            Work::Text(text) => {
                out.append(&text)?;
                continue;
            }
            Work::Leave(id) => {
                path.remove(&id);
                continue;
            }
            Work::ErrorName(start, message) => {
                let name = out.split_off(start);
                work.push(Work::ErrorMessage(start, name));
                work.push(Work::Value(message, None));
                continue;
            }
            Work::ErrorMessage(start, name) => {
                let message = out.split_off(start);
                out.push_str(&name);
                if !name.is_empty() && !message.is_empty() {
                    out.push_str(": ")
                }
                out.push_str(&message);
                continue;
            }
            Work::Value(v, sep) => (v, sep),
        };
        match &v {
            Value::List(list) => {
                let id = Rc::as_ptr(list) as usize;
                if !path.insert(id) {
                    continue;
                }
                work.push(Work::Leave(id));
                let items = list.read();
                let sep = separator.as_deref().unwrap_or(",");
                for (i, item) in items.iter().enumerate().rev() {
                    if !matches!(item, Value::None | Value::Undef) {
                        work.push(Work::Value(item.clone(), None));
                    }
                    if i > 0 {
                        work.push(Work::Text(sep.to_string()));
                    }
                }
            }
            Value::Map(m) if m.class().is_some_and(|c| c.error_kind().is_some()) => {
                let id = Rc::as_ptr(m) as usize;
                if !path.insert(id) {
                    return Err("RangeError: recursive error string conversion".into());
                }
                work.push(Work::Leave(id));
                let kind = m.class().unwrap().error_kind().unwrap();
                let name = m
                    .read()
                    .get("name")
                    .cloned()
                    .unwrap_or_else(|| Value::Str((&kind).into()));
                let message = m
                    .read()
                    .get("message")
                    .cloned()
                    .or_else(|| m.ext().unwrap().hidden.borrow().get("message").cloned())
                    .unwrap_or_else(|| Value::str(""));
                let name = if matches!(name, Value::Undef) {
                    Value::str("Error")
                } else {
                    name
                };
                let message = if matches!(message, Value::Undef) {
                    Value::str("")
                } else {
                    message
                };
                work.push(Work::ErrorName(out.len(), message));
                work.push(Work::Value(name, None));
            }
            Value::Str(s) => out.append(s)?,
            Value::Int(i) => out.push_str(&i.to_string()),
            Value::BigInt(s) => out.push_str(s),
            Value::Float(f) => out.push_str(&number_string(*f)),
            Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Value::None => out.push_str("null"),
            Value::Undef => out.push_str("undefined"),
            Value::Map(m) => out.push_str(&error_text(&m.read()).unwrap_or_else(|| "[object Object]".into())),
            Value::Class(c) => out.push_str(&format!("class {} {{ }}", c.name)),
            Value::Token(_, s) => out.append(s)?,
            Value::Regex(r) => out.push_str(&format!("/{}/{}", r.source, r.flags)),
            Value::Coll(c) => out.push_str(if c.read().set { "[object Set]" } else { "[object Map]" }),
            other => out.push_str(&other.to_py_str()),
        }
    }
    out.check()?;
    Ok(out.into_string())
}

/// ECMAScript StringToNumber: decimal literals (with `Infinity`), `0x`/`0o`/`0b` integers;
/// surrounding white space ignored; anything else is NaN.
pub fn str_to_number(s: &str) -> f64 {
    let t = s.trim_matches(is_js_space);
    if t.is_empty() {
        return 0.0;
    }
    let b = t.as_bytes();
    if b.len() > 2 && b[0] == b'0' {
        let radix = match b[1] {
            b'x' | b'X' => 16,
            b'o' | b'O' => 8,
            b'b' | b'B' => 2,
            _ => 0,
        };
        if radix != 0 {
            let mut f = 0f64;
            for c in t[2..].chars() {
                match c.to_digit(radix) {
                    Some(d) => f = f * radix as f64 + d as f64,
                    None => return f64::NAN,
                }
            }
            return f;
        }
    }
    let (sign, body) = match b[0] {
        b'+' => (1.0, &t[1..]),
        b'-' => (-1.0, &t[1..]),
        _ => (1.0, t),
    };
    if body == "Infinity" {
        return sign * f64::INFINITY;
    }
    if decimal_prefix(body) != body.len() || body.is_empty() {
        return f64::NAN;
    }
    sign * body.parse::<f64>().unwrap_or(f64::NAN)
}

/// The length of the longest prefix of `s` that is an unsigned decimal literal (`12`, `1.5`,
/// `.5`, `5.`, `1e3`), or 0.
pub fn decimal_prefix(s: &str) -> usize {
    let b = s.as_bytes();
    let digits = |mut i: usize| {
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        i
    };
    let int_end = digits(0);
    let mut end = int_end;
    let mut frac_digits = 0;
    if end < b.len() && b[end] == b'.' {
        let f = digits(end + 1);
        frac_digits = f - end - 1;
        if int_end > 0 || frac_digits > 0 {
            end = f;
        }
    }
    if int_end == 0 && frac_digits == 0 {
        return 0;
    }
    if end < b.len() && (b[end] == b'e' || b[end] == b'E') {
        let mut j = end + 1;
        if j < b.len() && (b[j] == b'+' || b[j] == b'-') {
            j += 1;
        }
        let k = digits(j);
        if k > j {
            end = k;
        }
    }
    end
}

/// ECMAScript ToNumber (as f64).
pub fn to_number(v: &Value) -> f64 {
    match v {
        Value::Int(i) => *i as f64,
        Value::BigInt(s) => s.parse().unwrap_or(f64::NAN),
        Value::Float(f) => *f,
        Value::Bool(b) => *b as i64 as f64,
        Value::None => 0.0,
        Value::Undef => f64::NAN,
        Value::Str(s) => str_to_number(s),
        // An array converts through its string form (`[5]` is 5, `[]` is 0, `[1, 2]` is NaN).
        Value::List(_) => str_to_number(&to_string(v)),
        _ => f64::NAN,
    }
}

/// ECMAScript ToInt32 / ToUint32 (bitwise operators).
pub fn to_int32(v: &Value) -> i32 {
    if let Value::Int(i) = v {
        return *i as i32;
    }
    to_uint32(v) as i32
}

pub fn to_uint32(v: &Value) -> u32 {
    if let Value::Int(i) = v {
        return *i as u32;
    }
    let f = to_number(v);
    if !f.is_finite() {
        return 0;
    }
    f.trunc().rem_euclid(4294967296.0) as u32
}

/// A number result: an integer when it is one (so `6 / 3` prints `2`), else a float (`-0` too).
pub fn number(f: f64) -> Value {
    if f == f.trunc() && f.abs() < 9.0e15 && !(f == 0.0 && f.is_sign_negative()) {
        Value::Int(f as i64)
    } else {
        Value::Float(f)
    }
}

/// The largest integer a double holds exactly, with all below it.
const SAFE: i64 = 1 << 53;

fn is_numeric(v: &Value) -> bool {
    matches!(v, Value::Int(_) | Value::Float(_) | Value::BigInt(_))
}

/// `a + b`: string concatenation if either side is (or converts to) a string, else numeric.
pub fn add(a: &Value, b: &Value) -> Result<Value, String> {
    if let (Value::Int(x), Value::Int(y)) = (a, b)
        && let Some(z) = x.checked_add(*y).filter(|z| z.unsigned_abs() <= SAFE as u64)
    {
        return Ok(Value::Int(z));
    }
    let stringy = |v: &Value| matches!(v, Value::Str(_) | Value::List(_) | Value::Map(_) | Value::Regex(_));
    if stringy(a) || stringy(b) {
        return concat(a, b);
    }
    Ok(number(to_number(a) + to_number(b)))
}

#[inline(never)]
fn concat(a: &Value, b: &Value) -> Result<Value, String> {
    let mut s = StrBuf::from(try_to_string(a)?);
    s.append(&try_to_string(b)?)?;
    Ok(Value::string(s.into_string()))
}

/// `x ** y` (and `Math.pow`).
pub fn pow(x: f64, y: f64) -> f64 {
    if y.is_nan() {
        return f64::NAN;
    }
    if y == 0.0 {
        return 1.0;
    }
    if (x == 1.0 || x == -1.0) && y.is_infinite() {
        return f64::NAN;
    }
    crate::jsmath::pow(x, y)
}

pub fn arith(op: char, a: &Value, b: &Value) -> Value {
    if let (Value::Int(x), Value::Int(y)) = (a, b) {
        let r = match op {
            '-' => x.checked_sub(*y),
            // `0 * -1` is -0: the float path.
            '*' => x.checked_mul(*y).filter(|z| *z != 0 || (*x < 0) == (*y < 0)),
            // `-4 % 2` is -0, and `MIN % -1` overflows: the float path.
            '%' if *y != 0 => x.checked_rem(*y).filter(|z| *z != 0 || *x >= 0),
            '^' if *y >= 0 && *y < 64 => x.checked_pow(*y as u32),
            _ => None,
        };
        if let Some(r) = r.filter(|z| z.unsigned_abs() <= SAFE as u64) {
            return Value::Int(r);
        }
    }
    let (x, y) = (to_number(a), to_number(b));
    let f = match op {
        '-' => x - y,
        '*' => x * y,
        '/' => x / y,
        '%' => x % y,
        '^' => pow(x, y),
        _ => f64::NAN,
    };
    number(f)
}

/// `===`: same type and value; arrays, objects and fragments by identity.
pub fn strict_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x == y,
        _ if is_numeric(a) && is_numeric(b) => to_number(a) == to_number(b),
        (Value::Str(x), Value::Str(y)) => **x == **y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::None, Value::None) | (Value::Undef, Value::Undef) => true,
        (Value::List(x), Value::List(y)) => Rc::ptr_eq(x, y),
        (Value::Map(x), Value::Map(y)) => Rc::ptr_eq(x, y),
        (Value::Coll(x), Value::Coll(y)) => Rc::ptr_eq(x, y),
        (Value::Frag(x), Value::Frag(y)) => Rc::ptr_eq(x, y),
        (Value::Token(x, _), Value::Token(y, _)) => x == y,
        (Value::Promise(x), Value::Promise(y)) => Rc::ptr_eq(x, y),
        (Value::Regex(x), Value::Regex(y)) => Rc::ptr_eq(x, y),
        (Value::Lambda(x, ex), Value::Lambda(y, ey)) => Rc::ptr_eq(x, y) && Rc::ptr_eq(ex, ey),
        (Value::Func(x), Value::Func(y)) => Rc::ptr_eq(&x.definition, &y.definition),
        (Value::Bound(x), Value::Bound(y)) => Rc::ptr_eq(x, y),
        (Value::Class(x), Value::Class(y)) => Rc::ptr_eq(x, y),
        (Value::Native(x), Value::Native(y)) => x == y,
        (Value::Setter(x), Value::Setter(y)) => x == y,
        (Value::Method(x), Value::Method(y)) => x.home.brand() == y.home.brand() && strict_eq(&x.function, &y.function),
        _ => false,
    }
}

/// An object (not a primitive) for `==` and `<`: converted to its string form first.
fn is_object(v: &Value) -> bool {
    matches!(v, Value::List(_) | Value::Map(_) | Value::Regex(_) | Value::Coll(_))
}

/// ECMAScript ToPrimitive of an array, object or regex: its string form.
fn to_primitive(v: &Value) -> Value {
    if is_object(v) {
        Value::string(to_string(v))
    } else {
        v.clone()
    }
}

/// `==`: `null == undefined`; an object against a primitive compares its string form; numbers,
/// strings and booleans compare as numbers (two strings as strings); else `===`.
pub fn loose_eq(a: &Value, b: &Value) -> bool {
    let nullish = |v: &Value| matches!(v, Value::None | Value::Undef);
    if nullish(a) || nullish(b) {
        return nullish(a) && nullish(b);
    }
    match (is_object(a), is_object(b)) {
        (true, true) => return strict_eq(a, b),
        (true, false) => return loose_eq(&to_primitive(a), b),
        (false, true) => return loose_eq(a, &to_primitive(b)),
        _ => {}
    }
    let prim = |v: &Value| is_numeric(v) || matches!(v, Value::Str(_) | Value::Bool(_));
    if prim(a) && prim(b) && !(matches!(a, Value::Str(_)) && matches!(b, Value::Str(_))) {
        return to_number(a) == to_number(b);
    }
    strict_eq(a, b)
}

/// Strings compare by UTF-16 code units, as JavaScript does.
pub fn cmp_strings(x: &str, y: &str) -> std::cmp::Ordering {
    if x.is_ascii() && y.is_ascii() {
        return x.cmp(y);
    }
    x.encode_utf16().cmp(y.encode_utf16())
}

/// `<`, `<=`, `>`, `>=`: strings by UTF-16 code units, everything else as numbers (NaN: false).
/// Arrays and objects compare by their string forms.
pub fn compare(op: &str, a: &Value, b: &Value) -> bool {
    if is_object(a) || is_object(b) {
        return compare(op, &to_primitive(a), &to_primitive(b));
    }
    if let (Value::Str(x), Value::Str(y)) = (a, b) {
        let o = cmp_strings(x, y);
        return match op {
            "<" => o.is_lt(),
            "<=" => o.is_le(),
            ">" => o.is_gt(),
            _ => o.is_ge(),
        };
    }
    if let (Value::Int(x), Value::Int(y)) = (a, b) {
        return match op {
            "<" => x < y,
            "<=" => x <= y,
            ">" => x > y,
            _ => x >= y,
        };
    }
    let (x, y) = (to_number(a), to_number(b));
    match op {
        "<" => x < y,
        "<=" => x <= y,
        ">" => x > y,
        _ => x >= y,
    }
}

pub fn type_of(v: &Value) -> &'static str {
    match v {
        Value::Undef => "undefined",
        Value::Bool(_) => "boolean",
        Value::Int(_) | Value::BigInt(_) | Value::Float(_) => "number",
        Value::Str(_) => "string",
        Value::Native(n) if n.is_namespace() => "object",
        Value::Method(_)
        | Value::Native(_)
        | Value::Lambda(..)
        | Value::Func(_)
        | Value::Setter(_)
        | Value::Bound(_)
        | Value::Class(_) => "function",
        _ => "object",
    }
}

/// A canonical array index (`"0"`, `"12"`; not `"01"`, `"-1"` or `"1.5"`).
pub fn array_index(k: &str) -> Option<u32> {
    let b = k.as_bytes();
    if b.is_empty()
        || b.len() > 10
        || !b[0].is_ascii_digit()
        || (b[0] == b'0' && b.len() > 1)
        || !b.iter().all(u8::is_ascii_digit)
    {
        return None;
    }
    k.parse::<u64>().ok().filter(|n| *n < u32::MAX as u64).map(|n| n as u32)
}

/// The array index a property key names (`1`, `1.0`, `"1"`), if any.
pub fn array_index_of(k: &Value) -> Option<usize> {
    match k {
        Value::Int(n) if *n >= 0 && *n < u32::MAX as i64 => Some(*n as usize),
        Value::Float(f) if *f == f.trunc() && *f >= 0.0 && *f < u32::MAX as f64 => Some(*f as usize),
        Value::Str(s) => array_index(s).map(|n| n as usize),
        _ => None,
    }
}

/// Set an object's property, in JavaScript's key order: integer-like keys first, ascending,
/// then the others in insertion order.
pub fn obj_insert(m: &mut Map, k: Rc<str>, v: Value) {
    // (`insert` keeps an existing key where it is, as JavaScript does.)
    let Some(i) = array_index(&k) else {
        m.insert(k, v);
        return;
    };
    if let Some(slot) = m.get_mut(&*k) {
        *slot = v;
        return;
    }
    // In a JS-made object the integer keys are a sorted prefix: a key past the last one appends
    // (keys in order, as JSON and loops write them), any other is placed by binary search. (A map
    // from host JSON keeps the host's order, which JS never makes; its position is the search's.)
    if m.last().is_none_or(|(x, _)| array_index(x).is_some_and(|j| j < i)) {
        m.insert(k, v);
        return;
    }
    let pos = m.partition_point(|x, _| array_index(x).is_some_and(|j| j < i));
    m.shift_insert(pos, k, v);
}

/// `a[i]`: array and string indexes (`1`, `1.0`, `"1"`), `length`; anything indexes objects by
/// its string form; indexing `null`/`undefined` is a TypeError.
pub fn index(v: &Value, i: &Value) -> Result<Value, String> {
    if let (Value::List(l), Value::Int(n)) = (v, i) {
        return Ok(if *n >= 0 {
            l.read().get(*n as usize).cloned().unwrap_or(Value::Undef)
        } else {
            Value::Undef
        });
    }
    let key = || to_string(i);
    let is_length = || matches!(i, Value::Str(s) if &**s == "length");
    Ok(match v {
        Value::Undef | Value::None => return Err(format!("TypeError: Cannot read properties of {} (reading '{}')", to_string(v), key())),
        // A token run (a model turn's `ids`) indexes like its list of ids.
        Value::Frag(_) if crate::interp::token_run(v).is_some() => {
            let ids = crate::interp::token_run(v).unwrap();
            match array_index_of(i) {
                Some(n) => ids.get(n).map_or(Value::Undef, |&t| Value::Int(t as i64)),
                None if is_length() => Value::Int(ids.len() as i64),
                None => Value::Undef,
            }
        }
        // Usually a fragment reached a slot that expects text or a list of content parts.
        Value::Frag(_) => return Err("TypeError: a fragment cannot be indexed: this slot expects a string or a list, got a fragment (did you use p`...`? use a plain `...` string, or a template that accepts fragments)".into()),
        Value::List(l) => match array_index_of(i) {
            Some(n) => l.read().get(n).cloned().unwrap_or(Value::Undef),
            None if is_length() => Value::Int(l.read().len() as i64),
            None => Value::Undef,
        },
        Value::Str(s) => match array_index_of(i) {
            Some(n) => s.chars().nth(n).map(|c| Value::string(c.to_string())).unwrap_or(Value::Undef),
            None if is_length() => Value::Int(s.char_count() as i64),
            None => Value::Undef,
        },
        Value::Map(m) => match i {
            Value::Str(k) => m.read().get(&**k).cloned().unwrap_or(Value::Undef),
            _ => m.read().get(key().as_str()).cloned().unwrap_or(Value::Undef),
        },
        // A Map's and a Set's only data property is `size` (their entries are not properties).
        Value::Coll(c) => if key() == "size" { Value::Int(c.read().len() as i64) } else { Value::Undef },
        _ => crate::interp::get_attr(v, &key())?,
    })
}

/// `for (const k in v)`: object keys, or array/string indexes (as strings, as in JS).
pub fn keys(v: &Value) -> Vec<Value> {
    match v {
        Value::Map(m) => m.read().keys().map(|k| Value::Str(k.into())).collect(),
        Value::List(l) => (0..l.read().len()).map(|i| Value::string(i.to_string())).collect(),
        Value::Str(s) => (0..s.chars().count()).map(|i| Value::string(i.to_string())).collect(),
        _ => vec![],
    }
}

/// `key in obj`: an own property (object key; array index or `length`).
pub fn has(obj: &Value, key: &Value) -> Result<bool, String> {
    Ok(match obj {
        // An instance also has what its class's prototype has (methods and accessors).
        Value::Map(m) if m.class().is_some() => {
            let k = to_string(key);
            let c = m.class().unwrap();
            m.read().contains_key(k.as_str())
                || m.ext().unwrap().hidden.borrow().contains_key(k.as_str())
                || c.find(&k, |c| &c.methods).is_some()
                || c.find(&k, |c| &c.getters).is_some()
                || c.find(&k, |c| &c.setters).is_some()
                || (k == "name" && c.error_kind().is_some())
                || crate::intrinsic::OBJECT.contains(&k.as_str())
        }
        Value::Map(m) => {
            let k = to_string(key);
            m.read().contains_key(k.as_str()) || inherits(obj, &k)
        }
        Value::List(l) => match array_index_of(key) {
            Some(i) => i < l.read().len(),
            None => matches!(key, Value::Str(s) if &**s == "length") || inherits(obj, &to_string(key)),
        },
        Value::Class(class) => {
            let name = to_string(key);
            let mut current = Some(class.clone());
            let mut found = false;
            while let Some(c) = current {
                if c.statics.read().contains_key(name.as_str())
                    || c.static_getters.contains_key(name.as_str())
                    || c.static_setters.contains_key(name.as_str())
                {
                    found = true;
                    break;
                }
                current = c.parent.clone();
            }
            found || inherits(obj, &name)
        }
        Value::Native(native) => {
            let name = to_string(key);
            native.owner.is_none()
                && (crate::jsbuiltins::is_global_function(native, &name)
                    || crate::jsbuiltins::global_constant(native, &name).is_some())
                || inherits(obj, &name)
        }
        Value::Method(_) | Value::Func(_) | Value::Lambda(..) | Value::Bound(_) | Value::Setter(_) => {
            inherits(obj, &to_string(key))
        }
        Value::Coll(_) => inherits(obj, &to_string(key)),
        other => {
            return Err(format!(
                "TypeError: Cannot use 'in' operator to search for '{}' in {}",
                to_string(key),
                to_string(other)
            ));
        }
    })
}

/// `Error.prototype.toString` for an error value (`new Error(..)`, or what a `catch` binds): an
/// object with a string `name` and `message` and no other keys but `stack`, `value`, `cause`.
pub fn error_text(m: &crate::value::Map) -> Option<String> {
    let (Some(Value::Str(name)), Some(Value::Str(message))) = (m.get("name"), m.get("message")) else {
        return None;
    };
    if !m
        .keys()
        .all(|k| matches!(&**k, "name" | "message" | "stack" | "value" | "cause"))
    {
        return None;
    }
    Some(if message.is_empty() {
        name.to_string()
    } else {
        format!("{}: {}", &**name, &**message)
    })
}

/// `Object.is`: identity for objects and functions; `NaN` is itself; `+0` and `-0` differ.
pub fn same_value(a: &Value, b: &Value) -> bool {
    let num = |v: &Value| matches!(v, Value::Int(_) | Value::Float(_));
    if num(a) && num(b) {
        let (x, y) = (to_number(a), to_number(b));
        return (x.is_nan() && y.is_nan()) || (x == y && x.is_sign_negative() == y.is_sign_negative());
    }
    strict_eq(a, b)
}

/// React's rule for an effect's deps: no deps array runs it after every render; otherwise it
/// runs when the length or any item differs by `Object.is`.
pub fn deps_changed(old: &Value, new: &Value) -> bool {
    match (old, new) {
        (Value::List(a), Value::List(b)) => {
            let (a, b) = (a.read(), b.read());
            a.len() != b.len() || a.iter().zip(b.iter()).any(|(x, y)| !same_value(x, y))
        }
        (Value::Undef, _) | (_, Value::Undef) => true,
        (a, b) => !same_value(a, b),
    }
}

/// `Object.prototype`'s members: every object has them (they are inherited, not own).
pub fn inherits(value: &Value, name: &str) -> bool {
    crate::intrinsic::member(value, name).is_some()
}

pub fn same_value_zero(a: &Value, b: &Value) -> bool {
    strict_eq(a, b) || matches!((a,b),(Value::Float(x),Value::Float(y)) if x.is_nan()&&y.is_nan())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Value;

    /// A JSON integer beyond 2^53 is a number (as in JavaScript) that keeps its exact digits when
    /// printed, so a view reproduces the reference text.
    #[test]
    fn large_json_integers_keep_their_digits() {
        let a = Value::from_json_exact(&crate::value::parse_json_exact("123456789012345678901234567890").unwrap());
        assert_eq!(to_string(&a), "123456789012345678901234567890");
        assert_eq!(type_of(&a), "number");
        assert!(strict_eq(&a, &Value::Float(123456789012345678901234567890.0)));
    }
}
