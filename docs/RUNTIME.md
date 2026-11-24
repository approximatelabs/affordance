# The agent runtime

Contract authority: [public contracts](../spec/CONTRACTS.md),
[language](../spec/LANGUAGE.md) and [harness](../spec/HARNESS.md).

An agent app is one view file. The runtime runs many agents of it, the way a browser runs many
tabs: it renders each app to token ids, sends them to a model server, reads the reply, runs the
app's handlers, and serves the effects they ask for. The runtime knows nothing about any app.
It defines no app-specific tool vocabulary, state schema, template tokens or fold policy.

- Contract: [HARNESS.md](../spec/HARNESS.md); ownership: [CONTRACTS.md](../spec/CONTRACTS.md).
- Code: `src/runtime/`.
- Binary: `src/bin/affordance/`.
- Example apps:
  - `views/demo/runtime/agent.tsx` (minimal);
  - `views/agents/session.tsx` (a long-session agent, with folding).
- Tests: `tests/runtime.rs`, `tests/session_agent.rs`, `tests/agents_bin.rs`.

```sh
affordance run              # the deployment in affordance.yaml (here or in a parent directory)
affordance run --restore     # continue from its journals
affordance config           # print the deployment a run would use
```

## The deployment file

A deployment (`affordance.yaml`) names the app, its model bindings, its machines and its agents.
`affordance run [PATH]` reads PATH (a file, or a directory holding `affordance.yaml`), or else
`affordance.yaml` in the current directory or the nearest parent with one. It is YAML (a JSON
deployment is valid YAML too); unknown keys are errors.

- **Layers**, lowest first: the file; `affordance.local.yaml` beside it, if present (for values
  that are not committed: a URL, a count); each `-f FILE`; each `--set KEY=VALUE` (KEY a dotted
  path, `agents[0].max_turns` for a list element; VALUE parsed as YAML); `--dir` and `--app`.
  Maps merge key by key, lists and scalars replace, `null` removes a key.
- **Environment:** `${VAR}` and `${VAR:-default}` in a value are filled at load; an unset
  variable with no default is an error naming the file and key. A model key comes from the
  environment with `key_env` (below), not through the file.
- **Paths** in a file are relative to that file: the app, tokenizers, key files, `library`,
  `dir`, and local machines' homes. Paths given on the command line are relative to the current
  directory, so a deployment runs the same from anywhere.
