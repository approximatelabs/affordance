// A first agent: it reads its task, may run shell commands, and stops when it answers.
// The page is the task, the tools and the log of what the model saw and did; ModelContext
// renders it in the bound model's format (a hosted chat API, or a native token-id server).
import { ModelContext } from "@affordance/lib/model.tsx";
import { useLog } from "@affordance/lib/log.tsx";
import { useLoop } from "@affordance/lib/loop.tsx";
import { Tool } from "@affordance/lib/tool.tsx";
import { machineTools } from "@affordance/tools/machine.tsx";

const PICTURE = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAOAAAADgCAIAAACVT/22AAACbklEQVR4nO3SMQHAIADAsDENyMG/CsTggJceiYIeHXPtD6r+1wFwY1DSDEqaQUkzKGkGJc2gpBmUNIOSZlDSDEqaQUkzKGkGJc2gpBmUNIOSZlDSDEqaQUkzKGkGJc2gpBmUNIOSZlDSDEqaQUkzKGkGJc2gpBmUNIOSZlDSDEqaQUkzKGkGJc2gpBmUNIOSZlDSDEqaQUkzKGkGJc2gpBmUNIOSZlDSDEqaQUkzKGkGJc2gpBmUNIOSZlDSDEqaQUkzKGkGJc2gpBmUNIOSZlDSDEqaQUkzKGkGJc2gpBmUNIOSZlDSDEqaQUkzKGkGJc2gpBmUNIOSZlDSDEqaQUkzKGkGJc2gpBmUNIOSZlDSDEqaQUkzKGkGJc2gpBmUNIOSZlDSDEqaQUkzKGkGJc2gpBmUNIOSZlDSDEqaQUkzKGkGJc2gpBmUNIOSZlDSDEqaQUkzKGkGJc2gpBmUNIOSZlDSDEqaQUkzKGkGJc2gpBmUNIOSZlDSDEqaQUkzKGkGJc2gpBmUNIOSZlDSDEqaQUkzKGkGJc2gpBmUNIOSZlDSDEqaQUkzKGkGJc2gpBmUNIOSZlDSDEqaQUkzKGkGJc2gpBmUNIOSZlDSDEqaQUkzKGkGJc2gpBmUNIOSZlDSDEqaQUkzKGkGJc2gpBmUNIOSZlDSDEqaQUkzKGkGJc2gpBmUNIOSZlDSDEqaQUkzKGkGJc2gpBmUNIOSZlDSDEqaQUkzKGkGJc2gpBmUNIOSZlDSDEqaQUkzKGkGJc2gpBmUNIOSZlDSDEqaQUkzKGkGJc2gpBmUNIOSZlDSDEqaQUkzKGkGJc2gpB3q+QLsWTeD8wAAAABJRU5ErkJggg==";

export default function Main({ task }) {
  const log = useLog();
  useLoop(log); // ask the model, run its calls, repeat
  const { shell } = machineTools(harness.machine.get("home"));
  /** Give your final answer. This ends the task. */
  function answer({ text }: { text: string }) {
    harness.stop({ answer: text });
    return "done";
  }
  /** Look at the picture. */
  function look({ why }: { why: string }) {
    return [{type: "text", text: "Here is the picture."}, {type: "image_url", image_url: {url: PICTURE}}];
  }
  return <ModelContext strategy="unrolled" log={[{ kind: "input", content: [{type: "image_url", image_url: {url: PICTURE}}, {type: "text", text: task}] }, ...log.records]}>
    <Tool fn={answer}>{"[answer]\n"}</Tool>
  </ModelContext>;
}
