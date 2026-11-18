// Mistral-Small-3.2-2506 (tokenizer V11) and Magistral-Small-2509 (V13): port of the official
// encoder mistral_common (ChatCompletionRequest.from_openai -> normalizer -> InstructTokenizerV11/V13).
// mistral_common never reads control tokens out of message text, so this view uses safe text:
// control tokens come only from literal markup ([INST] etc.), text is always text.
"use strict_tokens";
"use safe_text";

const present = x => x !== undefined && x !== null;

// json.loads inside try/except: undefined on any failure.
function tryParseJSON(a) {
  try { return JSON.parse(a); } catch (e) { return undefined; }
}

// ---- from_openai: messages -> mistral_common messages ----

function mc_args(a) {
  // FunctionCall stores dict args as JSON; the normalizer and _prepare_function_call
  // round-trip them through json.loads / json.dumps(ensure_ascii=False). Its pydantic
  // validator only coerces a dict (or None); any other type is rejected.
  if (isPlainObject(a)) { return formatJSON(a); }
  if (a === null || a === "") { return "{}"; }
  if (typeof a !== "string") { throw new Error("ValidationError: ToolCall.function.arguments Input should be a valid string"); }
  const parsed = tryParseJSON(a);
  return parsed === undefined ? a : formatJSON(parsed);
}

function chunk_reject(p) {
  // ChunkTypes rejects an unknown type before the later "unknown chunk" branch.
  // A bad image_url scheme fails later, in image_from_chunk, with the URL itself.
  const t = p.type ?? null;
  if (t === null) { return "Content chunk must have a type field."; }
  if (!["text", "image", "image_url", "input_audio", "audio_url", "thinking"].includes(t)) {
    return formatPythonRepr(t) + " is not a valid ChunkTypes";
  }
  if (t === "image_url") {
    const raw = p.image_url ?? null;
    const url = typeof raw === "string" ? raw : (isPlainObject(raw) ? raw.url ?? null : undefined);
    if (typeof url === "string" && !url.startsWith("data:image") && !url.startsWith("file://") && !url.startsWith("http")) {
      return "Unsupported image url scheme " + url;
    }
  }
  // ImageChunk.from_openai asserts the OpenAI type is image_url, so a bare
  // image chunk's message is the chunk itself. Other unported kinds keep that.
  return formatPythonValue(p);
}

// Laid-out content (a template app's screen) is text.
const isText = c => typeof c === "string" || isFragment(c);

function mc_chunks(content) {
  if (isText(content) || content === null || content === undefined) { return content; }
  // A dict or a number is not a chunk list. mistral_common raises ValueError before iteration.
  if (isPlainObject(content)) { throw new Error("Unknown content type: <class 'dict'>"); }
  if (Number.isInteger(content)) { throw new Error("Unknown content type: <class 'int'>"); }
  let out = [];
  for (const p of content) {
    if (p.type === "text") { out.push({kind: "text", text: p.text}); }
    else if (p.type === "thinking") { out.push({kind: "think", thinking: p.thinking, closed: "closed" in p ? p.closed : true}); }
    else { throw new Error(chunk_reject(p)); }
  }
  return out;
}

function has_think(content) { return Array.isArray(content) && content.some(c => c.kind === "think"); }

