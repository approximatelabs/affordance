import { mappingEntries } from "../common/checks.tsx";
// Qwen3.5 / Qwen3.6 / Qwen3.8 (vision-language, XML tool calls, thinking). `ver` selects the release.
"use strict_tokens";
import { Turns } from "../common/turns.tsx";
import { GeneratedTurn, XML_TOOL_FORMAT, countMedia, renderContent } from "./common.tsx";
import { QWEN35_EFFORT_LOW, QWEN35_EFFORT_XHIGH } from "../third_party/qwen/qwen.tsx";

function lastQueryIndex(messages) {
  for (let i = messages.length - 1; i >= 0; i--) {
    const m = messages[i];
    if (m.role === "user") {
      if (isFragment(m.content)) return i;
      const c = renderContent(m.content, false, false, 0, 0).trim();
      if (!(c.startsWith(tok.tool_response.text) && c.endsWith(tok.tool_response_end.text))) return i;
    }
  }
  return -1;
}

function reasoningInstructions(ver, enable_thinking, reasoning_effort) {
  if (ver < 38 || !(enable_thinking === undefined || enable_thinking === true)) return "";
  const e = reasoning_effort === undefined ? "xhigh" : reasoning_effort;
  if (!["xhigh", "medium", "low"].includes(e)) throw new Error("Unexpected reasoning effort " + formatPythonValue(reasoning_effort) + ". Supported types are xhigh (default), medium, and low.");
  if (e === "xhigh") return QWEN35_EFFORT_XHIGH;
  if (e === "low") return QWEN35_EFFORT_LOW;
  return "";
}

const argText = (v, ver) => ver >= 36 ? (typeof v === "string" ? v : formatJSON(v)) : (isPlainObject(v) || Array.isArray(v) ? formatJSON(v) : formatPythonValue(v));

function SystemBlock({ messages, tools, ri, ver }) {
  const first = messages[0];
  const sys = first.role === "system" ? renderContent(first.content, true, false, 0, 0).trim() : null;
  if (Array.isArray(tools) && tools.length > 0) {
    return p`${tok.im_start}system\n${ri ? ri + "\n\n" : ""}# Tools\n\nYou have access to the following functions:\n\n<tools>${
      tools.map(t => "\n" + formatJSON(t))}\n</tools>${XML_TOOL_FORMAT}${sys ? "\n\n" + sys : ""}${tok.im_end}\n`;
  }
  if (sys !== null) {
    if (ver < 38) return (tok.im_start + "system\n") + sys + (tok.im_end + "\n");
    if (sys) return (tok.im_start + "system\n") + (ri ? ri + "\n\n" : "") + sys + (tok.im_end + "\n");
  }
  return ri && (tok.im_start + "system\n") + ri + (tok.im_end + "\n");
}

function Assistant({ m, content, show, ver }) {
  let reasoning = "";
  if (typeof m.reasoning_content === "string") reasoning = m.reasoning_content;
  else if (ver < 38 && typeof content === "string" && content.includes(tok.think_end.text)) {
    reasoning = trimStartChars(trimEndChars(content.split(tok.think_end.text)[0], "\n").split(tok.think.text).at(-1), "\n");
    content = trimStartChars(content.split(tok.think_end.text).at(-1), "\n");
  }
  reasoning = reasoning.trim();
  const calls = Array.isArray(m.tool_calls) ? m.tool_calls : [];
  return <>
    {show ? tok.im_start + m.role + ("\n" + tok.think + "\n") + reasoning + ("\n" + tok.think_end + "\n\n") + content : tok.im_start + m.role + "\n" + content}
    {calls.map((tc, j) => {
      const f = tc.function !== undefined ? tc.function : tc;
      return <>
        {(j === 0 ? (content.trim() ? ("\n\n" + tok.tool_call + "\n<function=") : (tok.tool_call + "\n<function=")) : ("\n" + tok.tool_call + "\n<function=")) + f.name + ">\n"}
        {f.arguments !== undefined && (ver < 38 || f.arguments !== "") &&
          mappingEntries(f.arguments).map(([k, v]) => p`${"<parameter=" + k + ">\n"}${argText(v, ver)}\n</parameter>\n`)}
        {p`</function>\n${tok.tool_call_end}`}
      </>;
    })}
    {p`${tok.im_end}\n`}
  </>;
}

