// Kimi K2.5, K2.6 and K2.7 show a model its tools as TypeScript-style declarations. Moonshot's
// tokenizer class builds that text in Python (tool_declaration_ts.py in each model repository)
// before the chat template runs, and passes it to the template as `tools_ts_str`. This file is
// that code, function for function, so the view gives the template the same text.
import { J } from "../../common/jinja.tsx";

const INDENT = "  ";
const DELIMITER = ",\n";
const fail = message => { throw new Error(message); };
const isDict = v => isPlainObject(v);
// Python's `d.get(key, fallback)`.
const get = (d, key, fallback) => key in d ? d[key] : fallback;
const formatDescription = (description, indent = "") =>
  description.split("\n").map(line => line ? `${indent}// ${line}` : "").join("\n");
// Whether Python would call the number an int (`1`), not a float (`1.0`).
const isInt = v => typeof v === "number" && formatPythonValue(v).indexOf(".") < 0 && formatPythonValue(v).indexOf("e") < 0;

// What every type keeps: its description and the constraints it prints.
function base(props, allowed = []) {
  const constraints = {};
  for (const k of Object.keys(props)) if (allowed.includes(k)) constraints[k] = props[k];
  return {description: get(props, "description", ""), constraints};
}
function docstring(type, indent) {
  const lines = [];
  if (J.t(type.description)) lines.push(formatDescription(type.description, indent));
  const keys = Object.keys(type.constraints).sort((a, b) => a < b ? -1 : a > b ? 1 : 0);
  if (keys.length) lines.push(`${indent}// ${keys.map(k => `${k}: ${J.str(type.constraints[k])}`).join(", ")}`);
  return lines.map(x => x + "\n").join("");
}

function scalar(type, props) {
  const allowed = type === "string" ? ["maxLength", "minLength", "pattern"]
    : type === "number" || type === "integer" ? ["maximum", "minimum"] : [];
  return {kind: "scalar", type, ...base(props || {}, allowed)};
}

function object(schema, registry) {
  const type = {kind: "object", ...base(schema), properties: [], additional: null};
  if (!J.t(schema)) return type;
  if ("$defs" in schema && registry && J.t(schema["$defs"])) {
    for (const name of Object.keys(schema["$defs"])) registry.definitions[name] = schema["$defs"][name];
  }
  type.additional = get(schema, "additionalProperties", null);
  if (isDict(type.additional)) type.additional = {parsed: parse(type.additional, registry)};
  if (!("properties" in schema)) return type;
  const required = get(schema, "required", []);
  type.properties = Object.keys(schema.properties).map(name => {
    const prop = schema.properties[name];
    return {name, type: parse(prop, registry), optional: !required.includes(name), default: isDict(prop) ? get(prop, "default", null) : null};
  });
  return type;
}

function array(schema, registry) {
  return {kind: "array", ...base(schema, ["minItems", "maxItems"]),
    item: J.t(get(schema, "items", null)) ? parse(schema.items, registry) : scalar("any")};
}

function enumeration(schema) {
  if ("type" in schema) {
    let typ = schema.type;
    if (Array.isArray(typ)) {
      if (typ.length === 1) typ = typ[0];
      else if (typ.length === 2) {
        if (!typ.includes("null")) fail(`Enum type ${J.str(typ)} is not supported`);
        typ = typ[0] !== "null" ? typ[0] : typ[1];
      } else fail(`Enum type ${J.str(typ)} is not supported`);
    }
    for (const val of schema.enum) {
      if (val === null) continue;
      // Python counts a boolean as an int.
      const number = typeof val === "number" || typeof val === "boolean";
      if (typ === "string" && typeof val !== "string") fail(`Enum value ${J.str(val)} is not a string`);
      else if (typ === "number" && !number) fail(`Enum value ${J.str(val)} is not a number`);
      else if (typ === "integer" && !(typeof val === "boolean" || isInt(val))) fail(`Enum value ${J.str(val)} is not an integer`);
      else if (typ === "boolean" && typeof val !== "boolean") fail(`Enum value ${J.str(val)} is not a boolean`);
    }
  }
  return {kind: "enum", ...base(schema), values: schema.enum};
}

const UNION = {string: "string", number: "number", integer: "number", boolean: "boolean", null: "null", object: "{}", array: "Array<any>"};
function union(schema) {
  return {kind: "union", ...base(schema), types: schema.type.map(t => t in UNION ? UNION[t] : fail(`KeyError: ${J.str(t)}`))};
}

function reference(schema, registry) {
  const ref = schema["$ref"];
  if (ref === "#") {
    registry.selfRef = true;
    return {kind: "ref", ...base(schema), name: "parameters"};
  }
  if (typeof ref === "string" && ref.startsWith("#/$defs/")) {
    const name = ref.split("/").at(-1);
    if (!(name in registry.definitions)) fail(`Reference not found: ${ref}`);
    return {kind: "ref", ...base(schema), name};
  }
  return fail(`Unsupported reference format: ${J.str(ref)}`);
}

