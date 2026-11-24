//! Affordance renders state into exact token ids for a model, as a web browser renders a page
//! into pixels for a person.
//!
//! An app is a TSX view of its state ([`View`]). Rendering lays it out against a token budget
//! and produces the prompt's token ids and spans ([`Rendered`]): which ids are text, which are
//! the actions (`<aff>`) still visible after layout, and which handler each action runs. A reply
//! names an action; [`View::act`] runs its handler and returns the next state.
//!
//! The [`runtime`] runs many apps as agents: an event loop per instance, model and machine
//! bindings, a journal per agent that replays it exactly, and recovery images.
//!
//! ```no_run
//! use affordance::{View, emit::{Encoder, RunCache}, value::state};
//! use serde_json::json;
//!
//! let encoder = Encoder::from_file("tokenizer.json")?;
//! let view = View::load("views/demo/hello.tsx".as_ref(), "Hello", &encoder)?;
//! let now = chrono::Utc::now().naive_utc();
//! let rendered = view.render(&encoder, state(&json!({"name": "world"})), &mut RunCache::new(true), now)?;
//! println!("{:?}", rendered.ids);
//! # Ok::<(), String>(())
//! ```
//!
//! The language is specified in `spec/LANGUAGE.md`, the runtime in `spec/HARNESS.md`.
//!
//! # What this crate is
//!
//! Affordance is first a command, `affordance`, that runs apps written in TSX: most users never
//! write Rust. This library is for embedding the engine in another program. The supported
//! surface is small: [`View`] and [`emit::Encoder`] to render, and [`runtime`] ([`runtime::Runtime`],
//! [`runtime::RuntimeConfig`], [`runtime::Model`], [`runtime::Executor`]) to run agents with your
//! own model servers and machines. Benchmarks also use [`profile`]; [`scenario`] and [`lint`] are
//! the engines of `affordance test` and `affordance lint`. Everything else is private to the
//! crate. Before 1.0 the Rust API is not stable. What is: the language and runtime specs, and the
//! journal contract a run pins.

mod adapter;
mod ast;
mod bytecode;
pub mod chat_render;
pub mod connectors;
pub mod console;
pub mod effects;
pub mod emit;
mod fasttok;
mod fit;
mod heap;
mod interp;
mod intrinsic;
mod jsbuiltins;
mod jsmath;
mod jsregex;
mod jsvalue;
pub mod library;
pub mod lint;
mod memo;
mod microtasks;
mod migrate;
mod modules;
mod observe;
mod parser;
pub mod profile;
// The library's own tests allocate with mimalloc (a test-only dependency: Cargo.toml says why).
// The library sets no allocator.
#[cfg(test)]
#[global_allocator]
static TEST_ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

pub mod media;
pub mod media_kinds;
pub mod reply;
mod reply_formats;
pub mod reply_pattern;
pub mod runtime;
pub mod scenario;
mod schema;
mod style;
#[cfg(test)]
mod test_support;
mod tsx;
mod turns;
pub mod value;
mod web;

use emit::{Encoder, Live, RunCache, Span};
use interp::{Interp, Program};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use value::{Map, Value};

/// A loaded app: its program, bound to a tokenizer's special tokens.
pub struct View {
    pub(crate) prog: Program,
    specials: Rc<Map>,
    /// `tok`: the bound tokenizer's special tokens as first-class values, by short name and by string.
    tok_ns: Value,
    /// Native subtree runs share this encoder-bound cache across render/act calls.
    projection_cache: RefCell<RunCache>,
    /// SHA256 over relative source paths and contents: the app identity pinned by journals.
    /// Moving the source tree preserves it; changing which file supplies code does not.
    pub source_hash: String,
    /// The `harness` a render outside a run reads (`for_model`): the model binding's choice.
    model: Option<Value>,
}

/// An affordance the harness invokes, not a tool the model calls: timers, event callbacks, the
/// context budget, a Decision's choice, and a mounted child's callbacks (`mount:<key>:<prop>`).
/// A span the model may call: an action with a handler that is not one of the harness's own
/// callbacks. The one statement of that rule, for the tool table, the turn plan and reply checks.
pub(crate) fn is_model_action(span: &emit::Span) -> bool {
    span.kind == "affordance" && span.meta.get("handler").is_some() && !is_harness_callback(&span.label)
}

pub fn is_harness_callback(label: &str) -> bool {
    matches!(label, "timeout" | "interval" | "callback" | "choose") || label.starts_with("mount:")
}

/// Lowercase hex sha256 of `bytes`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    sha2::Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// A laid-out render: the prompt's token ids and what each range of them is.
#[derive(Clone)]
pub struct Rendered {
    /// The token ids, exactly as the model reads them.
    pub ids: Vec<u32>,
    /// Labelled ranges of `ids`: text spans, the visible actions and their handlers, protocol turns.
    pub spans: Vec<Span>,
    /// Local hook/viewport state first used in this render, with initial values (see
    /// `Interp::mounted_hooks`); an agent stores them in its state's `__ui`.
    pub mounted: Vec<(String, Value)>,
}

