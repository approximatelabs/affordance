// Meta Muse Glimmer 30B (ATEM tool protocol). Muse Spark has no open weights or public
// template (Meta says Muse Spark 1.2 will be open); Glimmer is the open Muse model.
"use strict_tokens";
import { MUSE_TOOLS_EXAMPLE, MUSE_TOOLS_INTRO } from "../third_party/meta-muse/glimmer.tsx";
import { templateItems, templateTruthy } from "../common/checks.tsx";

function render_content(c) {
  if (typeof c === "string" || isFragment(c)) { return c; }
  if (c === null) { return ""; }
  let out = "";
  for (const p of templateItems(c)) {
    if (p.type === "image") { out = out + tok.patch; }
    else if (p.type === "video") { out = out + tok.video; }
    else if (p.type === "text") { out = out + formatPythonValue(p.text); }
  }
  return out;
}

function atem_value(v) {
  if (typeof v === "boolean") { return v ? "true" : "false"; }
  if (v === null) { return "null"; }
  if (isPlainObject(v) || Array.isArray(v)) { return formatJSON(v); }
  return formatPythonValue(v);
}

function render_atem(tc) {
  let args = tc.function.arguments;
  if (!isPlainObject(args)) { throw new Error("Muse Glimmer ATEM chat template requires tool_call.function.arguments to be a dict (mapping); a JSON string cannot be parsed in the HF jinja sandbox."); }
  let out = "<atem:function_calls>\n<atem:invoke name=\"" + tc.function.name + "\">\n";
  for (const [k, v] of Object.entries(args)) { out = out + "<atem:parameter name=\"" + k + "\">" + atem_value(v) + "</atem:parameter>\n"; }
  return out + "</atem:invoke>\n</atem:function_calls>";
}

function fn_of(tool) { return tool.function !== undefined ? tool.function : tool; }

function tool_namespaces(tools) {
  let seen = [];
  for (const t of tools) {
    let ns = fn_of(t).name.split(".")[0];
    if (!seen.includes(ns)) { seen = [...seen, ns]; }
  }
  return seen;
}

function render_tool_defs(tools, tool_namespace_descriptions) {
  let out = MUSE_TOOLS_INTRO;
  let nd = tool_namespace_descriptions !== undefined ? tool_namespace_descriptions : {};
  for (const ns of tool_namespaces(tools)) {
    out = out + "{\"name\": " + formatJSON(ns) + ", \"description\": " + formatJSON(ns in nd ? nd[ns] : "") + "}\n";
  }
  out = out + "// Function schemas";
  for (const t of tools) {
    let f = fn_of(t);
    out = out + "\n{\"name\": " + formatJSON(f.name) + ", \"description\": " + formatJSON(f.description) + ", \"parameters\": " + formatJSON(f.parameters) + "}";
  }
  return out + MUSE_TOOLS_EXAMPLE;
}

function render_reasoning(reasoning_strength) {
  return "Reasoning strength: " + (templateTruthy(reasoning_strength) ? reasoning_strength : "high") + ".";
}

function render_system_meta(tools) {
  let r = ["\"self\""];
  if (templateTruthy(tools)) { for (const ns of tool_namespaces(tools)) { r.push("\"" + ns + ".*\""); } }
  r.push("\"user\"");
  return "# Valid recipients: " + r.join(", ") + ".";
}

function tool_name(m, messages) {
  let name = m.name;
  if (templateTruthy(name)) { return name; }
  let id = m.tool_call_id;
  let found = templateTruthy(id) ? id : "";
  for (const x of messages) {
    if (templateTruthy(x.tool_calls)) {
      for (const tc of x.tool_calls) { if (id !== undefined && id !== null && tc.id !== undefined && tc.id === id) { found = tc.function.name; } }
    }
  }
  return found;
}


export default function MuseGlimmer({ messages, tools, bos_token, add_generation_prompt, knowledge_cutoff, current_date, reasoning_strength, tool_namespace_descriptions }) {
  if (messages.length === 0) throw new Error("Cannot apply chat template to an empty conversation. Provide at least one message.");
  const lower = s => s.replaceAll("Reasoning effort", "Reasoning strength").replaceAll("Reasoning Effort", "Reasoning Strength")
                      .replaceAll("reasoning effort", "reasoning strength").replaceAll("REASONING EFFORT", "REASONING STRENGTH");
  const toolDefs = () => "\n\n" + render_tool_defs(tools, tool_namespace_descriptions);
  return [bos_token === undefined ? "" : formatPythonValue(bos_token),
    !messages.some(m => m.role === "system") && [
      p`${tok.start}system${tok.message}You are a helpful AI assistant.`,
      "\nKnowledge cutoff: " + (templateTruthy(knowledge_cutoff) ? knowledge_cutoff : "2026-01-04") + ".",
      templateTruthy(current_date) ? "\nCurrent date: " + current_date + "." : "\nCurrent date: " + formatNow("%Y-%m-%d") + ".",
      "\n\n" + render_reasoning(reasoning_strength), templateTruthy(tools) && toolDefs(), p`\n\n${render_system_meta(tools)}${tok.eot}`],
    messages.map((m, i) => {
      const endToken = i < messages.length - 1 && messages[i + 1].role === m.role ? tok.eom : tok.eot;
      if (m.role === "system") {
        const sys = lower(render_content(m.content));
        return [p`${tok.start}system${tok.message}${sys}`, !sys.toLowerCase().includes("reasoning strength") && "\n\n" + render_reasoning(reasoning_strength),
                templateTruthy(tools) && toolDefs(), p`\n\n${render_system_meta(tools)}${tok.eot}`];
      }
      if (m.role === "user") return p`${tok.start}user${tok.message}${render_content(m.content)}${tok.eot}`;
      if (m.role === "tool") {
        const name = tool_name(m, messages);
        return p`${(tok.start + "tool ") + name + (tok.message + "<tool_output name=\"") + name + "\">\n"}${render_content(m.content)}\n</tool_output>${tok.eot}`;
      }
      // A model turn logged with exact ids: the generation prompt, the ids, the end unless they have it.
      if (m.role === "assistant" && m.generated) return [p`${tok.start}assistant`, tokens(m.ids), [tok.eot.id, tok.eom.id].includes(m.ids.at(-1)) ? "" : endToken];
      if (m.role === "assistant") {
        const reasoning = templateTruthy(m.reasoning_content) && (tok.start + "assistant to=self" + tok.message) + m.reasoning_content + tok.eom;
        if (templateTruthy(m.tool_calls)) {
          return [reasoning, m.tool_calls.map((tc, j) => p`${(tok.start + "assistant to=") + tc.function.name + tok.message}${render_atem(tc)}${j === m.tool_calls.length - 1 ? endToken : tok.eom}`)];
        }
        const recipient = templateTruthy(m.recipient) ? m.recipient : "user";
        const endTurn = m.end_turn === undefined || m.end_turn === null ? !(templateTruthy(recipient) && recipient !== "user") : m.end_turn;
        return [reasoning, p`${tok.start}assistant${templateTruthy(recipient) && " to=" + recipient}${tok.message}${render_content(m.content)}${templateTruthy(endTurn) ? tok.eot : tok.eom}`];
      }
    }),
    add_generation_prompt && p`${tok.start}assistant`];
}

export { MuseGlimmer };
