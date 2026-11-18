# Getting started

You need Rust 1.89 or later. It runs on Linux and macOS; see
[platform evidence](DEVELOPING.md#current-platform-evidence). Python, Node and model weights are
not needed.

## Install

```sh
cargo install affordance
```

This installs one command, `affordance`. Everything below uses it; you do not write Rust to build
an app.

Or run it without installing anything, with [uv](https://docs.astral.sh/uv/) (Python's package
runner). Put `uvx` in front of each `affordance` command below:

```sh
uvx affordance init my-agent
```

The example apps are in the repository:

```sh
git clone https://github.com/approximatelabs/affordance && cd affordance
```

## Render an app

```sh
echo '{"name":"world"}' | affordance render views/demo/hello.tsx --state -
```

It prints `Hello, world! [greet]` and its spans. The greeting span marks text; the `greet` span
marks a visible action and the handler it runs. `--json` prints `ids` and `spans` for another
program. With no `--tokenizer`, `render` uses its bundled default (GPT-2), enough for an app that
writes no model-specific chat format.

The app is [views/demo/hello.tsx](../views/demo/hello.tsx):

```tsx
export default function Hello({name = "world"}) {
  return <box>
    <span label="greeting">{`Hello, ${name}!`}</span>
    <aff name="greet" onGreet={({name}) => ({name})}>{" [greet]"}</aff>
  </box>;
}
```

This is Affordance's TypeScript/JSX subset, interpreted in Rust (not React or Node). A render
evaluates the app against its state, then layout decides its exact token IDs and the actions that
survive. An action's handler changes the state; the next render shows it. See
[the language](../spec/LANGUAGE.md).

## Your first agent

```sh
affordance init my-agent
cd my-agent
```

`init` writes these files:

- `app.tsx`: the agent. It shows the model its task and the log of what it has seen and done,
  offers a `shell` tool and an `answer` tool, and loops until the model answers.
- `affordance.yaml`: where it runs. A model server (any OpenAI-compatible server on
  `127.0.0.1:8080`), a machine (`home`: this machine's shell) and the agent with its task.
- `.gitignore`: the run's journals (`.affordance/`) and local settings (`affordance.local.yaml`).
- `affordance.d.ts` and `tsconfig.json`: types for your editor (what `harness`, `model()`, the hooks
  and the elements are), so it completes and checks the app. They describe the platform; nothing
  runs them.

Start a model server. No key is needed for a local one:

```sh
llama-server -m your-model.gguf --port 8080 --jinja   # llama.cpp
# or: ollama serve      (then use --set model.url=http://127.0.0.1:11434/v1 --set model.model=<name>)
# or: vllm serve <model> --port 8080 --served-model-name local --enable-auto-tool-choice --tool-call-parser hermes
```

Then run the agent:

```sh
affordance run
```

`affordance run` finds `affordance.yaml` in the current directory (or a parent), runs the agent
and prints a report when it stops: `"stopped": {"answer": "..."}`.

The agent's tools reach the model as tool definitions, so the server must parse tool calls. That
is what `--jinja` (llama.cpp) and `--tool-call-parser` (vLLM, SGLang; the parser's name depends on
the model) turn on. If the server returns a tool call as plain text, `affordance run` says so. If
the server is not reachable, `affordance run` says so and keeps trying for five minutes
(`model.retry.for_s`). Each agent's journal and trace
are in `.affordance/<agent>/`. The journal records every input the agent received, so
`affordance run --resume` continues the run exactly; `affordance run --fresh` starts over.

### Change a setting

- For one run: `affordance run --set model.url=http://other:8080/v1 --set model.context=32768`.
  A key is a dotted path; a value is YAML.
- For this machine only: put it in `affordance.local.yaml` beside `affordance.yaml` (it is not
  committed). It layers over the main file.
- From the environment: write `${VAR}` or `${VAR:-default}` in a value.
- To see what the run will use: `affordance config` prints the resolved deployment, secrets hidden.

A hosted API takes its key from an environment variable (read at each call, never written to
the journal):

```yaml
model:
  kind: chat
  url: https://api.example.com/v1
  model: some-model
  context: 128000
  key_env: EXAMPLE_API_KEY
```

See [model bindings](MODEL_BINDINGS.md) for SGLang and vLLM token-id serving, Anthropic and
OpenAI Responses, and [running agents](RUNTIME.md) for the full deployment format.

## The library

An app imports the bundled TSX library with `@affordance/` paths:

```tsx
import { useLog } from "@affordance/lib/log.tsx";
import { ModelContext } from "@affordance/lib/model.tsx";
```

`@affordance/` names the library as an import map names a package: the copy bundled with the
`affordance` command, or a directory the deployment names with `library: path/to/views`. Your own
files import each other by relative path.

## Next

- **Write apps:** [running agents](RUNTIME.md), [the language and layout](../spec/LANGUAGE.md),
  and the apps in [`views/demo/`](../views/demo).
- **Add HTTP or MCP:** [Fetch and MCP](HTTP_AND_MCP.md).
- **Embed the engine in a Rust program** (not needed to build apps): the
  [harness guide](HARNESS_GUIDE.md) and [examples/embedding/hello.rs](../examples/embedding/hello.rs)
  (`cargo run --release --example hello -- TOKENIZER.json`).
- **Contribute:** [contributing](../CONTRIBUTING.md) and [development](DEVELOPING.md).
