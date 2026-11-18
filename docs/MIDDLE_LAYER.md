# App, turn strategy, template app

Contract authority: [public contracts](../spec/CONTRACTS.md),
[language](../spec/LANGUAGE.md) and [harness](../spec/HARNESS.md).

The middle layer accepts an app and a replay log. It chooses which observations
and interactions appear as turns. A model-template app consumes those turns and
owns protocol tokens. `messages` is a compatibility input only at that final
boundary. Product rendering and action execution stay in Rust.

```tsx
import { ModelContext } from "@affordance/lib/model.tsx";

export default function Main({ numbers, log }) {
  return <ModelContext strategy="last-turn" log={log}>
    <FindBigNumber label="find-big-number" numbers={numbers} budget={256}/>
  </ModelContext>;
}
```

An app is one file for every model. `ModelContext` (`views/lib/model.tsx`) takes `system`,
`log`, `strategy` and the screen as children, and writes the prompt with the template app the
model binding names: `model.template` in a deployment, with its `template_options`. Outside a
run, `affordance render APP.tsx --template NAME` (and `View::for_model` in Rust) makes the same
choice. `views/demo/find_big_number.tsx` is this example in full; it renders for
Qwen3-Instruct-2507, Qwen3 hybrid thinking, Qwen3.6, Nemotron 3 Nano, Llama 3.1, GLM-5.3 and
MiMo-V2.5 from the one file. The template names and their tokenizer models are in
`views/template_apps/index.json`. The shared registry loads every stock template safely under
each registered tokenizer. Selected templates still require their own special tokens.

`TurnStrategy` is an ordinary component, backed by Rust's `turnStrategy` builtin.
It projects the current app's visible actions after layout, then produces a
`TurnPlan`. Capture carries that plan as a typed item. `templateInput` consumes
exactly one plan; mixed children and an unconsumed plan fail explicitly.

| Policy | Prior screens | External input, model output and committed results | Current screen |
|---|---|---|---|
| `unrolled` | Every observation | Retained in log order | Final user turn |
| `last-turn` | Omitted | Retained in log order | Final user turn |

The caller owns the complete log. Neither policy mutates it. Metrics and events
stay in that source log without entering the prompt. These are two starting
policies; truncation, compaction and folding can replace the selection logic.

```json
[
  {"kind":"input","content":"Remember the goal."},
  {"kind":"screen","content":"Prior observation."},
  {"kind":"model","content":"","calls":[{"type":"function","function":{"name":"scroll__notebook","arguments":{"by":20}}}]},
  {"kind":"action","name":"scroll__notebook","result":"ok"},
  {"kind":"metric","prefill_ms":7}
]
```

`model.reasoning` maps to the template's reasoning field. Action records may
carry `tool_call_id`. Any record may carry exact token `ids` for its body instead (a
`model` record: the generated ids, rendered after the generation prompt), and the
current screen is the `turn:current` span of the render; see `docs/HARNESS_GUIDE.md`
for the record schema, the ids semantics and which template apps support them. A runtime can attach timing and replay metadata to every
record. The app owns this log as state, commonly through `useLog()`; the runtime journals
inputs and reconstructs app state on replay. An app may choose its own history schema.

`useLog()` exposes render-time `records`/`open` snapshots; its methods operate on current hook
state. `release(upto, keep?)` accepts a nonnegative safe-integer exclusive boundary, clamped
to the records present when it runs. It stores the unreleased prefix before replacing its
payloads with skeletons. Overlapping releases retry from the committed prefix; a failed write
leaves those records intact. `read(from, to)` snapshots current records and their batch
metadata together, then restores any released payloads. Append, release and read may therefore
run sequentially in one handler without waiting for another render.

Turn content accepts strings and native fragments. A current fragment holds
its original token IDs, spans and handler references. Token positions are rebased
inside the final model prompt. Historical fragments keep IDs and diagnostic
ranges, but lose callback and projected-tool metadata, so past screens cannot
dispatch past handlers. The live tool table comes only from the current app.
Outer clipping of already-projected live actions raises; put budgets inside the
app. `isFragment`, fragment-aware `trim`, and `asContent` let templates preserve
these values without flattening them into text.

Native boundaries intentionally preserve IDs. Arbitrary leading/trailing
whitespace can still differ from HF's concatenated-string tokenization or
string trimming; the existing adapter boundary study applies. FindBigNumber
places row separators before rows and a final newline outside the notebook
budget. This avoids a newline merge across the budget boundary in its complete
screen. Full-screen token exactness is audited independently through Qwen3.6
and Nemotron 3. Clipped screens preserve native IDs/actions; arbitrary clipped
screens are not claimed to equal full-string HF tokenization.

The FindBigNumber app has a goal box, a scratchpad, a token-budgeted notebook
with 1,024 seeded random numbers and scroll, an answer field, and submit. Its
handlers do not know the maximum. Submit returns the current answer as state.
The tests exercise real handlers, switching templates with preserved hook state,
historical fragment isolation and visibility after layout. This is a rendering
and plumbing result, with no learning claim.

```sh
cargo build --offline --release --example middle_demo
./target/release/examples/middle_demo --sample --family qwen36 --seed 20261008 --budget 96
./target/release/examples/middle_demo --sample --family nemotron3 --seed 20261008 --budget 96
# stdin: {"states":[STATE],"calls":[[{"name":"scroll__notebook","arguments":{"by":100}}]]}
```

`middle_demo` returns JSON containing next state, IDs, spans and diagnostic text.
The optional `--tokenizer PATH` binds it to an external model tokenizer. Model
inference lives in an adapter, not this driver.

Reference tests require the original cached tokenizer files, as the rest of the
repository does. Fetch the pinned NVIDIA files, then generate references on CPU:

```sh
.venv/bin/python tools/fetch_middle_reference.py --destination ref/middle_models/nvidia__NVIDIA-Nemotron-3-Nano-30B-A3B-BF16
TOKENIZERS_PARALLELISM=false .venv/bin/python tools/middle_references.py
cargo test --offline --release --test middle
cargo test --offline --release
```

The NVIDIA source revision and file hashes are recorded in
`ref/middle_out/nemotron_source.json`. Expected IDs come from its official HF
template. Reference Python constructs bodies, logs and schemas independently;
it never consumes Rust-rendered text or IDs. The hand-written views' index,
references and conformance tests are unchanged. The new tests cover 56
composition cases, 16 complete 1,024-number prompts and 120 separate NVIDIA
thinking/tool/history cases.

`useLog` owns one turn through calling, ready, dispatching and closed states. Its public
`open` remains a boolean. Pass the output of `log.call()` to `log.dispatch(out)` or
`log.close(out, why)`; output ownership travels with that value. Await pending work before
closing. Old closes cannot close a newer turn, and stale/cross-log dispatches fail explicitly.
The core's independent `model()` calls and idempotent `dispatch()` remain available to apps
that implement another policy.

The runtime and `useLog` library supply model/dispatch records, and the session app demonstrates
folding. An app gives a model images, audio and video with `<img>`, `<audio>` and `<video>`
(`docs/HARNESS_GUIDE.md`); resizing or re-encoding a file first is the integration's work. Experimental alternate engines
belong to the separate research project; the shipped framework uses the Rust interpreter.
The portable queued Qwen3-4B integration adapter checks native/HF equality before
each frozen model call. Its full-screen task does not measure scrolling ability
or establish a learning advantage.