function from_openai(messages) {
  let out = [];
  for (const m of messages) {
    if (m.role === "assistant") {
      let content = mc_chunks(m.content ?? null);
      let think = m.reasoning_content ?? null;
      if (think === null) { think = m.reasoning ?? null; }
      if (think !== null) {
        if (has_think(content)) { throw new Error("Message cannot have both thinking chunks in `content` and a top-level `reasoning` or `reasoning_content` field."); }
        const tc = {kind: "think", thinking: think, closed: true};
        if (typeof content === "string") { content = [tc, {kind: "text", text: content}]; }
        else if (content === null) { content = [tc]; }
        else { content = [tc, ...content]; }
      }
      let calls = [];
      for (const c of (m.tool_calls || [])) { calls.push({id: c.id ?? null, name: c.function.name, args: mc_args(c.function.arguments)}); }
      // A model turn logged with exact ids keeps them (its calls are metadata, for the pairing).
      out.push(m.generated ? {role: "assistant", content: content, tool_calls: calls, generated: true, ids: m.ids} : {role: "assistant", content: content, tool_calls: calls});
    } else if (["user", "system", "tool"].includes(m.role)) {
      // UserMessage.content is not optional in mistral_common: a null content fails pydantic.
      if (m.role === "user" && (m.content ?? null) === null) { throw new Error("ValidationError: UserMessage.content Input should be a valid string"); }
      // mistral_common converts a tool message by reading its `tool_call_id`: none is a KeyError.
      if (m.role === "tool" && !("tool_call_id" in m)) { throw new TypeError("KeyError: 'tool_call_id'"); }
      out.push({role: m.role, content: mc_chunks(m.content ?? null), tool_call_id: m.tool_call_id ?? null, name: m.name ?? null});
    } else {
      throw new Error("Unknown message role: " + formatPythonValue(m.role));
    }
  }
  return out;
}

// ---- normalizer (V7/V13): merge consecutive same-role messages ----

// Coalesce text across contents: "\n\n" between messages and between chunks. Returns a
// string if only text was seen, else a list of chunks.
function aggregate(contents) {
  let all = [];
  let cur = [];
  for (const c of contents) {
    const sep = cur.length > 0;
    if (!present(c) || c.length === 0) { continue; }
    if (isText(c)) { cur.push(sep ? "\n\n" + c : c); }
    else {
      for (const ch of c) {
        if (ch.kind === "text") {
          if (!ch.text) { continue; }
          cur.push((cur.length === 0 ? "" : "\n\n") + ch.text);
        } else {
          if (cur.length > 0) { all.push({kind: "text", text: cur.join("")}); cur = []; }
          all.push(ch);
        }
      }
    }
  }
  // Joined by concatenation: laid-out parts keep their tokens.
  const joined = cur.length === 0 ? "" : cur.reduce((a, b) => a + b);
  if (all.length === 0) { return joined; }
  if (cur.length > 0) { all.push({kind: "text", text: joined}); }
  return all;
}

function flush_group(group, role, call_ids, v13) {
  if (role === "tool") {
    let msgs = [];
    for (let i = 0; i < group.length; i++) {
      const c = aggregate([group[i].content]);
      if (!isText(c)) { throw new Error("AssertionError: tool content must be text"); }
      const rank = call_ids.indexOf(group[i].tool_call_id);
      msgs.push({role: "tool", content: c, tool_call_id: group[i].tool_call_id, _k: rank === -1 ? 1000000 : rank});
    }
    // Stable sort by call order; V11 and V13 both use InstructRequestNormalizerV13.
    return msgs.sort((x, y) => x._k - y._k);
  }
  if (role === "assistant" && group[0].generated) { return group; }
  if (role === "assistant") {
    let calls = [];
    for (const m of group) { calls = [...calls, ...m.tool_calls]; }
    return [{role: "assistant", content: aggregate(group.map(m => m.content)), tool_calls: calls}];
  }
  if (role === "user") { return [{role: "user", content: aggregate(group.map(m => m.content))}]; }
  if (role === "system") { return group.map(m => ({role: "system", content: aggregate([m.content])})); }
  return [];
}

function normalize(msgs, v13) {
  let out = [];
  let group = [];
  let role = null;
  let call_ids = [];
  for (const m of [...msgs, {role: "__end__"}]) {
    // A generated turn is never merged: its ids are the turn.
    if (m.role !== role || m.generated || group[0]?.generated) {
      out = [...out, ...flush_group(group, role, call_ids, v13)];
      if (role === "assistant") { for (const c of out.at(-1).tool_calls) { call_ids.push(c.id); } }
      else if (role === "tool") { call_ids = []; }
      group = [];
      role = m.role;
    }
    group.push(m);
  }
  return out;
}

