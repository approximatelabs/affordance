# Context policies example

A small fleet on the instance runtime. Each agent answers a question by
searching a document corpus through `bash`, on its own machine, under a context policy:

The budget is the model binding's `context` (the window is it less `max_new`). Dropping and
clipping are layout (`agent.tss`); summaries react to the layout event when a prompt passes `trig`
of the window.

| Policy | Rung | What happens |
|---|---|---|
| `none` | R0 | Nothing: the run ends when the context is full. |
| `trunc` | R1 | Past 85% of the window, the oldest whole turns drop, in steps, down to about half (`history` rule). |
| `compact` | R2 | One model call writes a handoff summary (`model(<Handoff/>)`); it replaces the turns. |
| `tree` | R4 | Each fold writes a leaf summary; three leaves merge into one; the `recall` tool reads a past turn in full. |
| `fold` | R5 | Context folding (FoldAgent): `branch` opens a side task, `return` folds its turns away to one summary; over budget in a branch, the agent is told to return. |
| `lazy` | R3 | Any policy: `bcp_search` and `bcp_get_document` are native tools, loaded with `load_tool`. |

- `agent.tsx`: one agent. Props: `question`, `policy`, `lazy`, `trig`, `template` (`messages` for a hosted
  chat API, `qwen36`, `glm53` for a local server), `onDone`.
- `ladder.tsx`: the root of an experiment. It mounts one agent per (question, policy), each on its own
  machine, grades each answer against `expect` (`onDone`), and records runs that stop without one (`onStop`).
  Its state is the results table.
- `tools/examples/search/` (development checkout only): `bcp_search` and `bcp_get_document` over a local directory (`BCP_CORPUS`, BM25). The demo uses
  this repository's `docs/`.
The full BrowseComp evaluation, retrieval server, and judge live in the separate
Affordance research project. This example uses the local documentation corpus.

Its deployment is [affordance.yaml](affordance.yaml): set the provider and export its key as
`API_KEY`, then `affordance run views/demo/research`.

Paths in `env` are absolute: a local machine runs commands in the agent's home. The results table
is the root's state (`.affordance/root/snapshot.json`).
The policies come from `views/strategies/context.tsx`. The Recursive LM (R7) is its own demo,
`views/demo/rlm/`; it does not use the `rlm` library the window-prototype rung ran (see git history before
window.rs was retired).
