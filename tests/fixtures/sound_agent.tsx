// An agent whose first message is a sound clip, for tests/media_layout.rs: the clip comes
// from the test.
import { ModelContext } from "../../views/lib/model.tsx";
import { ConsumerAgent } from "../../views/demo/consumer/agent.tsx";
export default function Main({ clip }) {
  const content = [{type: "input_audio", input_audio: {data: clip, format: "wav"}}, {type: "text", text: "What is said?"}];
  return <ModelContext log={[{kind: "input", content}]} strategy="unrolled"><ConsumerAgent label="agent" handle="a"/></ModelContext>;
}
