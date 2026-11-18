//! The agent runtime: the "browser" that runs agent apps (views) as tabs.
//!
//! - [`agent`]: one agent as a deterministic state machine with no I/O (journaled inputs in,
//!   commands out). Replaying its journal reproduces it exactly.
//! - [`host`]: the multi-agent runtime: a shared tokenizer, render workers, the model client,
//!   primitives (`exec`, `send`, `wait`, `store.*`), the app-scoped environ, journals and traces.
//!
//! See `docs/RUNTIME.md` for the app contract and the primitives.
pub mod agent;
pub mod blobs;
pub mod cancel;

pub use agent::{Agent, AgentConfig, Command, GenOptions, Input, Metrics};
pub mod exec;
pub mod host;
pub mod http;
pub mod inspect;
pub mod model;
mod model_config;
pub mod store;

pub use cancel::CancelToken;
pub use exec::{CallContext, CommandMachine, Exec, Executor, Output, Target};
pub use host::{AgentReport, AgentSpec, Inbox, Observed, Observer, Report, Runtime, RuntimeConfig};
pub use model::{
    Api, Chat, Completions, Credentials, Delta, FnModel, GenReply, GenRequest, Model, Sessions, Sglang, Window,
};

mod checkpoint;
mod jsonl;
