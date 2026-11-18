// A video shown as its frames: the first message is the frames and a question. The frames
// are pictures in the model's format, each sized by frames.tss.
import { ModelContext } from "@affordance/lib/model.tsx";
import { useLog } from "@affordance/lib/log.tsx";
import { useLoop } from "@affordance/lib/loop.tsx";
import { Tool } from "@affordance/lib/tool.tsx";
import { frames } from "@affordance/lib/media.tsx";
import "./frames.tss";

export default function Main({ task, shots }) {
  const log = useLog();
  useLoop(log);
  /** Give your final answer. This ends the task. */
  function answer({ text }: { text: string }) {
    harness.stop({ answer: text });
    return "done";
  }
  const content = [...frames(shots, {seconds_apart: 0.25, max_frames: 4}), {type: "text", text: task}];
  return <ModelContext strategy="unrolled" log={[{ kind: "input", content }, ...log.records]}>
    <Tool fn={answer}>{"[answer]\n"}</Tool>
  </ModelContext>;
}
