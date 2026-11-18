//! Every integration test, as one program. Each file under `tests/` is a module of it.
//!
//! One program, because the tests share what is costly to make: a tokenizer takes a few
//! hundred milliseconds to load, and as separate programs dozens of them each loaded the same
//! ones. Here a tokenizer is loaded once for the whole run (`shared::tokenizer`).
//!
//!     cargo test --release --features reference-tests --test suite            every test
//!     cargo test --release --features reference-tests --test suite runtime::  one file's tests
//!
//! `tests/closures_freed.rs` is a program of its own: it counts its process's allocations.
// The tests allocate with mimalloc (a test-only dependency: Cargo.toml says why).
#[global_allocator]
static ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[path = "support/each.rs"]
pub mod each;
#[path = "support/http_fixture.rs"]
pub mod http_fixture;
#[path = "support/shared.rs"]
pub mod shared;
#[path = "support/tokenizer.rs"]
pub mod tokenizer;

mod abort;
mod adapter_contract;
mod agents_bin;
mod any_model;
mod channel_scaling;
mod chat_client;
mod chat_render;
mod chat_tools;
mod cli_app;
mod cli_run;
mod conformance;
mod conformance_inventory;
mod courier;
mod decision;
mod effects;
mod features;
mod fetch;
mod first_agent;
mod group_commit;
mod hardening;
mod harness_consumer;
mod host;
mod hosted_client;
mod http_connector;
mod images;
mod inspect;
mod js_differential;
mod js_semantics;
mod lifecycle;
mod lint;
mod machine_stdin;
mod mcp;
mod media_layout;
mod memo;
mod memo_miss;
mod middle;
mod model_bindings;
mod modules;
mod page;
mod pause_latency;
mod promises;
mod regex_async;
mod reliability;
mod reply_coverage;
mod resume_image;
mod review;
mod runtime;
mod runtime_http;
mod runtime_model;
mod safe_text;
mod scenarios;
mod serve;
mod session_agent;
mod sglang_sessions;
mod spill;
mod template_sweep;
mod third_party;
mod tool_traces;
mod tools;
mod ts_syntax;
mod turn_spans;
mod values;
mod window;
