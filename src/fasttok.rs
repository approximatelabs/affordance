//! Fast exact pre-tokenization and encoding.
//!
//! The HF `tokenizers` crate runs the `Split` pre-tokenizer stages with the onig
//! backtracking regex engine and builds a `PreTokenizedString`/`NormalizedString`
//! graph for every encoded run. For the handful of fixed patterns used by the
//! supported chat models this is far more machinery than the work requires.
//!
//! Two things live here:
//!
//! 1. `FastPreTok`: a `PreTokenizer` that reproduces the recognized `Split` stages
//!    with the faster `regex` crate (one exception, the negative lookahead in the
//!    trailing `\s+(?!\S)|\s+`, is rewritten; see below). It can be dropped into a
//!    `TokenizerImpl`, so the normalizer, `ByteLevel` mapping, BPE model,
//!    post-processor and added vocabulary all stay exactly as loaded.
//! 2. `Direct`: for the same models, encodes a run without any of the tokenizer
//!    bookkeeping at all: normalize (only if needed), split into byte ranges with the
//!    same regex scan, apply the `ByteLevel` byte->unicode map, and call the BPE model
//!    per piece. This is the fast path used by the cold renderer.
//!
//! The only construct the `regex` crate cannot express is the negative lookahead in
//! `\s+(?!\S)|\s+`. At a whitespace run, onig's `\s+(?!\S)` matches the whole run when
//! it is at end of input, otherwise the run minus its final character (and fails for a
//! single character, where `\s+` takes over). So we match `\s+` and, when the match is
//! all-whitespace, newline-free, at least two characters and not at end, stop one
//! character early and let the scan resume there. This matters: the freed-up space can
//! then be taken by an earlier alternative (e.g. `" b"` is one piece), exactly as onig
//! would.

use serde::{Deserialize, Deserializer};
use std::sync::LazyLock;
use tokenizers::normalizer::Range;
use tokenizers::pre_tokenizers::PreTokenizerWrapper;
use tokenizers::pre_tokenizers::byte_level::ByteLevel;
use tokenizers::pre_tokenizers::split::SplitPattern;
use tokenizers::{
    Model, ModelWrapper, NormalizedString, NormalizerWrapper, PostProcessorWrapper, PreTokenizedString, PreTokenizer,
    Result, SplitDelimiterBehavior, TokenizerImpl,
};

pub type FastTokenizer =
    TokenizerImpl<ModelWrapper, NormalizerWrapper, FastPreTok, PostProcessorWrapper, tokenizers::DecoderWrapper>;

/// A pre-tokenizer that is either the original wrapped one, or a fast equivalent.
#[derive(Clone)]
pub struct FastPreTok {
    inner: PreTokenizerWrapper,
    plan: Plan,
}

#[derive(Clone)]
enum Plan {
    Fallback,
    Stages(Vec<Stage>),
}

#[derive(Clone)]
enum Stage {
    Split(regex::Regex),
    Byte(ByteLevel),
}

impl FastPreTok {
    fn new(inner: PreTokenizerWrapper) -> Self {
        // Escape hatch for benchmarking the original path on the same binary.
        let plan = if std::env::var_os("AFFORDANCE_NO_FASTTOK").is_some() {
            Plan::Fallback
        } else {
            build_plan(&inner).unwrap_or(Plan::Fallback)
        };
        FastPreTok { inner, plan }
    }

    /// True when the fast regex pre-tokenizer is active for this tokenizer.
    pub fn fast(&self) -> bool {
        matches!(self.plan, Plan::Stages(_))
    }

    /// Build the pipeline-free encoder for this pre-tokenizer, if every stage is a
    /// supported `Split` followed by a plain (no-regex, no-prefix) `ByteLevel`.
    pub fn direct(&self, identity_norm: bool, nfc_norm: bool) -> Option<Direct> {
        if !identity_norm && !nfc_norm {
            return None;
        }
        let stages = match &self.plan {
            Plan::Stages(s) => s,
            Plan::Fallback => return None,
        };
        let mut splits = Vec::new();
        for (i, stage) in stages.iter().enumerate() {
            match stage {
                Stage::Split(re) => splits.push(re.clone()),
                Stage::Byte(bl) => {
                    if i + 1 != stages.len() || bl.add_prefix_space {
                        return None;
                    }
                }
            }
        }
        if splits.is_empty() {
            return None;
        }
        Some(Direct {
            splits,
            identity: identity_norm,
            id: NEXT_DIRECT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        })
    }

