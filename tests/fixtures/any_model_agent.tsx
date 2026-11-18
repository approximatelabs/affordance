// A small agent app for tests/any_model.rs: one screen with three actions, shown to whichever
// model the binding names. The log comes from the test.
import { ModelContext } from "../../views/lib/model.tsx";
import { ConsumerAgent } from "../../views/demo/consumer/agent.tsx";
export default function Main({ log = [] }) {
  return <ModelContext log={log} strategy="unrolled"><ConsumerAgent label="agent" handle="a"/></ModelContext>;
}
