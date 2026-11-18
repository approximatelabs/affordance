// A first agent: it reads its task, may run shell commands, and stops when it answers.
// The page is the task, the tools and the log of what the model saw and did; ModelContext
// renders it in the bound model's format (a hosted chat API, or a native token-id server).
import { ModelContext } from "@affordance/lib/model.tsx";
import { useLog } from "@affordance/lib/log.tsx";
import { useLoop } from "@affordance/lib/loop.tsx";
import { Tool } from "@affordance/lib/tool.tsx";
import { machineTools } from "@affordance/tools/machine.tsx";

export default function Main({ task }) {
  const log = useLog();
  useLoop(log); // ask the model, run its calls, repeat
  const { shell } = machineTools(harness.machine.get("home"));
  /** Give your final answer. This ends the task. */
  function answer({ text }: { text: string }) {
    harness.stop({ answer: text });
    return "done";
  }
  return <ModelContext strategy="unrolled" log={[{ kind: "input", content: task }, ...log.records]}>
    <Tool fn={shell}>{"[shell] "}</Tool>
    <Tool fn={answer}>{"[answer]\n"}</Tool>
  </ModelContext>;
}
