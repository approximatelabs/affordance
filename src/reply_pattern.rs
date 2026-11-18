//! Reading tool calls in a format no parser was written for, from how the format itself writes
//! one.
//!
//! A model's chat format writes an assistant turn's tool calls into the prompt, and a model
//! trained on it writes its own calls the same way. So the format is asked to write calls whose
//! every piece is a string that occurs nowhere else (`views/template_apps/chat.tsx`,
//! `callProbe`): one call with one argument, one with two, one with a number, one with none,
//! and two calls. Where the pieces land is known, and what lies between them is the format's
//! own writing. `Pattern::derive` turns those texts into a pattern; `Pattern::read` matches a
//! reply against it.
//!
//! Two ways of writing arguments are recognized: as one JSON object (`{"city": "Oslo"}`), or as
//! key and value pairs in the format's own markup (`<parameter=city>Oslo</parameter>`,
//! `city="Oslo"`). Pair values are text, typed by the tool's schema as XML-style values are.
use crate::reply::Call;
use crate::reply_formats::Parts;
use serde_json::{Value as J, json};

/// The marks the probe's pieces begin with (private-use characters), each followed by a digit.
const NAME: char = '\u{E010}';
const KEY: char = '\u{E020}';
const VALUE: char = '\u{E030}';
const ID: char = '\u{E040}';
fn mark(kind: char, i: u8) -> String {
    format!("{kind}{i}")
}

#[derive(Debug, Clone, PartialEq)]
enum Piece {
    /// The format's own text. Matched ignoring differences in white space.
    Lit(String),
    Name,
    Id,
    /// The arguments as one JSON object.
    Json,
    /// The arguments as pairs: `key` `open` value, then `between` before the next pair.
    Pairs(Pairs),
}

#[derive(Debug, Clone, PartialEq, Default)]
struct Pairs {
    /// Between a key and its value.
    open: String,
    /// Between one value and the next key.
    between: String,
    /// Around a string value (empty: a string is written bare, like any value).
    quote: (String, String),
}

/// How a format writes an assistant turn's tool calls.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Pattern {
    head: String,
    call: Vec<Piece>,
    /// A call without arguments, where the format writes it differently.
    bare: Option<Vec<Piece>>,
    between: String,
    tail: String,
    /// The marks around thinking, if the format writes thinking.
    think: Option<(String, String)>,
    /// Where the tool's name is in a call id, for a format that writes ids and no names
    /// (Kimi: `functions.{name}:{index}`).
    id_name: Option<(String, String)>,
}

/// The call text with its marks renumbered from call 1's to call 2's.
fn second(text: &str) -> String {
    text.replace(&mark(NAME, 1), &mark(NAME, 2))
        .replace(&mark(KEY, 1), &mark(KEY, 2))
        .replace(&mark(VALUE, 1), &mark(VALUE, 2))
        .replace(&mark(ID, 0), &mark(ID, 1))
}

fn common_suffix(a: &str, b: &str) -> usize {
    let n: usize = a
        .chars()
        .rev()
        .zip(b.chars().rev())
        .take_while(|(x, y)| x == y)
        .map(|(x, _)| x.len_utf8())
        .sum();
    n
}

/// Split `text` at the marks into literal pieces and holes. `args` is the byte range of the
/// arguments (already located) and the piece that stands for them.
fn pieces(text: &str, args: Option<(std::ops::Range<usize>, Piece)>) -> Vec<Piece> {
    let mut out = vec![];
    let mut lit = String::new();
    let mut i = 0;
    while i < text.len() {
        if let Some((range, piece)) = &args
            && i == range.start
        {
            out.push(Piece::Lit(std::mem::take(&mut lit)));
            out.push(piece.clone());
            i = range.end;
            continue;
        }
        let c = text[i..].chars().next().unwrap();
        let hole = match c {
            NAME => Some(Piece::Name),
            ID => Some(Piece::Id),
            _ => None,
        };
        match hole {
            Some(piece) => {
                out.push(Piece::Lit(std::mem::take(&mut lit)));
                out.push(piece);
                // The mark and its digit.
                i += c.len_utf8() + 1;
            }
            None => {
                lit.push(c);
                i += c.len_utf8();
            }
        }
    }
    out.push(Piece::Lit(lit));
    out.retain(|p| !matches!(p, Piece::Lit(s) if s.is_empty()));
    out
}

