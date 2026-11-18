// Ministral-3 (2512), Mistral-Small-4 (2603) and Mistral-Medium-3.5: [SYSTEM_PROMPT]/[INST]/[TOOL_CALLS]name[ARGS]
// format. `v4=true` adds [MODEL_SETTINGS] reasoning effort and [THINK] chunks (Small-4, Medium-3.5).
"use strict_tokens";
import { SYS_MEDIUM35, SYS_MINISTRAL3, SYS_SMALL4 } from "../third_party/mistralai/system_prompts.tsx";
import { templateItems } from "../common/checks.tsx";

const present = x => x !== undefined && x !== null;

// Laid-out content (a template app's screen) is text.
const isText = c => typeof c === "string" || isFragment(c);

function render_content(content, context_name, desc, support_thinking, support_images) {
  if (isText(content)) { return content; }
  if (!present(content) || content.length === 0) { throw new Error(context_name + " must have non-empty content."); }
  let out = "";
  for (const b of templateItems(content)) {
    if (b.type === "text") { out = out + b.text; }
    else if (support_thinking && b.type === "thinking") {
      out = out + tok.THINK + b.thinking;
      if ("closed" in b ? b.closed : true) { out = out + tok.THINK_end; }
    }
    else if (support_images && ["image", "image_url"].includes(b.type)) { out = out + tok.IMG; }
    else { throw new Error("Only " + desc + " chunks are supported in " + context_name + "."); }
  }
  return out;
}

// Merge consecutive messages with the same role (not system/tool) into one. A model turn logged
// with exact ids stays as it is (its ids are the turn).
function aggregate(messages, v4) {
  let out = [];
  let group = [];
  let role = null;
  for (const message of [...messages, {role: "__sentinel__"}]) {
    if (message.role !== role || message.role === "system" || message.role === "tool" || message.generated || group[0]?.generated) {
      if (role === "tool" || group[0]?.generated) { out = [...out, ...group]; }
      else if (role !== null) {
        let text_parts = [];
        let chunks = [];
        let has_non_text = false;
        let calls = [];
        for (const original of group) {
          let msg = original;
          if (v4) {
            const reasoning = "reasoning_content" in msg ? msg.reasoning_content : ("reasoning" in msg ? msg.reasoning : null);
            if (reasoning !== null && reasoning !== "") {
              if (msg.content !== null && typeof msg.content !== "string") {
                for (const b of msg.content) { if (b.type === "thinking") { throw new Error("Message cannot have both thinking chunks in content and a top-level `reasoning` or `reasoning_content` field."); } }
              }
              const tc = {type: "thinking", thinking: reasoning};
              let nc = [tc];
              if (typeof msg.content === "string" && msg.content !== "") { nc = [tc, {type: "text", text: msg.content}]; }
              else if (msg.content !== null && typeof msg.content !== "string" && msg.content.length > 0) { nc = [tc, ...msg.content]; }
              if (present(msg.tool_calls)) { msg = {role: msg.role, content: nc, tool_calls: msg.tool_calls}; }
              else { msg = {role: msg.role, content: nc}; }
            }
          }
          if (isText(msg.content)) { text_parts.push(msg.content); }
          else if (msg.content !== null) {
            let mparts = [];
            for (const b of msg.content) {
              if (b.type === "text") {
                if (v4) { mparts.push(b.text); } else { text_parts.push(b.text); }
              } else {
                if (mparts.length > 0) { text_parts.push(mparts.join("")); mparts = []; }
                if (text_parts.length > 0) { chunks.push({type: "text", text: text_parts.join("\n\n")}); text_parts = []; }
                chunks.push(b);
                has_non_text = true;
              }
            }
            if (mparts.length > 0) { text_parts.push(mparts.join("")); }
          }
          if (present(msg.tool_calls)) { calls = [...calls, ...msg.tool_calls]; }
        }
        // Laid-out parts are joined by concatenation (keeping their tokens); plain ones as Python does.
        let merged = text_parts.some(x => isFragment(x)) ? text_parts.reduce((a, b) => a + "\n\n" + b) : text_parts.join("\n\n");
        if (has_non_text) {
          if (text_parts.length > 0) { chunks.push({type: "text", text: text_parts.join("\n\n")}); }
          merged = chunks;
        }
        if (calls.length > 0) { out.push({role: role, content: merged, tool_calls: calls}); }
        else { out.push({role: role, content: merged}); }
      }
      if (message.role !== "__sentinel__") { group = [message]; role = message.role; }
    } else {
      group.push(message);
    }
  }
  return out;
}

