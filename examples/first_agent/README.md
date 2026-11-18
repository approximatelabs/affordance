# A first agent

An agent that reads its task, may run shell commands on this machine, and stops when it answers.
`app.tsx` is the whole app; `affordance.yaml` binds it to a model server and a machine.
`affordance init` writes these same two files into a new directory.

## Run it

1. Start an OpenAI-compatible model server on port 8080. No key is needed:

   ```sh
   llama-server -m your-model.gguf --port 8080 --jinja   # llama.cpp
   # or: vllm serve <model> --port 8080 --served-model-name local --enable-auto-tool-choice --tool-call-parser hermes
   # or: ollama serve (then add --set model.url=http://127.0.0.1:11434/v1 --set model.model=<name>)
   ```

   Set `context` in `affordance.yaml` to your model's context length (the server's `-c`).

2. Run the agent from this directory:

   ```sh
   affordance run
   ```

The report prints when the agent stops: `"stopped": {"answer": "..."}`. It stops when it calls
`answer`, or when it replies with no tool call (that reply is then the answer). Its journal and trace
are in `.affordance/first/`; `affordance run --resume` continues the run, `--fresh` starts over.

For a hosted API, set `url`, `model` and `context`, and `key_env: NAME` for the environment
variable that holds its key. `affordance config` shows what a run will use.

`tests/first_agent.rs` runs this example, as written, against a stand-in server.