    /// How a section after the start of the input is pre-tokenized: Metaspace
    /// `prepend_scheme: first` prepends only at offset 0, so here it reads as `never`.
    pub fn after_start(&self) -> std::result::Result<FastPreTok, String> {
        fn never(v: &mut serde_json::Value) {
            match v {
                serde_json::Value::Object(o) => {
                    if o.get("type").is_some_and(|t| t == "Metaspace")
                        && o.get("prepend_scheme").is_some_and(|s| s == "first")
                    {
                        o.insert("prepend_scheme".into(), "never".into());
                    }
                    o.values_mut().for_each(never);
                }
                serde_json::Value::Array(a) => a.iter_mut().for_each(never),
                _ => {}
            }
        }
        let mut j = serde_json::to_value(&self.inner).map_err(|e| e.to_string())?;
        never(&mut j);
        Ok(FastPreTok::new(serde_json::from_value(j).map_err(|e| e.to_string())?))
    }
}

impl<'de> Deserialize<'de> for FastPreTok {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let inner = PreTokenizerWrapper::deserialize(d)?;
        Ok(FastPreTok::new(inner))
    }
}

fn build_plan(inner: &PreTokenizerWrapper) -> Option<Plan> {
    let seq = match inner {
        PreTokenizerWrapper::Sequence(s) => s,
        _ => return None,
    };
    let mut stages = Vec::new();
    for (i, pt) in seq.as_ref().iter().enumerate() {
        match pt {
            PreTokenizerWrapper::Split(s) => {
                if s.invert || !matches!(s.behavior, SplitDelimiterBehavior::Isolated) {
                    return None;
                }
                let pattern = match &s.pattern {
                    SplitPattern::Regex(r) => r,
                    SplitPattern::String(_) => return None,
                };
                stages.push(Stage::Split(compile_fast(pattern)?));
            }
            PreTokenizerWrapper::ByteLevel(bl) => {
                // With `use_regex` the ByteLevel stage re-splits on its own GPT-2 regex.
                if bl.use_regex || i + 1 != seq.as_ref().len() {
                    return None;
                }
                stages.push(Stage::Byte(*bl));
            }
            _ => return None,
        }
    }
    // ByteLevel must be the final stage: the fast scan byte-maps after all splits.
    if !matches!(stages.last(), Some(Stage::Byte(_))) {
        return None;
    }
    Some(Plan::Stages(stages))
}

/// Compile a `Split` pattern for the `regex` crate. Only the trailing lookahead
/// alternative is rewritten; any other unsupported construct falls back.
fn compile_fast(pattern: &str) -> Option<regex::Regex> {
    // This is an optimization for audited tokenizer patterns, not a replacement regex
    // engine. In particular arbitrary whitespace and nullable patterns are unsafe here.
    if !AUDITED_PATTERNS.contains(&pattern) {
        return None;
    }
    let rewritten = match pattern.strip_suffix(r"\s+(?!\S)|\s+") {
        Some(rest) => format!("{rest}\\s+"),
        None if !pattern.contains("(?!") => pattern.to_string(),
        None => return None,
    };
    regex::Regex::new(&rewritten).ok()
}

impl PreTokenizer for FastPreTok {
    fn pre_tokenize(&self, pretokenized: &mut PreTokenizedString) -> Result<()> {
        match &self.plan {
            Plan::Fallback => self.inner.pre_tokenize(pretokenized),
            Plan::Stages(stages) => {
                for stage in stages {
                    match stage {
                        Stage::Split(re) => {
                            pretokenized.split(|_, normalized| Ok(fast_split(&normalized, re)))?;
                        }
                        Stage::Byte(bl) => bl.pre_tokenize(pretokenized)?,
                    }
                }
                Ok(())
            }
        }
    }
}

