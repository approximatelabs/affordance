# Changelog

## 0.1.0 (beta)

The first public release.

- A TSX view language, interpreted in Rust, that renders state into exact token ids with
  layout under a token budget (token style sheets).
- Prompt formats for 740 models (57 written by hand, 135 more that use one of those formats
  unchanged, and 548 converted from the publisher's chat template), matching the official templates'
  token ids on the conformance corpus
  (`affordance check`). `tools/add_model.py` adds a model.
- Pictures and sound sized in tokens by the style sheet (`img { max-tokens: 256tok; }`), sent
  at the size laid out; a video shown as sized frames ([docs/MEDIA.md](docs/MEDIA.md)).
- An agent runtime: an event loop per instance, model bindings (SGLang, OpenAI-compatible
  completions and chat, Anthropic Messages, OpenAI Responses), machines (local, SSH, Docker),
  timers, messages, child instances, journals with deterministic replay, and recovery images.
- One command, `affordance`: `run` a deployment (`affordance.yaml`, found from the current
  directory upward, layered with `affordance.local.yaml`, `-f` files, `--set` and `${VAR}`),
  `config`, `init`, `render`, `test`, `lint` and `check`. The TSX library is bundled
  (`@affordance/...` imports).
- `affordance serve`: an app behind HTTP. Requests arrive as `request` events, answers go to the
  machine `client` (whole or streamed). `views/lib/openai_server.tsx` builds an
  OpenAI-compatible endpoint on it; `views/demo/memory_server` is a chat model with a memory
  longer than its context, for any OpenAI client.
- `AbortController` and `AbortSignal`: `model()`, machine calls and `fetch` take `{signal}`. An
  aborted model call resolves with what it generated (journaled, so the log keeps the partial
  reply); a machine call or `fetch` rejects with `AbortError`.
- The chat library for frontends (`views/lib/frontend.tsx`) reports each model reply, takes a
  `before` step (where an app folds its context), and lets an embedder replace the conversation
  (`reset`, `continue`); `useLog` gains `reset(records)`. With these, pi runs with Affordance as
  its agent core: pi's loop is an Affordance app, and pi's interface is unchanged.
- `views/lib/model_layer.tsx`: Affordance as another harness's model layer. The harness sends each
  model call (system prompt, messages, tools) and gets the parsed reply back, streamed; its loop,
  tools and sessions stay its own.
- `affordance host`: a deployment driven by a frontend over stdio (JSON lines in and out: prompts,
  aborts and answers; reply deltas, command output, trace records and questions on the machine
  `ui`). `views/lib/frontend.tsx` is its chat loop, and `views/demo/host` a coding chat that asks
  before each command.
- `views/tools/files.tsx`: `read`, `write` and `edit` tools over a machine's shell (library TSX).
- Editor types: `affordance init` writes `affordance.d.ts` and `tsconfig.json`.
- `useLoop` ends on a reply with no tool call, with the reply as the instance's answer; before,
  it asked again with the same page until the turn limit. `idle(out)` lets an app go on.
- `affordance run` exits with a failure when an app stops on an error (`loop_error`).
- `affordance test` runs the app `affordance init` writes (`"template": "messages"` in the
  scenario), and refuses a scenario with no `steps`.
- A model binding's `retry`: one policy for what happens after a model call fails
  (`{for_s, wait_s, on, refused}`; 300 s under the command, `forever` for a fleet). A call can
  state its own: `model(view, {retry: ...})`. Model clients make one attempt and name the kind of
  failure; every further attempt is journaled and traced. The command prints a
  failing model call, and a reply whose tool call the server left as text, on stderr.
- `Date.now()` (the journal's clock) and `console.log` and friends (the instance's trace, never
  journaled); a missing browser global names what to use instead.
- Streaming to live views: `RuntimeConfig.observer` sees native ids, hosted text and reasoning
  (chat, Anthropic Messages and Responses now stream over server-sent events), and a command's
  output as it is written.
