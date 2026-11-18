// DeepSeek-V4.1 (ported from its Jinja template, itself ported from encoding/encoding.py).
// Kwargs: thinking_mode ("thinking"|"chat"; enable_thinking=false means "chat"), drop_thinking,
// reasoning_effort (1..100 or low/medium/high/max).
"use strict_tokens";
import { DEEPSEEK_TOOLS_FOOTER, DEEPSEEK_V41_TOOLS_HEADER } from "../third_party/deepseek-ai/deepseek.tsx";
import { templateTruthy } from "../common/checks.tsx";
import { Turns } from "../common/turns.tsx";

const IMG = tok.deepseek_image;
const DSML = tok.DSML;


// Jinja `d.get(k, default)`: the default only when the key is missing (a null value stays null).
const getOr = (o, k, d) => k in o ? o[k] : d;

// Jinja `d.get(k) is none`: missing or null.
const isNone = x => x === undefined || x === null;

// Jinja `~`, `{{ }}` and `|join` print values with Python str(); strings, tokens and fragments pass through.
const pyText = x => x === undefined ? "" : x === null || typeof x === "number" || typeof x === "boolean" || Array.isArray(x) || isPlainObject(x) ? formatPythonValue(x) : x;
const pyJoin = (xs, sep) => xs.map(pyText).join(sep);

export const isImageBlock = b => ["image", "image_url"].includes(b.type);
// An image block's source (a URL or data URL), if it has one.
export const imageSrc = b => typeof b.image_url === "string" ? b.image_url : b.image_url?.url ?? (typeof b.image === "string" ? b.image : null);
// An image block is the template's placeholder. An app that sends pictures lays each one out as
// a fragment first (views/template_apps/deepseek_v41.tsx).
const image = b => IMG;

function checkTextBlock(b) {
  // A laid-out fragment (a template app's screen) is the app's own text: no placeholder to check.
  if (isFragment(b.text)) return b.text;
  if ((templateTruthy(b.text) ? b.text : "").includes(IMG.text)) { throw new Error("Text block contains image placeholder " + IMG.text + ". Images should be separate content blocks."); }
  return templateTruthy(b.text) ? b.text : "";
}

function joined_text(blocks) {
  let parts = [];
  for (const b of blocks) {
    if (b.type === "text") parts = [...parts, checkTextBlock(b)];
    else if (isImageBlock(b)) parts = [...parts, image(b)];
  }
  return pyJoin(parts, "\n\n");
}

function qualified_name(name, ns) {
  if (isPlainObject(ns)) { ns = ns.name; }
  const cut = name.indexOf("::");
  if (cut >= 0) {
    const head = name.slice(0, cut);
    if (ns !== null && ns !== head) { throw new Error("Conflicting tool namespaces: " + pyText(ns) + " != " + head); }
    ns = head;
    name = name.slice(cut + 2);
  }
  if (name.includes("::")) { throw new Error("Tool name must not contain '::': " + name); }
  if (ns !== null && (ns ?? "").includes("::")) { throw new Error("Tool namespace must not contain '::': " + ns); }
  return ns === null ? name : pyText(ns) + "::" + name;
}

function render_tools(tool_list) {
  const schemas = tool_list.map(tool => {
    const f = tool.function !== undefined ? tool.function : tool;
    const ns = !isNone(tool.namespace) ? tool.namespace : getOr(f, "namespace", null);
    let d = Object.fromEntries(Object.entries(f).filter(([k]) => k !== "namespace"));
    d = {...d, name: qualified_name(f.name, ns)};
    if (isPlainObject(ns) && templateTruthy(ns.description)) { d = {...d, description: pyText(ns.description) + "\n" + pyText(templateTruthy(f.description) ? f.description : "")}; }
    return formatJSON(d);
  });
  return DEEPSEEK_V41_TOOLS_HEADER + schemas.join("\n") + DEEPSEEK_TOOLS_FOOTER;
}


