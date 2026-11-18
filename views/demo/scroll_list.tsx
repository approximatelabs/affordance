// Demo: a long list of integers seen through a 64-token window, with actions.
// This app pages by item offset; its onScroll action returns the next offset as a state patch.
// The renderer attaches that action to the visible box. Native token scrolling uses overflow: scroll.
"use strict_tokens";

const NumberList = ({ numbers, offset, budget, onScroll }) =>
  <box label="numbers" style={{ maxTokens: budget, more: "\n[... more below]" }} onScroll={onScroll}>
    {numbers.slice(offset).map(n => p`${n}\n`)}
  </box>;

export default function ScrollDemo({ numbers, offset, budget, question }) {
  return <>
    {p`${tok.im_start}system\nYou see a window onto a list of ${numbers.length} integers.${tok.im_end}\n${tok.im_start}user\n`}
    <NumberList numbers={numbers} offset={offset} budget={budget}
                onScroll={({ by }) => ({ offset: Math.max(0, offset + by) })}/>
    {p`\n`}
    <aff name="answer" args={["value"]}>{question}</aff>
    {p`${tok.im_end}\n${tok.im_start}assistant\n`}
  </>;
}
