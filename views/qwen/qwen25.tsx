// Qwen2.5-Instruct / Qwen2.5-Coder-Instruct, QwQ-32B (qwq) and MiMo-7B-RL (other default system).
"use strict_tokens";
import { GeneratedTurn, ToolCall, ToolResponse, ToolsSection } from "./common.tsx";

export default function Qwen25({ messages, tools, add_generation_prompt, qwq = false,
                                 default_sys = "You are Qwen, created by Alibaba Cloud. You are a helpful assistant." }) {
  if (!messages?.length) throw new Error("Cannot apply chat template to an empty conversation. Provide at least one message.");
  const sys = qwq ? "" : default_sys;
  const first = messages[0];
  return <>
    {tools?.length > 0 ? p`${tok.im_start}system\n${first.role === "system" ? formatPythonValue(first.content) : sys}\n\n${<ToolsSection tools={tools}/>}${tok.im_end}\n`
      : first.role === "system" ? (tok.im_start + "system\n") + first.content + (tok.im_end + "\n")
      : !qwq && p`${tok.im_start}system\n${sys}${tok.im_end}\n`}
    {messages.map((m, i) => {
      if (m.role === "assistant" && m.generated) return <GeneratedTurn ids={m.ids} prompt={p`${tok.im_start}assistant\n${qwq ? (tok.think + "\n") : ""}`}/>;
      if (m.role === "user" || (m.role === "system" && i > 0) || (!qwq && m.role === "assistant" && !m.tool_calls?.length)) {
        return tok.im_start + m.role + "\n" + m.content + tok.im_end + "\n";
      }
      if (m.role === "assistant") {
        const content = qwq && i < messages.length - 1 ? trimStartChars(m.content.split(tok.think_end.text).at(-1), "\n") : m.content;
        if (qwq && !m.tool_calls?.length) return tok.im_start + m.role + "\n" + content + tok.im_end + "\n";
        return <>
          {tok.im_start + m.role}
          {m.content && "\n" + content}
          {(m.tool_calls || []).map(tc => <>{"\n"}<ToolCall tc={tc} strargs={false}/></>)}
          {p`${tok.im_end}\n`}
        </>;
      }
      if (m.role === "tool") return <ToolResponse messages={messages} i={i} content={m.content}/>;
    })}
    {add_generation_prompt && p`${tok.im_start}assistant\n${qwq ? (tok.think + "\n") : ""}`}
  </>;
}

export const QwQ = state => Qwen25({...state, qwq: true});
export const MiMo7B = state => Qwen25({...state, default_sys: "You are a helpful assistant."});

export { Qwen25 };
