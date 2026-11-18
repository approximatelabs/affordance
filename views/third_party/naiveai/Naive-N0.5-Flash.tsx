// Converted from the chat template of NaiveAI/Naive-N0.5-Flash by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let enable_thinking_0 = ctx["enable_thinking"];
  let keep_all_reasoning_0 = ctx["keep_all_reasoning"];
  let render_extra_keys_0 = ctx["render_extra_keys"];
  let render_content_0 = ctx["render_content"];
  let system_message_0 = ctx["system_message"];
  let loop_messages_0 = ctx["loop_messages"];
  let ns_0 = ctx["ns"];
  let tools_0 = ctx["tools"] ?? null;
  let messages_0 = ctx["messages"];
  let handled_keys_0 = ctx["handled_keys"];
  let content_0 = ctx["content"];
  let reasoning_content_0 = ctx["reasoning_content"];
  if (J.t(!J.t((add_generation_prompt_0 !== undefined)))) {
    add_generation_prompt_0 = false;
  }
  if (J.t(!J.t((enable_thinking_0 !== undefined)))) {
    enable_thinking_0 = true;
  }
  if (J.t(!J.t((keep_all_reasoning_0 !== undefined)))) {
    keep_all_reasoning_0 = true;
  }
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
  render_content_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_6 = k.caller;
    let message_content_5 = p.length > 0 ? p[0] : "message_content" in k ? k["message_content"] : undefined;
    if (J.t(J.test("string", message_content_5))) {
      out.push(J.str(message_content_5));
    } else {
      {
        let _items2 = J.iter(message_content_5);
        const _loop2 = J.loop(_items2);
        for (let _i2 = 0; _i2 < _items2.length; _i2++) {
          _loop2.index0 = _i2; const _it2 = _items2[_i2]; let content_7 = _it2;
          if (J.t((J.has("text", content_7)))) {
            out.push(J.str(J.item(content_7, "text")));
          }
        }
      }
    }
    return out.join("");
  };
  if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
    system_message_0 = J.item(J.item(messages_0, 0), "content");
    loop_messages_0 = J.slice(messages_0, 1, undefined, undefined);
  } else {
    loop_messages_0 = messages_0;
  }
  ns_0 = J.call(J.namespace, J.kw({"last_user_index": J.neg(1)}));
  {
    let _items3 = J.iter(loop_messages_0);
    const _loop3 = J.loop(_items3);
    for (let _i3 = 0; _i3 < _items3.length; _i3++) {
      _loop3.index0 = _i3; const _it3 = _items3[_i3]; let m_8 = _it3;
      if (J.t((J.eq(J.attr(m_8, "role"), "user")))) {
        J.setattr(ns_0, "last_user_index", J.attr(_loop3, "index0"));
      }
    }
  }
  if (J.t(!J.t((tools_0 !== undefined)))) {
    tools_0 = [];
  }
  if (J.t((system_message_0 !== undefined))) {
    out.push(J.str(J.add("<|im_start|>system\n", J.call(render_content_0, system_message_0))));
  } else {
    out.push(J.str("<|im_start|>system\nYou are Naive-N0.5-Flash, a helpful AI assistant engineered by Naive AI."));
  }
  if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", tools_0), 0)) : v)(J.test("iterable", tools_0)))) {
    out.push(J.str("\n\n# Tools\n\nYou may call one or more functions to assist with the user query.\n\nYou have access to the following functions:\n\n"), J.str("<tools>"));
    {
      let _items4 = J.iter(tools_0);
      const _loop4 = J.loop(_items4);
      for (let _i4 = 0; _i4 < _items4.length; _i4++) {
        _loop4.index0 = _i4; const _it4 = _items4[_i4]; let tool_9 = _it4;
        let handled_keys_10 = handled_keys_0;
        if (J.t((J.attr(tool_9, "function") !== undefined))) {
          tool_9 = J.attr(tool_9, "function");
        }
        out.push(J.str(J.concat("\n<function>\n<name>", J.attr(tool_9, "name"), "</name>")));
        if (J.t((J.attr(tool_9, "description") !== undefined))) {
          out.push(J.str(J.concat("\n<description>", J.filter("trim", J.attr(tool_9, "description")), "</description>")));
        }
        out.push(J.str("\n<parameters>"));
        if (J.t(((v) => J.t(v) ? J.test("mapping", J.attr(J.attr(tool_9, "parameters"), "properties")) : v)(((v) => J.t(v) ? (J.attr(J.attr(tool_9, "parameters"), "properties") !== undefined) : v)(((v) => J.t(v) ? J.test("mapping", J.attr(tool_9, "parameters")) : v)((J.attr(tool_9, "parameters") !== undefined)))))) {
          {
            let _items5 = J.iter(J.filter("items", J.attr(J.attr(tool_9, "parameters"), "properties")));
            const _loop5 = J.loop(_items5);
            for (let _i5 = 0; _i5 < _items5.length; _i5++) {
              _loop5.index0 = _i5; const _it5 = _items5[_i5]; let [param_name_11, param_fields_12] = J.unpack(_it5, 2);
              let handled_keys_13 = handled_keys_10;
              out.push(J.str("\n<parameter>"), J.str(J.concat("\n<name>", param_name_11, "</name>")));
              if (J.t((J.attr(param_fields_12, "type") !== undefined))) {
                out.push(J.str(J.concat("\n<type>", J.filter("string", J.attr(param_fields_12, "type")), "</type>")));
              }
              if (J.t((J.attr(param_fields_12, "description") !== undefined))) {
                out.push(J.str(J.concat("\n<description>", J.filter("trim", J.attr(param_fields_12, "description")), "</description>")));
              }
              handled_keys_13 = ["name", "type", "description"];
              out.push(J.str(J.call(render_extra_keys_0, param_fields_12, handled_keys_13)), J.str("\n</parameter>"));
            }
          }
        }
        handled_keys_10 = ["type", "properties"];
        out.push(J.str(J.call(render_extra_keys_0, J.attr(tool_9, "parameters"), handled_keys_10)), J.str("\n</parameters>"));
        handled_keys_10 = ["type", "name", "description", "parameters"];
        out.push(J.str(J.call(render_extra_keys_0, tool_9, handled_keys_10)), J.str("\n</function>"));
      }
    }
    out.push(J.str("\n</tools>"), J.str("\n\nFor each function call, output the function name and arguments in the following format:\n<tool_call>\n<function=example_function_name>\n<parameter=example_parameter_1>value_1</parameter>\n<parameter=example_parameter_2>This is the value for the second parameter\nthat can span\nmultiple lines</parameter>\n</function>\n</tool_call>\n\n<IMPORTANT>\n- Function calls MUST follow the specified format: an inner <function=...></function> block must be nested within <tool_call></tool_call> XML tags\n- DO NOT use function calls inside <think></think> tags.\n- The value enclosed between parameter tags is preserved exactly as-is, including newlines and spaces.\n</IMPORTANT>"));
  }
  out.push(J.str("<|im_end|>"));
  {
    let _items6 = J.iter(loop_messages_0);
    const _loop6 = J.loop(_items6);
    for (let _i6 = 0; _i6 < _items6.length; _i6++) {
      _loop6.index0 = _i6; const _it6 = _items6[_i6]; let message_14 = _it6;
      let content_15 = content_0;
      let reasoning_content_16 = reasoning_content_0;
      if (J.t(J.test("string", J.attr(message_14, "content")))) {
        content_15 = J.attr(message_14, "content");
      } else {
        content_15 = J.call(render_content_0, J.attr(message_14, "content"));
      }
      if (J.t((J.eq(J.attr(message_14, "role"), "assistant")))) {
        if (J.t(J.test("string", J.attr(message_14, "reasoning_content")))) {
          reasoning_content_16 = J.attr(message_14, "reasoning_content");
        } else {
          reasoning_content_16 = "";
          if (J.t((J.has("</think>", content_15)))) {
            reasoning_content_16 = J.item(J.call(J.attr(J.item(J.call(J.attr(content_15, "split"), "</think>"), 0), "split"), "<think>"), J.neg(1));
            content_15 = J.item(J.call(J.attr(content_15, "split"), "</think>"), J.neg(1));
          }
        }
        if (J.t(((v) => J.t(v) ? reasoning_content_16 : v)(((v) => J.t(v) ? v : (J.gt(J.attr(_loop6, "index0"), J.attr(ns_0, "last_user_index"))))(keep_all_reasoning_0)))) {
          out.push(J.str(J.add(J.add(J.add(J.add(J.add("<|im_start|>", J.attr(message_14, "role")), "\n<think>"), reasoning_content_16), "</think>"), content_15)));
        } else {
          out.push(J.str(J.add(J.add(J.add("<|im_start|>", J.attr(message_14, "role")), "\n<think></think>"), content_15)));
        }
        if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", J.attr(message_14, "tool_calls")), 0)) : v)(((v) => J.t(v) ? J.test("iterable", J.attr(message_14, "tool_calls")) : v)((J.attr(message_14, "tool_calls") !== undefined))))) {
          {
            let _items7 = J.iter(J.attr(message_14, "tool_calls"));
            const _loop7 = J.loop(_items7);
            for (let _i7 = 0; _i7 < _items7.length; _i7++) {
              _loop7.index0 = _i7; const _it7 = _items7[_i7]; let tool_call_17 = _it7;
              if (J.t((J.attr(tool_call_17, "function") !== undefined))) {
                tool_call_17 = J.attr(tool_call_17, "function");
              }
              out.push(J.str(J.add(J.add("<tool_call>\n<function=", J.attr(tool_call_17, "name")), ">\n")));
              if (J.t((J.attr(tool_call_17, "arguments") !== undefined))) {
                {
                  let _items8 = J.iter(J.filter("items", J.attr(tool_call_17, "arguments")));
                  const _loop8 = J.loop(_items8);
                  for (let _i8 = 0; _i8 < _items8.length; _i8++) {
                    _loop8.index0 = _i8; const _it8 = _items8[_i8]; let [args_name_18, args_value_19] = J.unpack(_it8, 2);
                    out.push(J.str(J.add(J.add("<parameter=", args_name_18), ">")));
                    args_value_19 = (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? !J.t(J.test("string", args_value_19)) : v)(J.test("sequence", args_value_19)))(J.test("mapping", args_value_19))) ? J.filter("safe", J.filter("tojson", args_value_19)) : J.filter("string", args_value_19));
                    out.push(J.str(args_value_19), J.str("</parameter>\n"));
                  }
                }
              }
              out.push(J.str("</function>\n</tool_call>"));
            }
          }
        }
        out.push(J.str("<|im_end|>"));
      } else if (J.t((J.eq(J.attr(message_14, "role"), "user")))) {
        out.push(J.str(J.add(J.add(J.add(J.add("<|im_start|>", J.attr(message_14, "role")), "\n"), J.call(render_content_0, J.attr(message_14, "content"))), "<|im_end|>")));
      } else if (J.t((J.eq(J.attr(message_14, "role"), "system")))) {
        out.push(J.str(J.add(J.add(J.add(J.add("<|im_start|>", J.attr(message_14, "role")), "\n"), J.call(render_content_0, J.attr(message_14, "content"))), "<|im_end|>")));
      } else if (J.t((J.eq(J.attr(message_14, "role"), "tool")))) {
        if (J.t(((v) => J.t(v) ? (!J.eq(J.attr(J.attr(_loop6, "previtem"), "role"), "tool")) : v)(J.attr(_loop6, "previtem")))) {
          out.push(J.str("<|im_start|>tool\n"));
        }
        out.push(J.str("<tool_response>\n"), J.str(J.call(render_content_0, J.attr(message_14, "content"))), J.str("\n</tool_response>\n"));
        if (J.t(((v) => J.t(v) ? (!J.eq(J.attr(J.attr(_loop6, "nextitem"), "role"), "tool")) : v)(!J.t(J.attr(_loop6, "last"))))) {
          out.push(J.str("<|im_end|>"));
        } else if (J.t(J.attr(_loop6, "last"))) {
          out.push(J.str("<|im_end|>"));
        }
      } else {
        out.push(J.str(J.add(J.add(J.add(J.add("<|im_start|>", J.attr(message_14, "role")), "\n"), J.call(render_content_0, J.attr(message_14, "content"))), "<|im_end|>")));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|im_start|>assistant\n"));
    if (J.t(!J.t(enable_thinking_0))) {
      out.push(J.str("<think></think>"));
    } else {
      out.push(J.str(""));
    }
  }
  return out.join("");
}
