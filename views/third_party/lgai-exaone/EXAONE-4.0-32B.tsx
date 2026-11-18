// Converted from the chat template of LGAI-EXAONE/EXAONE-4.0-32B by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let skip_think_0 = ctx["skip_think"];
  let role_indicators_0 = ctx["role_indicators"];
  let end_of_turn_0 = ctx["end_of_turn"];
  let available_tools_0 = ctx["available_tools"];
  let ns_0 = ctx["ns"];
  let messages_0 = ctx["messages"];
  let msg_0 = ctx["msg"];
  let role_0 = ctx["role"];
  let content_0 = ctx["content"];
  let reasoning_content_0 = ctx["reasoning_content"];
  let tools_0 = ctx["tools"] ?? null;
  let arguments_0 = ctx["arguments"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let enable_thinking_0 = ctx["enable_thinking"];
  if (J.t(!J.t((skip_think_0 !== undefined)))) {
    skip_think_0 = true;
  }
  role_indicators_0 = ({"user": "[|user|]\n", "assistant": "[|assistant|]\n", "system": "[|system|]\n", "tool": "[|tool|]\n"});
  end_of_turn_0 = "[|endofturn|]\n";
  available_tools_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_2 = k.caller;
    let tools_1 = p.length > 0 ? p[0] : "tools" in k ? k["tools"] : undefined;
    out.push(J.str("# Available Tools"), J.str("\nYou can use none, one, or multiple of the following tools by calling them as functions to help with the user\u2019s query."), J.str("\nHere are the tools available to you in JSON format within <tool> and </tool> tags:\n"));
    {
      let _items1 = J.iter(tools_1);
      const _loop1 = J.loop(_items1);
      for (let _i1 = 0; _i1 < _items1.length; _i1++) {
        _loop1.index0 = _i1; const _it1 = _items1[_i1]; let tool_3 = _it1;
        out.push(J.str("<tool>"), J.str(J.filter("safe", J.filter("tojson", tool_3, J.kw({"ensure_ascii": false})))), J.str("</tool>\n"));
      }
    }
    out.push(J.str("\nFor each function call you want to make, return a JSON object with function name and arguments within <tool_call> and </tool_call> tags, like:"), J.str("\n<tool_call>{\"name\": function_1_name, \"arguments\": {argument_1_name: argument_1_value, argument_2_name: argument_2_value}}</tool_call>"), J.str("\n<tool_call>{\"name\": function_2_name, \"arguments\": {...}}</tool_call>\n..."), J.str("\nNote that if no argument name is specified for a tool, you can just print the argument value directly, without the argument name or JSON formatting."));
    return out.join("");
  };
  ns_0 = J.call(J.namespace, J.kw({"last_query_index": J.sub(J.filter("length", messages_0), 1)}));
  {
    let _items2 = J.iter(messages_0);
    const _loop2 = J.loop(_items2);
    for (let _i2 = 0; _i2 < _items2.length; _i2++) {
      _loop2.index0 = _i2; const _it2 = _items2[_i2]; let message_4 = _it2;
      if (J.t(((v) => J.t(v) ? J.test("string", J.attr(message_4, "content")) : v)((J.eq(J.attr(message_4, "role"), "user"))))) {
        J.setattr(ns_0, "last_query_index", J.attr(_loop2, "index0"));
      }
    }
  }
  {
    let _items3 = J.iter(J.call(J.range, J.filter("length", messages_0)));
    const _loop3 = J.loop(_items3);
    for (let _i3 = 0; _i3 < _items3.length; _i3++) {
      _loop3.index0 = _i3; const _it3 = _items3[_i3]; let i_5 = _it3;
      let msg_6 = msg_0;
      let role_7 = role_0;
      let content_8 = content_0;
      let reasoning_content_9 = reasoning_content_0;
      msg_6 = J.item(messages_0, i_5);
      role_7 = J.attr(msg_6, "role");
      if (J.t((!J.has(role_7, role_indicators_0)))) {
        out.push(J.str(J.call(J.raise_exception, J.concat("Unknown role: ", role_7))));
      }
      if (J.t((J.eq(i_5, 0)))) {
        if (J.t((J.eq(role_7, "system")))) {
          out.push(J.str(J.item(role_indicators_0, "system")), J.str(J.attr(msg_6, "content")));
          if (J.t(((v) => J.t(v) ? tools_0 : v)((tools_0 !== undefined)))) {
            out.push(J.str("\n\n"), J.str(J.call(available_tools_0, tools_0)));
          }
          out.push(J.str(end_of_turn_0));
          continue;
        } else if (J.t(((v) => J.t(v) ? tools_0 : v)((tools_0 !== undefined)))) {
          out.push(J.str(J.item(role_indicators_0, "system")), J.str(J.call(available_tools_0, tools_0)), J.str(end_of_turn_0));
        }
      }
      if (J.t((J.eq(role_7, "assistant")))) {
        out.push(J.str(J.item(role_indicators_0, "assistant")));
        if (J.t(J.attr(msg_6, "content"))) {
          if (J.t((J.has("</think>", J.attr(msg_6, "content"))))) {
            content_8 = J.call(J.attr(J.item(J.call(J.attr(J.attr(msg_6, "content"), "split"), "</think>"), J.neg(1)), "strip"));
            reasoning_content_9 = J.call(J.attr(J.item(J.call(J.attr(J.attr(msg_6, "content"), "split"), "</think>"), 0), "strip"));
            if (J.t(J.call(J.attr(reasoning_content_9, "startswith"), "<think>"))) {
              reasoning_content_9 = J.call(J.attr(J.slice(reasoning_content_9, 9, undefined, undefined), "strip"));
            }
          } else {
            content_8 = J.attr(msg_6, "content");
          }
          if (J.t(J.attr(msg_6, "reasoning_content"))) {
            reasoning_content_9 = J.attr(msg_6, "reasoning_content");
          }
          if (J.t(((v) => J.t(v) ? (reasoning_content_9 !== undefined) : v)(((v) => J.t(v) ? J.attr(_loop3, "last") : v)(!J.t(skip_think_0))))) {
            out.push(J.str("<think>\n"), J.str(reasoning_content_9), J.str("\n</think>\n\n"));
          } else {
            out.push(J.str("<think>\n\n</think>\n\n"));
          }
          out.push(J.str(content_8));
        }
        if (J.t(J.attr(msg_6, "tool_calls"))) {
          if (J.t(J.attr(msg_6, "content"))) {
            out.push(J.str("\n"));
          } else {
            out.push(J.str("<think>\n\n</think>\n\n"));
          }
          {
            let _items4 = J.iter(J.attr(msg_6, "tool_calls"));
            const _loop4 = J.loop(_items4);
            for (let _i4 = 0; _i4 < _items4.length; _i4++) {
              _loop4.index0 = _i4; const _it4 = _items4[_i4]; let tool_call_10 = _it4;
              let arguments_11 = arguments_0;
              if (J.t((J.attr(tool_call_10, "function") !== undefined))) {
                tool_call_10 = J.attr(tool_call_10, "function");
              }
              if (J.t((J.attr(tool_call_10, "arguments") !== undefined))) {
                arguments_11 = J.attr(tool_call_10, "arguments");
              } else if (J.t((J.attr(tool_call_10, "parameters") !== undefined))) {
                arguments_11 = J.attr(tool_call_10, "parameters");
              } else {
                out.push(J.str(J.call(J.raise_exception, J.concat("arguments or parameters are mandatory: ", tool_call_10))));
              }
              out.push(J.str("<tool_call>"), "{\"name\": \"", J.str(J.attr(tool_call_10, "name")), "\", \"arguments\": ", J.str(J.filter("safe", J.filter("tojson", arguments_11, J.kw({"ensure_ascii": false})))), "}", J.str("</tool_call>"));
              if (J.t(!J.t(J.attr(_loop4, "last")))) {
                out.push(J.str("\n"));
              }
            }
          }
        }
        out.push(J.str(end_of_turn_0));
      } else if (J.t((J.eq(role_7, "tool")))) {
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.sub(i_5, 1)), "role"), "tool")))((J.eq(i_5, 0))))) {
          out.push(J.str(J.item(role_indicators_0, "tool")));
        }
        if (J.t((J.attr(msg_6, "content") !== undefined))) {
          out.push(J.str("<tool_result>"), "{\"result\": ", J.str(J.filter("safe", J.filter("tojson", J.attr(msg_6, "content"), J.kw({"ensure_ascii": false})))), "}", J.str("</tool_result>"));
        }
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.add(i_5, 1)), "role"), "tool")))(J.attr(_loop3, "last")))) {
          out.push(J.str(end_of_turn_0));
        } else {
          out.push(J.str("\n"));
        }
      } else {
        out.push(J.str(J.item(role_indicators_0, role_7)), J.str(J.attr(msg_6, "content")), J.str(end_of_turn_0));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str(J.item(role_indicators_0, "assistant")));
    if (J.t(((v) => J.t(v) ? J.test("true", enable_thinking_0) : v)((enable_thinking_0 !== undefined)))) {
      out.push(J.str("<think>\n"));
    } else {
      out.push(J.str("<think>\n\n</think>\n\n"));
    }
  }
  return out.join("");
}
