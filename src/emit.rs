//! Token emission. The renderer never builds the whole prompt string. It emits a
//! stream of special-token ids and text pieces. Consecutive text pieces form a *run*
//! (the text between two tokens that the tokenizer treats as atomic); each run is
//! encoded on its own with the model's tokenizer, and results are cached per run.
//!
//! Why runs and not per-piece encoding: BPE merges and pre-tokenizer splits can cross
//! the boundary between two pieces (e.g. "user\n" + "\nHello"). Inside a run the
//! renderer must therefore defer encoding until a hard boundary (an added token).
//! Added tokens are hard boundaries in every HF tokenizer, so per-run encoding gives
//! exactly the ids that tokenizing the full string would give, as long as two
//! tokenizer rules are reproduced:
//!   1. Metaspace `prepend_scheme: first` only prefixes the section at offset 0.
//!   2. Added tokens with `lstrip`/`rstrip` swallow neighbouring whitespace.

use crate::fasttok::{Direct, FastTokenizer};
use std::collections::HashMap;
use std::rc::Rc;
use tokenizers::Normalizer;

/// Layout items: unnameable outside the crate, so `Value::Frag` stays opaque to embedders.
mod items {
    use super::{Span, SpanSpec};
    use std::rc::Rc;

    #[derive(Debug, Clone)]
    pub enum Item {
        Text(Rc<str>),
        Tok(u32),
        Open(Rc<SpanSpec>),
        Close,
        /// Item boundary inside a box (for drop-start / drop-end overflow).
        Segment,
        /// A subtree already laid out by the same encoder. Wrappers preserve its token
        /// values and spans; no decoding or full-string tokenization is involved.
        Tokens(Rc<[u32]>, Rc<[Span]>),
        /// A strategy result consumed by a model-template app, never emitted as text.
        Turns(Rc<crate::turns::TurnPlan>),
    }
}
pub(crate) use items::Item;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SpanKind {
    Hidden,
    Label,
    Affordance,
    Box {
        max_tokens: Option<Size>,
        tok_size: Option<usize>,
        clip: Clip,
        pad: Option<u32>,
        more: Option<Rc<str>>,
        drop: Drop,
        drop_step: Option<Step>,
        scroll: Option<Scroll>,
        /// A tool result's style once its position matches `result:nth-last(n+K / S)`.
        graded: Option<Rc<Graded>>,
    },
}

/// A positional alternative for a result box: its kind when the
/// result is at least `k` from the end among its history's results, the boundary moving `s` at a
/// time.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Graded {
    pub k: usize,
    pub s: usize,
    pub kind: SpanKind,
}

/// Whether the `i`-th (0-based, oldest first) of `n` results takes its graded style: it is at
/// least `k` from the end, quantized so the boundary moves `s` results at a time.
pub fn graded_match(i: usize, n: usize, k: usize, s: usize) -> bool {
    n >= k && i <= s * ((n - k) / s)
}

/// The model a prompt is laid out for: its context length and the reply's room. The prompt's
/// window is what is left.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Window {
    /// The model's context length, in tokens.
    pub context: usize,
    /// The room kept for the reply.
    pub max_new: usize,
}
impl Window {
    /// The prompt's window: the context less the reply's room.
    pub fn tokens(self) -> usize {
        self.context.saturating_sub(self.max_new)
    }
}

/// A box's cap: tokens, or a share of the window (`80win`: 80% of the prompt's window, in
/// hundredths of a percent).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Size {
    Tokens(usize),
    Window(u32),
}

/// `drop-step`: tokens, or a share of the box's cap (hundredths of a percent).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Step {
    Tokens(usize),
    Percent(u32),
}
impl Step {
    pub fn tokens(self, limit: usize) -> usize {
        // Never more than the cap: a larger step drops the same (all but the minimum), and the
        // kernel's multiple of it cannot overflow.
        match self {
            Step::Tokens(n) => n.min(limit.max(1)),
            Step::Percent(p) => (limit as u64 * p as u64 / 10_000).max(1) as usize,
        }
    }
}

// A box's overflow policies and the drop kernel live with the arithmetic that fits a box to its
// limit (`fit.rs`, checked by Verus).
pub(crate) use crate::fit::drop_cut;
pub(crate) use crate::fit::{Clip, Drop};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Scroll {
    pub offset: usize,
    pub action: Rc<SpanSpec>,
}

/// Where a child span lands after a box shows only some ranges of its content. `ranges` are the
/// kept ranges in order, `(content start, content end, output offset)`; `[s, e)` is the span in
/// content coordinates (an empty span is a point). None when no kept range shows any of it;
/// else its output coordinates, clamped to what is shown, and whether one range holds it whole.
pub(crate) struct Rebased {
    pub start: usize,
    pub end: usize,
    pub whole: bool,
}

pub(crate) fn rebase(ranges: &[(usize, usize, usize)], s: usize, e: usize) -> Option<Rebased> {
    let point = s == e;
    let shows = |&&(a, b, _): &&(usize, usize, usize)| if point { a <= s && s < b } else { s < b && e > a };
    let first = ranges.iter().find(shows)?;
    let whole = ranges
        .iter()
        .any(|&(a, b, _)| if point { a <= s && s < b } else { a <= s && e <= b });
    let start = first.2 + s.max(first.0) - first.0;
    let end = if point {
        start
    } else {
        let last = ranges.iter().rev().find(shows).unwrap_or(first);
        last.2 + e.min(last.1) - last.0
    };
    Some(Rebased { start, end, whole })
}

