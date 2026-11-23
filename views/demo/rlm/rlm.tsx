import { useLog } from "@affordance/lib/log.tsx";
// A Recursive Language Model (after Prime Intellect's RLM, primeintellect.ai/blog/rlm), as an
// instance tree. The root model never reads the input: it lives in a persistent Python REPL on the
// agent's machine as `context`, and the model works on it with code (`python`). Code can call
// sub-LLMs, `llm_batch(prompts)` (in parallel) or `llm_query(prompt)`, and it answers by setting
// `answer["content"]` and `answer["ready"] = True`. What the model sees of a cell is its output,
// capped at 8192 characters.
//
// The REPL (tools/rlm_repl, a daemon per machine home) makes no model calls: `llm_batch` parks the
// cell, the `python` handler answers every prompt with a model call on `Sub` (model(<Sub/>)) (all at once,
// Promise.all), and resumes the cell. So sub-LLM calls are this instance's own model calls: in its
// journal, under its budget, replayed on restore.
//
// The default export is a small bench: per task, an `RLM` and (with `direct`) a `Direct` baseline
// that reads the whole context in its prompt; the root grades both (`@results`).
import { TurnStrategy } from "@affordance/strategies/turn_strategy.tsx";
import { useLoop } from "@affordance/lib/loop.tsx";
import { MessagesApp } from "@affordance/template_apps/messages.tsx";
import { GLM53App } from "@affordance/template_apps/glm53.tsx";

const SYSTEM = `You are tasked with answering a query about a context that is too large to read directly. You work in a persistent Python REPL through the \`python\` tool, where these are defined:

1. \`context\`: the input (a string or JSON value). Look at its type, size and a few samples before you process it; never print it whole.
2. \`llm_batch(prompts)\`: sends a list of prompts to sub-LLMs at once and returns the list of their replies, in order (a reply that failed reads "[sub-LLM error: ...]"). A sub-LLM reads about 100,000 characters well. Use it to read and classify parts of \`context\` you cannot handle with code alone, e.g. chunk the context and ask a sub-LLM about each chunk in one batch. \`llm_query(prompt)\` sends one.
3. \`answer\`: a dict {"content": "", "ready": False}. Put your final answer in answer["content"] and set answer["ready"] = True when you are certain; that ends the task.

Variables persist between calls. You see what a cell prints (and its last expression), up to 8192 characters, so print summaries, not data. Use code for what code does exactly (splitting, counting, filtering) and sub-LLMs for what needs reading (meaning, classification). Make sub-LLM prompts self-contained and ask for output that is easy to parse.`;

const SUB_SYSTEM = "You are a careful assistant. Follow the instructions exactly and reply in the format asked.";

function Template({ template, system, log, children }) {
  if (template == "glm53") return <GLM53App system={system}><TurnStrategy strategy="unrolled" log={log}>{children}</TurnStrategy></GLM53App>;
  return <MessagesApp system={system}><TurnStrategy strategy="unrolled" log={log.map(r => ({...r, ids: null}))}>{children}</TurnStrategy></MessagesApp>;
}

// A sub-LLM call: one prompt, one reply.
function Sub({ template, prompt }) {
  return <Template template={template} system={SUB_SYSTEM} log={[]}>{prompt}</Template>;
}

