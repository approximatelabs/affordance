// Converted from the chat template of ibm-granite/granite-4.2-8b by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let tool_to_json_0 = ctx["tool_to_json"];
  let enable_thinking_0 = ctx["enable_thinking"];
  let low_effort_0 = ctx["low_effort"];
  let truncate_history_thinking_0 = ctx["truncate_history_thinking"];
  let ns_0 = ctx["ns"];
  let loop_messages_0 = ctx["loop_messages"];
  let system_message_0 = ctx["system_message"];
  let tools_0 = ctx["tools"] ?? null;
  let ns_tool_0 = ctx["ns_tool"];
  let reasoning_effort_0 = ctx["reasoning_effort"];
  let messages_0 = ctx["messages"];
  let content_0 = ctx["content"];
  let include_content_0 = ctx["include_content"];
  let c_0 = ctx["c"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  tool_to_json_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_2 = k.caller;
    let tool_1 = p.length > 0 ? p[0] : "tool" in k ? k["tool"] : undefined;
    let ns_tool_3 = ns_tool_0;
    ns_tool_3 = J.call(J.namespace, J.kw({"first": true}));
    out.push(J.str("{"));
    {
      let _items1 = J.iter(J.call(J.attr(tool_1, "items")));
      const _loop1 = J.loop(_items1);
      for (let _i1 = 0; _i1 < _items1.length; _i1++) {
        _loop1.index0 = _i1; const _it1 = _items1[_i1]; let [k_4, v_5] = J.unpack(_it1, 2);
        if (J.t(((v) => J.t(v) ? (!J.eq(k_4, "strict")) : v)((!J.eq(k_4, "defer_loading"))))) {
          if (J.t(!J.t(J.attr(ns_tool_3, "first")))) {
            out.push(J.str(", "));
          }
          J.setattr(ns_tool_3, "first", false);
          out.push(J.str(J.concat("\"", k_4, "\": ", J.filter("tojson", v_5, J.kw({"ensure_ascii": false})))));
        }
      }
    }
    out.push(J.str("}"));
    return out.join("");
  };
  enable_thinking_0 = (J.t((enable_thinking_0 !== undefined)) ? enable_thinking_0 : true);
  low_effort_0 = (J.t((low_effort_0 !== undefined)) ? low_effort_0 : false);
  if (J.t(((v) => J.t(v) ? !J.t((reasoning_effort_0 === null)) : v)((reasoning_effort_0 !== undefined)))) {
    low_effort_0 = (J.eq(reasoning_effort_0, "low"));
  }
  truncate_history_thinking_0 = (J.t((truncate_history_thinking_0 !== undefined)) ? truncate_history_thinking_0 : true);
  ns_0 = J.call(J.namespace, J.kw({"last_user_idx": J.neg(1)}));
  loop_messages_0 = messages_0;
  {
    let _items2 = J.iter(loop_messages_0);
    const _loop2 = J.loop(_items2);
    for (let _i2 = 0; _i2 < _items2.length; _i2++) {
      _loop2.index0 = _i2; const _it2 = _items2[_i2]; let m_6 = _it2;
      if (J.t((J.eq(J.item(m_6, "role"), "user")))) {
        J.setattr(ns_0, "last_user_idx", J.attr(_loop2, "index0"));
      }
    }
  }
  if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
    system_message_0 = J.item(J.item(messages_0, 0), "content");
    loop_messages_0 = J.slice(messages_0, 1, undefined, undefined);
  } else {
    system_message_0 = "";
    loop_messages_0 = messages_0;
  }
  if (J.t(!J.t((tools_0 !== undefined)))) {
    tools_0 = [];
  }
  ns_0 = J.call(J.namespace, J.kw({"last_user_idx": J.neg(1)}));
  {
    let _items3 = J.iter(loop_messages_0);
    const _loop3 = J.loop(_items3);
    for (let _i3 = 0; _i3 < _items3.length; _i3++) {
      _loop3.index0 = _i3; const _it3 = _items3[_i3]; let m_7 = _it3;
      if (J.t((J.eq(J.item(m_7, "role"), "user")))) {
        J.setattr(ns_0, "last_user_idx", J.attr(_loop3, "index0"));
      }
    }
  }
  if (J.t((system_message_0 !== undefined))) {
    out.push(J.str(J.add("<|im_start|>system\n", system_message_0)));
  } else {
    if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", tools_0), 0)) : v)(J.test("iterable", tools_0)))) {
      out.push(J.str("<|im_start|>system\n"));
    }
  }
  if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", tools_0), 0)) : v)(J.test("iterable", tools_0)))) {
    if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", system_message_0), 0)) : v)((system_message_0 !== undefined)))) {
      out.push(J.str("\n\n"));
    }
    out.push(J.str("# Tools\n\nYou have access to the following functions:\n\n"), J.str("<tools>"));
    {
      let _items4 = J.iter(tools_0);
      const _loop4 = J.loop(_items4);
      for (let _i4 = 0; _i4 < _items4.length; _i4++) {
        _loop4.index0 = _i4; const _it4 = _items4[_i4]; let tool_8 = _it4;
        if (J.t((J.attr(tool_8, "function") !== undefined))) {
          tool_8 = J.attr(tool_8, "function");
        }
        if (J.t(((v) => J.t(v) ? v : !J.t(J.attr(tool_8, "defer_loading")))(!J.t((J.attr(tool_8, "defer_loading") !== undefined))))) {
          out.push(J.str(J.concat("\n", J.call(tool_to_json_0, tool_8))));
        }
      }
    }
    out.push(J.str("\n</tools>"), J.str("\n\nIf you choose to call a function ONLY reply in the following format with NO suffix:\n\n<tool_call>\n<function=example_function_name>\n<parameter=example_parameter_1>\nvalue_1\n</parameter>\n<parameter=example_parameter_2>\nThis is the value for the second parameter\nthat can span\nmultiple lines\n</parameter>\n</function>\n</tool_call>\n\n<IMPORTANT>\nReminder:\n- Function calls MUST follow the specified format: an inner <function=...></function> block must be nested within <tool_call></tool_call> XML tags\n- Required parameters MUST be specified\n- You may provide optional reasoning for your function call in natural language BEFORE the function call, but NOT after\n- If there is no function call available, answer the question like normal with your current knowledge and do not tell the user about function calls\n</IMPORTANT>"));
  }
  if (J.t((system_message_0 !== undefined))) {
    out.push(J.str("<|im_end|>\n"));
  } else {
    if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", tools_0), 0)) : v)(J.test("iterable", tools_0)))) {
      out.push(J.str("<|im_end|>\n"));
    }
  }
  {
    let _items5 = J.iter(loop_messages_0);
    const _loop5 = J.loop(_items5);
    for (let _i5 = 0; _i5 < _items5.length; _i5++) {
      _loop5.index0 = _i5; const _it5 = _items5[_i5]; let message_9 = _it5;
      let content_10 = content_0;
      let include_content_11 = include_content_0;
      let c_12 = c_0;
      if (J.t((J.eq(J.attr(message_9, "role"), "assistant")))) {
        if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", J.filter("trim", J.attr(message_9, "reasoning_content"))), 0)) : v)(((v) => J.t(v) ? J.test("string", J.attr(message_9, "reasoning_content")) : v)((J.attr(message_9, "reasoning_content") !== undefined))))) {
          content_10 = J.concat("<think>\n", J.attr(message_9, "reasoning_content"), "\n</think>\n", J.filter("default", J.attr(message_9, "content"), "", true));
        } else {
          content_10 = J.filter("default", J.attr(message_9, "content"), "", true);
          if (J.t(J.test("string", content_10))) {
            if (J.t(((v) => J.t(v) ? (!J.has("</think>", content_10)) : v)((!J.has("<think>", content_10))))) {
              content_10 = J.concat("<think></think>", content_10);
            }
          } else {
            content_10 = content_10;
          }
        }
        if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", J.attr(message_9, "tool_calls")), 0)) : v)(((v) => J.t(v) ? J.test("iterable", J.attr(message_9, "tool_calls")) : v)((J.attr(message_9, "tool_calls") !== undefined))))) {
          out.push(J.str("<|im_start|>assistant\n"));
          include_content_11 = !J.t(((v) => J.t(v) ? (J.lt(J.attr(_loop5, "index0"), J.attr(ns_0, "last_user_idx"))) : v)(truncate_history_thinking_0));
          if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", J.filter("trim", content_10)), 0)) : v)(J.test("string", content_10)))) {
            if (J.t(include_content_11)) {
              out.push(J.str(J.concat(J.filter("trim", content_10), "\n")));
            } else {
              c_12 = J.filter("string", content_10);
              if (J.t((J.has("</think>", c_12)))) {
                c_12 = J.item(J.call(J.attr(c_12, "split"), "</think>"), J.neg(1));
              } else if (J.t((J.has("<think>", c_12)))) {
                c_12 = J.item(J.call(J.attr(c_12, "split"), "<think>"), 0);
              }
              c_12 = J.concat("<think></think>", J.filter("trim", c_12));
              if (J.t((J.gt(J.filter("length", c_12), 0)))) {
                out.push(J.str(J.concat(c_12, "\n")));
              }
            }
          } else {
            out.push(J.str("<think></think>"));
          }
          {
            let _items6 = J.iter(J.attr(message_9, "tool_calls"));
            const _loop6 = J.loop(_items6);
            for (let _i6 = 0; _i6 < _items6.length; _i6++) {
              _loop6.index0 = _i6; const _it6 = _items6[_i6]; let tool_call_13 = _it6;
              if (J.t((J.attr(tool_call_13, "function") !== undefined))) {
                tool_call_13 = J.attr(tool_call_13, "function");
              }
              out.push(J.str(J.concat("<tool_call>\n<function=", J.attr(tool_call_13, "name"), ">\n")));
              if (J.t((J.attr(tool_call_13, "arguments") !== undefined))) {
                {
                  let _items7 = J.iter(J.filter("items", J.attr(tool_call_13, "arguments")));
                  const _loop7 = J.loop(_items7);
                  for (let _i7 = 0; _i7 < _items7.length; _i7++) {
                    _loop7.index0 = _i7; const _it7 = _items7[_i7]; let [args_name_14, args_value_15] = J.unpack(_it7, 2);
                    out.push(J.str(J.concat("<parameter=", args_name_14, ">\n")));
                    args_value_15 = (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? !J.t(J.test("string", args_value_15)) : v)(J.test("sequence", args_value_15)))(J.test("mapping", args_value_15))) ? J.filter("safe", J.filter("tojson", args_value_15)) : J.filter("string", args_value_15));
                    out.push(J.str(J.concat(args_value_15, "\n</parameter>\n")));
                  }
                }
              }
              out.push(J.str("</function>\n</tool_call>\n"));
            }
          }
          out.push(J.str("<|im_end|>\n"));
        } else {
          if (J.t(!J.t(((v) => J.t(v) ? (J.lt(J.attr(_loop5, "index0"), J.attr(ns_0, "last_user_idx"))) : v)(truncate_history_thinking_0)))) {
            out.push(J.str(J.concat("<|im_start|>assistant\n", J.filter("trim", J.filter("string", J.filter("default", content_10, "", true))), "<|im_end|>\n")));
          } else {
            c_12 = J.filter("string", J.filter("default", content_10, "", true));
            if (J.t(((v) => J.t(v) ? (J.has("</think>", c_12)) : v)((J.has("<think>", c_12))))) {
              c_12 = J.concat("<think></think>", J.item(J.call(J.attr(c_12, "split"), "</think>"), J.neg(1)));
            }
            c_12 = J.filter("trim", c_12);
            if (J.t((J.gt(J.filter("length", c_12), 0)))) {
              out.push(J.str(J.concat("<|im_start|>assistant\n", c_12, "<|im_end|>\n")));
            } else {
              out.push(J.str("<|im_start|>assistant\n<|im_end|>\n"));
            }
          }
        }
      } else if (J.t(((v) => J.t(v) ? v : (J.eq(J.attr(message_9, "role"), "system")))((J.eq(J.attr(message_9, "role"), "user"))))) {
        out.push(J.str(J.add(J.add("<|im_start|>", J.attr(message_9, "role")), "\n")));
        content_10 = J.filter("string", J.attr(message_9, "content"));
        if (J.t(((v) => J.t(v) ? low_effort_0 : v)(((v) => J.t(v) ? (J.eq(J.attr(_loop5, "index0"), J.attr(ns_0, "last_user_idx"))) : v)((J.eq(J.attr(message_9, "role"), "user")))))) {
          out.push(J.str(J.add(content_10, "\n\n{reasoning effort: low}")));
        } else {
          out.push(J.str(content_10));
        }
        out.push(J.str("<|im_end|>\n"));
      } else if (J.t((J.eq(J.attr(message_9, "role"), "tool")))) {
        if (J.t(((v) => J.t(v) ? (!J.eq(J.attr(J.attr(_loop5, "previtem"), "role"), "tool")) : v)(J.attr(_loop5, "previtem")))) {
          out.push(J.str("<|im_start|>user\n"));
        }
        out.push(J.str("<tool_response>\n"), J.str(J.attr(message_9, "content")), J.str("\n</tool_response>\n"));
        if (J.t(((v) => J.t(v) ? (!J.eq(J.attr(J.attr(_loop5, "nextitem"), "role"), "tool")) : v)(!J.t(J.attr(_loop5, "last"))))) {
          out.push(J.str("<|im_end|>\n"));
        } else if (J.t(J.attr(_loop5, "last"))) {
          out.push(J.str("<|im_end|>\n"));
        }
      } else {
        out.push(J.str(J.add(J.add(J.add(J.add("<|im_start|>", J.attr(message_9, "role")), "\n"), J.attr(message_9, "content")), "<|im_end|>\n")));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    if (J.t(enable_thinking_0)) {
      out.push(J.str("<|im_start|>assistant\n<think>\n"));
    } else {
      out.push(J.str("<|im_start|>assistant\n<think></think>"));
    }
  }
  return out.join("");
}
