//! Rendering chat requests for a model server, in place of a Jinja chat template and a tokenizer
//! pass: a conversation goes in as the server has it (messages, tools, the template's keyword
//! arguments) and its prompt comes out as token ids, through a template view (`views/`), which
//! produces the ids the model's official template and tokenizer would.
//!
//! A server sees the same conversation again and again, a turn longer each time. A
//! [`ChatRenderer`] keeps what it rendered: a request that starts with the messages of an earlier
//! one costs what is new. It owns its rendering threads (a view's values are not shared between
//! threads); a conversation always goes to the same thread, which keeps the conversations it
//! rendered most recently. The tokenizer is loaded once and shared.
//!
//! ```no_run
//! use affordance::chat_render::{ChatRenderer, Options};
//! use serde_json::json;
//! let renderer = ChatRenderer::json(Options::new("views/qwen/qwen35.tsx", "Qwen36", "tokenizer.json")).unwrap();
//! let messages = vec![json!({"role": "user", "content": "hello"})];
//! let ids = renderer.render(&messages, None, &json!({"add_generation_prompt": true})).unwrap();
//! ```
//!
//! The message type is the caller's: a server passes its own message values with a function
//! that turns one into the JSON a chat template reads, and that function runs only for messages
//! the renderer has not seen at their place before (they are compared with `==`).
use crate::emit::{Encoder, RunCache};
use crate::value::{Map, Value};
use crate::{View, state_from_json};
use serde_json::{Value as J, json};
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, mpsc};

/// What a [`ChatRenderer`] is made from.
#[derive(Debug, Clone)]
pub struct Options {
    /// The template view: a `.tsx` file.
    pub view: PathBuf,
    /// The view's component (`views/index.json` lists each model's).
    pub entry: String,
    /// The model's `tokenizer.json`.
    pub tokenizer: PathBuf,
    /// The tokenizer's special tokens as the template names them: `{"bos_token": "...", ...}`.
    pub special: J,
    /// Rendering threads. A conversation is always rendered by the same one.
    pub workers: usize,
    /// How many conversations each thread keeps the renders of (the least recently used goes).
    pub kept_conversations: usize,
    /// Token ids a thread may hold in kept conversations before the least recently used go
    /// (counted as the length of each conversation's last prompt).
    pub kept_tokens: usize,
}

impl Options {
    /// Options for a view, with defaults: up to 8 threads, 32 conversations and 4 million
    /// prompt tokens kept a thread, no special tokens.
    pub fn new(view: impl Into<PathBuf>, entry: impl Into<String>, tokenizer: impl Into<PathBuf>) -> Self {
        Options {
            view: view.into(),
            entry: entry.into(),
            tokenizer: tokenizer.into(),
            special: json!({}),
            workers: std::thread::available_parallelism().map_or(4, |n| n.get().min(8)),
            kept_conversations: 32,
            kept_tokens: 4_000_000,
        }
    }
}

