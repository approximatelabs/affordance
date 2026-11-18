// Converted from the chat template of nvidia/Nemotron-Cascade-2-30B-A3B by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let render_extra_keys_0 = ctx["render_extra_keys"];
  let enable_thinking_0 = ctx["enable_thinking"];
  let reasoning_budget_0 = ctx["reasoning_budget"];
  let truncate_history_thinking_0 = ctx["truncate_history_thinking"];
  let ns_0 = ctx["ns"];
  let loop_messages_0 = ctx["loop_messages"];
  let system_message_0 = ctx["system_message"];
  let tools_0 = ctx["tools"] ?? null;
  let messages_0 = ctx["messages"];
  let handled_keys_0 = ctx["handled_keys"];
  let content_0 = ctx["content"];
  let include_content_0 = ctx["include_content"];
  let c_0 = ctx["c"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  render_extra_keys_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_3 = k.caller;
    let json_dict_1 = p.length > 0 ? p[0] : "json_dict" in k ? k["json_dict"] : undefined;
    let handled_keys_2 = p.length > 1 ? p[1] : "handled_keys" in k ? k["handled_keys"] : undefined;
    if (J.t(J.test("mapping", json_dict_1))) {
      {
        let _items1 = J.iter(json_dict_1);
        _items1 = _items1.filter(_it1 => { let json_key_4 = _it1; return J.t((!J.has(json_key_4, handled_keys_2))); });
        const _loop1 = J.loop(_items1);
        for (let _i1 = 0; _i1 < _items1.length; _i1++) {
          _loop1.index0 = _i1; const _it1 = _items1[_i1]; let json_key_4 = _it1;
          if (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? !J.t(J.test("string", J.item(json_dict_1, json_key_4))) : v)(J.test("sequence", J.item(json_dict_1, json_key_4))))(J.test("mapping", J.item(json_dict_1, json_key_4))))) {
            out.push(J.str(J.concat("\n<", json_key_4, ">", J.filter("safe", J.filter("tojson", J.item(json_dict_1, json_key_4))), "</", json_key_4, ">")));
          } else {
            out.push(J.str(J.concat("\n<", json_key_4, ">", J.filter("string", J.item(json_dict_1, json_key_4)), "</", json_key_4, ">")));
          }
        }
      }
    }
    return out.join("");
  };
  enable_thinking_0 = (J.t((enable_thinking_0 !== undefined)) ? enable_thinking_0 : true);
  reasoning_budget_0 = (J.t((reasoning_budget_0 !== undefined)) ? reasoning_budget_0 : null);
  truncate_history_thinking_0 = (J.t((truncate_history_thinking_0 !== undefined)) ? truncate_history_thinking_0 : true);
  ns_0 = J.call(J.namespace, J.kw({"last_user_idx": J.neg(1)}));
  loop_messages_0 = messages_0;
  {
    let _items2 = J.iter(loop_messages_0);
    const _loop2 = J.loop(_items2);
    for (let _i2 = 0; _i2 < _items2.length; _i2++) {
      _loop2.index0 = _i2; const _it2 = _items2[_i2]; let m_5 = _it2;
      if (J.t((J.eq(J.item(m_5, "role"), "user")))) {
        J.setattr(ns_0, "last_user_idx", J.attr(_loop2, "index0"));
      }
    }
  }
  if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
    system_message_0 = J.item(J.item(messages_0, 0), "content");
    loop_messages_0 = J.slice(messages_0, 1, undefined, undefined);
  } else {
    system_message_0 = "You are a helpful and harmless assistant.\n\nYou are not allowed to use any tools.";
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
      _loop3.index0 = _i3; const _it3 = _items3[_i3]; let m_6 = _it3;
      if (J.t((J.eq(J.item(m_6, "role"), "user")))) {
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
        _loop4.index0 = _i4; const _it4 = _items4[_i4]; let tool_7 = _it4;
        let handled_keys_8 = handled_keys_0;
        if (J.t((J.attr(tool_7, "function") !== undefined))) {
          tool_7 = J.attr(tool_7, "function");
        }
        out.push(J.str(J.concat("\n<function>\n<name>", J.attr(tool_7, "name"), "</name>")));
        if (J.t((J.attr(tool_7, "description") !== undefined))) {
          out.push(J.str(J.concat("\n<description>", J.filter("trim", J.attr(tool_7, "description")), "</description>")));
        }
        out.push(J.str("\n<parameters>"));
        if (J.t(((v) => J.t(v) ? J.test("mapping", J.attr(J.attr(tool_7, "parameters"), "properties")) : v)(((v) => J.t(v) ? (J.attr(J.attr(tool_7, "parameters"), "properties") !== undefined) : v)(((v) => J.t(v) ? J.test("mapping", J.attr(tool_7, "parameters")) : v)((J.attr(tool_7, "parameters") !== undefined)))))) {
          {
            let _items5 = J.iter(J.filter("items", J.attr(J.attr(tool_7, "parameters"), "properties")));
            const _loop5 = J.loop(_items5);
            for (let _i5 = 0; _i5 < _items5.length; _i5++) {
              _loop5.index0 = _i5; const _it5 = _items5[_i5]; let [param_name_9, param_fields_10] = J.unpack(_it5, 2);
              let handled_keys_11 = handled_keys_8;
              out.push(J.str("\n<parameter>"), J.str(J.concat("\n<name>", param_name_9, "</name>")));
              if (J.t((J.attr(param_fields_10, "type") !== undefined))) {
                out.push(J.str(J.concat("\n<type>", J.filter("string", J.attr(param_fields_10, "type")), "</type>")));
              }
              if (J.t((J.attr(param_fields_10, "description") !== undefined))) {
                out.push(J.str(J.concat("\n<description>", J.filter("trim", J.attr(param_fields_10, "description")), "</description>")));
              }
              if (J.t((J.attr(param_fields_10, "enum") !== undefined))) {
                out.push(J.str(J.concat("\n<enum>", J.filter("safe", J.filter("tojson", J.attr(param_fields_10, "enum"))), "</enum>")));
              }
              handled_keys_11 = ["name", "type", "description", "enum"];
              out.push(J.str(J.call(render_extra_keys_0, param_fields_10, handled_keys_11)), J.str("\n</parameter>"));
            }
          }
        }
        handled_keys_8 = ["type", "properties", "required"];
        out.push(J.str(J.call(render_extra_keys_0, J.attr(tool_7, "parameters"), handled_keys_8)));
        if (J.t(((v) => J.t(v) ? (J.attr(J.attr(tool_7, "parameters"), "required") !== undefined) : v)((J.attr(tool_7, "parameters") !== undefined)))) {
          out.push(J.str(J.concat("\n<required>", J.filter("safe", J.filter("tojson", J.attr(J.attr(tool_7, "parameters"), "required"))), "</required>")));
        }
        out.push(J.str("\n</parameters>"));
        handled_keys_8 = ["type", "name", "description", "parameters"];
        out.push(J.str(J.call(render_extra_keys_0, tool_7, handled_keys_8)), J.str("\n</function>"));
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
    let _items6 = J.iter(loop_messages_0);
    const _loop6 = J.loop(_items6);
    for (let _i6 = 0; _i6 < _items6.length; _i6++) {
      _loop6.index0 = _i6; const _it6 = _items6[_i6]; let message_12 = _it6;
      let content_13 = content_0;
      let include_content_14 = include_content_0;
      let c_15 = c_0;
      if (J.t((J.eq(J.attr(message_12, "role"), "assistant")))) {
        if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", J.filter("trim", J.attr(message_12, "reasoning_content"))), 0)) : v)(((v) => J.t(v) ? J.test("string", J.attr(message_12, "reasoning_content")) : v)((J.attr(message_12, "reasoning_content") !== undefined))))) {
          content_13 = J.concat("<think>\n", J.attr(message_12, "reasoning_content"), "\n</think>\n", J.filter("default", J.attr(message_12, "content"), "", true));
        } else {
          content_13 = J.filter("default", J.attr(message_12, "content"), "", true);
          if (J.t(J.test("string", content_13))) {
            if (J.t(((v) => J.t(v) ? (!J.has("</think>", content_13)) : v)((!J.has("<think>", content_13))))) {
              content_13 = J.concat("<think></think>", content_13);
            }
          } else {
            content_13 = content_13;
          }
        }
        if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", J.attr(message_12, "tool_calls")), 0)) : v)(((v) => J.t(v) ? J.test("iterable", J.attr(message_12, "tool_calls")) : v)((J.attr(message_12, "tool_calls") !== undefined))))) {
          out.push(J.str("<|im_start|>assistant\n"));
          include_content_14 = !J.t(((v) => J.t(v) ? (J.lt(J.attr(_loop6, "index0"), J.attr(ns_0, "last_user_idx"))) : v)(truncate_history_thinking_0));
          if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", J.filter("trim", content_13)), 0)) : v)(J.test("string", content_13)))) {
            if (J.t(include_content_14)) {
              out.push(J.str(J.concat(J.filter("trim", content_13), "\n")));
            } else {
              c_15 = J.filter("string", content_13);
              if (J.t((J.has("</think>", c_15)))) {
                c_15 = J.item(J.call(J.attr(c_15, "split"), "</think>"), J.neg(1));
              } else if (J.t((J.has("<think>", c_15)))) {
                c_15 = J.item(J.call(J.attr(c_15, "split"), "<think>"), 0);
              }
              c_15 = J.concat("<think></think>", J.filter("trim", c_15));
              if (J.t((J.gt(J.filter("length", c_15), 0)))) {
                out.push(J.str(J.concat(c_15, "\n")));
              }
            }
          } else {
            out.push(J.str("<think></think>"));
          }
          {
            let _items7 = J.iter(J.attr(message_12, "tool_calls"));
            const _loop7 = J.loop(_items7);
            for (let _i7 = 0; _i7 < _items7.length; _i7++) {
              _loop7.index0 = _i7; const _it7 = _items7[_i7]; let tool_call_16 = _it7;
              if (J.t((J.attr(tool_call_16, "function") !== undefined))) {
                tool_call_16 = J.attr(tool_call_16, "function");
              }
              out.push(J.str(J.concat("<tool_call>\n<function=", J.attr(tool_call_16, "name"), ">\n")));
              if (J.t((J.attr(tool_call_16, "arguments") !== undefined))) {
                {
                  let _items8 = J.iter(J.filter("items", J.attr(tool_call_16, "arguments")));
                  const _loop8 = J.loop(_items8);
                  for (let _i8 = 0; _i8 < _items8.length; _i8++) {
                    _loop8.index0 = _i8; const _it8 = _items8[_i8]; let [args_name_17, args_value_18] = J.unpack(_it8, 2);
                    out.push(J.str(J.concat("<parameter=", args_name_17, ">\n")));
                    args_value_18 = (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? !J.t(J.test("string", args_value_18)) : v)(J.test("sequence", args_value_18)))(J.test("mapping", args_value_18))) ? J.filter("safe", J.filter("tojson", args_value_18)) : J.filter("string", args_value_18));
                    out.push(J.str(J.concat(args_value_18, "\n</parameter>\n")));
                  }
                }
              }
              out.push(J.str("</function>\n</tool_call>\n"));
            }
          }
          out.push(J.str("<|im_end|>\n"));
        } else {
          if (J.t(!J.t(((v) => J.t(v) ? (J.lt(J.attr(_loop6, "index0"), J.attr(ns_0, "last_user_idx"))) : v)(truncate_history_thinking_0)))) {
            out.push(J.str(J.concat("<|im_start|>assistant\n", J.filter("trim", J.filter("string", J.filter("default", content_13, "", true))), "<|im_end|>\n")));
          } else {
            c_15 = J.filter("string", J.filter("default", content_13, "", true));
            if (J.t(((v) => J.t(v) ? (J.has("</think>", c_15)) : v)((J.has("<think>", c_15))))) {
              c_15 = J.concat("<think></think>", J.item(J.call(J.attr(c_15, "split"), "</think>"), J.neg(1)));
            }
            c_15 = J.filter("trim", c_15);
            if (J.t((J.gt(J.filter("length", c_15), 0)))) {
              out.push(J.str(J.concat("<|im_start|>assistant\n", c_15, "<|im_end|>\n")));
            } else {
              out.push(J.str("<|im_start|>assistant\n<|im_end|>\n"));
            }
          }
        }
      } else if (J.t(((v) => J.t(v) ? v : (J.eq(J.attr(message_12, "role"), "system")))((J.eq(J.attr(message_12, "role"), "user"))))) {
        out.push(J.str(J.add(J.add("<|im_start|>", J.attr(message_12, "role")), "\n")));
        content_13 = J.filter("string", J.attr(message_12, "content"));
        if (J.t(((v) => J.t(v) ? !J.t((reasoning_budget_0 === null)) : v)(((v) => J.t(v) ? (J.eq(J.attr(_loop6, "index0"), J.attr(ns_0, "last_user_idx"))) : v)((J.eq(J.attr(message_12, "role"), "user")))))) {
          out.push(J.str(J.add(J.add(J.add(content_13, "\n\n{thinking token budget: "), J.filter("string", reasoning_budget_0)), "}")));
        } else {
          out.push(J.str(content_13));
        }
        out.push(J.str("<|im_end|>\n"));
      } else if (J.t((J.eq(J.attr(message_12, "role"), "tool")))) {
        if (J.t(((v) => J.t(v) ? (!J.eq(J.attr(J.attr(_loop6, "previtem"), "role"), "tool")) : v)(J.attr(_loop6, "previtem")))) {
          out.push(J.str("<|im_start|>user\n"));
        }
        out.push(J.str("<tool_response>\n"), J.str(J.attr(message_12, "content")), J.str("\n</tool_response>\n"));
        if (J.t(((v) => J.t(v) ? (!J.eq(J.attr(J.attr(_loop6, "nextitem"), "role"), "tool")) : v)(!J.t(J.attr(_loop6, "last"))))) {
          out.push(J.str("<|im_end|>\n"));
        } else if (J.t(J.attr(_loop6, "last"))) {
          out.push(J.str("<|im_end|>\n"));
        }
      } else {
        out.push(J.str(J.add(J.add(J.add(J.add("<|im_start|>", J.attr(message_12, "role")), "\n"), J.attr(message_12, "content")), "<|im_end|>\n")));
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
