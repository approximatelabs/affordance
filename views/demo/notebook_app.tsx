import { ModelContext } from "../lib/model.tsx";

// App code contains no model protocol tokens or tool table.
export function Notebook({ numbers, budget = 48 }) {
  const [offset, setOffset] = useState(0);
  const [note, setNote] = useState("");
  return <>
    <box label="numbers" max_tokens={budget} more="..."
         description="Scroll the number list by a number of rows."
         args_schema={{ type: "object", properties: { by: { type: "integer" } }, required: ["by"] }}
         onScroll={({ by }) => setOffset(o => Math.min(Math.max(0, o + by), Math.max(0, numbers.length - 1)))}>
      {p`List begins at position ${offset}\n`}
      {numbers.slice(offset).map((n, i) => p`${offset + i}: ${n}\n`)}
    </box>
    {"\n"}
    <aff label="notes" description="Replace the text in the notes input."
         args_schema={{ type: "object", properties: { text: { type: "string" } }, required: ["text"] }}
         onChange={({ text }) => setNote(text)}>{p`Notes: ${note}\n`}</aff>
  </>;
}

// The whole app, for any model: `ModelContext` writes it in the format the model binding names.
export default function Main({ numbers, budget = 48, log = [] }) {
  return <ModelContext log={log}><Notebook key="notebook" numbers={numbers} budget={budget}/></ModelContext>;
}
