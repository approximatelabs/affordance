// Converted from the chat template of upstage/Solar-Open2-250B by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let provider_system_prompt_0 = ctx["provider_system_prompt"];
  let reasoning_effort_0 = ctx["reasoning_effort"];
  let think_render_option_0 = ctx["think_render_option"];
  let sys_ns_0 = ctx["sys_ns"];
  let last_user_idx_0 = ctx["last_user_idx"];
  let render_system_message_0 = ctx["render_system_message"];
  let render_tool_instruction_0 = ctx["render_tool_instruction"];
  let render_json_response_format_instruction_0 = ctx["render_json_response_format_instruction"];
  let render_tool_arguments_0 = ctx["render_tool_arguments"];
  let ns_0 = ctx["ns"];
  let messages_0 = ctx["messages"];
  let response_format_0 = ctx["response_format"];
  let tools_0 = ctx["tools"] ?? null;
  let prev_is_tool_0 = ctx["prev_is_tool"];
  let start_idx_0 = ctx["start_idx"];
  let tool_ns_0 = ctx["tool_ns"];
  let prev_msg_0 = ctx["prev_msg"];
  let sep_ns_0 = ctx["sep_ns"];
  let reasoning_0 = ctx["reasoning"];
  add_generation_prompt_0 = (J.t((add_generation_prompt_0 !== undefined)) ? add_generation_prompt_0 : true);
  provider_system_prompt_0 = (J.t(provider_system_prompt_0) ? provider_system_prompt_0 : false);
  reasoning_effort_0 = (J.t((reasoning_effort_0 !== undefined)) ? reasoning_effort_0 : "high");
  think_render_option_0 = (J.t((think_render_option_0 !== undefined)) ? think_render_option_0 : "interleaved");
  sys_ns_0 = J.call(J.namespace, J.kw({"is_first_block": true}));
  last_user_idx_0 = J.call(J.namespace, J.kw({"value": J.neg(1)}));
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      if (J.t((J.eq(J.attr(message_1, "role"), "user")))) {
        J.setattr(last_user_idx_0, "value", J.attr(_loop1, "index0"));
      }
    }
  }
  render_system_message_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_3 = k.caller;
    let user_system_messages_2 = p.length > 0 ? p[0] : "user_system_messages" in k ? k["user_system_messages"] : undefined;
    if (J.t(provider_system_prompt_0)) {
      if (J.t(!J.t(J.attr(sys_ns_0, "is_first_block")))) {
        out.push(J.str("\n\n"));
      }
      J.setattr(sys_ns_0, "is_first_block", false);
      out.push(J.str(J.add(J.add("## Provider System Prompt\n\nYou are Solar Open2 250B, a large language model trained by Upstage AI, a Korean startup. Your knowledge cutoff is 2026-02. The current date is ", J.call(J.strftime_now, "%Y-%m-%d")), ".")));
    }
    if (J.t(user_system_messages_2)) {
      if (J.t(!J.t(J.attr(sys_ns_0, "is_first_block")))) {
        out.push(J.str("\n\n"));
      }
      J.setattr(sys_ns_0, "is_first_block", false);
      out.push(J.str("## System Prompt"));
      {
        let _items2 = J.iter(user_system_messages_2);
        const _loop2 = J.loop(_items2);
        for (let _i2 = 0; _i2 < _items2.length; _i2++) {
          _loop2.index0 = _i2; const _it2 = _items2[_i2]; let system_message_4 = _it2;
          out.push(J.str("\n\n"), J.str(system_message_4));
        }
      }
    }
    return out.join("");
  };
  render_tool_instruction_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_7 = k.caller;
    let tools_5 = p.length > 0 ? p[0] : "tools" in k ? k["tools"] : undefined;
    let is_last_block_6 = p.length > 1 ? p[1] : "is_last_block" in k ? k["is_last_block"] : undefined;
    if (J.t(!J.t(J.attr(sys_ns_0, "is_first_block")))) {
      out.push(J.str("\n\n"));
    }
    J.setattr(sys_ns_0, "is_first_block", false);
    out.push(J.str("## Tools\n- You may invoke one or more tools to assist with the user's query."), J.str("\n\n### Available Tools\n"));
    {
      let _items3 = J.iter(tools_5);
      const _loop3 = J.loop(_items3);
      for (let _i3 = 0; _i3 < _items3.length; _i3++) {
        _loop3.index0 = _i3; const _it3 = _items3[_i3]; let tool_8 = _it3;
        out.push(J.str("<|tool:start|>"), J.str(J.filter("tojson", J.attr(tool_8, "function"))), J.str("<|tool:end|>\n"));
      }
    }
    out.push(J.str("\n### Tool Call Instruction\n"), J.str("- If using a tool, any reasoning must strictly precede the call. Do not append any text after the tool call.\n"), J.str("- If no tool is required, answer directly from your knowledge without ever mentioning the availability or absence of tools.\n"), J.str("- Each tool call MUST use this following format: <|tool_call:start|>{example-tool-name}\n<|tool_arg:start|>{example-key-name-1}<|tool_arg:value|>{example-value-1}<|tool_arg:end|>\n<|tool_arg:start|>{example-key-name-2}<|tool_arg:value|>{example-value-2}<|tool_arg:end|>\n<|tool_call:end|>\n"));
    return out.join("");
  };
  render_json_response_format_instruction_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_10 = k.caller;
    let response_format_9 = p.length > 0 ? p[0] : "response_format" in k ? k["response_format"] : undefined;
    if (J.t(!J.t(J.attr(sys_ns_0, "is_first_block")))) {
      out.push(J.str("\n\n"));
    }
    J.setattr(sys_ns_0, "is_first_block", false);
    out.push(J.str("## Output Format Constraint"));
    if (J.t((J.eq(J.attr(response_format_9, "type"), "json_schema")))) {
      out.push(J.str("\n\n- Your final response should follow the JSON schema:\n```json\n"), J.str(J.filter("tojson", J.attr(J.attr(response_format_9, "json_schema"), "schema"))), J.str("\n```\n- Please ensure your answers adhere to this format and do not contain any unnecessary text."));
    } else if (J.t((J.eq(J.attr(response_format_9, "type"), "json_object")))) {
      out.push(J.str("\n\n- Your final response must be a valid JSON object."), J.str("\n- Do not include any text outside the JSON object."));
    }
    return out.join("");
  };
  render_tool_arguments_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_12 = k.caller;
    let tool_arguments_11 = p.length > 0 ? p[0] : "tool_arguments" in k ? k["tool_arguments"] : undefined;
    {
      let _items4 = J.iter(J.filter("items", tool_arguments_11));
      const _loop4 = J.loop(_items4);
      for (let _i4 = 0; _i4 < _items4.length; _i4++) {
        _loop4.index0 = _i4; const _it4 = _items4[_i4]; let [args_name_13, args_value_14] = J.unpack(_it4, 2);
        out.push(J.str(J.add(J.add("<|tool_arg:start|>", args_name_13), "<|tool_arg:value|>")));
        args_value_14 = (J.t(J.test("string", args_value_14)) ? args_value_14 : J.filter("safe", J.filter("tojson", args_value_14)));
        out.push(J.str(args_value_14), J.str("<|tool_arg:end|>\n"));
      }
    }
    return out.join("");
  };
  ns_0 = J.call(J.namespace, J.kw({"system_messages": []}));
  {
    let _items5 = J.iter(messages_0);
    const _loop5 = J.loop(_items5);
    for (let _i5 = 0; _i5 < _items5.length; _i5++) {
      _loop5.index0 = _i5; const _it5 = _items5[_i5]; let message_15 = _it5;
      if (J.t((J.eq(J.attr(message_15, "role"), "system")))) {
        J.setattr(ns_0, "system_messages", J.add(J.attr(ns_0, "system_messages"), [J.attr(message_15, "content")]));
      }
    }
  }
  if (J.t(((v) => J.t(v) ? v : response_format_0)(((v) => J.t(v) ? v : tools_0)(((v) => J.t(v) ? v : provider_system_prompt_0)(J.attr(ns_0, "system_messages")))))) {
    out.push(J.str("<|im:start|>system<|im:content|>"), J.str(J.call(render_system_message_0, J.attr(ns_0, "system_messages"))));
    if (J.t(tools_0)) {
      out.push(J.str(J.call(render_tool_instruction_0, tools_0, !J.t(response_format_0))));
    }
    if (J.t(response_format_0)) {
      out.push(J.str(J.call(render_json_response_format_instruction_0, response_format_0)));
    }
    out.push(J.str("<|im:end|>\n"));
  }
  {
    let _items6 = J.iter(messages_0);
    const _loop6 = J.loop(_items6);
    for (let _i6 = 0; _i6 < _items6.length; _i6++) {
      _loop6.index0 = _i6; const _it6 = _items6[_i6]; let message_16 = _it6;
      let prev_is_tool_17 = prev_is_tool_0;
      let start_idx_18 = start_idx_0;
      let tool_ns_19 = tool_ns_0;
      let prev_msg_20 = prev_msg_0;
      let sep_ns_21 = sep_ns_0;
      let reasoning_22 = reasoning_0;
      if (J.t((J.eq(J.attr(message_16, "role"), "user")))) {
        out.push(J.str(J.add(J.add("<|im:start|>user<|im:content|>", J.attr(message_16, "content")), "<|im:end|>\n")));
      } else if (J.t((J.eq(J.attr(message_16, "role"), "tool")))) {
        if (J.t(!J.t(J.attr(message_16, "tool_call_id")))) {
          out.push(J.str(J.call(J.raise_exception, "tool message is missing required 'tool_call_id' field")));
        }
        prev_is_tool_17 = ((v) => J.t(v) ? (J.eq(J.attr(J.item(messages_0, J.sub(J.attr(_loop6, "index0"), 1)), "role"), "tool")) : v)((J.gt(J.attr(_loop6, "index0"), 0)));
        if (J.t(!J.t(prev_is_tool_17))) {
          start_idx_18 = J.attr(_loop6, "index0");
          tool_ns_19 = J.call(J.namespace, J.kw({"items": [], "scanning": true}));
          {
            let _items7 = J.iter(J.call(J.range, start_idx_18, J.filter("length", messages_0)));
            const _loop7 = J.loop(_items7);
            for (let _i7 = 0; _i7 < _items7.length; _i7++) {
              _loop7.index0 = _i7; const _it7 = _items7[_i7]; let j_23 = _it7;
              if (J.t(J.attr(tool_ns_19, "scanning"))) {
                if (J.t((J.eq(J.attr(J.item(messages_0, j_23), "role"), "tool")))) {
                  J.setattr(tool_ns_19, "items", J.add(J.attr(tool_ns_19, "items"), [J.item(messages_0, j_23)]));
                } else {
                  J.setattr(tool_ns_19, "scanning", false);
                }
              }
            }
          }
          out.push(J.str("<|im:start|>tool<|im:content|>"));
          prev_msg_20 = (J.t((J.gt(start_idx_18, 0))) ? J.item(messages_0, J.sub(start_idx_18, 1)) : null);
          if (J.t(((v) => J.t(v) ? J.attr(prev_msg_20, "tool_calls") : v)(((v) => J.t(v) ? (J.eq(J.attr(prev_msg_20, "role"), "assistant")) : v)(prev_msg_20)))) {
            sep_ns_21 = J.call(J.namespace, J.kw({"first": true, "rendered_ids": []}));
            {
              let _items8 = J.iter(J.attr(prev_msg_20, "tool_calls"));
              const _loop8 = J.loop(_items8);
              for (let _i8 = 0; _i8 < _items8.length; _i8++) {
                _loop8.index0 = _i8; const _it8 = _items8[_i8]; let tool_call_24 = _it8;
                {
                  let _items9 = J.iter(J.attr(tool_ns_19, "items"));
                  const _loop9 = J.loop(_items9);
                  for (let _i9 = 0; _i9 < _items9.length; _i9++) {
                    _loop9.index0 = _i9; const _it9 = _items9[_i9]; let tool_msg_25 = _it9;
                    if (J.t((J.eq(J.attr(tool_msg_25, "tool_call_id"), J.attr(tool_call_24, "id"))))) {
                      if (J.t(!J.t(J.attr(sep_ns_21, "first")))) {
                        out.push(J.str("\n"));
                      }
                      J.setattr(sep_ns_21, "first", false);
                      J.setattr(sep_ns_21, "rendered_ids", J.add(J.attr(sep_ns_21, "rendered_ids"), [J.attr(tool_msg_25, "tool_call_id")]));
                      out.push(J.str(J.add(J.add("<|tool_response:start|>", J.attr(tool_msg_25, "content")), "<|tool_response:end|>")));
                    }
                  }
                }
              }
            }
            {
              let _items10 = J.iter(J.attr(tool_ns_19, "items"));
              const _loop10 = J.loop(_items10);
              for (let _i10 = 0; _i10 < _items10.length; _i10++) {
                _loop10.index0 = _i10; const _it10 = _items10[_i10]; let tool_msg_26 = _it10;
                if (J.t((!J.has(J.attr(tool_msg_26, "tool_call_id"), J.attr(sep_ns_21, "rendered_ids"))))) {
                  if (J.t(!J.t(J.attr(sep_ns_21, "first")))) {
                    out.push(J.str("\n"));
                  }
                  J.setattr(sep_ns_21, "first", false);
                  out.push(J.str(J.add(J.add("<|tool_response:start|>", J.attr(tool_msg_26, "content")), "<|tool_response:end|>")));
                }
              }
            }
          } else {
            {
              let _items11 = J.iter(J.attr(tool_ns_19, "items"));
              const _loop11 = J.loop(_items11);
              for (let _i11 = 0; _i11 < _items11.length; _i11++) {
                _loop11.index0 = _i11; const _it11 = _items11[_i11]; let tool_msg_27 = _it11;
                if (J.t(!J.t(J.attr(_loop11, "first")))) {
                  out.push(J.str("\n"));
                }
                out.push(J.str(J.add(J.add("<|tool_response:start|>", J.attr(tool_msg_27, "content")), "<|tool_response:end|>")));
              }
            }
          }
          out.push(J.str("\n<|im:end|>\n"));
        }
      } else if (J.t((J.eq(J.attr(message_16, "role"), "assistant")))) {
        out.push(J.str("<|im:start|>assistant<|im:content|>"));
        if (J.t((J.attr(message_16, "reasoning") !== undefined))) {
          reasoning_22 = J.attr(message_16, "reasoning");
        } else if (J.t((J.attr(message_16, "reasoning_content") !== undefined))) {
          reasoning_22 = J.attr(message_16, "reasoning_content");
        } else {
          reasoning_22 = null;
        }
        if (J.t((J.eq(think_render_option_0, "preserved")))) {
          if (J.t(reasoning_22)) {
            out.push(J.str(J.add(J.add("<|think:start|>", reasoning_22), "<|think:end|>")));
          } else {
            out.push(J.str("<|think:start|><|think:end|>"));
          }
        } else if (J.t((J.eq(think_render_option_0, "interleaved")))) {
          if (J.t(((v) => J.t(v) ? (J.gt(J.attr(_loop6, "index0"), J.attr(last_user_idx_0, "value"))) : v)(reasoning_22))) {
            out.push(J.str(J.add(J.add("<|think:start|>", reasoning_22), "<|think:end|>")));
          } else {
            out.push(J.str("<|think:start|><|think:end|>"));
          }
        }
        if (J.t(J.attr(message_16, "content"))) {
          out.push(J.str(J.attr(message_16, "content")));
        }
        if (J.t(J.attr(message_16, "tool_calls"))) {
          {
            let _items12 = J.iter(J.attr(message_16, "tool_calls"));
            const _loop12 = J.loop(_items12);
            for (let _i12 = 0; _i12 < _items12.length; _i12++) {
              _loop12.index0 = _i12; const _it12 = _items12[_i12]; let tool_call_28 = _it12;
              if (J.t(!J.t(J.attr(tool_call_28, "id")))) {
                out.push(J.str(J.call(J.raise_exception, "assistant tool_call is missing required 'id' field")));
              }
              if (J.t(!J.t(J.attr(_loop12, "first")))) {
                out.push(J.str("\n"));
              }
              out.push(J.str(J.add(J.add("<|tool_call:start|>", J.attr(J.attr(tool_call_28, "function"), "name")), "\n")));
              if (J.t((J.attr(J.attr(tool_call_28, "function"), "arguments") !== undefined))) {
                out.push(J.str(J.call(render_tool_arguments_0, J.attr(J.attr(tool_call_28, "function"), "arguments"))));
              }
              out.push(J.str("<|tool_call:end|>"));
            }
          }
          out.push(J.str("\n"));
        }
        out.push(J.str("<|im:end|>\n"));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    if (J.t((J.has(reasoning_effort_0, ["medium", "high", "xhigh"])))) {
      out.push(J.str("<|im:start|>assistant<|im:content|><|think:start|>"));
    } else {
      out.push(J.str("<|im:start|>assistant<|im:content|><|think:start|><|think:end|>"));
    }
  }
  return out.join("");
}
