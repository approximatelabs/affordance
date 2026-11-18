// OpenAI gpt-oss (Harmony).
"use strict_tokens";
import { BROWSER_TOOL, PYTHON_TOOL } from "../third_party/openai/gpt_oss.tsx";   // BROWSER_TOOL, PYTHON_TOOL (text copied from the template)
import { templateTruthy, textOperand } from "../common/checks.tsx";



// TypeScript-style signature of a JSON-schema parameter, as the template prints it.
function tsType(p, required) {
  if (p.type === "array") {
    let out = "any[]";
    if (p.items) {
      const it = p.items.type;
      if (it === "string") out = "string[]";
      else if (it === "number" || it === "integer") out = "number[]";
      else if (it === "boolean") out = "boolean[]";
      else {
        const inner = tsType(p.items, required);
        out = inner === "object | object" || inner.length > 50 ? "any[]" : inner + "[]";
      }
    }
    return p.nullable ? out + " | null" : out;
  }
  if (Array.isArray(p.type) && p.type[0] !== undefined) return p.type.length > 1 ? p.type.join(" | ") : formatPythonValue(p.type[0]);
  if (templateTruthy(p.oneOf)) {
    return p.oneOf.map((v, i) =>
      tsType(v, required) + (v.description ? "// " + textOperand(v.description) : "")
      // the template's bare `{{ }}` keeps 20 spaces of indentation here, and " | \n" between variants
      + (v.default !== undefined ? "                    // default: " + formatJSON(v.default) : "")
      + (i < p.oneOf.length - 1 ? " | \n" : "")).join("");
  }
  if (p.type === "string") return templateTruthy(p.enum) ? `"${p.enum.join("\" | \"")}"` : (p.nullable ? "string | null" : "string");
  if (p.type === "number" || p.type === "integer") return "number";
  if (p.type === "boolean") return "boolean";
  if (p.type === "object") {
    if (!templateTruthy(p.properties)) return "object";
    const req = p.required || [];
    // nested properties: the template's bare `{{ }}` keeps "\n" + 16 spaces before each type
    return "{\n" + Object.keys(p.properties).map(n =>
      n + (req.includes(n) ? "" : "?") + ": \n                " + tsType(p.properties[n], req)).join(", ") + "}";
  }
  return "any";
}

const Param = ({ name, spec, required }) => p`${spec.description && `// ${spec.description}\n`}${name}${!required.includes(name) && "?"}: ${tsType(spec, required)}${
  spec.default !== undefined && (templateTruthy(spec.enum) ? ", // default: " + textOperand(spec.default) : templateTruthy(spec.oneOf) ? "// default: " + textOperand(spec.default) : ", // default: " + formatJSON(spec.default))},\n`;

const ToolNamespace = ({ tools }) => p`## functions\n\nnamespace functions {\n\n${tools.map(t => {
  const f = t.function;
  const props = f.parameters && f.parameters.properties;
  // the template concatenates with +, so a missing description raises, as in Jinja
  return p`${"// " + textOperand(f.description) + "\n" + "type " + textOperand(f.name) + " = "}${templateTruthy(props)
    ? p`(_: {\n${Object.entries(props).map(kv => <Param name={kv[0]} spec={kv[1]} required={f.parameters.required || []}/>)}}) => any;\n\n`
    : "() => any;\n\n"}`;
})}} // namespace functions`;

const SystemMessage = ({ model_identity = "You are ChatGPT, a large language model trained by OpenAI.", reasoning_effort = "medium", builtin_tools, tools }) =>
  p`${tok.start}system${tok.message}${textOperand(model_identity) + "\n"}Knowledge cutoff: 2024-06
Current date: ${formatNow("%Y-%m-%d")}

Reasoning: ${textOperand(reasoning_effort)}

${templateTruthy(builtin_tools) && ["# Tools\n\n", builtin_tools.includes("browser") && BROWSER_TOOL, builtin_tools.includes("python") && PYTHON_TOOL]}# Valid channels: analysis, commentary, final. Channel must be included for every message.${
  templateTruthy(tools) && "\nCalls to these tools must go to the commentary channel: 'functions'."}${tok.end}`;

const CHANNEL_TAGS = [tok.channel.text + "analysis" + tok.message.text, tok.channel.text + "final" + tok.message.text];

