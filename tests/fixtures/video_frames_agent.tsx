// A video shown to a model as its frames (views/lib/media.tsx), for tests/media_layout.rs:
// the shots come from the test, four of them are shown, and the sheet gives each 64 tokens.
import { ModelContext } from "../../views/lib/model.tsx";
import { ConsumerAgent } from "../../views/demo/consumer/agent.tsx";
import { frames } from "../../views/lib/media.tsx";
import "./video_frames_agent.tss";
export default function Main({ shots = [] }) {
  const content = [...frames(shots, {seconds_apart: 0.5, max_frames: 4}), {type: "text", text: "What happens?"}];
  return <ModelContext log={[{kind: "input", content}]} strategy="unrolled"><ConsumerAgent label="agent" handle="a"/></ModelContext>;
}
