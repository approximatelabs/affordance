import { useLog } from "../../lib/log.tsx";
// Durable send (tests/runtime.rs): the tool's result is what the runtime answered.
import { TurnStrategy } from "../../strategies/turn_strategy.tsx";
import { useLoop } from "../../lib/loop.tsx";
import { GLM53App } from "../../template_apps/glm53.tsx";

export default function Main({ task }) {
  const log = useLog();
  useLoop(log);
  const app = <>
    <aff tool_name="send" description="Send a message." args_schema={{type: "object", properties: {to: {type: "string"}}, required: ["to"]}}
         onSend={async ({ to }) => JSON.stringify(await harness.send(to, {text: "hi"}))}>{"[send]\n"}</aff>
    <aff event="message" onCallback={async ({ from, text }) => { log.append({kind: "input", content: `[${from}] ${text}`}); }}/>
  </>;
  return <GLM53App>
    <TurnStrategy strategy="unrolled" log={[{kind: "input", content: task}, ...log.records]}>{app}</TurnStrategy>
  </GLM53App>;
}
