# The harness contract

Affordance has two parts. **The language** (`spec/LANGUAGE.md`) turns state into exact token ids
for a model, plus the actions it offers. **The harness** (this document) runs apps the way a
browser runs pages: it decides what is mounted where, which model and machines each app may
use, how its events and effects run, how effects reach the world, and how all of it survives a
crash. The runtime never calls a model on its own: an app calls it (§3).
This document is normative for `src/runtime/`. "Is" and "must" describe what the code does.
Sections marked *not built* describe what it does not do yet.

For ownership, document/computation boundaries and failure guarantees, see
[CONTRACTS.md](CONTRACTS.md).

Contract version: **9** (`CONTRACT_VERSION`, pinned in every journal). Version 9 carries an
event's delivery id on the journaled event (`id`), not in its payload (`_msg`), and every model
failure has its kind; version 8 pinned a model binding by which model answers (kind, model,
provider options, template, context), not how it is reached (docs/MODEL_BINDINGS.md).
A journal of another version requires that version's binary to restore.

## 1. Objects

| Object | What it is |
|---|---|
| **Serving** | `affordance serve` delivers each HTTP request to the first instance as the event `request` (`{id, method, path, query, headers, body}`); the app answers with calls on the machine `client` (`{respond: id, status, headers, body}`, or streamed: `stream: true`, then `chunk`s, then `end`). An embedder posts events into a running runtime with `Control::post`; each is journaled by the instance that takes it. See docs/RUNTIME.md. |
| **Frontends** | `affordance host` runs a deployment for a frontend over stdio (JSON lines): the control protocol's events go to the first instance; a model's reply and a command's output stream out as they are produced (the observer's view, never journaled); the machine `ui` is the frontend (a call with `ask` waits for its answer). See docs/RUNTIME.md. |
| **Control** | A run's embedder holds a `Control` (`Runtime::control`): `post` an event (answered once it is committed; a `key` makes a repeated post a duplicate), `close` with a timeout for running calls, `status`, `set_model_limit`. The same operations reach a run over stdio (`affordance host`) or a Unix socket (`--control PATH`), and SIGINT/SIGTERM close it. A run is `starting`, `running`, `closing`, then `closed`; `Report.ended` says why (`stopped`, `idle`, `closed`, `max_wall`, `worker_lost`). See docs/RUNTIME.md, "Controlling a run". |
| **Deployment** | The bindings a run starts with: model servers by name, machines by role, the run directory, limits. An `affordance.yaml` for `affordance run` (`docs/RUNTIME.md`), or a `RuntimeConfig` in Rust. |
| **App** | A `.tsx` file and its imports. Its entry is the requested function, else the file's default export. Its identity is a hash of the entry name and every loaded file's relative path and contents (`View::source_hash`). A served app imports only from its own directory (and below) and from the library (the deployment's `library`, by default the nearest directory above the root app holding `index.json`, the installed `views/`), as a page loads scripts only from its allowed origins (CSP `script-src`). Paths are resolved through symlinks before the check; an import outside is a load error. |
| **Instance** | An app mounted with props and bindings. It owns its state, journal and children. Its id is its path (`root/alice`). Every running thing is an instance: an agent, a root that mounts a fleet, a child of another app. |
| **Props** | The instance's inputs from its parent, in its state as top-level fields. They change only through `Props` inputs from the parent. |
| **State** | Props plus hooks (`useState`). A hook is named (`useState(init, "fold")`, stored as `__ui["@fold"]`) or positional (keyed by component path and call order). A hook's initial value is stored the first time a render uses it (state is fixed at mount, as in React), so `useState(prop)` does not follow later props. The instance is the only writer of its state. |
| **Log** | The harness keeps no conversation. An app that keeps a history of its model turns keeps it in its state: `views/lib/log.tsx` (`useLog`) writes `screen`, `model` and `action` records from `model()` and `dispatch()`, with exact ids, plus the `input` records the app appends, and releases what its prompt no longer shows to the store. |
| **Journal** | The ordered inputs of the instance, written before each takes effect and committed in groups (§7). Replaying it rebuilds the instance exactly. |
| **Binding** | Authority to use something outside the instance: a **model** (by name) or a **machine** (by role). An instance can reach only what it is bound to. |

## 2. The instance tree

An instance's render produces three things:

1. **Tokens**, for its model (when it has one).
2. **Affordances** (`<aff>`): tools the model calls, and callbacks the harness calls.
3. **Mounts**: its children. A mount produces no tokens:

   ```tsx
   import { Greeter } from "./greeter.tsx";
   <Mount key={name} model="default" machine={harness.machines[i]}>
     <Greeter name={name} heard={greetings} onGreet={g => setGreetings([...greetings, g])}/>
   </Mount>
   ```

   `<Mount>` takes exactly one component element and records it instead of rendering it: the file
   that defines the component and its name are the child's app and entry; its props are the child's
   props. `key`, `model` and `machine(s)` are the parent's decisions about the child's bindings.

After every input, the instance reconciles its children with its latest render. A new key
mounts a child. A missing key unmounts it. Changed props are a `Props` input to the child. A
changed app, model or machines remounts it. Reconciliation is a deterministic function of state,
so replay mounts the same children. A remount starts a fresh incarnation, including hook and
store state. Recovery after a process restart restores the current incarnation instead. Old
incarnations are preserved under the run directory's `retired/`. Child keys cannot
start with a dot or contain `/` or control characters. Inspection cursors are scoped to the
incarnation, so reusing a child key produces a fresh state read.

**The boundary.** The child runs as its own instance, so the element crosses a boundary, as a
React Server Component's props cross from server to client (and as a portal's child lives
elsewhere). Unlike an in-process component:
- **Props are JSON.** A function, markup, a token or a regex as a prop is a render error naming
  it, as is a value that contains itself or nests past 128 levels (as state). A function prop
  named `onX` is a callback (below). The child takes no children.
- **Callbacks are events.** Calling one in the child returns nothing; the parent's handler runs
  later, in the parent's own loop.
- **The child renders on its own schedule** (its model turns), not the parent's. Only changed props
  pass from the parent's renders.
- **Nothing is shared** but props and callbacks: no context, no state.

**One owner per state.** Props flow down. A callback prop (`onGreet={...}`) arrives in the child
as a stub. Calling it is an event in the parent (`mount:<key>:onGreet`), which runs the parent's
handler with the call's arguments. It is delivered even if the calling handler finishes or the
child stops. When a child stops, for any reason, the harness calls its parent's `onStop` with the
stop value.

