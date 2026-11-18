// Context policies: what an agent keeps of its own transcript when the prompt outgrows its budget.
// Every policy reads and writes one fold record, and `project` turns the transcript plus that record
// into the log the model sees (a pure function: the same transcript and fold give the same ids).
//
//   fold = {start, summary, nodes}
//     start    turns before it are gone from the prompt (covered by the summary or the nodes)
//     summary  one handoff summary of turns [0, start)            (compact)
//     nodes    a tree of summaries [{lo, hi, level, text}]          (tree)
//
// A turn is a model record and everything after it up to the next one. Dropping turns to fit is
// layout's (a `history` rule); these policies run when the app reacts to the layout event
// (useEvent("layout")). One that needs a model call takes `summarize(text)`, which the app writes
// as a model call on its own template (model(<View/>)), so this file knows no template. Policies:
//   compact      one handoff summary replaces the turns (it folds in the previous summary)
//   summaryTree  each fold writes a leaf; every three leaves merge into one parent; `recall` reads
//                any past turn in full
//   branches     context folding (FoldAgent): a `branch` tool opens a side task, `return` closes it
//                with a summary, and `project` folds the finished branch away: only its call and
//                its summary stay. A branch that outgrows the budget is told to return
//                (BRANCH_OVERFLOW).
// And for a memory that outgrows any context (OptMem's tree): `cover`.

export const FOLD = {start: 0, summary: "", nodes: []};

export const HANDOFF = "You are performing a CONTEXT CHECKPOINT COMPACTION. Create a handoff summary for another LLM that will resume the task.\n\nInclude:\n- Current progress and key decisions made\n- Important context and constraints\n- What remains to be done (clear next steps)\n- Any critical data needed to continue (exact ids, paths, queries already tried, candidate answers with the evidence for and against them)\n\nBe concise, structured, and focused on helping the next LLM seamlessly continue the work.";

const DROPPED = "[Earlier turns of this session were dropped to fit the context window. Redo any lookup you still need.]";
const SUMMARY_PREFIX = "Another language model started this task and produced a summary of its work so far. Use it to build on the work already done and avoid repeating it. Here is that summary:\n\n";
const TREE_PREFIX = "Summaries of your earlier work follow, oldest first. Each is labelled with the turns it covers. Use the `recall` tool with a turn number to read any earlier turn in full.\n\n";

// The index of each turn's model record in the transcript.
export function turnStarts(t) {
  const starts = [];
  for (const [i, r] of t.entries()) { if (r.kind == "model") starts.push(i); }
  return starts;
}

// One turn as plain text (what a summary or a recall reads).
export function turnText(t, k) {
  const starts = turnStarts(t);
  if (k < 0 || k >= starts.length) return "";
  const end = k + 1 < starts.length ? starts[k + 1] : t.length;
  let s = "";
  for (const r of t.slice(starts[k], end)) {
    if (r.kind == "model") {
      s += "ASSISTANT: " + (r.content || "");
      for (const c of r.calls || []) s += `\n[${c.function.name}] ${JSON.stringify(c.function.arguments)}`;
      s += "\n";
    } else if (r.kind == "action") s += "TOOL OUTPUT: " + r.result + "\n";
    else if (r.kind == "input") s += "USER: " + r.content + "\n";
  }
  return s;
}

export function turnsText(t, lo, hi) {
  let s = "";
  for (let k = lo; k < hi; k++) s += `[turn ${k}]\n` + turnText(t, k) + "\n";
  return s;
}

function nodesText(nodes) {
  return nodes.map(n => `[turns ${n.lo} to ${n.hi - 1}]\n${n.text}`).join("\n\n");
}

// Branches in the transcript: a tool result returned as openBranch(..) opens one, returnBranch(..)
// closes the open one. `done`: [{open, ret, desc, summary}] (record indices); `open`: the open one.
export function branches(t) {
  const done = [];
  let open = null;
  for (const [i, r] of t.entries()) {
    if (r.kind == "action" && r.branch == "open" && open == null) open = {open: i, desc: r.desc};
    if (r.kind == "action" && r.branch == "return" && open != null) {
      done.push({open: open.open, ret: i, desc: open.desc, summary: r.summary});
      open = null;
    }
  }
  return {done, open};
}

// Whether record `i` is folded away by the finished branch `b`. A branch is folded by whole
// replies: the other results of the reply that opened it stay with that reply, and the other
// results of the reply that returned from it go with that reply. Folding by index alone would
// leave a reply showing two calls and one result, or a result with no call, which a hosted API
// refuses and a template renders wrongly. It depends only on the records up to `i`.
export function foldedBy(t, b, i) {
  let from = b.open + 1;
  while (from < t.length && t[from].kind == "action") from++;
  let to = b.ret;
  while (to + 1 < t.length && to + 1 <= i && t[to + 1].kind == "action") to++;
  return i >= from && i <= to;
}

// A branch tool's result: `{result, branch: "open", desc}`; the return tool's: `{.., "return", summary}`.
export function openBranch(desc) {
  return {result: `Branch started: ${desc}. Work on this sub task only; call return with what you found or did.`, branch: "open", desc};
}
export function returnBranch(summary) {
  return {result: "Returned from branch.", branch: "return", summary};
}

