# Harness guide: driving an affordance app from Rust

Contract authority: [public contracts](../spec/CONTRACTS.md),
[language](../spec/LANGUAGE.md) and [harness](../spec/HARNESS.md).

For a complete first render/action/rerender without a model server, start with
[the embedding notes](MODEL_BINDINGS.md#rust-embedding) and [hello.rs](../examples/embedding/hello.rs).

For harness authors who render an agent app to token ids, send them to a token-id endpoint
(SGLang `/generate` with `input_ids`, vLLM `/v1/completions` with a token prompt), and feed the
reply back. The worked example is `views/demo/consumer/` (one app, three template entries) and
`tests/harness_consumer.rs`, which runs every snippet below.

## 1. Load once, render from many threads

`Encoder` is `Send + Sync` (a compile-time assertion in `src/emit.rs` keeps it that way). Load
the tokenizer once per process and share it. `View` holds `Rc` values and is `!Send`, so each
render thread loads its own `View`, bound to the shared encoder:

```rust
let enc = Arc::new(Encoder::from_file("ref/tokenizers/zai-org__GLM-5.3.json")?);
for _ in 0..workers {
    let enc = Arc::clone(&enc);
    std::thread::spawn(move || {
        let view = View::load(Path::new("views/demo/consumer/glm53.tsx"), "Main", &enc).unwrap();
        let mut cache = RunCache::new(true);           // per thread, keep it across renders
        loop { /* view.render(&enc, state, &mut cache, now) */ }
    });
}
```

GLM-5.3, a 29.5K-token session, 20 renders per thread (`tools/bench/render_threads.rs`):

| setup | 1 thread | 16 threads |
|---|---|---|
| `Encoder` + `View` per thread | 200 MiB | 3,099 MiB |
| one shared `Encoder`, `View` per thread | 203 MiB | 240 MiB (1.18×) |

The encoder accounts for about 196 MiB. Each extra thread adds about 2.5 MiB (its `View`,
`RunCache` and allocator arena). Warm render p50 is 2.7–3.0 ms in both setups.

## 2. Tool names: `tool_name`

A model-callable action is an `<aff>` with an `on*` handler. When `asChatTools` (inside
`TurnStrategy`) projects it to a function schema, the name defaults to `verb__label`: `onRun` on
`label="shell"` becomes `run__shell`. Set the name explicitly with `tool_name`:

```tsx
<aff label="shell" tool_name="shell" description="Run a shell command in your VM."
     args_schema={{type: "object", properties: {command: {type: "string"}}, required: ["command"]}}
     onRun={async ({ command }) => { ... }}>{"[shell]"}</aff>
```

Other props: `description`, and `args_schema` (a JSON schema; otherwise the schema comes from
the destructured parameters). Duplicate names get an `__h<n>` suffix. The rendered affordance
span has `label == tool_name`, so `View::act_effects(.., "shell", 0, args, ..)` dispatches the
model's call by the name the model used.

## 3. Strings and fragments

| Written as | Value |
|---|---|
| `"a" + b`, `` `a${b}` `` (plain template literal), `String(x)`, `JSON.stringify(x)` | string |
| `` p`a${b}` `` (tagged prompt literal), JSX (`<>...</>`, `<box>`), `tok.x + "..."` | fragment: text, special tokens and laid-out ids |

A fragment keeps token boundaries and special tokens; a string is plain text and is
tokenized where it is emitted. Read-only string methods (`startsWith`, `includes`, `indexOf`,
...) and `trim` work on fragments. Indexing (`c[0]`) is an error for a general fragment and names the fragment (see section 7).
An exact token-run fragment supports numeric indexing of its token IDs. A fragment joins
only with text: adding an object or a number to it is a TypeError, so convert first.

In a `TurnStrategy` log:
- **screen / input `content`, action `result`:** a string, a fragment, a list of content parts, or
  exact `ids` (section 5). GLM53App, Qwen3App, Qwen36App, Qwen3BaseApp, Llama31App, MiMo25App
  and MessagesApp accept fragments there (checked with `ids` records). Nemotron3App is
  unchecked: its tokenizer is not in this checkout. A map is rejected: serialize it first
  (`JSON.stringify(x)`).
- **model `content` and `reasoning`:** strings. `calls`: OpenAI-style
  `[{"type": "function", "id"?, "function": {"name", "arguments": {..}}}]`, with arguments as JSON
  values.

Use a plain string when you only need text. For exact token round-trips, log ids instead.

## 4. `TurnStrategy` log records

`<TurnStrategy strategy="unrolled" | "last-turn" log={log}>{app}</TurnStrategy>` turns the log
into chat turns. The current screen (the app) is the final user turn; an app with nothing on its
screen (a plain conversation) adds none, and the prompt ends with the log's last turn.

| `kind` | turn | fields read |
|---|---|---|
| `input` | user | `content` or `ids` |
| `screen` | user (`unrolled` only; skipped by `last-turn`) | `content` or `ids` |
| `model` | assistant | `content`, `reasoning` → `reasoning_content`, `calls` → `tool_calls`; or `ids` |
| `action` | tool | `result` (text, or a list of content parts: `{type: "text", text}`, `{type: "image_url", image_url: {url}}`) or `ids`, plus `name` and `tool_call_id` |
| `metric`, `event` | none (kept in the log, skipped) | none |

Any other kind is an error naming the record (`TurnStrategy log[7]: unsupported log record
kind "note"`). Every other field (`id`, `seq`, `pinned`, `exit`, timings, effect transcripts) is
ignored, so you can pass your full event log. Records are plain JSON. Inside the view you can
filter and rewrite the log before passing it in (the consumer's `selectLog`).

## 5. Exact ids: `ids` on records, and the current screen span

Exact-append prefix caching needs the next prompt to start with
`previous prompt ++ generated ids`. Text records cannot guarantee that:
- templates normalize (GLM strips content, harnesses trim reasoning);
- a decoded screen re-encodes differently across `<aff>` runs (in GLM-5.3, `[shell][send_message]`
  merges `][`);
- a model may emit non-canonical tokenizations.

Log ids instead.

**Screens.** `Rendered::current_turn()` is the token range of the current screen's body (span
label `turn:current`, without the template's role tokens). `current_turn_ids()` returns its ids:

```rust
let r = view.render(&enc, state, &mut cache, now)?;
log.push(json!({"kind": "screen", "ids": r.current_turn_ids().unwrap()}));
```

**Model turns.** Log the generated ids exactly as the server returned them, including the stop
token:

```rust
log.push(json!({"kind": "model", "ids": output_ids, "reasoning": parsed_reasoning, "calls": parsed_calls}));
```

The template app renders the turn as its generation prompt, the prefix `add_generation_prompt`
appended to the prompt the model saw (GLM-5.3: `<|assistant|><think>`; Qwen3.6:
`<|im_start|>assistant\n<think>\n`), followed by `ids` verbatim:
- With `ids`, `content`, `reasoning` and `calls` are **metadata only**. They stay in your log and
  are not rendered.
- **GLM** stops on the next turn's opener (`<|observation|>` or `<|user|>`), which the following
  turn emits, so one trailing opener is dropped.
- **Qwen** emits `<|im_end|>\n` after the ids, or only `\n` if the ids already end with
  `<|im_end|>`.
- Exact model turns are supported by every stock template app except **MessagesApp**: GLM53App,
  Qwen3App, Qwen36App, Qwen3BaseApp, Llama31App, MiMo25App and Nemotron3App. MessagesApp rejects
  them with an error naming the record. `input`, `screen` and `action` ids work in the template
  apps listed in section 3.

**Action results and inputs** may carry `ids` too, for example to replay a tool result exactly.

**In a template**, a generated turn's `m.ids` is a token run, the same fragment as its content (4
bytes an id, not a list value per id). It reads like a list of ids: `.length`, `[i]`, `.at(-1)`,
`.slice(0, -1)` (another token run), `.includes(id)`, spreading and `for … of`. `tokens(m.ids)`
passes it through.

**Images, audio and video.** An `<img>`, `<audio>` or `<video>` in a model's prompt lays out
its placeholder tokens, and the model request carries its `src`. A picture and a sound clip
are sized by layout where their model has metrics (`spec/LANGUAGE.md` 4.1): the style sheet
gives them a size in tokens (`img { max-tokens: 256tok; }`), layout holds that many
placeholders, and the request carries the file resampled or trimmed to it. A video is shown
as its frames (`views/lib/media.tsx`), each a picture. With `tok_size` an element is that many
placeholders and the file as it is. An SGLang binding sends `image_data`, `audio_data` and `video_data`,
one entry per element, each list in prompt order. A chat binding (vLLM's chat endpoint) sends
each as a content part of its message: `image_url`, `input_audio` (or `audio_url`) and
`video_url`. A template renders a content part that has a source as such an element (DeepSeek
V4.1: one `<｜deepseek_image｜>`, which the server expands, so the ids stay exact). A model
bound by its registered name gets the placeholder its publisher's template writes for each
kind; `views/index.json` records which kinds a model takes (`agent.media`). A tool returns an
image, a clip or a video as content parts; its record keeps them. A screen record keeps its
elements as `images: [{at, n, src}]`. A source is the file's bytes as a data URL
(`data:image/png;base64,`, `data:audio/wav;base64,`, `data:video/mp4;base64,` and so on); an
image may also be bare base64 PNG, JPEG, WebP or GIF bytes. A URL or a path, which the model
server or provider would fetch or open for the agent, fails the call before it is sent (CSP
`img-src data:`); a template still renders it (the ids stay exact). A completions binding
cannot send files and refuses such a request, naming the chat kind. Requests with files skip
sessions.

What SGLang does with a prompt's token ids when a file comes with them depends on the kind of
file and the model family. It either uses the ids as sent, or decodes them to text, runs the
publisher's processor on the text and tokenizes again. An app's ids are not always the ones the
tokenizer gives for the same text (two elements side by side are tokenized apart), so after a
second tokenizing the model can read different tokens from the ones sent. Measured on SGLang
0.5.23 by asking the server for the prompt tokens it processed:

| Model | Picture | Video | Sound |
|---|---|---|---|
| Qwen3.5-4B | the ids sent | tokenized again | |
| Gemma 4 E2B | the ids sent | fails in the server | tokenized again |
| GLM-4.6V-Flash | fails in the server | tokenized again | |
| Kimi-VL-A3B | tokenized again | | |
| LFM2.5-VL-1.6B | tokenized again | | |
| Ministral-3-8B | tokenized again, special tokens split into text | | |

For Mistral's models the second tokenizing also writes the model's special tokens (`<s>`,
`[INST]`) back as plain text, so the model reads a different prompt; the run counts it in
`token_mismatches`. Bind those models with the chat kind on SGLang.

SGLang keeps a client's ids only for pictures, and only where its processor for the family can
count a picture's tokens (`SGLANG_MM_AVOID_RETOKENIZE`, on by default). GLM's picture failure
is in that path: the ids are kept, and a mask from the tokenized-again prompt is applied to
them (`The shape of the mask ... does not match`). Where exact ids matter for a request with a
file, check the model first: `tools/check-media.sh` shows a real model a picture, a video and a
sound clip through its ids, says which it named, and fails when the server processed other
tokens than the ones sent. With the chat kind the server writes the prompt and no ids are sent.
SGLang's prefix cache stops at the first image, so an app that wants long contexts cached shows
an image for a turn and then renders a text stand-in for it (app policy, like folding).

**Reducing a record** (folding): `ids` take precedence over the text fields, so a view that
rewrites old records must drop them. `{...e, reasoning: "", ids: null}` renders from the fields
again.

`tests/harness_consumer.rs` checks the invariant for GLM-5.3, Qwen3.6 and Qwen3-2507. With ids,
`prev_prompt ++ generated` is a prefix of the next render on every turn of a 10-turn session.
That session includes whitespace before `</think>`, non-canonical generated ids, text-only
replies, an injected input and adjacent `<aff>` labels. The same session logged as text breaks
the prefix in every family.

## 5a. Incremental renders: `memoScan`

An agent re-renders its whole history every turn. With `memoScan`, a pass over the transcript
does the work of what is new:

```js
const r = memoScan(items, init, (state, item, index) => [nextState, out], deps);
// r.state: the final state; r.outs: [out, ...], one per item
```

- **Contract:** the step depends only on its arguments, declared dependency values and invariant
  module constants. Module constants are evaluated per document: include one in `deps` when its
  identity matters or its value can vary between documents. Dependency arrays compare entries
  with `Object.is`, like effect dependencies;
  their own identity is not a dependency. Other dependency arguments compare with `Object.is`.
  Omit `deps` or pass `null` when there are no captured dependencies.
- **Reuse:** the initial state and each item compare with `Object.is`: primitives by value,
  objects, functions and fragments by identity. A fresh object safely causes recomputation.
  Kept inputs and results are frozen by the runtime; copy them to change their contents.
  For record/array accumulators, use a `null` seed and initialize it on the first step, as the
  shipped templates do; module constants are evaluated per document. When the inputs match,
  the kept states and outputs are reused for the matching prefix, and only
  the rest are stepped. Reused outputs are the kept values themselves, so a downstream scan over
  them hits too.
- **Identity:** a scan is its call site, component path and order among that site's calls.
- **Generations:** a scan not used in a turn is dropped at the next turn.
- **Checking:** `AFFORDANCE_MEMO_VERIFY=1` also computes reused scans from scratch and compares result
  structure, including complete fragment spans. This diagnoses undeclared dependencies; it is
  not a proof of equivalence for arbitrary JavaScript objects or captured function environments.
- **Without an agent** (a plain render, `affordance check`), `memoScan` is the plain fold.

Lookahead becomes state. A message whose output depends on the next message is written when the
next one arrives (DeepSeek V4.1's generation prompt after a user turn), or a block is held in the
state until it ends (GLM-5.3's reordered tool results). The DeepSeek V4.1 and GLM-4.5..5.3 views
and `views/agents/session.tsx` are written this way. `TurnStrategy` keeps each record's turn, by
record.

## 6. Effects: the harness API for external actions

The runtime (`spec/HARNESS.md`, `docs/RUNTIME.md`) runs this loop for you: model calls, exact-id
records, messages, waits, machines, resume. Use what follows only for a custom host.

A handler that needs the outside world awaits a harness request. The host serves requests by
**API name** (`machine:<role>`, `send`, `store.*`), so it never matches tool names:

```tsx
<aff label="shell" tool_name="shell" ... onRun={async ({ command }) => {
  const r = await harness.machine.call({cmd: command});
  return {result: `${r.output}\n[exit ${r.exit}]`};
}}>{"[shell]"}</aff>
<aff label="message" tool_name="send_message" ... onSend={async ({ to, text }) => {
  const r = await harness.send(to, {text: text});
  return {result: `delivered to ${to}`};
}}>{"[send_message]"}</aff>
```

Host loop for one tool call (`name`, `args` parsed from the reply):

1. `let step = view.act_effects(&enc, state, now, name, 0, &args, &harness, transcript)?;`
   This renders `state`, finds the handler by tool name, and drains its queued work until it needs external
   input. `step.effects` is the transcript (`Request::Call { app, args }`, where `app` is
   `machine:` (the default machine), `machine:<role>`, `send`, `store.put`, ..., plus an `outcome`
   once served) and `step.blocked` lists the pending indices.
2. Serve each blocked request, on any thread or task. Nothing is held between steps.
3. When a result arrives, set `transcript[i].outcome = Some(Outcome::Ok(value))` (or
   `Outcome::Error(msg)`). Set `transcript[i].settled` to an increasing arrival counter, then call
   `act_effects` again. It reconstructs one execution and delivers those outcomes in arrival order
   through the same queue as the live Agent, either parking or finishing (`done == true`).
4. When `done` becomes true, `step.value` is the handler's return value: the tool result.
   Continue serving `blocked` requests if it started unawaited work. Append
   `{"kind": "action", "name": name, "result": ...}`.

`Promise.all([...])` issues several requests at once (spec §7.2). A changed request on replay is a
divergence error. Missing settlement counters use input order. Use `runtime::Agent` for long-lived
execution: it retains frames and only outstanding requests between inputs. The standalone
transcript API retains no execution between calls.

`tests/harness_consumer.rs::harness_api_effects_with_a_slow_external_call` runs this for two
agents. A machine command sleeps 300 ms on a worker thread while a `send` completes at once. The
send is committed first, the command's result later, and the next render shows each result as a
tool turn.

## 7. Errors

Render errors keep the message as the first line, then add the view stack, innermost first, and
the log record being rendered:

```
a fragment cannot be indexed: this slot expects a string or a list, got a fragment (did you use p`...`? use a plain `...` string, or a template that accepts fragments)
    at first_output (/tmp/t.tsx:4)
    at ToolTurn (/tmp/t.tsx:7)
    while rendering TurnStrategy log[2] (kind "action")
```

`affordance::error_message(&err)` returns the first part only. `affordance check` compares messages with
it. Malformed records fail in `TurnStrategy` with their index. Examples: a missing `kind`, `ids`
that are not token ids, a map as `content`/`result`, or exact model ids sent to a template app
that does not support them.

## Rendering chat requests in a model server

A model server that applies a Jinja chat template and then tokenizes can call the engine
instead: `affordance::chat_render::ChatRenderer` takes a request's messages, tools and template
arguments and returns the prompt's token ids, through the model's template view.

```rust
use affordance::chat_render::{ChatRenderer, Options};
let renderer = ChatRenderer::json(Options::new("views/qwen/qwen35.tsx", "Qwen36", "tokenizer.json"))?;
let ids = renderer.render(&messages, tools.as_ref(), &serde_json::json!({"add_generation_prompt": true}))?;
```

It keeps what it rendered, by conversation, so a request that extends an earlier one costs what
is new; it owns its rendering threads and shares one loaded tokenizer. A server with its own
message type passes a function from a message to the JSON a template reads
(`ChatRenderer::new`), which runs only for messages it has not seen. A request the official
template would reject comes back as `RenderError::Raised` with the template's message.
