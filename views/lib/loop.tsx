import { unanswered } from "./log.tsx";
// When the model acts is app code: the runtime never calls the model
// on its own. These are two common policies, as hooks over a log (`views/lib/log.tsx`): an app
// makes its log with `useLog()` and passes it. Ordinary TSX: copy them and change them.

// An autonomous agent (a `while (true)` loop): ask the model about the page, run its calls,
// repeat, each step one short task. `before` runs first in each step. Fitting the model is
// layout's, not the loop's: the window (the binding's `context`) and the sheet's history cap
// keep every prompt in it, and an app that summarizes reacts to the layout event
// (useEvent("layout")). A step that fails for good stops the instance with that error
// (`loop_error`), not leaving it idle. A reply's calls run one after another (`dispatch`:
// `{order: "parallel"}` runs them at once, `{stop_on_error: true}` skips the rest after one fails).
//
// A reply with no tool call ends the loop: asking again would send the same page and get the same
// reply until the turn limit. The instance stops with the reply as its answer (`{answer: text}`,
// or `{reason: "cut_off", text}` when the reply ran out of tokens). An app that wants to go on
// passes `idle(out)`: it changes the page (a nudge, a wait for a message) and returns true to ask
// again, as `views/agents/session.tsx` does.
export function useLoop(log, options) {
  const before = options ? options.before : undefined;
  const idle = options ? options.idle : undefined;
  const [step, setStep] = useState(0, (options && options.name) || "loop");
  useEffect(() => {
    const run = async () => {
      try {
        if (before) await before();
        const out = await log.call();
        await log.dispatch(out, options && options.dispatch);
        if (out.calls.length === 0 && !(idle && await idle(out))) {
          harness.stop(out.cut_off ? {reason: "cut_off", text: out.text} : {answer: out.text});
          return;
        }
      } catch (e) {
        harness.stop({reason: "loop_error", error: e.message});
        return;
      }
      setStep(s => s + 1);
    };
    run();
  }, [step]);
}

// A conversation: the model answers whenever the last record is unanswered (an input, or a tool
// result it has not seen), and is quiet after it speaks. Records added while it answers are
// written after its answer and answered together, next.
export function useConversation(log, options) {
  const t = log.records;
  const due = unanswered(t);
  const [step, setStep] = useState(0, (options && options.name) || "conversation");
  useEffect(() => {
    if (!due) return;
    const answer = async () => {
      try {
        await log.step();
      } catch (e) {
        harness.stop({reason: "conversation_error", error: e.message});
        return;
      }
      setStep(s => s + 1);
    };
    answer();
  }, [due, step, t.length]);
}
