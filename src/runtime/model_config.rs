//! Deployment-side model configuration and the shared adapter validation boundary.
use super::model::{Api, Chat, Completions, Delta, Model, ModelError, Sessions, Sglang, Window};
use crate::emit::Encoder;
use serde_json::{Value as J, json};
use std::{path::PathBuf, sync::Arc, time::Duration};

/// The presentation associated with a model binding, independent of its provider.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModelTemplate {
    /// The template app's name (`glm53`, `messages`, ...).
    pub name: String,
    /// Its options, as its `template_options` declares them.
    pub options: J,
}
impl ModelTemplate {
    /// A template by name with options, checked against what the app declares.
    pub fn new(name: impl Into<String>, options: J) -> Result<Self, String> {
        let template = Self {
            name: name.into(),
            options,
        };
        template.validate()?;
        Ok(template)
    }
    /// Whether `name` is a model the library has a format for (`org/name` or `org__name`).
    pub fn registered(name: &str) -> bool {
        static MODELS: std::sync::OnceLock<J> = std::sync::OnceLock::new();
        let models =
            MODELS.get_or_init(|| serde_json::from_str(include_str!("../../views/index.json")).expect("model index"));
        !name.starts_with('_') && models.get(name.replacen('/', "__", 1)).is_some_and(J::is_object)
    }
    /// Check the name and options against the template apps the library ships.
    pub fn validate(&self) -> Result<(), String> {
        static REGISTRY: std::sync::OnceLock<J> = std::sync::OnceLock::new();
        let registry = REGISTRY.get_or_init(|| {
            serde_json::from_str(include_str!("../../views/template_apps/index.json")).expect("template registry")
        });
        if registry.get(&self.name).is_none() {
            // A registered model's name: its format is rendered by the one template app for
            // any model (`ChatApp`), and its options are that model's own template's.
            if Self::registered(&self.name) {
                return match self.options.is_object() {
                    true => Ok(()),
                    false => Err("template_options must be an object".into()),
                };
            }
            return Err(format!(
                "unknown model template {:?}; use a template app (a key of views/template_apps/index.json) or a registered model's name (org/name, a key of views/index.json)",
                self.name
            ));
        }
        let options = self.options.as_object().ok_or("template_options must be an object")?;
        if let Some(key) = options.keys().find(|k| matches!(k.as_str(), "children" | "system")) {
            return Err(format!("template_options cannot override app-owned {key}"));
        }
        // An option the app does not take would be ignored silently: refused instead.
        let declared: Vec<&str> = registry[&self.name]["options"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(J::as_str)
            .collect();
        if let Some(key) = options.keys().find(|k| !declared.contains(&k.as_str())) {
            return Err(format!(
                "template {:?} takes no option {key:?} (its options: {})",
                self.name,
                if declared.is_empty() {
                    "none".into()
                } else {
                    declared.join(", ")
                }
            ));
        }
        Ok(())
    }
}

/// Attach presentation to any model client. This delegates inference without changing its wire.
pub fn with_template(client: Arc<dyn Model>, template: ModelTemplate) -> Result<Arc<dyn Model>, String> {
    template.validate()?;
    Ok(Arc::new(Presented {
        client,
        template: Some(template),
        window: Stated::No,
    }))
}
/// State the model's context length (every prompt's window) on any model client.
pub fn with_context(client: Arc<dyn Model>, context: usize) -> Arc<dyn Model> {
    Arc::new(Presented {
        template: client.template(),
        client,
        window: Stated::Configured(context),
    })
}
/// A client with the binding's presentation: its template and its context length.
struct Presented {
    client: Arc<dyn Model>,
    template: Option<ModelTemplate>,
    window: Stated,
}
/// What a `Presented` states of the window: nothing (its client's, if any, unstated), a
/// configured one (`with_context`), or its built-in client's, established at `validate`.
enum Stated {
    No,
    Configured(usize),
    Client,
}
impl Presented {
    /// The identity with the stated window: it shapes prompts, so it is part of the binding. A
    /// window from the server is the same binding as that window configured.
    fn identity(&self, client: J) -> J {
        let mut id = json!({"client": client, "template": self.template});
        if let (Stated::Configured(_) | Stated::Client, Some(c)) = (&self.window, self.context()) {
            id["context"] = json!(c);
        }
        id
    }
}
impl Model for Presented {
    fn validate(&self) -> Result<(), ModelError> {
        if let Some(t) = &self.template {
            t.validate().map_err(ModelError::fatal)?;
        }
        self.client.validate()
    }
    fn template(&self) -> Option<ModelTemplate> {
        self.template.clone()
    }
    fn context(&self) -> Option<usize> {
        match self.window {
            Stated::Configured(c) => Some(c),
            _ => self.client.context(),
        }
    }
    fn window_source(&self) -> Option<String> {
        match self.window {
            Stated::Configured(_) => Some("configured".into()),
            _ => self.client.window_source(),
        }
    }
    fn binding_identity(&self) -> Option<J> {
        self.client.binding_identity().map(|client| self.identity(client))
    }
    fn replay_identity(&self) -> Option<J> {
        self.client.replay_identity().map(|client| self.identity(client))
    }
    fn ranks(&self) -> usize {
        self.client.ranks()
    }
    fn retry(&self) -> super::model::RetryPolicy {
        self.client.retry()
    }
    fn generate(&self, req: &super::model::GenRequest) -> Result<super::model::GenReply, ModelError> {
        self.client.generate(req)
    }
    fn generate_stream(
        &self,
        req: &super::model::GenRequest,
        on_tokens: &mut dyn FnMut(Delta<'_>) -> bool,
    ) -> Result<super::model::GenReply, ModelError> {
        self.client.generate_stream(req, on_tokens)
    }
}

/// A binding-local bearer key: a `NAME=value` line of a file (`key_file` + `key_name`), or an
/// environment variable (`key_env`). Only the source is configuration; the key is read at call time.
#[derive(Debug, Clone)]
pub struct Credentials {
    /// None: `key_name` is an environment variable.
    pub key_file: Option<PathBuf>,
    /// The key's name: the `NAME=` line of `key_file`, or the environment variable.
    pub key_name: String,
}
impl Credentials {
    /// A key read from the line `name=...` of `file`.
    pub fn new(file: impl Into<PathBuf>, name: impl Into<String>) -> Self {
        Self {
            key_file: Some(file.into()),
            key_name: name.into(),
        }
    }
    /// The key in the environment variable `name`.
    pub fn env(name: impl Into<String>) -> Self {
        Self {
            key_file: None,
            key_name: name.into(),
        }
    }
    pub(crate) fn read(&self) -> Result<String, ModelError> {
        if self.key_name.is_empty() {
            return Err(ModelError::fatal("key_name must be nonempty"));
        }
        let Some(file) = &self.key_file else {
            let key = std::env::var(&self.key_name).map_err(|_| {
                ModelError::fatal(format!("environment variable {} (key_env) is not set", self.key_name))
            })?;
            return Self::check(key.trim().to_string());
        };
        let text = std::fs::read_to_string(file)
            .map_err(|e| ModelError::fatal(format!("credential file {}: {e}", file.display())))?;
        let raw = text
            .lines()
            .filter_map(|l| l.trim().trim_start_matches("export ").split_once('='))
            .find(|(k, _)| k.trim() == self.key_name)
            .map(|(_, v)| v.trim())
            .ok_or_else(|| ModelError::fatal(format!("credential file has no key named {}", self.key_name)))?;
        let key = if let Some(quote) = raw.chars().next().filter(|c| matches!(c, '\"' | '\'')) {
            if raw.len() < 2 || !raw.ends_with(quote) {
                return Err(ModelError::fatal(
                    "quoted bearer credential must close on the same line",
                ));
            }
            raw[1..raw.len() - 1].to_string()
        } else {
            raw.to_string()
        };
        Self::check(key)
    }
    fn check(key: String) -> Result<String, ModelError> {
        if key.is_empty() || !key.bytes().all(|b| b.is_ascii_graphic()) {
            return Err(ModelError::fatal(
                "bearer credential must be nonempty printable ASCII without whitespace",
            ));
        }
        Ok(key)
    }
    pub(crate) fn identity(&self) -> J {
        match &self.key_file {
            Some(file) => json!({"key_file": file.as_os_str().as_encoded_bytes(), "key_name": self.key_name}),
            None => json!({"key_env": self.key_name}),
        }
    }
}

pub(crate) fn auth_headers(auth: Option<&Credentials>) -> Result<Vec<(&'static str, String)>, ModelError> {
    auth.map(|a| a.read().map(|key| vec![("Authorization", format!("Bearer {key}"))]))
        .unwrap_or_else(|| Ok(vec![]))
}
pub(crate) fn fingerprint(mut value: J) -> String {
    value.sort_all_objects();
    crate::sha256_hex(value.to_string().as_bytes())
}

pub(crate) fn validate_chat_headers(
    headers: &std::collections::BTreeMap<String, String>,
    anthropic: bool,
) -> Result<(), ModelError> {
    let mut seen = std::collections::HashSet::new();
    for (name, value) in headers {
        if name.is_empty()
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
        {
            return Err(ModelError::fatal("chat headers require valid HTTP token names"));
        }
        let lower = name.to_ascii_lowercase();
        if lower == "content-type"
            || super::model::KEY_HEADERS.contains(&lower.as_str())
            || (anthropic && lower == "anthropic-version")
        {
            return Err(ModelError::fatal(format!("chat header {name} is client-owned")));
        }
        if !seen.insert(lower) {
            return Err(ModelError::fatal(format!("duplicate chat header {name}")));
        }
        if !value.bytes().all(|b| b == b'\t' || (b >= 0x20 && b != 0x7f)) {
            return Err(ModelError::fatal(format!(
                "chat header {name} contains an invalid value"
            )));
        }
    }
    Ok(())
}

pub(crate) const OWNED: &[&str] = &[
    "prompt",
    "input",
    "input_ids",
    "text",
    "messages",
    "tools",
    "model",
    "image_data",
    "rid",
    "request_id",
    "session_params",
    "session_id",
    "routed_dp_rank",
    "data_parallel_rank",
    "sampling_params",
    "stream",
    "stream_options",
    "return_token_ids",
    "skip_special_tokens",
    "no_stop_trim",
    "include_stop_str_in_output",
    "stop",
    "stop_token_ids",
    "stop_sequences",
    "max_tokens",
    "max_new_tokens",
    "max_completion_tokens",
    "max_output_tokens",
    "input_embeds",
    "inputs_embeds",
    "input_embeddings",
    "n",
    "url",
    "endpoint",
    "headers",
    "authorization",
    "api_key",
    "key_file",
    "key_name",
    "key_env",
    "provider_options",
    "system",
];
pub(crate) fn object<'a>(value: &'a J, name: &str) -> Result<Option<&'a serde_json::Map<String, J>>, ModelError> {
    match value {
        J::Null => Ok(None),
        J::Object(o) => Ok(Some(o)),
        _ => Err(ModelError::fatal(format!("{name} must be an object"))),
    }
}
pub(crate) fn validate_sampling(value: &J) -> Result<(), ModelError> {
    if let Some(o) = object(value, "sampling")?
        && let Some(key) = o.keys().find(|key| {
            OWNED.contains(&key.to_ascii_lowercase().as_str())
                || matches!(
                    key.as_str(),
                    "lora_path" | "lora_request" | "adapter" | "adapter_name" | "adapter_id"
                )
        })
    {
        return Err(ModelError::fatal(format!(
            "sampling cannot override host-owned field `{key}`; use explicit call options or the model binding"
        )));
    }
    Ok(())
}
pub(crate) fn validate_options(options: &J, sampling: &J, overrides: &J) -> Result<(), ModelError> {
    validate_options_allowing(options, sampling, overrides, &[])
}