export function RLM({ question, context, template = "messages", sub_template, onDone }) {
  const log = useLog();
  useLoop(log);
  const [loaded, setLoaded] = useState(false, "loaded");
  const [subcalls, setSubcalls] = useState(0, "subcalls");
  const repl = async (cmd, stdin) => {
    const r = await harness.machine.call({cmd, stdin, timeout_s: 600});
    const last = r.output.trim().split("\n").at(-1) || "";
    try { return JSON.parse(last); } catch (e) { return {status: "error", error: r.output.slice(-2000)}; }
  };
  const python = async ({ code }) => {
    // The input goes to the machine once, before the first cell, to a fresh REPL.
    if (!loaded) {
      await harness.machine.call({cmd: "rlm_repl stop >/dev/null; mkdir -p ~/.rlm && cat > ~/.rlm/context.json", stdin: JSON.stringify(context)});
      setLoaded(true);
    }
    let r = await repl("rlm_repl start ~/.rlm/context.json >/dev/null && rlm_repl exec", code);
    let calls = subcalls;
    while (r.status == "llm") {
      // A sub-LLM call that fails (after the harness's retries) answers with the error, so the
      // cell sees it and goes on; a cell is never left parked.
      const replies = await Promise.all(r.prompts.map(async p => {
        try { return (await model(<Sub template={sub_template || template} prompt={p}/>)).text; }
        catch (e) { return `[sub-LLM error: ${e}]`; }
      }));
      calls += replies.length;
      r = await repl("rlm_repl resume", JSON.stringify(replies));
    }
    setSubcalls(calls);
    if (r.status != "done") return "error: " + r.error;
    if (r.answer.ready) {
      if (onDone) onDone({answer: r.answer.content, turns: harness.turn + 1, subcalls: calls});
      harness.stop({answer: r.answer.content});
    }
    return r.output || "(no output)";
  };
  const app = <aff tool_name="python" description="Run Python code in the persistent REPL; returns what it prints (and its last expression)."
                   args_schema={{type: "object", properties: {code: {type: "string"}}, required: ["code"]}}
                   onPython={python}>{"[python]\n"}</aff>;
  const task = `Query: ${question}\n\nThe context is loaded in the REPL as \`context\`. Answer the query with the REPL.`;
  return <Template template={template} system={SYSTEM} log={[{kind: "input", content: task}, ...log.records]}>{app}</Template>;
}

// The baseline: the whole context in the prompt.
export function Direct({ question, context, template = "messages", onDone }) {
  const log = useLog();
  useLoop(log);
  const text = typeof context == "string" ? context : JSON.stringify(context);
  const app = <aff tool_name="answer" description="Give your final answer."
                   args_schema={{type: "object", properties: {text: {type: "string"}}, required: ["text"]}}
                   onAnswer={({ text }) => { if (onDone) onDone({answer: text, turns: harness.turn}); harness.stop({answer: text}); return "done"; }}>{"[answer]\n"}</aff>;
  const task = `Context:\n${text}\n\nQuery: ${question}\n\nRead the context and call \`answer\` with your final answer.`;
  // A reply cut off before it answered is dropped, so a retry costs one context, not a growing log.
  const kept = log.records.filter((r, i, t) => !r.cut_off && !(r.kind == "screen" && t[i + 1]?.cut_off));
  return <Template template={template} system={SUB_SYSTEM} log={[{kind: "input", content: task}, ...kept]}>{app}</Template>;
}

// Exact match on the answer's first number or word, as Oolong scores counts and ids.
function grade(answer, expect) {
  const m = String(answer).match(/-?\d+(\.\d+)?/);
  return (m ? m[0] : String(answer).trim()) == expect;
}

export default function Bench({ tasks, template = "messages", sub_template, direct = false }) {
  const [results, setResults] = useState({}, "results");
  const record = (key, expect) => (r) => setResults(all => ({...all, [key]: {...r, expect, correct: grade(r.answer, expect)}}));
  // Each RLM's REPL runs on one of the root's machines (in its own home); Direct needs none.
  return <>{tasks.map((t, i) => <>
    <Mount key={`${t.id}-rlm`} model="default" machine={harness.machines[i % harness.machines.length]}>
      <RLM question={t.question} context={t.context} template={template} sub_template={sub_template} onDone={record(`${t.id}-rlm`, t.expect)}/>
    </Mount>
    {direct ? <Mount key={`${t.id}-direct`} model="default">
      <Direct question={t.question} context={t.context} template={template} onDone={record(`${t.id}-direct`, t.expect)}/>
    </Mount> : ""}
  </>)}</>;
}
