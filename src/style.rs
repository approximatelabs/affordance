//! Token style sheets (`.tss`): CSS-like rules whose units are model tokens.
//!
//! ```css
//! /* specificity: #label > [prop] > Component; later rules win ties; inline style wins all */
//! NumberList            { max-tokens: 64tok; more: "\n[... more below]"; }
//! Turn[role="tool"]     { max-tokens: 2048tok; overflow: clip-start; }
//! #history              { max-tokens: 24000tok; overflow: clip-start; }
//! Reasoning[old]        { display: none; }
//! ```
//! Components and native elements share computed style. Token budgets introduce a hard
//! encoding boundary; nested boxes apply child limits before the parent's overflow policy.
//! `display: none` suppresses presentation while component lifecycle remains mounted.

use crate::value::Value;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct Selector {
    pub name: Option<String>,
    pub id: Option<String>,
    /// [prop] (None = present and truthy) or [prop="v"]
    pub attrs: Vec<(String, Option<String>)>,
    /// Descendant combinators: compounds an enclosing element must match, outermost first
    /// (`Context[policy="trunc"] history`: a history inside such a Context).
    pub ancestors: Vec<Selector>,
    /// `:nth-last(n+K / S)` (on `result` only): a tool result at least K from the end among the
    /// results of its history, the boundary moving S at a time.
    /// Resolved at layout, where the position is known.
    pub nth_last: Option<(usize, usize)>,
}

/// The elements around the one being styled, outermost first: (name, props).
pub type Ancestry = [(Rc<str>, Rc<Vec<(Rc<str>, Value)>>)];

#[derive(Debug, Clone)]
pub struct Rule {
    pub sel: Selector,
    pub decls: Vec<Declaration>,
    pub order: usize,
}

impl Selector {
    fn specificity(&self) -> (usize, usize, usize) {
        self.ancestors.iter().fold(
            (
                self.id.is_some() as usize,
                self.attrs.len() + self.nth_last.is_some() as usize,
                self.name.is_some() as usize,
            ),
            |(a, b, c), s| {
                let (x, y, z) = s.specificity();
                (a + x, b + y, c + z)
            },
        )
    }

    /// The element matches, and each ancestor compound matches an enclosing element, in order
    /// (each further out than the one after it).
    pub fn matches_in(&self, name: &str, props: &[(Rc<str>, Value)], ancestry: &Ancestry) -> bool {
        if !self.matches(name, props) {
            return false;
        }
        let mut outer = ancestry.len();
        for compound in self.ancestors.iter().rev() {
            match ancestry[..outer].iter().rposition(|(n, p)| compound.matches(n, p)) {
                Some(at) => outer = at,
                None => return false,
            }
        }
        true
    }

    /// `props` are the evaluated props of the component call; `label` comes from `label`/`id`.
    pub fn matches(&self, name: &str, props: &[(Rc<str>, Value)]) -> bool {
        if let Some(n) = &self.name
            && n != name
        {
            return false;
        }
        let get = |k: &str| props.iter().find(|(n, _)| &**n == k).map(|(_, v)| v);
        if let Some(id) = &self.id {
            let lab = get("id").or_else(|| get("label")).map(crate::jsvalue::to_string);
            if lab.as_deref() != Some(id.as_str()) {
                return false;
            }
        }
        for (k, want) in &self.attrs {
            match (get(k), want) {
                (Some(v), Some(w)) if crate::jsvalue::to_string(v) == *w => {}
                (Some(v), None) if crate::jsvalue::truthy(v) => {}
                _ => return false,
            }
        }
        true
    }
}

