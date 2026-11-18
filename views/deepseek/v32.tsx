// DeepSeek-V3.2 (port of encoding/encoding_dsv32.py; the repo ships no Jinja template).
// State adapter (same as ref/render_ref_pyenc.py): tools attach to the system message,
// thinking_mode = "thinking" if `thinking` or `enable_thinking`, add_generation_prompt is not used.
"use strict_tokens";
import { DEEPSEEK_V32_TOOLS_HEADER } from "../third_party/deepseek-ai/deepseek.tsx";
import { mappingEntries, templateTruthy } from "../common/checks.tsx";

const DSML = tok.DSML;



function adapt_messages(messages, tools) {
  if (!templateTruthy(tools)) { return messages; }
  if (messages.length > 0 && messages[0].role === "system") { return [{...messages[0], tools: tools}, ...messages.slice(1)]; }
  return [{role: "system", content: "", tools: tools}, ...messages];
}

function last_user_index(msgs) {
  for (let i = msgs.length - 1; i >= 0; i--) { if (["user", "developer"].includes(msgs[i].role)) { return i; } }
  return -1;
}

function render_tools_v32(tools) {
  const schemas = tools.map(t => formatJSON(t.function));
  return DEEPSEEK_V32_TOOLS_HEADER + schemas.join("\n") + "\n</functions>\n";
}

function dsml_params(argStr) {
  const lines = mappingEntries(JSON.parse(argStr)).map(([k, v]) =>
    "<" + DSML + "parameter name=\"" + k + "\" string=\"" + (typeof v === "string" ? "true" : "false") + "\">" + (typeof v === "string" ? v : formatJSON(v)) + "</" + DSML + "parameter>");
  return lines.join("\n");
}

function response_format(m) {
  return "\n\n## Response Format:\n\nYou MUST strictly adhere to the following schema to reply:\n" + formatJSON(m.response_format);
}


const dsmlCalls = (m, block) => {
  const calls = m.tool_calls.map(tc => "<" + DSML + "invoke name=\"" + tc.function.name + "\">\n" + dsml_params(tc.function.arguments) + "\n</" + DSML + "invoke>");
  return "\n\n<" + DSML + block + ">\n" + calls.join("\n") + "\n</" + DSML + block + ">";
};

export default function DeepSeekV32({ messages, tools, thinking, enable_thinking }) {
  const think = templateTruthy(thinking) || templateTruthy(enable_thinking);
  let msgs = adapt_messages(messages, tools);
  const lu = last_user_index(msgs);
  if (think) {
    // drop_thinking: earlier assistant turns lose reasoning, earlier developer turns are dropped
    let kept = [];
    for (const [i, m] of msgs.entries()) {
      if (["user", "system", "tool"].includes(m.role) || i >= lu) kept.push(m);
      else if (m.role === "assistant") kept.push(Object.fromEntries(Object.entries(m).filter(([k]) => k !== "reasoning_content")));
    }
    msgs = kept;
  }
  let out = [p`${tok.begin_of_sentence}`];
  for (const [i, m] of msgs.entries()) {
    const content = m.content ?? null;
    if (m.role === "system") {
      out.push(formatPythonValue(templateTruthy(content) ? content : ""), templateTruthy(m.tools) && "\n\n" + render_tools_v32(m.tools), templateTruthy(m.response_format) && response_format(m));
    } else if (m.role === "developer") {
      if (!templateTruthy(content)) throw new Error("Invalid message for role `" + m.role + "`: " + formatPythonValue(m));
      const dev = (templateTruthy(m.tools) ? "\n\n" + render_tools_v32(m.tools) : "") + (templateTruthy(m.response_format) ? response_format(m) : "");
      out.push(tok.User + dev + "\n\n# The user's message is: " + formatPythonValue(content) + tok.Assistant, i === lu && think ? tok.think : tok.think_end);
    } else if (m.role === "user") {
      out.push(tok.User + formatPythonValue(content) + tok.Assistant, i === lu && think ? tok.think : tok.think_end);
    } else if (m.role === "tool") {
      let pi = i - 1;
      while (pi >= 0 && msgs.at(pi).role === "tool") pi -= 1;
      const a = msgs.at(pi);
      if (!(i === 0 || (pi >= 0 && a.role === "assistant"))) throw new Error("Invalid messages");
      const order = i - pi;
      if (!(templateTruthy(a.tool_calls) && a.tool_calls.length >= order)) throw new Error("No tool calls but found tool output");
      out.push(order === 1 && "\n\n<function_results>", "\n<result>" + formatPythonValue(content) + "</result>",
               order === a.tool_calls.length && "\n</function_results>" + (i >= lu && think ? ("\n\n" + tok.think) : ("\n\n" + tok.think_end)));
    } else if (m.role === "assistant" && m.generated) {
      // A model turn logged with exact ids: the turn before it already wrote the generation prompt
      // (Assistant and the think tag), so the ids follow verbatim, then the end unless they have it.
      out.push(tokens(m.ids), m.ids.at(-1) === tok.end_of_sentence.id ? "" : p`${tok.end_of_sentence}`);
    } else if (m.role === "assistant") {
      if (think && i > lu) {
        if (!(templateTruthy(m.reasoning_content) || templateTruthy(m.tool_calls))) throw new Error("ThinkingMode: thinking, invalid message without reasoning_content/tool_calls `" + formatPythonValue(m) + "` after last user message");
        out.push(formatPythonValue(templateTruthy(m.reasoning_content) ? m.reasoning_content : "") + tok.think_end);
      }
      out.push(formatPythonValue(templateTruthy(content) ? content : ""), templateTruthy(m.tool_calls) && dsmlCalls(m, "function_calls"), p`${tok.end_of_sentence}`);
    } else {
      throw new Error("Unknown role: " + formatPythonValue(m.role ?? null));
    }
  }
  return out;
}

export { DeepSeekV32 };
