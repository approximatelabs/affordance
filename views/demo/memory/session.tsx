import { useLog } from "@affordance/lib/log.tsx";
// One session of an agent with permanent memory (`memory.tsx` is its parent). It wakes with the
// memory's cover, works on its task, records what has lasting effect (`note`), and can search
// every memory (`recall`) or open a summary into its two halves (`zoom`). `answer` ends it.
import { TurnStrategy } from "@affordance/strategies/turn_strategy.tsx";
import { useLoop } from "@affordance/lib/loop.tsx";
import { MessagesApp } from "@affordance/template_apps/messages.tsx";
import { GLM53App } from "@affordance/template_apps/glm53.tsx";

const SYSTEM = "You are an assistant with a permanent memory that outlives this session. Your memory is shown at the start, as lines `#i text` (one memory) and `#a-b text` (a summary of memories a..b, oldest collapsed most). Record anything of lasting effect with `note` (one line each, no duplicates). To find an old memory, `recall` searches every memory word for word and `zoom` opens a summary into its two halves. Finish with `answer`.";

function Context({ template, log, children }) {
  if (template == "glm53") return <GLM53App system={SYSTEM}><TurnStrategy strategy="unrolled" log={log}>{children}</TurnStrategy></GLM53App>;
  return <MessagesApp system={SYSTEM}><TurnStrategy strategy="unrolled" log={log.map(r => ({...r, ids: null}))}>{children}</TurnStrategy></MessagesApp>;
}

export function Session({ task, template = "messages", wake, memories, tree, onNote }) {
  const log = useLog();
  useLoop(log);
  // The session wakes once: later notes arrive as props but stay out of its first message, so each
  // prompt is an exact append of the last. It sees its own notes as its tool calls.
  const [woke] = useState(wake, "woke");
  const app = <>
    <aff tool_name="note" description="Record one memory: one line, at most 280 bytes."
         args_schema={{type: "object", properties: {text: {type: "string"}}, required: ["text"]}}
         onNote={({ text }) => { onNote(text); return `noted as #${memories.length}`; }}>{"[note]\n"}</aff>
    <aff tool_name="recall" description="Search every memory ever recorded, word for word (a regular expression, case-insensitive)."
         args_schema={{type: "object", properties: {pattern: {type: "string"}}, required: ["pattern"]}}
         onRecall={({ pattern }) => {
           const re = new RegExp(pattern, "i");
           const hits = [];
           for (const [i, m] of memories.entries()) { if (re.test(m)) hits.push(`#${i} ${m}`); }
           return hits.length ? hits.slice(0, 20).join("\n") : `no memory matches ${pattern}`;
         }}>{"[recall]\n"}</aff>
    <aff tool_name="zoom" description="Open a summary #a-b into its two halves (down to the memories themselves)."
         args_schema={{type: "object", properties: {lo: {type: "integer"}, hi: {type: "integer"}}, required: ["lo", "hi"]}}
         onZoom={({ lo, hi }) => {
           const end = hi + 1;
           if (end - lo < 2) return `#${lo} ${memories[lo] ?? "(none)"}`;
           const mid = Math.trunc((lo + end) / 2);
           const half = (a, b) => b - a == 1 ? `#${a} ${memories[a]}` : `#${a}-${b - 1} ${tree[`${a}-${b}`] ?? "(no summary)"}`;
           return half(lo, mid) + "\n" + half(mid, end);
         }}>{"[zoom]\n"}</aff>
    <aff tool_name="answer" description="Give your final answer for this session and end it."
         args_schema={{type: "object", properties: {text: {type: "string"}}, required: ["text"]}}
         onAnswer={({ text }) => { harness.stop(text); return "done"; }}>{"[answer]\n"}</aff>
  </>;
  const turns = [{kind: "input", content: `## Your memory\n${woke}\n\n## Task\n${task}`}, ...log.records];
  return <Context template={template} log={turns}>{app}</Context>;
}
