# Examples

An Affordance app is a folder with an `app.tsx` and an `affordance.yaml`, run with
`affordance run` in that folder. You do not write Rust to build one.

- **[first_agent/](first_agent/README.md)**: start here. A complete agent in one TSX file: it
  reads its task, may run shell commands on your machine, and stops when it answers.
  `affordance run` in its folder runs it against a local model server (llama.cpp, Ollama,
  vLLM) or a hosted API. `affordance init` writes a copy to start from.
- More apps: a long-session agent with context folding ([`views/agents/session.tsx`](../views/agents/session.tsx)),
  and in [`views/demo/`](../views/demo) a chat, a research agent comparing context policies, a
  memory that outlives its sessions, and a recursive language model (RLM).

Only if you are embedding the engine in your own Rust program:

- **[embedding/hello.rs](embedding/hello.rs)**: load an app and a tokenizer, render it, call an
  action and render the result, from Rust (`cargo run --release --example hello -- TOKENIZER.json`).