/// Parse a `.tss` file. Values: `64tok` / `64` (int), `"text"` (string), bare words (string).
pub fn parse(src: &str, file: &str) -> Result<Vec<Rule>, String> {
    let s = without_comments(src).map_err(|e| format!("{file}: {e}"))?;
    let mut rules = vec![];
    let mut pos = 0;
    let b = s.as_bytes();
    while pos < b.len() {
        let open = match delimiter(&s[pos..], '{') {
            Some(i) => pos + i,
            None => {
                if s[pos..].trim().is_empty() {
                    break;
                }
                return Err(format!("{}: expected `selector {{ ... }}`", file));
            }
        };
        let close = open + delimiter(&s[open..], '}').ok_or(format!("{}: unclosed rule", file))?;
        for sel in split_outside(&s[pos..open], ',') {
            let sel = parse_descendants(sel.trim()).map_err(|e| format!("{}: {}", file, e))?;
            let decls = parse_decls(&s[open + 1..close]).map_err(|e| format!("{}: {}", file, e))?;
            rules.push(Rule { sel, decls, order: 0 });
        }
        pos = close + 1;
    }
    Ok(rules)
}

/// `A B C`: C inside B inside A (whitespace outside brackets separates compounds).
fn parse_descendants(t: &str) -> Result<Selector, String> {
    let mut parts = vec![];
    let (mut depth, mut start) = (0, 0);
    let mut quote = None;
    for (i, ch) in t.char_indices() {
        match (quote, ch) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(ch),
            (None, '[' | '(') => depth += 1,
            (None, ']' | ')') => depth -= 1,
            (None, c) if c.is_whitespace() && depth == 0 => {
                if i > start {
                    parts.push(&t[start..i]);
                }
                start = i + c.len_utf8();
            }
            _ => {}
        }
    }
    if start < t.len() {
        parts.push(&t[start..]);
    }
    let mut compounds = parts.into_iter().map(parse_selector).collect::<Result<Vec<_>, _>>()?;
    let mut sel = compounds.pop().ok_or("empty selector")?;
    sel.ancestors = compounds;
    Ok(sel)
}

fn parse_selector(t: &str) -> Result<Selector, String> {
    let mut sel = Selector {
        name: None,
        id: None,
        attrs: vec![],
        ancestors: vec![],
        nth_last: None,
    };
    if t == "*" {
        return Ok(sel);
    }
    if t.is_empty() {
        return Err("empty selector".into());
    }
    let mut i = 0;
    let c: Vec<char> = t.chars().collect();
    let word = |i: &mut usize| {
        let st = *i;
        while *i < c.len() && (c[*i].is_alphanumeric() || c[*i] == '_' || c[*i] == '-' || c[*i] == '.') {
            *i += 1;
        }
        c[st..*i].iter().collect::<String>()
    };
    if i < c.len() && c[i].is_alphabetic() {
        sel.name = Some(word(&mut i));
    }
    while i < c.len() {
        match c[i] {
            '#' => {
                i += 1;
                let id = word(&mut i);
                if id.is_empty() {
                    return Err(format!("bad selector `{t}`"));
                }
                sel.id = Some(id);
            }
            ':' => {
                // `:nth-last(n+K / S)`
                let rest: String = c[i..].iter().collect();
                let close = rest.find(')').ok_or_else(|| format!("bad pseudo-class in `{t}`"))?;
                let inner = rest
                    .strip_prefix(":nth-last(")
                    .map(|r| &r[..close - ":nth-last(".len()])
                    .ok_or_else(|| {
                        format!("unsupported pseudo-class in `{t}` (supported: result:nth-last(n+K / S))")
                    })?;
                let (k, s) = inner
                    .split_once('/')
                    .ok_or_else(|| format!("`{t}`: :nth-last needs a step: nth-last(n+K / S)"))?;
                let k = k
                    .trim()
                    .strip_prefix("n+")
                    .and_then(|k| k.trim().parse::<usize>().ok())
                    .filter(|k| *k >= 1)
                    .ok_or_else(|| format!("`{t}`: :nth-last(n+K / S) with K >= 1"))?;
                let s = s
                    .trim()
                    .parse::<usize>()
                    .ok()
                    .filter(|s| *s >= 1)
                    .ok_or_else(|| format!("`{t}`: :nth-last(n+K / S) with S >= 1"))?;
                if sel.name.as_deref() != Some("result") {
                    return Err(format!("`{t}`: :nth-last applies to result only"));
                }
                sel.nth_last = Some((k, s));
                i += close + 1;
            }
            '[' => {
                i += 1;
                let k = word(&mut i);
                if k.is_empty() {
                    return Err(format!("bad attribute selector in `{t}`"));
                }
                let v = if i < c.len() && c[i] == '=' {
                    i += 1;
                    let Some(&q) = c.get(i).filter(|q| matches!(q, '\'' | '"')) else {
                        return Err(format!("expected quoted attribute value in `{t}`"));
                    };
                    i += 1;
                    let st = i;
                    while i < c.len() && c[i] != q {
                        i += 1;
                    }
                    if i == c.len() {
                        return Err(format!("unclosed attribute value in `{t}`"));
                    }
                    let v: String = c[st..i].iter().collect();
                    i += 1;
                    Some(v)
                } else {
                    None
                };
                if i >= c.len() || c[i] != ']' {
                    return Err(format!("bad attribute selector in `{}`", t));
                }
                i += 1;
                sel.attrs.push((k, v));
            }
            _ => return Err(format!("bad selector `{}`", t)),
        }
    }
    Ok(sel)
}

