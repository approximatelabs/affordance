import { useLog } from "../../lib/log.tsx";
// model(<View/>): a component rendered offscreen and run once on this instance's model, through
// the harness (tests/runtime.rs).
import { TurnStrategy } from "../../strategies/turn_strategy.tsx";
import { useLoop } from "../../lib/loop.tsx";
import { GLM53App } from "../../template_apps/glm53.tsx";

function Polish({ text }) {
  return <GLM53App system="Rewrite the user's text politely, in under ten words.">
    <TurnStrategy strategy="unrolled" log={[]}>{text}</TurnStrategy>
  </GLM53App>;
}

export default function Main({ task }) {
  const log = useLog();
  useLoop(log);
  const app = <aff tool_name="polish" description="Make a sentence polite."
                   args_schema={{type: "object", properties: {text: {type: "string"}}, required: ["text"]}}
                   onPolish={async ({ text }) => {
                     // Its own budget and sampling; tagged in captures and traces; admitted first.
                     try { return (await model(<Polish text={text}/>, {max_new: 64, sampling: {temperature: 0.2}, tag: "polish", priority: true})).text; }
                     catch (e) { return `error: ${e}`; }
                   }}>{"[polish]\n"}</aff>;
  return <GLM53App>
    <TurnStrategy strategy="unrolled" log={[{kind: "input", content: task}, ...log.records]}>{app}</TurnStrategy>
  </GLM53App>;
}
