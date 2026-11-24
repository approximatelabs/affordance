# Architecture

Affordance has one engine, implemented in Rust. Applications and reusable application policies
are written in the TSX subset described in [LANGUAGE.md](../spec/LANGUAGE.md).
[CONTRACTS.md](../spec/CONTRACTS.md) maps the public boundaries and their guarantees.
File modules isolate declarations and share values only through explicit imports/exports;
functions and closures retain their defining scope during calls and replay. Runtime source
loading (`loadModule` from `affordance:compiler`) uses those same owned lexical environments
and the same executor; apps supply dependency namespaces and own revision selection.

The renderer loads an app and its imports, evaluates it against state, lays out its token
fragments, and returns exact token IDs plus the actions still visible in the final layout.
Adapters project those actions into each model's protocol. Cropping never leaves an invisible
action callable by the model.

An instance's state machine takes one input and returns commands. The host runs model requests,
machine calls, timers, and message delivery, then feeds their results back as journaled inputs.
Each instance owns its state and children. The host owns external I/O, permissions, and durable
storage. Replaying the journal reconstructs the instance. Eligible idle/stopped data states
can use a verified checkpoint plus the newer journal inputs; full history stays on disk.

Conversation logs, folding, agent loops, and task-specific tools are TSX library or app code.
The core provides the operations these policies need; it does not choose a task or training
algorithm. Model servers and command executors are external bindings.

`src/connectors/` supplies host integrations through the existing executor interface. The HTTP
connector understands HTTP requests and responses; the MCP implementation is a TSX library
over global Fetch (`views/lib/mcp.tsx`). The embedded standard library (`src/web.tsx`) provides
Fetch, Headers and buffered Response behavior over the same executor boundary, without MCP
in the engine. Apps choose which discovered tools become affordances. See
`docs/HTTP_AND_MCP.md` for deployment and replay behavior.

## Repository boundary

The public framework consists of `src/`, reusable `views/`, the language/runtime specifications,
API guides, regression tests, and small examples. The one installed binary is `affordance`.
Rust benchmarks are development examples under `tools/bench/`.

Python in `ref/`, `bench/`, and test support is development tooling. It supplies independent
reference outputs and tests interoperability; no Python process is required to use the Rust
renderer or generic runtime. Examples that expose a Python tool do so explicitly through a
machine binding, as an application choice. Their executable helpers live in
`tools/examples/` in the development checkout and are excluded from the runtime package.

Training, game emulation, evaluation controllers and experimental model runners are a separate
project, `affordance-research`. Its results do not define the framework's behavior or install
dependencies.

A small Courier app remains as an example of state, layouts, and actions. Its training code,
model weights, datasets, and game-specific executables belong to the research project.

## Release boundary

`Cargo.toml` lists the files included in the framework source package. Development caches and
run output are ignored. A release must pass the validation in `docs/DEVELOPING.md`; publication
also requires project license and release metadata to be chosen by the project owner.
