// DeepSeek-V4-Flash / V4-Pro (port of encoding/encoding_dsv4.py; no Jinja template shipped).
// State adapter as in v32.tsx; reasoning_effort is used only if "max" or "high".
"use strict_tokens";
import { DEEPSEEK_TOOLS_FOOTER, DEEPSEEK_V4_EFFORT_MAX, DEEPSEEK_V4_TOOLS_HEADER } from "../third_party/deepseek-ai/deepseek.tsx";
import { mappingEntries, templateTruthy, textOperand } from "../common/checks.tsx";

const DSML = tok.DSML;
const TASK_TOKENS = {action: tok.action, query: tok.query, authority: tok.authority, domain: tok.domain, title: tok.title, read_url: tok.read_url};


// Python `d.get(k, default)`: the default only when the key is missing (a null value stays null).
const getOr = (o, k, d) => k in o ? o[k] : d;

// Python `x is None` for a `.get()` result: missing or null.
const isNone = x => x === undefined || x === null;


// Python `sep.join(xs)` takes only strings (a TypeError on None, numbers, lists).
const joinStrings = (xs, sep) => xs.map(textOperand).join(sep);

function adapt_messages(messages, tools) {
  if (!templateTruthy(tools)) { return messages; }
  if (messages.length > 0 && messages[0].role === "system") { return [{...messages[0], tools: tools}, ...messages.slice(1)]; }
  return [{role: "system", content: "", tools: tools}, ...messages];
}

function merge_tool_messages(messages) {
  let out = [];
  for (const msg of messages) {
    const role = msg.role;
    const last = out.at(-1);
    const prev_user = out.length > 0 && last.role === "user" && "content_blocks" in last;
    if (role === "tool") {
      const b = {type: "tool_result", tool_use_id: getOr(msg, "tool_call_id", ""), content: getOr(msg, "content", "")};
      if (prev_user) { out = [...out.slice(0, -1), {...last, content_blocks: [...last.content_blocks, b]}]; }
      else { out = [...out, {role: "user", content_blocks: [b]}]; }
    } else if (role === "user") {
      const b = {type: "text", text: getOr(msg, "content", "")};
      if (prev_user && isNone(last.task)) { out = [...out.slice(0, -1), {...last, content_blocks: [...last.content_blocks, b]}]; }
      else {
        let n = {role: "user", content: getOr(msg, "content", ""), content_blocks: [b]};
        for (const k of ["task", "wo_eos", "mask"]) { if (k in msg) { n = {...n, [k]: msg[k]}; } }
        out = [...out, n];
      }
    } else {
      out = [...out, msg];
    }
  }
  return out;
}

function sort_tool_results(messages) {
  let out = [];
  let order = {};
  for (const msg of messages) {
    if (msg.role === "assistant" && templateTruthy(msg.tool_calls)) {
      order = {};
      for (const [j, tc] of msg.tool_calls.entries()) {
        const id = templateTruthy(tc.id) ? tc.id : getOr(getOr(tc, "function", {}), "id", "");
        if (templateTruthy(id)) { order = {...order, [id]: j}; }
      }
      out = [...out, msg];
    } else if (msg.role === "user" && templateTruthy(msg.content_blocks)) {
      const tb = msg.content_blocks.filter(b => b.type === "tool_result");
      if (tb.length > 1 && Object.keys(order).length > 0) {
        const key = b => getOr(order, getOr(b, "tool_use_id", ""), 0);
        const sorted_blocks = tb.sort((x, y) => key(x) - key(y));
        let nb = [];
        let i = 0;
        for (const b of msg.content_blocks) {
          if (b.type === "tool_result") { nb = [...nb, sorted_blocks[i]]; i = i + 1; }
          else { nb = [...nb, b]; }
        }
        out = [...out, {...msg, content_blocks: nb}];
      } else {
        out = [...out, msg];
      }
    } else {
      out = [...out, msg];
    }
  }
  return out;
}

function last_user_index(msgs) {
  for (let i = msgs.length - 1; i >= 0; i--) { if (["user", "developer"].includes(msgs[i].role)) { return i; } }
  return -1;
}

function render_tools_v4(tools) {
  const schemas = tools.map(t => formatJSON(t.function));
  return DEEPSEEK_V4_TOOLS_HEADER + schemas.join("\n") + DEEPSEEK_TOOLS_FOOTER;
}

function dsml_params(argStr) {
  let args;
  try { args = JSON.parse(argStr); } catch (e) { args = {arguments: argStr}; }
  const lines = mappingEntries(args).map(([k, v]) =>
    "<" + DSML + "parameter name=\"" + k + "\" string=\"" + (typeof v === "string" ? "true" : "false") + "\">" + (typeof v === "string" ? v : formatJSON(v)) + "</" + DSML + "parameter>");
  return lines.join("\n");
}