impl Pattern {
    /// The pattern from the probe a template app measured (`callProbe`): `one` (one call, one
    /// string argument), `pair` (two arguments), `number` (a number argument), `none` (no
    /// arguments), `two` (two calls), each the text the format wrote or null; `think`, the marks
    /// around thinking; `id_name`, the text before and after a tool's name in a call id.
    pub fn derive(probe: &J) -> Result<Pattern, String> {
        // A format that writes JSON in ASCII writes a mark as its escape (`\\ue010`): read back.
        let unescaped: std::collections::HashMap<&str, String> = ["one", "pair", "number", "none", "two"]
            .into_iter()
            .filter_map(|k| Some((k, unescape_marks(probe[k].as_str()?))))
            .collect();
        // A past turn's thinking, written empty before its calls, is not part of how a call is
        // written: it is cut from every text, and says what the marks around thinking are.
        let thinking = unescaped.get("one").and_then(|one| empty_thinking(one));
        // Or the format opens thinking before the calls and ends it as it ends a turn (a channel
        // of its own): the opener is cut, and a call's own beginning ends the thinking.
        let opener = probe["think"][0]
            .as_str()
            .filter(|open| !open.trim().is_empty() && thinking.is_none())
            .filter(|open| unescaped.get("one").is_some_and(|one| one.starts_with(open)))
            .map_or(0, str::len);
        let text = |k: &str| {
            unescaped
                .get(k)
                .map(|s| &s[empty_thinking(s).map_or(opener.min(s.len()), |(length, _)| length)..])
                .filter(|s| !s.is_empty())
        };
        let one = text("one").ok_or("the format wrote no tool call")?;
        let (k1, v1) = (mark(KEY, 1), mark(VALUE, 1));
        let (at_key, at_value) = (
            one.find(&k1).ok_or("the format did not write an argument's name")?,
            one.find(&v1).ok_or("the format did not write an argument's value")?,
        );
        // The arguments as one JSON object: the braces around the key and the value.
        let object = one[..at_key].rfind('{').and_then(|open| {
            let close = at_value + v1.len() + one[at_value + v1.len()..].find('}')? + 1;
            (serde_json::from_str::<J>(&one[open..close]).ok()? == json!({k1.clone(): v1.clone()}))
                .then_some(open..close)
        });
        let args = match object {
            Some(range) => (range, Piece::Json),
            None => {
                let pair = text("pair").ok_or("the format writes arguments as pairs, and did not write two")?;
                let (k2, v2) = (mark(KEY, 2), mark(VALUE, 2));
                let at = |s: &str| pair.find(s).ok_or("the format did not write both arguments");
                let (pk1, pv1, pk2) = (at(&k1)?, at(&v1)?, at(&k2)?);
                at(&v2)?;
                if !(pk1 < pv1 && pv1 < pk2) {
                    return Err("the format wrote two arguments out of order".into());
                }
                let mut pairs = Pairs {
                    open: pair[pk1 + k1.len()..pv1].to_string(),
                    between: pair[pv1 + v1.len()..pk2].to_string(),
                    quote: Default::default(),
                };
                // A number is written without what surrounds a string: that is the quote.
                if let Some(number) = text("number")
                    && let Some(nk) = number.find(&k1)
                    && let Some(n7) = number[nk..].find('7').map(|i| nk + i)
                {
                    let bare_open = &number[nk + k1.len()..n7];
                    let (after_string, after_number) = (&one[at_value + v1.len()..], &number[n7 + 1..]);
                    if pairs.open.starts_with(bare_open) && after_string.ends_with(after_number) {
                        let open = pairs.open[bare_open.len()..].to_string();
                        let close = after_string[..after_string.len() - after_number.len()].to_string();
                        if !open.is_empty() && pairs.between.starts_with(&close) {
                            pairs.open = bare_open.to_string();
                            pairs.between = pairs.between[close.len()..].to_string();
                            pairs.quote = (open, close);
                        }
                    }
                }
                let end = at_value + v1.len() + pairs.quote.1.len();
                (at_key..end, Piece::Pairs(pairs))
            }
        };
        // Two calls are head, call, between, call, tail. The second call is the first with its
        // marks renumbered, so the text repeats with a period: the distance between the two
        // calls' first marks. Where exactly a call begins within that period is a choice (the
        // text before a call and the text after one may share characters); it is put after a
        // character that closes something, so the pieces read as the format's own units.
        let closes = |c: char| c.is_whitespace() || ">]})|".contains(c);
        let (mut head, mut end, mut between) = (0, one.len(), String::new());
        let first_mark = [mark(NAME, 1), mark(ID, 0), k1.clone()]
            .into_iter()
            .filter_map(|m| one.find(&m).map(|at| (at, m)))
            .min();
        if let Some(two) = text("two")
            && let Some((p1, m1)) = first_mark
            && two.find(&m1) == Some(p1)
            && let Some(p2) = two.find(&second(&m1))
        {
            let period = p2 - p1;
            let shared = common_suffix(&second(&two[..p1]), &two[..p2]).min(period).min(p1);
            let leftmost = p1 - shared;
            let start = (leftmost..=p1)
                .filter(|at| two.is_char_boundary(*at))
                .find(|at| *at == 0 || two[..*at].chars().next_back().is_some_and(closes))
                .unwrap_or(leftmost);
            let same = one[start..]
                .char_indices()
                .zip(two[start..].chars())
                .find(|((_, a), b)| a != b)
                .map_or(one.len() - start, |((n, _), _)| n)
                .min(period);
            let least = args.0.end.saturating_sub(start);
            let length = (least..=same)
                .rev()
                .filter(|n| one.is_char_boundary(start + n))
                .find(|n| *n > 0 && one[..start + n].chars().next_back().is_some_and(closes))
                .unwrap_or(same.max(least));
            if start <= args.0.start && start + length <= one.len() && start + period >= start + length {
                head = start;
                end = start + length;
                between = two.get(end..start + period).unwrap_or_default().to_string();
            }
        }
        let (body, tail) = (&one[head..end], &one[end..]);
        let (start, end) = (args.0.start - head, args.0.end - head);
        let call = pieces(body, Some((start..end, args.1)));
        let has = |p: &Piece| call.contains(p);
        let id_name = probe["id_name"]
            .as_array()
            .and_then(|a| Some((a.first()?.as_str()?.to_string(), a.get(1)?.as_str()?.to_string())));
        let named = has(&Piece::Name) || (has(&Piece::Id) && id_name.is_some());
        if !named {
            return Err("the format did not write the tool's name".into());
        }
        // A call with no arguments, written with the same head and tail.
        let bare = text("none")
            .filter(|none| none.len() >= head + tail.len() && none.starts_with(&one[..head]) && none.ends_with(tail))
            .map(|none| pieces(&none[head..none.len() - tail.len()], None))
            .filter(|bare| *bare != call && !bare.is_empty());
        Ok(Pattern {
            head: one[..head].to_string(),
            call,
            bare,
            between,
            tail: tail.to_string(),
            think: probe["think"]
                .as_array()
                .and_then(|a| Some((a.first()?.as_str()?.to_string(), a.get(1)?.as_str()?.to_string())))
                .filter(|(_, close)| !close.trim().is_empty())
                .or(thinking.map(|(_, marks)| marks)),
            id_name,
        })
    }