fn parse_decls(t: &str) -> Result<Vec<Declaration>, String> {
    split_outside(t, ';')
        .into_iter()
        .filter(|d| !d.trim().is_empty())
        .map(|d| {
            let (k, v) = d.split_once(':').ok_or_else(|| format!("bad declaration `{d}`"))?;
            let v = v.trim();
            let value = if let Some(n) = v.strip_suffix("tok") {
                Value::Int(n.trim().parse().map_err(|_| format!("bad token count `{v}`"))?)
            } else if let Ok(n) = v.parse::<i64>() {
                Value::Int(n)
            } else if v.starts_with('"') {
                Value::string(serde_json::from_str(v).map_err(|_| format!("bad string `{v}`"))?)
            } else {
                Value::str(v)
            };
            Declaration::parse(k.trim(), &value)
        })
        .collect()
}

/// Delimiters inside quoted values are data, including braces, commas and semicolons.
fn delimiter(text: &str, want: char) -> Option<usize> {
    let mut quote = None;
    let mut escape = false;
    for (i, c) in text.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        if let Some(q) = quote {
            if c == '\\' {
                escape = true
            } else if c == q {
                quote = None
            }
        } else if c == '"' || c == '\'' {
            quote = Some(c)
        } else if c == want {
            return Some(i);
        }
    }
    None
}
fn split_outside(mut text: &str, separator: char) -> Vec<&str> {
    let mut pieces = vec![];
    while let Some(i) = delimiter(text, separator) {
        pieces.push(&text[..i]);
        text = &text[i + separator.len_utf8()..];
    }
    pieces.push(text);
    pieces
}
fn without_comments(text: &str) -> Result<String, String> {
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    let mut quote = None;
    let mut escape = false;
    while let Some(c) = chars.next() {
        if escape {
            out.push(c);
            escape = false;
            continue;
        }
        if let Some(q) = quote {
            out.push(c);
            if c == '\\' {
                escape = true
            } else if c == q {
                quote = None
            }
        } else if c == '"' || c == '\'' {
            quote = Some(c);
            out.push(c)
        } else if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            let mut closed = false;
            while let Some(c) = chars.next() {
                if c == '*' && chars.peek() == Some(&'/') {
                    chars.next();
                    closed = true;
                    break;
                }
            }
            if !closed {
                return Err("unclosed style comment".into());
            }
            out.push(' ');
        } else {
            out.push(c)
        }
    }
    if quote.is_some() {
        return Err("unclosed style string".into());
    }
    Ok(out)
}

