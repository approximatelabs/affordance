import { ModelContext } from "../../views/lib/model.tsx";

// A small app for the `affordance render` tests: text, or an editable notes field.
export default function Main({ text, active }) {
  const body = active
    ? <aff label="notes" description="Replace notes" onChange={({ text }) => ({ note: text })}>{text}</aff>
    : text;
  return <ModelContext system="App test.">{body}</ModelContext>;
}
