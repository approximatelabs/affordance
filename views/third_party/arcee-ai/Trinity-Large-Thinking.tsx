// Converted from the chat template of arcee-ai/Trinity-Large-Thinking by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let render_extra_keys_0 = ctx["render_extra_keys"];
  let render_tool_call_0 = ctx["render_tool_call"];
  let system_message_0 = ctx["system_message"];
  let loop_messages_0 = ctx["loop_messages"];
  let tools_0 = ctx["tools"] ?? null;
  let has_tools_0 = ctx["has_tools"];
  let tool_call_0 = ctx["tool_call"];
  let args_value_0 = ctx["args_value"];
  let messages_0 = ctx["messages"];
  let handled_keys_0 = ctx["handled_keys"];
  let param_fields_0 = ctx["param_fields"];
  let role_0 = ctx["role"];
  let content_str_0 = ctx["content_str"];
  let trimmed_content_0 = ctx["trimmed_content"];
  let has_reasoning_content_0 = ctx["has_reasoning_content"];
  let has_reasoning_0 = ctx["has_reasoning"];
  let reasoning_value_0 = ctx["reasoning_value"];
  let has_tool_calls_0 = ctx["has_tool_calls"];
  let separator_0 = ctx["separator"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  out.push("<|begin_of_text|>");
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
  render_tool_call_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_6 = k.caller;
    let raw_tool_call_5 = p.length > 0 ? p[0] : "raw_tool_call" in k ? k["raw_tool_call"] : undefined;
    let tool_call_7 = tool_call_0;
    if (J.t(((v) => J.t(v) ? J.test("mapping", J.attr(raw_tool_call_5, "function")) : v)((J.attr(raw_tool_call_5, "function") !== undefined)))) {
      tool_call_7 = J.attr(raw_tool_call_5, "function");
    } else {
      tool_call_7 = raw_tool_call_5;
    }
    out.push(J.str(J.add(J.add("<tool_call>\n<function=", J.filter("string", J.filter("default", J.attr(tool_call_7, "name"), ""))), ">\n")));
    if (J.t(((v) => J.t(v) ? J.test("mapping", J.attr(tool_call_7, "arguments")) : v)((J.attr(tool_call_7, "arguments") !== undefined)))) {
      {
        let _items2 = J.iter(J.attr(tool_call_7, "arguments"));
        const _loop2 = J.loop(_items2);
        for (let _i2 = 0; _i2 < _items2.length; _i2++) {
          _loop2.index0 = _i2; const _it2 = _items2[_i2]; let args_name_8 = _it2;
          let args_value_9 = args_value_0;
          args_value_9 = J.item(J.attr(tool_call_7, "arguments"), args_name_8);
          out.push(J.str(J.add(J.add("<parameter=", J.filter("string", args_name_8)), ">\n")));
          if (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? !J.t(J.test("string", args_value_9)) : v)(J.test("sequence", args_value_9)))(J.test("mapping", args_value_9)))) {
            out.push(J.str(J.filter("safe", J.filter("tojson", args_value_9))));
          } else {
            out.push(J.str(J.filter("string", args_value_9)));
          }
          out.push(J.str("\n</parameter>\n"));
        }
      }
    }
    out.push(J.str("</function>\n</tool_call>"));
    return out.join("");
  };
  system_message_0 = null;
  if (J.t(((v) => J.t(v) ? (J.eq(J.item(J.item(messages_0, 0), "role"), "system")) : v)(messages_0))) {
    system_message_0 = J.item(J.item(messages_0, 0), "content");
    loop_messages_0 = J.slice(messages_0, 1, undefined, undefined);
  } else {
    loop_messages_0 = messages_0;
  }
  if (J.t(!J.t((tools_0 !== undefined)))) {
    tools_0 = [];
  }
  has_tools_0 = ((v) => J.t(v) ? (J.gt(J.filter("length", tools_0), 0)) : v)(((v) => J.t(v) ? !J.t(J.test("string", tools_0)) : v)(J.test("iterable", tools_0)));
  if (J.t(((v) => J.t(v) ? v : has_tools_0)(!J.t((system_message_0 === null))))) {
    out.push(J.str("<|im_start|>system\n"));
    if (J.t(!J.t((system_message_0 === null)))) {
      out.push(J.str(system_message_0));
    } else {
      out.push(J.str("You are Trinity Large, a helpful assistant developed by Arcee AI, that can interact with a computer to solve tasks."));
    }
    if (J.t(has_tools_0)) {
      out.push(J.str("\n\n# Tools\n\nYou have access to the following functions:\n\n<tools>"));
      {
        let _items3 = J.iter(tools_0);
        const _loop3 = J.loop(_items3);
        for (let _i3 = 0; _i3 < _items3.length; _i3++) {
          _loop3.index0 = _i3; const _it3 = _items3[_i3]; let tool_10 = _it3;
          let handled_keys_11 = handled_keys_0;
          if (J.t(((v) => J.t(v) ? J.test("mapping", J.attr(tool_10, "function")) : v)((J.attr(tool_10, "function") !== undefined)))) {
            tool_10 = J.attr(tool_10, "function");
          }
          out.push(J.str(J.concat("\n<function>\n<name>", J.filter("string", J.filter("default", J.attr(tool_10, "name"), "")), "</name>")));
          if (J.t(((v) => J.t(v) ? !J.t((J.attr(tool_10, "description") === null)) : v)((J.attr(tool_10, "description") !== undefined)))) {
            out.push(J.str(J.concat("\n<description>", J.filter("trim", J.filter("string", J.attr(tool_10, "description"))), "</description>")));
          }
          out.push(J.str("\n<parameters>"));
          if (J.t(((v) => J.t(v) ? J.test("mapping", J.item(J.attr(tool_10, "parameters"), "properties")) : v)(((v) => J.t(v) ? (J.has("properties", J.attr(tool_10, "parameters"))) : v)(((v) => J.t(v) ? J.test("mapping", J.attr(tool_10, "parameters")) : v)((J.attr(tool_10, "parameters") !== undefined)))))) {
            {
              let _items4 = J.iter(J.item(J.attr(tool_10, "parameters"), "properties"));
              const _loop4 = J.loop(_items4);
              for (let _i4 = 0; _i4 < _items4.length; _i4++) {
                _loop4.index0 = _i4; const _it4 = _items4[_i4]; let param_name_12 = _it4;
                let param_fields_13 = param_fields_0;
                let handled_keys_14 = handled_keys_11;
                param_fields_13 = J.item(J.item(J.attr(tool_10, "parameters"), "properties"), param_name_12);
                out.push(J.str(J.concat("\n<parameter>\n<name>", J.filter("string", param_name_12), "</name>")));
                if (J.t(((v) => J.t(v) ? !J.t((J.attr(param_fields_13, "type") === null)) : v)(((v) => J.t(v) ? (J.attr(param_fields_13, "type") !== undefined) : v)(J.test("mapping", param_fields_13))))) {
                  out.push(J.str(J.concat("\n<type>", J.filter("string", J.attr(param_fields_13, "type")), "</type>")));
                }
                if (J.t(((v) => J.t(v) ? !J.t((J.attr(param_fields_13, "description") === null)) : v)(((v) => J.t(v) ? (J.attr(param_fields_13, "description") !== undefined) : v)(J.test("mapping", param_fields_13))))) {
                  out.push(J.str(J.concat("\n<description>", J.filter("trim", J.filter("string", J.attr(param_fields_13, "description"))), "</description>")));
                }
                if (J.t(J.test("mapping", param_fields_13))) {
                  handled_keys_14 = ["name", "type", "description"];
                  out.push(J.str(J.call(render_extra_keys_0, param_fields_13, handled_keys_14)));
                }
                out.push(J.str("\n</parameter>"));
              }
            }
          }
          if (J.t((J.attr(tool_10, "parameters") !== undefined))) {
            handled_keys_11 = ["type", "properties"];
            out.push(J.str(J.call(render_extra_keys_0, J.attr(tool_10, "parameters"), handled_keys_11)));
          }
          out.push(J.str("\n</parameters>"));
          handled_keys_11 = ["type", "name", "description", "parameters"];
          out.push(J.str(J.call(render_extra_keys_0, tool_10, handled_keys_11)), J.str("\n</function>"));
        }
      }
      out.push(J.str("\n</tools>"), J.str("\n\nIf you choose to call a function ONLY reply in the following format with NO suffix:\n\n<tool_call>\n<function=example_function_name>\n<parameter=example_parameter_1>\nvalue_1\n</parameter>\n<parameter=example_parameter_2>\nThis is the value for the second parameter\nthat can span\nmultiple lines\n</parameter>\n</function>\n</tool_call>\n\n<IMPORTANT>\nReminder:\n- Function calls MUST follow the specified format: an inner <function=...></function> block must be nested within <tool_call></tool_call> XML tags\n- Required parameters MUST be specified\n- You may provide optional reasoning for your function call in natural language BEFORE the function call, but NOT after\n- If there is no function call available, answer the question like normal with your current knowledge and do not tell the user about function calls\n</IMPORTANT>"));
    }
    out.push(J.str("<|im_end|>\n"));
  }
  {
    let _items5 = J.iter(loop_messages_0);
    const _loop5 = J.loop(_items5);
    for (let _i5 = 0; _i5 < _items5.length; _i5++) {
      _loop5.index0 = _i5; const _it5 = _items5[_i5]; let message_15 = _it5;
      let role_16 = role_0;
      let content_str_17 = content_str_0;
      let trimmed_content_18 = trimmed_content_0;
      let has_reasoning_content_19 = has_reasoning_content_0;
      let has_reasoning_20 = has_reasoning_0;
      let reasoning_value_21 = reasoning_value_0;
      let has_tool_calls_22 = has_tool_calls_0;
      role_16 = J.filter("default", J.attr(message_15, "role"), "");
      if (J.t((J.eq(role_16, "assistant")))) {
        content_str_17 = (J.t((J.attr(message_15, "content") === null)) ? "" : J.filter("string", J.attr(message_15, "content")));
        trimmed_content_18 = J.filter("trim", content_str_17);
        has_reasoning_content_19 = (J.attr(message_15, "reasoning_content") !== undefined);
        has_reasoning_20 = ((v) => J.t(v) ? v : (J.attr(message_15, "reasoning") !== undefined))(has_reasoning_content_19);
        if (J.t(has_reasoning_content_19)) {
          reasoning_value_21 = J.attr(message_15, "reasoning_content");
        } else if (J.t((J.attr(message_15, "reasoning") !== undefined))) {
          reasoning_value_21 = J.attr(message_15, "reasoning");
        } else {
          reasoning_value_21 = null;
        }
        has_tool_calls_22 = ((v) => J.t(v) ? (J.gt(J.filter("length", J.attr(message_15, "tool_calls")), 0)) : v)(((v) => J.t(v) ? !J.t(J.test("string", J.attr(message_15, "tool_calls"))) : v)(((v) => J.t(v) ? J.test("iterable", J.attr(message_15, "tool_calls")) : v)((J.attr(message_15, "tool_calls") !== undefined))));
        out.push(J.str("<|im_start|>assistant\n"));
        if (J.t(has_reasoning_20)) {
          if (J.t(reasoning_value_21)) {
            out.push(J.str(J.add(J.add("<think>", J.filter("trim", J.filter("string", reasoning_value_21))), "</think>")));
          } else {
            out.push(J.str("<think></think>"));
          }
          if (J.t(trimmed_content_18)) {
            out.push(J.str(J.add("\n", trimmed_content_18)));
          }
        } else if (J.t(has_tool_calls_22)) {
          if (J.t(trimmed_content_18)) {
            out.push(J.str(trimmed_content_18));
          }
        } else {
          out.push(J.str(content_str_17));
        }
        if (J.t(has_tool_calls_22)) {
          {
            let _items6 = J.iter(J.attr(message_15, "tool_calls"));
            const _loop6 = J.loop(_items6);
            for (let _i6 = 0; _i6 < _items6.length; _i6++) {
              _loop6.index0 = _i6; const _it6 = _items6[_i6]; let tool_call_23 = _it6;
              let separator_24 = separator_0;
              separator_24 = (J.t(((v) => J.t(v) ? v : !J.t(J.attr(_loop6, "first")))(((v) => J.t(v) ? ((v) => J.t(v) ? v : trimmed_content_18)(has_reasoning_20) : v)(J.attr(_loop6, "first")))) ? "\n" : "");
              out.push(J.str(J.add(separator_24, J.call(render_tool_call_0, tool_call_23))));
            }
          }
        }
        out.push(J.str("<|im_end|>\n"));
      } else if (J.t(((v) => J.t(v) ? v : (J.eq(role_16, "function")))(((v) => J.t(v) ? v : (J.eq(role_16, "observation")))((J.eq(role_16, "tool")))))) {
        if (J.t(((v) => J.t(v) ? v : (!J.has(J.attr(J.attr(_loop5, "previtem"), "role"), ["tool", "observation", "function"])))(J.attr(_loop5, "first")))) {
          out.push(J.str("<|im_start|>user\n"));
        }
        out.push(J.str("<tool_response>\n"), J.str((J.t((J.attr(message_15, "content") === null)) ? "" : J.filter("string", J.attr(message_15, "content")))), J.str("\n</tool_response>\n"));
        if (J.t(((v) => J.t(v) ? v : (!J.has(J.attr(J.attr(_loop5, "nextitem"), "role"), ["tool", "observation", "function"])))(J.attr(_loop5, "last")))) {
          out.push(J.str("<|im_end|>\n"));
        }
      } else {
        out.push(J.str(J.add("<|im_start|>", J.filter("string", role_16))), J.str(J.add("\n", (J.t((J.attr(message_15, "content") === null)) ? "" : J.filter("string", J.attr(message_15, "content"))))), J.str("<|im_end|>\n"));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|im_start|>assistant\n<think>"));
  }
  return out.join("");
}
