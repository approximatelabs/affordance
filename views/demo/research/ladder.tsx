// The research ladder as an instance tree: one agent per (question, context policy), each on its
// own machine, run side by side. The root has no model: it mounts the agents, hears each answer
// (`onDone`) and grades it against the expected strings; a run that stops without answering
// (`onStop`, which the harness calls for any child) counts as a failure. Its state is the results table.
import { ResearchAgent } from "./agent.tsx";
// The context budget is the model binding's `context` (the window less `max_new`).
export default function Ladder({ questions, policies = ["none", "trunc", "compact", "tree"], template = "messages", lazy = false }) {
  const [results, setResults] = useState({}, "results");
  const runs = [];
  for (const [qi, q] of questions.entries()) {
    for (const policy of policies) runs.push({key: `q${qi}-${policy}`, q, policy});
  }
  return <>
    {runs.map((r, i) =>
      <Mount key={r.key} model="default" machine={harness.machines[i % harness.machines.length]}>
        <ResearchAgent question={r.q.question} template={template} policy={r.policy} lazy={lazy}
          onDone={({ answer, turns }) => {
            const exact = (answer.split("Exact Answer:")[1] || answer).split("\n")[0].trim();
            const correct = r.q.expect.every(e => answer.toLowerCase().includes(e.toLowerCase()));
            setResults(all => ({...all, [r.key]: {policy: r.policy, question: r.q.question, exact, correct, turns}}));
          }}
          onStop={(how) => {
            // A run that stopped without answering (out of turns, an error) is a failure.
            setResults(all => all[r.key] ? all : {...all, [r.key]: {policy: r.policy, question: r.q.question, exact: "", correct: false, stopped: how}});
          }}/>
      </Mount>)}
  </>;
}
