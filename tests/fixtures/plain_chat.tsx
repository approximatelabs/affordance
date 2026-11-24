// A chat with no tools for tests/any_model.rs: the log comes from the test.
import { ModelContext } from "../../views/lib/model.tsx";
export default function Main({ log = [] }) {
  return <ModelContext log={log} strategy="unrolled">{"Answer briefly."}</ModelContext>;
}
