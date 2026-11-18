import { ModelContext } from "../lib/model.tsx";

// The app itself: no model format, tool table, or history policy.
export function FindBigNumber({ numbers, budget = 256 }) {
  const [offset, setOffset] = useState(0);
  const [scratch, setScratch] = useState("");
  const [answer, setAnswer] = useState("");
  const count = String(numbers.length);
  const shown = numbers.length >= 1000 ? count.slice(0, count.length - 3) + "," + count.slice(count.length - 3) : count;
  return <>
    <box label="goal">{"Goal: Find the biggest of the " + shown + " notebook numbers. Write it in Answer, then press Submit.\n"}</box>
    <aff label="scratchpad" description="Replace your scratchpad."
         args_schema={{type:"object", properties:{text:{type:"string"}}, required:["text"]}}
         onChange={({ text }) => setScratch(text)}>{p`Scratchpad: ${scratch}\n`}</aff>
    <box label="notebook" max_tokens={budget} more="..."
         description="Scroll the notebook by a number of rows."
         args_schema={{type:"object", properties:{by:{type:"integer"}}, required:["by"]}}
         onScroll={({ by }) => setOffset(o => Math.min(Math.max(0, o + by), Math.max(0, numbers.length - 1)))}>
      {p`Notebook (${numbers.length} numbers), starts at row ${offset}:\n`}
      {numbers.slice(offset).map((n, i) => p`${i ? "\n" : ""}${offset + i}: ${n}`)}
    </box>
    {"\n"}
    <box label="answer-box">
      <aff label="answer" description="Replace the answer field."
           args_schema={{type:"object", properties:{text:{type:"string"}}, required:["text"]}}
           onChange={({ text }) => setAnswer(text)}>{p`Answer: ${answer}\n`}</aff>
      <aff label="submit" description="Submit the value currently in the answer field."
           args_schema={{type:"object", properties:{}, required:[]}}
           onClick={() => ({ submitted: answer })}>{"Submit"}</aff>
    </box>
  </>;
}

// The whole app, for any model: `ModelContext` renders it in the format the model binding names
// (a deployment's `model.template`, or `affordance render --template NAME`).
export default function Main({ numbers, budget = 256, strategy = "last-turn", log = [] }) {
  return <ModelContext strategy={strategy} log={log}>
    <FindBigNumber label="find-big-number" numbers={numbers} budget={budget}/>
  </ModelContext>;
}