impl Rendered {
    /// The current screen's token range in a template-app render (`TurnStrategy`'s final turn,
    /// span label `turn:current`): the body only, without the template's role tokens.
    pub fn current_turn(&self) -> Option<std::ops::Range<usize>> {
        self.spans
            .iter()
            .find(|s| s.label == turns::CURRENT_TURN)
            .map(|s| s.start..s.end)
    }

    /// The current screen's exact ids. Log them as `{"kind": "screen", "ids": [...]}` so the
    /// next render re-emits these ids verbatim (no decode, no re-tokenization).
    pub fn current_turn_ids(&self) -> Option<&[u32]> {
        self.current_turn().map(|r| &self.ids[r])
    }
}

impl View {
    /// The component this view renders (its entry point).
    pub fn entry(&self) -> &str {
        &self.prog.entry
    }

    /// Load a view file (and its imports) bound to a tokenizer. `entry` is the root component.
    pub fn load(path: &Path, entry: &str, enc: &Encoder) -> Result<View, String> {
        Self::load_in(path, entry, enc, vec![], vec![])
    }

    /// Load a view for one model format outside a run: `load`, then `for_model`, with the
    /// template made available to the app as a deployment's binding makes it.
    pub fn load_model(
        path: &Path,
        entry: &str,
        enc: &Encoder,
        template: &str,
        options: &serde_json::Value,
    ) -> Result<View, String> {
        Ok(Self::load_in(path, entry, enc, vec![], vec![template.to_string()])?.for_model(template, options))
    }

    /// Load an app the runtime serves: its modules (imports, style sheets, the app itself) must
    /// resolve, symlinks followed, inside the app's own directory or one of `library`'s. A page
    /// imports from its own origin and what its import map allows; an app cannot reach other files
    /// on the host (`../../shared/x.tsx`, an absolute path, a symlink out).
    pub fn load_within(path: &Path, entry: &str, enc: &Encoder, library: &[PathBuf]) -> Result<View, String> {
        Self::load_for(path, entry, enc, library, &[])
    }

    /// `load_within`, for a deployment whose model bindings name `templates`. A template that
    /// names a registered model (`org/name`, a key of the library's `index.json`) is made
    /// available to the app (`ModelContext`) from that model's format; a template app's name
    /// (`qwen3`, `messages`, ..) needs no mention here.
    pub fn load_for(
        path: &Path,
        entry: &str,
        enc: &Encoder,
        library: &[PathBuf],
        templates: &[String],
    ) -> Result<View, String> {
        let own = path.canonicalize().map_err(|e| format!("{}: {e}", path.display()))?;
        let mut roots = vec![own.parent().ok_or("view has no parent directory")?.to_path_buf()];
        for l in library {
            roots.push(l.canonicalize().map_err(|e| format!("library {}: {e}", l.display()))?);
        }
        Self::load_in(path, entry, enc, roots, templates.to_vec())
    }

    fn load_in(
        path: &Path,
        entry: &str,
        enc: &Encoder,
        roots: Vec<PathBuf>,
        templates: Vec<String>,
    ) -> Result<View, String> {
        let mut prog = Program {
            funcs: HashMap::new(),
            consts: vec![],
            entry: Rc::from(entry),
            root: Rc::from(""),
            modules: HashMap::new(),
            globals: HashMap::new(),
            pragmas: vec![],
            styles: vec![],
            roots,
            templates,
        };
        let mut seen = HashSet::new();
        web::install(&mut prog)?;
        interp::compiler::install(&mut prog)?;
        modules::load(&mut prog, path, enc, &mut seen, &mut vec![])?;
        let base = path.canonicalize().map_err(|e| e.to_string())?;
        let base = base.parent().ok_or("view has no parent directory")?;
        let mut files: Vec<String> = seen
            .iter()
            .map(|f| {
                let relative = relative_source_path(base, f);
                std::fs::read(f)
                    .map(|b| format!("{}:{}:{}", relative.len(), relative, sha256_hex(&b)))
                    .map_err(|e| format!("{}: {e}", f.display()))
            })
            .collect::<Result<_, _>>()?;
        files.sort();
        let source_hash = sha256_hex(format!("{entry}\n{}", files.join("\n")).as_bytes());
        let mut sp = Map::default();
        for (s, id) in &enc.special_strings {
            sp.insert(Rc::from(s.as_str()), Value::Int(*id as i64));
        }
        let mut ns = Map::default();
        for (s, id) in &enc.special_strings {
            ns.insert(Rc::from(s.as_str()), Value::Token(*id, Rc::from(s.as_str())));
        }
        let mut aliased: HashSet<String> = HashSet::new();
        for (s, id) in enc.special_strings.iter().rev() {
            if let Some(a) = emit::token_alias(s)
                && aliased.insert(a.clone())
            {
                ns.entry(Rc::from(a.as_str()))
                    .or_insert(Value::Token(*id, Rc::from(s.as_str())));
            }
        }
        let tok_ns = Value::map(ns);
        value::freeze(&tok_ns);
        Ok(View {
            prog,
            specials: Rc::new(sp),
            tok_ns,
            projection_cache: RefCell::new(RunCache::new(false)),
            source_hash,
            model: None,
        })
    }

