// Zhipu / Z.ai GLM chat formats: glm-4-9b-chat, GLM-4-0414 / GLM-Z1-0414, GLM-4.5 .. GLM-5.3.
"use strict_tokens";
import { Turns } from "../common/turns.tsx";
import { GLM_TOOLS_INTRO, GLM_Z1_SYS, glmToolsFormat45, glmToolsFormat47 } from "../third_party/zai-org/glm.tsx";
import { GLM4_BROWSER, GLM4_COGVIEW, GLM4_FN_TAIL, GLM4_PYTHON, GLM4_SYS } from "../third_party/zai-org/glm4.tsx";
import { mappingEntries, templateTruthy } from "../common/checks.tsx";

// ---- glm-4-9b-chat (2024): tools come from a per-message `tools` field ----
// ---- GLM-4-0414 (z1=false) and GLM-Z1-0414 (z1=true) ----
// ---- GLM-4.5 .. GLM-5.3 (v = 45, 47, 50, 51, 52, 53) ----

const MEDIA = ["image", "image_url", "video", "video_url", "audio", "audio_url", "input_audio"];


// `"k": formatJSON(v)` pairs, skipping defer_loading / strict (GLM-5.1+).
function tool_to_json(tool) {
  const parts = Object.entries(tool)
    .filter(([k]) => k !== "defer_loading" && k !== "strict")
    .map(([k, v]) => "\"" + k + "\": " + formatJSON(v));
  return "{" + parts.join(", ") + "}";
}

function visible_text(c, v) {
  if (isFragment(c)) return c;
  if (typeof c === "string") { return c; }
  if (Array.isArray(c)) {
    let out = "";
    for (const item of c) {
      if (isPlainObject(item) && item.type === "text") { out = out + formatPythonValue(item.text); }
      else if (typeof item === "string") { out = out + item; }
      else if (v >= 52 && isPlainObject(item) && MEDIA.includes(item.type)) {
        out = out + "<reminder>You are unable to process this " + item.type.replaceAll("_url", "").replaceAll("input_", "") + " because you don't have multi-modal input ability. Try different methods.</reminder>";
      }
    }
    return out;
  }
  if (v >= 53 && c === null) { return ""; }
  return formatPythonValue(c);
}

function tool_refs(refs, tools) {
  let out = (tok.tool_response + "<tools>\n");
  for (const tr of refs) {
    for (const tool of tools) {
      const t = "function" in tool ? tool.function : tool;
      if (t.name === tr.name) { out = out + tool_to_json(t) + "\n"; }
    }
  }
  return out + ("</tools>" + tok.tool_response_end);
}

function id_of(o) {
  if (o.tool_call_id) { return formatPythonValue(o.tool_call_id); }
  if (o.id) { return formatPythonValue(o.id); }
  return "";
}

function is_list_of_outputs(m) { return Array.isArray(m.content) && m.content.length > 0 && m.content[0]?.output !== undefined; }

function is_refs(x) { return Array.isArray(x) && x.length > 0 && x[0]?.type === "tool_reference"; }

function render_tool_response(m, tools) {
  if (typeof m.content === "string" || isFragment(m.content)) { return tok.tool_response + m.content + tok.tool_response_end; }
  if (is_refs(m.content)) { return tool_refs(m.content, tools); }
  if (is_list_of_outputs(m)) {
    let out = "";
    for (const tr of m.content) { out = out + (is_refs(tr.output) ? tool_refs(tr.output, tools) : tok.tool_response + visible_text(tr.output, 53) + tok.tool_response_end); }
    return out;
  }
  return tok.tool_response + visible_text(m.content, 53) + tok.tool_response_end;
}

// Ids of all tool results in a block (list-of-outputs messages contribute each entry).
function result_ids(block) {
  let ids = [];
  for (const m of block) {
    if (is_list_of_outputs(m)) { for (const e of m.content) { ids.push(id_of(e)); } }
    else { ids.push(id_of(m)); }
  }
  return ids;
}

// GLM-5.3: a block of consecutive tool messages, reordered to the call order of the assistant
// message before it (`calls`, null if none) when ids allow.
function glm53_tool_block(block, calls, tools) {
  let can_sort = true;
  if (calls === null) { can_sort = false; }
  else {
    const ids = result_ids(block);
    const call_ids = calls.map(c => id_of(c));
    for (const id of ids) {
      if (!id || ids.filter(x => x === id).length > 1 || !call_ids.includes(id)) { can_sort = false; }
    }
    for (let i = 0; i < call_ids.length; i++) {
      if (!call_ids[i] || call_ids.slice(i + 1).includes(call_ids[i])) { can_sort = false; }
    }
  }
  let out = "";
  if (can_sort) {
    for (const tc of calls) {
      const tid = id_of(tc);
      for (const m of block) {
        if (is_list_of_outputs(m)) {
          for (const e of m.content) {
            if (id_of(e) === tid) { out = out + (is_refs(e.output) ? tool_refs(e.output, tools) : tok.tool_response + visible_text(e.output, 53) + tok.tool_response_end); }
          }
        } else if (id_of(m) === tid) {
          out = out + render_tool_response(m, tools);
        }
      }
    }
  } else {
    for (const m of block) { out = out + render_tool_response(m, tools); }
  }
  return out;
}


