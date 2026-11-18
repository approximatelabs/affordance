// Xiaomi MiMo-V2-Flash / V2.5 / V2.5-Pro (XML tools, <think> in every assistant turn) and
// MiMo-V2.6 (compact XML tools, multimodal parts). MiMo-7B-RL is qwen/qwen25.tsx.
"use strict_tokens";
import { Turns } from "../common/turns.tsx";
import { ToolDef, isSeq } from "../common/xml_tools.tsx";
import { MIMO_PRO_SYSTEM, MIMO_TOOLS_INTRO, MIMO_TOOL_FORMAT } from "../third_party/xiaomi/mimo.tsx";
import { mappingEntries, templateItems } from "../common/checks.tsx";

// Jinja `k in x`: a key of a mapping, else membership (a substring of a string part).
const has = (x, k) => isPlainObject(x) ? k in x : x.includes(k);

// V2.5 content: strings, or parts rendered to media placeholders and text (a mapping iterates its keys).
function mimoContent(c, media) {
  if (isFragment(c)) return c;
  if (typeof c === "string") return c;
  let out = "";
  for (const prt of templateItems(c)) {
    if (media && (prt.type === "image" || has(prt, "image") || has(prt, "image_url"))) out += (tok.vision_start + tok.image_pad + tok.vision_end);
    else if (media && (prt.type === "audio" || has(prt, "audio") || has(prt, "audio_url"))) out += (tok.mimo_audio_start + tok.audio_pad + tok.mimo_audio_end);
    else if (media && (prt.type === "video" || has(prt, "video") || has(prt, "video_url"))) out += (tok.vision_start + tok.video_pad + tok.vision_end);
    else if (has(prt, "text")) out += prt.text;
  }
  return out;
}


// v=20: V2-Flash; v=25: V2.5 (multimodal content); v=251: V2.5-Pro (text-only content, own system prompt).
export default function MiMoV2({ messages, tools, add_generation_prompt, enable_thinking, keep_all_reasoning, v = 20 }) {
  if (tools !== undefined && tools !== null && !Array.isArray(tools)) throw new Error("Tools should either be a JSON schema, or a callable function with type hints and a docstring suitable for auto-conversion to a schema.");
  if (messages.length === 0) throw new Error("Cannot apply chat template to an empty conversation. Provide at least one message.");
  const parts = v >= 25;
  const render = c => parts ? mimoContent(c, v === 25) : c;
  const thinkOn = enable_thinking !== undefined ? enable_thinking : v >= 25;
  const keepAll = keep_all_reasoning !== undefined ? keep_all_reasoning : v >= 25;
  const hasSys = messages[0].role === "system";
  const msgs = hasSys ? messages.slice(1) : messages;
  const lui = msgs.findLastIndex(m => m.role === "user");
  const tls = tools !== undefined ? tools : [];
  const header = <>
    {hasSys ? (tok.im_start + "system\n") + render(messages[0].content)
      : v === 251 ? (tok.im_start + "system\n") + MIMO_PRO_SYSTEM
      : p`${tok.im_start}system\nYou are MiMo, a helpful AI assistant engineered by Xiaomi.`}
    {Array.isArray(tls) && tls.length > 0 && p`${MIMO_TOOLS_INTRO}${
      tls.map(t => <ToolDef tool={t}/>)}\n</tools>${MIMO_TOOL_FORMAT}`}
    {p`${tok.im_end}`}
  </>;
  return <>
    <Turns messages={msgs} header={header} start={0} message={(m, i) => {
      // A model turn logged with exact ids: the generation prompt, the ids, the turn end.
      if (m.role === "assistant" && m.generated) return [p`${tok.im_start}assistant\n${!thinkOn && (tok.think + tok.think_end)}`, tokens(m.ids), m.ids.at(-1) === tok.im_end.id ? "" : p`${tok.im_end}`];
      let content = typeof m.content === "string" || isFragment(m.content) ? m.content : (parts ? mimoContent(m.content, v === 25) : "");
      if (m.role === "assistant") {
        let reasoning = "";
        if (typeof m.reasoning_content === "string") reasoning = m.reasoning_content;
        else if (typeof content === "string" && content.includes(tok.think_end.text)) {
          reasoning = content.split(tok.think_end.text)[0].split(tok.think.text).at(-1);
          content = content.split(tok.think_end.text).at(-1);
        }
        return <>
          {(keepAll || i > lui) && reasoning ? tok.im_start + m.role + ("\n" + tok.think) + reasoning + tok.think_end + content
                                              : tok.im_start + m.role + ("\n" + tok.think + tok.think_end) + content}
          {Array.isArray(m.tool_calls) && m.tool_calls.map(tc => {
            const f = tc.function !== undefined ? tc.function : tc;
            return <>
              {(tok.tool_call + "\n<function=") + f.name + ">\n"}
              {f.arguments !== undefined && mappingEntries(f.arguments).map(([k, val]) => p`${"<parameter=" + k + ">"}${isSeq(val) ? formatJSON(val) : formatPythonValue(val)}</parameter>\n`)}
              {p`</function>\n${tok.tool_call_end}`}
            </>;
          })}
          {p`${tok.im_end}`}
        </>;
      }
      if (m.role === "user" || m.role === "system") return tok.im_start + m.role + "\n" + render(m.content) + tok.im_end;
      if (m.role === "tool") return <>
        {i > 0 && msgs[i - 1].role !== "tool" && p`${tok.im_start}tool\n`}
        {p`${tok.tool_response}\n${asContent(render(m.content))}\n${tok.tool_response_end}\n`}
        {(i === msgs.length - 1 || msgs[i + 1].role !== "tool") && p`${tok.im_end}`}
      </>;
      return tok.im_start + m.role + "\n" + render(m.content) + tok.im_end;
    }}/>
    {add_generation_prompt && p`${tok.im_start}assistant\n${!thinkOn && (tok.think + tok.think_end)}`}
  </>;
}