// The log the model sees: `head` (the task, as input records), what the fold says was removed, then
// the kept turns as they happened (exact ids where the records have them), with every finished
// branch folded to its opening call and one input holding its summary.
export function project(t, fold, head) {
  const starts = turnStarts(t);
  const out = [...head];
  if (fold.nodes.length > 0) out.push({kind: "input", content: TREE_PREFIX + nodesText(fold.nodes)});
  else if (fold.summary) out.push({kind: "input", content: SUMMARY_PREFIX + fold.summary});
  else if (fold.start > 0) out.push({kind: "input", content: DROPPED});
  const from = fold.start < starts.length ? starts[fold.start] : t.length;
  const done = branches(t).done;
  for (let i = from; i < t.length; i++) {
    const b = done.find(b => foldedBy(t, b, i));
    if (b == undefined) out.push(t[i]);
    else if (i == b.ret) out.push({kind: "input", content: `[branch finished: ${b.desc}]\n${b.summary}`});
  }
  return out;
}

// What a branch that outgrew the budget is told (FoldAgent's): return now, with a summary.
export const BRANCH_OVERFLOW = "The context limit has been exceeded for this branch. Finish the sub task directly and clearly state the progress made and what is still pending. Only summarize the sub task progress, using the return tool.";

// R1 is layout: `history { max-tokens: 85win; overflow: drop-start; drop-step: 41%; }` (the oldest
// whole turns drop, in steps; see views/demo/research/agent.tss).

// R2: one handoff summary of everything so far (the previous summary folded in) replaces the turns.
export async function compact(t, fold, summarize) {
  const now = turnStarts(t).length;
  const before = fold.summary ? SUMMARY_PREFIX + fold.summary + "\n\n" : "";
  const text = await summarize(before + turnsText(t, fold.start, now));
  return {...fold, summary: text.trim(), start: now};
}

// R4: the turns since the last fold become a leaf summary; three leaves merge into one parent.
export async function summaryTree(t, fold, summarize) {
  const now = turnStarts(t).length;
  const leaf = await summarize(turnsText(t, fold.start, now));
  let nodes = [...fold.nodes, {lo: fold.start, hi: now, level: 0, text: leaf.trim()}];
  const leaves = nodes.filter(n => n.level == 0);
  if (leaves.length >= 3) {
    const merged = await summarize(nodesText(leaves));
    nodes = [...nodes.filter(n => n.level != 0), {lo: leaves[0].lo, hi: leaves.at(-1).hi, level: 1, text: merged.trim()}];
  }
  return {...fold, nodes, start: now};
}

// A `recall` tool's answer: turn k in full (a sheet's `result[name="recall"]` caps it in tokens).
export function recall(t, k) {
  const n = turnStarts(t).length;
  if (!(k >= 0 && k < n)) return `No turn ${k} (turns run from 0 to ${n - 1}).`;
  return turnText(t, k);
}

// OptMem's cover: tile memories [0, T) with aligned power-of-two blocks, at most `budget` of them,
// finest near T, so recent memories stay verbatim and old ones collapse into tree nodes. A block
// [lo, hi) is kept whole when its size is at most alpha times its age; alpha is bisected to the
// budget, and what is left of it is spent splitting the newest coarse blocks.
function tile(T, alpha) {
  let root = 1;
  while (root < T) root *= 2;
  return split(0, root, T, alpha);
}

// The blocks of [lo, hi) in order: whole when small enough for its age, else its two halves.
function split(lo, hi, T, alpha) {
  if (lo >= T) return [];
  const size = hi - lo;
  if (size > 1 && (hi > T || size > alpha * (T - lo))) {
    const mid = Math.trunc((lo + hi) / 2);
    return [...split(lo, mid, T, alpha), ...split(mid, hi, T, alpha)];
  }
  return [[lo, hi]];
}

export function cover(T, budget) {
  if (T <= 0) return [];
  if (T <= budget) { const all = []; for (let i = 0; i < T; i++) all.push([i, i + 1]); return all; }
  let lo = 0;
  let hi = 1;
  for (let i = 0; i < 60; i++) {
    const mid = (lo + hi) / 2;
    if (tile(T, mid).length > budget) lo = mid; else hi = mid;
  }
  let out = tile(T, hi);
  while (out.length < budget) {
    let i = -1;
    for (const [j, b] of out.entries()) { if (b[1] - b[0] > 1) i = j; }
    if (i < 0) break;
    const [a, b] = out[i];
    const mid = Math.trunc((a + b) / 2);
    out = [...out.slice(0, i), [a, mid], [mid, b], ...out.slice(i + 1)];
  }
  return out;
}

// The blocks of the binary tree over [0, T) that can exist (both halves complete) and are not in
// `have` (a map "lo-hi" -> text), smallest first: what a memory still has to summarize.
export function pendingBlocks(T, have) {
  const todo = [];
  for (let size = 2; size <= T; size *= 2) {
    for (let k = 0; (k + 1) * size <= T; k++) {
      const key = `${k * size}-${(k + 1) * size}`;
      if (have[key] === undefined) todo.push([k * size, (k + 1) * size]);
    }
  }
  return todo;
}