// ---- glm-4-9b-chat (2024): tools come from a per-message `tools` field ----
// A GLM-4 model turn logged with exact ids: the generation prompt, then the ids, without the stop
// that ends them (<|user|>, <|observation|>): the next turn writes it.
function generatedGLM4(m, prompt) {
  const ids = m.ids?.length > 0 && [tok.observation.id, tok.user.id].includes(m.ids.at(-1)) ? m.ids.slice(0, -1) : m.ids;
  return [prompt, tokens(ids)];
}

export function GLM4Chat({ messages, add_generation_prompt }) {
  return [p`${tok.gMASK}${tok.sop}`, messages.map(item => item.role === "assistant" && item.generated ? generatedGLM4(item, p`${tok.assistant}`) : [
    item.tools !== undefined && [p`${tok.system}\n`, GLM4_SYS, item.tools.map(tool =>
      tool.type === "function" ? p`\n## ${formatPythonValue(tool.function.name)}\n\n${formatJSON(tool.function, {indent: 4})}\n${GLM4_FN_TAIL}`
      : tool.type === "python" ? GLM4_PYTHON : tool.type === "simple_browser" ? GLM4_BROWSER : tool.type === "cogview" && GLM4_COGVIEW)],
    templateTruthy(item.content) && p`<|${formatPythonValue(item.role)}|>${formatPythonValue(item.metadata)}\n${formatPythonValue(item.content)}`]),
    add_generation_prompt && p`${tok.assistant}`];
}

// ---- GLM-4-0414 (z1=false) and GLM-Z1-0414 (z1=true) ----
export function GLM4_0414({ messages, tools, add_generation_prompt, z1 = false }) {
  return [p`${tok.gMASK}${tok.sop}`,
    tools?.length > 0 && [p`${tok.system}\n`,
      z1 ? GLM_Z1_SYS : "# 可用工具\n",
      tools.map(tool => {
        const f = templateTruthy(tool.function) ? tool.function : tool;
        return p`\n## ${formatPythonValue(f.name)}\n\n${formatJSON(f, {indent: 4})}\n在调用上述函数时，请使用 Json 格式表示调用的参数。`;
      })],
    messages.filter(m => m.role === "system").map(m => p`${tok.system}\n${formatPythonValue(m.content)}`),
    messages.filter(m => m.role !== "system").map(m => {
      if (m.role === "assistant" && m.generated) return generatedGLM4(m, p`${tok.assistant}${z1 && ("\n" + textOrToken("<think>"))}`);
      const content = z1 && !isFragment(m.content) ? m.content.split(textOrToken("</think>")).at(-1).trim() : m.content;
      const meta = m.metadata ?? "";
      return m.role === "user" ? p`${tok.user}\n${formatPythonValue(content)}`
        : m.role === "assistant" && !templateTruthy(meta) ? p`${tok.assistant}\n${formatPythonValue(content)}`
        : m.role === "assistant" ? p`${tok.assistant}${formatPythonValue(meta)}\n${formatPythonValue(content)}`
        : m.role === "observation" && p`${tok.observation}\n${formatPythonValue(content)}`;
    }),
    add_generation_prompt && p`${tok.assistant}${z1 && ("\n" + textOrToken("<think>"))}`];
}

export const GLMZ1_0414 = state => GLM4_0414({...state, z1: true});

// ---- GLM-4.5 .. GLM-5.3 (v = 45, 47, 50, 51, 52, 53) ----

