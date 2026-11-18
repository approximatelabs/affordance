//! Bound HTTP requests, independent of any application protocol.
use crate::runtime::http::{Failure, HttpError, Request as Wire, Timing, exchange};
use crate::runtime::{CallContext, Executor};
use reqwest::{
    Method, Url,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use serde::Deserialize;
use serde_json::{Value as J, json};
use std::{collections::BTreeMap, time::Duration};

fn timeout_ms() -> u64 {
    30_000
}
fn max_bytes() -> usize {
    8 << 20
}

/// A protected deployment header. Environment values come from the actual caller's context,
/// never from request arguments. No credential is copied into the request's replay record.
#[derive(Clone, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Header {
    /// A fixed value.
    Value(String),
    /// A value taken from the caller's environment.
    Env {
        /// The environment variable's name.
        env: String,
        #[serde(default)]
        /// Text put before the value (`Bearer `).
        prefix: String,
    },
}

/// A fixed HTTP origin and limits, supplied by deployment code.
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HttpConfig {
    /// The origin requests go to; a call's path is resolved under it.
    pub url: String,
    #[serde(default)]
    /// Headers the host adds to every request; an app cannot set or read them.
    pub headers: BTreeMap<String, Header>,
    #[serde(default = "timeout_ms")]
    /// The longest a request may take, in milliseconds.
    pub timeout_ms: u64,
    #[serde(default = "max_bytes")]
    /// The largest request body, in bytes.
    pub max_request_bytes: usize,
    #[serde(default = "max_bytes")]
    /// The largest response body, in bytes.
    pub max_response_bytes: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    path: Option<String>,
    method: Option<String>,
    #[serde(default)]
    headers: BTreeMap<String, String>,
    body: Option<String>,
    timeout_ms: Option<u64>,
}

/// A generic HTTP executor on Affordance's one HTTP client (`runtime::http`), whose connection
/// pool, certificates and proxy settings every request shares. The runtime journals its results
/// exactly as it does any other machine's. It never follows redirects, retries requests or stores
/// cookies.
pub struct Http {
    url: Url,
    headers: Vec<(HeaderName, Header)>,
    config: HttpConfig,
}

fn header_name(name: &str) -> Result<HeaderName, String> {
    let name = HeaderName::from_bytes(name.as_bytes()).map_err(|_| "http: invalid header name")?;
    // The HTTP client owns framing and routing. Connection-specific headers would also make
    // behavior depend on a pooled socket rather than on the request.
    if [
        "host",
        "content-length",
        "transfer-encoding",
        "connection",
        "upgrade",
        "trailer",
        "te",
        "proxy-authorization",
        "proxy-connection",
    ]
    .contains(&name.as_str())
    {
        return Err(format!("http: transport controls the {} header", name.as_str()));
    }
    Ok(name)
}

fn header_value(value: &str) -> Result<HeaderValue, String> {
    HeaderValue::from_str(value).map_err(|_| "http: invalid header value".into())
}

impl Http {
    /// An HTTP executor for the origin and limits in `config`.
    pub fn new(config: HttpConfig) -> Result<Self, String> {
        let url = Url::parse(&config.url).map_err(|_| "http: invalid deployment URL")?;
        if !matches!(url.scheme(), "http" | "https")
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.fragment().is_some()
        {
            return Err("http: URL must be http(s), without credentials or a fragment".into());
        }
        if config.timeout_ms == 0 || config.max_request_bytes == 0 || config.max_response_bytes == 0 {
            return Err("http: timeout and byte limits must be positive".into());
        }
        let mut headers = vec![];
        for (name, source) in &config.headers {
            let name = header_name(name)?;
            if headers.iter().any(|(n, _)| *n == name) {
                return Err("http: duplicate protected header".into());
            }
            if let Header::Value(value) = source {
                header_value(value)?;
            }
            headers.push((name, source.clone()));
        }
        Ok(Self { url, headers, config })
    }

