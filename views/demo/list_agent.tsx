// Demo for the harness loop (spec section 10): a model must find the number at a position in a
// long list. History is rendered with the exact Qwen3 view and never changes; the live "screen"
// comes last, so each step reuses the model's KV cache for everything before it.
//   model actions:      scroll (on the list box), lookup (slow: the result arrives as a callback)
//   harness callbacks:  onCallback event="lookup:N" (lookup result), onTimeout after={4}
"use strict_tokens";
import { Qwen3 } from "../qwen/qwen3.tsx";
import { Qwen3Instruct2507 } from "../qwen/qwen3_2507.tsx";

const TOOLS = [
  { type: "function", function: { name: "scroll", description: "Move the list window by `by` items (negative moves up).",
    parameters: { type: "object", properties: { by: { type: "integer" } }, required: ["by"] } } },
  { type: "function", function: { name: "lookup", description: "Ask for the number at a position. Slow: the result appears on the screen later.",
    parameters: { type: "object", properties: { position: { type: "integer" } }, required: ["position"] } } },
  { type: "function", function: { name: "wait", description: "Let time pass so that pending lookups can finish.",
    parameters: { type: "object", properties: {} } } },
];

const WINDOW = 120; // tokens of list on screen

// The assistant turn opener: Qwen3 hybrid without thinking gets an empty think block; with
// thinking the model opens <think> itself; Qwen3-Instruct-2507 has no think block.
const Opener = ({ format, thinking }) =>
  p`${tok.im_start}assistant\n${format !== "2507" && !thinking ? p`${tok.think}\n\n${tok.think_end}\n\n` : ""}`;

function Screen({ numbers, offset, pending, found, note, format, thinking }) {
  return <>
    {p`${tok.im_start}user\n[screen]\nThe list has ${numbers.length} numbers (positions 0 to ${numbers.length - 1}). The window below shows positions ${offset} and up; scroll to see more.\n`}
    <box label="list" max_tokens={WINDOW} more="..."
         onScroll={({ by }) => ({ offset: Math.min(Math.max(0, offset + by), numbers.length - 1),
                                  result: `window now starts at position ${Math.min(Math.max(0, offset + by), numbers.length - 1)}` })}>
      {numbers.slice(offset).map((n, i) => p`${offset + i}: ${n}\n`)}
    </box>
    {"\n"}
    <aff name="lookup" onLookup={({ position }) => ({ pending: [...pending, position], result: "lookup started; the result will appear on the screen. Call wait to let it finish." })}/>
    {pending.length > 0 && <aff name="wait" onWait={() => ({ result: "waited" })}/>}
    {pending.map(pos =>
      <aff name={`lookup_${pos}`} event={`lookup:${pos}`}
           onCallback={({ value }) => ({ pending: pending.filter(q => q !== pos), found: [...found, [pos, value]] })}>
        {p`Lookup of position ${pos}: pending\n`}
      </aff>)}
    {found.map(([pos, value]) => p`Lookup of position ${pos}: ${value}\n`)}
    {note ? p`Note: ${note}\n`
          : <aff name="deadline" after={4} onTimeout={() => ({ note: "Time is up. Answer now with just the number." })}/>}
    {p`${tok.im_end}\n`}
    <Opener format={format} thinking={thinking}/>
  </>;
}

export default function ListAgent({ messages, numbers, offset = 0, pending = [], found = [], note = "", format = "qwen3", thinking = false }) {
  return <>
    {format === "2507"
      ? <Qwen3Instruct2507 messages={messages} tools={TOOLS} add_generation_prompt={false}/>
      : <Qwen3 messages={messages} tools={TOOLS} add_generation_prompt={false} enable_thinking={thinking}/>}
    <Screen numbers={numbers} offset={offset} pending={pending} found={found} note={note} format={format} thinking={thinking}/>
  </>;
}

export { Screen, TOOLS };