    /// This view, rendered for one model format outside a run: `template` names a template app
    /// (`qwen36`, `llama31`, `messages`, ..) and `options` are its options, as a deployment's
    /// model binding states them. The app reads the choice where it does in a run
    /// (`harness.model`, which `ModelContext` follows), so one app file serves every model.
    /// `render` and `act` use it; a call that passes its own `harness` uses that. The view was
    /// loaded with this template among its deployment's (`load_for`, or `load_model` for both).
    pub fn for_model(mut self, template: &str, options: &serde_json::Value) -> Self {
        let harness =
            serde_json::json!({"model": {"name": "default", "template": template, "template_options": options}});
        let harness = Value::from_json(&harness);
        value::freeze(&harness);
        self.model = Some(harness);
        self
    }

    pub(crate) fn checkpoint_context(&self) -> (&Value, Rc<RefCell<crate::memo::MemoStore>>) {
        (&self.tok_ns, self.projection_cache.borrow().memo.clone())
    }

    /// Diagnostic counters for native app encoding, separate from the caller's
    /// outer cache. Render honors `RunCache.enabled`; action dispatch reuses runs.
    pub fn projection_cache_stats(&self) -> (usize, usize) {
        let cache = self.projection_cache.borrow();
        (cache.hits, cache.misses)
    }

    /// Render `state` to token ids, laid out with the view's defaults. `now` is the clock the
    /// render reads (`Date`).
    pub fn render(
        &self,
        enc: &Encoder,
        state: Rc<Map>,
        cache: &mut RunCache,
        now: chrono::NaiveDateTime,
    ) -> Result<Rendered, String> {
        self.render_opts(enc, state, cache, now, false)
    }

    /// `piecewise`: diagnostic mode, every text piece is encoded on its own.
    pub fn render_opts(
        &self,
        enc: &Encoder,
        state: Rc<Map>,
        cache: &mut RunCache,
        now: chrono::NaiveDateTime,
        piecewise: bool,
    ) -> Result<Rendered, String> {
        let mut it = Interp::new(&self.prog, state, self.specials.clone(), now)?;
        it.bind_encoder(enc);
        it.bind_projection_cache(&self.projection_cache, cache.enabled);
        // A cache that keeps things between renders keeps `memoScan`'s scans too, as an agent's
        // renders do: a caller that renders a growing conversation with one cache, passing the
        // same values for the messages it has passed before, pays for what is new.
        if cache.enabled {
            it.bind_memo(cache.memo.clone());
        }
        it.set_const("tok", self.tok_ns.clone());
        if let Some(harness) = &self.model {
            it.set_const("harness", harness.clone());
        }
        let mut live = Live::new(enc, cache);
        live.piecewise = piecewise;
        live.safe = self.prog.pragmas.iter().any(|p| p == "safe_text");
        it.render(&mut live)?;
        let (ids, spans) = live.finish()?;
        Ok(Rendered {
            ids,
            spans,
            mounted: vec![],
        })
    }

    /// Chat-template compatibility entry point. HF's wrapper validates and normalizes
    /// tools before executing a template. Ordinary app state has no such restriction.
    pub fn render_chat_opts(
        &self,
        enc: &Encoder,
        state: Rc<Map>,
        cache: &mut RunCache,
        now: chrono::NaiveDateTime,
        piecewise: bool,
    ) -> Result<Rendered, String> {
        let state = prepare_chat_state(state)?;
        self.render_opts(enc, state, cache, now, piecewise)
    }
}

fn relative_source_path(base: &Path, file: &Path) -> String {
    let base: Vec<_> = base.components().collect();
    let file: Vec<_> = file.components().collect();
    let common = base.iter().zip(&file).take_while(|(a, b)| a == b).count();
    let mut path = std::path::PathBuf::new();
    for _ in common..base.len() {
        path.push("..");
    }
    for part in &file[common..] {
        path.push(part.as_os_str());
    }
    path.to_string_lossy().replace('\\', "/")
}

/// Result of an action: the next state and the render it was chosen from.
pub struct Acted {
    /// The state after the action's handler ran.
    pub state: serde_json::Value,
    /// The render the action was chosen from.
    pub before: Rendered,
}

/// Independent actions selected from the same public state and render.
pub struct ActedBatch {
    /// The state after each action, in order.
    pub states: Vec<serde_json::Value>,
    /// The render the actions were chosen from.
    pub before: Rendered,
}

impl View {
    /// The view -> decision -> next view step: render `state`, find the affordance named
    /// `action` (the n-th one if several, `which`), run its handler with `args`, and return the
    /// next state. A handler may call `useState` setters (stored in `state.__ui`) and/or return a
    /// dict that is merged into the state.
    pub fn act(
        &self,
        enc: &Encoder,
        state: Rc<Map>,
        now: chrono::NaiveDateTime,
        action: &str,
        which: usize,
        args: &serde_json::Value,
    ) -> Result<Acted, String> {
        let mut batch = self.act_independent_many(enc, state, now, &[(action, which, args.clone())])?;
        Ok(Acted {
            state: batch.states.remove(0),
            before: batch.before,
        })
    }

