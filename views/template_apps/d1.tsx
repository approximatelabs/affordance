// LiquidAI d1 (a decision model) as a template app: the screen is the state, the app's tools are
// the options, and the reply is one option code. The prompt is d1's own (views/decision/d1.tsx,
// token-exact against the publisher's prompt.py); the protocol span maps each code to its tool, so
// a reply `B` calls the second tool with no arguments (reply syntax `choice`). The readout, the
// most likely code at the answer slot, is the server's: restrict its next token to the codes.
"use strict_tokens";
import { d1Codes } from "../decision/d1.tsx";

export function D1App({ children, system = "Which action should be taken next?" }) {
  const input = templateInput(children, null, true);
  const screen = input.turns.length > 0 ? input.turns[input.turns.length - 1].content : "";
  const tools = input.tools.map(t => t.function);
  const codes = d1Codes(tools.map(t => t.name));
  const map = {};
  tools.forEach((t, i) => { map[codes[i]] = t.name; });
  return [<>
    {tok.startoftext}{tok.im_start}{"user\n"}
    {typeof screen === "string" ? screen : textContent(screen)}
    {"\n\n\nQUESTION:\n"}
    {system}
    {"\n\nOptions:\n"}
    {tools.map((t, i) => <>
      {i > 0 ? "\n" : ""}
      <aff option={t.name} code={codes[i]}>{codes[i] + " " + (t.description || t.name.split("_").join(" "))}</aff>
    </>)}
    {"\n\nReply with the option code only."}{tok.im_end}{"\n"}{tok.im_start}{"assistant\n"}
  </>, <span label="protocol" syntax="choice" codes={map} stop={[tok.im_end.id]} in_think={false}/>];
}