function user_text(m) {
  if (!templateTruthy(m.content_blocks)) { return formatPythonValue(templateTruthy(m.content) ? m.content : ""); }
  let parts = [];
  for (const b of m.content_blocks) {
    const t = b.type ?? null;
    if (t === "text") { parts = [...parts, getOr(b, "text", "")]; }
    else if (t === "tool_result") {
      let c = getOr(b, "content", "");
      if (Array.isArray(c)) {
        c = joinStrings(c.map(x => x.type === "text" ? getOr(x, "text", "") : "[Unsupported " + formatPythonValue(x.type ?? null) + "]"), "\n\n");
      }
      parts = [...parts, "<tool_result>" + formatPythonValue(c) + "</tool_result>"];
    } else { parts = [...parts, "[Unsupported " + formatPythonValue(t) + "]"]; }
  }
  return joinStrings(parts, "\n\n");
}


export default function DeepSeekV4({ messages, tools, thinking, enable_thinking, reasoning_effort }) {
  const think = templateTruthy(thinking) || templateTruthy(enable_thinking);
  const effort = ["max", "high"].includes(reasoning_effort) ? reasoning_effort : null;
  let msgs = sort_tool_results(merge_tool_messages(adapt_messages(messages, tools)));
  const drop = !msgs.some(m => templateTruthy(m.tools));
  if (think && drop) {
    const lu0 = last_user_index(msgs);
    let kept = [];
    for (const [i, m] of msgs.entries()) {
      if (["user", "system", "tool", "latest_reminder", "direct_search_results"].includes(m.role) || i >= lu0) kept.push(m);
      else if (m.role === "assistant") kept.push(Object.fromEntries(Object.entries(m).filter(([k]) => k !== "reasoning_content")));
    }
    msgs = kept;
  }
  const lu = last_user_index(msgs);
  const responseFormat = m => templateTruthy(m.response_format) && "\n\n## Response Format:\n\nYou MUST strictly adhere to the following schema to reply:\n" + formatJSON(m.response_format);
  let out = [p`${tok.begin_of_sentence}`];
  for (const [i, m] of msgs.entries()) {
    const content = m.content ?? null;
    if (i === 0 && think && effort === "max") out.push(DEEPSEEK_V4_EFFORT_MAX);
    if (m.role === "system") {
      out.push(formatPythonValue(templateTruthy(content) ? content : ""), templateTruthy(m.tools) && "\n\n" + render_tools_v4(m.tools), responseFormat(m));
    } else if (m.role === "developer") {
      if (!templateTruthy(content)) throw new Error("Invalid message for role `" + m.role + "`: " + formatPythonValue(m));
      out.push(p`${tok.User}${formatPythonValue(content)}`, templateTruthy(m.tools) && "\n\n" + render_tools_v4(m.tools), responseFormat(m));
    } else if (m.role === "user") {
      out.push(p`${tok.User}${user_text(m)}`);
    } else if (m.role === "latest_reminder") {
      out.push(p`${tok.latest_reminder}${formatPythonValue(content)}`);
    } else if (m.role === "tool") {
      throw new Error("deepseek_v4 merges tool messages into user");
    } else if (m.role === "assistant" && m.generated) {
      // A model turn logged with exact ids: the turn before it already wrote the generation prompt
      // (Assistant and the think tag), so the ids follow verbatim, then the end unless they have it.
      out.push(tokens(m.ids), m.ids.at(-1) === tok.end_of_sentence.id ? "" : p`${tok.end_of_sentence}`);
    } else if (m.role === "assistant") {
      const prevHasTask = i - 1 >= 0 && !isNone(msgs[i - 1].task);
      out.push(think && !prevHasTask && (!drop || i > lu) && formatPythonValue(templateTruthy(m.reasoning_content) ? m.reasoning_content : "") + tok.think_end,
               formatPythonValue(templateTruthy(content) ? content : ""),
               templateTruthy(m.tool_calls) && "\n\n<" + DSML + "tool_calls>\n" + m.tool_calls.map(tc =>
                 "<" + DSML + "invoke name=\"" + tc.function.name + "\">\n" + dsml_params(tc.function.arguments) + "\n</" + DSML + "invoke>").join("\n") + "\n</" + DSML + "tool_calls>",
               !templateTruthy(m.wo_eos) && p`${tok.end_of_sentence}`);
    } else {
      throw new Error("Unknown role: " + formatPythonValue(m.role ?? null));
    }
    if (i + 1 >= msgs.length || ["assistant", "latest_reminder"].includes(msgs[i + 1].role)) {
      const task = m.task;
      if (!isNone(task)) {
        if (!(task in TASK_TOKENS)) throw new Error("Invalid task");
        out.push(task !== "action" ? TASK_TOKENS[task] : p`${tok.Assistant}${!think ? tok.think_end : tok.think}${TASK_TOKENS[task]}`);
      } else if (["user", "developer"].includes(m.role)) {
        out.push(p`${tok.Assistant}${(!drop && think) || (drop && think && i >= lu) ? tok.think : tok.think_end}`);
      }
    }
  }
  return out;
}

export { DeepSeekV4 };
