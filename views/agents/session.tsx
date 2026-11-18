"use safe_text"; // a peer's message or a tool's output is text, never a control token
import { useLog, recordText } from "../lib/log.tsx";
// A long-session agent as one view file: the app owns its screen, its tools, its state
// and its context policy (folding included). The runtime supplies primitives (exec, send,
// store, wait, measure, model, machine); its log is its own state (views/lib/log.tsx); nothing
// app-specific lives in Rust. A summary is a model call on the agent's own context plus a
// maintenance request (model(<SessionContext/>)), on its own model.
//
// Context tiers, most stable first: T0 system + fixed tools (template head), T1 pinned identity
// and task, T2 epoch block (summaries, notes/todo at the fold, finished branches; changes only at
// folds), T3 folded region (records before fold.boundary at a reduced level), T4 verbatim window
// (exact ids, append-only), T5 the screen. Fold levels: 1 masks long tool outputs to a pointer
// (errors keep their tail), 2 also drops reasoning and old screens. Records before
// fold.summary_upto are covered by an epoch summary; once summarized they are released
// (log.release): the log keeps a skeleton of each (RELEASE_KEEP, what the branch and turn
// bookkeeping below reads), and read_event / search_history read them back in full (log.read).
import "./session.tss";
import { useLoop } from "../lib/loop.tsx";
import { foldedBy } from "../strategies/context.tsx";
import { ModelContext } from "../lib/model.tsx";
import { Tool } from "../lib/tool.tsx";
import { machineTools } from "../tools/machine.tsx";

const RELEASE_KEEP = ["kind", "name", "branch", "desc", "summary", "calls"];
const SYSTEM = "You are a long-running autonomous agent. Work step by step, keep notes and a todo list current, and use branches for self-contained side investigations.";

// The agent's prompt: template, system, intro and the projected log, then the screen.
function SessionContext({ intro, log, children, callable = true }) {
  const first = memoScan([intro], null, (s, content) => [null, {kind: "input", content}], null).outs;
  const all = [...first, ...log];
  return <ModelContext system={SYSTEM} strategy="unrolled" log={all} callable={callable}>{children}</ModelContext>;
}

// What a summary render asks, after the agent's own context: the raw records it covers are
// quoted, so the summary describes exactly that range even when the context shows it reduced.
function maintenance(from, to, text, notes, todo) {
  return `[Context maintenance, not a task step] Older events ${from}..${to} are about to be removed from your context. ` +
    "Write the summary you will rely on afterwards: the task, what was done and found (exact names, numbers, paths, error codes), " +
    "what failed, and what is still open. Do not call tools; reply with the summary only.\n\n" +
    `Your notes: ${notes}\nYour todo: ${todo}\n\nEvents ${from}..${to}:\n${text}`;
}

// The passes over the log are scans (memoScan): a record's result depends only on the
// record, its index, the state from the records before it and the scan's deps, so each render
// computes what is new, not the whole log.

// Branches from the log: an `action` marked branch=open, closed by one marked branch=return.
function branch_step(b, r, i) {
  b = b ?? {done: [], open: null};
  if (r.kind == "action" && r.branch == "open") return [{done: b.done, open: {open: i, desc: r.desc}}, null];
  if (r.kind == "action" && r.branch == "return" && b.open) {
    return [{done: [...b.done, {open: b.open.open, ret: i, desc: b.open.desc, summary: r.summary}], open: null}, null];
  }
  return [b, null];
}

function branches(t) {
  return memoScan(t, null, branch_step, null).state ?? {done: [], open: null};
}

// Derived from the log, not stored: unread messages, the text-only streak, whether the last reply was cut off.
function activity_step(s, r) {
  s = s ?? {unread: 0, streak: 0, cut_off: false};
  if (r.kind == "screen") return [{...s, unread: 0}, null];
  if (r.kind == "input") return [{...s, unread: s.unread + 1}, null];
  if (r.kind == "model") return [{...s, streak: r.calls.length ? 0 : s.streak + 1, cut_off: r.cut_off}, null];
  return [s, null];
}

function placeholder(r) {
  let s = `[output elided: ${r.full_tokens} tokens from ${r.name}, exit ${r.exit}. read_event("${r.blob || r.id}") to view]`;
  if (r.exit != 0 && r.err_tail) s += `\nlast lines:\n${r.err_tail}`;
  return s;
}

function epochBlock(fold, done) {
  let s = `[Context folded (epoch ${fold.epoch}). Older turns are shortened; use search_history or read_event to recall exact text.]\n`;
  for (const sm of fold.summaries) s += `\n## Summary of events ${sm.from}..${sm.to}\n${sm.text}\n`;
  if (done.length) {
    s += "\n## Finished branches\n";
    for (const b of done) s += `- ${b.desc}: ${b.summary.slice(0, 160)}\n`;
  }
  return s + `\n## Your notes (at fold)\n${fold.notes}\n\n## Your todo (at fold)\n${fold.todo}`;
}

