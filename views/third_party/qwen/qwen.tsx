// Prompt text from the Qwen chat templates (Qwen2.5, Qwen3, Qwen3-Coder, Qwen3.5), copied verbatim.
// Notice: ./NOTICE.md
"use strict_tokens";
// Hermes-style tools: the section's opening, and the call instruction (followed by the call tokens).
const QWEN_TOOLS_INTRO = `# Tools\n\nYou may call one or more functions to assist with the user query.\n\nYou are provided with function signatures within <tools></tools> XML tags:\n<tools>`;
const QWEN_TOOLS_CALL = "For each function call, return a json object with function name and arguments within ";
// XML-style tools (Qwen3-Coder, Qwen3.5): the call format and its reminder.
const XML_TOOL_FORMAT = ("\n\nIf you choose to call a function ONLY reply in the following format with NO suffix:\n\n" + tok.tool_call + "\n<function=example_function_name>\n<parameter=example_parameter_1>\nvalue_1\n</parameter>\n<parameter=example_parameter_2>\nThis is the value for the second parameter\nthat can span\nmultiple lines\n</parameter>\n</function>\n" + tok.tool_call_end + "\n\n<IMPORTANT>\nReminder:\n- Function calls MUST follow the specified format: an inner <function=...></function> block must be nested within " + tok.tool_call + tok.tool_call_end + " XML tags\n- Required parameters MUST be specified\n- You may provide optional reasoning for your function call in natural language BEFORE the function call, but NOT after\n- If there is no function call available, answer the question like normal with your current knowledge and do not tell the user about function calls\n</IMPORTANT>");
// Qwen3-Coder: the default system message when tools are given.
const QWEN3_CODER_SYSTEM = "You are Qwen, a helpful AI assistant that can interact with a computer to solve tasks.";
// Qwen3.5 reasoning effort.
const QWEN35_EFFORT_XHIGH = "Reasoning effort is set to xhigh. Please think carefully through the task, validate key assumptions, consider plausible alternatives, and prioritize correctness, consistency, and clarity in the final answer.";
const QWEN35_EFFORT_LOW = "Reasoning effort is set to low. Keep your thinking brief and focused, moving directly to the conclusion without unnecessary elaboration.";

export { QWEN35_EFFORT_LOW, QWEN3_CODER_SYSTEM, QWEN35_EFFORT_XHIGH, QWEN_TOOLS_CALL, QWEN_TOOLS_INTRO, XML_TOOL_FORMAT };
