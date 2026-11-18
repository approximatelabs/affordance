// A chat driven by a frontend (`affordance host`, or any embedder that posts events): the user's
// messages arrive as `user` events (`{text}`), and an `abort` event (Escape) stops the turn in
// progress. The model answers each message and runs its calls until it replies without one. An
// aborted turn keeps its partial answer in the log; the chat then waits for the next message.
//
// The frontend hears the chat on the machine `ui`, each a plain value it can draw:
//   {state: "working" | "idle"}       a turn started; the chat is waiting for the user
//   {reply: {text, reasoning, calls, aborted, cut_off}}   each model reply, when it ends
//   {calls: [{id, name, args}]}       the model's tool calls, as they start
//   {results: [{id, name, result, error, content?, details?}]}   their results
//   {history: [...]}                  the conversation so far, in answer to the event `history`
//   {error: message}                  a turn failed
// The events `reset` ({messages}: chat messages, as `model_layer.tsx` takes them) and `continue`
// let an embedder that keeps its own transcript replace the conversation (a branch) and resume it.
// A frontend that connects to a resumed run sends the event `history` to draw what came before:
// entries are {kind: "input", text}, {kind: "model", text, reasoning, calls, aborted} and
// {kind: "action", id, name, result, error}.
//
//   const log = useLog();
//   const turn = useFrontend(log);
//   const {shell} = machineTools(harness.machine.get("home"), {signal: turn.signal});
//
// `useFrontend(log, {before, call})`: `before` runs at the start of each turn, before the model is
// asked; `call()` gives more options for the turn's model call (`sampling`, `max_new`).
// `turn.signal()` is the current turn's AbortSignal (in handlers): a tool that passes it on stops
// when the turn is aborted. `ask(question)` asks the frontend and returns its answer (a
// permission, a choice); without a frontend it returns `fallback`.

import { chatRecords } from "./model_layer.tsx";
import { contentText, unanswered } from "./log.tsx";

/** Ask the frontend (the machine `ui`) and wait for its answer. */
export async function ask(question, fallback) {
  try {
    return await harness.machine.get("ui").call({ask: question});
  } catch (e) {
    return fallback;
  }
}

// Tell the frontend something; nothing waits on it, and without a frontend it is dropped.
async function tell(value) {
  try {
    await harness.machine.get("ui").call(value);
  } catch (e) {}
}


// A tool's result as the frontend is told it: its text, the raw content when that is not text (an
// image), and whatever else the tool returned beside `result` as `details`.
function told(r) {
  const {id, name, result, error, ...details} = r;
  const out = {id, name, result: contentText(result), error: !!error};
  if (typeof result !== "string" && result != null) out.content = result;
  if (Object.keys(details).length) out.details = details;
  return out;
}

// A log record as a frontend draws it; records that are not conversation (screens, released) are skipped.
function entry(record) {
  if (record.released) return null;
  if (record.kind === "input") return {kind: "input", text: contentText(record.content)};
  if (record.kind === "model") {
    return {kind: "model", text: contentText(record.content), reasoning: record.reasoning || "",
            calls: (record.calls || []).map(c => ({id: c.id, name: c.function.name, args: c.function.arguments})),
            aborted: !!record.aborted};
  }
  if (record.kind === "action") {
    return {kind: "action", id: record.tool_call_id, name: record.name, result: contentText(record.result), error: !!record.error};
  }
  return null;
}

export function useFrontend(log, options) {
  const name = (options && options.name) || "frontend";
  const before = options && options.before;
  const call = options && options.call;
  // The turn's controller, and `halted`: the chat waits for the user (a message, or `continue`)
  // before the model is asked again. It is set when a turn is stopped, when the conversation is
  // replaced, and when a turn fails, and cleared by the user's next move. A ref: it never renders.
  const [turn, setTurn] = useRef({controller: null, halted: false}, name + "~turn");
  const records = log.records;
  const [step, setStep] = useState(0, name);
  // A message is its `text`, or `content` (a string, or parts: text and images).
  useEvent("user", ({ text, content }) => {
    setTurn(t => ({...t, halted: false}));
    log.append({kind: "input", content: content === undefined ? text : content});
  });
  // The embedder replaces the conversation (a branch, an import): chat messages as the model layer
  // takes them. The chat then waits; `continue` answers what is unanswered.
  useEvent("reset", ({ messages }) => {
    const t = turn();
    if (t.controller) t.controller.abort();
    setTurn(x => ({...x, halted: true}));
    log.reset(chatRecords(messages || []));
  });
  useEvent("continue", () => {
    setTurn(t => ({...t, halted: false}));
    setStep(s => s + 1);
  });
  useEvent("abort", () => {
    const t = turn();
    if (t.controller) t.controller.abort();
    setTurn(x => ({...x, halted: true}));
  });
  useEvent("history", async () => {
    await tell({history: records.map(entry).filter(e => e !== null)});
  });
  const due = unanswered(records);
  useEffect(() => {
    if (!due) return;
    const answer = async () => {
      // One turn at a time: this effect also runs while a turn is telling the frontend something.
      if (turn().halted || turn().controller) return;
      const controller = new AbortController();
      setTurn(t => ({...t, controller}));
      await tell({state: "working"});
      let more = false;
      try {
        // The app's own step before the model is asked (folding the context, for one).
        if (before) await before();
        const out = await log.call(undefined, {...(call ? call() : {}), signal: controller.signal});
        const calls = out.calls.map(c => ({id: c.id, name: c.name, args: c.args}));
        await tell({reply: {text: out.text, reasoning: out.reasoning || "", calls, aborted: !!out.aborted, cut_off: out.cut_off}});
        if (controller.signal.aborted || turn().halted) {
          // The user stopped the turn as its reply arrived whole: its calls are not run. They are
          // answered "not run" in the log, so the conversation stays well formed.
          log.close(out, "the user stopped the turn");
        } else {
          if (calls.length) await tell({calls});
          const results = await log.dispatch(out);
          if (results.length) await tell({results: results.map(told)});
          more = out.calls.length > 0 && !turn().halted;
        }
      } catch (e) {
        // A turn that was stopped (Escape, or `reset` replacing the conversation under it) ends
        // quietly, whatever error its unwinding raised: it must not halt the chat that `continue`
        // has since started again.
        if (e.name !== "AbortError" && !controller.signal.aborted) {
          // A turn that failed is not tried again by itself: the chat waits for the user, who
          // sees why. The next message starts a new turn.
          setTurn(t => ({...t, halted: true}));
          await tell({error: e.message});
        }
      }
      setTurn(t => ({...t, controller: null}));
      if (!more) await tell({state: "idle"});
      setStep(s => s + 1);
    };
    answer();
  }, [due, step, records.length]);
  return {signal: () => turn().controller ? turn().controller.signal : undefined};
}