function generationPrompt(enable_thinking) {
  return p`${tok.im_start}assistant\n${enable_thinking === false ? (tok.think + "\n\n" + tok.think_end + "\n\n") : (tok.think + "\n")}`;
}

export default function Qwen35({ messages, tools, add_generation_prompt, enable_thinking, reasoning_effort, preserve_thinking, add_vision_id, ver = 35 }) {
  if (!messages?.length) throw new Error("Cannot apply chat template to an empty conversation. Provide at least one message.");
  const ri = reasoningInstructions(ver, enable_thinking, reasoning_effort);
  const head = <SystemBlock messages={messages} tools={tools} ri={ri} ver={ver}/>;
  const lqi = lastQueryIndex(messages);
  if (lqi < 0) throw new Error("No user query found in messages.");
  // The message loop is a scan (memoScan): what a message writes depends only on it, its index,
  // the state from the messages before it (the role before, the images and videos seen so far)
  // and the deps, so a conversation that grows by a turn computes only what is new.
  // An assistant message whose thinking is shown only after the last query is written both
  // ways once; which one is used is chosen below, so a new query re-renders nothing.
  const byQuery = ver === 35 || (ver === 36 ? preserve_thinking !== true : !(preserve_thinking === undefined || preserve_thinking === true));
  const r = memoScan(messages, null, (s, m, i) => {
    s = s ?? {prev: null, image: 0, video: 0};
    const content = renderContent(m.content, false, add_vision_id, s.image, s.video).trim();
    const next = {prev: m.role, image: s.image + countMedia([m], "image"), video: s.video + countMedia([m], "video")};
    // A block of tool results is closed where the next message starts (or at the end).
    const lead = s.prev === "tool" && m.role !== "tool" ? p`${tok.im_end}\n` : null;
    if (m.role === "system") { if (i > 0) throw new Error("System message must be at the beginning."); return [next, {lead, out: null}]; }
    if (m.role === "user") return [next, {lead, out: tok.im_start + m.role + "\n" + content + tok.im_end + "\n"}];
    if (m.role === "assistant" && m.generated) return [next, {lead, out: <GeneratedTurn ids={m.ids} prompt={generationPrompt(enable_thinking)}/>}];
    if (m.role === "assistant") {
      if (!byQuery) return [next, {lead, out: <Assistant m={m} content={content} show={true} ver={ver}/>}];
      return [next, {either: true,
        shown: {lead, out: <Assistant m={m} content={content} show={true} ver={ver}/>},
        hidden: {lead, out: <Assistant m={m} content={content} show={false} ver={ver}/>}}];
    }
    if (m.role === "tool") return [next, {lead, out: <>
      {i > 0 && s.prev !== "tool" && p`${tok.im_start}user`}
      {p`\n${tok.tool_response}\n${content}\n${tok.tool_response_end}`}
    </>}];
    throw new Error("Unexpected message role.");
  }, [ver, enable_thinking, preserve_thinking, add_vision_id]);
  const items = r.outs.map((o, i) => o.either ? (i > lqi ? o.shown : o.hidden) : o);
  return <>
    <Turns messages={messages} header={head} items={items} tail={r.state?.prev === "tool" && p`${tok.im_end}\n`}/>
    {add_generation_prompt && generationPrompt(enable_thinking)}
  </>;
}

export const Qwen36 = state => Qwen35({...state, ver: 36});
export const Qwen38 = state => Qwen35({...state, ver: 38});

export { SystemBlock };

export { Qwen35 };