// Record `i` as the model sees it at fold level `lvl` (none, one record or a branch's summary).
function selectOne(t, r, i, lvl, summary_upto, done) {
  if (i < summary_upto) return [];
  const hidden = done.find(br => foldedBy(t, br, i));
  if (hidden) {
    // A finished branch collapses to its call; its summary follows as one input.
    return i == hidden.ret ? [{kind: "input", content: `[branch finished: ${hidden.desc}]\n${hidden.summary}`}] : [];
  }
  if (r.kind == "screen") return lvl < 2 ? [r] : [];
  if (r.kind == "model") return [lvl >= 2 ? {...r, reasoning: "", provider_state: null, ids: null} : r];
  if (r.kind == "action") return [lvl >= 1 && (r.full_tokens || 0) > 64 ? {...r, result: placeholder(r), ids: null} : r];
  return [r];
}

// The records the model sees, from the full log, the fold state and its branches. Pure:
// the same log and fold give the same ids, so every non-fold turn is an exact append.
// Two scans: the folded region (a fold probe with a later boundary reuses it up to the
// current one) and the verbatim window after it.
export function selectLog(t, fold, b) {
  const {summary_upto, boundary, level} = fold;
  const head = memoScan(t.slice(0, boundary), null, (s, r, i) => [null, selectOne(t, r, i, level, summary_upto, b.done)], [summary_upto, level, b.done]);
  const tail = memoScan(t.slice(boundary), null, (s, r, j) => [null, selectOne(t, r, boundary + j, 0, summary_upto, b.done)], [summary_upto, boundary, b.done]);
  const epoch = memoScan(fold.epoch > 0 ? [epochBlock(fold, b.done)] : [], null,
    (s, content) => [null, {kind: "input", content}], null).outs;
  return [...epoch, ...head.outs.flat(), ...tail.outs.flat()];
}

// Index of the model record `k` model turns back (the start of the verbatim window).
function recentBoundary(t, k) {
  const models = [];
  for (const [i, r] of t.entries()) { if (r.kind == "model") models.push(i); }
  return models.length <= k ? 0 : models[models.length - k];
}

function Screen({ handle, budget, used, epoch, todo_open, unread, in_branch, warn, nudge, act, has_vm, messaging = true }) {
  return <>
    <box label="status">{`${handle} | context ${used}/${budget} tokens | epoch ${epoch} | todo open: ${todo_open} | unread: ${unread}${in_branch ? " | IN BRANCH" : ""}\n`}</box>
    {warn ? <box label="warn">{"Context is near its limit: update your notes and todo now; older turns will be shortened soon.\n"}</box> : ""}
    {nudge ? <box label="nudge">{nudge + "\n"}</box> : ""}
    {has_vm ? <Tool label="shell" name="shell" description="Run a shell command on the bound machine, with a 60-second foreground limit. Long output is truncated with a read_event pointer."
         schema={{type: "object", properties: {command: {type: "string"}}, required: ["command"]}} fn={act.shell}>{"[shell]\n"}</Tool> : ""}
    <Tool label="event" name="read_event" description="Read the full text of a past event (e.g. an elided tool output) by id."
         schema={{type: "object", properties: {id: {type: "string"}, offset: {type: "integer"}, limit: {type: "integer"}}, required: ["id"]}} fn={act.read}>{"[read_event]\n"}</Tool>
    <Tool label="history" name="search_history" description="Search your whole session history (including folded parts) for text. Returns event ids and snippets."
         schema={{type: "object", properties: {query: {type: "string"}}, required: ["query"]}} fn={act.search}>{"[search_history]\n"}</Tool>
    <Tool label="notes" name="set_notes" description="Replace your notes (facts to keep across context folds)."
         schema={{type: "object", properties: {text: {type: "string"}}, required: ["text"]}} fn={act.notes}>{"[notes]\n"}</Tool>
    <Tool label="todo" name="set_todo" description="Replace your todo list (one item per line, prefix done items with [x])."
         schema={{type: "object", properties: {items: {type: "string"}}, required: ["items"]}} fn={act.todo}>{"[todo]\n"}</Tool>
    <Tool label="branch" name="branch" description="Start a side branch for a self-contained subtask; its steps are removed from context when you return."
         schema={{type: "object", properties: {description: {type: "string"}}, required: ["description"]}} fn={act.branch}>{"[branch]\n"}</Tool>
    <Tool label="branch_return" name="branch_return" description="Finish the current branch and return a summary of what it found or did."
         schema={{type: "object", properties: {summary: {type: "string"}}, required: ["summary"]}} fn={act.ret}>{"[return]\n"}</Tool>
    {messaging ? <Tool label="message" name="send_message" description="Send a direct message to another agent by id (e.g. agent-0042). It arrives in their context before their next turn, marked with your id."
         schema={{type: "object", properties: {to: {type: "string"}, text: {type: "string"}}, required: ["to", "text"]}} fn={act.send}>{"[send_message]\n"}</Tool> : ""}
    <Tool label="wait" name="wait" description="Pause until a message arrives or the given number of seconds passes. Use this when you have nothing useful to do."
         schema={{type: "object", properties: {seconds: {type: "integer"}}, required: ["seconds"]}} fn={act.wait}>{"[wait]\n"}</Tool>
    <aff event="message" onCallback={act.message}/>
  </>;
}

