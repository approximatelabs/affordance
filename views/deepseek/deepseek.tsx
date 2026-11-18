import { templateItems, textOperand } from "../common/checks.tsx";
// DeepSeek chat formats from deepseek-llm (2023) to V3.1 (2025).
// Faithful to the shipped templates, including their quirks (V2.5/V3 drop tool calls when the
// content is a string and never reset `is_first` between messages). The templates' namespace
// flags are plain locals in a for..of loop.
"use strict_tokens";

// A model turn logged with exact ids (a template app's `generated` turn): what the generation
// prompt was at that point (`prompt`), then the ids verbatim, then the turn end unless the ids
// already have it. Each template below says what its prompt was and updates its own flags as it
// would after an assistant turn.
function generatedTurn(m, prompt, end = tok.end_of_sentence) {
  return [prompt, tokens(m.ids), m.ids.at(-1) === end.id ? "" : end];
}

export function DeepSeekLLM({ messages, bos_token, eos_token, add_generation_prompt }) {
  return <>
    {bos_token ?? ""}
    {messages.map(m => m.role === "assistant" && m.generated ? generatedTurn(m, "Assistant:")
                     : m.role === "user" ? "User: " + textOperand(m.content) + "\n\n"
                     : m.role === "assistant" ? "Assistant: " + textOperand(m.content) + eos_token
                     : m.role === "system" && textOperand(m.content) + "\n\n")}
    {add_generation_prompt && "Assistant:"}
  </>;
}


// System prompts: V3+ join them with a blank line; V2.5 keeps the last one.
function systemPrompt(messages, join) {
  const sys = messages.filter(m => m.role === "system").map(m => m.content);
  if (!join) return sys.length ? formatPythonValue(sys.at(-1)) : "";
  let out = "";
  for (const [i, s] of sys.entries()) out = i === 0 ? out + textOperand(s) : out + "\n\n" + textOperand(s);
  return out;
}

// The V3-style call: type, name, then the arguments in a ```json fence.
const callV3 = t => textOperand(t.type) + tok.tool_sep + textOperand(t.function.name) + "\n" + "```json" + "\n" + textOperand(t.function.arguments) + "\n" + "```" + tok.tool_call_end;

// DeepSeek-V2.5 (join=false) and DeepSeek-V3 (join=true).
export function DeepSeekV25({ messages, bos_token, add_generation_prompt, join = false }) {
  let out = [bos_token ?? "", systemPrompt(messages, join)];
  let isFirst = false, isTool = false, outputFirst = true;
  for (const m of messages) {
    if (m.role === "assistant" && m.generated) {
      out.push(generatedTurn(m, isTool ? tok.tool_outputs_end : tok.Assistant));
      isTool = false;
      if (m.tool_calls?.length) isFirst = true;
      continue;
    }
    if (m.role === "user") { isTool = false; out.push(tok.User + textOperand(m.content)); }
    if (m.role === "assistant" && m.content === null) {
      isTool = false;
      for (const t of templateItems(m.tool_calls)) {
        if (!isFirst) { out.push((tok.Assistant + tok.tool_calls_begin + tok.tool_call_begin) + callV3(t)); isFirst = true; }
        else out.push("\n" + tok.tool_call_begin + callV3(t), p`${tok.tool_calls_end}${tok.end_of_sentence}`);
      }
    }
    if (m.role === "assistant" && m.content !== null) {
      if (isTool) { out.push(tok.tool_outputs_end + textOperand(m.content) + tok.end_of_sentence); isTool = false; }
      else out.push(tok.Assistant + textOperand(m.content) + tok.end_of_sentence);
    }
    if (m.role === "tool") {
      isTool = true;
      if (outputFirst) { out.push((tok.tool_outputs_begin + tok.tool_output_begin) + textOperand(m.content) + tok.tool_output_end); outputFirst = false; }
      else out.push(("\n" + tok.tool_output_begin) + textOperand(m.content) + tok.tool_output_end);
    }
  }
  out.push(isTool && p`${tok.tool_outputs_end}`, add_generation_prompt && !isTool && p`${tok.Assistant}`);
  return out;
}

export const DeepSeekV3 = state => DeepSeekV25({...state, join: true});

