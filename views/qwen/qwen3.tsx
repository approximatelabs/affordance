import { textOperand } from "../common/checks.tsx";
// Qwen3 (hybrid thinking) and Qwen3-*-Thinking-2507 (generation prompt always opens <think>).
"use strict_tokens";
import { Turns } from "../common/turns.tsx";
import { GeneratedTurn, ToolCall, ToolResponse, ToolsSection } from "./common.tsx";

// Reasoning comes from reasoning_content, or from a <think>...</think> prefix in the content.
function splitThink(m, content) {
  if (typeof m.reasoning_content === "string") return [content, m.reasoning_content];
  if (typeof content === "string" && content.includes(tok.think_end.text)) {
    return [trimStartChars(content.split(tok.think_end.text).at(-1), "\n"),
            trimStartChars(trimEndChars(content.split(tok.think_end.text)[0], "\n").split(tok.think.text).at(-1), "\n")];
  }
  return [content, ""];
}

function Assistant({ m, showThinking }) {
  const [content, reasoning] = splitThink(m, typeof m.content === "string" || isFragment(m.content) ? m.content : "");
  return <>
    {showThinking(reasoning)
      ? tok.im_start + m.role + ("\n" + tok.think + "\n") + trimChars(reasoning, "\n") + ("\n" + tok.think_end + "\n\n") + trimStartChars(content, "\n")
      : tok.im_start + m.role + "\n" + content}
    {m.tool_calls?.length > 0 && m.tool_calls.map((c, i) => <>{(i > 0 || content) && "\n"}<ToolCall tc={c}/></>)}
    {p`${tok.im_end}\n`}
  </>;
}

// Index of the last real user query (not a wrapped tool response).
function lastQueryIndex(messages) {
  for (let i = messages.length - 1; i >= 0; i--) {
    const c = messages[i].content;
    if (messages[i].role === "user" && isFragment(c)) return i;
    if (messages[i].role === "user" && typeof c === "string" && !(c.startsWith(tok.tool_response.text) && c.endsWith(tok.tool_response_end.text))) return i;
  }
  return messages.length - 1;
}

export default function Qwen3({ messages, tools, add_generation_prompt, enable_thinking, thinking_2507 = false }) {
  if (!messages?.length) throw new Error("Cannot apply chat template to an empty conversation. Provide at least one message.");
  const first = messages[0];
  const lastQuery = lastQueryIndex(messages);
  const header = tools?.length > 0 ? p`${first.role === "system" ? (tok.im_start + "system\n") + textOperand(first.content) + "\n\n" : tok.im_start + "system\n"}${<ToolsSection tools={tools}/>}${tok.im_end}\n`
           : first.role === "system" && (tok.im_start + "system\n") + textOperand(first.content) + (tok.im_end + "\n");
  return <>
    <Turns messages={messages} header={header} message={(m, i) => {
      const content = typeof m.content === "string" || isFragment(m.content) ? m.content : "";
      switch (m.role) {
        case "user": return tok.im_start + m.role + "\n" + content + tok.im_end + "\n";
        case "system": return i > 0 && tok.im_start + m.role + "\n" + content + tok.im_end + "\n";
        case "assistant": return m.generated
          ? <GeneratedTurn ids={m.ids} prompt={p`${tok.im_start}assistant\n${thinking_2507 ? (tok.think + "\n") : enable_thinking === false ? (tok.think + "\n\n" + tok.think_end + "\n\n") : ""}`}/>
          : <Assistant m={m} showThinking={r => i > lastQuery && (i === messages.length - 1 || r)}/>;
        case "tool": return <ToolResponse messages={messages} i={i} content={content}/>;
      }
    }}/>
    {add_generation_prompt && p`${tok.im_start}assistant\n${thinking_2507 ? (tok.think + "\n") : enable_thinking === false ? (tok.think + "\n\n" + tok.think_end + "\n\n") : ""}`}
  </>;
}

export const Qwen3Thinking2507 = state => Qwen3({...state, thinking_2507: true});

export { Qwen3 };
