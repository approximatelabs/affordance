# Permanent memory (after OptMem)

An agent whose memory outlives its sessions, after VictorTaelin's
[OptMem](https://github.com/VictorTaelin/OptMem), built as an instance tree.

- `memory.tsx` is the root. It owns the memory: an append-only log of one-line memories (`@log`)
  and a binary tree of one-line summaries over it (`@tree`, where block `lo-hi` summarizes its two
  halves). It mounts one session at a time and moves to the next when a session stops.
- `session.tsx` (`Session`) is one session. It wakes with the memory's cover and works on its task
  with four tools:
  - `note` records a memory (through the root's `onNote` callback);
  - `recall` is a regex search over every memory, word for word;
  - `zoom` opens a summary into its two halves;
  - `answer` ends the session.

The cover is `cover` in `views/strategies/context.tsx`, OptMem's tiling. Recent memories stay
verbatim and older ones collapse into tree nodes, in at most `wake_lines` lines. A block is
compressed by asking for one line of at most N bytes that keeps what will still matter later and
adds nothing.

Where it differs from OptMem:

- **No naps.** When a note completes a block of the tree, the root compresses it at once with a
  model call (`model(<Compress/>)`), smallest block first. A session never stops its
  work to summarize.
- **Write access is structural.** A session writes memory only through the `onNote` callback its
  parent passes it. A helper the session mounts gets no `onNote`, so it can read what it is given
  but cannot write.
- **It is durable.** The memory is the root's journaled state, so a crash restores the whole tree
  (root and current session) from the journals.
- **A session wakes once.** `wake` is fixed when the session mounts (`useState(wake)`). Notes made
  during the session change the root's memory, not the session's first message.

Props: `sessions` (one task per session), `template` (`messages` for a hosted chat API, `glm53`),
`wake_lines` (default 96), `chars` (the maximum length of a memory or summary, default 280).

Run it on a hosted model. Four sessions on DeepSeek V4.1 Flash took 27 turns and about 40K prompt
tokens, under $0.01. The root needs a model binding (`root.model`) for its compressions.

Its deployment is [affordance.yaml](affordance.yaml): set the provider's `url` and `model`,
export its key as `API_KEY`, and run `affordance run views/demo/memory`.

The memory ends in `.affordance/root/snapshot.json` (`state.__ui`), and each session's wake is in
`.affordance/root/session-<n>/snapshot.json`. The offline test is
`a_memory_outlives_its_sessions_compresses_old_notes_and_recalls_them` in `tests/runtime.rs`.
