// Converted from the chat template of nvidia/NVIDIA-Nemotron-3.5-Super-VL-120B-A12B-BF16 by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let render_extra_keys_0 = ctx["render_extra_keys"];
  let render_content_0 = ctx["render_content"];
  let enable_thinking_0 = ctx["enable_thinking"];
  let medium_effort_0 = ctx["medium_effort"];
  let truncate_history_thinking_0 = ctx["truncate_history_thinking"];
  let ns_0 = ctx["ns"];
  let loop_messages_0 = ctx["loop_messages"];
  let system_message_0 = ctx["system_message"];
  let tools_0 = ctx["tools"] ?? null;
  let text_ns_0 = ctx["text_ns"];
  let counters_0 = ctx["counters"];
  let content_ns_0 = ctx["content_ns"];
  let render_ns_0 = ctx["render_ns"];
  let sep_0 = ctx["sep"];
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
  render_content_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_6 = k.caller;
    let content_5 = p.length > 0 ? p[0] : "content" in k ? k["content"] : undefined;
    let text_ns_7 = text_ns_0;
    let counters_8 = counters_0;
    let content_ns_9 = content_ns_0;
    let render_ns_10 = render_ns_0;
    if (J.t((content_5 === null))) {
      out.push(J.str(""));
    } else if (J.t(J.test("string", content_5))) {
      out.push(J.str(J.filter("string", content_5)));
    } else {
      text_ns_7 = J.call(J.namespace, J.kw({"val": ""}));
      counters_8 = J.call(J.namespace, J.kw({"images": 0, "videos": 0}));
      {
        let _items2 = J.iter(content_5);
        const _loop2 = J.loop(_items2);
        for (let _i2 = 0; _i2 < _items2.length; _i2++) {
          _loop2.index0 = _i2; const _it2 = _items2[_i2]; let part_11 = _it2;
          if (J.t(((v) => J.t(v) ? v : (J.eq(J.item(part_11, "type"), "input_image")))(((v) => J.t(v) ? v : (J.eq(J.item(part_11, "type"), "image_url")))((J.eq(J.item(part_11, "type"), "image")))))) {
            J.setattr(counters_8, "images", J.add(J.attr(counters_8, "images"), 1));
          } else if (J.t(((v) => J.t(v) ? v : (J.eq(J.item(part_11, "type"), "input_video")))(((v) => J.t(v) ? v : (J.eq(J.item(part_11, "type"), "video_url")))((J.eq(J.item(part_11, "type"), "video")))))) {
            J.setattr(counters_8, "videos", J.add(J.attr(counters_8, "videos"), 1));
          } else if (J.t(((v) => J.t(v) ? v : (J.eq(J.item(part_11, "type"), "input_text")))((J.eq(J.item(part_11, "type"), "text"))))) {
            J.setattr(text_ns_7, "val", J.add(J.attr(text_ns_7, "val"), J.filter("default", J.item(part_11, "text"), "", true)));
          }
        }
      }
      if (J.t((J.has("<image>", J.attr(text_ns_7, "val"))))) {
        J.setattr(counters_8, "images", 0);
      }
      if (J.t((J.has("<video>", J.attr(text_ns_7, "val"))))) {
        J.setattr(counters_8, "videos", 0);
      }
      content_ns_9 = J.call(J.namespace, J.kw({"val": ""}));
      render_ns_10 = J.call(J.namespace, J.kw({"image_index": 0, "after_text": false}));
      {
        let _items3 = J.iter(content_5);
        const _loop3 = J.loop(_items3);
        for (let _i3 = 0; _i3 < _items3.length; _i3++) {
          _loop3.index0 = _i3; const _it3 = _items3[_i3]; let part_12 = _it3;
          let sep_13 = sep_0;
          sep_13 = (J.t(((v) => J.t(v) ? !J.t(J.call(J.attr(J.attr(content_ns_9, "val"), "endswith"), "\n")) : v)(J.attr(render_ns_10, "after_text"))) ? "\n" : "");
          if (J.t(((v) => J.t(v) ? v : (J.eq(J.item(part_12, "type"), "input_image")))(((v) => J.t(v) ? v : (J.eq(J.item(part_12, "type"), "image_url")))((J.eq(J.item(part_12, "type"), "image")))))) {
            J.setattr(render_ns_10, "image_index", J.add(J.attr(render_ns_10, "image_index"), 1));
            if (J.t((J.gt(J.attr(counters_8, "images"), 0)))) {
              if (J.t((J.gt(J.attr(counters_8, "images"), 1)))) {
                J.setattr(content_ns_9, "val", J.add(J.add(J.add(J.add(J.attr(content_ns_9, "val"), sep_13), "<image "), J.filter("string", J.attr(render_ns_10, "image_index"))), "><image>"));
              } else {
                J.setattr(content_ns_9, "val", J.add(J.add(J.attr(content_ns_9, "val"), sep_13), "<image>"));
              }
              J.setattr(render_ns_10, "after_text", false);
            }
          } else if (J.t(((v) => J.t(v) ? v : (J.eq(J.item(part_12, "type"), "input_video")))(((v) => J.t(v) ? v : (J.eq(J.item(part_12, "type"), "video_url")))((J.eq(J.item(part_12, "type"), "video")))))) {
            if (J.t((J.gt(J.attr(counters_8, "videos"), 0)))) {
              J.setattr(content_ns_9, "val", J.add(J.add(J.attr(content_ns_9, "val"), sep_13), "<video>"));
              J.setattr(render_ns_10, "after_text", false);
            }
          } else if (J.t(((v) => J.t(v) ? v : (J.eq(J.item(part_12, "type"), "input_text")))((J.eq(J.item(part_12, "type"), "text"))))) {
            J.setattr(content_ns_9, "val", J.add(J.attr(content_ns_9, "val"), J.filter("default", J.item(part_12, "text"), "", true)));
            if (J.t(J.filter("default", J.item(part_12, "text"), "", true))) {
              J.setattr(render_ns_10, "after_text", true);
            }
          }
        }
      }
      out.push(J.str(J.attr(content_ns_9, "val")));
    }
    return out.join("");
  };
  enable_thinking_0 = (J.t((enable_thinking_0 !== undefined)) ? enable_thinking_0 : true);
  medium_effort_0 = (J.t((medium_effort_0 !== undefined)) ? medium_effort_0 : false);
  truncate_history_thinking_0 = (J.t((truncate_history_thinking_0 !== undefined)) ? truncate_history_thinking_0 : true);
  ns_0 = J.call(J.namespace, J.kw({"last_user_idx": J.neg(1)}));
  loop_messages_0 = messages_0;
  {
    let _items4 = J.iter(loop_messages_0);
    const _loop4 = J.loop(_items4);
    for (let _i4 = 0; _i4 < _items4.length; _i4++) {
      _loop4.index0 = _i4; const _it4 = _items4[_i4]; let m_14 = _it4;
      if (J.t((J.eq(J.item(m_14, "role"), "user")))) {
        J.setattr(ns_0, "last_user_idx", J.attr(_loop4, "index0"));
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
    let _items5 = J.iter(loop_messages_0);
    const _loop5 = J.loop(_items5);
    for (let _i5 = 0; _i5 < _items5.length; _i5++) {
      _loop5.index0 = _i5; const _it5 = _items5[_i5]; let m_15 = _it5;
      if (J.t((J.eq(J.item(m_15, "role"), "user")))) {
        J.setattr(ns_0, "last_user_idx", J.attr(_loop5, "index0"));
      }
    }
  }
  if (J.t((system_message_0 !== undefined))) {
    out.push(J.str(J.concat("<|im_start|>system\n", J.call(render_content_0, system_message_0))));
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
      let _items6 = J.iter(tools_0);
      const _loop6 = J.loop(_items6);
      for (let _i6 = 0; _i6 < _items6.length; _i6++) {
        _loop6.index0 = _i6; const _it6 = _items6[_i6]; let tool_16 = _it6;
        let handled_keys_17 = handled_keys_0;
        if (J.t((J.attr(tool_16, "function") !== undefined))) {
          tool_16 = J.attr(tool_16, "function");
        }
        out.push(J.str(J.concat("\n<function>\n<name>", J.attr(tool_16, "name"), "</name>")));
        if (J.t((J.attr(tool_16, "description") !== undefined))) {
          out.push(J.str(J.concat("\n<description>", J.filter("trim", J.attr(tool_16, "description")), "</description>")));
        }
        out.push(J.str("\n<parameters>"));
        if (J.t(((v) => J.t(v) ? J.test("mapping", J.attr(J.attr(tool_16, "parameters"), "properties")) : v)(((v) => J.t(v) ? (J.attr(J.attr(tool_16, "parameters"), "properties") !== undefined) : v)(((v) => J.t(v) ? J.test("mapping", J.attr(tool_16, "parameters")) : v)((J.attr(tool_16, "parameters") !== undefined)))))) {
          {
            let _items7 = J.iter(J.filter("items", J.attr(J.attr(tool_16, "parameters"), "properties")));
            const _loop7 = J.loop(_items7);
            for (let _i7 = 0; _i7 < _items7.length; _i7++) {
              _loop7.index0 = _i7; const _it7 = _items7[_i7]; let [param_name_18, param_fields_19] = J.unpack(_it7, 2);
              let handled_keys_20 = handled_keys_17;
              out.push(J.str("\n<parameter>"), J.str(J.concat("\n<name>", param_name_18, "</name>")));
              if (J.t((J.attr(param_fields_19, "type") !== undefined))) {
                out.push(J.str(J.concat("\n<type>", J.filter("string", J.attr(param_fields_19, "type")), "</type>")));
              }
              if (J.t((J.attr(param_fields_19, "description") !== undefined))) {
                out.push(J.str(J.concat("\n<description>", J.filter("trim", J.attr(param_fields_19, "description")), "</description>")));
              }
              if (J.t((J.attr(param_fields_19, "enum") !== undefined))) {
                out.push(J.str(J.concat("\n<enum>", J.filter("safe", J.filter("tojson", J.attr(param_fields_19, "enum"))), "</enum>")));
              }
              handled_keys_20 = ["name", "type", "description", "enum"];
              out.push(J.str(J.call(render_extra_keys_0, param_fields_19, handled_keys_20)), J.str("\n</parameter>"));
            }
          }
        }
        handled_keys_17 = ["type", "properties", "required"];
        out.push(J.str(J.call(render_extra_keys_0, J.attr(tool_16, "parameters"), handled_keys_17)));
        if (J.t(((v) => J.t(v) ? (J.attr(J.attr(tool_16, "parameters"), "required") !== undefined) : v)((J.attr(tool_16, "parameters") !== undefined)))) {
          out.push(J.str(J.concat("\n<required>", J.filter("safe", J.filter("tojson", J.attr(J.attr(tool_16, "parameters"), "required"))), "</required>")));
        }
        out.push(J.str("\n</parameters>"));
        handled_keys_17 = ["type", "name", "description", "parameters"];
        out.push(J.str(J.call(render_extra_keys_0, tool_16, handled_keys_17)), J.str("\n</function>"));
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
    let _items8 = J.iter(loop_messages_0);
    const _loop8 = J.loop(_items8);
    for (let _i8 = 0; _i8 < _items8.length; _i8++) {
      _loop8.index0 = _i8; const _it8 = _items8[_i8]; let message_21 = _it8;
      let content_22 = content_0;
      let include_content_23 = include_content_0;
      let c_24 = c_0;
      content_22 = J.call(render_content_0, J.filter("default", J.attr(message_21, "content"), "", true));
      if (J.t((J.eq(J.attr(message_21, "role"), "assistant")))) {
        if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", J.filter("trim", J.attr(message_21, "reasoning_content"))), 0)) : v)(((v) => J.t(v) ? J.test("string", J.attr(message_21, "reasoning_content")) : v)((J.attr(message_21, "reasoning_content") !== undefined))))) {
          content_22 = J.concat("<think>\n", J.attr(message_21, "reasoning_content"), "</think>", content_22);
        } else {
          if (J.t(J.test("string", content_22))) {
            if (J.t(((v) => J.t(v) ? (!J.has("</think>", content_22)) : v)((!J.has("<think>", content_22))))) {
              content_22 = J.concat("<think></think>", content_22);
            }
          }
        }
        if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", J.attr(message_21, "tool_calls")), 0)) : v)(((v) => J.t(v) ? J.test("iterable", J.attr(message_21, "tool_calls")) : v)((J.attr(message_21, "tool_calls") !== undefined))))) {
          out.push(J.str("<|im_start|>assistant\n"));
          include_content_23 = !J.t(((v) => J.t(v) ? (J.lt(J.attr(_loop8, "index0"), J.attr(ns_0, "last_user_idx"))) : v)(truncate_history_thinking_0));
          if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", J.filter("trim", content_22)), 0)) : v)(J.test("string", content_22)))) {
            if (J.t(include_content_23)) {
              out.push(J.str(J.concat(J.filter("trim", content_22), "\n")));
            } else {
              c_24 = J.filter("string", content_22);
              if (J.t((J.has("</think>", c_24)))) {
                c_24 = J.item(J.call(J.attr(c_24, "split"), "</think>"), J.neg(1));
              } else if (J.t((J.has("<think>", c_24)))) {
                c_24 = J.item(J.call(J.attr(c_24, "split"), "<think>"), 0);
              }
              c_24 = J.concat("<think></think>", c_24);
              if (J.t((J.gt(J.filter("length", c_24), 0)))) {
                out.push(J.str(J.concat(c_24, "\n")));
              }
            }
          } else {
            out.push(J.str("<think></think>"));
          }
          {
            let _items9 = J.iter(J.attr(message_21, "tool_calls"));
            const _loop9 = J.loop(_items9);
            for (let _i9 = 0; _i9 < _items9.length; _i9++) {
              _loop9.index0 = _i9; const _it9 = _items9[_i9]; let tool_call_25 = _it9;
              if (J.t((J.attr(tool_call_25, "function") !== undefined))) {
                tool_call_25 = J.attr(tool_call_25, "function");
              }
              out.push(J.str(J.concat("<tool_call>\n<function=", J.attr(tool_call_25, "name"), ">\n")));
              if (J.t((J.attr(tool_call_25, "arguments") !== undefined))) {
                {
                  let _items10 = J.iter(J.filter("items", J.attr(tool_call_25, "arguments")));
                  const _loop10 = J.loop(_items10);
                  for (let _i10 = 0; _i10 < _items10.length; _i10++) {
                    _loop10.index0 = _i10; const _it10 = _items10[_i10]; let [args_name_26, args_value_27] = J.unpack(_it10, 2);
                    out.push(J.str(J.concat("<parameter=", args_name_26, ">\n")));
                    args_value_27 = (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? !J.t(J.test("string", args_value_27)) : v)(J.test("sequence", args_value_27)))(J.test("mapping", args_value_27))) ? J.filter("safe", J.filter("tojson", args_value_27)) : J.filter("string", args_value_27));
                    out.push(J.str(J.concat(args_value_27, "\n</parameter>\n")));
                  }
                }
              }
              out.push(J.str("</function>\n</tool_call>\n"));
            }
          }
          out.push(J.str("<|im_end|>\n"));
        } else {
          if (J.t(!J.t(((v) => J.t(v) ? (J.lt(J.attr(_loop8, "index0"), J.attr(ns_0, "last_user_idx"))) : v)(truncate_history_thinking_0)))) {
            out.push(J.str(J.concat("<|im_start|>assistant\n", J.filter("trim", J.filter("string", J.filter("default", content_22, "", true))), "<|im_end|>\n")));
          } else {
            c_24 = J.filter("string", J.filter("default", content_22, "", true));
            if (J.t(((v) => J.t(v) ? (J.has("</think>", c_24)) : v)((J.has("<think>", c_24))))) {
              c_24 = J.concat("<think></think>", J.item(J.call(J.attr(c_24, "split"), "</think>"), J.neg(1)));
            }
            c_24 = J.filter("trim", c_24);
            if (J.t((J.gt(J.filter("length", c_24), 0)))) {
              out.push(J.str(J.concat("<|im_start|>assistant\n", c_24, "<|im_end|>\n")));
            } else {
              out.push(J.str("<|im_start|>assistant\n<|im_end|>\n"));
            }
          }
        }
      } else if (J.t(((v) => J.t(v) ? v : (J.eq(J.attr(message_21, "role"), "system")))((J.eq(J.attr(message_21, "role"), "user"))))) {
        out.push(J.str(J.add(J.add("<|im_start|>", J.attr(message_21, "role")), "\n")));
        if (J.t(((v) => J.t(v) ? medium_effort_0 : v)(((v) => J.t(v) ? (J.eq(J.attr(_loop8, "index0"), J.attr(ns_0, "last_user_idx"))) : v)((J.eq(J.attr(message_21, "role"), "user")))))) {
          out.push(J.str(J.add(content_22, "\n\n{reasoning effort: efficient}")));
        } else {
          out.push(J.str(content_22));
        }
        out.push(J.str("<|im_end|>\n"));
      } else if (J.t((J.eq(J.attr(message_21, "role"), "tool")))) {
        if (J.t(((v) => J.t(v) ? (!J.eq(J.attr(J.attr(_loop8, "previtem"), "role"), "tool")) : v)(J.attr(_loop8, "previtem")))) {
          out.push(J.str("<|im_start|>user\n"));
        }
        out.push(J.str("<tool_response>\n"), J.str(content_22), J.str("\n</tool_response>\n"));
        if (J.t(((v) => J.t(v) ? (!J.eq(J.attr(J.attr(_loop8, "nextitem"), "role"), "tool")) : v)(!J.t(J.attr(_loop8, "last"))))) {
          out.push(J.str("<|im_end|>\n"));
        } else if (J.t(J.attr(_loop8, "last"))) {
          out.push(J.str("<|im_end|>\n"));
        }
      } else {
        out.push(J.str(J.concat("<|im_start|>", J.attr(message_21, "role"), "\n", content_22, "<|im_end|>\n")));
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
