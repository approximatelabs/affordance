# Public contracts and ownership

This is the map of Affordance's shipped contracts. [LANGUAGE.md](LANGUAGE.md) is normative
for source evaluation and document layout; [HARNESS.md](HARNESS.md) is normative for instance
execution and recovery. The supported HTTP/Fetch and MCP surfaces are specified in
[HTTP_AND_MCP.md](../docs/HTTP_AND_MCP.md). Guides explain those contracts.

Affordance implements a documented TypeScript/JSX subset. Familiar syntax is not a promise
of complete ECMAScript, browser API, Node or npm compatibility. Supported features, intentional
differences and unsupported features are listed in the language and HTTP specifications.

## Boundaries

| Owner | Responsibilities |
|---|---|
| App and TSX libraries | State, conversation/history, control flow, context selection, tools, protocol clients such as MCP, and which actions a model sees. |
| Language and layout engine | Source/module loading, owned lexical scopes, components, text/token values, budgets, spans and action attachment. |
| Instance state machine | Applying one input at a time, tasks, effects, hooks, mounts, model calls requested by the app, and dispatch against a captured view. |
| Host | Scheduling, journaling, external model/executor I/O, capability checks, cancellation and storage. |
| Deployment | Models, executor roles, credentials, limits and whether a run has a persistence directory. |

File and runtime source modules share explicit imports/exports, platform globals and documented rendering inputs. Runtime loading adds no I/O capabilities; apps own source acquisition and revision history. Instances share only the
documented JSON props, callback events, messages and inspection interfaces. A library import
does not grant host capabilities. Child bindings may narrow their parent's authority.
Conversation policies remain libraries even when the engine supplies layout helpers they use.

## Document and computation

A render is a pure computation from the app source, bound tokenizer, state, harness snapshot,
logical clock and referenced blobs to exact token IDs, labelled spans/actions and child mount
declarations. Render performs no external effects or state writes. Hooks may declare initial
values and effects; the instance commits them through its execution loop after successful
root evaluation/layout. Components and native elements share typed style computation and token
box layout. Hiding/clipping affects presentation and model actions, not component/instance
lifecycle. Removing an element cleans up its effects and local state; instance-wide named
state remains explicit. Child mount declarations and harness callbacks survive presentation
clipping. Structural `key` identity is independent of style labels.

Plain text joins into tokenizer runs. Special tokens and token-budget boxes establish the
specified boundaries. Layout determines which actions survive; hidden or partly clipped
actions cannot be offered to the model. Already laid-out fragments preserve their IDs. Exact
output is defined by those rules, not by equivalence to tokenizing an arbitrary flat string.

A page `model()` call captures its laid-out view and handlers. `dispatch(output)` runs the
handlers from that captured view, even if the current state has since changed. Dispatch of
ad-hoc model views is unsupported. Output retention, repeated dispatch and validation follow
the harness contract.

An instance applies ordered inputs to its state and emits commands for its host. A function's
synchronous prefix runs immediately; awaits and reactions resume through its FIFO microtask
queue. Saved frames retain locals, operands, references and exception state. State writes
apply once against live state. Recovery restores an eligible verified data checkpoint and feeds newer journaled inputs
through the same executor, or replays the complete journal. It does not serialize suspended
interpreter/native stacks or operating-system resources.
The standalone transcript API reconstructs one execution and compares each request exactly.

These are observable semantic contracts. Rust ASTs, interpreter frames, emitter items and
memory layouts are implementation details, not a portable bytecode or stable binary ABI.

## Persistence and failure

| Boundary | Guarantee and limit |
|---|---|
| Input -> instance | With a run directory, write and sync the input before applying it or dispatching its resulting commands. Without a run directory, execution is in memory. |
| Completed effect -> recovery | A journaled result is replayed without executing the external operation again. |
| External operation -> journaled result | Outstanding calls can run again after a crash. Execution is at-least-once; external writes need application/server idempotency. |
| Store append -> caller | Sync the append before reporting success; recover the complete prefix after a torn final append. This is not a transaction spanning remote writes and journals. |
| Instance -> child | Separate state and journal; JSON props and callback events cross the boundary. Remount starts a fresh incarnation; process recovery restores the current one. |
| Process -> recovery | Rebuild from persisted inputs, optionally starting at a verified compatible data checkpoint. Unsupported live state uses full replay. Complete journals remain retained; host resources are not serialized. |
| Version -> replay | Pin the harness contract, app and tokenizer. Refuse another contract version; report source/tokenizer changes and prompt divergence, refusing prompt divergence in strict mode. This is not automatic state migration. |

Deployment credentials and environment are supplied again on restore. Changing code, bindings
or environment can change future behavior. Cancellation stops participating host work; it
does not roll back a remote operation that has already taken effect.

## Evidence

`tests/hardening.rs` checks reproduced boundary violations, class lifetimes, lexical intrinsics,
state admission and process-crash containment. The [security gate](../docs/SECURITY.md) checks
dependencies and reviewed exposure exceptions. `tests/modules.rs` checks independent module scopes, import/export failures, closure/class
behavior, schemas, source loading, mounts and effect replay. `tests/lifecycle.rs` covers
source revision changes and recovery, and `tests/closures_freed.rs` checks reclamation. `tests/features.rs`, `tests/chat_tools.rs` and
`tests/adapter_contract.rs` check document boundaries and action visibility. `tests/page.rs`,
`tests/effects.rs` and `tests/promises.rs` check task, model and dispatch semantics.
`tests/reliability.rs`, `tests/spill.rs` and `tests/pause_latency.rs` check recovery and lifetime
boundaries. Storage/checkpoint tests also cover append rollback, per-instance locks,
identity-preserving images, cache invalidation and parent/child suffix recovery. `tests/conformance.rs` compares exact IDs against cached official templates;
`tests/js_differential.rs` compares the supported language subset against Node.

These are finite conformance checks. They do not establish full JavaScript/library
compatibility, arbitrary-input exactness, or performance on unmeasured workloads.
