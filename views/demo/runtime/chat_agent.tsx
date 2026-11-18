import { useLog } from "../../lib/log.tsx";
// A minimal agent on a hosted chat API (a `chat` model binding): MessagesApp sends the turns and
// tools as JSON. Records render from their fields, so the transcript drops its token ids.
import { TurnStrategy } from "../../strategies/turn_strategy.tsx";
import { useLoop } from "../../lib/loop.tsx";
import { MessagesApp } from "../../template_apps/messages.tsx";

export default function Main({ task }) {
  const log = useLog();
  useLoop(log);
  const turns = [{kind: "input", content: task}, ...log.records.map(r => ({...r, ids: null}))];
  const app = <>
    <aff tool_name="shell" description="Run a bash command on your machine; returns its output."
         args_schema={{type: "object", properties: {command: {type: "string"}}, required: ["command"]}}
         onRun={async ({ command }) => {
           const r = await harness.machine.call({cmd: command, timeout_s: 30});
           return `${r.output}[exit ${r.exit}]`;
         }}>{"[shell]\n"}</aff>
    <aff tool_name="answer" description="Give your final answer and finish."
         args_schema={{type: "object", properties: {text: {type: "string"}}, required: ["text"]}}
         onAnswer={({ text }) => { harness.stop(text); return "done"; }}>{"[answer]\n"}</aff>
  </>;
  return <MessagesApp system="You are a careful assistant with a Linux shell. Use tools; answer with the answer tool.">
    <TurnStrategy strategy="unrolled" log={turns}>{app}</TurnStrategy>
  </MessagesApp>;
}
