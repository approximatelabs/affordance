import { textOperand } from "../common/checks.tsx";
// Qwen3-*-Instruct-2507 and Qwen3-Next-Instruct: the Qwen3 format without thinking.
"use strict_tokens";
import { Turns } from "../common/turns.tsx";
import { GeneratedTurn, ToolCall, ToolResponse, ToolsSection } from "./common.tsx";

export default function Qwen3Instruct2507({ messages, tools, add_generation_prompt }) {
  if (!messages?.length) throw new Error("Cannot apply chat template to an empty conversation. Provide at least one message.");
  const first = messages[0];
  const header = tools?.length > 0 ? p`${first.role === "system" ? (tok.im_start + "system\n") + textOperand(first.content) + "\n\n" : tok.im_start + "system\n"}${<ToolsSection tools={tools}/>}${tok.im_end}\n`
    : first.role === "system" && (tok.im_start + "system\n") + textOperand(first.content) + (tok.im_end + "\n");
  const message = (m, i) => {
    const content = typeof m.content === "string" || isFragment(m.content) ? m.content : "";
    if (m.role === "user" || (m.role === "system" && i > 0)) return tok.im_start + m.role + "\n" + content + tok.im_end + "\n";
    if (m.role === "assistant" && m.generated) return <GeneratedTurn ids={m.ids} prompt={p`${tok.im_start}assistant\n`}/>;
    if (m.role === "assistant") return <>
      {tok.im_start + m.role + "\n" + content}
      {m.tool_calls?.length > 0 && m.tool_calls.map((tc, j) => <>{(j > 0 || content) && "\n"}<ToolCall tc={tc}/></>)}
      {p`${tok.im_end}\n`}
    </>;
    if (m.role === "tool") return <ToolResponse messages={messages} i={i} content={content}/>;
  };
  return <>
    <Turns messages={messages} header={header} message={message}/>
    {add_generation_prompt && p`${tok.im_start}assistant\n`}
  </>;
}

export { Qwen3Instruct2507 };
