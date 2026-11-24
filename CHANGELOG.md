# Changelog

## 0.2.0 (beta)

A run is now controlled the way a browser's embedder controls a page, and its files are laid out
by id. Runs written by 0.1 are refused, with what to do (finish them with 0.1, or start over with
`--fresh`). Deployments, apps that use `harness.agent`, and `command` machines need the changes
listed under "Changed".

### Added

- `Control` (`Runtime::control()` in Rust): post an event to an instance, close the run with a
  time limit for running calls, read its status, and change a model binding's limit. A posted
  event is answered once it is committed, and one posted again with the same `key` is delivered
  once. The same protocol runs over stdio (`affordance host`) and over a Unix socket
  (`affordance run --control PATH`). SIGINT and SIGTERM close a run; a second signal closes it at
  once.
- `subscribe {streams: ["status"]}` on the control protocol pushes the run's status each time it
  changes.
- `dispatch(output, {calls: [i]})` runs only the tool calls an app picks, each once. The library's
  loops run a reply's calls one after another by default; `{order: "parallel"}` and
  `{stop_on_error: true}` change that.
- `run.json` records which version of Affordance wrote a run directory. A run of another version
  is refused before anything in it is read, and `--fresh` moves the whole directory under
  `retired/`.

### Changed

- `--resume` is `--restore`.
- A deployment lists its instances as `instances: [{id, app?, count?, ...}]`. An entry with
  `count: N` is N copies.
- Each instance's files are in `instances/<UUIDv7>/`, named by an id the runtime assigns: an
  instance's name never becomes a path. `per_instance` machine places are named by that id.
- The core says instance: `harness.instance` (was `harness.agent`), `InstanceSpec`,
  `add_instance`, `instance.json`, and the status and report keys `instances` and `per_instance`.
- A `command` machine receives `{client: {id, name}, call, env, args}` (was `{agent, task, env,
  args}`). A call sent again after a restore carries the same `client.id` and `call`, so a
  machine doing something that cannot be undone can recognise a repeat. In Rust,
  `CallContext.client` and `CallContext.call`; model captures and hosted session ids say `client`.
- `input_batch` groups journal writes only: each event in a batch takes its own turn.
- The clock stands still while a run closes and while its instances restore, so a timer neither
  fires nor loses time then.

### Removed

- The file-based control: `control/pause.json`, the inbox and receipt files, `model_inflight`,
  `observed.json` and the root `checkpoint.json`. Use the control protocol instead.

### Fixed

- DeepSeek V4.1 without tools kept a past reply's thinking, which its official template drops.
- Eight models sent no pictures at all. A picture rule that fails is now a render error.
- One malformed tool call no longer fails the good calls to the same tool in a reply.
- A restored instance keeps its own secret. A journal missing committed inputs fails its restore
  instead of replaying a shorter history.
- Hosted model bindings report an unreachable server and a cancelled call as the native ones do.
- ssh machines put `--` before the host, and an instance's environment reaches ssh and docker
  machines on standard input, not the command line. Snapshots hide secret values, and an app
  cannot forge fields of the host protocol.
- A streamed reply no longer holds a model thread while a slow reader catches up, and the native
  bindings stream too.

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
