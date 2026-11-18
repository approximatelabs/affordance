// Prompt text from the GLM-Z1-0414 and GLM-4.5 to GLM-5.3 chat templates, copied verbatim. The
// tools introduction is the same text as Qwen's (see ../qwen/NOTICE.md).
// Notice: ./NOTICE.md
"use strict_tokens";
const GLM_TOOLS_INTRO = (tok.system + "\n# Tools\n\nYou may call one or more functions to assist with the user query.\n\nYou are provided with function signatures within <tools></tools> XML tags:\n<tools>\n");
// GLM-Z1-0414: the system message before the tools.
const GLM_Z1_SYS = "你是一个名为 ChatGLM 的人工智能助手。你是基于智谱 AI 公司训练的语言模型 GLM-4 模型开发的，你的任务是针对用户的问题和要求提供适当的答复和支持。\n\n# 可用工具";
// Functions, not constants: GLM-4 has no <tool_call> token, so tok.tool_call is only read for GLM-4.5 and later.
const glmToolsFormat45 = () => ("</tools>\n\nFor each function call, output the function name and arguments within the following XML format:\n" + tok.tool_call + "{function-name}\n" + tok.arg_key + "{arg-key-1}" + tok.arg_key_end + "\n" + tok.arg_value + "{arg-value-1}" + tok.arg_value_end + "\n" + tok.arg_key + "{arg-key-2}" + tok.arg_key_end + "\n" + tok.arg_value + "{arg-value-2}" + tok.arg_value_end + "\n...\n" + tok.tool_call_end);
const glmToolsFormat47 = () => ("</tools>\n\nFor each function call, output the function name and arguments within the following XML format:\n" + tok.tool_call + "{function-name}" + tok.arg_key + "{arg-key-1}" + tok.arg_key_end + tok.arg_value + "{arg-value-1}" + tok.arg_value_end + tok.arg_key + "{arg-key-2}" + tok.arg_key_end + tok.arg_value + "{arg-value-2}" + tok.arg_value_end + "..." + tok.tool_call_end);

export { GLM_TOOLS_INTRO, GLM_Z1_SYS, glmToolsFormat45, glmToolsFormat47 };