    /// Render once, then execute actual handlers in separate interpreter
    /// forks. Every action starts from `state`; none consumes an earlier
    /// action's returned patch or useState updates.
    pub fn act_independent_many(
        &self,
        enc: &Encoder,
        state: Rc<Map>,
        now: chrono::NaiveDateTime,
        actions: &[(&str, usize, serde_json::Value)],
    ) -> Result<ActedBatch, String> {
        if !(1..=64).contains(&actions.len()) {
            return Err("independent action batch must contain 1 to 64 actions".into());
        }
        let mut it = Interp::new(&self.prog, state.clone(), self.specials.clone(), now)?;
        it.bind_encoder(enc);
        it.bind_projection_cache(&self.projection_cache, true);
        it.set_const("tok", self.tok_ns.clone());
        if let Some(harness) = &self.model {
            it.set_const("harness", harness.clone());
        }
        let mut cache = RunCache::new(false);
        let mut live = Live::new(enc, &mut cache);
        live.safe = self.prog.pragmas.iter().any(|p| p == "safe_text");
        it.render(&mut live)?;
        let (ids, spans) = live.finish()?;
        let handlers = actions
            .iter()
            .map(|(action, which, args)| {
                let span = find_affordance(&spans, action, *which)?;
                if let Some(req) = span.meta.get("required_params").and_then(|r| r.as_array()) {
                    let missing: Vec<&str> = req
                        .iter()
                        .filter_map(|n| n.as_str())
                        .filter(|n| args.get(*n).is_none_or(|v| v.is_null()))
                        .collect();
                    if !missing.is_empty() {
                        return Err(format!(
                            "missing required argument{} {} for `{}`",
                            if missing.len() > 1 { "s" } else { "" },
                            missing.iter().map(|m| format!("`{m}`")).collect::<Vec<_>>().join(", "),
                            action
                        ));
                    }
                }
                Ok((bind_handler(&it, span)?, args))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let mut states = Vec::with_capacity(actions.len());
        for (handler, args) in handlers {
            let mut action_it = it.fork_for_action();
            let patch = action_it.call_handler(&handler, vec![Value::from_json(args)])?;
            let mut next = (*state).clone();
            if let Value::Map(p) = &patch {
                for (k, v) in p.read().iter() {
                    next.insert(k.clone(), v.clone());
                }
            }
            if !action_it.ui_updates.is_empty() {
                let mut ui = match next.get("__ui") {
                    Some(Value::Map(m)) => m.read().clone(),
                    _ => Map::default(),
                };
                for (k, v) in action_it.ui_updates.iter() {
                    ui.insert(k.clone(), v.clone());
                }
                next.insert(Rc::from("__ui"), Value::map(ui));
            }
            states.push(Value::map(next).to_wire());
        }
        Ok(ActedBatch {
            states,
            before: Rendered {
                ids,
                spans,
                mounted: vec![],
            },
        })
    }
}

/// Build the template state from a case: messages, tools, kwargs, special-token strings.
/// [`state_from_json`] for a case read with [`value::parse_json_exact`]: integers past 64 bits
/// in its messages, tools and arguments are the numbers, with all their digits.
pub fn state_from_json_exact(case: &serde_json::Value, special_map: &serde_json::Value) -> Rc<Map> {
    let mut m = (*state_from_json(case, special_map)).clone();
    if let Some(serde_json::Value::Object(kw)) = case.get("kwargs") {
        for (k, v) in kw {
            m.insert(Rc::from(k.as_str()), Value::from_json_exact(v));
        }
    }
    if let Some(messages) = case.get("messages") {
        m.insert(Rc::from("messages"), Value::from_json_exact(messages));
    }
    if let Some(tools) = case.get("tools").filter(|t| !t.is_null()) {
        m.insert(Rc::from("tools"), Value::from_json_exact(tools));
    }
    Rc::new(m)
}

pub fn state_from_json(case: &serde_json::Value, special_map: &serde_json::Value) -> Rc<Map> {
    let mut m = Map::default();
    if let serde_json::Value::Object(o) = special_map {
        for (k, v) in o {
            m.insert(Rc::from(k.as_str()), Value::from_json(v));
        }
    }
    if let Some(serde_json::Value::Object(kw)) = case.get("kwargs") {
        for (k, v) in kw {
            m.insert(Rc::from(k.as_str()), Value::from_json(v));
        }
    }
    m.insert(
        Rc::from("messages"),
        Value::from_json(case.get("messages").unwrap_or(&serde_json::Value::Null)),
    );
    match case.get("tools") {
        Some(t) if !t.is_null() => {
            m.insert(Rc::from("tools"), Value::from_json(t));
        }
        _ => {
            m.insert(Rc::from("tools"), Value::None);
        }
    }
    Rc::new(m)
}

/// The `transformers` `apply_chat_template` wrapper validates `tools` before running the
/// template: every tool must be a JSON schema (mapping) or a callable. A mapping iterates to
/// its string keys, so it fails too. Reproduce that check here, before interpretation, as a
/// `raise:`-kind error so the view raises at the same point as the official path.
pub const TOOLS_SCHEMA_ERROR: &str = "Tools should either be a JSON schema, or a callable function with type hints and a docstring suitable for auto-conversion to a schema.";

/// Hugging Face's `apply_chat_template` refuses an empty conversation before any template runs.
pub const EMPTY_CONVERSATION_ERROR: &str =
    "Cannot apply chat template to an empty conversation. Provide at least one message.";

fn prepare_chat_state(state: Rc<Map>) -> Result<Rc<Map>, String> {
    if matches!(state.get("messages"), Some(Value::List(m)) if m.read().is_empty()) {
        return Err(format!("raise: {EMPTY_CONVERSATION_ERROR}"));
    }
    let Some(tools) = state.get("tools") else {
        return Ok(state);
    };
    if matches!(tools, Value::None | Value::Undef) {
        return Ok(state);
    }
    // Python iteration: a list's items, a string's characters, a mapping's keys.
    let schemas = match tools {
        Value::Map(m) => m.read().keys().map(|k| Value::Str(k.into())).collect(),
        Value::List(_) | Value::Str(_) => interp::iterate(tools)?,
        other => return Err(format!("TypeError: '{}' object is not iterable", other.type_name())),
    };
    if !schemas.iter().all(|t| matches!(t, Value::Map(_))) {
        return Err(format!("raise: {}", TOOLS_SCHEMA_ERROR));
    }
    if matches!(tools, Value::List(_)) {
        return Ok(state);
    }
    let mut prepared = (*state).clone();
    prepared.insert(Rc::from("tools"), Value::list(schemas));
    Ok(Rc::new(prepared))
}

/// A handler has completed or parked, with its complete effect transcript and only its writes.
#[derive(Debug)]
pub struct EffectAction {
    /// The handler returned (false: it is parked on an effect).
    pub done: bool,
    /// The handler's return value (`null` while parked). Window apps return a state patch
    /// (merged into `writes`); agent-runtime tools return their tool result.
    pub value: serde_json::Value,
    /// The state fields the handler wrote.
    pub writes: serde_json::Value,
    /// Every effect the handler requested, in order.
    pub effects: Vec<effects::Effect>,
    /// The indices of `effects` still awaiting a result.
    pub blocked: Vec<usize>,
    /// The return value as a value (an effect setup's cleanup function).
    pub returned: Value,
}

impl View {
    /// Re-render the original snapshot and reconstruct one execution, delivering recorded
    /// completions in arrival order. Every request must equal its transcript entry.
    pub fn act_effects(
        &self,
        enc: &Encoder,
        state: Rc<Map>,
        now: chrono::NaiveDateTime,
        action: &str,
        which: usize,
        args: &serde_json::Value,
        harness: &serde_json::Value,
        transcript: Vec<effects::Effect>,
    ) -> Result<EffectAction, String> {
        self.prepare_handler(enc, state, now, action, which, harness)?
            .run(args, transcript)
    }

    /// Render a handler's start snapshot once and find the handler. `run` reconstructs execution
    /// against an arrival-ordered transcript. Use `runtime::Instance` to retain live continuations.
    pub fn prepare_handler<'v>(
        &'v self,
        enc: &'v Encoder,
        state: Rc<Map>,
        now: chrono::NaiveDateTime,
        action: &str,
        which: usize,
        harness: &serde_json::Value,
    ) -> Result<PreparedHandler<'v>, String> {
        let mut it = Interp::new(&self.prog, state, self.specials.clone(), now)?;
        it.bind_encoder(enc);
        it.bind_projection_cache(&self.projection_cache, true);
        it.set_const("tok", self.tok_ns.clone());
        it.set_const("harness", Value::from_json(harness));
        if let Some(owner) = harness["owner"].as_str() {
            it.bind_hook_namespace(owner);
        }
        let mut cache = RunCache::new(false);
        let mut live = Live::new(enc, &mut cache);
        live.safe = self.prog.pragmas.iter().any(|p| p == "safe_text");
        it.render(&mut live)?;
        let (_, spans) = live.finish()?;
        let handler = find_handler(&it, &spans, action, which)?;
        Ok(PreparedHandler { it, handler })
    }

