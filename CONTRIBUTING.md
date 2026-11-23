# Contributing

Start with [your first app](docs/GETTING_STARTED.md), then read the
[public boundaries](spec/CONTRACTS.md). Affordance is currently an experimental Rust framework
with a TSX application language. Python reference tools belong to development; training and
research applications belong to the separate research project.

For a small engine or documentation contribution:

1. Describe the observable problem, expected behavior and a minimal example.
2. Locate the owning layer: application policy stays in TSX, external I/O stays in the host,
   and generic language/platform primitives live in the engine.
3. Make the smallest coherent repair. Consolidate the shared mechanism when a bug has several
   copies; avoid another special case or compatibility wrapper.
4. Verify the behavior through the existing corpus or scenario runner. Prefer an independent
   oracle and an adjacent boundary case to implementation snapshots. Remove obsolete checks
   when the mechanism changes. See [the validation guide](docs/DEVELOPING.md).
5. Run `./tools/check-engine.sh`. It needs Node, but no downloaded models or Python
   reference environment. Template changes also need the full conformance gate documented there.

What the checks on a pull request cover. `./tools/check-engine.sh`, which GitHub runs, is every
test that needs no downloaded reference data. The tests that compare a model's prompt with its
publisher's own template and tokenizer, and the tests of pictures and sound on real model
formats, need about 8 GB of tokenizers and official outputs that the repository does not
carry. A maintainer runs that full suite (`./tools/test-parallel.sh`, in
[the validation guide](docs/DEVELOPING.md)) on a machine that has the data before a pull
request is merged and before a release is tagged. A green check on GitHub therefore means the
engine's own tests pass; it does not yet mean every model's tokens were compared.

Explain what changed, why, and what you ran in a pull request. Do not include local caches,
model weights, run directories, or generated study outputs. Public contract changes should
include their design argument and documentation in the same contribution.

## Changing the platform

The runtime, `harness.*`, effects, journal and replay semantics, and the app-facing contracts are
the platform, held to the standard of a core web API. Before changing them, write the argument
down (a design note for a large change, the pull request for a small one): [docs/DESIGN.md](docs/DESIGN.md)
shows the test applied to decisions already made.

1. **The browser parallel.** What does the web platform use for this need (the event loop,
   `setTimeout`, `fetch` with `AbortController`, `Promise.race`, `postMessage`)? How would a web
   app solve it with standard APIs alone?
2. **Where it lives.** Most needs are compositions of existing primitives: show the composition
   first. App behavior belongs in a view or library; a deployment's policy belongs to the program
   that runs Affordance. Only a genuinely missing primitive belongs in the engine.
3. **The smallest general primitive.** Generic (no product, vendor or one client's feature),
   useful beyond the case at hand, deterministic under replay, and consistent with cancellation,
   close and restore.
4. **The standard cases still read naturally:** a chat bot, an autonomous tool-using agent, a
   multi-agent fleet, and a resumed journal. If one needs a workaround, the change is wrong.

Affordance is licensed under [Apache-2.0](LICENSE); a contribution is offered under the same
license. No stable Rust ABI, complete JavaScript/npm compatibility, or cross-platform runtime
support is promised yet.
