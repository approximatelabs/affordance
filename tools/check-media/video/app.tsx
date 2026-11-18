// A first agent: it reads its task, may run shell commands, and stops when it answers.
// The page is the task, the tools and the log of what the model saw and did; ModelContext
// renders it in the bound model's format (a hosted chat API, or a native token-id server).
import { ModelContext } from "@affordance/lib/model.tsx";
import { useLog } from "@affordance/lib/log.tsx";
import { useLoop } from "@affordance/lib/loop.tsx";
import { Tool } from "@affordance/lib/tool.tsx";
import { machineTools } from "@affordance/tools/machine.tsx";

const CLIP = "data:video/mp4;base64,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";

export default function Main({ task }) {
  const log = useLog();
  useLoop(log); // ask the model, run its calls, repeat
  const { shell } = machineTools(harness.machine.get("home"));
  /** Give your final answer. This ends the task. */
  function answer({ text }: { text: string }) {
    harness.stop({ answer: text });
    return "done";
  }
  /** Watch the video. */
  function watch({ why }: { why: string }) {
    return [{type: "text", text: "Here is the video."}, {type: "video_url", video_url: {url: CLIP}}];
  }
  return <ModelContext strategy="unrolled" log={[{ kind: "input", content: [{type: "video_url", video_url: {url: CLIP}}, {type: "text", text: task}] }, ...log.records]}>
    <Tool fn={answer}>{"[answer]\n"}</Tool>
  </ModelContext>;
}
