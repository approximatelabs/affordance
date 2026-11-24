# Working in this repository

Notes for coding agents (and people) changing Affordance. The README says what it is; this file
says how the tree is laid out, how to check a change, and what a change must not break.

## Layout

| Path | What it holds |
|---|---|
| `src/` | The Rust library: parser and interpreter for the TSX subset (`parser.rs`, `interp.rs`, `js*.rs`), layout and token emission (`emit.rs`, `style.rs`, `turns.rs`), reply parsing (`reply.rs`) |
| `src/runtime/` | The runtime: one instance's event loop (`instance.rs`), the host that runs many (`host.rs`), model bindings (`model.rs`), machines (`exec.rs`), journals and recovery images |
| `src/bin/affordance/` | The `affordance` command: `run` a deployment (`affordance.yaml`), `serve` it over HTTP, `host` it for a frontend over stdio, `config`, `init`, and `render`, `test`, `lint`, `check` |
| `views/` | TSX: chat-template apps per model family, libraries (`views/lib/`), context strategies, demos |
| `spec/` | The normative contracts: `LANGUAGE.md` (the TSX subset and layout), `HARNESS.md` (instances, effects, replay), `CONTRACTS.md` (the map) |
| `docs/` | Guides: getting started, runtime, model bindings, embedding, development |
| `tests/` | Integration tests; `tests/fixtures/` holds small independent fixtures |
| `examples/` | Example apps (`first_agent/`: `app.tsx` + `affordance.yaml`, run with `affordance run`) and, for embedding the engine in Rust, `embedding/hello.rs` |
| `tools/`, `bench/`, `ref/` | Development only: checks, benchmarks, reference-output generators |

## Check a change

```sh
cargo test --release                       # every test; reference-data tests are ignored without ref/
./tools/check-engine.sh                    # the PR gate: tests, fmt, clippy -D warnings, docs, lint, package
cargo run --release --bin affordance -- lint      # TSX in views/, tests/fixtures/, spec/ and docs/
```

Tests that compare against official tokenizer and template output need reference data under
`ref/`; [docs/DEVELOPING.md](docs/DEVELOPING.md) explains how to fetch it and run
`cargo test --release --features reference-tests`. A template change must keep every conformance
set exact (`affordance check`).

## What must stay true

- **Exactness.** A render's token ids are the product. Template apps reproduce the official chat
  template's ids, a past reply included: it is written the publisher's way by default. With
  `keep_replies`, or where the publisher writes a reply as generated, an agent's prompts are
  exact appends of the previous prompt and reply unless its context policy folds.
- **Replay.** An agent is rebuilt from its journal. Anything a handler observes (time, randomness,
  results, model replies) arrives as a journaled input; nothing in an instance reads the outside
  world directly.
- **Ownership.** Application policy (tools, conversation, context folding) is TSX in `views/`.
  The engine adds a primitive only when no composition of existing ones does the job
  ([docs/DESIGN.md](docs/DESIGN.md) shows the reasoning on real decisions). See
  "Changing the platform" in [CONTRIBUTING.md](CONTRIBUTING.md) before changing `src/runtime/`,
  `harness.*` or journal semantics.
- **No compatibility layers.** Change the callers in the same commit.

## Style

- Rust: `rustfmt.toml` (120 columns) and clippy with `-D warnings`.
- Comments say what a thing is or why it is so, in plain sentences. No ticket numbers, dates or
  names of people or machines.
- A test's name states the behavior it checks; its assertions check that behavior, not an
  implementation detail.
- Errors name what failed and how to fix it; they never include a server's address or a secret.

## Writing (docs, the site, READMEs, posts, commit messages)

- Titles and headings say plainly what was measured or built: the model, the task, the number
  with its unit, and what it was compared against. No teasers, hooks, theses or story frames.
- Write for a reader who is curious and knows nothing yet. Define a term in plain words before
  using it, and open with what was done and what happened, not with a grand claim.
- Put the number where the claim is, before and after, with the comparison named, so a range
  never reads as a regression.
- Docs say what is. No dated history or change notes ("removed on ...", "we used to ...").
- No staged prose: no "X, not Y" reveals, rhetorical questions, one-line punchlines, closing
  aphorisms, fragment runs, triads, "the key thing is" designations, em dashes or emoji.
  Sentences a person would say aloud, about 12 to 20 words, with a person or a measurement as
  the subject.
- After drafting, reread every title, heading and paragraph opening against these rules, then
  render the page and read its first screen before showing it to anyone.