Callbacks are queued notifications. Their recovery boundary is the parent's journal: a crash
or forced shutdown can lose a notification not yet journaled there. Use an acknowledged
`harness.send` when the sender needs confirmation of durable delivery.

**Bindings only narrow.** A child's machines must be a subset of its parent's. Both the parent's
reconciliation and the host check this. A child's model is the one `<Mount model>` names, else
its parent's. Top-level instances are bound to the deployment's machines unless their spec lists
roles. With a model binding, an instance calls its model when its code says so
(`model()`, §3); without one it only reacts.

Children live on their parent's worker. Each journals in its own instance directory (below),
like any instance. On restore, a
parent's replay rebuilds its children set, and each child restores from its own journal and first
props.

## 3. The loop

**Prepared documents.** Event delivery reuses the last prepared document while state, app,
tokenizer and the render's observed context fields remain valid. Direct `harness` field/index
reads observe that field; escaping the context object conservatively observes all fields.
Render-time date helpers observe the clock. Reused event snapshots receive current globals,
preserving unchanged field identities; model-turn and running-task snapshots stay immutable.
`AFFORDANCE_RENDER_VERIFY=1` checks reuse against fresh token output, mounts and effect declarations.
Runtime reports include diagnostic `render_stats.evaluations`/`reused`; these are process-local
work counters, separate from persisted replay metrics and excluding extra verification renders.

An instance is a page: it renders
a view of its state, reacts to events, and calls its model when its code decides to, as a page
calls `fetch`. The runtime never calls the model on its own, and keeps no conversation.

Each instance applies one input at a time: journal it, apply it, reconcile children, report a
stop. Applying an input (a reply, a result, an event, a tick, props):
1. settles what it answers and resumes the tasks waiting on it;
2. runs the listeners of an event (`<aff event="name" onCallback>`, `mount:` callbacks) at once,
   even while other tasks wait on effects;
3. renders, and runs the effects whose deps changed (`useEffect`: cleanups, then setups), again
   while setups change state, at most 50 passes (else it stops with `max_update_depth`).

A task (a listener, an effect setup or cleanup, a tool call) runs its synchronous prefix.

**Opt-in input frames.** Deployment `input_batch` (1..64, default 1) coalesces already-queued
events/results per instance in a bounded worker window. A group is one `{"input":"batch",
"inputs":[...]}` journal record, at most 1 MiB after blob lowering, written before execution or
acknowledgement and committed like any record (§7). Oversized groups use individual records. Replies, ticks, props, lifecycle
operations and inspection are scheduling barriers; nested batches are
invalid. No timer delays incoming work to fill a batch.

Each member takes its own turn, as if it had arrived alone: its listeners run from a render of
the current state, its microtasks drain, and its effects and reconciliation run, with a fresh
execution allowance. A frame is a journal record, not an app-visible unit: batching never changes
what an app computes (two queued `setN(n + 1)` give `n + 2`, as without it). A member that stops
the instance ends the frame. Recorded batch boundaries govern replay. Agent `journal_len` counts logical members; JSONL line count
counts physical frames. Checkpoint offsets and hashes remain byte-based. Existing individual
records retain their semantics; older binaries cannot read the new batch shape.
Every await yields to the instance's FIFO microtask queue. The current input's listeners run
before that checkpoint. Await continuations, reactions and promise adoption share the queue;
unawaited work remains owned by the task until it settles or the task is cancelled.
A setter applies once to live state in execution order: a later write wins. An updater runs
synchronously against the current frozen value. Suspended functions retain their locals and
references; earlier writes and updater computations do not run again on resumption.
Durable recovery uses a verified eligible checkpoint plus newer inputs, or re-executes the
complete journal when no usable checkpoint exists (§8). Clock and randomness
come from the deterministic instance, while host operations return promises.
A journal replays only on a binary that implements the same contracts (`spec/CONTRACTS.md`).

**Model calls.** `model(view?, opts?)` sends a view to the instance's model and resolves with an
output: no view means the page's own, laid out once, at the call, from live state (its handlers
are kept for `dispatch`); an element is an ad-hoc prompt (a summary, a sub-question). The output:
`{text, reasoning, calls: [{name, args, id, problem?}], problems, cut_off, ids, screen: {ids,
images?, content?} | null, provider_state?, usage: {prompt_tokens, cached_tokens, exact, generated_tokens}, server}`, with `ids` (and
the screen's) as token runs. `server` is what the call measured, as a fetch's Resource Timing is:
`{cached_tokens, ms, first_token_ms, wall_ms}`, the prefix the server reported reusing (null when it
reports none), the call's duration and time to its first token, and the reply's arrival in
milliseconds since the Unix epoch. It is journaled with the reply, so a replay reads the same, and
an app can render its own latency or cache reuse from it; it is null for a reply journaled before it
existed. `cached_tokens` counts the prompt's prefix shared with the previous page prompt and its reply; an ad-hoc call is off that chain (`cached_tokens: 0`, `exact: false`). Hosted syntax also returns the screen content as sent, including image parts. Optional `provider_state` is opaque JSON text tagged by protocol/model, retained by useLog on model records and passed through logTurns. Deliberate edits must clear it. Hosted log records retain fields instead of a second copy of JSON reply IDs. `opts`: `max_new`, `sampling` (merged over the binding's), `tag`,
`priority`, and `signal` (below). A prompt that does not fit the binding's `context` with its reply is not sent:
the call fails with `ContextError: ...` naming the largest boxes. A page that fails to render (a
component or template that throws, an image without a data source) rejects the call with that
error, as `fetch` rejects: the app catches it and the instance goes on. A page over `max_prompt` stops
the instance (`prompt_over_limit`); the `max_turns`-th page call stops it
(`max_turns`). Any number of calls may be open at once.

