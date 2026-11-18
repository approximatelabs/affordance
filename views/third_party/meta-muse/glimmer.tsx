// Tool-protocol text from the Meta Muse Glimmer chat template, copied verbatim.
// Notice: ./NOTICE.md
"use strict_tokens";
// Before the tool metadata and schemas.
const MUSE_TOOLS_INTRO = "In this environment you have access to a set of tools you can use to answer the user's question.\n\n"
  + "You can invoke a function by writing a \"<atem:function_calls>\" block like the following:\n"
  + "<atem:function_calls>\n<atem:invoke name=\"$FUNCTION_NAME\">\n<atem:parameter name=\"$PARAMETER_NAME\">$PARAMETER_VALUE</atem:parameter>\n...\n</atem:invoke>\n</atem:function_calls>\n\n"
  + "String and scalar parameters should be specified as is, while lists and objects should use JSON format. Note that spaces for string values are not stripped. The output is not expected to be valid XML and is parsed with regular expressions.\n"
  + "Here are the functions available in JSONSchema format:\n"
  + "// Tool metadata\n";
// After the function schemas.
const MUSE_TOOLS_EXAMPLE = "\n\nHere's an example of how to call a function in the tool set:\n"
  + "(If the tool namespace is not specified, invoke the function directly as `example_function_name` rather than `example_tool_name.example_function_name`)\n\n"
  + "to=example_tool_name.example_function_name\n\n"
  + "<atem:function_calls>\n<atem:invoke name=\"example_tool_name.example_function_name\">\n"
  + "<atem:parameter name=\"example_parameter_1\">value_1</atem:parameter>\n"
  + "<atem:parameter name=\"example_parameter_2\">This is the value for the second parameter\nthat can span\n\"multiple\" lines\n</atem:parameter>\n"
  + "</atem:invoke>\n</atem:function_calls>";

export { MUSE_TOOLS_EXAMPLE, MUSE_TOOLS_INTRO };