/// The `Isolated` split of one normalized string by `re`: every match and every gap
/// between matches, in order, as byte-range substrings.
fn fast_split(n: &NormalizedString, re: &regex::Regex) -> Vec<NormalizedString> {
    let s = n.get();
    let mut ranges = Vec::with_capacity(8);
    split_ranges(s, re, &mut ranges);
    ranges
        .into_iter()
        .filter_map(|(a, b)| n.slice(Range::Normalized(a..b)))
        .collect()
}

/// Split `s` into byte ranges the way the onig `Split` pre-tokenizer with `Isolated`
/// behavior does: runs of consecutive regex matches, with the gaps between them kept.
///
/// The scan is manual (rather than `find_iter`) because a whitespace match may be
/// shortened to emulate onig's `\s+(?!\S)`; the regex then has to be re-run from the
/// shortened end so the last space can be absorbed by the *next* match.
fn split_ranges(s: &str, re: &regex::Regex, out: &mut Vec<(usize, usize)>) {
    each_range(s, re, &mut |a, b| out.push((a, b)));
}

/// `split_ranges` without collecting: `f` sees each piece in order.
fn each_range(s: &str, re: &regex::Regex, f: &mut dyn FnMut(usize, usize)) {
    if s.is_empty() {
        return;
    }
    let mut pos = 0usize;
    while pos < s.len() {
        let m = match re.find_at(s, pos) {
            Some(m) => m,
            None => break,
        };
        let (a, mut b) = (m.start(), m.end());
        if pos < a {
            f(pos, a);
        }
        if b > a && shorten_whitespace(s, a, b) {
            b = last_char_start(s, b);
        }
        f(a, b);
        pos = b;
    }
    if pos < s.len() {
        f(pos, s.len());
    }
}

/// True for a `\s+`-family match where onig's `\s+(?!\S)` would have stopped one
/// character early: the match is all whitespace, has no newline (a newline run is
/// taken by the earlier `\s*[\r\n]+` alternative), is not at end of input, and has at
/// least two characters.
fn shorten_whitespace(s: &str, a: usize, b: usize) -> bool {
    if b >= s.len() {
        return false;
    }
    let mut chars = 0usize;
    for c in s[a..b].chars() {
        chars += 1;
        if c == '\n' || c == '\r' || !c.is_whitespace() {
            return false;
        }
    }
    chars >= 2
}

fn last_char_start(s: &str, b: usize) -> usize {
    let mut i = b - 1;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// The GPT-2 byte -> unicode table (same as `tokenizers`' `ByteLevel`).
fn byte_to_char() -> &'static [char; 256] {
    static TABLE: LazyLock<[char; 256]> = LazyLock::new(|| {
        let mut bs: Vec<u8> = (b'!'..=b'~').collect();
        bs.extend(b'\xA1'..=b'\xAC');
        bs.extend(b'\xAE'..=b'\xFF');
        let mut cs: Vec<u32> = bs.iter().map(|&b| b as u32).collect();
        let mut n = 0u32;
        let mut table = ['\0'; 256];
        for (i, &b) in bs.iter().enumerate() {
            table[b as usize] = char::from_u32(cs[i]).unwrap();
        }
        for b in 0..=255u8 {
            if !bs.contains(&b) {
                cs.push(256 + n);
                table[b as usize] = char::from_u32(256 + n).unwrap();
                n += 1;
            }
        }
        table
    });
    &TABLE
}

/// Pipeline-free exact encoder for one of the recognized byte-level BPE models.
pub struct Direct {
    splits: Vec<regex::Regex>,
    identity: bool,
    /// Names this encoder's entries in the piece cache (`PIECES`).
    id: u64,
}

/// Pieces to keep per encoder and thread before the cache starts over.
const PIECE_CACHE: usize = 1 << 14;

