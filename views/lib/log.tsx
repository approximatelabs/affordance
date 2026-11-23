// A log of turns: what a model has seen and said, as records, kept in one hook. The engine keeps
// no conversation: `model(view?, opts)` returns an output, and
// `dispatch(output)` runs its calls. This library turns those into the records a template app's
// turn strategy reads (`<TurnStrategy log={log.records}>`), the way a chat web app keeps its
// messages in state. It is ordinary TSX: copy it and change it.
//
// Records:
//   {kind: "input", content}                                        what the app adds (a message, a task)
//   {kind: "screen", ids?, images?, content?}                                  the page's current turn, as the model saw it
//   {kind: "model", ids?, reasoning, content, calls, problems, cut_off, provider_state?}   the reply, with its exact ids
//   {kind: "action", name, tool_call_id, result, error?, ...meta}  a call's result
//
// A turn is open from `call` until `dispatch` (or `close`) writes its results. Records appended
// meanwhile are held and written after them, so a prompt plus its reply is always a prefix of the
// next prompt (exact appends, prefix-cache reuse).

// `id` identifies a committed record; `ids` are its optional exact token IDs. Tool-call IDs
// remain `tool_call_id`. Extra metadata (exit status, branch policy, etc.) belongs to the app.
export type ContentPart = {type: string; text?: string; image_url?: {url: string}; [key: string]: unknown};
export type Content = string | ContentPart[] | null;
export type ToolCall = {type: "function"; id?: string; function: {name: string; arguments: unknown}};
export type RecordFields = {id?: string; ids?: number[] | null; released?: boolean; [key: string]: unknown};
export type InputRecord = RecordFields & {kind: "input"; content: Content};
export type ScreenRecord = RecordFields & {kind: "screen"; content?: Content; images?: {at: number; n: number; src: string}[]};
export type ModelRecord = RecordFields & {kind: "model"; reasoning?: string; provider_state?: string | null; content: Content; calls: ToolCall[]; problems?: unknown[]; cut_off?: boolean};
export type ActionRecord = RecordFields & {kind: "action"; name: string; tool_call_id?: string; result: Content; error?: boolean};
export type LogRecord = InputRecord | ScreenRecord | ModelRecord | ActionRecord | (RecordFields & {kind: "metric" | "event"});
export type ReleasedRecord = {kind: string; id: string; released: true};

// Content (a string, a list of text and image parts, or nothing) as plain text, with `[image]`
// where an image is. The one such function: frontends, template apps and searches use it.
export function contentText(content) {
  if (content == null) return "";
  if (typeof content === "string") return content;
  if (!Array.isArray(content)) return JSON.stringify(content);
  return content.map(p => typeof p === "string" ? p : p.type === "text" ? p.text || ""
    : p.type === "image" || p.type === "image_url" ? "[image]" : JSON.stringify(p)).join("\n");
}

// Whether the model owes an answer: the last record is an input, or a tool result it has not
// seen. While a turn is open nothing new is committed (inputs are held), so the records alone
// say it. The one statement of when a conversation's model speaks next.
export function unanswered(records) {
  const last = records.length ? records[records.length - 1] : null;
  return last != null && (last.kind === "input" || last.kind === "action");
}

// A diagnostic/search form, not prompt rendering. Read a released record before formatting it.
export function recordText(record: LogRecord): string {
  if (record.released) throw new Error("recordText: read the released record through log.read or log.get first");
  if (record.kind === "action") return contentText(record.result);
  if (record.kind === "input" || record.kind === "screen") return contentText(record.content);
  if (record.kind !== "model") return "";
  let text = record.reasoning ? `[reasoning] ${record.reasoning}\n` : "";
  text += contentText(record.content);
  for (const c of record.calls || []) text += `\n[call] ${c.function.name} ${JSON.stringify(c.function.arguments)}`;
  return text;
}

function recordId(key, i) { return `${key === "log" ? "" : key + ":"}e${i}`; }
function commit(key, existing, records) {
  return [...existing, ...records.map((r, i) => ({...r, id: recordId(key, existing.length + i)}))];
}