impl Drop {
    fn name(self) -> &'static str {
        match self {
            Drop::None => "none",
            Drop::Start => "drop-start",
            Drop::End => "drop-end",
            Drop::Children => "drop-children",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpanSpec {
    pub(crate) label: Rc<str>,
    pub(crate) kind: SpanKind,
    pub(crate) meta: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
/// A labelled range of a render's token ids: text, an action (`<aff>`), or a box.
pub struct Span {
    /// Its label: the action's name, the box's or span's `label`.
    pub label: String,
    /// `label`, `affordance` or `box`.
    pub kind: String,
    /// The first token id of the range.
    pub start: usize,
    /// One past its last.
    pub end: usize,
    /// True if a boundary of this span falls inside a merged token.
    pub straddle: bool,
    /// What the element carried: an action's handler and schema, a box's `content_tokens` and `hidden_tokens`, extra attributes.
    pub meta: serde_json::Value,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct TokFlags {
    pub lstrip: bool,
    pub rstrip: bool,
}

/// A loaded tokenizer. `Send + Sync`: load it once per process and share it (`Arc<Encoder>`)
/// across render threads; each thread binds its own `View` (about 2.5 MiB with its run cache).
pub struct Encoder {
    /// The tokenizer.
    pub tok: FastTokenizer,
    /// The model's size rules for pictures, sound and video (`crate::media::Rules`): read from
    /// the processor files beside the tokenizer, or set from the model's registry row.
    pub media: crate::media::Rules,
    /// Same tokenizer with added-token matching disabled, special or not: text that happens to
    /// contain `"<|im_end|>"` or `"<think>"` stays text (views with `"use safe_text"`).
    pub tok_safe: FastTokenizer,
    /// `tok_safe` for runs that do not start the prompt, on a position-dependent tokenizer
    /// (set with `nonfirst_prefix`): the prefix trick needs the prefix read as a token, so the
    /// safe path instead encodes as a non-first section does (Metaspace `prepend_scheme: never`).
    tok_safe_rest: Option<FastTokenizer>,
    /// Added tokens that are safe to resolve statically (normalized=false, single_word=false).
    pub added: HashMap<String, u32>,
    /// How each added token strips the whitespace around it.
    pub(crate) flags: HashMap<u32, TokFlags>,
    /// If set, runs that do not start the prompt are encoded as `prefix + run` and the
    /// first id is dropped (reproduces Metaspace prepend_scheme=first).
    pub nonfirst_prefix: Option<(String, u32)>,
    /// The added tokens' texts and ids.
    pub special_strings: Vec<(String, u32)>,
    /// sha256 of the tokenizer file: what a journal pins (`docs/RUNTIME.md`, resume).
    pub identity: String,
    /// Matcher over `special_strings` (leftmost-longest, like the tokenizer's added vocabulary).
    pub special_ac: Option<aho_corasick::AhoCorasick>,
    /// Pipeline-free exact encoder for recognized byte-level BPE models. `None` for
    /// everything else, which then uses `tok`/`encode_run`.
    pub direct: Option<Direct>,
}

impl Encoder {
    /// Load a `tokenizer.json`.
    pub fn from_file(path: &str) -> Result<Encoder, String> {
        let mut tok = FastTokenizer::from_file(path).map_err(|e| format!("{}: {}", path, e))?;
        // A tokenizer file can ask for its output to be cut to a length or padded to one (a
        // training setting some repositories ship). A prompt is never cut or padded here: its
        // length is the app's and the layout's.
        tok.with_truncation(None).map_err(|e| format!("{path}: {e}"))?;
        tok.with_padding(None);
        let identity;
        // With no normalizer (or an empty one), "normalized" added tokens match the raw text too,
        // so they are tokens like any other (DeepSeek's <｜User｜>, <｜tool▁sep｜>, ...).
        let (identity_norm, nfc_norm, deterministic_bpe) = {
            let j: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            if j["model"]["dropout"].as_f64().is_some_and(|p| p != 0.0) {
                return Err(format!(
                    "{path}: tokenizer dropout is nondeterministic; exact rendering requires dropout 0 or null"
                ));
            }
            identity = crate::sha256_hex(&std::fs::read(path).map_err(|e| e.to_string())?);
            let identity = match &j["normalizer"] {
                serde_json::Value::Null => true,
                n => n["type"] == "Sequence" && n["normalizers"].as_array().is_some_and(|a| a.is_empty()),
            };
            (identity, j["normalizer"]["type"] == "NFC", j["model"]["type"] == "BPE")
        };
        let mut added = HashMap::new();
        let mut flags = HashMap::new();
        let mut special_strings = vec![];
        let mut prefix_cand: Option<(String, u32)> = None;
        let mut dec: Vec<_> = tok.get_added_tokens_decoder().into_iter().collect();
        dec.sort_by_key(|(id, _)| *id);
        for (id, t) in dec {
            if t.lstrip || t.rstrip {
                flags.insert(
                    id,
                    TokFlags {
                        lstrip: t.lstrip,
                        rstrip: t.rstrip,
                    },
                );
            }
            if (!t.normalized || identity_norm) && !t.single_word {
                added.insert(t.content.clone(), id);
                special_strings.push((t.content.clone(), id));
                if t.special && !t.lstrip && !t.rstrip && prefix_cand.is_none() {
                    prefix_cand = Some((t.content.clone(), id));
                }
            }
        }
        special_strings.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        let (tok_safe, own_model) = text_only(&tok).map_err(|e| format!("{path}: {e}"))?;
        let special_ac = if special_strings.is_empty() {
            None
        } else {
            aho_corasick::AhoCorasick::builder()
                .match_kind(aho_corasick::MatchKind::LeftmostLongest)
                .build(special_strings.iter().map(|(s, _)| s.as_str()))
                .ok()
        };
        let added_supported = tok
            .get_added_tokens_decoder()
            .values()
            .all(|t| !t.single_word && (!t.normalized || identity_norm));
        // (The pipeline-free path encodes with one model; a text-only tokenizer with a model of
        // its own takes the tokenizer's path.)
        let direct = if deterministic_bpe && added_supported && !own_model {
            tok.get_pre_tokenizer().and_then(|p| p.direct(identity_norm, nfc_norm))
        } else {
            None
        };
        let mut enc = Encoder {
            media: crate::media::Rules::beside(std::path::Path::new(path)),
            tok,
            tok_safe,
            tok_safe_rest: None,
            added,
            flags,
            nonfirst_prefix: None,
            special_strings,
            identity,
            special_ac,
            direct,
        };
        // Detect whether a run's position (first in prompt or not) changes its encoding.
        if let Some((p, pid)) = prefix_cand {
            let probes = ["hello world", " hello", "\nhi", "Hi there", "[INST]", "▁x", "  two"];
            let differs = probes.iter().any(|s| {
                let a = enc.raw(s);
                let b = enc.raw(&format!("{}{}", p, s));
                b.first() != Some(&pid) || b[1..] != a[..]
            });
            if differs {
                // Safe text after the start encodes as a non-first section does; it must agree
                // with the prefix trick on text that holds no added token, or nothing is safe.
                let mut rest = enc.tok_safe.clone();
                let pre = enc
                    .tok
                    .get_pre_tokenizer()
                    .map(|p| p.after_start())
                    .transpose()
                    .map_err(|e| format!("{path}: {e}"))?;
                rest.with_pre_tokenizer(pre);
                let trick = |s: &str| enc.raw(&format!("{p}{s}")).get(1..).map(<[u32]>::to_vec);
                let plain = |s: &&&str| !enc.special_ac.as_ref().is_some_and(|ac| ac.is_match(**s));
                if let Some(s) = probes
                    .iter()
                    .filter(plain)
                    .find(|s| rest.encode_fast(**s, false).map(|e| e.get_ids().to_vec()).ok() != trick(s))
                {
                    return Err(format!(
                        "{path}: position-dependent tokenization other than Metaspace prepend_scheme=first is not supported (probe {s:?})"
                    ));
                }
                enc.tok_safe_rest = Some(rest);
                enc.nonfirst_prefix = Some((p, pid));
            }
        }
        Ok(enc)
    }

    /// The ids of `s` as the tokenizer encodes it alone, special tokens matched.
    pub fn raw(&self, s: &str) -> Vec<u32> {
        self.tok
            .encode_fast(s, false)
            .map(|e| e.get_ids().to_vec())
            .unwrap_or_default()
    }

    /// Encode one run of text. `first`: the run starts the prompt. `safe`: never match an
    /// added token (special or not) inside the text.
    pub fn encode_run(&self, s: &str, first: bool, safe: bool) -> Vec<u32> {
        let t = match (&self.nonfirst_prefix, first, safe) {
            (Some((p, _)), false, false) => {
                let mut buf = String::with_capacity(p.len() + s.len());
                buf.push_str(p);
                buf.push_str(s);
                let mut v = self.raw(&buf);
                v.remove(0);
                return v;
            }
            (Some(_), false, true) => self.tok_safe_rest.as_ref().expect("set with nonfirst_prefix"),
            (_, _, true) => &self.tok_safe,
            (_, _, false) => &self.tok,
        };
        t.encode_fast(s, false)
            .map(|e| e.get_ids().to_vec())
            .unwrap_or_default()
    }

    /// Pipeline-free encoding of one run, used by the cold path for recognized models.
    /// Returns `None` when this run must go through the tokenizer (not a recognized
    /// model, position-dependent tokenization, or an added-token string in the text).
    pub fn encode_direct(&self, s: &str) -> Option<Vec<u32>> {
        // The renderer splits dynamic text on these strings before it reaches a run,
        // so this only guards against unexpected paths (`piecewise`, hand-built runs).
        if let Some(ac) = &self.special_ac
            && ac.is_match(s)
        {
            return None;
        }
        self.encode_direct_text(s)
    }

    /// `encode_direct` for a safe run: added-token strings are text, which is exactly what the
    /// pipeline-free path encodes (it has no added vocabulary).
    fn encode_direct_text(&self, s: &str) -> Option<Vec<u32>> {
        let d = self.direct.as_ref()?;
        if self.nonfirst_prefix.is_some() {
            return None;
        }
        let model = self.tok.get_model();
        if d.identity() || s.is_ascii() {
            d.encode(model, s).ok()
        } else {
            let mut ns = tokenizers::NormalizedString::from(s);
            self.tok.get_normalizer()?.normalize(&mut ns).ok()?;
            d.encode(model, ns.get()).ok()
        }
    }

    /// The token length of `s` as one safe run (`encode_run(s, true, true).len()`). Recognized
    /// models count on the pipeline-free path without materializing ids, so counting a tool
    /// output of many megabytes costs one pre-token of memory, not the tokenizer's encoding.
    pub fn count_run(&self, s: &str) -> usize {
        let direct = self
            .direct
            .as_ref()
            .filter(|d| self.nonfirst_prefix.is_none() && (d.identity() || s.is_ascii()));
        match direct.and_then(|d| d.count(self.tok.get_model(), s).ok()) {
            Some(n) => n,
            None => self.encode_run(s, true, true).len(),
        }
    }

    /// The ids of one run of prompt text with each id's byte range in it; `first`: the run starts the prompt; `safe`: added tokens in it stay text.
    pub fn encode_run_offsets_safe(&self, s: &str, first: bool, safe: bool) -> (Vec<u32>, Vec<(usize, usize)>) {
        let (text, shift, drop) = match (&self.nonfirst_prefix, first, safe) {
            (Some((p, _)), false, false) => (format!("{}{}", p, s), p.len(), 1),
            _ => (s.to_string(), 0, 0),
        };
        let tok = match (&self.tok_safe_rest, first, safe) {
            (Some(rest), false, true) => rest,
            (_, _, true) => &self.tok_safe,
            (_, _, false) => &self.tok,
        };
        let e = tok.encode(text.as_str(), false).expect("encode");
        let ids = e.get_ids()[drop..].to_vec();
        let offs = e.get_offsets()[drop..]
            .iter()
            .map(|&(a, b)| (a.saturating_sub(shift), b.saturating_sub(shift)))
            .collect();
        (ids, offs)
    }

    /// The id of the token whose text is exactly `s`, if there is one.
    pub fn token_id(&self, s: &str) -> Option<u32> {
        self.added.get(s).copied().or_else(|| self.tok.token_to_id(s))
    }
}

/// `tok` reading every added token as text: each is marked special and special matching is
/// turned off (the `tokenizers` flag skips special added tokens only). Ids are unchanged.
///
/// Some vocabularies can also build a special token out of ordinary pieces: a SentencePiece
/// vocabulary exported to this format lists its control pieces among its merges, so the text
/// ` <PRE>` merges into CodeLlama's infill token with no added vocabulary at all. The text-only
/// tokenizer's model has those merges removed, so text is never a special token. The second
/// value says whether it has a model of its own for that reason.
fn text_only(tok: &FastTokenizer) -> Result<(FastTokenizer, bool), String> {
    let mut safe = tok.clone();
    let added = tok.get_added_tokens_decoder();
    let tokens: Vec<_> = added
        .values()
        .filter(|t| !t.special)
        .cloned()
        .map(|t| tokenizers::AddedToken { special: true, ..t })
        .collect();
    if !tokens.is_empty() {
        safe.add_special_tokens(tokens).map_err(|e| e.to_string())?;
    }
    safe.set_encode_special_tokens(true);
    let buildable: std::collections::HashSet<&str> = added
        .values()
        .filter(|t| {
            t.special
                && t.content.chars().count() > 1
                && tokenizers::Model::token_to_id(tok.get_model(), &t.content).is_some()
        })
        .map(|t| t.content.as_str())
        .collect();
    let mut own_model = false;
    if !buildable.is_empty() && matches!(tok.get_model(), tokenizers::ModelWrapper::BPE(_)) {
        let mut model = serde_json::to_value(tok.get_model()).map_err(|e| e.to_string())?;
        if let Some(merges) = model["merges"].as_array_mut() {
            let before = merges.len();
            merges.retain(|merge| {
                let joined = match merge {
                    serde_json::Value::Array(pair) => pair.iter().filter_map(|x| x.as_str()).collect::<String>(),
                    serde_json::Value::String(pair) => pair.replacen(' ', "", 1),
                    _ => String::new(),
                };
                !buildable.contains(joined.as_str())
            });
            if merges.len() != before {
                let model: tokenizers::ModelWrapper = serde_json::from_value(model).map_err(|e| e.to_string())?;
                safe.with_model(model);
                own_model = true;
            }
        }
    }
    Ok((safe, own_model))
}

/// Encode one run, preferring the pipeline-free `Direct` path. `piecewise` runs are
/// encoded on their own by design and may contain added-token strings, so they always
/// take the tokenizer path.
fn enc_one(enc: &Encoder, s: &str, first: bool, safe: bool, piecewise: bool) -> Vec<u32> {
    if !piecewise {
        let direct = if safe {
            enc.encode_direct_text(s)
        } else {
            enc.encode_direct(s)
        };
        if let Some(v) = direct {
            return v;
        }
    }
    enc.encode_run(s, first, safe)
}

/// Per-thread cache of encoded runs. Re-rendering a view whose history did not change
/// hits this cache for every old message. It is generational: `next_generation` starts a new
/// one, and an entry not used since the one before is dropped, so the cache holds what recent
/// renders used (a window), not everything a long run ever rendered.
#[derive(Default)]
pub struct RunCache {
    /// [safe][first]
    maps: [[HashMap<Box<str>, Rc<[u32]>, ahash::RandomState>; 2]; 2],
    /// The previous generation: an entry found here moves to `maps`.
    old: [[HashMap<Box<str>, Rc<[u32]>, ahash::RandomState>; 2]; 2],
    /// Runs found in the cache.
    pub hits: usize,
    /// Runs that had to be encoded.
    pub misses: usize,
    /// Whether it keeps anything (an agent's renders do; a single render need not).
    pub enabled: bool,
    /// `memoScan`'s kept scans (an agent's renders only: when `enabled`).
    pub memo: Rc<std::cell::RefCell<crate::memo::MemoStore>>,
}

impl RunCache {
    /// An empty cache; `enabled`: keep runs between renders.
    pub fn new(enabled: bool) -> Self {
        RunCache {
            enabled,
            memo: Rc::new(std::cell::RefCell::new(crate::memo::MemoStore::new())),
            ..Default::default()
        }
    }

    /// Start a new generation; entries of the one before the last are dropped.
    pub fn next_generation(&mut self) {
        self.memo.borrow_mut().next_generation();
        self.next_run_generation();
    }

    /// A new generation of token runs only: runs not used since the one before are dropped.
    /// Advanced per layout, so an evaluation that is not laid out keeps the runs alive.
    pub fn next_run_generation(&mut self) {
        self.old = std::mem::take(&mut self.maps);
    }
}

/// Output sink. `Capture` records items (component children); `Live` produces ids.
pub(crate) trait Out {
    fn text(&mut self, s: &str);
    fn text_rc(&mut self, s: &Rc<str>) {
        self.text(s)
    }
    fn tok(&mut self, id: u32);
    fn open(&mut self, spec: Rc<SpanSpec>);
    fn close(&mut self) -> Result<(), String>;
    /// Mark an item boundary (used by boxes that drop whole items).
    fn segment(&mut self) {}
    fn tokens(&mut self, _ids: &Rc<[u32]>, _spans: &Rc<[Span]>) -> Result<(), String> {
        Err("this output sink does not support laid-out subtrees".into())
    }
    fn turns(&mut self, _plan: &Rc<crate::turns::TurnPlan>) -> Result<(), String> {
        Err("TurnStrategy must be a child of a model-template app".into())
    }
    fn replay(&mut self, items: &[Item]) -> Result<(), String> {
        for it in items {
            match it {
                Item::Text(s) => self.text_rc(s),
                Item::Tok(t) => self.tok(*t),
                Item::Open(s) => self.open(s.clone()),
                Item::Close => self.close()?,
                Item::Segment => self.segment(),
                Item::Tokens(ids, spans) => self.tokens(ids, spans)?,
                Item::Turns(plan) => self.turns(plan)?,
            }
        }
        Ok(())
    }
}

#[derive(Default)]
pub(crate) struct Capture {
    pub items: Vec<Item>,
}

impl Out for Capture {
    fn turns(&mut self, plan: &Rc<crate::turns::TurnPlan>) -> Result<(), String> {
        self.items.push(Item::Turns(plan.clone()));
        Ok(())
    }
    fn tokens(&mut self, ids: &Rc<[u32]>, spans: &Rc<[Span]>) -> Result<(), String> {
        self.items.push(Item::Tokens(ids.clone(), spans.clone()));
        Ok(())
    }
    fn text(&mut self, s: &str) {
        if let Some(Item::Text(prev)) = self.items.last_mut() {
            let mut n = String::with_capacity(prev.len() + s.len());
            n.push_str(prev);
            n.push_str(s);
            *prev = Rc::from(n);
        } else {
            self.items.push(Item::Text(Rc::from(s)));
        }
    }
    fn text_rc(&mut self, s: &Rc<str>) {
        if matches!(self.items.last(), Some(Item::Text(_))) {
            self.text(s)
        } else {
            self.items.push(Item::Text(s.clone()))
        }
    }
    fn tok(&mut self, id: u32) {
        self.items.push(Item::Tok(id))
    }
    fn open(&mut self, spec: Rc<SpanSpec>) {
        self.items.push(Item::Open(spec))
    }
    fn close(&mut self) -> Result<(), String> {
        self.items.push(Item::Close);
        Ok(())
    }
    fn segment(&mut self) {
        self.items.push(Item::Segment);
    }
}

struct OpenSpan {
    spec: Rc<SpanSpec>,
    start: usize,
    start_mark: Option<usize>,
    /// Item boundaries inside a drop box, as mark ids: the box's content is encoded as one run
    /// (as the template wrote it), so a boundary is resolved like a span's.
    segments: Vec<usize>,
    child_spans_start: usize,
    /// Tokens a graded child (an elided result) took off this drop box's content, so far; with
    /// each segment mark, the amount before it. A drop measures undegraded lengths: grading never
    /// moves the cut.
    inflation: i64,
    seg_inflation: Vec<i64>,
    /// A result box that took its graded style: its limit under the static style (None: no cap),
    /// for the undegraded length its drop box measures.
    undegraded: Option<Option<usize>>,
}

pub(crate) struct Live<'e> {
    hidden_depth: usize,
    enc: &'e Encoder,
    cache: &'e mut RunCache,
    pub ids: Vec<u32>,
    run: String,
    strip_next_ws: bool,
    /// (byte position in run, mark id) for marks waiting on the run's encoding.
    pending_marks: Vec<(usize, usize)>,
    /// mark id -> (token index, straddle)
    marks: Vec<(usize, bool)>,
    open: Vec<OpenSpan>,
    pub spans: Vec<Span>,
    /// Ids produced so far count as "the prompt has started" for Metaspace-first.
    started: bool,
    /// This layout continues a prompt (`start_in_prompt`): it stays not-at-the-start however
    /// little it has written so far.
    in_prompt: bool,
    /// Label/affordance spans whose start/end still hold mark ids.
    unresolved: Vec<usize>,
    /// `pragma safe_text;`: dynamic text never turns into special tokens.
    pub safe: bool,
    /// Diagnostic: encode every text piece on its own (no runs). Used by the boundary study.
    pub piecewise: bool,
    /// The prompt's window in tokens (the model's context less the reply's room), which
    /// `max-tokens: Nwin` is a share of. None when the binding states no context.
    pub window: Option<usize>,
    /// Graded results (`result:nth-last`): a history's subtree is recorded until it closes, its
    /// graded results counted, then laid out with each one's position known.
    capture: Option<(Vec<Item>, usize)>,
    /// Per history being laid out: (graded results in it, the next one's index). Hosted
    /// flattening seeds one for its messages.
    pub positions: Vec<(usize, usize)>,
}

impl<'e> Live<'e> {
    pub fn new(enc: &'e Encoder, cache: &'e mut RunCache) -> Self {
        Live {
            hidden_depth: 0,
            enc,
            cache,
            ids: vec![],
            run: String::new(),
            strip_next_ws: false,
            pending_marks: vec![],
            marks: vec![],
            open: vec![],
            spans: vec![],
            started: false,
            in_prompt: false,
            unresolved: vec![],
            safe: false,
            piecewise: false,
            window: None,
            capture: None,
            positions: vec![],
        }
    }

    /// A box's cap in tokens. A share of the window with no window (the binding states no
    /// `context`) is no cap: a page with no viewport is not clipped.
    pub(crate) fn resolve_size(&self, size: Size) -> Option<usize> {
        match size {
            Size::Tokens(n) => Some(n),
            Size::Window(p) => self.window.map(|w| (w as u64 * p as u64 / 10_000) as usize),
        }
    }

    /// Native chat bodies are embedded after protocol tokens, including for
    /// tokenizers whose normalization differs at the start of a prompt.
    pub fn start_in_prompt(&mut self) {
        self.started = true;
        self.in_prompt = true;
    }

    fn flush(&mut self) {
        if self.run.is_empty() {
            for (_, m) in self.pending_marks.drain(..) {
                self.marks[m] = (self.ids.len(), false);
            }
            return;
        }
        let first = !self.started;
        self.started = true;
        let base = self.ids.len();
        // A mark inside the run needs the token offsets (an uncached encode); one at the run's
        // start or end is exact without them (a turn's marks sit at special tokens or run ends).
        if self
            .pending_marks
            .iter()
            .any(|&(pos, _)| pos > 0 && pos < self.run.len())
        {
            let (ids, offs) = self.enc.encode_run_offsets_safe(&self.run, first, self.safe);
            for (pos, m) in self.pending_marks.drain(..) {
                let idx = offs.iter().take_while(|o| o.0 < pos).count();
                let straddle = offs.iter().any(|o| o.0 < pos && pos < o.1);
                self.marks[m] = (base + idx, straddle);
            }
            self.ids.extend_from_slice(&ids);
            self.run.clear();
            return;
        }
        self.encode_run(first);
        for (pos, m) in self.pending_marks.drain(..) {
            self.marks[m] = (if pos == 0 { base } else { self.ids.len() }, false);
        }
        self.run.clear();
    }

    /// Encode the run (no marks inside it), through the run cache when it is enabled.
    fn encode_run(&mut self, first: bool) {
        if self.cache.enabled {
            let (s, f) = (self.safe as usize, first as usize);
            // A run from the last generation moves over with its key (no reallocation).
            if let Some((k, v)) = self.cache.old[s][f].remove_entry(self.run.as_str()) {
                self.cache.maps[s][f].insert(k, v);
            }
            let map = &mut self.cache.maps[s][f];
            if let Some(v) = map.get(self.run.as_str()) {
                self.cache.hits += 1;
                self.ids.extend_from_slice(v);
            } else {
                self.cache.misses += 1;
                let ids = enc_one(self.enc, &self.run, first, self.safe, self.piecewise);
                let v: Rc<[u32]> = ids.into();
                self.ids.extend_from_slice(&v);
                if map.len() > 200_000 {
                    map.clear();
                }
                map.insert(Box::from(self.run.as_str()), v);
            }
        } else {
            let v = enc_one(self.enc, &self.run, first, self.safe, self.piecewise);
            self.ids.extend_from_slice(&v);
        }
    }

    fn mark(&mut self) -> usize {
        let id = self.marks.len();
        self.marks.push((self.ids.len(), false));
        if self.run.is_empty() {
            // resolved at once unless text follows in this run
            self.pending_marks.push((0, id));
        } else {
            self.pending_marks.push((self.run.len(), id));
        }
        id
    }

    pub fn finish(mut self) -> Result<(Vec<u32>, Vec<Span>), String> {
        self.flush();
        if !self.open.is_empty() || self.hidden_depth != 0 {
            return Err("unclosed span".into());
        }
        self.resolve_spans();
        Ok((self.ids, self.spans))
    }

    fn resolve_spans(&mut self) {
        for i in std::mem::take(&mut self.unresolved) {
            let (s, s_str) = self.marks[self.spans[i].start];
            let (e, e_str) = self.marks[self.spans[i].end];
            let sp = &mut self.spans[i];
            sp.start = s;
            sp.end = e;
            sp.straddle = s_str || e_str;
        }
    }
}

impl<'e> Live<'e> {
    fn text_plain(&mut self, s: &str) {
        let s = if self.strip_next_ws {
            let t = s.trim_start();
            if !t.is_empty() {
                self.strip_next_ws = false;
            }
            t
        } else {
            s
        };
        if self.piecewise && !self.run.is_empty() && !s.is_empty() {
            self.flush();
        }
        self.run.push_str(s);
    }
}

impl<'e> Live<'e> {
    /// While a graded history is being recorded, an item goes to the record.
    fn recording(&mut self, item: impl FnOnce() -> Item) -> bool {
        match &mut self.capture {
            Some((items, _)) => {
                items.push(item());
                true
            }
            None => false,
        }
    }

    /// The history's subtree is recorded: count its graded results, lay it out with their
    /// positions known, then close the history.
    fn replay_graded(&mut self) -> Result<(), String> {
        let (items, _) = self.capture.take().unwrap();
        let n = items
            .iter()
            .filter(|it| matches!(it, Item::Open(s) if matches!(&s.kind, SpanKind::Box { graded: Some(_), .. })))
            .count();
        self.positions.push((n, 0));
        let r = self.replay(&items);
        self.positions.pop();
        r
    }
}

impl<'e> Out for Live<'e> {
    fn turns(&mut self, plan: &Rc<crate::turns::TurnPlan>) -> Result<(), String> {
        if self.recording(|| Item::Turns(plan.clone())) {
            return Ok(());
        }
        if self.hidden_depth > 0 {
            Ok(())
        } else {
            Err("TurnStrategy must be a child of a model-template app".into())
        }
    }
    fn tokens(&mut self, ids: &Rc<[u32]>, spans: &Rc<[Span]>) -> Result<(), String> {
        if self.recording(|| Item::Tokens(ids.clone(), spans.clone())) {
            return Ok(());
        }
        if self.hidden_depth > 0 {
            return Ok(());
        }
        self.flush();
        let base = self.ids.len();
        self.ids.extend_from_slice(ids);
        for span in spans.iter() {
            if span.start > span.end || span.end > ids.len() {
                return Err("subtree span is outside its tokens".into());
            }
            let mut span = span.clone();
            span.start += base;
            span.end += base;
            self.spans.push(span);
        }
        if !ids.is_empty() {
            self.started = true;
            self.strip_next_ws = ids
                .last()
                .and_then(|id| self.enc.flags.get(id))
                .is_some_and(|f| f.rstrip);
        }
        Ok(())
    }
    fn segment(&mut self) {
        if self.recording(|| Item::Segment) {
            return;
        }
        if self.hidden_depth > 0 {
            return;
        }
        if self
            .open
            .last()
            .is_some_and(|o| matches!(o.spec.kind, SpanKind::Box { drop, .. } if drop != Drop::None))
        {
            let m = self.mark();
            let o = self.open.last_mut().unwrap();
            o.segments.push(m);
            o.seg_inflation.push(o.inflation);
        }
    }
    /// Dynamic text that contains added-token strings is split here, exactly where the
    /// tokenizer's added vocabulary would split it, so every message becomes its own cached run.
    /// (Not in safe_text mode, where such strings stay text.)
    fn text(&mut self, s: &str) {
        if self.recording(|| Item::Text(Rc::from(s))) {
            return;
        }
        if self.hidden_depth > 0 {
            return;
        }
        if !self.safe
            && !self.piecewise
            && let Some(ac) = &self.enc.special_ac
        {
            let mut last = 0;
            let mut found = false;
            for m in ac.find_iter(s) {
                found = true;
                if m.start() > last {
                    self.text_plain(&s[last..m.start()]);
                }
                self.tok(self.enc.special_strings[m.pattern().as_usize()].1);
                last = m.end();
            }
            if found {
                if last < s.len() {
                    self.text_plain(&s[last..]);
                }
                return;
            }
        }
        self.text_plain(s)
    }
    fn tok(&mut self, id: u32) {
        if self.recording(|| Item::Tok(id)) {
            return;
        }
        if self.hidden_depth > 0 {
            return;
        }
        let f = self.enc.flags.get(&id).copied().unwrap_or_default();
        if f.lstrip {
            let l = self.run.trim_end().len();
            self.run.truncate(l);
        }
        self.flush();
        self.ids.push(id);
        self.started = true;
        self.strip_next_ws = f.rstrip;
    }
    fn open(&mut self, spec: Rc<SpanSpec>) {
        if let Some((items, depth)) = &mut self.capture {
            items.push(Item::Open(spec));
            *depth += 1;
            return;
        }
        if self.hidden_depth > 0 || matches!(spec.kind, SpanKind::Hidden) {
            self.hidden_depth += 1;
            return;
        }
        if let SpanKind::Box {
            graded,
            max_tokens,
            tok_size,
            ..
        } = &spec.kind
        {
            // A result box takes its graded style when its position in its history matches.
            let mut spec = spec.clone();
            let mut undegraded = None;
            if let Some(g) = graded.clone()
                && let Some(frame) = self.positions.last_mut()
            {
                let (n, i) = *frame;
                frame.1 += 1;
                if graded_match(i, n, g.k, g.s) {
                    undegraded = Some(tok_size.or_else(|| max_tokens.and_then(|m| self.resolve_size(m))));
                    spec = Rc::new(SpanSpec {
                        label: spec.label.clone(),
                        kind: g.kind.clone(),
                        meta: spec.meta.clone(),
                    });
                }
            }
            // A box is a hard tokenization boundary: its content is encoded on its own.
            self.flush();
            self.open.push(OpenSpan {
                spec,
                start: self.ids.len(),
                start_mark: None,
                segments: vec![],
                child_spans_start: self.spans.len(),
                inflation: 0,
                seg_inflation: vec![],
                undegraded,
            });
        } else {
            if let SpanKind::Affordance = spec.kind {
                // An action owns whole tokens: affordances are hard tokenization boundaries.
                self.flush();
            }
            // A history under positional rules is recorded until it closes (`replay_graded`).
            let graded_history =
                &*spec.label == "history" && spec.meta.get("graded") == Some(&serde_json::Value::Bool(true));
            let m = self.mark();
            self.open.push(OpenSpan {
                spec,
                start: 0,
                start_mark: Some(m),
                segments: vec![],
                child_spans_start: self.spans.len(),
                inflation: 0,
                seg_inflation: vec![],
                undegraded: None,
            });
            if graded_history {
                self.capture = Some((vec![], 0));
            }
        }
    }
    fn close(&mut self) -> Result<(), String> {
        if let Some((items, depth)) = &mut self.capture {
            if *depth > 0 {
                items.push(Item::Close);
                *depth -= 1;
                return Ok(());
            }
            // The recorded history's own close: lay its subtree out, then close it below.
            self.replay_graded()?;
        }
        if self.hidden_depth > 0 {
            self.hidden_depth -= 1;
            return Ok(());
        }
        let o = self.open.pop().ok_or("close without open")?;
        match &o.spec.kind {
            SpanKind::Box {
                max_tokens,
                tok_size,
                clip,
                pad,
                more,
                drop,
                drop_step,
                scroll,
                ..
            } => {
                self.flush();
                self.resolve_spans();
                let content: Vec<u32> = self.ids.split_off(o.start);
                let n = content.len();
                // A fixed size is the limit; else the cap, if there is one.
                let cap = match tok_size {
                    Some(_) => None,
                    None => max_tokens.and_then(|size| self.resolve_size(size)),
                };
                let limit = tok_size.or(cap);
                // The `more` marker for `hidden` hidden tokens (`{hidden}` and `{id}` filled in),
                // never longer than the limit.
                let marker = |this: &Self, hidden: usize, lim: usize| -> Vec<u32> {
                    let Some(m) = more.as_ref() else { return vec![] };
                    let text = m
                        .replace("{hidden}", &hidden.to_string())
                        .replace("{id}", &o.spec.label);
                    let mut ids = this.enc.encode_run(&text, false, this.safe);
                    ids.truncate(lim);
                    ids
                };
                let mut dropped_items = 0usize;
                let mut scroll_range = None;
                let drop_box = *drop != Drop::None;
                // A drop box measures its items undegraded: what graded children (elided results)
                // took off is added back, so grading never moves the cut.
                let n_measured = if drop_box {
                    (n as i64 + o.inflation).max(0) as usize
                } else {
                    n
                };
                // What the box shows of its content and the room its marker gets: the arithmetic
                // is `fit.rs`, which is proved to stay within the limit (docs/VERIFICATION.md).
                let room = crate::fit::room(&content, n_measured, *tok_size, cap, scroll.is_some());
                let over = room.over();
                // The marker's room is reserved at its longest (the most hidden tokens it can name).
                let reserve = over.map_or(0, |lim| marker(self, n, lim).len());
                let fit = match (over, scroll) {
                    (None, _) => room.whole(),
                    (Some(_), Some(viewport)) => {
                        let (fit, range) = room.scroll(reserve, viewport.offset);
                        scroll_range = range;
                        fit
                    }
                    (Some(lim), None) if drop_box => {
                        // Drop whole items from the drop side; keep at least one (drop-children:
                        // none), clipped if it alone is too big.
                        let mut bounds: Vec<(usize, bool, i64)> = vec![(0, false, 0)];
                        bounds.extend(
                            o.segments
                                .iter()
                                .zip(&o.seg_inflation)
                                .map(|(&m, &f)| {
                                    let (b, s) = self.marks[m];
                                    (b - o.start, s, f)
                                })
                                .filter(|&(b, _, _)| b > 0 && b < n),
                        );
                        bounds.push((n, false, o.inflation));
                        bounds.dedup_by_key(|b| b.0);
                        let offsets: Vec<usize> = bounds.iter().map(|b| b.0).collect();
                        let measured: Vec<usize> = bounds.iter().map(|b| (b.0 as i64 + b.2).max(0) as usize).collect();
                        let step = drop_step.map_or(0, |s| s.tokens(lim));
                        let (fit, lo, hi) = room.drop(&offsets, &measured, reserve, *drop, step);
                        dropped_items = offsets.len() - 1 - (hi - lo);
                        if bounds[lo].1 || bounds[hi].1 {
                            return Err(format!(
                                "overflow: {} would cut inside a token that two items share; end an item at a token boundary (e.g. a newline or a special token)",
                                drop.name()
                            ));
                        }
                        fit
                    }
                    (Some(_), None) => room.clip(reserve, *clip),
                };
                let hidden = n - fit.shown_tokens();
                let more_ids = if hidden > 0 {
                    marker(self, hidden, limit.unwrap_or(usize::MAX))
                } else {
                    vec![]
                };
                // The box's tokens, and each kept range: (content start, content end, output offset).
                let (out, ranges) = fit.assemble(&more_ids, *pad);
                let start = self.ids.len();
                self.ids.extend(out);
                // Children are located in the pre-crop token coordinates. Prune hidden
                // actions and rebase visible spans to the final prompt before publishing it.
                if hidden > 0 {
                    let children = self.spans.split_off(o.child_spans_start);
                    for mut span in children {
                        let (from, to) = (span.start - start, span.end - start);
                        // A zero-width span at the very end of the content (an action after the
                        // last token) is shown when the content's tail is: it belongs to the end
                        // of the last kept range, which `rebase` (half-open ranges) does not cover.
                        let at_kept_end =
                            ranges
                                .last()
                                .filter(|last| from == to && from == n && last.1 == n)
                                .map(|last| Rebased {
                                    start: last.2 + (n - last.0),
                                    end: last.2 + (n - last.0),
                                    whole: true,
                                });
                        let r = at_kept_end.or_else(|| rebase(&ranges, from, to));
                        if span.meta["projected_tool"] == true && !r.as_ref().is_some_and(|r| r.whole) {
                            return Err("cannot crop an already projected tool; apply the budget inside asChatTools before declaring tools".into());
                        }
                        let Some(r) = r else { continue };
                        // An action whose label was partly cut is unavailable as a whole.
                        if span.kind == "affordance" && !r.whole {
                            continue;
                        }
                        (span.start, span.end) = (start + r.start, start + r.end);
                        self.spans.push(span);
                    }
                }
                // Closing a box that wrote nothing does not put a layout that continues a prompt
                // back at the start (its first text would get the tokenizer's start-of-text form).
                self.started = self.in_prompt || !self.ids.is_empty();
                self.strip_next_ws = self
                    .ids
                    .last()
                    .and_then(|id| self.enc.flags.get(id))
                    .is_some_and(|f| f.rstrip);
                let mut meta = o.spec.meta.clone();
                if let serde_json::Value::Object(m) = &mut meta {
                    m.insert("content_tokens".into(), n.into());
                    m.insert("hidden_tokens".into(), hidden.into());
                    m.insert("overflow".into(), (hidden > 0).into());
                    if *drop != Drop::None {
                        m.insert("dropped_items".into(), dropped_items.into());
                    }
                    if let Some((offset, max, _)) = scroll_range {
                        m.insert("scroll_offset".into(), offset.into());
                        m.insert("scroll_max".into(), max.into());
                    }
                }
                if let (Some(viewport), Some((offset, max, capacity))) = (scroll, scroll_range)
                    && max > 0
                    && capacity > 0
                    && viewport.action.meta.get("handler").is_some()
                {
                    let mut meta = viewport.action.meta.clone();
                    meta["scroll_max"] = max.into();
                    meta["scroll_offset"] = offset.into();
                    meta["content_tokens"] = n.into();
                    self.spans.push(Span {
                        label: viewport.action.label.to_string(),
                        kind: "affordance".into(),
                        start,
                        end: self.ids.len(),
                        straddle: false,
                        meta,
                    });
                }
                self.spans.push(Span {
                    label: o.spec.label.to_string(),
                    kind: "box".into(),
                    start,
                    end: self.ids.len(),
                    straddle: false,
                    meta,
                });
                // A graded result: its enclosing drop box measures it at its static length.
                if let Some(static_limit) = o.undegraded {
                    let undegraded = static_limit.map_or(n, |l| n.min(l)) as i64;
                    let shown = (self.ids.len() - start) as i64;
                    if let Some(d) = self
                        .open
                        .iter_mut()
                        .rev()
                        .find(|d| matches!(d.spec.kind, SpanKind::Box { drop, .. } if drop != Drop::None))
                    {
                        d.inflation += undegraded - shown;
                    }
                }
            }
            kind => {
                debug_assert!(!matches!(kind, SpanKind::Hidden));
                if let SpanKind::Affordance = kind {
                    self.flush();
                }
                let m_end = self.mark();
                // resolve marks lazily: force resolution only when the run flushes
                let kind_s = if matches!(kind, SpanKind::Affordance) {
                    "affordance"
                } else {
                    "label"
                };
                self.spans.push(Span {
                    label: o.spec.label.to_string(),
                    kind: kind_s.into(),
                    start: o.start_mark.unwrap(),
                    end: m_end,
                    straddle: false,
                    meta: o.spec.meta.clone(),
                });
                // start/end currently hold mark ids; fixed up in resolve_spans().
                let idx = self.spans.len() - 1;
                self.unresolved.push(idx);
            }
        }
        Ok(())
    }
}

/// Short name of a special token for the `tok` namespace: `<|im_start|>` -> `im_start`,
/// `<｜User｜>` -> `User`, `<｜tool▁sep｜>` -> `tool_sep`, `[INST]` -> `INST`, `[/INST]` -> `INST_end`,
/// `</s>` -> `s_end`. None if nothing usable is left.
pub fn token_alias(s: &str) -> Option<String> {
    let mut t = s;
    for (a, b) in [("<|", "|>"), ("<｜", "｜>"), ("[", "]"), ("<", ">")] {
        if t.starts_with(a) && t.ends_with(b) && t.len() > a.len() + b.len() {
            t = &t[a.len()..t.len() - b.len()];
            break;
        }
    }
    let (t, end) = match t.strip_prefix('/') {
        Some(r) => (r, true),
        None => (t, false),
    };
    let mut out: String = t
        .chars()
        .map(|c| if c.is_alphanumeric() && c.is_ascii() { c } else { '_' })
        .collect();
    while out.contains("__") {
        out = out.replace("__", "_");
    }
    let out = out.trim_matches('_').to_string();
    if out.is_empty() || out.as_bytes()[0].is_ascii_digit() {
        return None;
    }
    Some(if end { format!("{}_end", out) } else { out })
}

// One loaded `Encoder` serves every render thread (`Arc<Encoder>`); keep it shareable.
const _: fn() = || {
    fn shareable<T: Send + Sync>() {}
    shareable::<Encoder>();
};

#[cfg(kani)]
#[path = "emit_proofs.rs"]
mod proofs;

#[cfg(test)]
mod drop_cut_tests {
    use super::{Drop, drop_cut};

    /// A deterministic generator (xorshift), so a failure names its seed.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self, n: u64) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0 % n
        }
    }

    /// Over random boxes: the cut keeps whole items from the right side, fits whenever the item
    /// minimum allows, and is maximal (keeping one more item would not fit).
    #[test]
    fn the_cut_is_the_largest_fitting_run_on_the_kept_side() {
        for seed in 1..20_000u64 {
            let mut r = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
            let items = 1 + r.next(12) as usize;
            let mut bounds = vec![0];
            for _ in 0..items {
                let last = *bounds.last().unwrap();
                bounds.push(last + 1 + r.next(40) as usize);
            }
            let (lim, more_len) = (r.next(200) as usize, r.next(8) as usize);
            for drop in [Drop::Start, Drop::End, Drop::Children] {
                let (lo, hi) = drop_cut(&bounds, lim, more_len, drop, 0);
                let ctx = format!("seed {seed} {drop:?} bounds {bounds:?} lim {lim} more {more_len} -> ({lo}, {hi})");
                let min = if drop == Drop::Children { 0 } else { 1 };
                assert!(lo <= hi && hi < bounds.len() && hi - lo >= min.min(items), "{ctx}");
                match drop {
                    Drop::End => assert_eq!(lo, 0, "{ctx}"),
                    _ => assert_eq!(hi, items, "{ctx}"),
                }
                let fits = |lo: usize, hi: usize| bounds[hi] - bounds[lo] + more_len <= lim;
                // It fits, or it is down to the minimum number of items.
                assert!(fits(lo, hi) || hi - lo == min, "{ctx}");
                // Maximal: keeping the next dropped item would not fit.
                if drop == Drop::End && hi < items {
                    assert!(!fits(lo, hi + 1), "{ctx}");
                }
                if drop != Drop::End && lo > 0 {
                    assert!(!fits(lo - 1, hi), "{ctx}");
                }
            }
        }
    }

    /// `graded_match`: the matched results are a prefix (oldest first), the newest K-1 never
    /// match, and as results are appended the prefix only grows, S at a time, once every S.
    #[test]
    fn graded_results_grow_in_steps() {
        use super::graded_match;
        for k in 1..12 {
            for s in 1..9 {
                let mut prev = 0usize;
                for n in 0..60 {
                    let m = (0..n).filter(|&i| graded_match(i, n, k, s)).count();
                    assert!((0..m).all(|i| graded_match(i, n, k, s)), "prefix k={k} s={s} n={n}");
                    assert!(n < k || n - m >= k - 1, "the newest K-1 stay k={k} s={s} n={n} m={m}");
                    assert!(m >= prev, "forward only");
                    assert!(
                        m == prev || m - prev == s || (prev == 0 && m == 1),
                        "moves by S k={k} s={s} n={n}: {prev} -> {m}"
                    );
                    prev = m;
                }
            }
        }
    }

    /// With a step, the amount dropped is the first boundary at or past a multiple of the step,
    /// and as the content grows on the far side the cut only moves forward, once per step.
    #[test]
    fn a_stepped_cut_is_stable_as_content_grows() {
        for seed in 1..5_000u64 {
            let mut r = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
            let lim = 40 + r.next(300) as usize; // above the largest item, so the cut is never forced to the last one
            // A step leaves at least `lim - step` kept: room for the largest item (30 tokens).
            let step = 1 + r.next((lim - 31) as u64) as usize;
            let mut bounds = vec![0];
            let mut cuts = vec![];
            for _ in 0..(5 + r.next(60)) {
                let last = *bounds.last().unwrap();
                bounds.push(last + 1 + r.next(30) as usize);
                let n = *bounds.last().unwrap();
                let (lo, hi) = drop_cut(&bounds, lim, 0, Drop::Start, step);
                let ctx = format!("seed {seed} step {step} lim {lim} bounds {bounds:?} -> {lo}");
                assert_eq!(hi, bounds.len() - 1, "{ctx}");
                if n > lim {
                    let need = (n - lim).div_ceil(step) * step;
                    // The cut is at the first boundary at or past the stepped amount (or at the last item).
                    assert!(bounds[lo] >= need || lo == bounds.len() - 2, "{ctx}");
                    if lo > 0 {
                        assert!(bounds[lo - 1] < need, "{ctx}");
                    }
                    cuts.push(bounds[lo]);
                }
            }
            // Forward only; and between two moves at least one step of new content arrived.
            assert!(cuts.windows(2).all(|w| w[0] <= w[1]), "seed {seed}: {cuts:?}");
            let moves = cuts.windows(2).filter(|w| w[0] != w[1]).count();
            let grown = bounds.last().unwrap() - lim.min(*bounds.last().unwrap());
            assert!(
                moves <= grown / step + 1,
                "seed {seed}: {moves} moves for {grown} tokens of growth, step {step}"
            );
        }
    }
}

#[cfg(test)]
mod live_tests {
    use super::*;
    use serde_json::json;

    #[test]
    #[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
    fn native_ids_and_spans_are_appended_without_reencoding() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ref/tokenizers/Qwen__Qwen3-4B-Instruct-2507.json");
        let e = Encoder::from_file(path.to_str().unwrap()).unwrap();
        let mut cache = RunCache::new(false);
        let mut live = Live::new(&e, &mut cache);
        live.text("prefix");
        let prefix = e.encode_run("prefix", true, false);
        let ids: Rc<[u32]> = vec![0, 1, 2, 3].into();
        let spans: Rc<[Span]> = vec![Span {
            label: "native".into(),
            kind: "label".into(),
            start: 1,
            end: 3,
            straddle: false,
            meta: json!({}),
        }]
        .into();
        live.tokens(&ids, &spans).unwrap();
        live.text("suffix");
        let (got, spans) = live.finish().unwrap();
        assert_eq!(
            got,
            [prefix.clone(), ids.to_vec(), e.encode_run("suffix", false, false)].concat()
        );
        assert_eq!((spans[0].start, spans[0].end), (prefix.len() + 1, prefix.len() + 3));
    }
}