export default function GptOss(state) {
  const { messages, tools, add_generation_prompt } = state;
  const hasDev = messages[0].role === "developer" || messages[0].role === "system";
  const dev = hasDev ? messages[0].content : "";
  const rest = hasDev ? messages.slice(1) : messages;
  let lastTool = null;
  let out = [<SystemMessage model_identity={state.model_identity} reasoning_effort={state.reasoning_effort} builtin_tools={state.builtin_tools} tools={tools}/>];
  if (templateTruthy(dev) || templateTruthy(tools)) {
    out.push(p`${tok.start}developer${tok.message}${templateTruthy(dev) && p`# Instructions\n\n${formatPythonValue(dev)}\n\n`}${templateTruthy(tools) && p`# Tools\n\n${<ToolNamespace tools={tools}/>}`}${tok.end}`);
  }
  for (const [i, m] of rest.entries()) {
    // A model turn logged with exact ids: the generation prompt, then its messages verbatim. A final
    // answer's <|return|> becomes <|end|> once the conversation goes on, as Harmony specifies.
    if (m.role === "assistant" && m.generated) {
      const ids = m.ids.at(-1) === tok.return.id ? [...m.ids.slice(0, -1), tok.end.id] : m.ids;
      out.push(p`${tok.start}assistant`, tokens(ids), [tok.end.id, tok.call.id].includes(ids.at(-1)) ? "" : p`${tok.end}`);
      lastTool = m.tool_calls?.length ? (m.tool_calls[0].function ?? m.tool_calls[0]).name : null;
      continue;
    }
    if (m.role === "assistant") {
      if ("content" in m && CHANNEL_TAGS.some(t => m.content.includes(t))) throw new Error("You have passed a message containing " + tok.channel.text + " tags in the content field. Instead of doing this, you should pass analysis messages (the string between '" + tok.message.text + "' and '" + tok.end.text + "') in the 'thinking' field, and final messages (the string between '" + tok.message.text + "' and '" + tok.end.text + "') in the 'content' field.");
      if ("thinking" in m && CHANNEL_TAGS.some(t => m.thinking.includes(t))) throw new Error("You have passed a message containing " + tok.channel.text + " tags in the thinking field. Instead of doing this, you should pass analysis messages (the string between '" + tok.message.text + "' and '" + tok.end.text + "') in the 'thinking' field, and final messages (the string between '" + tok.message.text + "' and '" + tok.end.text + "') in the 'content' field.");
      if ("tool_calls" in m) {
        const laterFinal = rest.slice(i + 1).some(f => f.role === "assistant" && !("tool_calls" in f));
        const c = templateTruthy(m.tool_calls[0].function) ? m.tool_calls[0].function : m.tool_calls[0];
        if (templateTruthy(m.content) && templateTruthy(m.thinking)) throw new Error("Cannot pass both content and thinking in an assistant message with tool calls! Put the analysis message in one or the other, but not both.");
        const analysis = !laterFinal && (templateTruthy(m.content) ? m.content : m.thinking);
        out.push(templateTruthy(analysis) && p`${tok.start}assistant${tok.channel}analysis${tok.message}${textOperand(analysis)}${tok.end}`,
                 p`${tok.start}assistant to=${"functions." + textOperand(c.name)}${tok.channel}commentary ${c.content_type !== undefined ? c.content_type : "json"}${tok.message}${formatJSON(c.arguments)}${tok.call}`);
        lastTool = c.name;
      } else if (i === rest.length - 1 && !add_generation_prompt) {
        out.push("thinking" in m && p`${tok.start}assistant${tok.channel}analysis${tok.message}${textOperand(m.thinking)}${tok.end}`,
                 p`${tok.start}assistant${tok.channel}final${tok.message}${textOperand(m.content)}${tok.return}`);
      } else {
        out.push(p`${tok.start}assistant${tok.channel}final${tok.message}${textOperand(m.content)}${tok.end}`);
        lastTool = null;
      }
    } else if (m.role === "tool") {
      if (lastTool === null) throw new Error("Message has tool role, but there was no previous assistant message with a tool call!");
      out.push(p`${tok.start}functions.${lastTool} to=assistant${tok.channel}commentary${tok.message}${formatJSON(m.content)}${tok.end}`);
    } else if (m.role === "user") {
      out.push(p`${tok.start}user${tok.message}${textOperand(m.content)}${tok.end}`);
    }
  }
  if (add_generation_prompt) out.push(p`${tok.start}assistant`);
  return out;
}

export { GptOss };
