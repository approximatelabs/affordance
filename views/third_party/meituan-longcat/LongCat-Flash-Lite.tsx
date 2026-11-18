// Converted from the chat template of meituan-longcat/LongCat-Flash-Lite by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let tool_choice_0 = ctx["tool_choice"];
  let ns_0 = ctx["ns"];
  let thinking_budget_0 = ctx["thinking_budget"];
  let tools_0 = ctx["tools"] ?? null;
  let messages_0 = ctx["messages"];
  let msg_0 = ctx["msg"];
  let enable_thinking_0 = ctx["enable_thinking"];
  let _args_0 = ctx["_args"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  tool_choice_0 = J.filter("default", tool_choice_0, "auto");
  ns_0 = J.call(J.namespace, J.kw({"tool_types": [], "last_query_index": J.neg(1)}));
  if (J.t(((v) => J.t(v) ? (!J.eq(tool_choice_0, "none")) : v)(tools_0))) {
    out.push(J.str("<longcat_tool_declare>\n"), J.str("# Tools\n"), J.str("You have access to the following tools:\n\n"));
    {
      let _items1 = J.iter(tools_0);
      const _loop1 = J.loop(_items1);
      for (let _i1 = 0; _i1 < _items1.length; _i1++) {
        _loop1.index0 = _i1; const _it1 = _items1[_i1]; let tool_1 = _it1;
        if (J.t((!J.has(J.attr(tool_1, "type"), J.attr(ns_0, "tool_types"))))) {
          J.setattr(ns_0, "tool_types", J.add(J.attr(ns_0, "tool_types"), [J.attr(tool_1, "type")]));
          out.push(J.str(J.concat("## Tool namespace: ", J.attr(tool_1, "type"), "\n\n")));
        }
        if (J.t((J.eq(J.attr(tool_1, "type"), "code_interpreter")))) {
          tool_1 = ({"type": "code_interpreter", "function": ({"name": "code_interpreter_preview", "description": "The code will be executed in a stateful Jupyter notebook sandbox environment, only supports local computation, data processing, and file operations.\nCode sandbox environment (network isolated) Any external network requests or online API calls are prohibited.\nIf online functionality is needed, please use other permitted tools.\nCode will respond with the output of the execution or time out after 60.0 seconds. ", "parameters": ({"type": "object", "properties": ({"language": ({"type": "string", "description": "The programming language of the code to be executed. Available values: python (Default), java, go, js, ts, c, c++."}), "code": ({"type": "string", "description": "Python code to be executed must not include the following:\n- Importing network libraries such as requests, httplib, etc.\n- Any form of HTTP requests.\n- External API calls.\n- Network port operations. Example: ```python\nimport pandas as pd\npd.DataFrame({'A':[1,2]})\n```"}), "timeout": ({"type": "number", "description": "The maximum execution time of the code, in seconds. Default is 60.0."})})}), "required": ["code"]})});
        }
        out.push(J.str(J.add(J.add("### Tool name: ", J.attr(J.attr(tool_1, "function"), "name")), "\n")), J.str(J.add(J.add("Description: ", J.attr(J.attr(tool_1, "function"), "description")), "\n\n")), J.str(J.add(J.add("InputSchema: ", J.filter("tojson", J.attr(J.attr(tool_1, "function"), "parameters"), J.kw({"ensure_ascii": false}))), "\n\n")));
      }
    }
    out.push(J.str("**Note**: For each function call, output the function name and arguments within the following XML format:\n<longcat_tool_call>{function-name}\n<longcat_arg_key>{arg-key-1}</longcat_arg_key>\n<longcat_arg_value>{arg-value-1}</longcat_arg_value>\n<longcat_arg_key>{arg-key-2}</longcat_arg_key>\n<longcat_arg_value>{arg-value-2}</longcat_arg_value>\n...\n</longcat_tool_call>\n"), J.str("</longcat_tool_declare>"));
    {
      let _items2 = J.iter(J.call(J.range, J.sub(J.filter("length", messages_0), 1)));
      const _loop2 = J.loop(_items2);
      for (let _i2 = 0; _i2 < _items2.length; _i2++) {
        _loop2.index0 = _i2; const _it2 = _items2[_i2]; let idx_2 = _it2;
        let msg_3 = msg_0;
        msg_3 = J.item(messages_0, idx_2);
        if (J.t(((v) => J.t(v) ? !J.t(J.attr(msg_3, "tool_calls")) : v)((J.eq(J.attr(msg_3, "role"), "assistant"))))) {
          J.setattr(ns_0, "last_query_index", idx_2);
        }
      }
    }
  }
  {
    let _items3 = J.iter(messages_0);
    const _loop3 = J.loop(_items3);
    for (let _i3 = 0; _i3 < _items3.length; _i3++) {
      _loop3.index0 = _i3; const _it3 = _items3[_i3]; let msg_4 = _it3;
      if (J.t((J.eq(J.attr(msg_4, "role"), "system")))) {
        out.push(J.str(J.add("<longcat_system>", J.attr(msg_4, "content"))));
      } else if (J.t((J.eq(J.attr(msg_4, "role"), "user")))) {
        out.push(J.str("<longcat_user>"));
        if (J.t(J.item(msg_4, "files"))) {
          out.push(J.str(J.concat("<longcat_files>\n", J.filter("tojson", J.attr(msg_4, "files"), J.kw({"indent": 2})), "\n</longcat_files>")));
        }
        out.push(J.str(J.attr(msg_4, "content")));
      } else if (J.t((J.eq(J.attr(msg_4, "role"), "assistant")))) {
        out.push(J.str("<longcat_assistant>"));
        if (J.t(((v) => J.t(v) ? (J.gt(J.attr(_loop3, "index0"), J.attr(ns_0, "last_query_index"))) : v)(((v) => J.t(v) ? (!J.eq(J.attr(ns_0, "tool_types"), [])) : v)(((v) => J.t(v) ? J.attr(msg_4, "reasoning_content") : v)((J.eq(enable_thinking_0, true))))))) {
          out.push(J.str(J.concat("\n<longcat_think>\n", J.attr(msg_4, "reasoning_content"), "\n</longcat_think>\n")));
        }
        if (J.t(J.attr(msg_4, "content"))) {
          out.push(J.str(J.attr(msg_4, "content")));
        }
        if (J.t(J.attr(msg_4, "tool_calls"))) {
          {
            let _items4 = J.iter(J.attr(msg_4, "tool_calls"));
            const _loop4 = J.loop(_items4);
            for (let _i4 = 0; _i4 < _items4.length; _i4++) {
              _loop4.index0 = _i4; const _it4 = _items4[_i4]; let tool_call_5 = _it4;
              let _args_6 = _args_0;
              out.push(J.str(J.concat("<longcat_tool_call>", J.attr(J.attr(tool_call_5, "function"), "name"), "\n")));
              _args_6 = J.attr(J.attr(tool_call_5, "function"), "arguments");
              {
                let _items5 = J.iter(J.call(J.attr(_args_6, "items")));
                const _loop5 = J.loop(_items5);
                for (let _i5 = 0; _i5 < _items5.length; _i5++) {
                  _loop5.index0 = _i5; const _it5 = _items5[_i5]; let [k_7, v_8] = J.unpack(_it5, 2);
                  out.push(J.str(J.concat("<longcat_arg_key>", k_7, "</longcat_arg_key>\n")), J.str(J.concat("<longcat_arg_value>", (J.t(J.test("string", v_8)) ? v_8 : J.filter("tojson", v_8, J.kw({"ensure_ascii": false}))), "</longcat_arg_value>\n")));
                }
              }
              out.push(J.str("</longcat_tool_call>\n"));
            }
          }
        }
        out.push(J.str("</longcat_s>"));
      } else if (J.t((J.eq(J.attr(msg_4, "role"), "tool")))) {
        if (J.t((!J.eq(J.attr(J.item(messages_0, J.sub(J.attr(_loop3, "index0"), 1)), "role"), "tool")))) {
          out.push(J.str("<longcat_user>"));
        }
        out.push(J.str(J.concat("<longcat_tool_response>", J.attr(msg_4, "content"), "</longcat_tool_response>")));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    if (J.t((J.eq(enable_thinking_0, true)))) {
      out.push(J.str(" /think_on"));
      if (J.t(thinking_budget_0)) {
        if (J.t((J.lt(thinking_budget_0, 1024)))) {
          thinking_budget_0 = 1024;
        }
        out.push(J.str(J.concat("\nthinking_budget: < ", thinking_budget_0, ".")));
      }
      out.push(J.str(" <longcat_assistant><longcat_think>\n"));
    } else if (J.t((J.eq(enable_thinking_0, false)))) {
      out.push(J.str(" /think_off <longcat_assistant><longcat_think>\n\n</longcat_think>\n"));
    } else {
      out.push(J.str("<longcat_assistant>"));
    }
  }
  return out.join("");
}