    /// The parts of a reply written in this pattern.
    pub fn read(&self, text: &str, in_think: bool) -> Parts {
        let mut parts = Parts {
            think_closed: true,
            ..Default::default()
        };
        let mut rest = text;
        if let Some((open, close)) = &self.think {
            let (open, close) = (open.trim(), close.trim());
            let begun = !open.is_empty() && rest.trim_start().starts_with(open);
            if in_think || begun {
                let body = if begun { &rest.trim_start()[open.len()..] } else { rest };
                match body.find(close) {
                    Some(at) => {
                        parts.think = Some(body[..at].trim().to_string());
                        rest = &body[at + close.len()..];
                    }
                    // Thinking that a call ends: it runs up to where the call begins.
                    None => match self.anchor().and_then(|anchor| body.find(anchor)) {
                        Some(at) => {
                            parts.think = Some(body[..at].trim().to_string());
                            rest = &body[at..];
                        }
                        None => {
                            parts.think = Some(body.trim().to_string());
                            parts.think_closed = false;
                            return parts;
                        }
                    },
                }
            }
        }
        // The calls begin at the first text the format writes before one.
        let anchor = self.anchor();
        let begins_with_hole =
            self.head.trim().is_empty() && !matches!(self.call.first(), Some(Piece::Lit(s)) if !s.trim().is_empty());
        let start = match anchor {
            Some(_) if begins_with_hole => Some(0),
            Some(anchor) => rest.find(anchor),
            None => None,
        };
        let Some(start) = start else {
            parts.prose = rest.trim().to_string();
            return parts;
        };
        let mut at = start;
        let matched = (|| -> Result<Vec<Call>, String> {
            at = lit(rest, at, &self.head).ok_or("the calls do not begin as the format writes them")?;
            let mut calls = vec![];
            loop {
                let (call, next) = match self.one(rest, at, &self.call) {
                    Ok(done) => done,
                    Err(error) => match self.bare.as_ref().map(|bare| self.one(rest, at, bare)) {
                        Some(Ok(done)) => done,
                        _ => return Err(error),
                    },
                };
                calls.push(call);
                at = next;
                // Another call follows where the format's text between calls does.
                let more = match self.between.trim().is_empty() {
                    true => self.one(rest, at, &self.call).is_ok(),
                    false => lit(rest, at, &self.between).is_some(),
                };
                if !more {
                    break;
                }
                at = lit(rest, at, &self.between).unwrap_or(at);
            }
            // The tail may be missing: the reply's stop token is cut off, and often is the tail.
            at = lit(rest, at, &self.tail).unwrap_or(at);
            Ok(calls)
        })();
        match matched {
            Ok(calls) => {
                parts.prose = rest[..start].trim().to_string();
                parts.calls = calls;
                parts.trailing = rest[at..].trim().to_string();
            }
            Err(error) if begins_with_hole => {
                // A reply that is not a call at all.
                let _ = error;
                parts.prose = rest.trim().to_string();
            }
            Err(error) => {
                parts.prose = rest[..start].trim().to_string();
                parts.errors.push(error);
            }
        }
        parts
    }