// The agent. `shell` runs on the machine bound to it (harness.machine); with none bound it has
// no shell. Called as a plain function, so its hook state keeps the same keys as when it was the entry.
// `messaging` false (a best-of-N fleet) leaves out send_message. `instructions` supplies host
// and fleet facts from the deployment. `summary`: the summary render's model options (`{max_new, sampling}`, e.g. a lower temperature);
// absent, the agent's own.
export function SessionAgent({ id, task, instructions = "", c_hi = 48000, keep_recent = 3, messaging = true, summary = {} }) {
  const identity = id;
  const hasVm = harness.machines.length > 0;
  const [notes, setNotes] = useState("(empty)", "notes");
  const [todo, setTodo] = useState("(empty)", "todo");
  const [fold, setFold] = useState({epoch: 0, boundary: 0, level: 0, summary_upto: 0, summaries: [], notes: "", todo: ""}, "fold");
  const log = useLog();
  const t = log.records;
  const f = fold;
  const b = branches(t);

  const {unread, streak, cut_off} = memoScan(t, null, activity_step, null).state ?? {unread: 0, streak: 0, cut_off: false};
  // A reply cut off by the token budget did not choose to skip acting: it ran out of room.
  const nudge = cut_off ? "Your last reply hit the token limit before it finished, so no tool call was made. Think more briefly and act sooner, or split the work into smaller steps."
    : streak == 0 ? "" : streak < 3
    ? `Your last reply had no tool call. No human reads plain replies: act with tools, ${messaging ? "talk to agents with send_message, " : ""}or call wait if there is nothing useful to do.`
    : `${streak} replies in a row without a tool call. If you are blocked or done, call wait(seconds) so you are woken when a message arrives.`;
  const todo_open = todo.split("\n").filter(l => l.trim() && !l.trim().startsWith("[x]")).length;
  const intro = `You are ${identity}. This id is permanent.\n` +
    (hasVm ? "- `shell` runs on your bound machine.\n" : "- No shell is bound.\n") +
    (messaging ? "- `send_message(to, text)` delivers a direct message; replies arrive marked with the sender's id.\n" : "") +
    (instructions ? instructions + "\n" : "") + `\nTask: ${task}`;
  const shell = hasVm ? machineTools(harness.machine.get(harness.machines[0])).shell : null;
  const screen = (act) => <Screen label="session" handle={identity} budget={c_hi} used={harness.prompt_tokens} epoch={f.epoch} todo_open={todo_open} unread={unread}
                                  in_branch={b.open != null} warn={harness.prompt_tokens > 0.85 * c_hi} nudge={nudge} act={act} has_vm={hasVm} messaging={messaging}/>;

  const act = {
    // The full output is the record; layout shows its head and tail (session.tss), read_event the rest.
    shell: async ({ command }) => {
      const r = await shell({command, timeout_s: 60});
      let out = r.result;
      if (r.timed_out) out += "\n[timed out; run long jobs in the background, e.g. nohup cmd > log 2>&1 &]";
      const full_tokens = countTokens(out);
      // The last lines, read from the end: splitting a whole large output would copy it into lines.
      const err_tail = r.exit != 0 ? out.slice(-4000).split("\n").slice(-5).join("\n") : "";
      return {result: out, full_tokens, exit: r.exit, timed_out: r.timed_out, seconds: r.seconds, err_tail};
    },
    read: async ({ id, offset = 0, limit = 4000 }) => {
      let text = null;
      if (id.startsWith("b")) {
        const r = await harness.store.get({key: id});
        if (!r.missing) text = r.text;
      } else {
        const record = await log.get(id);
        if (record) text = recordText(record);
      }
      if (text == null) return `no event ${id}`;
      const end = Math.min(offset + limit, text.length);
      const start = Math.min(offset, end);
      return `event ${id}, chars ${start}..${end} of ${text.length}:\n${text.slice(start, end)}`;
    },
    search: async ({ query }) => {
      const q = query.toLowerCase();
      const hits = [];
      // Released records are read back in full, a chunk at a time.
      for (let lo = 0; lo < t.length; lo += 500) {
        const hi = Math.min(lo + 500, t.length);
        const rs = t.slice(lo, hi).some(r => r.released) ? await log.read(lo, hi) : t.slice(lo, hi);
        for (const [j, r] of rs.entries()) {
          const text = recordText(r);
          const at = text.toLowerCase().indexOf(q);
          if (at >= 0) hits.push(`${r.id} [${r.kind}${r.name ? " " + r.name : ""}]: ...${text.slice(Math.max(0, at - 80), at + q.length + 80).replaceAll("\n", " ")}...`);
        }
      }
      const blobs = await harness.store.search({query, limit: 5});
      for (const h of blobs.hits.filter(h => !h.key.startsWith("log:"))) hits.push(`${h.key} [stored output]: ...${h.snippet.replaceAll("\n", " ")}...`);
      if (!hits.length) return `no matches for ${JSON.stringify(query)}`;
      return `${hits.length} matching events (showing up to 5, oldest first):\n${hits.slice(0, 5).join("\n")}`;
    },
    notes: ({ text }) => { setNotes(text); return `notes saved (${text.length} chars)`; },
    todo: ({ items }) => { setTodo(items); return `todo now:\n${items}`; },
    branch: ({ description }) => b.open
      ? "error: already in a branch"
      : {result: `Branch started: ${description}. Call branch_return with a summary when done.`, branch: "open", desc: description},
    ret: ({ summary }) => b.open
      ? {result: "Returned from branch.", branch: "return", summary}
      : "error: not in a branch",
    send: async ({ to, text }) => {
      const r = await harness.send(to, {text});
      return r.delivered ? `delivered to ${to}` : `error: no agent named ${to}`;
    },
    wait: async ({ seconds }) => {
      const r = await harness.wait({ms: Math.min(Math.max(seconds, 0), 1800) * 1000, events: ["message"]});
      return r.woke == "timeout" ? `waited ${seconds}s; no messages` : "woken by a new message";
    },
    // Messages arrive between turns, stamped by the runtime; the app shows them as input.
    message: async ({ from, text }) => {
      log.append({kind: "input", content: `[message from ${from}] ${text}`});
    },
  };

  // The fold policy, from layout facts (useEvent("layout"), after each prompt is laid out): while
  // a prompt passes c_hi, take one step per turn over the records before the recent window
  // (all but the last keep_recent model turns): mask long tool outputs, then drop reasoning and
  // old screens, then summarize them into the epoch block and release them. Each step shows in
  // the next prompt's layout; nothing measures candidate renders. The window itself is layout's
  // (session.tss): no prompt exceeds it.
  const [folding, setFolding] = useState(false, "folding");
  useEvent("layout", async ({ prompt_tokens }) => {
    if (folding || prompt_tokens <= c_hi) return;
    const boundary = Math.max(recentBoundary(t, keep_recent), fold.boundary);
    const next = {...fold, epoch: fold.epoch + 1, notes, todo, boundary};
    if (fold.level < 2) {
      setFold({...next, level: fold.level + 1, last: {before: prompt_tokens, summarized: false}});
      return;
    }
    if (boundary <= fold.summary_upto) return;
    setFolding(true);
    try {
      await summarize(boundary, next, prompt_tokens);
    } finally {
      setFolding(false);
    }
  });
  const summarize = async (boundary, next, prompt_tokens) => {
    let text = "";
    // Quoted, bounded: each record clipped, the whole capped (the context above shows them all).
    for (const r of t.slice(fold.summary_upto, boundary)) {
      if (text.length > 24000) { text += "(later events in this range are shown above)\n"; break; }
      text += `${r.kind}: ${recordText(r).slice(0, 300)}\n`;
    }
    // The summary is this agent's own context plus one maintenance request, run on its model.
    const s = await model(<SessionContext callable={false} intro={intro} log={selectLog(t, f, b)}>
      {screen(act)}<box label="maintenance">{maintenance(fold.summary_upto, boundary, text, notes, todo)}</box>
    </SessionContext>, {...summary, tag: "summary", priority: true});
    const summaries = [...next.summaries, {from: fold.summary_upto, to: boundary, text: s.text}].slice(-6);
    setFold({...next, summaries, summary_upto: boundary, last: {before: prompt_tokens, summarized: true}});
    // What the summaries now cover leaves the prompt for good: release it.
    await log.release(boundary, RELEASE_KEEP);
  };

  // The agent's loop: ask, act, repeat.
  // A reply with no call gets a nudge on the next page (above); the session goes on.
  useLoop(log, {idle: () => true});
  return <SessionContext intro={intro} log={selectLog(t, f, b)}>{screen(act)}</SessionContext>;
}

export default function Main(props) {
  return SessionAgent(props);
}
