import { useLog } from "../../lib/log.tsx";
// An app whose fold (on the layout event) always fails (it has no machine): the turn proceeds
// with the prompt as it is, and the agent is not stopped (tests/runtime.rs).
import { TurnStrategy } from "../../strategies/turn_strategy.tsx";
import { useLoop } from "../../lib/loop.tsx";
import { GLM53App } from "../../template_apps/glm53.tsx";

export default function Main({ task }) {
  const log = useLog();
  useEvent("layout", async ({ prompt_tokens }) => { if (prompt_tokens > 10) await harness.machine.call({cmd: "summarize"}); });
  useLoop(log);
  const app = <>
    <box label="status">{`turn ${harness.turn}\n`}</box>
  </>;
  return <GLM53App>
    <TurnStrategy strategy="unrolled" log={[{kind: "input", content: task}, ...log.records]}>{app}</TurnStrategy>
  </GLM53App>;
}
