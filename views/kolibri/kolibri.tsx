// Aleph Alpha Kolibri-1: Qwen3-style ChatML with a reasoning-effort system section.
"use strict_tokens";
import { HIGH_REASONING, LOW_REASONING, MEDIUM_REASONING, NO_REASONING } from "../third_party/aleph-alpha/kolibri.tsx";
import { ToolCall, ToolResponse, ToolsSection } from "../qwen/common.tsx";


function last_query_index(messages) {
  for (let i = messages.length - 1; i >= 0; i--) {
    const m = messages[i];
    if (m.role === "user" && typeof m.content === "string"
        && !(m.content.startsWith(tok.tool_response.text) && m.content.endsWith(tok.tool_response_end.text))) {
      return i;
    }
  }
  return messages.length - 1;
}


export default function Kolibri({ messages, tools, add_generation_prompt, reasoning_effort, enable_thinking, preserve_thinking }) {
  if (messages.length === 0) throw new Error("Cannot apply chat template to an empty conversation. Provide at least one message.");
  const effort = reasoning_effort === undefined ? null : reasoning_effort;
  const disabled = effort === null ? enable_thinking === false : effort === "none";
  const sentence = disabled ? NO_REASONING : ["minimal", "low"].includes(effort) ? LOW_REASONING : effort === "medium" ? MEDIUM_REASONING : HIGH_REASONING;
  const hasSys = messages.length > 0 && messages[0].role === "system";
  const lqi = last_query_index(messages);
  return <>
    {p`${tok.im_start}system\n${hasSys && formatPythonValue(messages[0].content) + "\n\n"}${"# Reasoning effort\n\n" + sentence}${tools?.length > 0 && ["\n\n", <ToolsSection tools={tools}/>]}${tok.im_end}\n`}
    {messages.map((m, i) => {
      if (m.role === "user" || (m.role === "system" && i > 0)) return tok.im_start + m.role + "\n" + m.content + tok.im_end + "\n";
      if (m.role === "assistant") {
        let content = typeof m.content === "string" ? m.content : "";
        let reasoning = null;
        if (m.reasoning !== undefined && m.reasoning !== null) reasoning = m.reasoning;
        else if (m.reasoning_content !== undefined && m.reasoning_content !== null) reasoning = m.reasoning_content;
        else if (m.thinking !== undefined && m.thinking !== null) reasoning = m.thinking;
        else if (content.includes(tok.think_end.text)) {
          reasoning = trimStartChars(trimEndChars(content.split(tok.think_end.text)[0], "\n").split(tok.think.text).at(-1), "\n");
          content = trimStartChars(content.split(tok.think_end.text).at(-1), "\n");
        }
        return <>
          {tok.im_start + m.role + "\n"}
          {(i > lqi || preserve_thinking === true) && p`${tok.think}\n${reasoning !== null && reasoning.trim() && trimChars(reasoning, "\n")}\n${tok.think_end}\n\n`}
          {trimStartChars(content, "\n")}
          {m.tool_calls?.length > 0 && m.tool_calls.map((tc, j) => <>{(j > 0 || content) && "\n"}<ToolCall tc={tc}/></>)}
          {p`${tok.im_end}\n`}
        </>;
      }
      if (m.role === "tool") return <ToolResponse messages={messages} i={i} content={m.content}/>;
    })}
    {add_generation_prompt && p`${tok.im_start}assistant\n${disabled && (tok.think + "\n\n" + tok.think_end + "\n\n")}`}
  </>;
}

export { Kolibri };