    /// `render_with_harness`, keeping the render's handlers (`Prepared`) to run any of them later:
    /// `evaluate`, then `layout`.
    pub fn render_prepared(
        &self,
        enc: &Encoder,
        state: Rc<Map>,
        cache: &mut RunCache,
        now: chrono::NaiveDateTime,
        harness: Value,
        window: Option<emit::Window>,
    ) -> Result<(Rendered, Rc<Prepared>), String> {
        let prepared = self.evaluate(enc, state, cache, now, harness, window)?;
        let r = self.layout(enc, &prepared.doc, cache)?;
        Ok((
            Rendered {
                ids: r.ids.clone(),
                spans: r.spans.clone(),
                mounted: prepared.doc.mounted.clone(),
            },
            prepared,
        ))
    }

    /// The document evaluated (handlers, effects, mounts, hooks, listeners) but not laid out:
    /// what an effects pass or a reconcile reads. Layout
    /// (`layout`) is what a prompt or a model-chosen action needs.
    pub fn evaluate(
        &self,
        enc: &Encoder,
        state: Rc<Map>,
        cache: &RunCache,
        now: chrono::NaiveDateTime,
        harness: Value,
        window: Option<emit::Window>,
    ) -> Result<Rc<Prepared>, String> {
        let mut it = Interp::new(&self.prog, state, self.specials.clone(), now)?;
        it.bind_encoder(enc);
        it.bind_window(window);
        it.bind_projection_cache(&self.projection_cache, cache.enabled);
        if cache.enabled {
            it.bind_memo(cache.memo.clone());
        }
        it.set_const("tok", self.tok_ns.clone());
        if let Ok(Value::Str(owner)) = interp::get_attr(&harness, "owner") {
            it.bind_hook_namespace(&owner);
        }
        it.set_const("harness", harness);
        let (root, tail) = it.evaluate()?;
        let effects = std::mem::take(&mut it.effect_decls)
            .into_iter()
            .map(|(k, deps, setup)| {
                value::freeze(&setup);
                value::freeze(&deps);
                (k, deps, setup)
            })
            .collect();
        let listeners = listeners(&root, &tail);
        let doc = Evaluated {
            root,
            tail,
            safe: self.prog.pragmas.iter().any(|p| p == "safe_text"),
            window,
            mounted: it.mounted_hooks()?,
            laid_out: std::cell::OnceCell::new(),
        };
        Ok(Rc::new(Prepared {
            mounts: std::mem::take(&mut it.mount_decls),
            snap: it.snapshot()?,
            doc: Rc::new(doc),
            listeners: Rc::new(listeners),
            effects,
            window,
        }))
    }

