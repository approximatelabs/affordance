import { useLog } from "../../lib/log.tsx";
// A greeter: asks its machine its name, then greets once (a one-shot render polishes the text)
// and stops. Its parent passed `onGreet`; calling it is an event in the parent.
import { TurnStrategy } from "../../strategies/turn_strategy.tsx";
import { useLoop } from "../../lib/loop.tsx";
import { GLM53App } from "../../template_apps/glm53.tsx";

function Polish({ text }) {
  return <GLM53App system="Rewrite the user's text politely, in under ten words.">
    <TurnStrategy strategy="unrolled" log={[]}>{text}</TurnStrategy>
  </GLM53App>;
}

export default function Greeter({ name, heard, onGreet }) {
  const log = useLog();
  useLoop(log);
  const app = <>
    <box label="screen">{`Greetings so far: ${heard.join(" | ") || "(none)"}\n`}</box>
    <aff tool_name="hostname" description="Ask your machine its name." onRun={async () => (await harness.machine.call({cmd: "hostname"})).output}>{"[hostname]\n"}</aff>
    <aff tool_name="greet" description="Greet everyone."
         args_schema={{type: "object", properties: {text: {type: "string"}}, required: ["text"]}}
         onGreet={async ({ text }) => {
           const polite = await model(<Polish text={text}/>);
           onGreet(`${name}: ${polite.text}`);
           harness.stop("greeted");
           return "greeted";
         }}>{"[greet]\n"}</aff>
  </>;
  return <GLM53App system={`You are ${name}. Say hello once, then stop.`}>
    <TurnStrategy strategy="unrolled" log={log.records}>{app}</TurnStrategy>
  </GLM53App>;
}

export { Greeter };
