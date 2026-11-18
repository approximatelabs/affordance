// Demo: view -> decision -> next view with component state (useState) and actions (on* props).
// The window position is component state; the "scroll" action changes it; the next render
// reuses every token before the window (the cached prefix).
"use strict_tokens";

function NumberList({ id, numbers, budget }) {
  const [offset, setOffset] = useState(0);
  return <box label={id} style={{ maxTokens: budget, more: "\n[... more below]" }}
              onScroll={({ by }) => setOffset(o => Math.min(Math.max(0, o + by), numbers.length - 1))}>
    {p`[showing from item ${offset}]\n`}
    {numbers.slice(offset).map(n => p`${n}\n`)}
  </box>;
}

export default function ScrollState({ numbers, budget, question }) {
  return <>
    {p`${tok.im_start}system\nYou see a window onto a list of ${numbers.length} integers. Scroll to read more.${tok.im_end}\n${tok.im_start}user\n${question}\n`}
    <NumberList id="numbers" numbers={numbers} budget={budget}/>
    {p`${tok.im_end}\n${tok.im_start}assistant\n`}
  </>;
}