    /// Lay out an evaluated document, once: the result is kept with it, and its affordances
    /// (cropping and scroll viewports are layout's) are what model-chosen actions run from.
    pub fn layout(&self, enc: &Encoder, doc: &Evaluated, cache: &mut RunCache) -> Result<Rc<Rendered>, String> {
        if let Some(r) = doc.laid_out.get() {
            return Ok(r.clone());
        }
        let mut live = Live::new(enc, cache);
        live.window = doc.window.map(emit::Window::tokens);
        live.safe = doc.safe;
        interp::emit_child(&doc.root, &mut live)?;
        for spec in &doc.tail {
            emit::Out::open(&mut live, spec.clone());
            emit::Out::close(&mut live)?;
        }
        let (ids, spans) = live.finish()?;
        let r = Rc::new(Rendered {
            ids,
            spans,
            mounted: vec![],
        });
        let _ = doc.laid_out.set(r.clone());
        Ok(r)
    }

    /// The handler of the `which`-th affordance named `action` of a kept render, ready to run.
    pub fn prepare_from<'v>(
        &'v self,
        enc: &'v Encoder,
        prepared: &Prepared,
        action: &str,
        which: usize,
    ) -> Result<PreparedHandler<'v>, String> {
        let mut it = Interp::from_snapshot(&self.prog, &prepared.snap);
        it.bind_encoder(enc);
        it.bind_window(prepared.window);
        it.bind_projection_cache(&self.projection_cache, true);
        // Listeners and callbacks come from the evaluated document (a listener in a cropped or
        // hidden box is live); actions a model chose, from the laid-out one.
        let handler = if is_harness_callback(action) {
            find_handler(&it, &prepared.listeners, action, which)?
        } else {
            let laid = prepared
                .doc
                .laid_out
                .get()
                .ok_or("internal: an action's handler needs its render laid out")?;
            find_handler(&it, &laid.spans, action, which)?
        };
        Ok(PreparedHandler { it, handler })
    }

    /// A closure of a kept render (an effect's setup or cleanup), ready to run as a handler.
    pub fn prepare_closure<'v>(
        &'v self,
        enc: &'v Encoder,
        prepared: &Prepared,
        closure: &Value,
    ) -> PreparedHandler<'v> {
        let mut it = Interp::from_snapshot(&self.prog, &prepared.snap);
        it.bind_encoder(enc);
        it.bind_window(prepared.window);
        it.bind_projection_cache(&self.projection_cache, true);
        PreparedHandler {
            it,
            handler: closure.clone(),
        }
    }

    /// `render_window` with the `harness` context as a value.
    pub fn render_with_harness(
        &self,
        enc: &Encoder,
        state: Rc<Map>,
        cache: &mut RunCache,
        now: chrono::NaiveDateTime,
        harness: Value,
    ) -> Result<Rendered, String> {
        self.render_prepared(enc, state, cache, now, harness, None)
            .map(|(r, _)| r)
    }
}

/// An evaluated document before layout (`View::evaluate`): its root value and the zero-token
/// spans after it, and the hooks it used first. Laid out at most once (`View::layout`).
pub struct Evaluated {
    root: Value,
    tail: Vec<Rc<emit::SpanSpec>>,
    safe: bool,
    window: Option<emit::Window>,
    /// Hook state first used in this evaluation, with initial values (as `Rendered::mounted`).
    pub mounted: Vec<(String, Value)>,
    laid_out: std::cell::OnceCell<Rc<Rendered>>,
}

impl Evaluated {
    /// Its layout, if it has been laid out.
    pub fn laid_out(&self) -> Option<Rc<Rendered>> {
        self.laid_out.get().cloned()
    }
}

