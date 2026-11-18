// Port of NVIDIA-Nemotron-3-Nano's official tokenizer_config chat template.
"use strict_tokens";
import { NEMOTRON_TOOL_FORMAT } from "../third_party/nvidia/nemotron.tsx";
import { Turns } from "../common/turns.tsx";


// Jinja `x | tojson` for mappings and lists, `x | string` (Python str) for the rest.
const nemotronValue = v => isPlainObject(v) || Array.isArray(v) ? formatJSON(v) : formatPythonValue(v);

function extraKeys(obj, handled) {
  if (!isPlainObject(obj)) return "";
  return Object.entries(obj).filter(([k, v]) => !handled.includes(k)).map(([k, v]) =>
    "\n<" + k + ">" + nemotronValue(v) + "</" + k + ">");
}

function ToolDef({ tool }) {
  const t = tool.function !== undefined ? tool.function : tool;
  const params = t.parameters;
  return <>
    {"\n<function>\n<name>" + formatPythonValue(t.name) + "</name>"}
    {t.description !== undefined && "\n<description>" + t.description.trim() + "</description>"}
    {"\n<parameters>"}
    {isPlainObject(params) && isPlainObject(params.properties) && Object.entries(params.properties).map(([name, spec]) => <>
      {"\n<parameter>\n<name>" + name + "</name>"}
      {spec.type !== undefined && "\n<type>" + formatPythonValue(spec.type) + "</type>"}
      {spec.description !== undefined && "\n<description>" + spec.description.trim() + "</description>"}
      {spec.enum !== undefined && "\n<enum>" + formatJSON(spec.enum) + "</enum>"}
      {extraKeys(spec, ["name", "type", "description", "enum"])}
      {"\n</parameter>"}
    </>)}
    {extraKeys(params, ["type", "properties", "required"])}
    {params?.required !== undefined && "\n<required>" + formatJSON(params.required) + "</required>"}
    {"\n</parameters>"}
    {extraKeys(t, ["type", "name", "description", "parameters"])}
    {"\n</function>"}
  </>;
}

// The template edits assistant text around its think tags as strings, as the official one does;
// the tags it leaves are tokens (`tags`), not text, whatever the encoding of data text.
const tags = s => s.split(tok.think_end.text).flatMap((part, i) => [...(i ? [tok.think_end] : []), ...part.split(tok.think.text).flatMap((x, j) => j ? [tok.think, x] : [x])]);
const OPEN_CLOSED = [tok.think, tok.think_end];

function trimHistory(content) {
  if (content.includes(tok.think_end.text)) content = content.split(tok.think_end.text).at(-1);
  else if (content.includes(tok.think.text)) content = content.split(tok.think.text)[0];
  return (tok.think.text + tok.think_end.text) + content.trim();
}

function Assistant({ m, i, lastUser, truncate_history_thinking }) {
  // Jinja `content | default('', true)`: any Python-falsy content (also [] and {}) becomes "".
  const raw = m.content;
  const empty = !raw || (Array.isArray(raw) && raw.length === 0) || (isPlainObject(raw) && Object.keys(raw).length === 0);
  let content = empty ? "" : raw;
  if (typeof m.reasoning_content === "string" && m.reasoning_content.trim())
    content = (tok.think.text + "\n") + m.reasoning_content + ("\n" + tok.think_end.text + "\n") + content;
  else if (typeof content === "string" && !content.includes(tok.think.text) && !content.includes(tok.think_end.text))
    content = (tok.think.text + tok.think_end.text) + content;
  const cut = truncate_history_thinking && i < lastUser;
  if (Array.isArray(m.tool_calls) && m.tool_calls.length > 0) return <>
    {p`${tok.im_start}assistant\n`}
    {typeof content === "string" && content.trim() ? tags((cut ? trimHistory(content) : content.trim()) + "\n") : OPEN_CLOSED}
    {m.tool_calls.map(tc => {
      const f = tc.function !== undefined ? tc.function : tc;
      return <>
        {(tok.tool_call + "\n<function=") + f.name + ">\n"}
        {f.arguments !== undefined && Object.entries(f.arguments).map(([k, v]) =>
          "<parameter=" + k + ">\n" + nemotronValue(v) + "\n</parameter>\n")}
        {p`</function>\n${tok.tool_call_end}\n`}
      </>;
    })}
    {p`${tok.im_end}\n`}
  </>;
  if (cut && typeof content === "string" && content.includes(tok.think.text) && content.includes(tok.think_end.text))
    content = (tok.think.text + tok.think_end.text) + content.split(tok.think_end.text).at(-1);
  return <>{p`${tok.im_start}assistant\n`}{tags(asContent(content).trim())}{p`${tok.im_end}\n`}</>;
}

export function Nemotron3({ messages, tools = [], add_generation_prompt, enable_thinking = true, truncate_history_thinking = true }) {
  const sys = messages[0].role === "system" ? messages[0].content : "";
  const rest = messages[0].role === "system" ? messages.slice(1) : messages;
  const lastUser = rest.findLastIndex(m => m.role === "user");
  const header = <>
    {p`${tok.im_start}system\n${sys}`}
    {Array.isArray(tools) && tools.length > 0 && <>
      {sys && "\n\n"}
      {"# Tools\n\nYou have access to the following functions:\n\n<tools>"}
      {tools.map(t => <ToolDef tool={t}/>)}
      {"\n</tools>" + NEMOTRON_TOOL_FORMAT}
    </>}
    {p`${tok.im_end}\n`}
  </>;
  return <>
    <Turns messages={rest} header={header} start={0} message={(m, i) => {
      // A model turn logged with exact ids: the generation prompt, the ids, the turn end.
      if (m.role === "assistant" && m.generated) return [p`${tok.im_start}assistant\n${enable_thinking ? tok.think + "\n" : tok.think + tok.think_end}`, tokens(m.ids), m.ids.at(-1) === tok.im_end.id ? "\n" : p`${tok.im_end}\n`];
      if (m.role === "assistant") return <Assistant m={m} i={i} lastUser={lastUser} truncate_history_thinking={truncate_history_thinking}/>;
      if (m.role === "tool") return <>
        {i > 0 && rest[i - 1].role !== "tool" && p`${tok.im_start}user\n`}
        {p`${tok.tool_response}\n${m.content}\n${tok.tool_response_end}\n`}
        {(i === rest.length - 1 || rest[i + 1].role !== "tool") && p`${tok.im_end}\n`}
      </>;
      return p`${tok.im_start}${m.role}\n${asContent(m.content)}${tok.im_end}\n`;
    }}/>
    {add_generation_prompt && p`${tok.im_start}assistant\n${enable_thinking ? tok.think + "\n" : tok.think + tok.think_end}`}
  </>;
}