**Aborting a call.** `AbortController` and `AbortSignal` are the browser's: `model(view, {signal})`,
`harness.machine.call(args, {signal})` and `fetch(url, {signal})` take one, and
`controller.abort(reason?)` stops every call given its signal. A model call stops generating and
resolves with what it generated, as a streaming `fetch` reader keeps the chunks it read: the
output has `aborted: true` and `cut_off: true` (its text is kept; a call it began is not made).
The partial reply is journaled with `aborted: true`, so a replay reads the same output. A reply
that was already complete when the abort reached the model is journaled whole and resolves
normally (`aborted` absent): the journal decides the race. A machine call (and so `fetch`) is
cancelled and rejects with `AbortError`. A call given a signal already aborted is never issued; it
rejects with `AbortError` at once. A model call aborted and not yet answered when the host stops is
stopped again on restore (it answers at once with what it has) rather than issued whole. Signals are
the instance's: whether one is aborted, and why (`signal.aborted`, `signal.reason`,
`signal.throwIfAborted()`), is kept with the instance (journal, images), so a controller kept in
state, which is frozen, still aborts. They are read and aborted in handlers. `AbortSignal.abort()`
makes one already aborted; `addEventListener("abort")`, `AbortSignal.timeout` and `AbortSignal.any`
are not supported.

`sampling` is an object of provider sampling parameters (for example `temperature`, `top_p`
and `seed`). Prompt/input IDs, messages, tools, model identity, native-token/stream protocol,
stop controls, routing/session identity and output limits are host-owned. Attempts to supply
those fields through sampling fail before network I/O; use explicit call options or bindings.
Native model responses must contain unsigned 32-bit integer token IDs; malformed IDs are
protocol errors, rather than being silently dropped or truncated. Adapters do not retry these
errors; the instance-level retry policy below still applies.