thread_local! {
    /// A piece's ids, by encoder and piece text: BPE of a piece depends on the piece alone, so a
    /// repeated piece skips the byte mapping and the model (whose tokens carry their strings).
    static PIECES: std::cell::RefCell<std::collections::HashMap<u64, Pieces>> = Default::default();
}

type Pieces = std::collections::HashMap<Box<str>, Box<[u32]>, ahash::RandomState>;

static NEXT_DIRECT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

impl Direct {
    /// Encode an already-normalized string to ids. `model` is the tokenizer's own BPE
    /// model, so merges and the vocabulary are unchanged.
    pub fn encode(&self, model: &ModelWrapper, text: &str) -> Result<Vec<u32>> {
        let mut ranges: Vec<(usize, usize)> = vec![(0, text.len())];
        for re in &self.splits {
            let mut next = Vec::with_capacity(ranges.len() + 4);
            for &(a, b) in &ranges {
                if a >= b {
                    continue;
                }
                // Work on the piece's own sub-slice so matches cannot cross pieces.
                let mut local = Vec::with_capacity(4);
                split_ranges(&text[a..b], re, &mut local);
                next.extend(local.into_iter().map(|(x, y)| (a + x, a + y)));
            }
            ranges = next;
        }
        let mut ids = Vec::with_capacity(text.len() / 3 + 1);
        let mut buf = String::new();
        self.with_cache(|cache| {
            for (a, b) in ranges {
                if a < b {
                    piece_ids(model, cache, &text[a..b], &mut buf, &mut |p| ids.extend_from_slice(p))?
                }
            }
            Ok::<(), tokenizers::Error>(())
        })?;
        Ok(ids)
    }

    fn with_cache<R>(&self, f: impl FnOnce(&mut Pieces) -> R) -> R {
        PIECES.with(|p| f(p.borrow_mut().entry(self.id).or_default()))
    }

    /// `encode(..).len()` without building the ids or the piece list: each final piece is
    /// tokenized as it is found, so memory stays at one piece however long `text` is.
    pub fn count(&self, model: &ModelWrapper, text: &str) -> Result<usize> {
        let (mut n, mut buf, mut failed) = (0usize, String::new(), None);
        self.with_cache(|cache| {
            self.pieces(text, 0, &mut |piece| {
                if failed.is_some() {
                    return;
                }
                if let Err(e) = piece_ids(model, cache, piece, &mut buf, &mut |p| n += p.len()) {
                    failed = Some(e)
                }
            })
        });
        match failed {
            Some(e) => Err(e),
            None => Ok(n),
        }
    }

    fn pieces(&self, text: &str, depth: usize, f: &mut dyn FnMut(&str)) {
        if depth == self.splits.len() {
            return f(text);
        }
        each_range(text, &self.splits[depth], &mut |a, b| {
            if a < b {
                self.pieces(&text[a..b], depth + 1, f)
            }
        });
    }

    /// True when the tokenizer's normalizer is the identity (so no normalization pass
    /// is needed at all).
    pub fn identity(&self) -> bool {
        self.identity
    }
}

/// One pre-tokenized piece's ids, from the cache or the BPE model (after the `ByteLevel` map).
fn piece_ids(
    model: &ModelWrapper,
    cache: &mut Pieces,
    piece: &str,
    buf: &mut String,
    f: &mut dyn FnMut(&[u32]),
) -> Result<()> {
    if let Some(ids) = cache.get(piece) {
        f(ids);
        return Ok(());
    }
    let table = byte_to_char();
    buf.clear();
    for &byte in piece.as_bytes() {
        buf.push(table[byte as usize]);
    }
    let ids: Box<[u32]> = model.tokenize(buf)?.into_iter().map(|t| t.id).collect();
    f(&ids);
    if cache.len() >= PIECE_CACHE {
        cache.clear()
    }
    cache.insert(Box::from(piece), ids);
    Ok(())
}

