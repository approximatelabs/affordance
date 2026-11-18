// Converted from the chat template of meituan-longcat/LongCat-2.0 by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let tool_choice_0 = ctx["tool_choice"];
  let enable_thinking_0 = ctx["enable_thinking"];
  let save_history_reasoning_content_0 = ctx["save_history_reasoning_content"];
  let ns_0 = ctx["ns"];
  let messages_0 = ctx["messages"];
  let thinking_budget_0 = ctx["thinking_budget"];
  let role_0 = ctx["role"];
  let c_0 = ctx["c"];
  let parts_0 = ctx["parts"];
  let msg_0 = ctx["msg"];
  let tools_0 = ctx["tools"] ?? null;
  let reasoning_content_0 = ctx["reasoning_content"];
  let _args_0 = ctx["_args"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  tool_choice_0 = J.filter("default", tool_choice_0, "auto");
  enable_thinking_0 = J.filter("default", enable_thinking_0, true);
  save_history_reasoning_content_0 = J.filter("default", save_history_reasoning_content_0, true);
  ns_0 = J.call(J.namespace, J.kw({"final_messages": [], "tool_types": [], "last_query_index": J.neg(1)}));
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      let role_2 = role_0;
      let c_3 = c_0;
      let parts_4 = parts_0;
      role_2 = J.call(J.attr(message_1, "get"), "role");
      c_3 = J.call(J.attr(message_1, "get"), "content");
      if (J.t(J.test("string", c_3))) {
        J.setattr(ns_0, "final_messages", J.add(J.attr(ns_0, "final_messages"), [message_1]));
      } else if (J.t(((v) => J.t(v) ? !J.t(J.test("mapping", c_3)) : v)(J.test("iterable", c_3)))) {
        parts_4 = J.call(J.namespace, J.kw({"text": [], "thinking": [], "tool_calls": [], "tool_results": []}));
        {
          let _items2 = J.iter(c_3);
          const _loop2 = J.loop(_items2);
          for (let _i2 = 0; _i2 < _items2.length; _i2++) {
            _loop2.index0 = _i2; const _it2 = _items2[_i2]; let item_5 = _it2;
            if (J.t((J.eq(J.item(item_5, "type"), "text")))) {
              J.setattr(parts_4, "text", J.add(J.attr(parts_4, "text"), [J.item(item_5, "text")]));
            } else if (J.t((J.eq(J.item(item_5, "type"), "thinking")))) {
              if (J.t(J.item(item_5, "thinking"))) {
                J.setattr(parts_4, "thinking", J.add(J.attr(parts_4, "thinking"), [J.item(item_5, "thinking")]));
              }
            } else if (J.t((J.eq(J.item(item_5, "type"), "tool_use")))) {
              J.setattr(parts_4, "tool_calls", J.add(J.attr(parts_4, "tool_calls"), [({"id": J.item(item_5, "id"), "type": "function", "function": ({"name": J.item(item_5, "name"), "arguments": J.item(item_5, "input")})})]));
            } else if (J.t((J.eq(J.item(item_5, "type"), "tool_result")))) {
              J.setattr(parts_4, "tool_results", J.add(J.attr(parts_4, "tool_results"), [({"role": "tool", "name": J.call(J.attr(item_5, "get"), "name", ""), "tool_call_id": J.item(item_5, "tool_use_id"), "content": J.item(item_5, "content")})]));
            }
          }
        }
        J.setattr(ns_0, "final_messages", J.add(J.attr(ns_0, "final_messages"), J.attr(parts_4, "tool_results")));
        if (J.t(!J.t(((v) => J.t(v) ? (J.eq(role_2, "user")) : v)(((v) => J.t(v) ? (J.eq(J.filter("length", J.attr(parts_4, "text")), 0)) : v)((J.gt(J.filter("length", J.attr(parts_4, "tool_results")), 0))))))) {
          J.setattr(ns_0, "final_messages", J.add(J.attr(ns_0, "final_messages"), [({"role": role_2, "content": J.filter("join", J.attr(parts_4, "text"), ""), "reasoning_content": J.filter("join", J.attr(parts_4, "thinking"), ""), "tool_calls": J.attr(parts_4, "tool_calls")})]));
        }
      } else {
        J.setattr(ns_0, "final_messages", J.add(J.attr(ns_0, "final_messages"), [message_1]));
      }
    }
  }
  messages_0 = J.attr(ns_0, "final_messages");
  {
    let _items3 = J.iter(J.call(J.range, J.filter("length", messages_0)));
    const _loop3 = J.loop(_items3);
    for (let _i3 = 0; _i3 < _items3.length; _i3++) {
      _loop3.index0 = _i3; const _it3 = _items3[_i3]; let idx_6 = _it3;
      let msg_7 = msg_0;
      msg_7 = J.item(messages_0, idx_6);
      if (J.t((J.eq(J.attr(msg_7, "role"), "user")))) {
        J.setattr(ns_0, "last_query_index", idx_6);
      }
    }
  }
  if (J.t(((v) => J.t(v) ? (!J.eq(tool_choice_0, "none")) : v)(tools_0))) {
    out.push(J.str("<longcat_tool_declare>\n"), J.str("# Tools\n"), J.str("You have access to the following tools:\n\n"));
    {
      let _items4 = J.iter(tools_0);
      const _loop4 = J.loop(_items4);
      for (let _i4 = 0; _i4 < _items4.length; _i4++) {
        _loop4.index0 = _i4; const _it4 = _items4[_i4]; let tool_8 = _it4;
        if (J.t((!J.has(J.attr(tool_8, "type"), J.attr(ns_0, "tool_types"))))) {
          J.setattr(ns_0, "tool_types", J.add(J.attr(ns_0, "tool_types"), [J.attr(tool_8, "type")]));
          out.push(J.str(J.concat("## Tool namespace: ", J.attr(tool_8, "type"), "\n\n")));
        }
        if (J.t((J.eq(J.attr(tool_8, "type"), "code_interpreter")))) {
          tool_8 = ({"type": "code_interpreter", "function": ({"name": "code_interpreter_preview", "description": "The code will be executed in a stateful Jupyter notebook sandbox environment, only supports local computation, data processing, and file operations.\nCode sandbox environment (network isolated) Any external network requests or online API calls are prohibited.\nIf online functionality is needed, please use other permitted tools.\nCode will respond with the output of the execution or time out after 60.0 seconds. ", "parameters": ({"type": "object", "properties": ({"language": ({"type": "string", "description": "The programming language of the code to be executed. Available values: python (Default), java, go, js, ts, c, c++."}), "code": ({"type": "string", "description": "Python code to be executed must not include the following:\n- Importing network libraries such as requests, httplib, etc.\n- Any form of HTTP requests.\n- External API calls.\n- Network port operations. Example: ```python\nimport pandas as pd\npd.DataFrame({'A':[1,2]})\n```"}), "timeout": ({"type": "number", "description": "The maximum execution time of the code, in seconds. Default is 60.0."})})}), "required": ["code"]})});
        }
        out.push(J.str(J.add(J.add("### Tool name: ", J.attr(J.attr(tool_8, "function"), "name")), "\n")), J.str(J.add(J.add("Description: ", J.attr(J.attr(tool_8, "function"), "description")), "\n\n")), J.str(J.add(J.add("InputSchema: ", J.filter("tojson", J.attr(J.attr(tool_8, "function"), "parameters"), J.kw({"ensure_ascii": false}))), "\n\n")));
      }
    }
    out.push(J.str("**Note**: For each function call, output the function name and arguments within the following XML format:\n<longcat_tool_call>{function-name}\n<longcat_arg_key>{arg-key-1}</longcat_arg_key>\n<longcat_arg_value>{arg-value-1}</longcat_arg_value>\n<longcat_arg_key>{arg-key-2}</longcat_arg_key>\n<longcat_arg_value>{arg-value-2}</longcat_arg_value>\n...\n</longcat_tool_call>\n"), J.str("</longcat_tool_declare>"));
  }
  {
    let _items5 = J.iter(messages_0);
    const _loop5 = J.loop(_items5);
    for (let _i5 = 0; _i5 < _items5.length; _i5++) {
      _loop5.index0 = _i5; const _it5 = _items5[_i5]; let msg_9 = _it5;
      let thinking_budget_10 = thinking_budget_0;
      let reasoning_content_11 = reasoning_content_0;
      if (J.t((J.eq(J.attr(msg_9, "role"), "system")))) {
        out.push(J.str(J.add("<longcat_system>", J.attr(msg_9, "content"))));
      } else if (J.t((J.eq(J.attr(msg_9, "role"), "user")))) {
        out.push(J.str("<longcat_user>"));
        if (J.t(J.item(msg_9, "files"))) {
          out.push(J.str(J.concat("<longcat_files>\n", J.filter("tojson", J.attr(msg_9, "files"), J.kw({"indent": 2})), "\n</longcat_files>")));
        }
        out.push(J.str(J.attr(msg_9, "content")));
        if (J.t(((v) => J.t(v) ? !J.t((enable_thinking_0 === null)) : v)(save_history_reasoning_content_0))) {
          if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.add(J.attr(_loop5, "index0"), 1)), "role"), "assistant")))(J.attr(_loop5, "last")))) {
          } else {
            if (J.t((J.eq(enable_thinking_0, true)))) {
              out.push(J.str(" /think_on"));
              if (J.t(thinking_budget_10)) {
                if (J.t((J.lt(thinking_budget_10, 1024)))) {
                  thinking_budget_10 = 1024;
                }
                out.push(J.str(J.concat("\nthinking_budget: < ", thinking_budget_10, ".")));
              }
              out.push(J.str(" "));
            } else if (J.t((J.eq(enable_thinking_0, false)))) {
              out.push(J.str(" /think_off "));
            }
          }
        } else if (J.t(((v) => J.t(v) ? !J.t((enable_thinking_0 === null)) : v)((J.ge(J.attr(_loop5, "index0"), J.attr(ns_0, "last_query_index")))))) {
          if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.add(J.attr(_loop5, "index0"), 1)), "role"), "assistant")))(J.attr(_loop5, "last")))) {
          } else {
            if (J.t((J.eq(enable_thinking_0, true)))) {
              out.push(J.str(" /think_on"));
              if (J.t(thinking_budget_10)) {
                if (J.t((J.lt(thinking_budget_10, 1024)))) {
                  thinking_budget_10 = 1024;
                }
                out.push(J.str(J.concat("\nthinking_budget: < ", thinking_budget_10, ".")));
              }
              out.push(J.str(" "));
            } else if (J.t((J.eq(enable_thinking_0, false)))) {
              out.push(J.str(" /think_off "));
            }
          }
        }
      } else if (J.t((J.eq(J.attr(msg_9, "role"), "assistant")))) {
        out.push(J.str("<longcat_assistant>"));
        if (J.t(((v) => J.t(v) ? v : (J.gt(J.attr(_loop5, "index0"), J.attr(ns_0, "last_query_index"))))((J.eq(save_history_reasoning_content_0, true))))) {
          if (J.t((J.eq(enable_thinking_0, true)))) {
            if (J.t(J.attr(msg_9, "reasoning_content"))) {
              reasoning_content_11 = J.filter("trim", J.attr(msg_9, "reasoning_content"));
              out.push(J.str(J.concat("<longcat_think>\n", reasoning_content_11, "\n</longcat_think>\n")));
            } else {
              out.push(J.str("<longcat_think>\n\n</longcat_think>\n"));
            }
          } else if (J.t((J.eq(enable_thinking_0, false)))) {
            out.push(J.str("<longcat_think>\n\n</longcat_think>\n"));
          }
        }
        if (J.t(J.attr(msg_9, "content"))) {
          out.push(J.str(J.attr(msg_9, "content")));
        }
        if (J.t(J.attr(msg_9, "tool_calls"))) {
          {
            let _items6 = J.iter(J.attr(msg_9, "tool_calls"));
            const _loop6 = J.loop(_items6);
            for (let _i6 = 0; _i6 < _items6.length; _i6++) {
              _loop6.index0 = _i6; const _it6 = _items6[_i6]; let tool_call_12 = _it6;
              let _args_13 = _args_0;
              out.push(J.str(J.concat("<longcat_tool_call>", J.attr(J.attr(tool_call_12, "function"), "name"), "\n")));
              _args_13 = J.attr(J.attr(tool_call_12, "function"), "arguments");
              {
                let _items7 = J.iter(J.call(J.attr(_args_13, "items")));
                const _loop7 = J.loop(_items7);
                for (let _i7 = 0; _i7 < _items7.length; _i7++) {
                  _loop7.index0 = _i7; const _it7 = _items7[_i7]; let [k_14, v_15] = J.unpack(_it7, 2);
                  out.push(J.str(J.concat("<longcat_arg_key>", k_14, "</longcat_arg_key>\n")), J.str(J.concat("<longcat_arg_value>", (J.t(J.test("string", v_15)) ? v_15 : J.filter("tojson", v_15, J.kw({"ensure_ascii": false}))), "</longcat_arg_value>\n")));
                }
              }
              out.push(J.str("</longcat_tool_call>\n"));
            }
          }
        }
        out.push(J.str("</longcat_s>"));
      } else if (J.t((J.eq(J.attr(msg_9, "role"), "tool")))) {
        if (J.t((!J.eq(J.attr(J.item(messages_0, J.sub(J.attr(_loop5, "index0"), 1)), "role"), "tool")))) {
          out.push(J.str("<longcat_observation>"));
        }
        out.push(J.str(J.concat("<longcat_tool_response>", J.attr(msg_9, "name"), "\n", J.attr(msg_9, "content"), "</longcat_tool_response>")));
        if (J.t((J.eq(J.attr(_loop5, "index0"), J.sub(J.filter("length", messages_0), 1))))) {
        } else if (J.t(((v) => J.t(v) ? (!J.eq(J.attr(J.item(messages_0, J.add(J.attr(_loop5, "index0"), 1)), "role"), "tool")) : v)(((v) => J.t(v) ? v : (J.gt(J.attr(_loop5, "index0"), J.attr(ns_0, "last_query_index"))))(save_history_reasoning_content_0)))) {
          if (J.t((J.eq(enable_thinking_0, true)))) {
            out.push(J.str(" /think_on"));
            if (J.t(thinking_budget_10)) {
              if (J.t((J.lt(thinking_budget_10, 1024)))) {
                thinking_budget_10 = 1024;
              }
              out.push(J.str(J.concat("\nthinking_budget: < ", thinking_budget_10, ".")));
            }
            out.push(J.str(" "));
          } else if (J.t((J.eq(enable_thinking_0, false)))) {
            out.push(J.str(" /think_off "));
          }
        }
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
      out.push(J.str(" <longcat_assistant><longcat_think>"));
    } else if (J.t((J.eq(enable_thinking_0, false)))) {
      out.push(J.str(" /think_off <longcat_assistant><longcat_think>\n\n</longcat_think>\n"));
    } else {
      out.push(J.str(" /think_off <longcat_assistant><longcat_think>\n\n</longcat_think>\n"));
    }
  }
  return out.join("");
}