    fn prepare(&self, ctx: &CallContext, request: Request) -> Result<Prepared, String> {
        let url = match &request.path {
            Some(path) => self.url.join(path).map_err(|_| "http: invalid request path")?,
            None => self.url.clone(),
        };
        if url.origin() != self.url.origin()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.fragment().is_some()
        {
            return Err("http: request path must stay on the bound origin, without credentials or a fragment".into());
        }
        let method = Method::from_bytes(request.method.as_deref().unwrap_or("GET").as_bytes())
            .map_err(|_| "http: invalid method")?;
        if matches!(
            method.as_str().to_ascii_uppercase().as_str(),
            "CONNECT" | "TRACE" | "TRACK"
        ) {
            return Err("http: CONNECT, TRACE and TRACK are not supported".into());
        }
        let timeout = request.timeout_ms.unwrap_or(self.config.timeout_ms);
        if timeout == 0 || timeout > self.config.timeout_ms {
            return Err("http: request timeout must be positive and no greater than the deployment limit".into());
        }
        let body = request.body.unwrap_or_default();
        if body.len() > self.config.max_request_bytes {
            return Err("http: request body exceeds deployment byte limit".into());
        }
        let mut headers = HeaderMap::new();
        for (key, value) in request.headers {
            let key = header_name(&key)?;
            if self.headers.iter().any(|(name, _)| *name == key) {
                return Err("http: app cannot replace a protected deployment header".into());
            }
            if headers.insert(key, header_value(&value)?).is_some() {
                return Err("http: duplicate request header".into());
            }
        }
        for (name, source) in &self.headers {
            let text = match source {
                Header::Value(value) => value.clone(),
                Header::Env { env, prefix } => format!(
                    "{prefix}{}",
                    ctx.env
                        .get(env)
                        .ok_or("http: protected header environment value is missing")?
                ),
            };
            headers.insert(name, header_value(&text)?);
        }
        let headers = headers
            .iter()
            .map(|(k, v)| {
                Ok((
                    k.as_str().to_string(),
                    v.to_str().map_err(|_| "http: invalid header value")?.to_string(),
                ))
            })
            .collect::<Result<_, String>>()?;
        Ok(Prepared {
            method: method.as_str().to_string(),
            url,
            headers,
            body: body.into_bytes(),
            timeout: Duration::from_millis(timeout),
        })
    }

    fn fetch(&self, ctx: &CallContext, request: Prepared) -> Result<J, String> {
        let headers: Vec<(&str, String)> = request.headers.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
        let wire = Wire {
            method: &request.method,
            url: request.url.as_str(),
            headers: &headers,
            body: request.body,
        };
        let failed = |e: HttpError| transport_error(ctx, e);
        let mut response = exchange(wire, Timing::deadline(request.timeout), &ctx.cancel).map_err(failed)?;
        let status = response.status;
        let status_text = response.reason.clone().unwrap_or_else(|| {
            reqwest::StatusCode::from_u16(status)
                .ok()
                .and_then(|s| s.canonical_reason())
                .unwrap_or("")
                .to_string()
        });
        let mut headers: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (name, value) in &response.headers {
            headers.entry(name.clone()).or_default().push(value.clone());
        }
        // Bound actual body bytes: HEAD and 304 can advertise a large representation in
        // Content-Length while carrying no body at all.
        let mut body = vec![];
        while let Some(chunk) = response.next_chunk().map_err(failed)? {
            if chunk.len() > self.config.max_response_bytes.saturating_sub(body.len()) {
                return Err("http: response body exceeds deployment byte limit".into());
            }
            body.extend_from_slice(&chunk);
        }
        let body = String::from_utf8(body).map_err(|_| "http: response body is not UTF-8")?;
        Ok(
            json!({"status": status, "statusText": status_text, "url": request.url.as_str(), "headers": headers, "body": body}),
        )
    }
}

/// A request checked against the binding: what goes on the wire.
struct Prepared {
    method: String,
    url: Url,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    timeout: Duration,
}

// Deliberately omit URLs, headers and bodies from transport errors: they may contain secrets.
fn transport_error(ctx: &CallContext, error: HttpError) -> String {
    if ctx.cancel.is_cancelled() {
        format!("AbortError: {}", ctx.cancel.reason())
    } else if error.message.starts_with("timed out") || error.message.starts_with("stalled") {
        "TimeoutError: HTTP request timed out".into()
    } else if error.failure == Failure::Unsent {
        "http: connection failed".into()
    } else {
        "http: request or response failed".into()
    }
}

impl Executor for Http {
    fn call(&self, ctx: &CallContext, args: &J) -> Result<J, String> {
        if ctx.cancel.is_cancelled() {
            return Err(format!("AbortError: {}", ctx.cancel.reason()));
        }
        let request: Request = serde_json::from_value(args.clone())
            .map_err(|_| "http: expected {path?, method?, headers?, body?, timeout_ms?}, with text headers and body")?;
        let request = self.prepare(ctx, request)?;
        self.fetch(ctx, request)
    }
}