/// The harness callbacks (listeners, `<Mount>` callbacks, timers) of a document, in document
/// order, as affordance spans (positions are not meaningful): its root's items, the screens laid
/// out inside it, and the tail. Hidden and cropped ones included: layout does not remove a
/// listener.
fn listeners(root: &Value, tail: &[Rc<emit::SpanSpec>]) -> Vec<Span> {
    fn span(spec: &emit::SpanSpec) -> Span {
        Span {
            label: spec.label.to_string(),
            kind: "affordance".into(),
            start: 0,
            end: 0,
            straddle: false,
            meta: spec.meta.clone(),
        }
    }
    fn walk(v: &Value, out: &mut Vec<Span>) {
        let Value::Frag(items) = v else { return };
        for it in items.iter() {
            match it {
                emit::Item::Open(spec)
                    if matches!(spec.kind, emit::SpanKind::Affordance) && is_harness_callback(&spec.label) =>
                {
                    out.push(span(spec))
                }
                emit::Item::Tokens(_, spans) => out.extend(
                    spans
                        .iter()
                        .filter(|s| s.kind == "affordance" && is_harness_callback(&s.label))
                        .cloned(),
                ),
                _ => {}
            }
        }
    }
    let mut out = vec![];
    walk(root, &mut out);
    out.extend(tail.iter().filter(|s| is_harness_callback(&s.label)).map(|s| span(s)));
    out
}

/// A completed document's handlers, effects and mounts (see `Interp::snapshot`). Handlers run
/// from it without rendering the view again (`View::prepare_from`).
pub struct Prepared {
    pub(crate) mounts: Vec<serde_json::Value>,
    snap: interp::Snapshot,
    /// The evaluated document (laid out on demand, once).
    pub doc: Rc<Evaluated>,
    /// The listeners and callbacks of the evaluated document, in document order, whatever
    /// layout later crops or hides (`listeners`).
    pub listeners: Rc<Vec<Span>>,
    /// The render's `useEffect` declarations: (identity, deps (frozen), setup).
    pub effects: Vec<(String, Value, Value)>,
    /// The model the render was laid out for: handlers run from it lay out their own prompts
    /// (`model(<View/>)`) for the same window.
    window: Option<emit::Window>,
}

impl std::fmt::Debug for Prepared {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Prepared({} listeners)", self.listeners.len())
    }
}

impl Prepared {
    /// Refresh globals for a new event without mutating a model turn or a running handler.
    pub(crate) fn with_context(&self, harness: Value, now: chrono::NaiveDateTime) -> Rc<Self> {
        Rc::new(Self {
            snap: self.snap.with_context(harness, now),
            mounts: self.mounts.clone(),
            doc: self.doc.clone(),
            listeners: self.listeners.clone(),
            effects: self.effects.clone(),
            window: self.window,
        })
    }
}

/// A handler's rendered start snapshot (see `View::prepare_handler`).
pub struct PreparedHandler<'v> {
    it: Interp<'v>,
    handler: Value,
}

impl PreparedHandler<'_> {
    fn arguments(args: &serde_json::Value) -> Vec<Value> {
        match args
            .as_object()
            .and_then(|m| if m.len() == 1 { m.get("$args") } else { None })
        {
            Some(serde_json::Value::Array(xs)) => xs.iter().map(Value::from_json).collect(),
            _ => vec![Value::from_json(args)],
        }
    }
    /// Reconstruct one execution and deliver recorded host completions through its queue.
    pub fn run(&self, args: &serde_json::Value, transcript: Vec<effects::Effect>) -> Result<EffectAction, String> {
        let mut a = self.it.fork_for_action();
        let realm = microtasks::Queue::new();
        realm.borrow_mut().begin(0, a.run);
        a.task = 0;
        a.realm = Some(realm.clone());
        a.effects = Some(effects::Effects::new(transcript));
        a.call_handler(&self.handler, Self::arguments(args))?;
        loop {
            if realm.borrow_mut().next_task().is_some() {
                microtasks::Queue::run(&realm, &mut a, 0)?;
                continue;
            }
            let next = {
                let effects = a.effects.as_ref().unwrap();
                let q = realm.borrow();
                effects
                    .slots
                    .iter()
                    .enumerate()
                    .filter(|(i, e)| e.outcome.is_some() && q.groups[&0].hosts.contains_key(i))
                    .min_by_key(|(i, e)| (e.settled.unwrap_or(0), *i))
                    .map(|(i, e)| (i, e.request.clone(), e.outcome.clone().unwrap()))
            };
            if let Some((i, request, outcome)) = next {
                realm.borrow_mut().reply(0, i, &request, &outcome);
            } else {
                break;
            }
        }
        let result = realm.borrow().groups[&0].root.completed();
        Self::result(&mut a, result, true)
    }
    pub(crate) fn step(
        &self,
        args: &serde_json::Value,
        next_effect: usize,
        live: Rc<RefCell<Map>>,
        realm: microtasks::Realm,
        task: u64,
        services: (i64, &mut u64, &mut effects::Signals),
    ) -> Result<EffectAction, String> {
        let (clock, rng, signals) = services;
        let mut a = self.it.fork_for_action();
        let fresh = !realm.borrow().groups.contains_key(&task);
        if fresh {
            realm.borrow_mut().begin(task, a.run);
        }
        a.run = realm.borrow().groups[&task].run;
        a.task = task;
        a.realm = Some(realm.clone());
        a.live_ui = Some(live);
        a.effects = Some(effects::Effects::resume(next_effect));
        a.services = Some((clock, *rng));
        a.signals = std::mem::take(signals);
        a.steps = realm.borrow().fuel;
        if let Some(ui) = realm.borrow_mut().groups.get_mut(&task).unwrap().ui.take() {
            a.put_ui(ui);
        }
        let result = if fresh {
            a.call_handler(&self.handler, Self::arguments(args)).map(|_| ())
        } else if realm.borrow_mut().next_task() == Some(task) {
            microtasks::Queue::run(&realm, &mut a, task)
        } else {
            Ok(())
        };
        *rng = a.services.unwrap().1;
        *signals = std::mem::take(&mut a.signals);
        realm.borrow_mut().fuel = a.steps;
        let ui = a.take_ui();
        realm.borrow_mut().groups.get_mut(&task).unwrap().ui = Some(ui);
        result?;
        let result = realm.borrow().groups[&task].root.completed();
        Self::result(&mut a, result, false)
    }
    fn result(a: &mut Interp<'_>, result: Option<effects::Settle>, validate: bool) -> Result<EffectAction, String> {
        let (done, patch) = match result {
            Some(effects::Settle::Ok(v)) => (true, v),
            Some(effects::Settle::Err(e)) => return Err(e.text),
            _ => (false, Value::None),
        };
        let value = if done { patch.to_wire() } else { serde_json::Value::Null };
        let mut writes = match &patch {
            Value::Map(m) => m.read().clone(),
            _ => Map::default(),
        };
        if !a.ui_updates.is_empty() {
            writes.insert(Rc::from("__ui"), Value::map(a.ui_updates.clone()));
        }
        let effects = a.effects.take().unwrap();
        if validate && effects.cursor != effects.slots.len() {
            return Err("effect transcript diverged: unconsumed requests".into());
        }
        let blocked = effects
            .slots
            .iter()
            .enumerate()
            .filter(|(_, effect)| effect.outcome.is_none())
            .map(|(i, _)| i)
            .collect();
        Ok(EffectAction {
            done,
            value,
            writes: Value::map(writes).to_wire(),
            effects: effects.slots,
            blocked,
            returned: if done { patch } else { Value::Undef },
        })
    }
}