/// Run `pre` over `text` and return its output pieces. Used to prove split equality.
#[cfg(test)]
pub(crate) fn split_pieces(pre: &dyn PreTokenizer, text: &str) -> Vec<String> {
    use tokenizers::{OffsetReferential, OffsetType};
    let mut pt = PreTokenizedString::from(text);
    let _ = pre.pre_tokenize(&mut pt);
    pt.get_splits(OffsetReferential::Normalized, OffsetType::None)
        .into_iter()
        .map(|(s, _, _)| s.to_string())
        .collect()
}

// Official Split patterns audited for the supported model families. Unknown patterns
// retain the original onig pipeline. These are syntax rules, never token answers.
const AUDITED_PATTERNS: &[&str] = &[
    "(?i:'s|'t|'re|'ve|'m|'ll|'d)|[^\\r\\n\\p{L}\\p{N}]?\\p{L}+|\\p{N}{1}| ?[^\\s\\p{L}\\p{N}]+[\\r\\n]*|\\s*[\\r\\n]+|\\s+(?!\\S)|\\s+",
    "(?i:'s|'t|'re|'ve|'m|'ll|'d)|[^\\r\\n\\p{L}\\p{N}]?\\p{L}+|\\p{N}| ?[^\\s\\p{L}\\p{N}]+[\\r\\n]*|\\s*[\\r\\n]+|\\s+(?!\\S)|\\s+",
    "(?i:'s|'t|'re|'ve|'m|'ll|'d)|[^\\r\\n\\p{L}\\p{N}]?[\\p{L}\\p{M}]+|\\p{N}| ?[^\\s\\p{L}\\p{M}\\p{N}]+[\\r\\n]*|\\s*[\\r\\n]+|\\s+(?!\\S)|\\s+",
    "\\p{N}{1,3}",
    "[一-龥぀-ゟ゠-ヿ]+",
    "[!\"#$%&'()*+,\\-./:;<=>?@\\[\\\\\\]^_`{|}~][A-Za-z]+|[^\r\n\\p{L}\\p{P}\\p{S}]?[\\p{L}\\p{M}]+| ?[\\p{P}\\p{S}]+[\r\n]*|\\s*[\r\n]+|\\s+(?!\\S)|\\s+",
    "(?i:'s|'t|'re|'ve|'m|'ll|'d)|[^\\r\\n\\p{L}\\p{N}]?\\p{L}+|\\p{N}{1,3}| ?[^\\s\\p{L}\\p{N}]+[\\r\\n]*|\\s*[\\r\\n]+|\\s+(?!\\S)|\\s+",
    "[^\\r\\n\\p{L}\\p{N}]?[\\p{Lu}\\p{Lt}\\p{Lm}\\p{Lo}\\p{M}]*[\\p{Ll}\\p{Lm}\\p{Lo}\\p{M}]+(?i:'s|'t|'re|'ve|'m|'ll|'d)?|[^\\r\\n\\p{L}\\p{N}]?[\\p{Lu}\\p{Lt}\\p{Lm}\\p{Lo}\\p{M}]+[\\p{Ll}\\p{Lm}\\p{Lo}\\p{M}]*(?i:'s|'t|'re|'ve|'m|'ll|'d)?|\\p{N}{1,3}| ?[^\\s\\p{L}\\p{N}]+[\\r\\n/]*|\\s*[\\r\\n]+|\\s+(?!\\S)|\\s+",
    "[^\\r\\n\\p{L}\\p{N}]?[\\p{Lu}\\p{Lt}\\p{Lm}\\p{Lo}\\p{M}]*[\\p{Ll}\\p{Lm}\\p{Lo}\\p{M}]+|[^\\r\\n\\p{L}\\p{N}]?[\\p{Lu}\\p{Lt}\\p{Lm}\\p{Lo}\\p{M}]+[\\p{Ll}\\p{Lm}\\p{Lo}\\p{M}]*|\\p{N}| ?[^\\s\\p{L}\\p{N}]+[\\r\\n/]*|\\s*[\\r\\n]+|\\s+(?!\\S)|\\s+",
];

#[cfg(test)]
#[path = "fasttok_tests.rs"]
mod tests;