export const MiMoV25 = state => MiMoV2({...state, v: 25});
export const MiMoV25Pro = state => MiMoV2({...state, v: 251});

// ---- MiMo-V2.6 ----

function v26Content(c) {
  if (typeof c === "string" || isFragment(c)) return c;
  if (!Array.isArray(c) && !isPlainObject(c)) return "";
  let out = "";
  for (const prt of templateItems(c)) {
    if (!isPlainObject(prt)) out += formatPythonValue(prt);
    else if (prt.type === "image" || "image" in prt || "image_url" in prt) out += (tok.vision_start + tok.image_pad + tok.vision_end);
    else if (prt.type === "audio" || prt.type === "input_audio" || "audio" in prt || "audio_url" in prt || "input_audio" in prt) out += (tok.mimo_audio_start + tok.audio_pad + tok.mimo_audio_end);
    else if (prt.type === "video" || "video" in prt || "video_url" in prt) out += (tok.vision_start + tok.video_pad + tok.vision_end);
    else if ("text" in prt) out += formatPythonValue(prt.text);
  }
  return out;
}

const v26Tools = tools => "You are provided with the following tools:\n\n<tools>" + tools.map(t => "\n" + formatJSON(t)).join("") + "\n</tools>";

function v26Call(tc) {
  const f = tc.function !== undefined ? tc.function : tc.custom !== undefined ? tc.custom : tc;
  const a = f.arguments;
  const body = typeof f.input === "string" ? f.input
    : !a || (Array.isArray(a) && a.length === 0) || (isPlainObject(a) && Object.keys(a).length === 0) ? ""
    : typeof a === "string" ? a
    : mappingEntries(a).map(([k, val]) => "<parameter=" + k + ">" + (typeof val === "string" ? val : formatJSON(val)) + "</parameter>").join("");
  return (tok.tool_call + "<function=") + formatPythonValue(f.name) + ">" + body + ("</function>" + tok.tool_call_end);
}

export function MiMoV26({ messages, tools, add_generation_prompt, enable_thinking }) {
  if (messages.length === 0) throw new Error("Cannot apply chat template to an empty conversation. Provide at least one message.");
  return <>
    {Array.isArray(tools) && tools.length > 0 && (tok.im_start + "system\n") + v26Tools(tools) + tok.im_end}
    {messages.map(m => {
      // A model turn logged with exact ids: the generation prompt, the ids, the end unless they have it.
      if (m.role === "assistant" && m.generated) return [p`${tok.im_start}assistant\n${enable_thinking === false && (tok.think + tok.think_end)}`, tokens(m.ids), m.ids.at(-1) === tok.im_end.id ? "" : p`${tok.im_end}`];
      if (m.role === "assistant") return [
        (tok.im_start + "assistant\n" + tok.think) + (typeof m.reasoning_content === "string" ? m.reasoning_content : "") + tok.think_end + v26Content(m.content),
        Array.isArray(m.tool_calls) && m.tool_calls.length > 0 && m.tool_calls.map(v26Call).join(""),
        p`${tok.im_end}`];
      const body = v26Content(m.content);
      return [tok.im_start + formatPythonValue(m.role) + "\n" + body,
              Array.isArray(m.tools) && m.tools.length > 0 && (body ? "\n\n" : "") + v26Tools(m.tools),
              p`${tok.im_end}`];
    })}
    {add_generation_prompt && p`${tok.im_start}assistant\n${enable_thinking === false && (tok.think + tok.think_end)}`}
  </>;
}

export { MiMoV2 };