`dispatch(output, {calls?})` runs a page output's calls on the handlers of the view the model saw:
all of them, or those at the indices in `calls`, and resolves with their
`[{name, id, result, error?, ...meta}]` in the order asked. Each call's return value is its result,
and the calls one dispatch starts run concurrently. A call runs once: dispatched again (alone or
with others), it is waited for if running and its result given again once answered. So an app
runs a reply's calls in order, all at once, or stopping at the first error, with plain promises
(`views/lib/log.tsx` runs them in order by default). A call with a `problem` (an unknown tool, bad
arguments) is answered with it and not run. Dispatching the same output again returns the same
results; an output's view is kept until it is dispatched, or until 8 newer page calls have not been
dispatched either; the output itself (for a repeated dispatch's results) is kept for 64 page calls.
An ad-hoc output cannot be dispatched. Whether and when to call the model, and what to remember,
is the app's (`views/lib/log.tsx`, `views/lib/loop.tsx`: `useLog`, `useLoop`, `useConversation`).
The supplied `useLog` allows one open turn at a time: `call` rejects until the previous turn is
dispatched or closed. `close(out, why?)` requires the resolved output of that turn; closing
while `call` or `dispatch` is pending raises `InvalidStateError`. An older output's repeated
close leaves a newer turn unchanged; dispatch through the log requires the current output.
Separate logs and independent `model()` calls can run concurrently.

The TSX library exports `LogRecord` and its input/screen/model/action variants, plus
`recordText(record)` for diagnostic/search text (reasoning and calls included; image parts
become `[image]`). It is not the prompt renderer. Each committed record has a library-owned
`id`: `e0`, `e1`, … for the default log, or `<name>:e0`, … for a named log. Caller-supplied
IDs are overwritten. Held inputs receive IDs only when committed after the turn's results.
`release` always keeps record IDs, while removing exact token `ids` from the stored payload.
`read(from,to)` reads a nonnegative ordered half-open range, reconstructing released records;
`get(id)` returns that log's full record or null. Read released records before `recordText`.

## 4. The harness API (in a handler)

| Call | Result | Notes |
|---|---|---|
| `model(opts?)` | an output (see §3) | The page's own view to the instance's model. `{signal}` aborts it (§3). |
| `model(<App .../>, opts?)` | an output | An ad-hoc prompt: lays out a component to exact ids and runs it once on the instance's model, through the same admission gate, rank pinning, captures and cancellation. The component's `protocol` span reads the reply. A prompt over `max_prompt` is not sent; the call fails with an error the task can catch. |
| `dispatch(output, {calls?})` | `[{name, id, result, error?, ...meta}]` | Runs a page output's calls, or those at the indices given (see §3). |
| `fetch(input, init?)` | `Promise<Response>` | Global Fetch over the bound `http` role. Supported options: `method`, `headers`, text `body`, `redirect`, `signal`. Headers, body readers and limitations: `docs/HTTP_AND_MCP.md`. |
| `harness.machine.call(args, {signal}?)` | executor-specific JSON | The instance's first machine. An aborted `signal` cancels the call, which rejects with `AbortError`. Shell executors take `{cmd, timeout_s?, env?, stdin?}` and return `{output, exit, seconds, timed_out, truncated}`. Large strings cross as blob references (§7). |
| `harness.machine.get(role).call(args)` | executor-specific JSON | A named one of several. `get` itself only names the machine; `call` issues the effect. |
| `harness.send(to, payload, {event?})` | `{to, delivered}` | Event `message` (or `event`) to instance `to`, with `from` set by the harness (an operator's message has no `from`, so an app trusts control fields only without one). Completes when the receiver has journaled it. `mount:*` events are reserved (a `<Mount>`'s callbacks). |
| `harness.send(ids, payload, {event?})` | `{delivered, failed, undelivered}` | Array of target IDs, deduplicated in input order. One result after all recipient acknowledgements; counts plus unconfirmed IDs in target order. Empty arrays succeed with zero counts. Self/missing/failed targets fail as for a single send. |
| `harness.wait({ms?, events?})` | `{woke: "timeout"}` or `{woke: "event", event, payload}` | Parks the task. Without `ms`, waits only for an event. It wakes at once on a matching event journaled after the render its task runs from (one the model had not seen). |
| `harness.inspect(targets, {select?, since?, as?})` | `{since, instances: {[id]: {version, changed, changes?, status}}}`, or with `as: "text"` `{since, text}` | A read-only copy of other instances' committed state (`targets`: an id, a list, or `"*"`), made on their workers between inputs; nothing is written to their journals. `select`: named hooks and `"props"` (whole, `{key, set}`), or `{key, path?, tail?}` (a followed list: `{key, path, from, items, length}`). `since`: an earlier read's opaque cursor; only changes to selections covered by that cursor are omitted. New/reselected paths, whole/follow modes and tail windows receive their initial copy even at the same journal position. The next cursor covers this call's selections. Values are JSON tagged where JSON has no word (`{"$": "map", entries}`). Needs the spec's `inspect` grant (§5). |
| `harness.store.put / get / search / list(args)` | `{key}`, `{text, offset, total}`, `{hits}`, `{keys}` | Per-instance storage. |
| `harness.now()`, `harness.random()` | number | Logged values: replay sees the same. |
| `harness.stop(value)` | nothing | Stops the instance. |
| `Promise.all / race / any / allSettled` | as in JavaScript | Reactions run through the instance FIFO queue; `race` does not cancel other inputs. |

**Data in a render:**
- `harness.instance` (the instance id), `turn` (page calls so far), `clock_ms`, `env`, `prompt_tokens` (the last page prompt's length);
- `machines` (the bound roles);
- `owner`.

Effects are allowed only in handlers. Live execution resumes saved frames through the instance
queue; standalone transcript reconstruction compares every request exactly. A tool answers when its body returns (its
return value is its result, like a Service Worker's `respondWith`); work it started and
did not await goes on and settles later (like `waitUntil`).

Inspection selections accept a dotted string path or an array of literal key segments. Response
paths are always arrays: `["a.b"]` names one key, while `["a", "b"]` names two nested keys.

Inspection list followers (`{key, path?, tail?}`) track appended items by length. They are for
append-only lists; same-length replacement is not a list delta. Select the whole hook with a
string key, or reset the cursor, when replacements must be observed. Serialization failures
return a per-target `error` without advancing that target's cursor.

Tool argument validation follows JSON Schema's object rules: `additionalProperties: false`
rejects extra fields even with empty `properties`; omitting it permits extra fields. A schema
in `additionalProperties` validates those extra values. Unsupported `schemaOf` types, including
function-valued properties, produce errors rather than silently losing fields.

## 5. Machines

A machine binding names an external executor: a shell, HTTP endpoint, or custom adapter.
Machine calls are at-least-once: after a crash, outstanding calls
are resubmitted, so commands with durable side effects should be idempotent per (instance, task).
There is no other way out of the harness. Running on the harness's own host means binding a
`local` machine. Network access is through a machine.

| Kind | Config | Runs |
|---|---|---|
| `local` | `home?`, `per_instance?` | `bash` on this host with only the instance's environ, in `home` when given (with `per_instance`, `home/<instance id>`). |
| `ssh` | `target`, `key?`, `control_dir?`, `known_hosts?` | `bash` on a VM over a shared connection. |
| `docker` | `image`, `name?` | `bash` in a container per instance (`<name>-<instance id>`), started on first use. |
| `bwrap` | `home`, `per_instance?`, `network?`, `read?` | `bash` in a bubblewrap sandbox: host read-only, `home` writable, no network by default. *Tested only where bubblewrap is installed.* |
| `command` | `argv`, `max_reply_bytes?` | A machine that brings its own executor: each call runs `argv` in its own process group, with `{agent, task, env, args}` as JSON on stdin, and takes a JSON result from stdout (at most `max_reply_bytes`, default 128 MiB: past it the call fails and the group is stopped). A cancelled call's group gets SIGTERM, then SIGKILL after 5 s. |
| `http` | `url`, `headers?`, `timeout_ms?`, `max_request_bytes?`, `max_response_bytes?` | A generic HTTP executor on a bound origin. Calls take `{path?, method?, headers?, body?, timeout_ms?}` and return `{status, statusText, url, headers, body}`. Global Fetch uses the role `http`; named fetch functions use their assigned role. Protected headers are inserted by the host. See `docs/HTTP_AND_MCP.md`. |

An instance's place on a machine (a `per_instance` home, its container) is named by the instance's
id in the run (`instances/<id>/`), never by its name; outside a run (an executor called directly)
the name stands in, escaped into one component (`i-` and the UTF-8 bytes, ASCII letters, digits,
`-` and `.` kept and every other byte `_hh`: `root/child` is `i-root_2fchild`). Docker names are
validated and names and images are passed as arguments. An ssh target and key are what the
deployment says. `count` makes a pool of roles (`vm-{n:04}`, `{n}` the member's number) that a
root hands out.

A call's `stdin` reaches the command byte for byte as its standard input; a command that does not
read it never runs it as script. A `command` machine gets it as `args.stdin`.

Output past `spill_bytes` is written straight to a blob (§7). Capture is bounded (16 MiB).

**Capabilities.** An agent's `capabilities` (e.g. `["store.*"]`; a trailing `*` matches a
prefix) limit the platform APIs it may call (`send`, `store.*`). A call outside them returns the
error `this agent may not call <api>`. Absent means all. Children inherit their parent's.
`harness.inspect` is not among them: an agent's spec `inspect` lists the id prefixes it may read
(`["*"]`, `["agent-"]`); it is empty by default and children do not inherit it. A spec with
`inspectable: false` (inherited) is never read.
Machines are bound by `machines`, never by capabilities.

## 6. Models

A model binding is a name, a tokenizer and a client. Clients:
- `sglang`: `/generate` with `input_ids`;
- `completions`: `/v1/completions` with token-id prompts and `return_token_ids` (vLLM);
- `chat`: a hosted `/chat/completions` (TrustedRouter, OpenRouter);
- `anthropic`: a hosted Anthropic `/v1/messages`, with prompt-cache breakpoints;
- `responses`: a hosted OpenAI `/responses`, with native reasoning and function tools.

In a render, frozen `harness.model` is `{name, template, template_options}`, or null for
an instance without a model. An unconfigured native template is null. These are presentation
facts; endpoints, credentials and provider settings are not exposed through this object.
`views/lib/model.tsx` exports `ModelContext`: it selects the binding's registered template,
passes its options, and composes `TurnStrategy` through `log`, `strategy` and `callable` props.
The app supplies `system` and children. Missing templates fail explicitly. Hosted histories
render from fields instead of JSON reply IDs. A passive (`callable:false`) render keeps tool
definitions but disables provider calls in all hosted adapters, after merging request settings.
Provider call responses violate this restriction and fail explicitly, including partial calls. Explicit template imports remain available.
Template selection/options are part of configured binding identity; imported template sources
are part of the app source pin. Direct `Instance` users supply resolved `InstanceConfig.template`.

The stock CLI and `runtime::model::from_config` use one strict configuration contract.
Unknown keys, wrong types, adapter-inapplicable settings and protected-field conflicts fail
before inference. Each binding config is an object with these fields:

| Fields | Adapters | Contract/default |
|---|---|---|
| `kind`, `url` | All | Required kind; base URL required except hosted Anthropic/Responses defaults below. All adapters take HTTP(S); token-ID adapters check TLS certificates against the Mozilla roots. No URL credentials/query/fragment. |
| `tokenizer` | All, deployment metadata | CLI default falls back to `app.tokenizer`; each entry in `models` requires its own path. Rust callers supply the encoder. |
| `sampling` | All | Object, default `{}`. Per-call sampling shallowly overrides defaults. |
| `context` | All | The model's context length in tokens. Less the call's `max_new`, it is every prompt's window (`max-tokens: 80win`); a prompt that does not fit it with its reply is not sent (`ContextError`). A native binding asks its server when it is validated (`GET /v1/models`: the entry named `name`, or for SGLang the only entry, `max_model_len` or `context_length`; else SGLang `GET /get_server_info` `context_length`). A positive integer here may only lower that (more than the server reports fails validation); a server that cannot be asked leaves it as configured, with a warning. Hosted APIs report none: a hosted binding needs it. `"none"` runs without a window and asks nothing. A binding with no window otherwise fails validation. Part of the binding's identity (a window the server reports is the same identity as that window configured). |
| `template`, `template_options` | All | Optional registry key (`views/template_apps/index.json`) and props object, default `{}`. Chat, Anthropic and Responses default to and require `messages`. Native clients do not guess a template. Options require a template and cannot replace app-owned `system` or `children`. |
| `provider_options` | All | Object, default `{}`. Fixed per binding; copied to the top-level provider body. |
| `key_file`, `key_name` | All | Both or neither for native adapters; both required for hosted adapters. A binding-local credential read at call time. |
| `key_header` | Chat, Anthropic, Responses | The header the key is sent in: `authorization` (bearer), `x-api-key`, `api-key` (Azure OpenAI) or `x-goog-api-key`. Default: the API's own. Redacted like a bearer key; `headers` cannot name these. |
| `request_timeout_s` | All | Finite positive seconds for the whole request; native non-streaming 5400, chat 600. SSE uses `stall_s`. |
| `retry` | All | What happens after a call fails: the retry policy (see "Model call failures and retries"). |
| `ranks` | SGLang, completions | Positive integer, default 1. |
| `stream`, `stall_s` | SGLang, completions | Boolean (default true); finite positive stall seconds (5400). |
| `sessions` | SGLang | False/omitted, true (4 turns/600 s), or `{turns?, timeout_s?}` with positive values and no other keys. |
| `name` | Completions | Nonempty provider model selector, default `"default"`. |
| `model` | Chat | Required nonempty provider model selector. |
| `headers` | Chat | Optional object of non-secret string values. HTTP token names, valid header values, case-insensitive uniqueness; Authorization and Content-Type are client-owned. |
| `stream` | Chat | Only false is supported; omitted means false. |
| `model`, `headers`, `stream` | Anthropic | As for chat; `url` defaults to `https://api.anthropic.com`; `x-api-key` and `anthropic-version` are also client-owned headers. |
| `model`, `headers`, `stream` | Responses | As for chat; `url` defaults to `https://api.openai.com/v1`; bearer auth. |
| `stall_s` | Chat, Anthropic, Responses | The longest a streamed reply may go without data, in seconds; the call then fails as `interrupted`. Unset: only `request_timeout_s` bounds it. |
| `strict_tools` | Responses | Boolean tool-schema strictness (false). |
| `cache_ttl` | Anthropic | `"5m"` (default) or `"1h"`. |

Provider options cannot replace prompts/IDs/messages/tools, images, model selectors,
request/session/routing IDs, authentication/destinations, stops, limits or stream controls.
Sampling cannot set those fields or model/LoRA selectors. A key cannot occur in both provider
options and binding/per-call sampling. Native SGLang `provider_options.lora_path` is a nonempty
string or null for its single prompt. Other provider extensions are forwarded unchanged;
the server validates their names and semantics. Local validation does not claim a universal
schema for arbitrary provider versions. Credentials are not provider options and are redacted
from request captures, never sent to the app or written into journal pins.

Each SGLang session pins its client identity. Changing binding identity requires fresh sessions;
per-call sampling may vary. Built-in clients declare identity through `Model::binding_identity`:
the existing persisted instance pins include its digest and binding name. Strict recovery
rejects changed or missing model identity before inference; warning recovery discloses the
change and preserves original pins. Key rotation at the same file/name does not change identity.
Custom adapters must declare their own identity to get this check. Already-recorded replies
replay without another request; outstanding work retains the existing at-least-once contract.
See [model binding examples and transport details](../docs/MODEL_BINDINGS.md).

A `chat` binding renders through `MessagesApp`, whose JSON turns the client sends. It writes the
reply back as `{reasoning, content, tool_calls}` (`syntax="chat"`). Its key is read at call time
and never logged. Credentials stay file-only because agent tools inherit environment variables.
Optional `headers` supplies fixed non-secret request headers (for example `HTTP-Referer` and
`X-Title`); these are sent as request headers and included in request captures.
`AFFORDANCE_CAPTURE_DIR` uses the same atomic request/response layout as native clients, with bearer
credentials redacted, including provider echoes in diagnostic metadata. An `anthropic` binding
follows the same rules with the Anthropic Messages wire; see
[Anthropic Messages](../docs/MODEL_BINDINGS.md#anthropic-messages). A `responses` binding uses the same machinery for native OpenAI Responses, including opaque encrypted reasoning, provider call IDs, cache keys and usage. See [OpenAI Responses](../docs/MODEL_BINDINGS.md#openai-responses). Both native hosted adapters honor provider retry waits; their timeout is per HTTP attempt. No provider state is hidden outside the app log/journal.

The host owns prices, spending limits and account state. Chat has no prices, dollar budget,
reservation ledger or inferred cost. The former `price_in`, `price_out` and `budget_usd`
configuration fields are rejected as unknown. Remove them from consumer profiles when upgrading;
normal binding-identity recovery checks still apply. The existing `max_new` request limit maps
to `max_tokens`; there is no cumulative token budget.

Per-request logs retain local prompt/output token counts and provider cached-token observations.
`model_meta.usage` preserves the provider's usage object (including any cost, currency or vendor
fields) without interpreting it, subject to credential redaction. Provider token counts describe
the provider's framing/tokenizer; local counts describe the IDs Affordance rendered/received.
A valid response does not require a usage object or provider cost report.

### Model call failures and retries

A model client makes one attempt at a call. If the attempt fails, the client reports what kind
of failure it was:

| Kind | What happened |
|---|---|
| `unreachable` | Nothing was sent: the connection was refused, the name has no address, or the connect timed out. |
| `interrupted` | The request was sent and the exchange broke: cut, reset, stalled, timed out, or ended by the server's `abort`. |
| `busy` | The server said it is busy: HTTP 429, 503 or 529. |
| `server` | The server failed: any other 5xx. |
| `refused` | The server refused the request (any other 4xx), or its reply broke the protocol or a size limit. The same request would meet the same answer. |
| `cancelled` | This side stopped the call. |
| `partial` | The call failed after part of its reply was written to a pipe. |

Whether the call is made again is decided in one place, by the retry policy:

| Field | Meaning | Default |
|---|---|---|
| `for_s` | How long, from the first failure, the call is tried again: seconds, or `"forever"`. | forever in the Rust API, 300 under the `affordance` command |
| `wait_s` | `[first, longest]`: the wait before the next attempt. It starts at the first and doubles up to the longest. | `[1, 30]` |
| `on` | The kinds that are tried again within `for_s`, from `unreachable`, `interrupted`, `busy`, `server`. | all four |
| `refused` | How many more attempts a `refused` failure gets. | 0 |

A binding states its policy as `retry` in the deployment. One call states its own with
`model(view, {retry: {...}})`; the fields it gives replace the binding's, and `retry: false`
means the first failure goes to the app. `cancelled` and `partial` failures are never tried
again.

When the server asks for a wait (`Retry-After` on any binding kind, or a hosted provider's
rate-limit reset headers), the
next attempt waits at least that long, up to 300 s.

Each failure is a journaled input carrying its kind and the server's requested wait, and the
wait runs on the instance's clock. A replay or a restore therefore retries exactly as the run
did. The trace has one `model_error` record per failure with `kind`, `failures` and
`retry_in_ms`. A restored instance keeps the policy it was started with.

When the policy gives no further attempt, the awaiting task sees the error as a rejected
promise, which the app can catch (`useLoop` stops the instance with it: `loop_error`).

A decision model whose answer is a next-token readout runs on a token-id binding: the `d1`
template app lists the screen's tools as options and reads its reply as one option code
(`spec/LANGUAGE.md` §5.2), and the binding restricts the next token to the codes.

*Not built:* readout bindings for models that score options with their own heads (the clef and
clm prompts in `views/decision/` render the option and question spans such a binding would
read); a human as a model binding.

## 7. Storage

`harness.store.*` is per-instance key-value text, persisted to `store.jsonl`. Automatic `bN`
keys skip keys already present, including explicit keys. Recovery requires string `key` and
`text` fields and a nondecreasing unsigned `next` counter in every complete record. A complete
JSON value with an invalid record shape is an error; only a syntactically torn final append is
recovered as a torn suffix. With a run
directory the store keeps each key's byte range in that file, not its text, and reads it back on
`get` and `search`.

**Memory.** Large strings are stored as mapped blobs, journals are streamed from disk on
recovery, and render caches retain recent work. These reduce resident payload memory; they do
not impose a fixed heap limit. App state, released-log skeletons, store indexes, and message
IDs used for deduplication may grow with history. Apps choose their own retention policy.
`log.release` moves record payloads to the store while retaining indexed skeletons. There is no
`resident_bytes` setting or per-instance metric; the deployment CLI rejects one rather than
silently ignoring it.

Store operations serialize within one instance; another instance's store has a separate lock.
A put publishes its key/counter/index once its complete record is written. With a run
directory the record is synced by the next journal commit, before any line that learns the key
is committed, and the commit records the store's length (below).
A failed append truncates and syncs back to the previous file end. If rollback fails too, that
space refuses further puts until a successful reload reconciles the file; existing indexed
reads remain available. A failed put does not consume an automatic key. Fresh mounts archive
old stores before loading their new incarnation.

Without a runtime (`Store` used directly), store appends are synced before success is
reported. Recovery preserves a torn final append
in `torn-store-<offset>.bin` and retains the complete prefix. Invalid complete records are
reported as corruption rather than silently discarded.

**Journal commits.** A journal record is written before its input is applied, and becomes
durable at a commit. A commit syncs every journal written since the previous one and then
appends their lengths to `<dir>/commit.jsonl`. A commit starts as soon as something waits on
one or a worker has nothing queued, and otherwise at most 2 ms after the first record it covers.
Effects that leave the process wait for the commit of the input that caused them: model
requests, machine calls, the answer to an embedder's post, and the clock record. A fresh mount over an old
instance directory (which it archives) waits for the commit too. Deliveries between instances
(`harness.send`, its acknowledgement, callbacks), timers, the instance tree and `store.*`
operations do not wait. Store records and blob files are written without a sync; a commit syncs
the store files and blob files written before it closed, with the journals, before its record.
Every line that refers to a blob or learns a store key was written after that blob or record,
so the commit covering the line covers the file. A recovery image syncs pending blobs before it
is written. On recovery every journal and store is first truncated to its length in the last
commit, or emptied if the commit does not name it for the instance's incarnation. The bytes past
it are preserved in `uncommitted-journal-<offset>.bin` (`uncommitted-store-<offset>.bin`). A
put of a text whose blob file exists checks the file's content once per process, so a file a
crash left named but empty is rewritten rather than reused. A record is written only after every record that caused
it, so the truncated journals are a state the run passed through. No receiver keeps a delivery
whose send was lost, and no sender keeps an acknowledgement for a delivery that was lost. A
crash loses only the work since the last commit, and nothing outside the process observed that
work. A clean shutdown commits everything and closes the log; a restore after it (or of a run
directory without a commit log) truncates nothing, so the journals stand as they are. Only the
log's final line may be torn (an interrupted commit); a bad line before it is an error naming
its byte offset, and recovery truncates nothing. Each truncation is reported on stderr with the
instance id and byte counts. An instance whose `instance.json` cannot be read is left as it is and
is not restored; a journal whose last record cannot be read fails that instance's restore alone.

A runtime owns its run directory: before recovery it takes an exclusive lock on
`<dir>/runtime.lock` and holds it until the run returns. A second runtime over the same
directory fails to start ("in use by another runtime") and touches nothing. The lock dies with
its process, so a crash leaves nothing to clear.

Root and child input journals use the same recovery policy as the store: preserve and truncate
a terminal incomplete append before replay and subsequent writes. This includes a write cut
inside a UTF-8 character. Complete records with invalid shapes, invalid UTF-8 bytes, and syntax
errors that are not an incomplete suffix fail without discarding those bytes. A valid final
record missing its newline is completed before appending. Preserved journal tails use
`torn-journal-<offset>.bin`; repeated recovery retains the committed prefix.

## 8. Determinism, replay and versions

A render is a pure function of the app, the tokenizer, the state, the `harness` snapshot, the
clock and referenced blobs. A handler is deterministic given its effect transcript. The journal
holds every input. Persistence requires a configured run directory; without one, execution
is in memory. Replay reconstructs computation, not live sockets, processes or a JavaScript
heap. Outstanding external calls can run again (§5); cancellation does not undo remote writes.
Deployment environment and credentials must be supplied again on restore.

**Recovery checkpoints.** With a run directory the host may write an instance's `checkpoint.json`,
an optional recovery cache. It keeps the complete journal and all historical data. A compatible
image restores supported data/metadata at a complete input boundary, then applies only newer
journal inputs. An image is of an idle agent (no pending execution, cleanup closures or
undispatched views) or of an agent at a **step boundary**: the point inside an input where the
work that ended has drained and the effects pass it asks for has not run, so nothing executes
(a `useLoop` agent reaches one at the end of every step). A step-boundary image records that the
pass is pending; restore runs it, exactly as the original did there, before the newer inputs.
The agent keeps its latest step boundary in memory (shared values, no copy) and the host writes
it when an image is due. Stopped agents may omit execution metadata that can never run again.
Functions/modules, class instances, promises, collections and rendered markup fall back to full
replay, as does any object a `memoScan` step or the turn cache made that is reachable from state
or effect dependencies (a restored agent's cold cache would make a new object where the
original reused that one); scans whose outputs stay in the render do not prevent an image.
Opaque token buffers without markup/spans are supported as data. Shared data identity, frozen
flags, undefined/numeric distinctions, environment/token-namespace references, effect
dependencies, change notes (`harness.inspect`), counters, clock/randomness and dispatch results
are preserved.

The host writes an image after 256 additional inputs (of the agent if idle, else of its latest
step boundary), and at shutdown and stop, so a close images every agent that reached a step
boundary and its restore replays at most the current step's inputs. Instances with configured
secret environment values do not use recovery caches. Checkpoints require the same app source,
entry, tokenizer, contract, complete InstanceConfig, replay mode, spill threshold and model
template; a model server's address, credentials, ranks and sampling are not part of the
identity. The cache validates its checksum, the journal's last 64 KiB before the image's
offset (so an image belongs to its journal) and the existence of every blob the journal refers
to (recorded as lines are journaled); an unusable cache falls back to full replay. Restoring
from an image reads only the journal after it: validation covers that suffix (the delivery ids
already taken are in the image), and the last clock comes from the journal's tail. A full replay still reads and validates the whole
journal. Cache-write failure leaves the durable journal authoritative. Checkpoints neither
migrate older formats nor serialize suspended stacks or host resources.
`snapshot.json` remains diagnostic.

**Pins.** `instance.json` records `pins: {contract, app, tokenizer}` from the first start. Each
journaled reply carries the hash of the prompt it answered. On restore, the first turn whose prompt
the current app no longer renders is reported (`diverged_at`). With `replay: "strict"` the restore
is refused. A journal pinned to another `contract` is always refused: its inputs mean something
else under this one, so it restores only on a binary of its own contract.

**The clock is logical.** It continues from the saved clock, so time while stopped does not expire
waits.

## 9. Limits

| Limit | Default | Effect |
|---|---|---|
| `max_new` | 4096 | Tokens per model turn. |
| `max_turns` | none | The instance stops (`max_turns`). Children inherit it. |
| `max_prompt` | none | A prompt over it is never sent; the instance stops (`prompt_over_limit`). Children inherit it. |
| Fuel | 10M steps | Loop iterations and calls per render or handler run. Running out is an error, never a hung worker. |
| Recursion | depth 400 | Catchable `RangeError`, including on runtime workers. |
| Source nesting / expression chain | 256 recursive grammar levels / 256 links | Load error. |
| State / inspection / `<Mount>` prop nesting | 128 container levels | Explicit error; circular serializable graphs are refused. |
| JSON.stringify nesting | 400 container levels | Catchable `RangeError`. |
| JSON.parse nesting | 5000 container levels | Catchable `RangeError`; any call depth, any worker stack. |
| Child list nesting | 400 levels | Catchable `RangeError`; a child list inside itself is a `TypeError`. |
| String length | 536,870,888 code points (V8's) | Catchable `RangeError: Invalid string length`, before allocating. |
| Bulk array length | 10M items | Catchable `RangeError` from a step that makes or grows an array by many items (`Array(n).fill`, spreads, `concat`, `split`, borrowed arraylikes); native flattening also consumes fuel. |
| `call_timeout` | 3600 s | Per machine call. |
| Exec capture | 16 MiB | `truncated: true`. |
| `command` reply (`max_reply_bytes`) | 128 MiB | The call fails and the command is stopped (a JSON reply cut short is not a result); stderr is kept to 64 KiB. |
| `spill_bytes` | 64 KiB | Strings past it live on disk. |
| `model_inflight` | `model_threads` | Model calls in flight, balanced across ranks. Adjustable while running (`Control::set_model_limit`). |
| `restore_concurrency` | CPUs available | Instances restoring at once on restore; the rest wait. Bounds the boot peak to the steady heap plus this many restores. |

*Not built:* storage quotas, per-minute call rates.

## 10. Operating a run

Root specs accept `dedicated_worker: true`: allocate an additional render worker for that root
and its mounted descendants, excluding other roots. `workers` still sizes the shared root
pool. Placement applies to startup/restore membership; there is no dynamic root-spawn API.
Reports expose the physical worker index as diagnostics, not app state. Which root to isolate
is deployment policy; there is no built-in channel hub. Children retain parent-worker placement.

Multicast uses the existing `send` capability and delivery-id replay deduplication (a send's
event carries its message id as the journaled event's `id`, never in its payload). It is not an
atomic multi-recipient transaction. A queued delivery dropped before commit reports failure;
a completed aggregate result is replayed without sending again. Unfinished sends may repeat,
and already-journaled receivers deduplicate them. Old sender incarnations cannot receive late
acknowledgements. Journals are synced together by commits (§7), across instances and workers.

With `dir`, the run directory holds the run's durable state (a controller uses `Control`, not
these files). An instance's name is a label and never becomes a path: each instance, a child
included, lives in `instances/<id>/` under an id the runtime assigned when it was first created
(a UUIDv7), and its `instance.json` records its name. The run's own files sit beside `instances/`,
where no name reaches them.

| Path | Meaning |
|---|---|
| `run.json` | The run's stamp: the contract it was written under. A run directory belongs to the version that wrote it: a runtime refuses one stamped with another contract (or a version before stamps left: a lock and no stamp) before reading anything in it, and `--fresh` sets the whole directory aside under `retired/`. |
| `instances/<id>/instance.json` | The instance's name, spec and pins. Secrets are removed. |
| `instances/<id>/journal.jsonl` | Its inputs. **Sensitive:** tool outputs and messages are stored as they were. |
| `commit.jsonl` | The journals' commits: each journal's committed length, by instance (§7). A restore truncates journals to the last one. |
| `runtime.lock` | Locked by the runtime that owns the directory (§7). |
| `instances/<id>/trace.jsonl` | Renders, replies, model calls (usage, cost), semantic tool completions, handler errors, stops. Secrets redacted. |
| `instances/<id>/snapshot.json` | Diagnostic state and metrics at shutdown. Secrets redacted. |
| `instances/<id>/checkpoint.json` | Optional verified recovery image and journal cursor (§8). |
| `retired/<id>/` | An instance a fresh mount (or a fresh run) replaced, as it was. |
| `blobs/` | Large strings (§7). |
| `clock.json` | Where the logical clock stood (private recovery state, not an interface): rewritten about every 100 ms by its own thread, and at the end of the run, so a restore continues the clock from there, even after a crash during a long quiet wait. |

A run's live status is `Control::status` (also `status` over stdio or the control socket): its
`state`, `clock_ms`, the model bindings' windows (`models`: `{name: {context, source}}`, §6
`context`; `source` is `server /v1/models`, `server /get_server_info`, `configured`, or
`configured (the server reported none: ...)`; null without a window), the model queue, and each
instance. An instance's status has its `state` (`starting`, `idle`, `model`, `tool`, `waiting`,
`retrying`, `stopped`, `failed`), `healthy` and `handler_errors` (the errors thrown by its
handlers, effects and callbacks, which no awaiter may see). While a model call retries a failure
(§6), the state is `retrying` and `retrying` is `{failures, error}`: the failures so far and the
last one's kind (`rate_limited`, `server_error`, `rejected`, `unreachable`, `timeout`,
`aborted`, `cancelled` or `error`; never the server's address). A retrying instance is still
healthy; otherwise `retrying` is null.

`AFFORDANCE_CAPTURE_DIR` (`RuntimeConfig.capture_dir` in Rust) captures every model request and
response of the run. Capture is best-effort, like a browser's network log: a failed write never
fails or resends the call; it is logged, counted in the status's and the run report's
`capture_errors` (per run), and leaves no file.

### Semantic tool completion traces

The journal's clock (`clock_ms`) is logical: it stands still while a run's agents restore and
from the moment the run begins closing, and continues from there, so neither a slow restore nor
the time a run is closed takes anything from waits and timeouts. Each start and restore writes a trace record
`{t:"clock", at_ms, wall_ms}`; until the next one, wall time is `at_ms + (wall_ms - at_ms)`.

Every trace record has `at_ms`, its time in the journal's clock (`clock_ms`): when what it
records began where that differs (a tool's start, a model call's start), else the input being
handled. Records link a turn's work: a page call's `render` and `reply` carry its `turn` (and the
render the `task` of the model call it issued, which the `generate` record carries), and a `tool`
record the `turn` whose reply asked for it.

A page call's `render` record says how much of a prefix-caching server's work it keeps: `shared` is
the number of leading tokens it shares with the previous page prompt and its reply, and `append`
is `null` (the first call), `"exact"` (it starts with all of them), or `{rewrite_at, of}` (it
differs from token `rewrite_at` on, of the `of` tokens the server held). The `generate` record
carries the server's own `cached_tokens` when it reports one.

At the first answer of a valid dispatched tool, the runtime emits one additive
`{t:"tool", tool, at_ms, turn, dispatch, index, ms, status, timed_out}` trace, from `at_ms` for
`ms`. `tool` is the
canonical tool name; names over 128 Unicode characters use a 63-character
prefix plus `#` and the full SHA256 of the name to remain bounded and distinct.
`(agent, dispatch, index)` identifies the semantic call. `ms` is nonnegative logical elapsed milliseconds
from actual dispatch, not the earlier clock captured by its view. It includes
logical time passing across close/restore; it is not executor CPU time. `status` is
`"error"` for a thrown handler error or, in a `{result, ...meta}` answer, explicit
`error:true`, `timed_out:true`, or a nonzero integer `exit`; otherwise it is `"ok"`.
Other returned values are ordinary result data, not metadata envelopes.
`timed_out` comes only from explicit boolean metadata in that envelope; private
text and exit codes alone are never interpreted as timeout evidence. Apps must preserve that metadata (the
session shell does on both inline and stored-output paths). A normal `wait`
deadline is a successful answer, not an executor timeout.

Arguments, call IDs supplied by the model, result bodies and error text are
excluded. Machine/store/send subcalls, callbacks and pending or rejected
undispatched calls do not produce semantic completion events. An early answer
is counted when returned even if background work continues; its later success
or failure produces no second completion. Restore discards reconstructed
traces and preserves outstanding work, so restoring completed calls does not
append duplicates. Traces are best-effort diagnostics, not a transactional
exactly-once lifetime ledger: a process crash between journal and trace writes
can lose a completion. Consumers count completions in their bounded trace tail;
old uninstrumented traces do not establish zero calls or complete coverage.

Runtime source modules are supported through `loadModule` from `affordance:compiler`
([LANGUAGE.md §2.2](LANGUAGE.md#22-runtime-source-modules)). Store namespaces in hook state;
recover source from journaled inputs. Pending functions retain their original lexical module
across replacement. This uses existing journals, not a new persisted-code format. Nested source
components share one instance; a file-backed wrapper may load source from props in a separate
`<Mount>`. Startup and runtime module initialization are both pure, and every function value owns
its lexical environment.

## 11. Not built

- **A parent showing a child's screen** inside its own render (a window in a window).
- **Live network streams and app-facing AbortSignal.** Buffered global Fetch exists over
  a bound HTTP executor; it does not expose sockets or streaming bodies.
- **Readout model bindings, human bindings, storage quotas, call rates** (§6, §9).