// _prepare_function_call: json.loads if possible, then json.dumps(ensure_ascii=False).
function args_text(a) {
  const parsed = tryParseJSON(a);
  return formatJSON(parsed === undefined ? a : parsed);
}

function tools_json(tools) {
  let out = [];
  for (const t of tools) {
    const f = t.function;
    const params = f.parameters ?? null;
    out.push({type: "type" in t ? t.type : "function", function: {name: f.name, description: f.description ?? "", parameters: params === null ? {} : params}});
  }
  return formatJSON(out);
}

// ---- validator (MistralRequestValidatorV11 / V13), run on the converted request ----

const ALNUM = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
function ascii_alnum(x) { return [...x].every(c => ALNUM.includes(c)); }
function ascii_name(x) { return typeof x === "string" && x.length >= 1 && x.length <= 64 && [...x].every(c => (ALNUM + "_-").includes(c)); }

function check_id(id, v13, what) {
  if (v13) {
    if (!id || id === "null") { throw new Error(what + ": invalid tool call id"); }
  } else if (!(typeof id === "string" && id.length === 9 && ascii_alnum(id))) {
    // The exception class is not part of the message body the reference stores.
    throw new Error("Tool call id was " + formatPythonValue(id) + " but must be a-z, A-Z, 0-9, with a length of 9.");
  }
}

function validate(msgs, finetuning, v13, tools) {
  const n = msgs.length;
  if (n === 0) { throw new Error("Conversation must have at least one message"); }
  if (n === 1 && !["user", "system"].includes(msgs[0].role)) { throw new Error("Conversation must start with a user message or system message"); }
  if (finetuning || n > 1) {
    const last = msgs.at(-1).role;
    if (finetuning && last !== "assistant") { throw new Error("Expected last role Assistant for finetuning"); }
    if (!finetuning && last === "assistant") { throw new Error("Expected last role User or Tool (or Assistant with prefix) for serving"); }
  }
  const allowed = {system: ["user", "assistant", "system"], user: ["assistant", "system", "user"], assistant: ["assistant", "user", "tool"], tool: ["assistant", "tool", "user"]};
  for (let i = 1; i < n; i++) {
    if (!allowed[msgs[i - 1].role].includes(msgs[i].role)) { throw new Error("Unexpected role '" + msgs[i].role + "' after role '" + msgs[i - 1].role + "'"); }
  }
  // tool call / result pairing by id (the first message is skipped, as in mistral_common)
  let expected = [];
  let observed = [];
  for (const m of msgs.slice(1)) {
    if (m.role === "tool") {
      if (observed.includes(m.tool_call_id)) { throw new Error("Duplicate tool call id in tool results"); }
      if (!expected.includes(m.tool_call_id)) { throw new Error("Unexpected tool call id " + formatPythonValue(m.tool_call_id) + " in tool results"); }
      observed.push(m.tool_call_id);
    } else if (m.role === "assistant") {
      if (expected.length !== observed.length) { throw new Error("Not the same number of function calls and responses"); }
      expected = [];
      observed = [];
      for (const c of m.tool_calls) {
        if (expected.includes(c.id)) { throw new Error("Duplicate tool call id in assistant message"); }
        expected.push(c.id);
      }
    }
  }
  if (finetuning && expected.length < observed.length) { throw new Error("More tool responses than tool calls"); }
  // per-message content
  for (const m of msgs) {
    if (m.role === "assistant") {
      for (const c of m.tool_calls) {
        check_id(c.id, v13, "InvalidFunctionCallException");
        if (!ascii_name(c.name)) { throw new Error("Function name was " + formatPythonValue(c.name) + " but must be a-z, A-Z, 0-9, or contain underscores and dashes, with a maximum length of 64."); }
      }
    } else if (m.role === "tool") {
      if (present(m.name) && !ascii_name(m.name)) { throw new Error("Invalid tool message name"); }
      if (!present(m.tool_call_id)) { throw new Error("Tool call id has to be defined."); }
      check_id(m.tool_call_id, v13, "InvalidToolMessageException");
    } else if (m.role === "system") {
      if (m.content === null) { throw new Error("System prompt must have content"); }
    }
  }
  for (const t of (tools || [])) {
    if (!ascii_name(t.function.name)) { throw new Error("Invalid function name in tools"); }
  }
  return true;
}

