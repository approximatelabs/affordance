import { useLog } from "../../lib/log.tsx";
// The hello world: a chat bot with one tool. It answers whatever is unanswered and is quiet after
// it speaks; messages that arrive while it answers are answered together, next. The log is app
// state (views/lib/log.tsx), the policy a library hook (views/lib/loop.tsx), the tool a function
// whose name, doc and parameter type are its name, description and schema (views/lib/tool.tsx).
import { TurnStrategy } from "../../strategies/turn_strategy.tsx";
import { useConversation } from "../../lib/loop.tsx";
import { Tool } from "../../lib/tool.tsx";
import { GLM53App } from "../../template_apps/glm53.tsx";

/** Look something up. */
async function lookup({ q }: { q: string }) {
  return (await harness.machine.call({ cmd: q })).output;
}

export default function Main() {
  const log = useLog();
  useConversation(log);
  useEvent("message", ({ text }) => log.append({ kind: "input", content: text }));
  return <GLM53App>
    <TurnStrategy strategy="unrolled" log={log.records}>
      <Tool fn={lookup}>{"[lookup]\n"}</Tool>
    </TurnStrategy>
  </GLM53App>;
}
