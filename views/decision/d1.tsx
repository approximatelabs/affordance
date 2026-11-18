// LiquidAI d1 (a published decision model, LiquidAI/d1-3B): one choice question over a state and
// images, rendered exactly as the checkpoint's own prompt.py renders it (state style json_only, no
// system turn, option style desc). Each option is an <aff option> carrying the code letter d1
// reads at the answer slot. The readout (softmax over the code tokens' logits, each code scored by
// its best form) runs in the serving host: <Decision readout="token_logprob">. No question span:
// d1 reads only the answer slot, and its whole-prompt tokenization merges the question with its
// neighbouring newlines.
"use strict_tokens";

// prompt.py option_codes: the names themselves when every name is one letter, else A..Z, else 00..
export function d1Codes(names) {
  let letters = names.length > 0;
  for (const n of names) { if (n.length !== 1 || n.toLowerCase() === n.toUpperCase()) letters = false; }
  if (letters) return names;
  if (names.length <= 26) return names.map((n, i) => "ABCDEFGHIJKLMNOPQRSTUVWXYZ"[i]);
  return names.map((n, i) => (i < 10 ? "0" : "") + String(i));
}

// options: [{name, desc}] in the order d1 should see them. state: a string, any JSON value, or null
// when the images are the whole state. images: screen sources, one <image> placeholder each.
export function D1Choice({ state, images = [], instructions, options }) {
  const codes = d1Codes(options.map(o => o.name));
  const stateText = state === null ? "" : (typeof state === "string" ? state : formatJSON(state, {indent: 2})) + "\n\n\nQUESTION:\n";
  return <>
    {tok.startoftext}{tok.im_start}{"user\n"}
    {images.map(src => <img src={src} tok_size={1} t="<image>"/>)}
    {stateText}
    {instructions}
    {"\n\nOptions:\n"}
    {options.map((o, i) => <>
      {i > 0 ? "\n" : ""}
      <aff option={o.name} code={codes[i]}>{codes[i] + " " + (o.desc ?? o.name.split("_").join(" "))}</aff>
    </>)}
    {"\n\nReply with the option code only."}{tok.im_end}{"\n"}{tok.im_start}{"assistant\n"}
  </>;
}