// ---- encoder (InstructTokenizerV11 / V13) ----


// Control tokens only come from p`...` literals here: under "use safe_text", ${...} values are text.
const Chunks = ({ content, v13 }) => content.map(ch => {
  if (ch.kind === "think") {
    if (!v13) throw new Error("Think not implemented for tokenizer < V13.");
    return ch.closed === false ? p`${tok.THINK}${ch.thinking}` : p`${tok.THINK}${ch.thinking}${tok.THINK_end}`;
  }
  return ch.text;
});

const Content = ({ c, v13 }) => isText(c) ? c : <Chunks content={c} v13={v13}/>;

export default function MistralCommon({ messages, tools, v13 = false }) {
  const converted = from_openai(messages);
  validate(converted, messages.length > 0 && messages.at(-1).role === "assistant", v13, tools);
  const msgs = normalize(converted, v13);
  const firstUser = msgs.findIndex(m => m.role === "user");
  const lastUser = msgs.findLastIndex(m => m.role === "user");
  const hasTools = present(tools) && tools.length > 0;
  return [p`${tok.s}`, msgs.map((m, i) => {
    if (m.role === "user") return <>
      {hasTools && (v13 ? i === firstUser : i === lastUser) && p`${tok.AVAILABLE_TOOLS}${tools_json(tools)}${tok.AVAILABLE_TOOLS_end}`}
      {p`${tok.INST}${<Content c={m.content} v13={v13}/>}${tok.INST_end}`}
    </>;
    if (m.role === "tool") return v13 ? p`${tok.TOOL_RESULTS}${m.content}${tok.TOOL_RESULTS_end}`
                                      : p`${tok.TOOL_RESULTS}${m.tool_call_id}${tok.TOOL_CONTENT}${m.content}${tok.TOOL_RESULTS_end}`;
    // A model turn logged with exact ids: what the model wrote after [/INST], verbatim.
    if (m.role === "assistant" && m.generated) return [tokens(m.ids), m.ids.at(-1) === tok.s_end.id ? "" : p`${tok.s_end}`];
    if (m.role === "assistant") {
      const hasContent = present(m.content) && m.content.length > 0;
      if (!hasContent && m.tool_calls.length === 0) {
        // InstructTokenizerV7.encode_assistant_message: str(AssistantMessage) with defaults.
        const content = present(m.content) ? formatPythonRepr(m.content) : "None";
        // No calls: AssistantMessage stores None, even when the normalized list is empty.
        throw new Error("Invalid assistant message: role='assistant' content=" + content + " tool_calls=None prefix=False");
      }
      return [hasContent && (typeof m.content === "string" ? trimEndChars(m.content, " ") : <Chunks content={m.content} v13={v13}/>),
              m.tool_calls.map(c => p`${tok.TOOL_CALLS}${c.name}${!v13 && c.id && c.id !== "null" ? p`${tok.CALL_ID}${c.id}` : ""}${tok.ARGS}${args_text(c.args)}`),
              p`${tok.s_end}`];
    }
    if (m.role === "system") return p`${tok.SYSTEM_PROMPT}${<Content c={m.content} v13={v13}/>}${tok.SYSTEM_PROMPT_end}`;
  })];
}

export const Magistral = state => MistralCommon({...state, v13: true});

export { MistralCommon };