/// Why a request was not rendered.
#[derive(Debug, Clone, PartialEq)]
pub enum RenderError {
    /// The template raised: the request is one the model's official template rejects too (a
    /// tool message with no call before it, an unknown role). The text is the template's.
    Raised(String),
    /// Anything else: a message that could not be converted, a view that failed to run.
    Failed(String),
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderError::Raised(message) | RenderError::Failed(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for RenderError {}

/// Turns one of the caller's messages into the JSON a chat template reads
/// (`{"role": ..., "content": ..., ...}`).
pub type Convert<M> = fn(&M) -> Result<J, String>;

struct Job<M> {
    key: u64,
    messages: Vec<M>,
    /// Everything but the messages: `{"tools": ..., "kwargs": {...}}`.
    rest: J,
    reply: mpsc::Sender<Result<Vec<u32>, RenderError>>,
}

/// One conversation as a thread remembers it: the messages of its last request, each with the
/// value the view was given for it, and what the view memoized.
struct Conversation<M> {
    key: u64,
    used: u64,
    tokens: usize,
    cache: RunCache,
    kept: Vec<(M, Value)>,
    /// The tool list of its last request, with the value the view was given for it: the same
    /// list is given as the same value, since a view's kept turns depend on the tools they were
    /// rendered with.
    tools: Option<(J, Value)>,
}

struct Worker<M> {
    enc: Arc<Encoder>,
    view: View,
    special: J,
    convert: Convert<M>,
    conversations: Vec<Conversation<M>>,
    clock: u64,
    kept_conversations: usize,
    kept_tokens: usize,
}

impl<M: PartialEq> Worker<M> {
    fn render(&mut self, key: u64, messages: Vec<M>, rest: &J) -> Result<Vec<u32>, RenderError> {
        self.clock += 1;
        let at = match self.conversations.iter().position(|c| c.key == key) {
            Some(at) => at,
            None => {
                self.conversations.push(Conversation {
                    key,
                    used: 0,
                    tokens: 0,
                    cache: RunCache::new(true),
                    kept: Vec::new(),
                    tools: None,
                });
                self.conversations.len() - 1
            }
        };
        let conversation = &mut self.conversations[at];
        conversation.used = self.clock;
        let mut values = Vec::with_capacity(messages.len());
        for (i, message) in messages.into_iter().enumerate() {
            if conversation.kept.get(i).is_some_and(|(kept, _)| *kept == message) {
                values.push(conversation.kept[i].1.clone());
                continue;
            }
            let value = Value::from_json(&(self.convert)(&message).map_err(RenderError::Failed)?);
            conversation.kept.truncate(i);
            conversation.kept.push((message, value.clone()));
            values.push(value);
        }
        conversation.kept.truncate(values.len());
        let mut state: Map = (*state_from_json(rest, &self.special)).clone();
        state.insert(Rc::from("messages"), Value::list(values));
        if !rest["tools"].is_null() {
            match &conversation.tools {
                Some((kept, value)) if *kept == rest["tools"] => {
                    state.insert(Rc::from("tools"), value.clone());
                }
                _ => {
                    if let Some(value) = state.get("tools") {
                        conversation.tools = Some((rest["tools"].clone(), value.clone()));
                    }
                }
            }
        }
        conversation.cache.next_generation();
        let now = chrono::Utc::now().naive_utc();
        let rendered = match self
            .view
            .render(&self.enc, Rc::new(state), &mut conversation.cache, now)
        {
            Ok(rendered) => rendered,
            Err(error) => {
                // A conversation whose first render fails is not kept: requests that only ever
                // fail must not fill the cache with entries nothing evicts.
                if conversation.tokens == 0 {
                    self.conversations.remove(at);
                }
                return Err(match error.strip_prefix("raise: ") {
                    Some(message) => RenderError::Raised(message.to_owned()),
                    None => RenderError::Failed(error),
                });
            }
        };
        conversation.tokens = rendered.ids.len();
        // Keep within the bounds: the least recently used conversations go (never this one).
        while self.conversations.len() > 1
            && (self.conversations.len() > self.kept_conversations
                || self.conversations.iter().map(|c| c.tokens).sum::<usize>() > self.kept_tokens)
        {
            let oldest = (0..self.conversations.len())
                .min_by_key(|&i| self.conversations[i].used)
                .expect("not empty");
            self.conversations.swap_remove(oldest);
        }
        Ok(rendered.ids)
    }
}

/// Renders chat requests to token ids with a template view. See the module's description.
pub struct ChatRenderer<M = J> {
    workers: Vec<mpsc::Sender<Job<M>>>,
}

impl ChatRenderer<J> {
    /// A renderer whose messages are already the JSON a chat template reads.
    pub fn json(options: Options) -> Result<Self, String> {
        Self::new(options, |message| Ok(message.clone()))
    }
}

impl<M: PartialEq + Clone + std::fmt::Debug + Send + 'static> ChatRenderer<M> {
    /// A renderer for the caller's message type. The tokenizer is loaded once; each rendering
    /// thread loads the view. A view or tokenizer that does not load is an error here, not at the
    /// first request.
    pub fn new(options: Options, convert: Convert<M>) -> Result<Self, String> {
        let enc = Arc::new(Encoder::from_file(&options.tokenizer.to_string_lossy())?);
        let mut workers = Vec::new();
        let mut loading = Vec::new();
        for _ in 0..options.workers.max(1) {
            let (jobs, inbox) = mpsc::channel::<Job<M>>();
            let (loaded, load) = mpsc::channel::<Result<(), String>>();
            let (enc, options) = (enc.clone(), options.clone());
            std::thread::Builder::new()
                .name("affordance-render".into())
                .spawn(move || {
                    let view = match View::load(&options.view, &options.entry, &enc) {
                        Ok(view) => view,
                        Err(error) => {
                            let _ = loaded.send(Err(error));
                            return;
                        }
                    };
                    let _ = loaded.send(Ok(()));
                    let mut worker = Worker {
                        enc,
                        view,
                        special: options.special,
                        convert,
                        conversations: Vec::new(),
                        clock: 0,
                        kept_conversations: options.kept_conversations.max(1),
                        kept_tokens: options.kept_tokens,
                    };
                    while let Ok(job) = inbox.recv() {
                        let _ = job.reply.send(worker.render(job.key, job.messages, &job.rest));
                    }
                })
                .map_err(|e| format!("chat renderer: {e}"))?;
            workers.push(jobs);
            loading.push(load);
        }
        for load in loading {
            load.recv()
                .map_err(|_| "chat renderer: a rendering thread died".to_string())??;
        }
        Ok(ChatRenderer { workers })
    }

    /// The prompt of a request, as token ids. `tools` is the request's tool list as the template
    /// reads it (or none); `kwargs` the template's keyword arguments, with
    /// `add_generation_prompt` among them. Blocks while a rendering thread renders it.
    pub fn render(&self, messages: &[M], tools: Option<&J>, kwargs: &J) -> Result<Vec<u32>, RenderError> {
        // A conversation's key: its first two messages (a system prompt alone is shared by many).
        // Two conversations with the same key share a slot, which costs time, never correctness:
        // a request is only given the values of messages equal to its own.
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for message in messages.iter().take(2) {
            format!("{message:?}").hash(&mut hasher);
        }
        let key = hasher.finish();
        let (reply, answer) = mpsc::channel();
        let job = Job {
            key,
            messages: messages.to_vec(),
            rest: json!({"messages": [], "tools": tools, "kwargs": kwargs}),
            reply,
        };
        let died = || RenderError::Failed("chat renderer: a rendering thread died".into());
        self.workers[(key % self.workers.len() as u64) as usize]
            .send(job)
            .map_err(|_| died())?;
        answer.recv().map_err(|_| died())?
    }
}
