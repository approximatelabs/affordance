// Demo: a long list of integers shown as a scrollable list (token budget + affordances).
// State: {numbers: [int], offset: int, budget: int, question: str}. Rendered in Qwen3 ChatML.
// Scrolling is two text actions ([scroll up] /
// [scroll down]) whose attributes (target, direction, to) are copied into the span meta.
// views/demo/scroll_list.tsx is the newer form, with onScroll on the list box itself.
"use strict_tokens";

function NumberList({ numbers, offset, budget }) {
  return <>
    {/* the box clips its content to exactly `budget` tokens; `more` is counted inside the budget */}
    <box label="numbers" max_tokens={budget} more={"\n[... more below]"}>
      {numbers.slice(offset).map(n => p`${n}\n`)}
    </box>
    {offset > 0 && <aff name="scroll" target="numbers" direction="up" to={offset - 10 > 0 ? offset - 10 : 0}>{"[scroll up]"}</aff>}
    <aff name="scroll" target="numbers" direction="down" to={offset + 10}>{"[scroll down]"}</aff>
  </>;
}

export default function ScrollDemo({ numbers, offset, budget, question }) {
  return <>
    {p`${tok.im_start}system\nYou see a window onto a list of ${numbers.length} integers.${tok.im_end}\n${tok.im_start}user\n`}
    <NumberList numbers={numbers} offset={offset} budget={budget}/>
    {"\n"}
    <aff name="answer" args={["value"]}>{question}</aff>
    {p`${tok.im_end}\n${tok.im_start}assistant\n`}
  </>;
}