// The `screen` and `model` records of an output (IDs are assigned when committed).
export function turnRecords(out) {
  // Hosted screens retain the content actually sent; their JSON reply IDs are not native
  // assistant tokens. Keep one representation in the log, including opaque continuation state.
  const hosted = out.provider_state != null || (out.screen && out.screen.content !== undefined);
  const screen = !out.screen ? [] : out.screen.content !== undefined ? [{kind: "screen", content: out.screen.content}]
    : out.screen.images ? [{kind: "screen", ids: out.screen.ids, images: out.screen.images}] : [{kind: "screen", ids: out.screen.ids}];
  const calls = out.calls.map(c => ({type: "function", id: c.id, function: {name: c.name, arguments: c.args}}));
  const record = {kind: "model", ids: hosted ? null : out.ids, reasoning: out.reasoning, content: out.text, calls, problems: out.problems, cut_off: out.cut_off};
  if (out.provider_state != null) record.provider_state = out.provider_state;
  // Stopped by its signal: the content is what was generated before.
  if (out.aborted) record.aborted = true;
  return [...screen, record];
}

// A `dispatch` result as an `action` record.
export function actionRecord(r) {
  const {id, name, result, ...meta} = r;
  return {...meta, kind: "action", name, tool_call_id: id, result};
}

// The calls of an output that was not dispatched, answered `not run (why)`.
export function notRun(out, why) {
  return out.calls.map(c => ({kind: "action", name: c.name, tool_call_id: c.id, result: c.problem || `error: not run (${why})`, error: true}));
}

function skeleton(record, keep) {
  const s = {};
  for (const k of keep) if (record[k] !== undefined && k !== "ids") s[k] = record[k];
  s.released = true;
  s.id = record.id;
  return s;
}

function withoutIds(record) {
  const {ids, ...rest} = record;
  return rest;
}