// The template's passes are scans (memoScan): each message's result depends only on the message,
// the scan's state from the messages before it, and the scan's deps, so an agent's render keeps
// the results for the transcript it has already rendered and computes only what is new.

// 1. Merge tool results (and consecutive user turns) into user messages made of content blocks.
// The state is the last message out while it is a user message (a later one may join it); `out`
// lists the messages this one closes. With tools, the first message out is a system message
// that carries them.
function merge_step(open, msg, i, tools) {
  const content = getOr(msg, "content", null);
  const is_blocks = Array.isArray(content);
  if (typeof content === "string" && content.includes(IMG.text)) { throw new Error("Message content contains image special token " + IMG.text + ". Images should be provided as image content blocks."); }
  const rc = !isNone(msg.reasoning_content) ? msg.reasoning_content : getOr(msg, "thinking", null);
  if (typeof rc === "string" && rc.includes(IMG.text)) { throw new Error("reasoning_content contains image special token " + IMG.text); }
  const first = i === 0 && templateTruthy(tools) && msg.role !== "system" ? [{role: "system", content: "", tools: tools}] : [];
  const closed = open === null ? first : [...first, open];
  if (msg.role === "tool") {
    const tblock = {type: "tool_result", tool_use_id: getOr(msg, "tool_call_id", ""), content: is_blocks ? joined_text(content) : getOr(msg, "content", "")};
    if (open !== null) return [{...open, content_blocks: [...open.content_blocks, tblock]}, []];
    return [{role: "user", content_blocks: [tblock]}, closed];
  }
  if (msg.role === "user") {
    const blocks = is_blocks ? content : [{type: "text", text: templateTruthy(content) ? content : ""}];
    if (open !== null && isNone(open.task)) return [{...open, content_blocks: [...open.content_blocks, ...blocks]}, []];
    return [{...msg, content_blocks: blocks}, closed];
  }
  const m = i === 0 && templateTruthy(tools) && !templateTruthy(msg.tools) ? {...msg, tools: tools} : msg;
  return [null, [...closed, m]];
}

function merge_messages(messages, tools) {
  const r = memoScan(messages, null, (open, msg, i) => merge_step(open, msg, i, tools), [tools]);
  const out = r.state === null ? r.outs.flat() : [...r.outs.flat(), r.state];
  return out.length === 0 && templateTruthy(tools) ? [{role: "system", content: "", tools: tools}] : out;
}

// 2. Reorder tool results to match the tool_calls order of the preceding assistant message (the state: its call ids).
function reorder_step(ids, m) {
  ids = ids ?? [];
  if (m.role === "assistant" && templateTruthy(m.tool_calls)) {
    return [m.tool_calls.map(tc => templateTruthy(tc.id) ? tc.id : getOr(getOr(tc, "function", {}), "id", "")), m];
  }
  if (m.role === "user" && templateTruthy(m.content_blocks)) {
    const tb = m.content_blocks.filter(b => b.type === "tool_result").map(b => {
      let idx = 0;
      for (const [j, id] of ids.entries()) { if (templateTruthy(id) && id === getOr(b, "tool_use_id", "")) { idx = j; } }
      return {...b, _order: idx};
    });
    if (tb.length > 1) {
      const sorted_blocks = tb.sort((x, y) => x._order - y._order);
      let rb = [];
      let i = 0;
      for (const b of m.content_blocks) {
        if (b.type === "tool_result") { rb = [...rb, sorted_blocks[i]]; i = i + 1; }
        else { rb = [...rb, b]; }
      }
      return [ids, {...m, content_blocks: rb}];
    }
  }
  return [ids, m];
}

function reorder_results(msgs) {
  return memoScan(msgs, null, reorder_step, null).outs;
}

