// Mistral-7B-Instruct-v0.3 (v3=true) and Mistral-Nemo-Instruct-2407 (v3=false).
// Tool-call ids must be 9 characters; roles must alternate (both enforced like the templates).
"use strict_tokens";
import { templateLength, textOperand } from "../common/checks.tsx";

// Jinja `|length`: Undefined has length 0; None (or a number) is a TypeError.

// The template's `content + eos_token` is Python `str + str`: a non-string content is a
// TypeError there, so call a string-only method (a TypeError on lists, null, numbers).

const hasCalls = m => m.tool_calls !== undefined && m.tool_calls !== null;

function check_alternation(msgs) {
  let i = 0;
  for (const m of msgs) {
    if (!(m.role === "tool" || m.role === "tool_results" || hasCalls(m))) {
      if ((m.role === "user") !== (i % 2 === 0)) {
        throw new Error("After the optional system message, conversation roles must alternate user/assistant/user/assistant/...");
      }
      i = i + 1;
    }
  }
}

// The template compares messages by value (`message == user_messages[-1]`), not identity. Laid-out
// content (a template app's screen) has no JSON value: it is the same message or not.
const sameValue = (a, b) => isFragment(a.content) || isFragment(b.content) ? a === b
  : formatJSON(a, {sortKeys: true}) === formatJSON(b, {sortKeys: true});

const AvailableTools = ({ tools, sp }) => p`${tok.AVAILABLE_TOOLS}${sp}[${tools.map(t =>
  "{\"type\": \"function\", \"function\": {" + Object.entries(t.function).filter(([k, v]) => k !== "return").map(([k, v]) =>
    typeof v === "string" ? "\"" + k + "\": \"" + v + "\"" : "\"" + k + "\": " + formatJSON(v)).join(", ") + "}}").join(", ")}${tools.length > 0 ? "]" : ""}${tok.AVAILABLE_TOOLS_end}`;

export default function Mistral({ messages, tools, bos_token, eos_token, v3 = true }) {
  if (messages.length === 0) throw new Error("Cannot apply chat template to an empty conversation. Provide at least one message.");
  const sp = v3 ? " " : "";
  const hasSys = messages[0].role === "system";
  const sys = hasSys ? messages[0].content : undefined;
  const msgs = hasSys ? messages.slice(1) : messages;
  const userMsgs = msgs.filter(m => m.role === "user");
  check_alternation(msgs);
  return [bos_token, msgs.map((m, i) => {
    // A model turn logged with exact ids: what the model wrote after [/INST], verbatim.
    if (m.role === "assistant" && m.generated) return [tokens(m.ids), m.ids.at(-1) === tok.s_end.id ? "" : eos_token];
    if (m.role === "user") return <>
      {tools !== null && tools !== undefined && sameValue(m, userMsgs.at(-1)) && <AvailableTools tools={tools} sp={sp}/>}
      {i === msgs.length - 1 && sys !== undefined ? tok.INST + sp + sys + "\n\n" + m.content + tok.INST_end : tok.INST + sp + m.content + tok.INST_end}
    </>;
    if (hasCalls(m)) return [tok.TOOL_CALLS + sp + "[", m.tool_calls.map(tc => {
      if (templateLength(tc.id) !== 9) throw new Error("Tool call IDs should be alphanumeric strings with length 9!");
      return formatJSON(tc.function).slice(0, -1) + ", \"id\": \"" + tc.id + "\"}";
    }).join(", "), m.tool_calls.length > 0 ? "]" + eos_token : ""];
    if (m.role === "assistant") return v3 ? " " + formatPythonValue(m.content).trim() + eos_token : textOperand(m.content) + eos_token;
    if (m.role === "tool_results" || m.role === "tool") {
      const content = m.content?.content !== undefined ? m.content.content : m.content;
      if (templateLength(m.tool_call_id) !== 9) throw new Error("Tool call IDs should be alphanumeric strings with length 9!");
      return tok.TOOL_RESULTS + sp + "{\"content\": " + formatPythonValue(content) + ", " + "\"call_id\": \"" + m.tool_call_id + ("\"}" + tok.TOOL_RESULTS_end);
    }
    throw new Error("Only user and assistant roles are supported, with the exception of an initial optional system message!");
  })];
}

export const MistralNemo = state => Mistral({...state, v3: false});

export { Mistral };
