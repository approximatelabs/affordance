// A fleet as a root app (tests/agents_bin.rs): one agent per machine in the root's pool. The
// launch config only binds the pool and the model; which agent gets which machine is here.
import { Main } from "../runtime/agent.tsx";

export default function Fleet({ task }) {
  return <>
    {harness.machines.map((m, n) =>
      <Mount key={`agent-${n}`} model="default" machine={m}>
        <Main task={`${task} You are agent ${n}.`}/>
      </Mount>)}
  </>;
}