function user_text(m) {
  let parts = [];
  for (const b of m.content_blocks) {
    if (b.type === "text") {
      parts = [...parts, checkTextBlock(b)];
    } else if (isImageBlock(b)) {
      parts = [...parts, image(b)];
    } else if (b.type === "tool_result") {
      let tc = getOr(b, "content", "");
      if (Array.isArray(tc)) {
        tc = pyJoin(tc.map(s => s.type === "text" ? getOr(s, "text", "") : isImageBlock(s) ? image(s) : "[Unsupported " + pyText(s.type) + "]"), "\n\n");
      }
      parts = [...parts, p`<tool_result>${pyText(tc)}</tool_result>`];
    } else {
      parts = [...parts, "[Unsupported " + pyText(b.type) + "]"];
    }
  }
  return pyJoin(parts, "\n\n");
}

function tool_calls_text(m) {
  const lines = m.tool_calls.map(tc => {
    const f = tc.function;
    const args = isPlainObject(f.arguments) ? f.arguments : {arguments: f.arguments};
    const params = Object.entries(args).map(([k, v]) =>
      "<" + DSML + " parameter name=\"" + k + "\" string=\"" + (typeof v === "string" ? "true" : "false") + "\">" + (typeof v === "string" ? v : formatJSON(v)) + "</" + DSML + " parameter>");
    const name = qualified_name(f.name, templateTruthy(tc.namespace) ? tc.namespace : getOr(f, "namespace", null));
    return "<" + DSML + " invoke name=\"" + name + "\">\n" + params.join("\n") + "\n</" + DSML + " invoke>";
  });
  return "\n\n<" + DSML + " calls>\n" + lines.join("\n") + "\n</" + DSML + " calls>";
}

function effort_budget(e) {
  if (isNone(e)) { e = "high"; }
  const m = {low: 50, medium: 62, high: 75, max: 100};
  if (typeof e === "string" && e in m) { return m[e]; }
  if (Number.isInteger(e) && 1 <= e && e <= 100) { return e; }
  throw new Error("Invalid reasoning effort: " + formatPythonValue(e) + ", should be int within [1,100] or one of \"low\", \"medium\", \"high\", \"max\"");
}

const TASK_TOKENS = {action: tok.action, query: tok.query, authority: tok.authority, domain: tok.domain, title: tok.title, read_url: tok.read_url};


// What follows message `m` (at idx) when `next` is the message after it (null after the last):
// its task's tokens, or the generation prompt after a user message or a later system message.
function after(m, idx, next, c, add_generation_prompt) {
  if (next !== null && !["assistant", "latest_reminder"].includes(next.role)) return [];
  const task = getOr(m, "task", null);
  if (task !== null) {
    if (!(task in TASK_TOKENS)) throw new Error("Invalid task: '" + pyText(task) + "'");
    return [task === "action" && p`${tok.Assistant}${c.think ? tok.think : tok.think_end}`, TASK_TOKENS[task]];
  }
  if ((m.role === "user" || (m.role === "system" && idx > 0)) && (next !== null || templateTruthy(add_generation_prompt))) {
    return [p`${tok.Assistant}${c.think && (!c.effectiveDrop || idx >= c.lastUser) ? tok.think : tok.think_end}`];
  }
  return [];
}

