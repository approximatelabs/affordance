// Meta Llama 2, 3, 3.1/3.3, 3.2 and 4 chat formats.
"use strict_tokens";
import { templateItems, textOperand } from "../common/checks.tsx";
import { Turns } from "../common/turns.tsx";
import { LLAMA31_TOOLS_IN_SYSTEM, LLAMA31_TOOLS_IN_USER, LLAMA4_TOOL_SYSTEM, LLAMA4_TOOLS_IN_USER_SYSTEM } from "../third_party/meta-llama/llama.tsx";

// Jinja `| trim`: non-strings print as Python str() first.
const pyTrim = c => (typeof c === "string" || isFragment(c) ? c : formatPythonValue(c)).trim();

// An operand of the template's Python `str + x`: a non-string is a TypeError there, so call a
// string-only method (a TypeError on lists, null, numbers).

export function Llama2({ messages, bos_token, eos_token }) {
  if (messages.length === 0) throw new Error("Cannot apply chat template to an empty conversation. Provide at least one message.");
  const hasSys = messages[0].role === "system";
  const sys = hasSys ? messages[0].content : false;
  const rest = hasSys ? messages.slice(1) : messages;
  return rest.map((m, i) => {
    if ((m.role === "user") !== (i % 2 === 0)) throw new Error("Conversation roles must alternate user/assistant/user/assistant/...");
    // A model turn logged with exact ids: what the model wrote after " [/INST]", verbatim.
    if (m.role === "assistant" && m.generated) return [tokens(m.ids), m.ids.at(-1) === tok.s_end.id ? "" : eos_token];
    const content = i === 0 && sys !== false ? "<<SYS>>\n" + textOperand(sys) + "\n<</SYS>>\n\n" + textOperand(m.content) : m.content;
    if (m.role === "user") return bos_token + textOrToken("[INST]") + " " + content.trim() + " " + textOrToken("[/INST]");
    if (m.role === "assistant") return " " + content.trim() + " " + eos_token;
  });
}

export function Llama3({ messages, bos_token, add_generation_prompt }) {
  if (messages.length === 0) throw new Error("Cannot apply chat template to an empty conversation. Provide at least one message.");
  return <>
    {messages.map((m, i) => m.role === "assistant" && m.generated
      ? [p`${tok.start_header_id}assistant${tok.end_header_id}\n\n`, tokens(m.ids), m.ids.at(-1) === tok.eot_id.id ? "" : p`${tok.eot_id}`]
      : <>{i === 0 && bos_token}{tok.start_header_id + m.role + (tok.end_header_id + "\n\n") + pyTrim(m.content) + tok.eot_id}</>)}
    {add_generation_prompt && p`${tok.start_header_id}assistant${tok.end_header_id}\n\n`}
  </>;
}

const ToolList = ({ tools }) => tools.map(t => formatJSON(t, {indent: 4}) + "\n\n");

// Llama 3.1 / 3.3 (v=31) and 3.2 (v=32: date from the clock, no builtin tools).
export function Llama31(state) {
  const { messages, bos_token, add_generation_prompt, v = 31 } = state;
  if (messages.length === 0) throw new Error("Cannot apply chat template to an empty conversation. Provide at least one message.");
  const tls = state.custom_tools !== undefined ? state.custom_tools : (state.tools !== undefined ? state.tools : null);
  const inUser = state.tools_in_user_message !== undefined ? state.tools_in_user_message : true;
  const date = state.date_string !== undefined ? state.date_string : (v === 32 ? formatNow("%d %b %Y") : "26 Jul 2024");
  const builtin = v === 31 && state.builtin_tools !== undefined;
  let msgs = messages;
  let sys = "";
  if (messages[0].role === "system") { sys = pyTrim(messages[0].content); msgs = messages.slice(1); }
  let out = [bos_token, p`${tok.start_header_id}system${tok.end_header_id}\n\n`,
             (builtin || tls !== null) && "Environment: ipython\n",
             builtin && "Tools: " + [...state.builtin_tools].filter(x => x !== "code_interpreter").join(", ") + "\n\n",
             "Cutting Knowledge Date: December 2023\n", "Today Date: " + date + "\n\n"];
  if (tls !== null && !inUser) {
    out.push(LLAMA31_TOOLS_IN_SYSTEM, <ToolList tools={tls}/>);
  }
  out.push(asContent(sys), p`${tok.eot_id}`);
  if (inUser && tls !== null) {
    if (msgs.length === 0) throw new Error("Cannot put tools in the first user message when there's no first user message!");
    const first = pyTrim(msgs[0].content);
    msgs = msgs.slice(1);
    out.push(p`${tok.start_header_id}user${tok.end_header_id}\n\n${LLAMA31_TOOLS_IN_USER}`,
             <ToolList tools={tls}/>, first + tok.eot_id);
  }
  const message = m => {
    const parts = [];
    // A model turn logged with exact ids (TurnStrategy `model` record with `ids`).
    if (m.role === "assistant" && m.generated) {
      return [p`${tok.start_header_id}assistant${tok.end_header_id}\n\n`, tokens(m.ids), [tok.eot_id.id, tok.eom_id.id].includes(m.ids.at(-1)) ? "" : p`${tok.eot_id}`];
    }
    if (!(m.role === "ipython" || m.role === "tool" || "tool_calls" in m)) {
      parts.push(tok.start_header_id + m.role + (tok.end_header_id + "\n\n") + pyTrim(m.content) + tok.eot_id);
    } else if ("tool_calls" in m) {
      if (!(m.tool_calls.length === 1)) throw new Error("This model only supports single tool-calls at once!");
      const tc = m.tool_calls[0].function;
      parts.push(p`${tok.start_header_id}assistant${tok.end_header_id}\n\n`);
      if (builtin && state.builtin_tools.includes(tc.name)) {
        parts.push(tok.python_tag + tc.name + ".call(", Object.entries(tc.arguments).map(([k, val]) => k + "=\"" + textOperand(val) + "\"").join(", "), ")");
      } else {
        parts.push("{\"name\": \"" + tc.name + "\", ", "\"parameters\": ", formatJSON(tc.arguments), "}");
      }
      parts.push(builtin ? p`${tok.eom_id}` : p`${tok.eot_id}`);
    } else if (m.role === "tool" || m.role === "ipython") {
      parts.push(p`${tok.start_header_id}ipython${tok.end_header_id}\n\n`,
               isFragment(m.content) ? m.content : typeof m.content === "string" || Array.isArray(m.content) || isPlainObject(m.content) ? formatJSON(m.content) : formatPythonValue(m.content), p`${tok.eot_id}`);
    }
    return parts;
  };
  // The system header holds the tools and, with tools in the user message, the first user turn.
  return [<Turns messages={msgs} header={out} message={message} joins={() => false} start={0}/>,
          add_generation_prompt && p`${tok.start_header_id}assistant${tok.end_header_id}\n\n`];
}

