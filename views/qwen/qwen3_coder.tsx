import { mappingEntries } from "../common/checks.tsx";
// Qwen3-Coder: XML function calls (<function=name><parameter=k>v</parameter>).
"use strict_tokens";
import { ToolDef, isSeq } from "../common/xml_tools.tsx";
import { XML_TOOL_FORMAT } from "./common.tsx";
import { QWEN3_CODER_SYSTEM } from "../third_party/qwen/qwen.tsx";

const argText = v => isSeq(v) ? formatJSON(v) : formatPythonValue(v);

function CoderToolCalls({ m }) {
  return <>
    {tok.im_start + m.role}
    {typeof m.content === "string" && m.content.trim().length > 0 && "\n" + m.content.trim() + "\n"}
    {m.tool_calls.map(tc => {
      const f = tc.function !== undefined ? tc.function : tc;
      return <>
        {("\n" + tok.tool_call + "\n<function=") + f.name + ">\n"}
        {f.arguments !== undefined && mappingEntries(f.arguments).map(([k, v]) => p`${"<parameter=" + k + ">\n"}${argText(v)}\n</parameter>\n`)}
        {p`</function>\n${tok.tool_call_end}`}
      </>;
    })}
    {p`${tok.im_end}\n`}
  </>;
}

export default function Qwen3Coder({ messages, tools, add_generation_prompt }) {
  if (!messages?.length) throw new Error("Cannot apply chat template to an empty conversation. Provide at least one message.");
  const hasSys = messages[0].role === "system";
  const sys = hasSys ? messages[0].content : undefined;
  const rest = hasSys ? messages.slice(1) : messages;
  const hasTools = Array.isArray(tools) && tools.length > 0;
  return <>
    {hasSys ? (tok.im_start + "system\n") + sys
            : hasTools && p`${tok.im_start}system\n${QWEN3_CODER_SYSTEM}`}
    {hasTools && p`\n\n# Tools\n\nYou have access to the following functions:\n\n<tools>${tools.map(t => <ToolDef tool={t}/>)}\n</tools>${XML_TOOL_FORMAT}`}
    {(hasSys || hasTools) && p`${tok.im_end}\n`}
    {rest.map((m, i) => {
      if (m.role === "assistant" && Array.isArray(m.tool_calls) && m.tool_calls.length > 0) return <CoderToolCalls m={m}/>;
      if (m.role === "user" || m.role === "system" || m.role === "assistant") return tok.im_start + m.role + "\n" + m.content + tok.im_end + "\n";
      if (m.role === "tool") {
        const prev = rest[i - 1], next = rest[i + 1];
        return <>
          {i > 0 && prev && prev.role !== "tool" && p`${tok.im_start}user\n`}
          {p`${tok.tool_response}\n${formatPythonValue(m.content)}\n${tok.tool_response_end}\n`}
          {(i === rest.length - 1 || next.role !== "tool") && p`${tok.im_end}\n`}
        </>;
      }
      return tok.im_start + m.role + "\n" + m.content + (tok.im_end + "\n");
    })}
    {add_generation_prompt && p`${tok.im_start}assistant\n`}
  </>;
}

export { Qwen3Coder };
