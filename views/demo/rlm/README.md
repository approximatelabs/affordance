# Recursive Language Model

A Recursive Language Model, after Prime Intellect's [RLM](https://www.primeintellect.ai/blog/rlm),
built as an instance tree. The root model never reads the input. The input sits in a persistent
Python REPL on the agent's machine, as `context`, and the model works on it with code through one
tool, `python`.

Code in the REPL can call sub-LLMs: `llm_batch(prompts)` runs them in parallel, and
`llm_query(prompt)` runs one. The model answers by setting `answer["content"]` and then
`answer["ready"] = True`. It sees what a cell prints, capped at 8192 characters.

- `rlm.tsx`
  - `RLM`: the agent.
  - `Direct`: the baseline, which reads the whole context in its prompt.
  - The default export, `Bench`: mounts both per task and grades their answers into `@results`.
- `tools/examples/rlm/rlm_repl` (development checkout only): the REPL, a daemon per machine home (`start FILE`, `exec`, `resume`, `stop`).
  It is an optional Python helper, excluded from the framework package, and makes no model calls.
- `tools/fixtures/make_rlm_tasks.py`: Oolong-style aggregation tasks. Each is a log of user messages whose category
  (complaint, praise, question, request) is never written down, and each question aggregates over
  those categories (how many complaints, which user praised most, how many questions from one
  user). Answering means reading every line, not finding one.

## How sub-LLM calls work

`llm_batch` parks the running cell, and the `exec` call returns the prompts. The `python` handler
then:

1. answers every prompt at once with `Promise.all` over `model(<Sub prompt=.../>)`;
2. sends the replies back with `resume`;
3. loops until the cell finishes.

So sub-LLM calls are the instance's own model calls. They are in its journal and under its budget,
and on resume they replay instead of being called again. The REPL never sees a key or a model
endpoint.

## Run

```sh
python3 tools/fixtures/make_rlm_tasks.py 11 3000 > tasks.json   # 3,000 messages, ~307K chars
```

Its deployment is [affordance.yaml](affordance.yaml): put the tasks in `root.props.tasks`, set the
provider and export its key as `API_KEY`, then `affordance run views/demo/rlm`.

`env.PATH` must be absolute, because a local machine runs commands in the agent's home. The
results are in `.affordance/root/snapshot.json` (`state.__ui["@results"]`). The REPL daemons outlive
the run, since they belong to the machines; stop them with `HOME=<home> rlm_repl stop`.

The offline test is `a_recursive_lm_works_on_its_context_in_a_repl_and_batches_sub_llm_calls` in
`tests/runtime.rs`.