function Assistant({ m, i, v, lui, hasThinking, enable_thinking, clear_thinking, clear }) {
  let content = visible_text(m.content, v);
  let think;
  if (v <= 50) {
    let rc = "";
    if (typeof m.reasoning_content === "string") rc = m.reasoning_content;
    else if (content.includes(tok.think_end.text)) {
      rc = trimStartChars(trimEndChars(content.split(tok.think_end.text)[0], "\n").split(tok.think.text).at(-1), "\n");
      content = trimStartChars(content.split(tok.think_end.text).at(-1), "\n");
    }
    if (v === 45) think = i > lui && rc ? ("\n" + tok.think) + rc.trim() + tok.think_end : ("\n" + tok.think + tok.think_end);
    else think = ((clear_thinking !== undefined && !clear_thinking) || i > lui) && rc ? tok.think + rc.trim() + tok.think_end : tok.think_end;
    if (content.trim()) content = (v === 45 ? "\n" : "") + content.trim(); else content = "";
  } else {
    let rc = undefined;
    if (typeof m.reasoning_content === "string") rc = m.reasoning_content;
    else if (content.includes(tok.think_end.text)) {
      rc = content.split(tok.think_end.text)[0].split(tok.think.text).at(-1);
      content = content.split(tok.think_end.text).at(-1);
    } else if (v === 51 && i > lui && !(enable_thinking !== undefined && !enable_thinking)) rc = "";
    else if (v === 51 && i < lui && hasThinking) rc = "";
    const show = v === 53 ? (!clear || i > lui) : ((clear !== undefined && !clear) || i > lui);
    think = show && rc !== undefined ? tok.think + rc + tok.think_end : v === 51 ? tok.think_end : (tok.think + tok.think_end);
    content = content.trim() ? content.trim() : "";
  }
  return [p`${tok.assistant}`, think, content,
    m.tool_calls?.length > 0 && m.tool_calls.map(tc => {
      const f = templateTruthy(tc.function) ? tc.function : tc;
      const pairs = mappingEntries(f.arguments).map(([k, val]) => {
        const value = typeof val === "string" ? val : formatJSON(val);
        return v === 45 ? tok.arg_key + k + (tok.arg_key_end + "\n" + tok.arg_value) + value + (tok.arg_value_end + "\n") : tok.arg_key + k + (tok.arg_key_end + tok.arg_value) + value + tok.arg_value_end;
      }).join("");
      return v === 45 ? ("\n" + tok.tool_call) + f.name + "\n" + pairs + tok.tool_call_end : tok.tool_call + f.name + pairs + tok.tool_call_end;
    })];
}

// A tool message before GLM-5.3 (`prev`: the message before it, null for the first).
function ToolMessage({ m, prev, v, tools }) {
  const firstInBlock = prev === null || prev.role !== "tool";
  if (v <= 50) {
    if (typeof m.content === "string" || isFragment(m.content)) return [firstInBlock && p`${tok.observation}`, v === 45 ? ("\n" + tok.tool_response + "\n") + m.content + ("\n" + tok.tool_response_end) : tok.tool_response + m.content + tok.tool_response_end];
    return [p`${tok.observation}`, m.content.map(tr => {
      const out = formatPythonValue(tr.output !== undefined ? tr.output : tr);
      return v === 45 ? ("\n" + tok.tool_response + "\n") + out + ("\n" + tok.tool_response_end) : tok.tool_response + out + tok.tool_response_end;
    })];
  }
  const c = m.content;
  const listy = Array.isArray(c) && c.length > 0;
  return [firstInBlock && p`${tok.observation}`,
    typeof c === "string" ? tok.tool_response + c + tok.tool_response_end
    : listy && c[0]?.type === "tool_reference" ? tool_refs(c, tools)
    : v === 52 && listy && isPlainObject(c[0]) && c[0].output !== undefined ? c.map(tr => tok.tool_response + tr.output + tok.tool_response_end)
    : tok.tool_response + visible_text(c, v) + tok.tool_response_end];
}

// The message loop is a scan (memoScan): a message's output depends only on it, its index, the
// state from the messages before it and the deps `c`, so an agent's render computes only what is
// new. The state: the message before, GLM-5.1's hasThinking, and GLM-5.3's open block of tool
// results (written whole, reordered, once the block ends).
function message_step(s, m, i, c) {
  s = s ?? {prev: null, hasThinking: false, block: null};
  const { v, enable_thinking } = c;
  const out = [];
  // GLM-5.3 writes the tool results that answer an assistant as one observation block, here at
  // the next message; it ends the assistant's turn, so it is kept apart from this message's own.
  let block = s.block, lead = null;
  if (block !== null && m.role !== "tool") {
    lead = [p`${tok.observation}`, glm53_tool_block(block.msgs, block.calls, c.tools)];
    block = null;
  }
  let hasThinking = s.hasThinking;
  if (m.role === "user") {
    const vt = visible_text(m.content, v);
    out.push(v === 45 ? p`${tok.user}\n${vt}${enable_thinking !== undefined && !enable_thinking && !vt.endsWith(tok.nothink_end.text) && tok.nothink_end}` : p`${tok.user}${vt}`);
    if (v === 51) hasThinking = c.thinkingIdx.includes(i);
  } else if (m.role === "assistant" && m.generated) {
    out.push(GeneratedAssistant({m, v, enable_thinking}));
  } else if (m.role === "assistant") {
    out.push(Assistant({m, i, v, lui: c.lui, hasThinking, enable_thinking, clear_thinking: c.clear_thinking, clear: c.clear}));
  } else if (m.role === "tool" && v === 53) {
    const prev = s.prev;
    block = block !== null ? {...block, msgs: [...block.msgs, m]}
      : {msgs: [m], calls: prev !== null && prev.role === "assistant" && prev.tool_calls?.length > 0 ? prev.tool_calls : null};
  } else if (m.role === "tool") {
    out.push(ToolMessage({m, prev: s.prev, v, tools: c.tools}));
  } else if (m.role === "system") {
    out.push(v === 45 ? p`${tok.system}\n${visible_text(m.content, v)}` : p`${tok.system}${visible_text(m.content, v)}`);
  }
  return [{prev: m, hasThinking, block}, {lead, out}];
}

