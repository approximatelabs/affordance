import { useLog } from "../../lib/log.tsx";
// A turn strategy is app code: this chat shows the model only the records since the last user
// message, plus a one-line note of how many came before. Any policy (a window, a summary tree,
// a fold) is a function of the log like this one.
import { TurnStrategy } from "../../strategies/turn_strategy.tsx";
import { useConversation } from "../../lib/loop.tsx";
import { GLM53App } from "../../template_apps/glm53.tsx";

function sinceLastInput(log) {
  let start = 0;
  for (let i = 0; i < log.length; i++) if (log[i].kind === "input") start = i;
  const note = start > 0 ? [{role: "user", content: `(${start} earlier records are not shown)`}] : [];
  return [...note, ...logTurns(log.slice(start))];
}

export default function Main() {
  const log = useLog();
  useConversation(log);
  return <GLM53App>
    <TurnStrategy strategy={sinceLastInput} log={log.records}>
      <aff event="message" onCallback={({ text }) => log.append({kind: "input", content: text})}/>
    </TurnStrategy>
  </GLM53App>;
}
