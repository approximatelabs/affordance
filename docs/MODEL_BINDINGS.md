# Configuring model bindings

The configuration contract is in [HARNESS.md §6](../spec/HARNESS.md#6-models).
A binding combines a tokenizer and one stock client. `model` names the default binding;
`models` adds names selected by `agents[].model` or `<Mount model="B">`. Every client has its
own endpoint, options, credentials, defaults and session table. Identical tokenizer
paths share an immutable encoder in the CLI.

## Template selection

A native binding names its model's format as `"template"`, in one of two ways:

- **A registered model's name**, as on Hugging Face: `"template":"moonshotai/Kimi-K2-Instruct"`.
  Any model in `views/index.json` can be named. Its prompt is the publisher's own chat template,
  and what an agent needs to read its replies was measured from that template
  (`docs/ADDING_A_MODEL.md` says how a model gets into the index). `template_options` are passed
  to the template as its own variables (`enable_thinking`, and so on).

- **A template app's key** from `views/template_apps/index.json`: `"template":"glm53"`, with
  `"template_options":{"reasoning_effort":"high"}` (each app's options are declared in the
  registry; another is refused). These nine are written by hand for the formats we tune.

No model-name heuristics select a format.

**How a past reply is written.** Whichever way the format is named, a past reply is written
the way the publisher's template writes one. Some templates write it differently from what the
model generated: they drop its thinking, change its last token, or move the tool list. That is
the publisher's decision about what the model reads, and it is the default, with the template's
own switches at their own defaults (`preserve_thinking` off for Qwen3.6, `drop_thinking` on for
DeepSeek V4.1, `clear_thinking` off for GLM-5.3). Each switch is a template option. Where a
template writes a past reply as exactly the text the model generated, the reply's own tokens
are kept, so the prompt extends the last one and the server reuses its work.
`"template_options":{"keep_replies":true}` keeps every reply's tokens always, for a deployment
that prefers that reuse to the publisher's rewriting. A logged reply therefore needs what it
said (its `content`, `reasoning` and `calls`) as well as its `ids`; the records the loop writes
have all of them. Hosted chat, Anthropic and Responses default
to and require `messages`. Options must be an object and cannot replace app-owned `system`
or `children`; other props follow the selected template's semantics. Use the matching
tokenizer: an absent required special token is a render error before a request is sent.

An app can then omit model names:

```tsx
import { ModelContext } from "@affordance/lib/model.tsx";
import { useLog } from "@affordance/lib/log.tsx";
import { useLoop } from "@affordance/lib/loop.tsx";

export default function Main() {
  const log = useLog();
  useLoop(log);
  return <ModelContext system="Help with the task." log={log.records} strategy="unrolled">
    Ready.
  </ModelContext>;
}
```

`harness.model` exposes only the binding's name, template key and template options. It is null
for a coordinator without a model. Explicitly imported template apps remain useful for
standalone renderers and custom compositions. Native bindings without a configured template
can still use them; `ModelContext` requires explicit configuration.

Presentation is pinned with the binding; strict recovery rejects changed template/options.
Template source changes use the existing app source pin. Supply host instructions through
app props, for example the bundled session's `instructions`, rather than shared-code paths.

## The window

Every prompt's window is the binding's `context` (the model's context length) less the call's
`max_new`, as a page's viewport is the screen it is shown on. The model server knows it: a native
binding (SGLang, completions) asks once, when the runtime validates it, `GET /v1/models` (the
entry named `name`; for SGLang, which names no model, the only entry; its `max_model_len` or
`context_length`), then SGLang's `GET /get_server_info` (`context_length`). A server still
starting is asked again twice. So a native binding needs no `context`:

- `"context": 65536` only lowers the window: more than the server reports fails validation,
  with both numbers. A server that cannot be asked leaves the configured value (one warning line).
- `"context": "none"` runs without a window, by choice: no request, no layout cap, no pre-send
  check.
- With neither, a server that reports no context length fails validation, naming the binding and
  each endpoint tried.

A `chat` binding with no `context` asks its server the same way (`GET {url}/models`): a self-hosted
OpenAI-compatible server (vLLM, SGLang) reports its window there. Hosted APIs (and the Anthropic
and Responses bindings) do not report one: such a binding needs `context` (or `"none"`), and the
error says so. The runtime logs each binding's window on stderr
(`model binding "default": window 262144 tokens (server /v1/models)`), and the run's status
(`Control::status`) shows it under `models` (`{"default": {"context": 262144, "source": "server /v1/models"}}`).

## Provider options and sampling

`provider_options` is copied directly to the top level of the outgoing provider JSON object.
For native SGLang, use `{"provider_options":{"lora_path":"variant-b"}}`. The adapter must
already be available on that server; this configuration does not load model weights.
SGLang's compatible OpenAI APIs instead select LoRAs through their model selector, for example
completions `"name":"base-model:adapter-name"`. See [SGLang's serving contract](https://docs.sglang.io/docs/advanced_features/lora).

Binding `sampling` supplies defaults. The model call's `sampling` overrides those defaults
one field at a time. SGLang puts these in `sampling_params`; completions and chat put them at
the top level. Nested objects are replaced, not recursively merged.

Provider options are fixed for a binding. A sampling key cannot also be a provider option;
model/adapter identity, authentication and host-controlled request fields cannot be sampling
keys. Select another configured binding to use another model or LoRA. Do not put an endpoint,
credential, prompt, stop rule, output limit, stream control, session or request ID in provider
options. Such conflicts fail before sending a request.

The framework rejects unknown configuration keys, invalid types and protected fields. It
forwards provider extensions unchanged; the server validates extension names and semantics.
There is no universal schema for every provider version. A misspelling inside an opaque
provider extension can only be rejected if that server rejects it. Use wire-fixture tests
when adopting a provider extension whose acceptance matters. SGLang `lora_path` is locally
checked as a nonempty string or null because each request contains one prompt.

## Credentials and transport

`key_file` and `key_name` select a binding-local bearer credential. Both are required together;
any binding can omit both for a server that takes no key (a local llama.cpp, Ollama or vLLM).
`key_env: NAME` reads the key from an environment variable instead. The file contains `NAME=value` lines,
optionally prefixed with `export`; single/double quotes around a value are supported, without
shell expansion. A key must be nonempty printable ASCII without whitespace. Values are read
at request time so the file can be rotated in place.
A hosted binding sends the key in its API's own header (`Authorization: Bearer`, Anthropic's
`x-api-key`) unless `key_header` names another credential header (`api-key` for Azure OpenAI,
`x-goog-api-key`); the non-secret `headers` cannot carry one.

Credentials go only in Authorization headers, including SGLang session and abort requests.
Captures redact that header; provider error messages and diagnostic metadata also redact
echoed credential values. They are absent from argv, model-visible options and persisted
binding identity. Provider options are ordinary request data and may be captured; use the
credential fields for secrets.

Chat bindings additionally accept `"headers": {"HTTP-Referer":"https://example.test/", "X-Title":"My app"}`.
These are fixed non-secret strings. Names and values are validated; duplicate names ignoring
case, Authorization and Content-Type are rejected. They are sent as request headers and are
visible in request captures. Keep secrets in the binding's
credential (`key_file` and `key_name`, or `key_env`), never in `headers`. Changing headers changes
binding identity.

Chat requests and responses use the shared atomic capture writer when `AFFORDANCE_CAPTURE_DIR` is
set. Provider usage is preserved under `model_meta.usage`, including provider cost fields,
without price conversion or enforcement. Local input/output counts and provider cached-token
counts remain in per-request logs. Missing usage does not invalidate an otherwise valid reply.
There is no cumulative budget; `max_new` still bounds a single request's output. Money and
account limits belong to the embedding host. Remove `price_in`, `price_out` and `budget_usd`
from older configurations; strict validation rejects them.

Every binding uses Affordance's one HTTP client (`src/runtime/http.rs`), the one `fetch` uses
too, including server-sent events and cancellation. URLs take `http://` or `https://`. Over
`https://` the server's certificate must chain to a root the operating system trusts or to the
Mozilla roots; one that fails is a protocol error, not retried. Proxies come from the environment
as curl reads them (`HTTPS_PROXY`, `http_proxy`, `ALL_PROXY`, `NO_PROXY`), and loopback addresses
never use one. Chat takes a single choice; IPv6 literal hosts are supported there.
`request_timeout_s` bounds the whole request, from connecting to the reply's last byte, for
native non-streaming calls and every hosted call (streamed or not); native SSE uses `stall_s` per
event instead. Fractional seconds are supported. The client bounds what it reads: a 64 MiB body
(or SSE event), a response head of 100 header fields, the first 8 KiB of a failure's body (which
only goes into the error), and 1 MiB read ahead of the caller. A reply over a limit is a protocol
error, not retried.
Base URLs cannot contain credentials, queries or fragments. Paths such as `/api/v1` are valid.
Do not include the final `/generate`, `/v1/completions` or `/chat/completions` suffix.

## Describing tools without allowing calls

`ModelContext callable={false}` (or `TurnStrategy callable={false}`) retains tool definitions
and removes callable handler references. `MessagesApp` carries that restriction to hosted
clients. After merging binding/provider/per-call settings, chat and Responses send
`tool_choice:"none"`; Anthropic sends `tool_choice:{"type":"none"}`. A forced-tool default
cannot override a passive render. Without definitions, the tool-choice field is omitted;
legacy function options are removed from passive requests. Active requests keep their usual
settings and framing.

If the provider nevertheless returns a tool call, including a truncated call, the adapter
returns a non-transient protocol error rather than silently producing an empty summary or
accepting an invocation. Ordinary prose quoting a tool is still text. Disabling tool calls
does not guarantee nonempty prose: applications that require a summary must validate its text.

Tool and system definitions remain identical. Anthropic documents that changing tool_choice
invalidates the messages-cache tier while retaining tools/system caches; identical descriptions
do not guarantee a full history cache hit. See the provider
[caching rules](https://platform.claude.com/docs/en/build-with-claude/prompt-caching).

## Anthropic Messages

`"kind":"anthropic"` is a hosted binding for `POST {url}/v1/messages`. Like chat, it renders
through and requires the `messages` template, reads its key at request time, streams when a frontend
or a pipe wants the reply as it is generated, and writes the reply back as
`{reasoning, content, tool_calls}`.

| Field | Contract/default |
|---|---|
| `model` | Required provider model id, e.g. `claude-opus-5-5`. |
| `url` | Optional; default `https://api.anthropic.com`. HTTP(S), without the `/v1/messages` suffix. |
| `key_file`, `key_name` | Required. The key is sent as `x-api-key` and redacted like a bearer key. |
| `headers` | Optional fixed non-secret headers, e.g. `anthropic-beta`. `x-api-key`, `anthropic-version`, `Authorization` and `Content-Type` are client-owned; the client sends `anthropic-version: 2023-06-01`. |
| `cache_ttl` | `"5m"` (default) or `"1h"`: the TTL of the binding's cache breakpoints. |
| `sampling`, `provider_options`, `request_timeout_s`, `template`, `stream` | As for chat. `thinking`, `output_config`, `tool_choice`, `metadata` and other request fields go in `provider_options`. |

The client maps `MessagesApp`'s turns: the leading system turn becomes the top-level `system`
(a later system turn is user text); tools become `{name, description, input_schema}`; an
assistant turn uses its original native content blocks when available, including signed/redacted thinking and tool-use IDs; without native state it becomes `text` and `tool_use` blocks; tool results become `tool_result` blocks
in the following user message, paired with their calls by `tool_call_id` (or position), with
image parts as image blocks (base64 sources). Empty
text is omitted and consecutive same-role messages are merged, as the API requires. `max_new`
is `max_tokens`; `stop_reason: "max_tokens"` sets the reply's cut-off flag.

The binding places up to three explicit `cache_control` breakpoints: the last tool,
the system prompt and the newest cacheable history block before the live screen. Thinking
blocks are never marked directly. Cache reads depend on the provider's minimum prefix length
and actual prefix equality. `useLog` retains each hosted screen's sent text/image content;
it no longer becomes blank on the next turn. A fold deliberately starts a different prefix.

Usage is preserved under `model_meta.usage`, including `input_tokens`,
`cache_creation_input_tokens`, `cache_read_input_tokens` and `output_tokens`. Additive aliases
are `prompt_tokens` (all three input counts), `prompt_tokens_details.cached_tokens` (reads),
`prompt_tokens_details.cache_write_tokens` (creation) and `completion_tokens`. When reported,
thinking-token details also receive a `completion_tokens_details.reasoning_tokens` alias. No prices are
calculated. `cached_tokens` in request logs reports the cache read.

Thinking text is visible as `reasoning`; the original content blocks are also retained in
opaque `provider_state`. `useLog` and `ModelContext` carry this field unchanged into later
requests, including signatures, redacted blocks and original tool IDs. A lossy tokenizer is
rejected rather than corrupting the signed text. Adaptive thinking may skip thinking on any
turn; it does not require a thinking block on every response. Legacy manual thinking requires
native continuation state for the last assistant tool turn.

Rewriting history can invalidate a provider-bound thinking prefix. The app must explicitly
clear `provider_state` when discarding or rewriting an assistant record; it must not send old
opaque text alongside a different visible message. The example session clears it when dropping
old reasoning. For models/accounts that require a prefix policy across folds, fixed binding
options can explicitly request dropping mismatched thinking blocks, as below. Affordance does
not silently strip valid thinking during ordinary turns.

```json
"models": {
  "claude": {
    "kind": "anthropic", "model": "claude-opus-5-5",
    "tokenizer": "ref/tokenizers/zai-org__GLM-5.3.json",
    "key_file": "/run/secrets/anthropic.env", "key_name": "ANTHROPIC_API_KEY",
    "headers": {"anthropic-beta": "thinking-binding-controls-2026-08-01"},
    "provider_options": {
      "thinking": {"type": "adaptive", "block_binding": {"prefix_mismatch_behavior": "drop_block"}},
      "output_config": {"effort": "high"}
    },
    "cache_ttl": "5m", "request_timeout_s": 900
  }
}
```

Models with budgeted thinking (for example Haiku 4.5) take
`"thinking": {"type": "enabled", "budget_tokens": 2048}` instead, below `max_new`.
`tests/hosted_client.rs` exercises the real TSX app/log/dispatch path, including checkpoint
and full replay. The small live check uses Haiku 5.5 with adaptive thinking:

```bash
AFFORDANCE_ANTHROPIC_SMOKE=1 cargo test --release --test hosted_client live_native_sessions -- --nocapture
```

`AFFORDANCE_ANTHROPIC_KEY_FILE` names a file with an `ANTHROPIC_API_KEY=` line. It requires cache reads from turn two and exercises native reasoning.

## OpenAI Responses

`"kind":"responses"` calls `POST {url}/responses` directly. The default base URL is
`https://api.openai.com/v1`. It uses the same file-only bearer credentials, validated non-secret
headers, captures, cancellation and synchronous transport as chat. Its template is `messages`.
Thinking/reasoning wire settings belong in provider_options or per-call sampling, rather than
MessagesApp template props; presentation and wire configuration remain separate.
The provider API owns model-specific option validation; unsupported sampling settings produce
an explicit provider error rather than being silently removed.

```json
"models": {
  "gpt": {
    "kind": "responses", "model": "gpt-6.1-sol",
    "tokenizer": "ref/tokenizers/zai-org__GLM-5.3.json",
    "key_file": "/run/secrets/openai.env", "key_name": "OPENAI_API_KEY",
    "provider_options": {
      "reasoning": {"effort": "high", "context": "all_turns"},
      "text": {"verbosity": "low"},
      "prompt_cache_options": {"ttl": "30m"}
    },
    "strict_tools": false, "request_timeout_s": 600
  }
}
```

System messages become developer input; user content becomes `input_text`/`input_image`.
Assistant native output items are replayed intact, including encrypted reasoning and function
calls. Function results cite the provider's **call_id**, not the function-call item ID. Without
native state, visible assistant text/calls are converted to message/function-call items. Tool
text and images travel together inside function_call_output content, including parallel results. Function tools retain their schemas;
`strict_tools` is a boolean, default false. Setting it true asks the provider for strict schema
validation; the binding does not rewrite optional fields or expand the schema.

The runtime supplies `max_output_tokens` from `max_new`, `store:false` and encrypted-reasoning
inclusion. These transport-owned fields cannot be overridden by provider options. A response
with `status:incomplete` and reason `max_output_tokens` sets `cut_off`; partial calls are not
dispatched or replayed as unanswered native calls. Valid reasoning state remains available. HTTP-200 error objects, failed responses and other incomplete states are errors.

`provider_options` accepts `reasoning`, `text`, `parallel_tool_calls`, `service_tier`, cache
options and other provider extensions. A stable default `prompt_cache_key` hashes binding
identity plus agent ID. It contains no credential path in clear text. An explicit nonempty
key of at most 64 bytes may be supplied in provider options. Keep it stable across turns.
Current models use `prompt_cache_options.ttl`; see the official
[caching contract](https://developers.openai.com/api/docs/guides/prompt-caching).

Usage retains the provider's fields and adds `prompt_tokens`, `completion_tokens`,
`prompt_tokens_details` and `completion_tokens_details` aliases for input/output usage and
cache/reasoning details. Provider usage is authoritative; local tokenizer counts measure the
transport document, including opaque state, and are not provider billing counts.

A hosted client makes one attempt at a call. A failed call is made again by the agent, as the
binding's `retry` policy says (`spec/HARNESS.md`, "Model call failures and retries"). A valid
numeric or HTTP-date `Retry-After` is the least the next attempt waits. On 429, OpenAI
reset-duration headers and Anthropic reset-date headers are also read; the longest valid minimum is used.
Waits are cancellable; `request_timeout_s` applies separately to each HTTP attempt, not the
provider-requested wait. Exhausted transient failures retain the runtime's existing retry
policy. Binding identity pins the model, its request options and its headers during strict recovery;
how the server is reached (address, credentials, timeouts) is not part of it.

```bash
AFFORDANCE_RESPONSES_SMOKE=1 cargo test --release --test hosted_client live_native_sessions -- --nocapture
```

This runs three GPT-6.1-sol app turns with reasoning/tools, checks cache reads from turn two,
and verifies checkpoint/full replay makes no new calls. `AFFORDANCE_RESPONSES_KEY_FILE` names a
file with an `OPENAI_API_KEY=` line. Provider reasoning rules are documented by
[OpenAI](https://developers.openai.com/api/docs/guides/reasoning) and
[Anthropic](https://platform.claude.com/docs/en/build-with-claude/thinking-tool-workflows).

## Sessions and recovery

A SGLang session belongs to one client identity. A changed endpoint, provider options,
credential source or binding defaults requires a fresh client or `Sessions` value. Per-call
sampling can vary. Two clients using the same server and agent ID still have separate sessions.
Server session loss retains the existing fallback to a full prompt; sockets and server sessions
are never serialized into a journal.

Persisted instances record a digest of their built-in model binding's replay identity alongside
the existing app/tokenizer pins: which model answers (kind, model, provider options such as a
LoRA, template, context: the window in effect, the same whether the server reported it or the
config stated it), not how it is reached (address, credentials, ranks, timeouts,
cache TTL) or sampled, so moving a server or changing a timeout restores. `"replay":"strict"`
refuses a changed or missing model identity before inference. Warning replay reports the
difference and preserves the journal's original pins. Existing journals without this identity
cannot acquire a strict guarantee retrospectively.
Keep the original model for strict recovery, or explicitly use warning replay when accepting a
changed/unverifiable binding. Weights swapped under the same model name are outside the guarantee.

Recorded replies replay without network I/O. Outstanding work may run again using the pinned
binding, following the existing at-least-once recovery contract. Provider-side replacement of
weights under the same model/adapter name is outside that guarantee: deploy versioned names
when model identity must be reproducible.

## Small deployment example

Create `bindings-app.tsx` at the repository root:

```tsx
import {ModelContext} from "./views/lib/model.tsx";

export default function Main({task}) {
  useEffect(() => {
    (async () => {
      const out = await model();
      harness.stop({reason: "complete", answer: out.text});
    })();
  }, []);
  return <ModelContext strategy="unrolled" log={[{kind: "input", content: task}]}>{task}</ModelContext>;
}
```

Here A uses the existing `default` name. A/B share a server with separate LoRAs and credentials;
C uses a compatible completions endpoint. All three servers in this example use the GLM
vocabulary/template; select matching binding templates and tokenizer files for other families.
The endpoint addresses and credential paths are supplied by the deployment operator.

```json
{
  "app": {"view": "bindings-app.tsx", "tokenizer": "ref/tokenizers/zai-org__GLM-5.3.json"},
  "model": {
    "kind": "sglang", "url": "http://127.0.0.1:30000",
    "template": "glm53", "template_options": {"reasoning_effort": "high"},
    "key_file": "/run/secrets/model-a.env", "key_name": "API_KEY",
    "sampling": {"temperature": 0.2},
    "provider_options": {"lora_path": "variant-a"}
  },
  "models": {
    "B": {
      "kind": "sglang", "url": "http://127.0.0.1:30000",
      "template": "glm53", "template_options": {"reasoning_effort": "high"},
      "tokenizer": "ref/tokenizers/zai-org__GLM-5.3.json",
      "key_file": "/run/secrets/model-b.env", "key_name": "API_KEY",
      "sampling": {"temperature": 0.7},
      "provider_options": {"lora_path": "variant-b"}
    },
    "C": {
      "kind": "completions", "url": "http://127.0.0.1:8000",
      "template": "glm53", "template_options": {"reasoning_effort": "high"},
      "name": "glm-base:variant-c",
      "tokenizer": "ref/tokenizers/zai-org__GLM-5.3.json",
      "key_file": "/run/secrets/model-c.env", "key_name": "API_KEY",
      "sampling": {"temperature": 0.1},
      "provider_options": {"logprobs": 1}
    }
  },
  "agents": [
    {"id": "one", "model": "default", "props": {"task": "Say hello."}},
    {"id": "two", "model": "B", "props": {"task": "Say hello."}},
    {"id": "three", "model": "C", "props": {"task": "Say hello."}}
  ],
  "dir": "runs/bindings", "replay": "strict"
}
```

Save it as `affordance.yaml` (JSON is valid YAML) and run `affordance run`; add `--restore` to
recover that run.
This example adds no machine bindings or tool execution.

## Rust embedding

`runtime::model::from_config(&json, encoder)` builds and validates the same stock adapters used
by the CLI, returning `Arc<dyn Model>`. Bind it with `Runtime::bind_model(name, encoder, client)`.
The caller supplies the encoder; `tokenizer` in deployment JSON is loaded by the CLI.

Direct constructors remain available: `Sglang::new`, `Completions::new`, `Chat::new`,
`Chat::anthropic` and `Chat::responses` (whose `api` fields hold protocol settings).
`model::with_template(client, ModelTemplate::new("glm53", options)?)` attaches presentation
to any client while delegating its inference, streaming, cancellation and rank behavior.
`Chat::new` advertises `messages` by default. Low-level `Agent` callers set
`AgentConfig.template` to the same resolved `ModelTemplate` on fresh execution and restore;
`Runtime` does this automatically for each binding, including mounted children.
Wrapping a custom client preserves its identity opt-in: template metadata does not establish
the identity of an otherwise unidentified inference implementation. Such clients must supply
`Model::binding_identity` to receive strict binding-configuration checks. Checkpoint cache
identity still includes the resolved `AgentConfig.template`.
Native clients expose `auth: Option<Credentials>` and all stock clients expose `provider_options`.
`Model::validate` runs before runtime workers start and before stock generation; mutating public
client settings does not bypass validation. Custom adapters can implement `binding_identity`
with non-secret, versioned configuration. Its default is absent: the runtime cannot certify
undeclared implementation identity for a custom adapter.