pub(crate) fn validate_options_allowing(
    options: &J,
    sampling: &J,
    overrides: &J,
    allowed: &[&str],
) -> Result<(), ModelError> {
    validate_sampling(sampling)?;
    validate_sampling(overrides)?;
    if let Some(o) = object(options, "provider_options")? {
        for (key, value) in o {
            if OWNED.contains(&key.to_ascii_lowercase().as_str()) && !allowed.contains(&key.as_str()) {
                return Err(ModelError::fatal(format!(
                    "provider_options cannot override host-owned field `{key}`"
                )));
            }
            if sampling.get(key).is_some() || overrides.get(key).is_some() {
                return Err(ModelError::fatal(format!(
                    "`{key}` is configured in both provider_options and sampling"
                )));
            }
            if key == "lora_path" && !(value.is_null() || value.as_str().is_some_and(|s| !s.is_empty())) {
                return Err(ModelError::fatal(
                    "provider_options.lora_path must be a nonempty string or null for one prompt",
                ));
            }
        }
    }
    Ok(())
}
pub(crate) fn validate_url(url: &str, chat: bool) -> Result<(), ModelError> {
    let rest = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))
        .ok_or_else(|| ModelError::fatal("model url must use http:// or https://"))?;
    let authority = rest.split('/').next().unwrap_or_default();
    if authority.is_empty()
        || url.bytes().any(|b| b.is_ascii_control() || b.is_ascii_whitespace())
        || url.contains(['?', '#', '"', '\\'])
        || authority.contains('@')
    {
        return Err(ModelError::fatal(
            "model url must have a host and no credentials, whitespace, query or fragment",
        ));
    }
    let port = if let Some(ipv6) = authority.strip_prefix('[') {
        if !chat {
            return Err(ModelError::fatal(
                "native HTTP transport does not support IPv6 literals",
            ));
        }
        let (host, tail) = ipv6
            .split_once(']')
            .ok_or_else(|| ModelError::fatal("invalid IPv6 model host"))?;
        host.parse::<std::net::Ipv6Addr>()
            .map_err(|_| ModelError::fatal("invalid IPv6 model host"))?;
        if tail.is_empty() {
            None
        } else {
            Some(
                tail.strip_prefix(':')
                    .ok_or_else(|| ModelError::fatal("invalid model URL authority"))?,
            )
        }
    } else {
        let (host, port) = authority
            .rsplit_once(':')
            .map_or((authority, None), |(h, p)| (h, Some(p)));
        if host.is_empty()
            || host
                .bytes()
                .any(|b| !(b.is_ascii_alphanumeric() || b".-_".contains(&b)))
        {
            return Err(ModelError::fatal("model url requires a hostname or IP address"));
        }
        port
    };
    if port.is_some_and(|p| p.parse::<u16>().ok().filter(|p| *p > 0).is_none()) {
        return Err(ModelError::fatal("model url port must be between 1 and 65535"));
    }
    Ok(())
}
pub(crate) fn validate_native(
    url: &str,
    ranks: usize,
    timeout: Duration,
    stall: Duration,
    auth: Option<&Credentials>,
) -> Result<(), ModelError> {
    validate_url(url, false)?;
    if ranks == 0 || timeout.is_zero() || stall.is_zero() {
        return Err(ModelError::fatal(
            "ranks, request_timeout_s and stall_s must be positive",
        ));
    }
    if let Some(a) = auth {
        a.read()?;
    }
    Ok(())
}

