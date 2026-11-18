import { TurnStrategy } from "../../strategies/turn_strategy.tsx";
import { Qwen36App } from "../../template_apps/qwen36.tsx";
import { ConsumerAgent } from "./agent.tsx";
// This demo logs each reply with its exact token ids and asks for them to be kept
// (`keep_replies`): every prompt is then the last prompt, the reply and what follows. Without
// it a past reply is written the way the model's publisher writes one.
export default function Main({ log, handle, unread = 0, notes = "", strategy = "unrolled" }) {
  const app = <ConsumerAgent label="agent" handle={handle} unread={unread} notes={notes}/>;
  return <Qwen36App enable_thinking={true} keep_replies={true}><TurnStrategy strategy={strategy} log={log}>{app}</TurnStrategy></Qwen36App>;
}