- **The journal directory** is `dir`, by default `.affordance/` beside the file. A run that
  finds an earlier run there stops and asks for `--restore` (continue it) or `--fresh` (start over;
  the earlier run's instances move under `retired/`).
- **`affordance config`** prints the resolved deployment (same arguments as `run`), with the
  values of `secrets` hidden.

## Writing an app

```tsx
import { ModelContext } from "@affordance/lib/model.tsx";
import { useLoop } from "@affordance/lib/loop.tsx";
import { useLog } from "@affordance/lib/log.tsx";
import { Tool } from "@affordance/lib/tool.tsx";
import { machineTools } from "@affordance/tools/machine.tsx";

export default function Main({ task }) {
  const log = useLog();                                        // what the model saw and said: app state
  useLoop(log);                                                // ask the model, run its calls, repeat
  const [notes, setNotes] = useState("(none)");               // persisted by the runtime
  const {shell} = machineTools(harness.machine.get("home")); // the deployment binds this role
  /** Replace your notes. */
  function set_notes({text}: {text: string}) { setNotes(text); return "saved"; }
  const app = <>
    <box label="status">{`${harness.agent} | notes: ${notes}\n`}</box>
    <Tool fn={shell}>{"[shell]"}</Tool>
    <Tool fn={set_notes}>{" [set_notes]\n"}</Tool>
    <aff event="message" onCallback={async ({ from, text }) => {
      log.append({kind: "input", content: `[message from ${from}] ${text}`});
    }}/>
  </>;
  return <ModelContext strategy="unrolled" log={[{kind: "input", content: task}, ...log.records]}>{app}</ModelContext>;
}
```

Prefer imported tool functions through `<Tool fn={...}>`. `machineTools(machine)` in
`views/tools/machine.tsx` creates `shell` from an explicitly selected machine capability;
its output is `{result, exit, timed_out, seconds, ...}`. The same module exports `wait`,
which takes optional `seconds` and event names (`events`), with at least one required.
Apps own output truncation, process/file-watcher protocols and host instructions. A typed
function imported from another file retains its defining module's docs and schema. Use
`name`, `description` or `schema` overrides when exact names or integer constraints matter.

The contract:

| What | How |
|---|---|
| The screen | The app's render: the current turn of a template app (`TurnStrategy`'s final user turn). |
| Tools | Functions: `<Tool fn={lookup}/>` (`views/lib/tool.tsx`) takes the name from the function, the description from its JSDoc and the argument schema from its parameter's type. Underneath, `<aff>` elements with an `on*` handler (`tool_name`, `description`, `args_schema`). |
| Tool results | The handler's return value: a string, any JSON (sent as JSON text), or `{result, ...meta}`. The meta keys stay on the log's `action` record for the app; templates show only `result`. A thrown error becomes `error: <message>`. |
| State | `useState` hooks, persisted and replayed by the runtime. Handlers change state only through setters. |
| History | App state: `useLog()` (`views/lib/log.tsx`) keeps what the model saw and said (see below). The app chooses what to show by passing `log.records`, filtered and reduced, to `TurnStrategy`. |
| The model | The app calls it: `model(view?, opts)` sends the page's own view (or an element) and resolves with the output; `dispatch(output)` runs its calls on the view the model saw. `log.step()` does both and writes the records. `views/lib/loop.tsx` has the common loops: `useLoop(log, {before?})` (ask, act, repeat: an autonomous agent) and `useConversation(log)` (answer whatever is unanswered: a chat). The runtime never calls the model on its own. |
| Effects | In a task: `await harness.machine.call(args)`, `harness.store.*`, `harness.send`, `harness.wait`, `model`, `dispatch`, ... (below); `Promise.all([...])` runs several concurrently. `harness.now()`, `harness.random()` and `harness.stop(value)` are also available. `useEffect(setup, deps)` runs `setup` after a render whose deps changed (by `Object.is`, as in React). |
| Events | `useEvent("message", listener)` (or `<aff event="message" onCallback={...}/>`, zero tokens). A listener runs when its event arrives, even while tools wait. |
| Context window | Layout's: the binding's `context` (the model's context length, which a native binding's server reports when the runtime validates it; configured, it may only lower that; `"none"` opts out; logged per binding and in the run status's `models`) less `max_new` is every prompt's window; sheets cap boxes in it (`history { max-tokens: 75win; overflow: drop-start; drop-step: 25% }`, `result { ... clip-middle }`), and a prompt that does not fit is never sent (`ContextError`). An app that summarizes reacts to `useEvent("layout")`. `max_prompt` stays as a hard ceiling. |
| Reading replies | The template adapter's `protocol` span: the syntax, the stop tokens, and whether the prompt ends inside thinking. Every stock template app declares it: GLM53App (`glm`), Qwen3App and Qwen3BaseApp (`hermes`), Qwen36App, MiMo25App and Nemotron3App (`xml`), Llama31App (`llama31`). Parsers exist for every rendered family (`docs/REPLY_PARSING.md`). |
| Language | TypeScript/JSX with JavaScript semantics (`spec/LANGUAGE.md` §7), in apps and template adapters alike. |

`harness` in a render:
- `agent` (id);
- `turn` (page model calls so far);
- `clock_ms`;
- `env` (the app-scoped environ);
- `prompt_tokens` (length of the last prompt);
- `machines` (the roles of the machines bound to this agent);
- `model` (`{name, template, template_options}`, or null without a model);
- `probe` (only inside a `measure` render).

## The log

For a fleet with bursty messages, set deployment `"input_batch": 64` and use
`harness.send(memberIds, payload)` for one durable multicast result. A hub that is a top-level
spec can set `"dedicated_worker": true`; its descendants share the isolated worker. The
shared `workers` pool remains available to other roots. Channels and membership stay app code.
Batching groups journal writes only: each event still takes its own turn, so an app computes the
same with or without it. Details and replay rules are in [HARNESS.md §3](../spec/HARNESS.md#3-the-loop).

The runtime keeps no conversation. `useLog()` keeps one
in app state, writing records from `model()`'s output and `dispatch()`'s results:

| Record | Fields | When |
|---|---|---|
| `screen` | `ids` (the exact screen tokens), `images?`, hosted `content?` | The model replied to it (`output.screen`). |
| `model` | `ids` (exact generated ids, stop token included), `reasoning`, `content`, `calls`, `problems`, `cut_off`, `provider_state?` | Each reply. |
| `action` | `name`, `tool_call_id`, `result`, plus the app's meta keys, plus `error: true` | `dispatch` ran the call, or it was invalid (unknown tool or bad arguments, answered with an error result the model sees). In call order. |
| `input` | `content` | `log.append(record)`, e.g. an incoming message. While a turn is open, appends are held and written after its results. |

`useLog` assigns every committed record a stable `id` (`e0`, `e1`, …; named logs use
`<name>:e0`). These identify records; `ids` still means exact token IDs. The library exports
`LogRecord` and the four record types. `recordText(record)` produces search/diagnostic text,
including reasoning and calls; image parts appear as `[image]`. `log.get(id)` reads a full
record, including one released to storage; `log.read(from,to)` reads a half-open range.
Release preserves IDs on skeletons. Read a released record before formatting it.

For summaries, render the ordinary screen through `ModelContext callable={false}` (or
`TurnStrategy callable={false}`). Tool descriptions and screen tokens stay identical, while
the projected tree has no model-callable handlers. Hosted bindings also disable provider tool
invocation, overriding binding/per-call settings that require a tool; a violating response is
an error. No stub-handler table or provider-specific summary option is needed.

Records with `ids` re-render to exactly those ids (`docs/HARNESS_GUIDE.md` §5). A template app
writes a past reply the way the model's publisher does. Where the publisher writes it as the
model generated it, or the binding sets `keep_replies` (`docs/MODEL_BINDINGS.md`), every turn
whose history the app does not rewrite is an exact append, and the runtime counts both kinds:
`metrics.exact_appends` vs `rewrites`, and `trace.jsonl`'s `append` field.

Hosted records keep sent screen content and opaque provider continuation state, rather than duplicate JSON reply token IDs. Provider state must travel unchanged during ordinary turns; clear it explicitly when rewriting an assistant record. Provider prefix rules across folds are binding/app policy (see MODEL_BINDINGS.md).

To reduce a record, the app maps it, e.g. `{...r, reasoning: "", provider_state: null, ids: null}`; dropping `ids` makes
it render from its fields. That is how `views/agents/session.tsx` folds.

Reply problems come from `reply.rs`: `no_action`, `unclosed_think`, `cut_off`, `bad_syntax`,
`unknown_action`, `bad_args`, ... A reply with no tool call is allowed: the next screen follows,
and the app can nudge. A reply cut off before `</think>` is thinking, never content. A cut-off
tool call is never run.

## Harness APIs

| Call (in a handler) | Result | Served by |
|---|---|---|
| `fetch(input, init?)` | `Promise<Response>` | The bound `http` executor, with standard response readers. See [Fetch and MCP](HTTP_AND_MCP.md) for the supported Web API subset. |
| `harness.machine.call(args)` | executor-specific JSON | The agent's first machine (below). Shell executors take `{cmd, timeout_s?, env?, stdin?}` and return `{output, exit, seconds, timed_out, truncated}`. `harness.machine.get(role).call(args)` picks a named executor. |
| `model(opts?)` | an output: `{text, reasoning, calls, problems, cut_off, ids, screen, usage}` | the agent's model: the page's own view, laid out once, at the call. |
| `model(<App .../>, opts?)` | an output (no `screen`; not dispatchable) | the agent's model, through the harness: the component is laid out to exact ids and run once; its `protocol` span reads the reply. |
| `dispatch(output, {calls?})` | `[{name, id, result, error?, ...meta}]` | agent: runs a page output's calls (or those at the indices in `calls`) on the view the model saw; each call runs once, and a later dispatch of it gives its result again. `log.dispatch(out, {order, stop_on_error})` runs them one after another by default. |
| `harness.send(to, payload, {event?})` | `{to, delivered}` | runtime: event `message` (or `event`) to agent `to`, with `payload.from` set by the runtime. |
| `harness.wait({ms?, events?})` | `{woke: "timeout"}` or `{woke: "event", event, payload}` | agent: parks the handler (no model calls) until `ms` passes or an event in `events` arrives. |
| `harness.store.put/get/search/list(args)` | `{key}`; `{text, total}`; `{hits}`; `{keys}` | runtime: per-agent storage (persisted with `dir`). |

## Machines

A machine binding names an external executor: a shell, HTTP endpoint, or custom adapter.
The deployment binds machines by role; an agent reaches only
the machines bound to it (`harness.machines`), and the host checks every call against that
binding. There is no other way out: running on the harness's own host means binding a `local`
machine.

| Kind | Config | Runs |
|---|---|---|
| `local` | `home?`, `per_instance?` | `bash` on this host, with only the agent's environ, in `home` when given (with `per_instance`, each instance its own directory under it, `home/<instance id>`). |
| `ssh` | `target`, `key?`, `control_dir?`, `known_hosts?` | `bash` on a VM over a shared SSH connection. A host's key is accepted the first time it is seen and checked from then on. The known-hosts file and the connection sockets are kept in `~/.config/affordance/ssh`, which only this user can enter, unless the deployment names other places. |
| `docker` | `image`, `name?` (default `aff`) | `bash` in a container per instance, `<name>-<instance id>`, started on first use and kept. |
| `bwrap` | `home`, `per_instance?`, `network?` (default false), `read?` | `bash` in a bubblewrap sandbox. The command sees the system's programs and libraries read-only (`/usr`, `/bin`, `/sbin`, `/lib*`, `/etc`, `/opt`), its `home` writable and a private /tmp. No other file on the host is there for it: not a home directory, not mounted data. `read` lists more paths to show read-only. Linux, where the kernel lets an ordinary user make a sandbox. |
| `command` | `argv` | An external executor: JSON requests on stdin, JSON responses on stdout. |
| `http` | `url`, `headers?`, `timeout_ms?`, byte limits | HTTP requests on a bound origin. Grant the role `http` for global `fetch(input, init)`. See [Fetch and MCP](HTTP_AND_MCP.md). |

An instance's place on a machine is named by the id the runtime gave it, never by its name. An
instance's `machines` lists the roles it is bound to; without it, every deployment machine. A
pool, `"vm-{n:03}": {..., count: 512}`, binds the roles `vm-000` to `vm-511`, with `{n}` (the
member's number) filled in each member's fields; a root app hands them out (`<Mount machine>`).

## Folding (context policy in the app)

`views/agents/session.tsx` keeps its fold state in a hook and renders history through
`selectLog(log.records, fold)`. Its sheet (`session.tss`) caps the history at 75% of the window
and tool output in tokens. Its fold policy reacts to the layout event: while a laid-out prompt
passes `c_hi`, it takes one step per turn over the records before the recent window:

1. mask long tool outputs (level 1), then drop reasoning and old screens (level 2);
2. then summarize: `model(<SessionContext/>)`, the agent's own context plus a maintenance
   request (same prefix, so it reuses the model server's cache), and release what it covers.

Each step shows in the next prompt's layout; nothing measures candidate renders.

The fold is state, so prompts are rewritten only when it changes, in bulk. Every other turn is an
exact append.

`tests/session_agent.rs` runs the consumer's 300-turn scenario on this app, with a scripted GLM
model (bound with `keep_replies`) and a fake shell, at 48K/20K. The following baseline predates binding-selected templates;
the current scenario continues to check exact appends, bounded folding, recall and replay:

| | |
|---|---|
| invariant violations | 0 |
| exact appends | 285 of 299 (the other 14: 12 folds and 2 branch returns) |
| fold landings | 17.9–19.9K, plus 1 summarized at 7.7K |
| max prompt | 47,972 |
| prefix reuse | 94.5% |
| recall from folded history | 3/3 questions |
| `read_event` | byte-exact, 2/2 |
| journal replay | identical state (the log included) |

These match the consumer's own Rust harness: 13 folds, 284 exact appends, 94.1% reuse. The run
takes 8.6 s for 300 turns on one thread, test scaffolding included; the mean turn render is 3.0 ms.

## The environ and secrets

Each agent's `harness.env` has two layers: the deployment layer (`env` in the config), then the
agent's own `env`. The runtime process's real environment is never visible.

- A local `exec` child gets only `PATH`, `HOME`, `LANG` and the agent's environ.
- An app may read its secrets and use them, the way a page reads its cookies. Showing a secret to
  the model is the app's choice.
- Values of keys listed in `secrets` are redacted (`[secret:KEY]`) in `trace.jsonl` and
  `agent.json`, as values (a PEM key with newlines and quotes is found). On restore they come
  from the config again. A secret under 4 bytes is refused: it cannot be told from text.
- `journal.jsonl` stores inputs verbatim (replay needs them). Treat it as sensitive.
- The runtime's own credentials (SSH keys, endpoints, API keys) are machine and model
  configuration and never reach an app.

## Running many agents

`affordance run`, or `Runtime` from Rust. The contract is `spec/HARNESS.md`; this section is how
to run it.
- **The instances.** `instances` lists them: `[{"id": "root", "app": "fleet.tsx", "props": {...}}]`
  runs a root app that mounts the rest (`views/demo/tree/fleet.tsx` mounts one agent per machine
  of a pool), and `[{"id": "a"}, {"id": "b"}]` runs the deployment's `app` twice. An entry with
  `count: N` is N copies, `<id>-0` to `<id>-(N-1)`, with the same fields.
- **Where apps import from.** An app's own directory (and below), and the library, which
  `@affordance/...` imports name: `library` in the deployment; else the library checkout the app
  lives in (the nearest directory above it holding `index.json`, or a checkout's `views/`); else
  the copy bundled with `affordance`. Any other import is a load error.
- **One `Encoder` per model binding.** Render workers each own the views they load and a set of
  instances (children live with their parent).
- **Pools.** Model and machine calls run on thread pools (`model_threads`, `exec_threads`).
- **No polling.** An agent waiting on a slow command, a `wait` or a message costs no model calls.
- **DP-rank pinning.** Model calls are pinned to a data-parallel rank (`rank` per agent, or a
  stable hash), so each agent's prefix stays in one rank's cache.
- **Model servers** (`model`, and more by name in `models`):
  - `sglang`: `/generate` with `input_ids`, `stop_token_ids`, `no_stop_trim`,
    `routed_dp_rank`;
  - `completions`: `/v1/completions` with a token-id prompt and `return_token_ids`;
  - `chat`: an OpenAI-compatible `/chat/completions` (`url`, `model`, a key from `key_env` (an
    environment variable) or `key_file` + `key_name` (a file's `NAME=value` line), none for a
    local server, optional non-secret `headers`); the app renders through `MessagesApp`. With no
    `tokenizer`, a chat binding sizes its layout with the bundled default (GPT-2), since the
    server tokenizes the messages itself. Provider usage is preserved
    without cost interpretation. Prices, spending enforcement and account state belong to the
    host; legacy price/budget fields are rejected. See the model section in `spec/HARNESS.md`.

  A model client makes one attempt. When it fails, the binding's `retry` policy decides whether
  the agent makes the call again with the same prompt: `{for_s, wait_s, on, refused}`, described
  in "Model call failures and retries" in `spec/HARNESS.md`. By default an unreachable, cut, busy
  or failing server is tried again after 1 s, doubling to 30 s. Under the `affordance` command
  that goes on for 300 s unless the binding says otherwise; `retry: {for_s: forever}` keeps a
  fleet waiting through any outage. One call can state its own policy:
  `model(view, {retry: false})`. After the last attempt the app's `model()` gets the error. The
  command prints each failing call and what happens next on stderr. Streams stall
  after `stall_s` (default 5400 s).
  Model configurations are strict, including `provider_options` and per-binding bearer
  credentials. Native LoRA selection uses `provider_options.lora_path`; it is not sampling.
  See [the configuration contract](../spec/HARNESS.md#6-models) and
  [multi-binding examples, sessions and recovery](MODEL_BINDINGS.md).
- **Files in the journal directory (`dir`):**
  - `<dir>/instances/<id>/agent.json`: its name and spec (each instance lives under an id the
    runtime assigned, never under its name);
  - `journal.jsonl`: every input, appended before it takes effect (durable at the next
    commit, below);
  - `trace.jsonl`: renders with prompt size, append check and render time; replies with
    problems; handler errors (a handler that threw, and a promise that rejected with nothing awaiting or
    catching it, such as an async function called and not awaited: `unhandled_rejection`); model errors;
    each machine call a tool makes (`call`: its task, tool and call id, so live output can be put under its tool call);
  - `store.jsonl`;
  - optional `checkpoint.json` (a verified recovery image; full journal retained);
  - `<dir>/blobs/<sha256>`: large strings (below);
  - `<dir>/commit.jsonl`: the journals' commits (below);
  - `<dir>/runtime.lock`: locked by the one runtime that owns the directory. A second runtime
    over it (`--restore` or a fresh run) fails before it recovers anything; a crash releases it.
- **Large strings live on disk.** With `dir`, a string of at least `spill_bytes` (default 64 KiB)
  is written once to `<dir>/blobs/<sha256>` and travels through state, journal,
  store and commands as a short reference. The app sees an ordinary string: the language reads
  it as host text mapped from the file (`length`, `slice`, `indexOf`, ... read the mapped
  bytes without copying the whole string), so its pages are reclaimable file cache, not heap.
  `exec` writes an output past that size straight to a blob file as it arrives. Machine calls and
  reports get the text back; `send` and `store.*` pass references through. A 16 MiB tool output
  costs about a hundred bytes of heap wherever the app keeps it. Without `dir`, strings stay on
  the heap.
- **Journals are committed in groups.** A worker writes an input's journal line when it applies
  the input and does not wait for the disk. A commit thread makes the lines durable together: it
  syncs every journal written since the last commit (in parallel) and then appends each one's
  length to `commit.jsonl`. It commits as soon as something waits on a commit or a worker has
  nothing queued, and otherwise at most 2 ms after the first line it covers. Whatever leaves the
  process waits for the commit of the input that caused it: model requests, machine calls,
  and the answer to an embedder's post. A `harness.send` to another instance, timers, the
  instance tree and `store.*` operations do not wait. Store records and blob files are written
  without a sync too. A commit syncs them with the journals, before the record that commits
  any line referring to them, and the commit records each store's length beside its journal's. A model-free round trip between two instances costs a fraction of a millisecond instead
  of four disk syncs. This is the browser's IndexedDB with relaxed durability: a transaction
  completes before it reaches the disk, and the platform flushes in the background. Here,
  though, nothing outside the process sees a result before it is durable.
- **A crash restores from the last commit.** A restore first truncates every journal and store to
  the last committed length (bytes past it are kept in `uncommitted-journal-<offset>.bin` or
  `uncommitted-store-<offset>.bin`). A file the commit does not name for its instance's
  incarnation is emptied. A blob a committed line refers to was synced by that commit. A blob
  file a crash left without its content is rewritten when its text is next stored: a put checks
  an existing file's content once per process before reusing it. A line is written only after
  every line that caused it, in any journal, so a commit never holds a delivery without the
  send that caused it, nor a sender's acknowledgement without the delivery. The run restores in
  a state it passed through. The work after the last commit (a few milliseconds of in-process
  work) is computed again, and nothing outside the process saw it. A recovery image taken after
  the last commit is ignored, and that agent replays its journal in full. A clean shutdown
  commits everything and closes the log, so the next restore truncates nothing. Only the log's
  last line may be torn: a bad line before it is damage, not an older commit, and recovery
  refuses it (naming its byte offset) rather than cut every journal back. Each truncation is
  printed with the instance id and byte counts.
- **Restore.** `--restore` rebuilds every agent from its journal (same renders, same state) and
  resubmits its outstanding work. External work in flight at the crash is submitted again, so
  machine commands with durable side effects should deduplicate by agent and task id.

```yaml
app:
  view: views/agents/session.tsx
  tokenizer: tokenizers/GLM-5.3.json
model:
  kind: sglang
  url: http://10.0.0.1:30000
  template: glm53
  template_options: {reasoning_effort: high}
  ranks: 8
  sampling: {temperature: 0.6}
machines:
  "vm-{n:03}": {kind: ssh, target: "agent@10.0.1.{n}", key: keys/agent_ed25519, timeout_s: 60, count: 512}
instances:
  - id: root
    app: fleet.tsx            # mounts one session per machine of the pool, as views/demo/tree/fleet.tsx does
    max_new: 4096
    props: {task: "...", c_hi: 48000, c_lo: 20000}
env: {SUB_MODEL_KEY: "${SUB_MODEL_KEY}"}
secrets: [SUB_MODEL_KEY]
workers: 8
model_threads: 1024
exec_threads: 1024
max_wall_s: 86400
```

The pool binds 512 roles, one VM each; the root app gives each mounted session its own.

`ranks` is the model server's data-parallel size: each agent is pinned to one rank, so its prefix
stays in one engine's cache (`sglang`: `routed_dp_rank`; `completions`, vLLM: the
`X-data-parallel-rank` header). Without it, a data-parallel server spreads an agent's turns over
its engines and most of each prompt is prefilled again.

The Rust API: `Runtime::new(config, Arc<Encoder>, Arc<dyn Model>)` (the `default` model), then
`.bind_model(name, enc, model)` and `.bind_machine(role, machine)`, then `.add_agent(spec)` or
`Runtime::restore`, then `.run() -> Report`.
`runtime::Agent` is the single-agent core with no I/O (`start`, `handle(input) -> commands`,
`restore(journal)`), for custom hosts and tests. Its one exception: with `set_blobs`, it writes
large strings as content-addressed blob files (idempotent, so replay is unchanged).

## Serving over HTTP

`affordance serve [PATH]` runs a deployment behind HTTP, as a Service Worker answers `fetch`
events. It takes `run`'s options and `--listen HOST:PORT` (or `serve: {listen, max_body_bytes,
timeout_s}` in the deployment), and keeps running.

- Each request reaches the first instance (the root, or the one agent) as the event `request`:
  `{id, method, path, query, headers, body}` (headers lowercase; the body as text). It is a
  journaled input, so a served app replays exactly.
- The app answers on the machine `client`: `{respond: id, status, headers, body}` (a body that is
  not a string is sent as JSON), or `{respond: id, status, headers, stream: true}`, then
  `{respond: id, chunk}`..., then `{respond: id, end: true}`. An answer to a request that is gone
  (answered, disconnected, or from before a restart) fails as a call error.
- A request not answered within `timeout_s` (default 600) gets a 504; a streamed response is cut
  off at the same limit. HTTP/1.1 is hyper's server: connections are kept alive, a body may be
  length-delimited or chunked (at most `max_body_bytes`, else 413), and a request's head must
  arrive within 30 s and fit in 64 KiB. No TLS and no login: the app can check the
  `authorization` header itself, a proxy in front can add TLS, and `serve` warns when it listens
  beyond loopback ([SECURITY.md](SECURITY.md#serving)).

- A model's reply can be **piped** into a streamed response as it is generated:
  `model(view, {pipe: {machine, call, text?, reasoning?, tool_call?}})`. Each piece of the reply
  is written to the machine as `{...call, chunk}`, where the chunk is the kind's frame with the
  piece substituted (escaped as the inside of a JSON string) for the placeholder; a kind with
  no frame is not written. The platform writes the chunks: no app code runs for a piece, nothing
  of a piece is journaled, and the call resolves with the same whole reply as without a pipe. On
  a replay nothing is written. A sink that fails (the client went away), or falls more than 1024
  pieces behind, closes the pipe; the model call goes on unless its `signal` stops it, and no
  model call ever waits on a sink. The call resolves after every piece written has reached the
  sink, so what the app writes next comes after them. A hosted binding's pieces are its stream's;
  a native binding's are its new token ids, decoded with its tokenizer.

`views/lib/http_server.tsx` gives an app `useRequests(async (req, res) => ...)` with `res.json`,
`res.text` and `res.stream` (whose `out.pipe(frames)` makes the `pipe` option, with `PIECE` as the
placeholder); `views/lib/openai_server.tsx` builds an OpenAI-compatible
`/v1/chat/completions` (streamed or not; a streamed request hands the app a `pipe` for its model
call, so the reply streams as it is generated) and `/v1/models` on it. `views/demo/memory_server` is a
chat model with a memory longer than its context behind that endpoint, and `views/demo/serve` the
smallest served app. An embedding program posts its own events into a running runtime with
`Control::post` (see "Controlling a run").

## Driving an app from a frontend

`affordance host [PATH]` runs a deployment for a frontend (a terminal UI, an editor) over stdio,
one JSON object a line, as a browser's DevTools protocol drives a page. It takes `run`'s options
and keeps running; stdout carries only the protocol (the summary goes to stderr).

- **In** (stdin): the control protocol (see "Controlling a run"): `{"type": "event", "name",
  "payload"}` posts an event to the first instance (or to `target`), and `close`, `status`,
  `set_model_limit` and `subscribe` work as there; `{"type": "answer", "request", "value"}`
  answers a question. The end of stdin closes the run (`--restore` continues it).
- **Out** (stdout): `ready {agent}` once the agents are running; `delta {agent, task, text, reasoning?}` (or `delta {agent, task, call: {index, id?, name?, arguments}}`,
  more of a hosted reply's tool call: its place in the reply, its id and name when they arrive, and more of
  its arguments' JSON text), a model's reply as it
  is generated; `output {agent, task, text}`, a command's output as it is written;
  `trace {agent, record}`, the trace records (renders, replies, each model call's `generate`
  statistics such as `first_token_ms` and `tokens_per_s`, finished tools, handler errors, the
  app's `console` lines);
  `ui {agent, value}` and `question {agent, request, ...}`; answers to requests that had an `id`
  (`result {id, value}`, `error {id, message}`), and `error {message}` for a failed one without;
  `exit {error?}`.
- **The machine `ui` is the frontend.** A call with `ask` (`{ask: {kind: "permission", ...}}`)
  becomes a `question`, and waits for the frontend's `answer`, whose value it returns; any other
  call is shown as `ui` and returns null at once. What to ask, and when, is the app's.

The deltas and output are what the observer sees (`RuntimeConfig.observer`); they are never
journaled. Events and answers are: an event is an input, an answer is the `ui` call's result.

`views/lib/frontend.tsx` is the chat loop: `useFrontend(log)` answers each `user` message, runs
the model's calls until it replies without one, and aborts the turn in progress on `abort`.
It tells the frontend what it is doing as `ui` values: `{state: "working" | "idle"}`,
`{calls: [{id, name, args}]}` as the model's tool calls start, `{results: [...]}` when they end,
and, in answer to the event `history`, `{history: [...]}`: the conversation so far, for a frontend
that connects to a restored run. These are the app's own messages, not the protocol's: another app
tells its frontend whatever it likes.
Through `turn.signal`, a tool's command stops with it. `ask(question)` asks the frontend.
`views/tools/files.tsx` has `read`, `write` and `edit` tools over a machine's shell.
`views/demo/host` is a coding chat that uses them and asks before each call:

```sh
cd views/demo/host && affordance host --set model.url=http://127.0.0.1:8080/v1
{"type":"event","name":"user","payload":{"text":"What is in this directory?"}}
```

An embedding program does the same with `Runtime::control()` (`post`, and `close` to end the run),
`Runtime::observe`, and a machine of its own.

**As another harness's model layer.** A harness that keeps its own loop, tools and sessions can use
Affordance for the model call alone. `views/lib/model_layer.tsx` (`useModelLayer()`) takes each call
as the event `generate` (`{id, system, messages, tools, max_new}`: what a chat API takes), renders
the conversation through the bound model's template, calls the model and answers on `ui` with
`{generated: {id, output: {text, reasoning, calls, cut_off, aborted, usage}}}`. The reply streams
as `delta` events, and the event `abort` (`{id}`) stops a call. The tools are declared for the model
and never run here: the caller runs them. Nothing of the app's own is added to the conversation.

## Controlling a run

A program outside the run controls it the way a browser's embedder controls a page: through one
handle, over whichever channel it has. It never writes files into the run directory.

| Operation | What it does |
|---|---|
| `event {target?, name, payload?, key?}` | Posts an event, as `postMessage` does. The answer comes once the target's journal line is committed, or says why the event was not delivered (no such instance, the run is closing). With a `key`, a second post of the same key is journaled but not taken again, so a controller can post again after a lost answer or a restore. |
| `close {timeout_ms?}` | Closes the run. The clock stops and no new model or machine call starts. Calls still running get `timeout_ms` to finish (default 0), then are cancelled. The journals are committed and the run can be restored with `--restore`. |
| `status` | The run's `state` (`starting`, `running`, `closing`, `closed`), its `clock_ms`, each agent's state, the model bindings' windows, and the model queue. |
| `set_model_limit {limit}` | How many model calls may run at once. 0 holds every model call while the rest of the run goes on. |
| `subscribe {streams: ["status"]}` | Pushes `{type: "status", value}` each time the status changes. |

The same operations are available four ways:

- **In Rust:** `Runtime::control()` returns a `Control` before the run starts. It has `post` (whose `Delivery` you can `wait` on), `close`, `set_model_limit` and `status`. `Runtime::run` returns a `Report` whose `ended` says how the run ended: `stopped`, `idle`, `closed`, `max_wall` or `worker_lost`.
- **Over stdio:** through `affordance host` (above).
- **Over a Unix socket:** for a run started on its own, with `affordance run|serve|host --control PATH`. The protocol is the same JSON lines. A request with an `id` is answered `{type: "result", id, value}` or `{type: "error", id, message}`. Several controllers may connect, and one that disconnects leaves the run as it is. The socket is the user's alone, and it is removed when the run ends, after telling each client `{type: "exit", ended}`.
- **With signals:** SIGINT (Ctrl-C) and SIGTERM close the run with 30 seconds for running calls. A second signal closes it at once.

`affordance run --keep-running` (or `keep_running: true`; `RuntimeConfig.serve` in Rust) keeps the
run up while instances are idle, a chat waiting for its user for example. It then ends when every
instance stops, at `max_wall`, or when it is closed. Without it (the default, for batches) the run
returns once nothing is in flight and no instance waits on anything. The status reports an
instance with nothing outstanding as `idle`. A send may name its event too:
`harness.send(to, payload, {event: "ping"})`.

Every end goes through closing:
- every instance stopped,
- a batch with nothing left,
- `max_wall` (no time for running calls),
- a lost render worker,
- `close`.

While the run closes:
- **New calls do not start.** Model and machine calls that instances ask for stay outstanding in their journals, and a restored run issues them.
- **Calls that the host cancels are not failures.** Calls still running when the timeout passes are cancelled by the host. Nothing is journaled for them, and a restored run issues them again. A call the app cancels, or one whose instance is unmounted, is recorded as before.
- **The clock stands still.** It stays at the moment closing began, so a timer neither fires nor loses time while running calls finish, and a restore continues each timer with what it had left. The clock also stands still while a run's agents restore, so a slow restore takes no time from them.

Errors never name a model server's address: transport failures say the binding and endpoint
(`sglang /generate: the server closed the connection without a response`), and a hosted
binding's error has its host replaced by "the model server". The address is in the binding's
configuration, not in journals, traces, captures or what an app sees.

An instance's failure is its own, as a crashed tab's is. An error or a panic in its own code
(loading its app, starting, replaying its journal, handling an input) fails that instance alone:
it is reported with the error, never run or imaged again, and its worker serves the others. A
restore replays the same journal and fails it the same way. A render worker that dies outside
its instances' code ends the run at once: its instances are reported failed, the others can be
restored.

Closing does not wait behind queued deliveries that will be issued again on restore.
- **A queued `harness.send` is not applied.** The receiver does not journal it and the sender gets no answer (not `delivered: false`), so the send stays outstanding in the sender's journal and the restored run delivers it.
- **A queued post is answered that the run is closing,** so the controller posts it again after the restore.
- **Everything else still queued is applied as before:** model and machine completions, timers, mounts, props, callbacks.
- **A multicast send that reached some targets before closing stays pending.** It is resent to all of them on restore. A target that already journaled it journals the copy but does not take it twice (its delivery id), the same at-least-once delivery as after a crash. This is a page being unloaded: its queued messages are
not delivered, and the sender's request stays pending and is retried.

## Cancellation and streaming

An app stops its own calls with `AbortController`, as a web page stops a `fetch`:

```tsx
const [turn, setTurn] = useRef(null, "turn");
useEvent("user", async ({text}) => {
  const controller = new AbortController();
  setTurn(controller);
  const output = await model({signal: controller.signal});
  // Escape: output.aborted, and output.text is what was generated before it.
});
useEvent("abort", () => turn()?.abort());
```

- A model call stops generating and resolves with what it has (`aborted: true`, `cut_off:
  true`); the partial reply is journaled, so the log keeps it and a replay matches. A reply that
  finished first is kept whole.
- A machine call or `fetch` is cancelled as below and rejects with `AbortError`.
- A call given a signal already aborted is not issued.

The rules are in `spec/HARNESS.md` §3. Under them, every model and machine call carries a host
cancel token (`runtime::CancelToken`):
- **When work is cancelled.**
  - An agent stops: it cancels everything it asked for.
  - A handler fails: the calls it started and did not await are cancelled. (A handler that
    returns normally keeps its un-awaited work going until it settles.)
  - The runtime shuts down (end of run, or `max_wall`).
  - A machine call passes `call_timeout`.
- **How it stops.**
  - A local `exec` sends SIGTERM to the command's whole process group (led by `timeout`), then
    SIGKILL after 5 s.
  - Cancellation drops an active HTTP request; remote effects already performed are not rolled back.
  - Over SSH, cancelling kills the local `ssh` client; the remote command runs on until its
    remote `timeout` fires.
- **Late results** of cancelled work are ignored (deterministically: the journal replays the same).

Model replies stream by default (SGLang `/generate` with `stream`, or `/v1/completions` with
`stream` + `return_token_ids`; set `stream: false` in the model binding to turn it off):
- **Abort.** A cancelled request closes its stream, which stops generation on the server. A
  blocking request would keep generating to the end.
- **Stalls.** If no token arrives for `stall` (default 5400 s, `"stall_s"`), the request is aborted
  and retried.
- **Cut streams.** A stream that ends without a finish reason (server or network died) is an
  error and is retried, never mistaken for a complete reply. So is a request the server finished
  with `abort` (stream or not): what it generated is not a reply.
- **Live view.** `RuntimeConfig.observer` sees what each call produces as it arrives
  (`runtime::Observed`): a native model's new ids, a hosted model's new text and reasoning, and a
  machine call's output as the command writes it. It is never journaled, so watching changes
  nothing. Hosted calls stream (server-sent events) when something watches or can stop them (an
  observer, or a call with an `AbortSignal`); the streamed events are rebuilt into the response the
  provider sends unstreamed, so the reply ids are the same either way.
- **Trace.** Each model call gets a `generate` entry: prompt and generated tokens, ms, time to
  first token, tokens/s, cached tokens, rank.

## Flags and the cache warning

Flags are this machine's settings for the `affordance` command, kept apart from any deployment as a
browser keeps its `about:flags` choices in the user's profile. They change what the command
reports, never what a run does. They are read from `~/.config/affordance/flags` (or
`$XDG_CONFIG_HOME/affordance/flags`), one `name = value` per line with `#` comments, and
`--flag name=value` overrides one for a run of `run` or `serve`. An unknown flag is an error.

| Flag | Default | Meaning |
|---|---|---|
| `cache-warn` | `auto` | Warn when a prompt throws away most of the server's prefix cache: `on`, `off`, or `auto` (on when stderr is a terminal). |
| `cache-warn-below` | `0.2` | Warn when the new prompt shares less than this share of the previous prompt and its reply. |
| `cache-warn-min-tokens` | `1024` | Ignore previous prompts shorter than this. |

A model server keeps its work on the start of the previous prompt and reuses it when the next one
starts the same way. When an agent's prompt changes near its start on a long prompt, the server
computes it again, and the run prints, as a browser console reports a forced reflow:

```
affordance: warning: e3 turn 7: the prompt changed at token 226 of 9140, so the server recomputes 8914 cached tokens. Is something near the start changing every turn? A history can drop in steps (drop-step).
```

The usual causes are a value near the top of the prompt that changes on every turn (a clock, a
counter) and a history that drops just enough to fit on each turn (`drop-step` fixes that one).
Sometimes an app means to start over, and the warning is only information. Each agent's
warnings are printed on its 1st, 2nd, 4th, 8th... occurrence, and after a dozen lines a run
only sums them up. When the server reports reusing much less than the prompt allowed, a softer
`note` says so: the cause is usually the server (an evicted cache, another replica), not the
app. The numbers come from the trace's `render` and `generate` records (spec/HARNESS.md §10).

## Limits

- **Not built yet:**
  - a background-process table (`proc.*`);
  - vsock `exec`;
  - dispatching a tool call before its reply has finished streaming;
  - per-agent send rate limits (policy belongs in the app; a hard cap in the runtime is not
    built).
- **One default root app per deployment.** Child mounts can select imported components from
  other files, with their own state, journals and bindings.
- **The log's turn is serial; model calls are not.** Any number of `model()` calls may be open at
  once. A log (`useLog`) holds one turn open at a time: records appended meanwhile wait in
  `log.held` and are written after the turn's results (so a prompt plus its reply stays a prefix of
  the next prompt).
- **Hook writes: the last writer wins**, in journal order. A setter given a function
  (`setNotes(ns => [...ns, x])`) applies it to the current value; a setter given a value computed
  from a render's copy (`setNotes([...notes, x])`) can overwrite a write made since that render.
- **Exact model turns need a supporting template app.** Every stock template app except
  MessagesApp (text for hosted APIs) supports them. Tokenizer availability is a development-reference concern, not a different runtime contract.

## Choosing context limits

The session app's `c_hi` and `keep_recent` choose when and how it folds history; `max_new` bounds generation. Choose them against the bound model's context
limit and the app's measured workloads. Rendering conformance does not establish task quality
or an optimal folding policy. Training and comparative task studies belong to the separate
research project.