export function useLog(name) {
  const key = name || "log";
  const [log, setLog] = useState({records: [], released: []}, key);
  // The open turn's bookkeeping (its phase, number and the records held for after it) is a ref:
  // it never renders, so a call writes nothing the page shows and the page need not be evaluated
  // again for it. Read in handlers only.
  const [book, setBook] = useRef({phase: "closed", turn: 0, held: []}, key + "~turn");

  // The log's current value inside a handler: `setLog` with a function sees writes made earlier
  // in the same handler; this is a read, not a write.
  const current = () => {
    let snapshot;
    setLog(l => { snapshot = l; return l; });
    return snapshot;
  };
  const releasedTo = l => l.released.reduce((n, b) => Math.max(n, b.to), 0);

  // Add records: now, or after the open turn's results.
  const append = (...records) => {
    if (book().phase !== "closed") setBook(b => ({...b, held: [...b.held, ...records]}));
    else setLog(l => ({...l, records: commit(key, l.records, records)}));
  };

  // Close `turn` with its records, then the ones held while it was open.
  const finish = (turn, records) => {
    const b = book();
    if (b.turn !== turn || b.phase === "closed") return;
    setLog(l => ({...l, records: commit(key, l.records, [...records, ...b.held])}));
    setBook({...b, phase: "closed", held: []});
  };
  const owner = out => {
    if (!out || !out.__logTurn || out.__logTurn.key !== key) throw new Error("InvalidStateError: output does not belong to this log");
    return out.__logTurn.turn;
  };
  const requireTurn = (turn, phase) => {
    const b = book();
    if (b.turn !== turn || b.phase !== phase) throw new Error("InvalidStateError: output does not own the current log turn");
  };

  // One resolved output owns each turn through dispatch/close. This local ticket
  // stays with the suspended call until the output resolves.
  const call = async (view, opts) => {
    const b = book();
    if (b.phase !== "closed") throw new Error("This log already has an open turn; dispatch or close it before calling again");
    const turn = b.turn + 1;
    setBook({...b, phase: "calling", turn});
    try {
      const out = await model(view, opts);
      requireTurn(turn, "calling");
      setLog(l => ({...l, records: commit(key, l.records, turnRecords(out))}));
      setBook(x => ({...x, phase: "ready"}));
      return {...out, __logTurn: {key, turn}};
    } catch (e) {
      finish(turn, []);
      throw e;
    }
  };

  // Run the output's calls and write their results. `order`: "sequential" (the default, as most
  // harnesses do: each call starts when the one before it has answered, so a call sees what the
  // ones before it did) or "parallel" (all at once). `stop_on_error`: in order, the calls after
  // one that failed are not run (answered `not run`). An app that wants another policy calls
  // `dispatch(out, {calls: [i]})` itself: each call is its own promise.
  const run = async (out, options) => {
    const order = (options && options.order) || "sequential";
    const stopOnError = !!(options && options.stop_on_error);
    if (order !== "sequential" && order !== "parallel") throw new TypeError(`dispatch order must be "sequential" or "parallel", not ${JSON.stringify(order)}`);
    const turn = owner(out);
    requireTurn(turn, "ready");
    setBook(x => ({...x, phase: "dispatching"}));
    try {
      let records;
      let results;
      if (order === "parallel") {
        results = await dispatch(out);
        records = results.map(actionRecord);
      } else {
        results = [];
        records = [];
        for (let i = 0; i < out.calls.length; i++) {
          if (stopOnError && results.some(r => r.error)) {
            records.push(notRun({calls: [out.calls[i]]}, "an earlier call failed")[0]);
            continue;
          }
          const [r] = await dispatch(out, {calls: [i]});
          results.push(r);
          records.push(actionRecord(r));
        }
      }
      finish(turn, records);
      return results;
    } catch (e) {
      finish(turn, notRun(out, e.message));
      throw e;
    }
  };

  // A pending request/dispatch must settle first. A repeated close of an older output is
  // harmless; it cannot release a newer turn or move that turn's held inputs.
  const close = (out, why) => {
    const b = book();
    if (b.phase === "closed") return;
    const turn = owner(out);
    if (turn !== b.turn) return;
    requireTurn(turn, "ready");
    finish(turn, notRun(out, why || "not dispatched"));
  };

  // One step: call, then dispatch (with `dispatchOptions`, as `dispatch` takes them).
  const step = async (view, opts, dispatchOptions) => {
    const out = await call(view, opts);
    const results = await run(out, dispatchOptions);
    return {out, results};
  };

  // Bound memory: records before `upto` become skeletons (the `keep` fields and `released: true`);
  // the full records (without token `ids`) go to the store, one batch a release. Record IDs stay.
  const release = async (upto, keep) => {
    if (!Number.isSafeInteger(upto) || upto < 0) throw new RangeError("release boundary must be a nonnegative safe integer");
    for (;;) {
      const snapshot = current();
      const from = releasedTo(snapshot);
      const to = Math.min(upto, snapshot.records.length);
      if (to <= from) return from;
      const put = await harness.store.put({key: `${key}:${from}-${to}`, text: JSON.stringify(snapshot.records.slice(from, to).map(withoutIds))});
      let committed = false;
      setLog(l => {
        if (releasedTo(l) !== from) return l;
        committed = true;
        return {...l, records: l.records.map((r, i) => i >= from && i < to ? skeleton(r, keep || ["kind"]) : r),
                released: [...l.released, {from, to, key: put.key}]};
      });
      if (committed) return to;
    }
  };

  const read = async (from, to) => {
    if (!Number.isSafeInteger(from) || !Number.isSafeInteger(to) || from < 0 || to < from) throw new RangeError("read requires a nonnegative ordered integer range");
    const snapshot = current();
    const rs = snapshot.records.slice(from, to);
    const batches = {};
    for (const b of snapshot.released) {
      if (b.to > from && b.from < to) batches[b.key] = JSON.parse((await harness.store.get({key: b.key})).text);
    }
    return rs.map((r, j) => {
      if (!r || !r.released) return r;
      const b = snapshot.released.find(x => x.from <= from + j && from + j < x.to);
      return batches[b.key][from + j - b.from];
    });
  };

  // Stable record IDs are commit indices; lookup needs no extra index or retained text copy.
  const get = async id => {
    const prefix = recordId(key, 0).slice(0, -1);
    if (typeof id !== "string" || !id.startsWith(prefix)) return null;
    const index = Number(id.slice(prefix.length));
    if (!Number.isSafeInteger(index) || index < 0 || recordId(key, index) !== id) return null;
    if (index >= current().records.length) return null;
    return (await read(index, index + 1))[0] || null;
  };

  // Replace the conversation: `records` become the whole log, with ids assigned anew. A turn that
  // is open no longer owns the log, so what it would have written is not committed.
  const reset = records => {
    setBook(b => ({phase: "closed", turn: b.turn + 1, held: []}));
    setLog({records: commit(key, [], records), released: []});
  };

  // `isOpen()` and `held()` read the turn's bookkeeping: in handlers only (it does not render).
  return {records: log.records, isOpen: () => book().phase !== "closed", held: () => book().held, append, call, dispatch: run, close, step, release, read, get, reset};
}