const ALLOWED_AFTER = {system: ["user", "assistant", "system"], user: ["assistant", "system", "user"], assistant: ["assistant", "user", "tool"], tool: ["assistant", "tool", "user"]};

function check_order(msgs) {
  if (msgs.length > 0 && !["user", "system"].includes(msgs[0].role)) { throw new Error("Conversation must start with a user or system message, got " + msgs[0].role + "."); }
  let prev = null;
  for (const m of msgs) {
    if (prev !== null && prev in ALLOWED_AFTER && !ALLOWED_AFTER[prev].includes(m.role)) {
      throw new Error("Unexpected role '" + m.role + "' after role '" + prev + "'");
    }
    prev = m.role;
  }
  return true;
}


export default function Mistral3({ messages, tools, bos_token, eos_token, reasoning_effort, default_system = SYS_MINISTRAL3, v4 = false }) {
  if (messages.length === 0) throw new Error("Cannot apply chat template to an empty conversation. Provide at least one message.");
  const availableTools = present(tools) && tools.length > 0 ? tok.AVAILABLE_TOOLS + formatJSON(tools) + tok.AVAILABLE_TOOLS_end : "";
  let settings = "";
  if (v4) {
    const effort = reasoning_effort ?? "none";
    if (!["none", "high"].includes(effort)) throw new Error("reasoning_effort must be either \"none\" or \"high\"");
    settings = (tok.MODEL_SETTINGS + "{\"reasoning_effort\": \"") + effort + ("\"}" + tok.MODEL_SETTINGS_end);
  }
  const msgs = aggregate(messages, v4);
  check_order(msgs);
  const firstUser = msgs.findIndex(m => m.role === "user");
  return [bos_token,
    messages[0].role !== "system" && default_system !== "" && tok.SYSTEM_PROMPT + default_system + tok.SYSTEM_PROMPT_end,
    msgs.map((m, i) => {
      if (m.role === "user") {
        const c = m.content;
        const uc = !isText(c) && present(c) && c.length === 2 && c[0].type === "text" && ["image", "image_url"].includes(c[1].type) ? [c[1], c[0]] : c;
        return <>{i === firstUser && [availableTools, settings]}{p`${tok.INST}${render_content(uc, "user message content", "text, image and image_url", false, true)}${tok.INST_end}`}</>;
      }
      // A model turn logged with exact ids: what the model wrote after [/INST], verbatim.
      if (m.role === "assistant" && m.generated) return [tokens(m.ids), m.ids.at(-1) === tok.s_end.id ? "" : eos_token];
      if (m.role === "assistant") {
        const calls = present(m.tool_calls) ? m.tool_calls : [];
        const noContent = !present(m.content) || m.content.length === 0;
        if (noContent && calls.length === 0) {
          throw new Error("Assistant message must have a string or a list of chunks in content or a list of tool calls.");
        }
        return [!noContent && render_content(m.content, "assistant message contents", v4 ? "text and thinking" : "text", v4, false),
                calls.map(t => {
                  const args = typeof t.function.arguments !== "string" ? formatJSON(t.function.arguments) : t.function.arguments === "" ? "{}" : t.function.arguments;
                  return p`${tok.TOOL_CALLS}${t.function.name + tok.ARGS + args}`;
                }),
                eos_token];
      }
      if (m.role === "tool") return v4 ? p`${tok.TOOL_RESULTS}${render_content(m.content, "tool message contents", "text and image", false, true)}${tok.TOOL_RESULTS_end}`
                                       : tok.TOOL_RESULTS + formatPythonValue(m.content) + tok.TOOL_RESULTS_end;
      if (m.role === "system") return p`${tok.SYSTEM_PROMPT}${render_content(m.content, "system message contents", "text", false, false)}${tok.SYSTEM_PROMPT_end}`;
      throw new Error("Only user, assistant, system and tool roles are supported, got " + m.role + ".");
    })];
}

export const MistralSmall4 = state => Mistral3({...state, default_system: SYS_SMALL4, v4: true});
export const MistralMedium35 = state => Mistral3({...state, default_system: SYS_MEDIUM35, v4: true});

export { Mistral3 };