// GLM-5.1: which user turns are followed by an assistant turn with reasoning (the state: the last user turn).
function thinking_step(s, m, i) {
  s = s ?? {lastU: -1, idx: []};
  if (m.role === "user") return [{...s, lastU: i}, null];
  if (m.role === "assistant" && typeof m.reasoning_content === "string") return [{...s, idx: [...s.idx, s.lastU]}, null];
  return [s, null];
}

export function GLM45(state) {
  const { messages, tools, add_generation_prompt, enable_thinking, reasoning_effort, clear_thinking, v = 45 } = state;
  const clear = clear_thinking !== undefined ? clear_thinking : (v === 53 ? false : undefined);
  let out = [p`${tok.gMASK}${tok.sop}`];
  if (v === 52) {
    const eff = reasoning_effort === "high" ? "high" : "max";
    if (enable_thinking === undefined || enable_thinking) out.push(p`${tok.system}Reasoning Effort: ${capitalize(eff)}`);
  } else if (v === 53) {
    const eff = ["low", "high"].includes(reasoning_effort) ? reasoning_effort : "max";
    out.push(p`${tok.system}Reasoning Effort: ${capitalize(eff)}`);
  }
  if (tools?.length > 0) {
    out.push(GLM_TOOLS_INTRO, tools.map(tool => {
      if (v >= 51) {
        const t = "function" in tool ? tool.function : tool;
        return (t.defer_loading === undefined || !t.defer_loading) && tool_to_json(t) + "\n";
      }
      return formatJSON(tool) + "\n";
    }), v === 45 ? glmToolsFormat45() : glmToolsFormat47());
  }
  // The last user turn changes every turn; it is a dep only where an assistant turn reads it
  // (GLM-5.3 shows all reasoning unless clear_thinking).
  const lui = v === 53 && !clear ? -1 : messages.findLastIndex(m => m.role === "user");
  const thinkingIdx = v === 51 ? memoScan(messages, null, thinking_step, null).state?.idx ?? [] : null;
  const c = {v, tools, lui, thinkingIdx, enable_thinking, clear_thinking, clear};
  const r = memoScan(messages, null, (s, m, i) => message_step(s, m, i, c), [v, tools, lui, thinkingIdx, enable_thinking, clear_thinking, clear]);
  const last = r.state?.block != null && [p`${tok.observation}`, glm53_tool_block(r.state.block.msgs, r.state.block.calls, tools)];
  // A turn is an assistant with the tool results that answer it (written before the next
  // message, or last), or a message; the system messages before the first other one are the header.
  let start = 0;
  while (start < messages.length && messages[start].role === "system") start++;
  return [<Turns messages={messages} header={out} items={r.outs} start={start} joins={i => messages[i].role === "tool"} tail={last}/>,
          add_generation_prompt && generation_prompt(v, enable_thinking)];
}

function generation_prompt(v, enable_thinking) {
  const off = enable_thinking !== undefined && !enable_thinking;
  return [p`${tok.assistant}`, v === 45 ? off && ("\n" + tok.think + tok.think_end) : v <= 51 ? (off ? tok.think_end : tok.think) : v === 52 ? (off ? (tok.think + tok.think_end) : tok.think) : tok.think];
}

// A model turn logged with exact ids (TurnStrategy `model` record with `ids`): the generation
// prompt the model saw, then its output verbatim. GLM stops on the next turn's opener
// (<|observation|> or <|user|>), which the following turn emits, so a trailing one is dropped.
function GeneratedAssistant({ m, v, enable_thinking }) {
  const ids = m.ids?.length > 0 && [tok.observation.id, tok.user.id].includes(m.ids.at(-1)) ? m.ids.slice(0, -1) : m.ids;
  return [generation_prompt(v, enable_thinking), tokens(ids)];
}

export const GLM47 = state => GLM45({...state, v: 47});
export const GLM5 = state => GLM45({...state, v: 50});
export const GLM51 = state => GLM45({...state, v: 51});
export const GLM52 = state => GLM45({...state, v: 52});
export const GLM53 = state => GLM45({...state, v: 53});