// 3. One message, after what follows the one before it (the state: that message). That `lead`
// (the assistant's opener after a user message; the system header at idx 0) is kept apart: it is
// empty before a user or system message, where an exchange starts.
function render_step(prev, m, idx, c) {
  const lead = prev === null ? [] : after(prev, idx - 1, m, c, null);
  const out = [];
  let content = getOr(m, "content", null);
  if (Array.isArray(content)) content = joined_text(content);
  if (idx === 0) lead.push((c.think || m.role === "system") && p`${tok.System}`,
                          c.think && `Reasoning Effort: ${c.budget} (range 1-100, the higher the value, the more thorough the reasoning)\n\n`);
  if (m.role === "system") {
    out.push(idx > 0 && p`${tok.System}`, pyText(templateTruthy(content) ? content : ""),
             templateTruthy(m.tools) && "\n\n" + render_tools(m.tools),
             templateTruthy(m.response_format) && "\n\n## Response Format:\n\nYou MUST strictly adhere to the following schema to reply:\n" + formatJSON(m.response_format));
  } else if (m.role === "user") {
    out.push(p`${tok.User}${user_text(m)}`);
  } else if (m.role === "latest_reminder") {
    out.push(p`${tok.latest_reminder}${pyText(content)}`);
  } else if (m.role === "assistant" && m.generated) {
    // A model turn logged with exact ids: the turn before it already wrote the generation prompt
    // (Assistant and the think tag), so the ids follow verbatim, then the end unless they have it.
    out.push(tokens(m.ids), m.ids.at(-1) === tok.end_of_sentence.id ? "" : p`${tok.end_of_sentence}`);
  } else if (m.role === "assistant") {
    const prevHasTask = idx > 0 && !isNone(prev.task);
    const reasoning = !isNone(m.reasoning_content) ? m.reasoning_content : getOr(m, "thinking", null);
    out.push(c.think && !prevHasTask && (!c.effectiveDrop || idx > c.lastUser) &&
               p`${pyText(templateTruthy(reasoning) ? reasoning : "")}${tok.think_end}`,
             pyText(templateTruthy(content) ? content : ""),
             templateTruthy(m.tool_calls) && tool_calls_text(m),
             !templateTruthy(getOr(m, "wo_eos", false)) && p`${tok.end_of_sentence}`);
  } else {
    throw new Error("Unknown role: " + pyText(m.role));
  }
  return [m, {lead, out}];
}

export default function DeepSeekV41({ messages, tools, add_generation_prompt, thinking_mode, enable_thinking, drop_thinking, reasoning_effort }) {
  const mode = thinking_mode !== undefined ? thinking_mode
             : (enable_thinking !== undefined && enable_thinking !== null && !templateTruthy(enable_thinking)) ? "chat" : "thinking";
  if (!["chat", "thinking"].includes(mode)) throw new Error("Invalid thinking_mode `" + formatPythonValue(mode) + "`, should be \"chat\" or \"thinking\"");
  const drop = drop_thinking !== undefined ? drop_thinking : true;
  const budget = effort_budget(reasoning_effort);
  const msgs = reorder_results(merge_messages(messages, tools));
  const effectiveDrop = templateTruthy(drop) && !msgs.some(m => templateTruthy(m.tools));
  // The last user turn matters only when reasoning is dropped (it changes every turn: the scan's deps leave it out otherwise).
  const lastUser = effectiveDrop ? msgs.findLastIndex((m, i) => m.role === "user" || (m.role === "system" && i > 0)) : -1;
  const c = {think: mode === "thinking", effectiveDrop, lastUser, budget};
  // The last user turn moves with every new one, and a message reads it in one way only: whether
  // it comes after it (`idx > lastUser`, for its lead and for its own reasoning). So when
  // reasoning is dropped a message is written both ways once, and which is used is chosen
  // outside the scan: a new user turn re-renders nothing.
  const late = {...c, lastUser: -1}, early = {...c, lastUser: Infinity};
  // Only an assistant message (or a reminder) reads it: its own reasoning, and the opener written
  // before it. Any other message is written once.
  const r = memoScan(msgs, null, (prev, m, idx) => effectiveDrop && ["assistant", "latest_reminder"].includes(m.role)
    ? [m, {either: true, late: render_step(prev, m, idx, late)[1], early: render_step(prev, m, idx, early)[1]}]
    : render_step(prev, m, idx, c), [c.think, effectiveDrop, budget]);
  const items = r.outs.map((o, i) => o.either ? (i > lastUser ? o.late : o.early) : o);
  // A turn is an exchange: a user (or later system) message with the assistant answering it.
  return [<Turns messages={msgs} header={p`${tok.begin_of_sentence}`} items={items} joins={i => !["user", "system"].includes(msgs[i].role)}/>,
          r.state === null ? [] : after(r.state, msgs.length - 1, null, c, add_generation_prompt)];
}

export { DeepSeekV41 };