/// Build an existing stock adapter from the same strict JSON contract used by `affordance run`.
/// The caller supplies the binding's encoder; the CLI loads it from `tokenizer`.
pub fn from_config(m: &J, enc: Arc<Encoder>) -> Result<Arc<dyn Model>, String> {
    let o = m.as_object().ok_or("model binding must be an object")?;
    let kind = m["kind"]
        .as_str()
        .ok_or("model.kind must be sglang, completions, chat, anthropic or responses")?;
    let specific: &[&str] = match kind {
        "sglang" => &["ranks", "stream", "stall_s", "sessions"],
        "completions" => &["name", "ranks", "stream", "stall_s"],
        "chat" => &["model", "headers", "key_header", "stream", "stall_s"],
        "anthropic" => &["model", "headers", "key_header", "stream", "stall_s", "cache_ttl"],
        "responses" => &["model", "headers", "key_header", "stream", "stall_s", "strict_tools"],
        _ => {
            return Err(format!(
                "model.kind {kind:?}: use sglang, completions, chat, anthropic or responses"
            ));
        }
    };
    const COMMON: &[&str] = &[
        "kind",
        "url",
        "tokenizer",
        "sampling",
        "provider_options",
        "key_file",
        "key_name",
        "key_env",
        "request_timeout_s",
        "template",
        "template_options",
        "context",
        "retry",
    ];
    if let Some(key) = o
        .keys()
        .find(|key| !COMMON.contains(&key.as_str()) && !specific.contains(&key.as_str()))
    {
        return Err(format!("model ({kind}): unknown or unsupported field `{key}`"));
    }
    let string = |name: &str| -> Result<&str, String> {
        m[name]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| format!("model.{name} must be a nonempty string"))
    };
    let hosted = matches!(kind, "chat" | "anthropic" | "responses");
    let template = match m.get("template") {
        Some(_) => Some(string("template")?),
        None if hosted => Some("messages"),
        None => None,
    };
    if hosted && template != Some("messages") {
        return Err("chat bindings require the messages template".into());
    }
    if template.is_none() && m.get("template_options").is_some() {
        return Err("template_options requires a template".into());
    }
    let template = template
        .map(|name| ModelTemplate::new(name, m.get("template_options").cloned().unwrap_or(json!({}))))
        .transpose()?;
    let count = |name: &str, default: usize, positive: bool| -> Result<usize, String> {
        match m.get(name) {
            None => Ok(default),
            Some(v) => v
                .as_u64()
                .and_then(|n| usize::try_from(n).ok())
                .filter(|n| !positive || *n > 0)
                .ok_or_else(|| {
                    format!(
                        "model.{name} must be a {}integer",
                        if positive { "positive " } else { "nonnegative " }
                    )
                }),
        }
    };
    let duration = |v: &J, name: &str| -> Result<Duration, String> {
        v.as_f64()
            .filter(|n| n.is_finite() && *n > 0.0)
            .and_then(|n| Duration::try_from_secs_f64(n).ok())
            .filter(|d| !d.is_zero())
            .ok_or_else(|| format!("{name} must be finite positive seconds"))
    };
    let stream = match m.get("stream") {
        None => !hosted,
        Some(J::Bool(b)) => *b,
        _ => return Err("model.stream must be a boolean".into()),
    };
    if hosted && stream {
        return Err("chat does not support streaming".into());
    }
    if m.get("tokenizer").is_some() {
        string("tokenizer")?;
    }
    let auth = match (m.get("key_file"), m.get("key_name"), m.get("key_env")) {
        (None, None, None) => None,
        (Some(_), Some(_), None) => Some(Credentials::new(string("key_file")?, string("key_name")?)),
        (None, None, Some(_)) => Some(Credentials::env(string("key_env")?)),
        (_, _, Some(_)) => return Err("key_env takes the key from the environment; drop key_file and key_name".into()),
        _ => return Err("model credentials require both key_file and key_name (or key_env)".into()),
    };
    for key in ["sampling", "provider_options"] {
        if m.get(key).is_some_and(|v| !v.is_object()) {
            return Err(format!("model.{key} must be an object"));
        }
    }
    let sampling = m.get("sampling").cloned().unwrap_or(json!({}));
    let options = m.get("provider_options").cloned().unwrap_or(json!({}));
    let timeout = m
        .get("request_timeout_s")
        .map(|v| duration(v, "model.request_timeout_s"))
        .transpose()?;
    let stall = m.get("stall_s").map(|v| duration(v, "model.stall_s")).transpose()?;
    // The model's context length: the server's (a native binding asks it at `validate`), at
    // most `context`; `"none"` runs without a window. Construction asks no server.
    let window = match m.get("context") {
        None => Window::Server,
        Some(J::String(s)) if s == "none" => Window::Unbounded,
        Some(_) => Window::Configured(count("context", 0, true).map_err(|e| format!("{e} or \"none\""))?),
    };
    // What happens after a call fails: the one retry policy (`RetryPolicy`).
    let retry = super::model::RetryPolicy::default()
        .with(m.get("retry").unwrap_or(&J::Null))
        .map_err(|e| format!("model.{e}"))?;
    let client: Arc<dyn Model> = match kind {
        "sglang" => {
            let mut s = Sglang::new(string("url")?, count("ranks", 1, true)?);
            s.auth = auth;
            s.sampling = sampling;
            s.provider_options = options;
            s.stream = stream;
            if let Some(t) = timeout {
                s.timeout = t
            }
            if let Some(t) = stall {
                s.stall = t
            }
            s.sessions = match m.get("sessions") {
                None | Some(J::Bool(false)) => None,
                Some(J::Bool(true)) => Some(Sessions::new(4, Duration::from_secs(600))),
                Some(J::Object(o)) => {
                    if let Some(k) = o.keys().find(|k| !matches!(k.as_str(), "turns" | "timeout_s")) {
                        return Err(format!("model.sessions: unknown field `{k}`"));
                    }
                    let turns = match o.get("turns") {
                        None => 4,
                        Some(v) => v
                            .as_u64()
                            .and_then(|n| usize::try_from(n).ok())
                            .filter(|n| *n > 0)
                            .ok_or("model.sessions.turns must be a positive integer")?,
                    };
                    let timeout = o
                        .get("timeout_s")
                        .map(|v| duration(v, "model.sessions.timeout_s"))
                        .transpose()?
                        .unwrap_or(Duration::from_secs(600));
                    Some(Sessions::new(turns, timeout))
                }
                _ => return Err("model.sessions must be false, true or {turns, timeout_s}".into()),
            };
            s.window = window;
            s.retry = retry;
            s.check().map_err(|e| e.message)?;
            Arc::new(s)
        }
        "completions" => {
            let name = if m.get("name").is_some() {
                string("name")?
            } else {
                "default"
            };
            let mut c = Completions::new(string("url")?, name);
            c.auth = auth;
            c.sampling = sampling;
            c.provider_options = options;
            c.ranks = count("ranks", 1, true)?;
            c.stream = stream;
            if let Some(t) = timeout {
                c.timeout = t
            }
            if let Some(t) = stall {
                c.stall = t
            }
            c.window = window;
            c.retry = retry.clone();
            c.check().map_err(|e| e.message)?;
            Arc::new(c)
        }
        "chat" | "anthropic" | "responses" => {
            // Without key_file and key_name the server takes no key (a local llama.cpp, Ollama, vLLM).
            let mut c = if kind == "chat" {
                Chat::new(string("url")?, string("model")?, auth, enc)
            } else if kind == "responses" {
                let url = if m.get("url").is_some() {
                    string("url")?
                } else {
                    "https://api.openai.com/v1"
                };
                let mut c = Chat::responses(url, string("model")?, auth, enc);
                let strict_tools = match m.get("strict_tools") {
                    None => false,
                    Some(J::Bool(b)) => *b,
                    _ => return Err("model.strict_tools must be a boolean".into()),
                };
                c.api = Api::Responses { strict_tools };
                c
            } else {
                let url = if m.get("url").is_some() {
                    string("url")?
                } else {
                    "https://api.anthropic.com"
                };
                let mut c = Chat::anthropic(url, string("model")?, auth, enc);
                if m.get("cache_ttl").is_some() {
                    c.api = Api::Anthropic {
                        cache_ttl: string("cache_ttl")?.into(),
                    };
                }
                c
            };
            if m.get("key_header").is_some() {
                c.key_header = Some(string("key_header")?.into())
            }
            c.sampling = sampling;
            c.provider_options = options;
            if let Some(headers) = m.get("headers") {
                c.headers = serde_json::from_value(headers.clone())
                    .map_err(|_| "model.headers must be an object of string values")?;
            }
            if let Some(t) = timeout {
                c.timeout = t
            }
            c.stall = stall;
            c.window = window;
            c.retry = retry.clone();
            // An OpenAI-compatible server may report its window: it is asked when the run starts
            // (`validate`), as a native server is, not while the configuration is read.
            if c.api == Api::Chat && window == Window::Server {
                c.check().map_err(|e| e.message)?;
            } else {
                c.validate().map_err(|e| e.message)?;
            }
            Arc::new(c)
        }
        _ => unreachable!(),
    };
    let client = match template {
        Some(template) => with_template(client, template)?,
        None => client,
    };
    // The window is stated as a configured one was, so a window the server reports is the
    // same binding as that window configured; without one, the binding is its client's.
    Ok(match window {
        Window::Unbounded => client,
        _ => Arc::new(Presented {
            template: client.template(),
            client,
            window: Stated::Client,
        }),
    })
}
