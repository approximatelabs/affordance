// Tool-call format text from the NVIDIA Nemotron-3-Nano chat template, copied verbatim.
// Notice: ./NOTICE.md
"use strict_tokens";
const NEMOTRON_TOOL_FORMAT = p`\n\nIf you choose to call a function ONLY reply in the following format with NO suffix:\n\n${tok.tool_call}\n<function=example_function_name>\n<parameter=example_parameter_1>\nvalue_1\n</parameter>\n<parameter=example_parameter_2>\nThis is the value for the second parameter\nthat can span\nmultiple lines\n</parameter>\n</function>\n${tok.tool_call_end}\n\n<IMPORTANT>\nReminder:\n- Function calls MUST follow the specified format: an inner <function=...></function> block must be nested within ${tok.tool_call}${tok.tool_call_end} XML tags\n- Required parameters MUST be specified\n- You may provide optional reasoning for your function call in natural language BEFORE the function call, but NOT after\n- If there is no function call available, answer the question like normal with your current knowledge and do not tell the user about function calls\n</IMPORTANT>`;

export { NEMOTRON_TOOL_FORMAT };