/// The handler value of the `which`-th affordance named `action` in a render.
fn find_affordance<'a>(spans: &'a [Span], action: &str, which: usize) -> Result<&'a Span, String> {
    let callable = |s: &&Span| s.kind == "affordance" && s.meta.get("handler").is_some();
    // The action of that name. Only when none has it, an action on the element so named (its
    // `target`): a box called `save` with another action must not answer for the action `save`.
    spans
        .iter()
        .filter(callable)
        .filter(|s| s.label == action)
        .nth(which)
        .or_else(|| {
            spans
                .iter()
                .filter(callable)
                .filter(|s| s.meta["target"].as_str() == Some(action) && !is_harness_callback(&s.label))
                .nth(which)
        })
        .ok_or_else(|| format!("no affordance `{action}` (#{which}) with a handler in this view"))
}

/// Bind mechanical actions to their captured layout; all dispatch entry points use this.
fn bind_handler(it: &Interp, span: &Span) -> Result<Value, String> {
    let handler = span.meta["handler"]
        .as_u64()
        .and_then(|h| it.handlers.get(h as usize))
        .cloned()
        .ok_or_else(|| format!("affordance `{}` has no handler in this render", span.label))?;
    Ok(match span.meta.get("scroll_max") {
        Some(max) => Value::Bound(Rc::new((handler, Value::Undef, vec![Value::from_json(max)]))),
        None => handler,
    })
}
fn find_handler(it: &Interp, spans: &[Span], action: &str, which: usize) -> Result<Value, String> {
    bind_handler(it, find_affordance(spans, action, which)?)
}

/// An error's message without the view stack that render errors carry after it
/// (`\n    at Component (file:line)` frames and `\n    while rendering TurnStrategy log[i] ...`).
pub fn error_message(e: &str) -> &str {
    match e
        .find("\n    at ")
        .into_iter()
        .chain(e.find("\n    while rendering TurnStrategy"))
        .min()
    {
        Some(i) => &e[..i],
        None => e,
    }
}

#[cfg(test)]
mod view_tests {
    use super::*;

    /// A module's class definitions are freed with the program that holds them.
    #[test]
    fn module_classes_are_released_with_the_program() {
        let enc = Encoder::from_file(test_support::path().to_str().unwrap()).unwrap();
        let dir = std::env::temp_dir().join(format!("aff-view-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let app = dir.join("app.tsx");
        std::fs::write(
            &app,
            "class Private{} export default function Main(){return new Private() instanceof Private?'yes':'no';}",
        )
        .unwrap();
        let view = View::load(&app, "Main", &enc).unwrap();
        let weak = view
            .prog
            .consts
            .iter()
            .find_map(|(_, e)| match e {
                ast::Expr::Class(c) if c.name.as_deref() == Some("Private") => Some(Rc::downgrade(c)),
                _ => None,
            })
            .unwrap();
        let now = chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        view.render(&enc, Rc::new(Default::default()), &mut RunCache::new(false), now)
            .unwrap();
        drop(view);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(weak.strong_count(), 0);
    }
}