    /// The first text the format writes before a call.
    fn anchor(&self) -> Option<&str> {
        std::iter::once(self.head.as_str())
            .chain(self.call.iter().filter_map(|p| match p {
                Piece::Lit(s) => Some(s.as_str()),
                _ => None,
            }))
            .map(str::trim)
            .find(|s| !s.is_empty())
    }

    /// One call at `at`: the call, and where it ends.
    fn one(&self, text: &str, mut at: usize, call: &[Piece]) -> Result<(Call, usize), String> {
        let (mut name, mut id, mut args) = (None, None, json!({}));
        // A format that writes an argument as an element named after its key writes the key
        // again to close it: text after a pair has the last key where the probe's key was.
        let key_mark = mark(KEY, 1);
        let mut last_key = String::new();
        let keyed = |s: &str, key: &str| s.replace(&key_mark, key);
        for (i, piece) in call.iter().enumerate() {
            // What a hole's text runs up to: the next literal.
            let next = call[i + 1..].iter().find_map(|p| match p {
                Piece::Lit(s) if !s.trim().is_empty() => Some(s.trim()),
                _ => None,
            });
            let until = |from: usize| -> usize {
                let stops = [next, Some(self.between.trim()), Some(self.tail.trim())];
                stops
                    .into_iter()
                    .flatten()
                    .filter(|s| !s.is_empty())
                    .filter_map(|s| text[from..].find(first_run(s)))
                    .min()
                    .map_or(text.len(), |n| from + n)
            };
            match piece {
                // The call's last text may be cut short: the token that stopped the reply is often
                // part of it.
                Piece::Lit(s) if i + 1 == call.len() => {
                    let s = keyed(s, &last_key);
                    at = lit_or_end(text, at, &s).ok_or_else(|| format!("expected {:?} to end the call", s.trim()))?
                }
                Piece::Lit(s) => {
                    let s = keyed(s, &last_key);
                    at = lit(text, at, &s).ok_or_else(|| format!("expected {:?} in the call", s.trim()))?
                }
                Piece::Name => {
                    at += text[at..].len() - text[at..].trim_start().len();
                    let end = at
                        + text[at..]
                            .find(|c: char| !(c.is_alphanumeric() || "_-.:/".contains(c)))
                            .unwrap_or(text.len() - at);
                    if end == at {
                        return Err("the call names no tool".into());
                    }
                    name = Some(text[at..end].to_string());
                    at = end;
                }
                Piece::Id => {
                    let end = until(at);
                    id = Some(text[at..end].trim().to_string());
                    at = end;
                }
                Piece::Json => {
                    let from = at + (text[at..].len() - text[at..].trim_start().len());
                    let mut values = serde_json::Deserializer::from_str(&text[from..]).into_iter::<J>();
                    match values.next() {
                        Some(Ok(value)) if value.is_object() => {
                            args = value;
                            at = from + values.byte_offset();
                        }
                        _ => return Err("the call's arguments are not a JSON object".into()),
                    }
                }
                Piece::Pairs(pairs) => {
                    let mut map = serde_json::Map::new();
                    loop {
                        let open = first_run(&pairs.open);
                        let key_end = text[at..]
                            .find(open)
                            .map(|n| at + n)
                            .ok_or("an argument has no value")?;
                        let key = text[at..key_end].trim().to_string();
                        // The text after this pair, with this pair's key where the format repeats it.
                        let between = keyed(&pairs.between, &key);
                        let after_pairs = next.map(|n| keyed(n, &key));
                        at = lit(text, key_end, &pairs.open).ok_or("an argument has no value")?;
                        // White space the format writes around a value is not part of it.
                        let lead = &pairs.open[pairs.open.trim_end().len()..];
                        if !lead.is_empty() && text[at..].starts_with(lead) {
                            at += lead.len();
                        }
                        let (quote_open, quote_close) = (pairs.quote.0.trim(), pairs.quote.1.trim());
                        let quoted = !quote_open.is_empty() && text[at..].trim_start().starts_with(quote_open);
                        let value = if quoted {
                            let from = at + text[at..].find(quote_open).unwrap() + quote_open.len();
                            let end = text[from..]
                                .find(quote_close)
                                .map(|n| from + n)
                                .ok_or("a string argument is not closed")?;
                            at = end + quote_close.len();
                            J::String(text[from..end].to_string())
                        } else {
                            // Up to the next pair or the end of the pairs, whichever comes first.
                            let more = first_run(&between);
                            let end = [
                                text[at..].find(more).filter(|_| !more.is_empty()).map(|n| at + n),
                                after_pairs
                                    .as_deref()
                                    .map(first_run)
                                    .filter(|run| !run.is_empty())
                                    .and_then(|run| text[at..].find(run))
                                    .map(|n| at + n),
                                Some(until(at)),
                            ]
                            .into_iter()
                            .flatten()
                            .min()
                            .unwrap_or(text.len());
                            let trail = &between[..between.len() - between.trim_start().len()];
                            let raw = &text[at..end];
                            let raw = match trail.is_empty() {
                                true => raw,
                                false => raw.strip_suffix(trail).unwrap_or(raw),
                            };
                            at = end;
                            // A value written bare where strings are quoted is JSON (a number, a list).
                            match quote_open.is_empty() {
                                true => J::String(raw.to_string()),
                                false => serde_json::from_str(raw.trim()).unwrap_or(J::String(raw.trim().to_string())),
                            }
                        };
                        map.insert(key.clone(), value);
                        last_key = key;
                        // Another pair follows where the text between pairs does, with a key and
                        // its opener before the call's next text (the text between pairs can
                        // also begin what closes the last one).
                        let Some(after) = lit(text, at, &between).filter(|_| !between.trim().is_empty()) else {
                            break;
                        };
                        let Some(key_end) = text[after..].find(open).map(|n| after + n) else {
                            break;
                        };
                        let limit = after_pairs
                            .as_deref()
                            .and_then(|n| text[at..].find(first_run(n)))
                            .map(|n| at + n);
                        if limit.is_some_and(|l| l < key_end) {
                            break;
                        }
                        at = after;
                    }
                    args = J::Object(map);
                }
            }
        }
        let name = match (name, &id, &self.id_name) {
            (Some(name), _, _) => name,
            (None, Some(id), Some((before, after))) => id
                .strip_prefix(before.as_str())
                .and_then(|rest| match after.is_empty() {
                    true => Some(rest),
                    false => rest.rfind(after.as_str()).map(|n| &rest[..n]),
                })
                .ok_or_else(|| format!("the call id {id:?} does not name a tool"))?
                .to_string(),
            _ => return Err("the call names no tool".into()),
        };
        Ok((Call { name, args, id }, at))
    }
}

