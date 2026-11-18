// Converted from the chat template of nvidia/Nemotron-3-Nano-Omni-30B-A3B-Reasoning-BF16 by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let render_extra_keys_0 = ctx["render_extra_keys"];
  let enable_thinking_0 = ctx["enable_thinking"];
  let reasoning_budget_0 = ctx["reasoning_budget"];
  let truncate_history_thinking_0 = ctx["truncate_history_thinking"];
  let toggle_0 = ctx["toggle"];
  let ns_0 = ctx["ns"];
  let loop_messages_0 = ctx["loop_messages"];
  let system_message_0 = ctx["system_message"];
  let tools_0 = ctx["tools"] ?? null;
  let sys_content_0 = ctx["sys_content"];
  let messages_0 = ctx["messages"];
  let c_0 = ctx["c"];
  let handled_keys_0 = ctx["handled_keys"];
  let content_0 = ctx["content"];
  let include_content_0 = ctx["include_content"];
  let text_ns_0 = ctx["text_ns"];
  let mm_content_0 = ctx["mm_content"];
  let counters_0 = ctx["counters"];
  let image_tags_0 = ctx["image_tags"];
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
  toggle_0 = J.call(J.namespace, J.kw({"enable": enable_thinking_0}));
  {
    let _items2 = J.iter(messages_0);
    const _loop2 = J.loop(_items2);
    for (let _i2 = 0; _i2 < _items2.length; _i2++) {
      _loop2.index0 = _i2; const _it2 = _items2[_i2]; let m_5 = _it2;
      let c_6 = c_0;
      if (J.t(((v) => J.t(v) ? v : (J.eq(J.item(m_5, "role"), "system")))((J.eq(J.item(m_5, "role"), "user"))))) {
        if (J.t(J.test("string", J.item(m_5, "content")))) {
          c_6 = J.item(m_5, "content");
          if (J.t((J.has("/think", J.call(J.attr(c_6, "replace"), "</think>", ""))))) {
            J.setattr(toggle_0, "enable", true);
          } else if (J.t((J.has("/no_think", c_6)))) {
            J.setattr(toggle_0, "enable", false);
          }
        } else {
          {
            let _items3 = J.iter(J.item(m_5, "content"));
            const _loop3 = J.loop(_items3);
            for (let _i3 = 0; _i3 < _items3.length; _i3++) {
              _loop3.index0 = _i3; const _it3 = _items3[_i3]; let part_7 = _it3;
              let c_8 = c_6;
              if (J.t((J.eq(J.item(part_7, "type"), "text")))) {
                c_8 = J.item(part_7, "text");
                if (J.t((J.has("/think", J.call(J.attr(c_8, "replace"), "</think>", ""))))) {
                  J.setattr(toggle_0, "enable", true);
                } else if (J.t((J.has("/no_think", c_8)))) {
                  J.setattr(toggle_0, "enable", false);
                }
              }
            }
          }
        }
      }
    }
  }
  ns_0 = J.call(J.namespace, J.kw({"last_user_idx": J.neg(1)}));
  loop_messages_0 = messages_0;
  {
    let _items4 = J.iter(loop_messages_0);
    const _loop4 = J.loop(_items4);
    for (let _i4 = 0; _i4 < _items4.length; _i4++) {
      _loop4.index0 = _i4; const _it4 = _items4[_i4]; let m_9 = _it4;
      if (J.t((J.eq(J.item(m_9, "role"), "user")))) {
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
      _loop5.index0 = _i5; const _it5 = _items5[_i5]; let m_10 = _it5;
      if (J.t((J.eq(J.item(m_10, "role"), "user")))) {
        J.setattr(ns_0, "last_user_idx", J.attr(_loop5, "index0"));
      }
    }
  }
  if (J.t((system_message_0 !== undefined))) {
    sys_content_0 = J.filter("string", system_message_0);
    sys_content_0 = J.call(J.attr(J.call(J.attr(J.call(J.attr(J.call(J.attr(sys_content_0, "replace"), "</think>", "<_end_think>"), "replace"), "/think", ""), "replace"), "/no_think", ""), "replace"), "<_end_think>", "</think>");
    out.push(J.str(J.add("<|im_start|>system\n", sys_content_0)));
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
        _loop6.index0 = _i6; const _it6 = _items6[_i6]; let tool_11 = _it6;
        let handled_keys_12 = handled_keys_0;
        if (J.t((J.attr(tool_11, "function") !== undefined))) {
          tool_11 = J.attr(tool_11, "function");
        }
        out.push(J.str(J.concat("\n<function>\n<name>", J.attr(tool_11, "name"), "</name>")));
        if (J.t((J.attr(tool_11, "description") !== undefined))) {
          out.push(J.str(J.concat("\n<description>", J.filter("trim", J.attr(tool_11, "description")), "</description>")));
        }
        out.push(J.str("\n<parameters>"));
        if (J.t(((v) => J.t(v) ? J.test("mapping", J.attr(J.attr(tool_11, "parameters"), "properties")) : v)(((v) => J.t(v) ? (J.attr(J.attr(tool_11, "parameters"), "properties") !== undefined) : v)(((v) => J.t(v) ? J.test("mapping", J.attr(tool_11, "parameters")) : v)((J.attr(tool_11, "parameters") !== undefined)))))) {
          {
            let _items7 = J.iter(J.filter("items", J.attr(J.attr(tool_11, "parameters"), "properties")));
            const _loop7 = J.loop(_items7);
            for (let _i7 = 0; _i7 < _items7.length; _i7++) {
              _loop7.index0 = _i7; const _it7 = _items7[_i7]; let [param_name_13, param_fields_14] = J.unpack(_it7, 2);
              let handled_keys_15 = handled_keys_12;
              out.push(J.str("\n<parameter>"), J.str(J.concat("\n<name>", param_name_13, "</name>")));
              if (J.t((J.attr(param_fields_14, "type") !== undefined))) {
                out.push(J.str(J.concat("\n<type>", J.filter("string", J.attr(param_fields_14, "type")), "</type>")));
              }
              if (J.t((J.attr(param_fields_14, "description") !== undefined))) {
                out.push(J.str(J.concat("\n<description>", J.filter("trim", J.attr(param_fields_14, "description")), "</description>")));
              }
              if (J.t((J.attr(param_fields_14, "enum") !== undefined))) {
                out.push(J.str(J.concat("\n<enum>", J.filter("safe", J.filter("tojson", J.attr(param_fields_14, "enum"))), "</enum>")));
              }
              handled_keys_15 = ["name", "type", "description", "enum"];
              out.push(J.str(J.call(render_extra_keys_0, param_fields_14, handled_keys_15)), J.str("\n</parameter>"));
            }
          }
        }
        handled_keys_12 = ["type", "properties", "required"];
        out.push(J.str(J.call(render_extra_keys_0, J.attr(tool_11, "parameters"), handled_keys_12)));
        if (J.t(((v) => J.t(v) ? (J.attr(J.attr(tool_11, "parameters"), "required") !== undefined) : v)((J.attr(tool_11, "parameters") !== undefined)))) {
          out.push(J.str(J.concat("\n<required>", J.filter("safe", J.filter("tojson", J.attr(J.attr(tool_11, "parameters"), "required"))), "</required>")));
        }
        out.push(J.str("\n</parameters>"));
        handled_keys_12 = ["type", "name", "description", "parameters"];
        out.push(J.str(J.call(render_extra_keys_0, tool_11, handled_keys_12)), J.str("\n</function>"));
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
      _loop8.index0 = _i8; const _it8 = _items8[_i8]; let message_16 = _it8;
      let content_17 = content_0;
      let include_content_18 = include_content_0;
      let c_19 = c_0;
      let text_ns_20 = text_ns_0;
      let mm_content_21 = mm_content_0;
      let counters_22 = counters_0;
      let image_tags_23 = image_tags_0;
      if (J.t((J.eq(J.attr(message_16, "role"), "assistant")))) {
        if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", J.filter("trim", J.attr(message_16, "reasoning_content"))), 0)) : v)(((v) => J.t(v) ? J.test("string", J.attr(message_16, "reasoning_content")) : v)((J.attr(message_16, "reasoning_content") !== undefined))))) {
          content_17 = J.concat("<think>\n", J.attr(message_16, "reasoning_content"), "\n</think>\n", J.filter("default", J.attr(message_16, "content"), "", true));
        } else {
          content_17 = J.filter("default", J.attr(message_16, "content"), "", true);
          if (J.t(J.test("string", content_17))) {
            if (J.t(((v) => J.t(v) ? (!J.has("</think>", content_17)) : v)((!J.has("<think>", content_17))))) {
              content_17 = J.concat("<think></think>", content_17);
            }
          } else {
            content_17 = content_17;
          }
        }
        if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", J.attr(message_16, "tool_calls")), 0)) : v)(((v) => J.t(v) ? J.test("iterable", J.attr(message_16, "tool_calls")) : v)((J.attr(message_16, "tool_calls") !== undefined))))) {
          out.push(J.str("<|im_start|>assistant\n"));
          include_content_18 = !J.t(((v) => J.t(v) ? (J.lt(J.attr(_loop8, "index0"), J.attr(ns_0, "last_user_idx"))) : v)(truncate_history_thinking_0));
          if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", J.filter("trim", content_17)), 0)) : v)(J.test("string", content_17)))) {
            if (J.t(include_content_18)) {
              out.push(J.str(J.concat(J.filter("trim", content_17), "\n")));
            } else {
              c_19 = J.filter("string", content_17);
              if (J.t((J.has("</think>", c_19)))) {
                c_19 = J.item(J.call(J.attr(c_19, "split"), "</think>"), J.neg(1));
              } else if (J.t((J.has("<think>", c_19)))) {
                c_19 = J.item(J.call(J.attr(c_19, "split"), "<think>"), 0);
              }
              c_19 = J.concat("<think></think>", J.filter("trim", c_19));
              if (J.t((J.gt(J.filter("length", c_19), 0)))) {
                out.push(J.str(J.concat(c_19, "\n")));
              }
            }
          } else {
            out.push(J.str("<think></think>"));
          }
          {
            let _items9 = J.iter(J.attr(message_16, "tool_calls"));
            const _loop9 = J.loop(_items9);
            for (let _i9 = 0; _i9 < _items9.length; _i9++) {
              _loop9.index0 = _i9; const _it9 = _items9[_i9]; let tool_call_24 = _it9;
              if (J.t((J.attr(tool_call_24, "function") !== undefined))) {
                tool_call_24 = J.attr(tool_call_24, "function");
              }
              out.push(J.str(J.concat("<tool_call>\n<function=", J.attr(tool_call_24, "name"), ">\n")));
              if (J.t((J.attr(tool_call_24, "arguments") !== undefined))) {
                {
                  let _items10 = J.iter(J.filter("items", J.attr(tool_call_24, "arguments")));
                  const _loop10 = J.loop(_items10);
                  for (let _i10 = 0; _i10 < _items10.length; _i10++) {
                    _loop10.index0 = _i10; const _it10 = _items10[_i10]; let [args_name_25, args_value_26] = J.unpack(_it10, 2);
                    out.push(J.str(J.concat("<parameter=", args_name_25, ">\n")));
                    args_value_26 = (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? !J.t(J.test("string", args_value_26)) : v)(J.test("sequence", args_value_26)))(J.test("mapping", args_value_26))) ? J.filter("safe", J.filter("tojson", args_value_26)) : J.filter("string", args_value_26));
                    out.push(J.str(J.concat(args_value_26, "\n</parameter>\n")));
                  }
                }
              }
              out.push(J.str("</function>\n</tool_call>\n"));
            }
          }
          out.push(J.str("<|im_end|>\n"));
        } else {
          if (J.t(!J.t(((v) => J.t(v) ? (J.lt(J.attr(_loop8, "index0"), J.attr(ns_0, "last_user_idx"))) : v)(truncate_history_thinking_0)))) {
            out.push(J.str(J.concat("<|im_start|>assistant\n", J.filter("trim", J.filter("string", J.filter("default", content_17, "", true))), "<|im_end|>\n")));
          } else {
            c_19 = J.filter("string", J.filter("default", content_17, "", true));
            if (J.t(((v) => J.t(v) ? (J.has("</think>", c_19)) : v)((J.has("<think>", c_19))))) {
              c_19 = J.concat("<think></think>", J.item(J.call(J.attr(c_19, "split"), "</think>"), J.neg(1)));
            }
            c_19 = J.filter("trim", c_19);
            if (J.t((J.gt(J.filter("length", c_19), 0)))) {
              out.push(J.str(J.concat("<|im_start|>assistant\n", c_19, "<|im_end|>\n")));
            } else {
              out.push(J.str("<|im_start|>assistant\n<|im_end|>\n"));
            }
          }
        }
      } else if (J.t(((v) => J.t(v) ? v : (J.eq(J.attr(message_16, "role"), "system")))((J.eq(J.attr(message_16, "role"), "user"))))) {
        out.push(J.str(J.add(J.add("<|im_start|>", J.attr(message_16, "role")), "\n")));
        if (J.t(J.test("string", J.attr(message_16, "content")))) {
          content_17 = J.filter("string", J.attr(message_16, "content"));
        } else {
          text_ns_20 = J.call(J.namespace, J.kw({"val": ""}));
          mm_content_21 = "";
          counters_22 = J.call(J.namespace, J.kw({"images": 0, "videos": 0, "audios": 0}));
          {
            let _items11 = J.iter(J.item(message_16, "content"));
            const _loop11 = J.loop(_items11);
            for (let _i11 = 0; _i11 < _items11.length; _i11++) {
              _loop11.index0 = _i11; const _it11 = _items11[_i11]; let part_27 = _it11;
              if (J.t(((v) => J.t(v) ? v : (J.eq(J.item(part_27, "type"), "image_url")))((J.eq(J.item(part_27, "type"), "image"))))) {
                J.setattr(counters_22, "images", J.add(J.attr(counters_22, "images"), 1));
              } else if (J.t(((v) => J.t(v) ? v : (J.eq(J.item(part_27, "type"), "video_url")))((J.eq(J.item(part_27, "type"), "video"))))) {
                J.setattr(counters_22, "videos", J.add(J.attr(counters_22, "videos"), 1));
              } else if (J.t(((v) => J.t(v) ? v : (J.eq(J.item(part_27, "type"), "audio_url")))((J.eq(J.item(part_27, "type"), "audio"))))) {
                J.setattr(counters_22, "audios", J.add(J.attr(counters_22, "audios"), 1));
              } else if (J.t((J.eq(J.item(part_27, "type"), "text")))) {
                J.setattr(text_ns_20, "val", J.add(J.attr(text_ns_20, "val"), J.item(part_27, "text")));
              }
            }
          }
          if (J.t((J.has("<image>", J.attr(text_ns_20, "val"))))) {
            J.setattr(counters_22, "images", 0);
          }
          if (J.t((J.has("<video>", J.attr(text_ns_20, "val"))))) {
            J.setattr(counters_22, "videos", 0);
          }
          if (J.t((J.has("<so_embedding>", J.attr(text_ns_20, "val"))))) {
            J.setattr(counters_22, "audios", 0);
          }
          if (J.t((J.gt(J.attr(counters_22, "images"), 1)))) {
            image_tags_23 = J.call(J.namespace, J.kw({"tags": []}));
            {
              let _items12 = J.iter(J.call(J.range, J.attr(counters_22, "images")));
              const _loop12 = J.loop(_items12);
              for (let _i12 = 0; _i12 < _items12.length; _i12++) {
                _loop12.index0 = _i12; const _it12 = _items12[_i12]; let i_28 = _it12;
                J.setattr(image_tags_23, "tags", J.add(J.attr(image_tags_23, "tags"), [J.add(J.add("<image ", J.filter("string", J.add(i_28, 1))), "><image>")]));
              }
            }
            mm_content_21 = J.add(J.call(J.attr(" ", "join"), J.attr(image_tags_23, "tags")), "\n");
          } else if (J.t((J.eq(J.attr(counters_22, "images"), 1)))) {
            mm_content_21 = "<image>\n";
          }
          mm_content_21 = J.add(mm_content_21, J.mul("<video>\n", J.attr(counters_22, "videos")));
          mm_content_21 = J.add(mm_content_21, J.mul("<so_embedding>\n", J.attr(counters_22, "audios")));
          content_17 = J.add(mm_content_21, J.call(J.attr(J.attr(text_ns_20, "val"), "lstrip"), "\n"));
        }
        content_17 = J.call(J.attr(J.call(J.attr(J.call(J.attr(J.call(J.attr(content_17, "replace"), "</think>", "<_end_think>"), "replace"), "/think", ""), "replace"), "/no_think", ""), "replace"), "<_end_think>", "</think>");
        content_17 = J.filter("trim", content_17);
        if (J.t(((v) => J.t(v) ? !J.t((reasoning_budget_0 === null)) : v)(((v) => J.t(v) ? (J.eq(J.attr(_loop8, "index0"), J.attr(ns_0, "last_user_idx"))) : v)((J.eq(J.attr(message_16, "role"), "user")))))) {
          out.push(J.str(J.add(J.add(J.add(content_17, "\n\n{thinking token budget: "), J.filter("string", reasoning_budget_0)), "}")));
        } else {
          out.push(J.str(content_17));
        }
        out.push(J.str("<|im_end|>\n"));
      } else if (J.t((J.eq(J.attr(message_16, "role"), "tool")))) {
        if (J.t(((v) => J.t(v) ? (!J.eq(J.attr(J.attr(_loop8, "previtem"), "role"), "tool")) : v)(J.attr(_loop8, "previtem")))) {
          out.push(J.str("<|im_start|>user\n"));
        }
        out.push(J.str("<tool_response>\n"), J.str(J.attr(message_16, "content")), J.str("\n</tool_response>\n"));
        if (J.t(((v) => J.t(v) ? (!J.eq(J.attr(J.attr(_loop8, "nextitem"), "role"), "tool")) : v)(!J.t(J.attr(_loop8, "last"))))) {
          out.push(J.str("<|im_end|>\n"));
        } else if (J.t(J.attr(_loop8, "last"))) {
          out.push(J.str("<|im_end|>\n"));
        }
      } else {
        out.push(J.str(J.add(J.add(J.add(J.add("<|im_start|>", J.attr(message_16, "role")), "\n"), J.attr(message_16, "content")), "<|im_end|>\n")));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    if (J.t(J.attr(toggle_0, "enable"))) {
      out.push(J.str("<|im_start|>assistant\n<think>\n"));
    } else {
      out.push(J.str("<|im_start|>assistant\n<think></think>"));
    }
  }
  return out.join("");
}