function parse(schema, registry) {
  if (typeof schema === "boolean") return scalar(schema ? "any" : "null");
  if (!isDict(schema)) return fail(`Invalid JSON Schema object: ${J.str(schema)}`);
  if ("$ref" in schema && registry) return reference(schema, registry);
  if ("anyOf" in schema) return {kind: "anyOf", ...base(schema), types: schema.anyOf.map(t => parse(t, registry))};
  if ("enum" in schema) return enumeration(schema);
  if ("type" in schema) {
    const typ = schema.type;
    if (Array.isArray(typ)) return union(schema);
    if (typ === "object") return object(schema, registry);
    if (typ === "array") return array(schema, registry);
    return scalar(typ, schema);
  }
  if (Object.keys(schema).length === 0) return scalar("any");
  return fail(`Invalid JSON Schema object: ${J.str(schema)}`);
}

// One parameter of a function, or one field of an object.
function field(p, indent = "") {
  let comments = docstring(p.type, indent);
  if (p.default !== null && p.default !== undefined) {
    const plain = typeof p.default === "number" || typeof p.default === "boolean";
    comments += `${indent}// Default: ${plain ? formatPythonValue(p.default) : J.filter("tojson", p.default)}\n`;
  }
  return comments + `${indent}${p.name}${p.optional ? "?" : ""}: ${ts(p.type, indent)}`;
}

const byName = (a, b) => a.name < b.name ? -1 : a.name > b.name ? 1 : 0;
function ts(type, indent = "") {
  if (type.kind === "scalar") return type.type === "integer" ? "number" : J.str(type.type);
  if (type.kind === "object") {
    // The required fields first, each group by name.
    const fields = [...type.properties.filter(p => !p.optional).sort(byName), ...type.properties.filter(p => p.optional).sort(byName)]
      .map(p => field(p, indent + INDENT));
    if (type.additional !== null) {
      const extra = type.additional === true ? "any" : type.additional === false ? "never"
        : isDict(type.additional) && "parsed" in type.additional ? ts(type.additional.parsed, indent + INDENT)
        : fail(`Unknown additionalProperties: ${J.str(type.additional)}`);
      fields.push(`${indent + INDENT}[k: string]: ${extra}`);
    }
    if (!fields.length) return "{}";
    return `{\n${fields.join(DELIMITER)}\n${indent}}`;
  }
  if (type.kind === "array") {
    const doc = docstring(type.item, indent + INDENT);
    return doc ? "Array<\n" + doc + indent + INDENT + ts(type.item, indent + INDENT) + "\n" + indent + ">" : `Array<${ts(type.item, indent)}>`;
  }
  if (type.kind === "enum") return type.values.map(e => typeof e === "string" ? `"${e}"` : J.str(e)).join(" | ");
  if (type.kind === "anyOf") return type.types.map(t => ts(t, indent)).join(" | ");
  if (type.kind === "union") return type.types.join(" | ");
  return type.name;
}

// One OpenAI function definition as its TypeScript-style declaration.
function declaration(fn) {
  const registry = {definitions: {}, selfRef: false};
  const parsed = object(J.t(get(fn, "parameters", null)) ? fn.parameters : {}, registry);
  const interfaces = [];
  const root = registry.selfRef ? "parameters" : null;
  if (root) {
    const fields = parsed.properties.map(p => field(p, INDENT)).join(DELIMITER);
    interfaces.push(`interface ${root} {${fields ? `\n${fields}\n` : ""}}`);
  }
  for (const name of Object.keys({...registry.definitions})) {
    const schema = registry.definitions[name];
    const body = ts(parse(schema, registry));
    const description = isDict(schema) ? get(schema, "description", "") : "";
    interfaces.push(`${J.t(description) ? formatDescription(description) + "\n" : ""}interface ${name} ${body}`);
  }
  const name = get(fn, "name", "function");
  const typeDef = root ? `type ${J.str(name)} = (_: ${root}) => any;` : `type ${J.str(name)} = (_: ${ts(parsed)}) => any;`;
  const description = get(fn, "description", null);
  return [interfaces.join("\n"), J.t(description) ? formatDescription(description) : "", typeDef].filter(x => J.t(x)).join("\n");
}

// The tools as the text Kimi's template prints, or "" when there is no function among them.
function encode(tools) {
  if (!J.t(tools)) return "";
  const functions = [];
  for (const tool of tools) {
    if (get(tool, "type", null) !== "function") continue;
    const fn = get(tool, "function", {});
    if (J.t(fn)) functions.push(declaration(fn));
  }
  if (!functions.length) return "";
  return "# Tools\n\n## functions\nnamespace functions {\n" + functions.join("\n") + "\n}\n";
}

// What Kimi's `apply_chat_template` does before the template: the tools with their keys
// sorted, `thinking` and `add_generation_prompt` on unless the caller says otherwise, and
// `tools_ts_str` when the declarations could be built (if building them fails, the template
// gets none and prints the tools as JSON).
export function kimiInputs(given) {
  const tools = J.deep_sort(given.tools ?? null);
  const ctx = {...given, tools,
    thinking: "thinking" in given ? given.thinking : true,
    add_generation_prompt: "add_generation_prompt" in given ? given.add_generation_prompt : true};
  if (J.t(tools)) {
    try {
      ctx.tools_ts_str = encode(tools);
    } catch (e) {}
  }
  return ctx;
}