/// `\uE0xx` escapes of the probe's marks, as the marks.
fn unescape_marks(text: &str) -> String {
    static ESCAPE: std::sync::LazyLock<regex::Regex> =
        std::sync::LazyLock::new(|| regex::Regex::new(r"(?i)\\u(e0[1-4][0-9a-f])").unwrap());
    ESCAPE
        .replace_all(text, |c: &regex::Captures| {
            char::from_u32(u32::from_str_radix(&c[1], 16).unwrap())
                .unwrap()
                .to_string()
        })
        .into_owned()
}

/// An empty thinking block at the start of a turn's text (`<think>\n\n</think>`, or its close
/// alone when the prompt opened it): its length, and the marks that open and close thinking.
fn empty_thinking(text: &str) -> Option<(usize, (String, String))> {
    static EMPTY: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"^\s*(?:<([\w:|]*think[\w:|]*)>\s*)?(</[\w:|]*think[\w:|]*>)\s*").unwrap()
    });
    let found = EMPTY.captures(text)?;
    let close = found[2].to_string();
    let open = found
        .get(1)
        .map_or_else(|| close.replacen("</", "<", 1), |name| format!("<{}>", name.as_str()));
    Some((found[0].len(), (open, close)))
}

/// The first run of non-space text in a literal: what marks where it begins in a reply.
fn first_run(s: &str) -> &str {
    let s = s.trim_start();
    &s[..s.find(char::is_whitespace).unwrap_or(s.len())]
}

/// `lit`, or the end of the text where it runs out part-way through `want`.
fn lit_or_end(text: &str, at: usize, want: &str) -> Option<usize> {
    let have: String = text[at..].chars().filter(|c| !c.is_whitespace()).collect();
    let want_run: String = want.chars().filter(|c| !c.is_whitespace()).collect();
    match want_run.starts_with(&have) {
        true => Some(text.len()),
        false => lit(text, at, want),
    }
}

/// Match the format's text `want` at `at`, ignoring differences in white space: where it ends.
fn lit(text: &str, at: usize, want: &str) -> Option<usize> {
    let mut rest = text[at..].char_indices().peekable();
    let mut end = at;
    for w in want.chars().filter(|c| !c.is_whitespace()) {
        loop {
            let (i, c) = rest.next()?;
            if c.is_whitespace() {
                continue;
            }
            if c != w {
                return None;
            }
            end = at + i + c.len_utf8();
            break;
        }
    }
    Some(end)
}