// DeepSeek-R1 (Jan 2025).
export function DeepSeekR1({ messages, bos_token, add_generation_prompt }) {
  let out = [bos_token ?? "", systemPrompt(messages, true)];
  let isFirst = false, isTool = false, outputFirst = true;
  for (const m of messages) {
    if (m.role === "assistant" && m.generated) {
      out.push(generatedTurn(m, isTool ? tok.tool_outputs_end : p`${tok.Assistant}${tok.think}\n`));
      isTool = false;
      if (m.tool_calls?.length) isFirst = true;
      continue;
    }
    if (m.role === "user") { isTool = false; out.push(tok.User + textOperand(m.content)); }
    if (m.role === "assistant" && "tool_calls" in m) {
      isTool = false;
      for (const t of m.tool_calls) {
        if (!isFirst) {
          out.push(m.content === null ? (tok.Assistant + tok.tool_calls_begin + tok.tool_call_begin) + callV3(t)
                                      : tok.Assistant + textOperand(m.content) + (tok.tool_calls_begin + tok.tool_call_begin) + callV3(t));
          isFirst = true;
        } else out.push("\n" + tok.tool_call_begin + callV3(t));
      }
      out.push(p`${tok.tool_calls_end}${tok.end_of_sentence}`);
    }
    if (m.role === "assistant" && !("tool_calls" in m)) {
      if (isTool) { out.push(tok.tool_outputs_end + textOperand(m.content) + tok.end_of_sentence); isTool = false; }
      else {
        const content = m.content.includes(tok.think_end.text) ? m.content.split(tok.think_end.text).at(-1) : m.content;
        out.push(tok.Assistant + textOperand(content) + tok.end_of_sentence);
      }
    }
    if (m.role === "tool") {
      isTool = true;
      if (outputFirst) { out.push((tok.tool_outputs_begin + tok.tool_output_begin) + textOperand(m.content) + tok.tool_output_end); outputFirst = false; }
      else out.push(tok.tool_output_begin + textOperand(m.content) + tok.tool_output_end);
    }
  }
  out.push(isTool && p`${tok.tool_outputs_end}`, add_generation_prompt && !isTool && p`${tok.Assistant}${tok.think}\n`);
  return out;
}

// DeepSeek-V3-0324 (strip_think=false) and DeepSeek-R1-0528 (strip_think=true).
export function DeepSeekV3_0324({ messages, bos_token, add_generation_prompt, strip_think = false }) {
  let out = [bos_token ?? "", systemPrompt(messages, true)];
  let isFirst = false, isTool = false, outputFirst = true, lastUser = false;
  for (const m of messages) {
    let content = m.content;
    if (m.role === "assistant" && m.generated) {
      out.push(generatedTurn(m, isTool ? tok.tool_outputs_end : lastUser ? "" : tok.Assistant));
      if (m.tool_calls?.length) [isFirst, isTool, outputFirst] = [true, false, true];
      [isTool, lastUser] = [false, false];
      continue;
    }
    if (m.role === "user") { [isTool, isFirst, lastUser] = [false, false, true]; out.push(tok.User + textOperand(content) + tok.Assistant); }
    if (strip_think && m.role === "assistant" && content.includes(tok.think_end.text)) content = content.split(tok.think_end.text).at(-1);
    if (m.role === "assistant" && m.tool_calls !== undefined && m.tool_calls !== null) {
      lastUser = false;
      out.push(isTool && p`${tok.tool_outputs_end}`);
      [isFirst, isTool, outputFirst] = [false, false, true];
      for (const t of m.tool_calls) {
        if (!isFirst) {
          out.push(content === null ? (tok.tool_calls_begin + tok.tool_call_begin) + callV3(t)
                                    : textOperand(content) + (tok.tool_calls_begin + tok.tool_call_begin) + callV3(t));
          isFirst = true;
        } else out.push("\n" + tok.tool_call_begin + callV3(t));
      }
      out.push(p`${tok.tool_calls_end}${tok.end_of_sentence}`);
    }
    if (m.role === "assistant" && (m.tool_calls === undefined || m.tool_calls === null)) {
      lastUser = false;
      if (isTool) { out.push(tok.tool_outputs_end + textOperand(content) + tok.end_of_sentence); isTool = false; }
      else out.push(textOperand(content) + tok.end_of_sentence);
    }
    if (m.role === "tool") {
      [lastUser, isTool] = [false, true];
      if (outputFirst) { out.push((tok.tool_outputs_begin + tok.tool_output_begin) + textOperand(content) + tok.tool_output_end); outputFirst = false; }
      else out.push(("\n" + tok.tool_output_begin) + textOperand(content) + tok.tool_output_end);
    }
  }
  out.push(isTool && p`${tok.tool_outputs_end}`, add_generation_prompt && !lastUser && !isTool && p`${tok.Assistant}`);
  return out;
}

export const DeepSeekR1_0528 = state => DeepSeekV3_0324({...state, strip_think: true});