export const Llama32 = state => Llama31({...state, v: 32});


const contentParts = (content, trimText) => typeof content === "string" || isFragment(content) ? content
  : content.map(c => c.type === "image" ? tok.image : c.type === "text" && (trimText ? pyTrim(c.text) : c.text));

export function Llama4(state) {
  const { messages, bos_token, add_generation_prompt } = state;
  if (messages.length === 0) throw new Error("Cannot apply chat template to an empty conversation. Provide at least one message.");
  let tls = state.custom_tools?.length > 0 ? state.custom_tools : state.tools;
  let toolDef = undefined;
  if (tls?.length > 0) toolDef = formatJSON(tls, {indent: 4}); else tls = null;
  let userSys = false, sys = "", msgs = messages;
  if (messages[0].role === "system") {
    userSys = true;
    sys = typeof messages[0].content === "string" ? pyTrim(messages[0].content) : pyTrim(messages[0].content[0].text);
    msgs = messages.slice(1);
  } else if (tls !== null) {
    sys = LLAMA4_TOOL_SYSTEM;
  }
  let out = [bos_token];
  if (sys) {
    out.push(p`${tok.header_start}system${tok.header_end}\n\n${sys}`,
             userSys && tls ? [LLAMA4_TOOLS_IN_USER_SYSTEM, toolDef]
                            : toolDef !== undefined && toolDef,
             p`${tok.eot}`);
  }
  for (const m of msgs) {
    // A model turn logged with exact ids: the generation prompt, the ids, the end unless they have it.
    if (m.role === "assistant" && m.generated) {
      out.push(p`${tok.header_start}assistant${tok.header_end}\n\n`, tokens(m.ids), [tok.eot.id, tok.eom.id].includes(m.ids.at(-1)) ? "" : p`${tok.eot}`);
      continue;
    }
    if (!(m.role === "ipython" || m.role === "tool" || ("tool_calls" in m && m.tool_calls.length !== 0))) {
      out.push(tok.header_start + m.role + (tok.header_end + "\n\n"), contentParts(m.content, true), p`${tok.eot}`);
    } else if ("tool_calls" in m) {
      if (m.role === "assistant") {
        out.push(p`${tok.header_start}assistant${tok.header_end}\n\n`, contentParts(m.content, false), "[",
                 m.tool_calls.map(tc => {
                   const f = tc.function !== undefined ? tc.function : tc;
                   // Jinja iterates a string's characters; indexing a string by a character is undefined ("").
                   const a = f.arguments, s = typeof a === "string";
                   return f.name + "(" + (s ? [...a] : Object.keys(a)).map(k => k + "=\"" + (s ? "" : formatPythonValue(a[k])) + "\"").join(", ") + ")";
                 }).join(", "),
                 p`]${tok.eot}`);
      }
    } else if (m.role === "tool" || m.role === "ipython") {
      out.push(p`${tok.header_start}ipython${tok.header_end}\n\n`,
               typeof m.content === "string" ? formatJSON(m.content) : templateItems(m.content).filter(c => c.type === "text").map(c => formatJSON(c.text)),
               p`${tok.eot}`);
    }
  }
  out.push(add_generation_prompt && p`${tok.header_start}assistant${tok.header_end}\n\n`);
  return out;
}
