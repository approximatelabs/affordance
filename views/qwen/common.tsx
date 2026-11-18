// Pieces shared by the Qwen (Hermes-style) chat formats.
"use strict_tokens";
import { QWEN_TOOLS_CALL, QWEN_TOOLS_INTRO, XML_TOOL_FORMAT } from "../third_party/qwen/qwen.tsx";

// The "# Tools" system section: function signatures as JSON lines in <tools>.
const ToolsSection = ({ tools }) => p`${QWEN_TOOLS_INTRO}${tools.map(t => "\n" + formatJSON(t))}
</tools>

${QWEN_TOOLS_CALL}${tok.tool_call}${tok.tool_call_end} XML tags:
${tok.tool_call}
{"name": <function-name>, "arguments": <args-json-object>}
${tok.tool_call_end}`;

// One <tool_call> block. strargs: string arguments are printed raw (Qwen3); else always JSON (Qwen2.5).
function ToolCall({ tc, strargs = true }) {
  const f = tc.function ? tc.function : tc;
  const args = strargs && typeof f.arguments === "string" ? f.arguments : formatJSON(f.arguments);
  return p`${tok.tool_call}\n{"name": "${formatPythonValue(f.name)}", "arguments": ${args}}\n${tok.tool_call_end}`;
}

// Consecutive tool messages are grouped into one user turn.
// Qwen3 has <tool_response> tokens; for Qwen2.5 the same text is plain text.
const ToolResponse = ({ messages, i, content }) => <>
  {(i === 0 || messages[i - 1].role !== "tool") && p`${tok.im_start}user`}
  {p`\n${textOrToken("<tool_response>")}\n${asContent(content)}\n${textOrToken("</tool_response>")}`}
  {(i === messages.length - 1 || messages[i + 1].role !== "tool") && p`${tok.im_end}\n`}
</>;

const isImage = item => isPlainObject(item) && ("image" in item || "image_url" in item || item.type === "image");
const isVideo = item => isPlainObject(item) && ("video" in item || item.type === "video");

// Images or videos in the content lists of `msgs`.
const countMedia = (msgs, kind) => msgs.map(m => Array.isArray(m.content)
  ? m.content.filter(it => kind === "image" ? isImage(it) : !isImage(it) && isVideo(it)).length : 0).reduce((a, b) => a + b, 0);

// Qwen3-VL style content to a string (images become vision placeholder tokens).
// img0/vid0: media seen before this message (for "Picture N:" labels).
function renderContent(content, isSystem, addVisionId, img0, vid0) {
  if (isFragment(content)) return content;
  if (typeof content === "string") return content;
  if (content === null || content === undefined) return "";
  if (!Array.isArray(content)) throw new Error("Unexpected content type.");
  let out = "", ni = img0, nv = vid0;
  for (const item of content) {
    if (isImage(item)) {
      if (isSystem) throw new Error("System message cannot contain images.");
      ni += 1;
      out += (addVisionId ? `Picture ${ni}: ` : "") + (tok.vision_start + tok.image_pad + tok.vision_end);
    } else if (isVideo(item)) {
      if (isSystem) throw new Error("System message cannot contain videos.");
      nv += 1;
      out += (addVisionId ? `Video ${nv}: ` : "") + (tok.vision_start + tok.video_pad + tok.vision_end);
    } else if (isPlainObject(item) && "text" in item) {
      out += item.text;
    } else {
      throw new Error("Unexpected item type in content.");
    }
  }
  return out;
}



// A model turn logged with exact ids (TurnStrategy `model` record with `ids`): the generation
// prompt the model saw, its output verbatim, and the turn end unless the output already has it.
function GeneratedTurn({ ids, prompt }) {
  return [prompt, tokens(ids), ids?.at(-1) === tok.im_end.id ? "\n" : p`${tok.im_end}\n`];
}

export { GeneratedTurn, ToolCall, ToolResponse, ToolsSection, XML_TOOL_FORMAT, countMedia, renderContent };