/// The finite token-style vocabulary, shared by sheets, native attributes and inline style.
#[derive(Debug, Clone)]
pub enum Declaration {
    MaxTokens(crate::emit::Size),
    MinTokens(usize),
    ObjectFit(crate::media::ObjectFit),
    DropStep(crate::emit::Step),
    TokenSize(usize),
    Overflow(Overflow),
    More(Rc<str>),
    Pad(Rc<str>),
    Hidden(bool),
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Overflow {
    #[default]
    ClipEnd,
    ClipStart,
    ClipMiddle,
    Elide,
    DropStart,
    DropEnd,
    DropChildren,
    Scroll,
}
#[derive(Debug, Clone, Default)]
pub struct Computed {
    pub max_tokens: Option<crate::emit::Size>,
    /// The fewest tokens a picture takes (it is enlarged to reach them).
    pub min_tokens: Option<usize>,
    /// How a picture meets its `max-tokens`.
    pub object_fit: crate::media::ObjectFit,
    pub drop_step: Option<crate::emit::Step>,
    pub tok_size: Option<usize>,
    pub overflow: Overflow,
    pub more: Option<Rc<str>>,
    pub pad: Option<Rc<str>>,
    pub hidden: bool,
}
impl Declaration {
    fn parse(name: &str, value: &Value) -> Result<Self, String> {
        let mut key = String::new();
        for c in name.chars() {
            if c.is_ascii_uppercase() {
                key.push('-');
                key.push(c.to_ascii_lowercase());
            } else {
                key.push(if c == '_' { '-' } else { c });
            }
        }
        let bad = || format!("invalid token style `{name}`: {}", crate::jsvalue::to_string(value));
        let number = || match value {
            Value::Int(n) if *n >= 0 => usize::try_from(*n).map_err(|_| bad()),
            Value::Float(n) if n.is_finite() && n.fract() == 0.0 && *n >= 0.0 && *n < usize::MAX as f64 => {
                Ok(*n as usize)
            }
            _ => Err(bad()),
        };
        let text = || value.as_str().map(Rc::from).ok_or_else(bad);
        // `80win`, `25%`: a share, in hundredths of a percent.
        let share = |suffix: &str| -> Option<Result<u32, String>> {
            let n = value.as_str()?.trim().strip_suffix(suffix)?.trim().parse::<f64>().ok();
            Some(
                n.filter(|n| n.is_finite() && *n > 0.0 && *n <= 100.0)
                    .map(|n| (n * 100.0).round() as u32)
                    .ok_or_else(bad),
            )
        };
        let tokens = || match value.as_str().and_then(|s| s.trim().strip_suffix("tok")) {
            Some(n) => n.trim().parse::<usize>().map_err(|_| bad()),
            None => number(),
        };
        Ok(match key.as_str() {
            "max-tokens" => Self::MaxTokens(match share("win") {
                Some(p) => crate::emit::Size::Window(p?),
                None => crate::emit::Size::Tokens(tokens()?),
            }),
            "min-tokens" => Self::MinTokens(tokens()?),
            "object-fit" => Self::ObjectFit(match value.as_str() {
                Some("scale-down") => crate::media::ObjectFit::ScaleDown,
                Some("contain") => crate::media::ObjectFit::Contain,
                Some("none") => crate::media::ObjectFit::None,
                _ => return Err(bad()),
            }),
            "drop-step" => Self::DropStep(match share("%") {
                Some(p) => crate::emit::Step::Percent(p?),
                None => crate::emit::Step::Tokens(tokens()?.max(1)),
            }),
            "tok-size" => Self::TokenSize(number()?),
            "more" => Self::More(text()?),
            "pad" => Self::Pad(text()?),
            "display" => Self::Hidden(match value.as_str() {
                Some("none") => true,
                Some("block" | "contents") => false,
                _ => return Err(bad()),
            }),
            "overflow" => Self::Overflow(match value.as_str() {
                Some("clip-end") => Overflow::ClipEnd,
                Some("clip-start") => Overflow::ClipStart,
                Some("clip-middle") => Overflow::ClipMiddle,
                Some("elide") => Overflow::Elide,
                Some("drop-start") => Overflow::DropStart,
                Some("drop-end") => Overflow::DropEnd,
                Some("drop-children") => Overflow::DropChildren,
                Some("scroll") => Overflow::Scroll,
                _ => return Err(bad()),
            }),
            _ => return Err(format!("unsupported token style `{name}`")),
        })
    }
}
impl Computed {
    fn apply(&mut self, declaration: &Declaration) {
        match declaration {
            Declaration::MaxTokens(n) => self.max_tokens = Some(*n),
            Declaration::MinTokens(n) => self.min_tokens = Some(*n),
            Declaration::ObjectFit(v) => self.object_fit = *v,
            Declaration::DropStep(s) => self.drop_step = Some(*s),
            Declaration::TokenSize(n) => self.tok_size = Some(*n),
            Declaration::Overflow(v) => self.overflow = *v,
            Declaration::More(s) => self.more = Some(s.clone()),
            Declaration::Pad(s) => self.pad = Some(s.clone()),
            Declaration::Hidden(v) => self.hidden = *v,
        }
    }
    pub fn boxed(&self) -> bool {
        self.max_tokens.is_some() || self.tok_size.is_some()
    }
    pub fn scrolls(&self) -> bool {
        self.boxed() && self.overflow == Overflow::Scroll
    }
}

/// Presentation attributes < sheets (specificity, then order) < inline style. No inheritance.
pub fn compute(
    rules: &[Rule],
    name: &str,
    props: &[(Rc<str>, Value)],
    native: bool,
    ancestry: &Ancestry,
) -> Result<Computed, String> {
    let mut style = Computed::default();
    if native {
        for (key, value) in props {
            if matches!(
                &**key,
                "max_tokens"
                    | "min_tokens"
                    | "object_fit"
                    | "tok_size"
                    | "overflow"
                    | "more"
                    | "pad"
                    | "display"
                    | "drop_step"
            ) && !matches!(value, Value::Undef | Value::None)
            {
                style.apply(&Declaration::parse(key, value)?);
            }
        }
    }
    let mut hit = rules
        .iter()
        .filter(|r| r.sel.nth_last.is_none() && r.sel.matches_in(name, props, ancestry))
        .collect::<Vec<_>>();
    hit.sort_by_key(|r| (r.sel.specificity(), r.order));
    for rule in hit {
        for declaration in &rule.decls {
            style.apply(declaration);
        }
    }
    if let Some((_, value)) = props.iter().find(|(key, _)| &**key == "style") {
        match value {
            Value::Undef | Value::None => {}
            Value::Map(map) => {
                for (key, value) in map.read().iter() {
                    if !matches!(value, Value::Undef | Value::None) {
                        style.apply(&Declaration::parse(key, value)?);
                    }
                }
            }
            _ => return Err("style must be an object".into()),
        }
    }
    Ok(style)
}

/// A result's styles: the static one, and, when a positional rule (`result:nth-last(n+K / S)`)
/// matches it apart from its position, the style it takes once its position matches, with K and
/// S. Layout chooses between them (the position is known only there).
pub fn compute_graded(
    rules: &[Rule],
    name: &str,
    props: &[(Rc<str>, Value)],
    ancestry: &Ancestry,
) -> Result<(Computed, Option<(Computed, usize, usize)>), String> {
    let base = compute(rules, name, props, false, ancestry)?;
    let mut hit = rules
        .iter()
        .filter(|r| r.sel.nth_last.is_some() && r.sel.matches_in(name, props, ancestry))
        .collect::<Vec<_>>();
    if hit.is_empty() {
        return Ok((base, None));
    }
    hit.sort_by_key(|r| (r.sel.specificity(), r.order));
    let (k, s) = hit.last().unwrap().sel.nth_last.unwrap();
    // The graded style is the full cascade with the positional rules in it.
    let mut all = rules
        .iter()
        .filter(|r| r.sel.matches_in(name, props, ancestry))
        .collect::<Vec<_>>();
    all.sort_by_key(|r| (r.sel.specificity(), r.order));
    let mut graded = Computed::default();
    for rule in all {
        for d in &rule.decls {
            graded.apply(d);
        }
    }
    if let Some((_, Value::Map(map))) = props.iter().find(|(key, _)| &**key == "style") {
        for (key, value) in map.read().iter() {
            if !matches!(value, Value::Undef | Value::None) {
                graded.apply(&Declaration::parse(key, value)?);
            }
        }
    }
    Ok((base, Some((graded, k, s))))
}
