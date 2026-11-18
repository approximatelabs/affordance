// XML-style tool definitions shared by Qwen3-Coder and MiMo-V2 (<function><name>..</name><parameters>..).
"use strict_tokens";

const isSeq = v => isPlainObject(v) || Array.isArray(v);

// Keys of `d` not in `handled`, as <key>value</key> lines (objects and arrays as JSON).
const ExtraKeys = ({ d, handled }) => isPlainObject(d) && Object.keys(d).filter(k => !handled.includes(k)).map(k =>
  p`\n<${k}>${isSeq(d[k]) ? formatJSON(d[k]) : formatPythonValue(d[k])}</${k}>`);

function ToolDef({ tool }) {
  const t = tool.function !== undefined ? tool.function : tool;
  const props = isPlainObject(t.parameters) && isPlainObject(t.parameters.properties) ? Object.entries(t.parameters.properties) : [];
  return p`\n<function>\n<name>${formatPythonValue(t.name)}</name>${t.description !== undefined && `\n<description>${t.description.trim()}</description>`}
<parameters>${props.map(([name, prm]) => p`
<parameter>
<name>${name}</name>${prm.type !== undefined && `\n<type>${formatPythonValue(prm.type)}</type>`}${prm.description !== undefined && `\n<description>${prm.description.trim()}</description>`}${
  <ExtraKeys d={prm} handled={["name", "type", "description"]}/>}
</parameter>`)}${<ExtraKeys d={t.parameters} handled={["type", "properties"]}/>}
</parameters>${<ExtraKeys d={t} handled={["type", "name", "description", "parameters"]}/>}
</function>`;
}

export { ToolDef, isSeq };
