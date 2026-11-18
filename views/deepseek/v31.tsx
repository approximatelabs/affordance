// DeepSeek-V3.1 / V3.1-Terminus. The template's namespace flags are plain locals.
"use strict_tokens";
import { textOperand } from "../common/checks.tsx";


const Call = ({ c }) => tok.tool_call_begin + textOperand(c.function.name) + tok.tool_sep + textOperand(c.function.arguments) + tok.tool_call_end;

// Python `s.split(sep, 1)[1]`: everything after the first `sep`.
const afterFirst = (s, sep) => s.slice(s.indexOf(sep) + sep.length);

export default function DeepSeekV31({ messages, bos_token, add_generation_prompt, thinking = false }) {
  const system = messages.filter(m => m.role === "system").map(m => textOperand(m.content));
  let out = [bos_token, system.join("\n\n")];
  let afterUser = false, afterTool = false;
  for (const m of messages) {
    // A model turn logged with exact ids: the generation prompt it followed (the Assistant tag
    // after a user turn; nothing after tool output), then the ids and the turn end.
    if (m.role === "assistant" && m.generated) {
      out.push(afterUser && !afterTool && p`${tok.Assistant}${thinking ? tok.think : tok.think_end}`, tokens(m.ids), m.ids.at(-1) === tok.end_of_sentence.id ? "" : tok.end_of_sentence);
      [afterUser, afterTool] = [false, false];
      continue;
    }
    if (m.role === "user") {
      out.push(tok.User + textOperand(m.content));
      [afterUser, afterTool] = [true, false];
    } else if (m.role === "assistant" && m.tool_calls !== undefined && m.tool_calls !== null) {
      out.push(afterUser && p`${tok.Assistant}${tok.think_end}`,
               m.tool_calls.length > 0 && [m.content === null ? "" : textOperand(m.content), p`${tok.tool_calls_begin}`],
               m.tool_calls.map(c => <Call c={c}/>),
               p`${tok.tool_calls_end}${tok.end_of_sentence}`);
      [afterUser, afterTool] = [false, false];
    } else if (m.role === "assistant") {
      const body = afterTool || !m.content.includes(tok.think_end.text) ? m.content : afterFirst(m.content, tok.think_end.text);
      out.push(afterUser && p`${tok.Assistant}${m.prefix && thinking ? tok.think : tok.think_end}`,
               p`${textOperand(body)}${tok.end_of_sentence}`);
      [afterUser, afterTool] = [false, false];
    } else if (m.role === "tool") {
      out.push(p`${tok.tool_output_begin}${textOperand(m.content)}${tok.tool_output_end}`);
      [afterUser, afterTool] = [false, true];
    }
  }
  if (add_generation_prompt && afterUser && !afterTool) out.push(p`${tok.Assistant}${thinking ? tok.think : tok.think_end}`);
  return out;
}

export { DeepSeekV31 };
