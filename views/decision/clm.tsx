// Contrastive-CLM (CLM-v0.1-8B, a published System One model: frozen Qwen3-8B, a state head and an
// action head, scaled cosine). The state head reads the state text and the question; the action
// head reads each option's text on its own. So the state text is a <span label="question"> and
// each option an <aff option>, each in its own <box> (a separate encoder input, tokenized alone).
// <Decision readout="contrastive"> sends their spans; the host embeds each span's ids, caches the
// candidates' embeddings, and scores. Text follows CLM's schema.py (to_text, state_text).
"use strict_tokens";

// schema.py to_text: strings as they are; a map as `key: value` fields (top level separated by a
// blank line, nested ones indented by two); a list as one `- item` line per element.
export function clmText(x, indent = 0) {
  if (x === null || x === undefined) return "";
  if (typeof x === "string") return x;
  if (typeof x === "boolean") return x ? "true" : "false";
  if (typeof x === "number") return formatPythonValue(x);
  const pad = "                ".slice(0, indent);
  const nested = v => (isPlainObject(v) && Object.keys(v).length > 0) || (Array.isArray(v) && v.length > 0);
  if (isPlainObject(x)) {
    const parts = Object.entries(x).map(kv => nested(kv[1]) ? pad + kv[0] + ":\n" + clmText(kv[1], indent + 2) : pad + kv[0] + ": " + clmText(kv[1]));
    return parts.join(indent === 0 ? "\n\n" : "\n");
  }
  return x.map(v => nested(v) ? pad + "-\n" + clmText(v, indent + 2) : pad + "- " + clmText(v)).join("\n");
}

// options: [{name, desc}] (the action head reads desc, or the name when there is none).
export function ClmChoice({ state, instructions, options }) {
  const s = clmText(state).trim();
  const q = clmText(instructions).trim();
  return <>
    <box><span label="question">{s && q ? s + "\n\n" + q : (s || q)}</span></box>
    {options.map(o => <box><aff option={o.name}>{o.desc ? clmText(o.desc) : o.name}</aff></box>)}
  </>;
}
