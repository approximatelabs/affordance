// Converted from the chat template of Qwen/Qwen3-Coder-Next by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let render_extra_keys_0 = ctx["render_extra_keys"];
  let system_message_0 = ctx["system_message"];
  let loop_messages_0 = ctx["loop_messages"];
  let tools_0 = ctx["tools"] ?? null;
  let messages_0 = ctx["messages"];
  let handled_keys_0 = ctx["handled_keys"];
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
          if (J.t(J.test("string", J.item(json_dict_1, json_key_4)))) {
            out.push(J.str(J.concat("\n<", json_key_4, ">", J.filter("string", J.item(json_dict_1, json_key_4)), "</", json_key_4, ">")));
          } else {
            out.push(J.str(J.concat("\n<", json_key_4, ">", J.filter("safe", J.filter("tojson", J.item(json_dict_1, json_key_4))), "</", json_key_4, ">")));
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
  if (J.t(!J.t((tools_0 !== undefined)))) {
    tools_0 = [];
  }
  if (J.t((system_message_0 !== undefined))) {
    out.push(J.str(J.add("<|im_start|>system\n", system_message_0)));
  } else {
    if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", tools_0), 0)) : v)(J.test("iterable", tools_0)))) {
      out.push(J.str("<|im_start|>system\nYou are Qwen, a helpful AI assistant that can interact with a computer to solve tasks."));
    }
  }
  if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", tools_0), 0)) : v)(J.test("iterable", tools_0)))) {
    out.push(J.str("\n\n# Tools\n\nYou have access to the following functions:\n\n"), J.str("<tools>"));
    {
      let _items2 = J.iter(tools_0);
      const _loop2 = J.loop(_items2);
      for (let _i2 = 0; _i2 < _items2.length; _i2++) {
        _loop2.index0 = _i2; const _it2 = _items2[_i2]; let tool_5 = _it2;
        let handled_keys_6 = handled_keys_0;
        if (J.t((J.attr(tool_5, "function") !== undefined))) {
          tool_5 = J.attr(tool_5, "function");
        }
        out.push(J.str(J.concat("\n<function>\n<name>", J.attr(tool_5, "name"), "</name>")));
        if (J.t((J.attr(tool_5, "description") !== undefined))) {
          out.push(J.str(J.concat("\n<description>", J.filter("trim", J.attr(tool_5, "description")), "</description>")));
        }
        out.push(J.str("\n<parameters>"));
        if (J.t(((v) => J.t(v) ? J.test("mapping", J.attr(J.attr(tool_5, "parameters"), "properties")) : v)(((v) => J.t(v) ? (J.attr(J.attr(tool_5, "parameters"), "properties") !== undefined) : v)(((v) => J.t(v) ? J.test("mapping", J.attr(tool_5, "parameters")) : v)((J.attr(tool_5, "parameters") !== undefined)))))) {
          {
            let _items3 = J.iter(J.filter("items", J.attr(J.attr(tool_5, "parameters"), "properties")));
            const _loop3 = J.loop(_items3);
            for (let _i3 = 0; _i3 < _items3.length; _i3++) {
              _loop3.index0 = _i3; const _it3 = _items3[_i3]; let [param_name_7, param_fields_8] = J.unpack(_it3, 2);
              let handled_keys_9 = handled_keys_6;
              out.push(J.str("\n<parameter>"), J.str(J.concat("\n<name>", param_name_7, "</name>")));
              if (J.t((J.attr(param_fields_8, "type") !== undefined))) {
                out.push(J.str(J.concat("\n<type>", J.filter("string", J.attr(param_fields_8, "type")), "</type>")));
              }
              if (J.t((J.attr(param_fields_8, "description") !== undefined))) {
                out.push(J.str(J.concat("\n<description>", J.filter("trim", J.attr(param_fields_8, "description")), "</description>")));
              }
              handled_keys_9 = ["name", "type", "description"];
              out.push(J.str(J.call(render_extra_keys_0, param_fields_8, handled_keys_9)), J.str("\n</parameter>"));
            }
          }
        }
        handled_keys_6 = ["type", "properties"];
        out.push(J.str(J.call(render_extra_keys_0, J.attr(tool_5, "parameters"), handled_keys_6)), J.str("\n</parameters>"));
        handled_keys_6 = ["type", "name", "description", "parameters"];
        out.push(J.str(J.call(render_extra_keys_0, tool_5, handled_keys_6)), J.str("\n</function>"));
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
    let _items4 = J.iter(loop_messages_0);
    const _loop4 = J.loop(_items4);
    for (let _i4 = 0; _i4 < _items4.length; _i4++) {
      _loop4.index0 = _i4; const _it4 = _items4[_i4]; let message_10 = _it4;
      if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", J.attr(message_10, "tool_calls")), 0)) : v)(((v) => J.t(v) ? J.test("iterable", J.attr(message_10, "tool_calls")) : v)(((v) => J.t(v) ? (J.attr(message_10, "tool_calls") !== undefined) : v)((J.eq(J.attr(message_10, "role"), "assistant"))))))) {
        out.push(J.str(J.add("<|im_start|>", J.attr(message_10, "role"))));
        if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", J.filter("trim", J.attr(message_10, "content"))), 0)) : v)(((v) => J.t(v) ? J.test("string", J.attr(message_10, "content")) : v)((J.attr(message_10, "content") !== undefined))))) {
          out.push(J.str(J.add(J.add("\n", J.filter("trim", J.attr(message_10, "content"))), "\n")));
        }
        {
          let _items5 = J.iter(J.attr(message_10, "tool_calls"));
          const _loop5 = J.loop(_items5);
          for (let _i5 = 0; _i5 < _items5.length; _i5++) {
            _loop5.index0 = _i5; const _it5 = _items5[_i5]; let tool_call_11 = _it5;
            if (J.t((J.attr(tool_call_11, "function") !== undefined))) {
              tool_call_11 = J.attr(tool_call_11, "function");
            }
            out.push(J.str(J.add(J.add("\n<tool_call>\n<function=", J.attr(tool_call_11, "name")), ">\n")));
            if (J.t((J.attr(tool_call_11, "arguments") !== undefined))) {
              {
                let _items6 = J.iter(J.filter("items", J.attr(tool_call_11, "arguments")));
                const _loop6 = J.loop(_items6);
                for (let _i6 = 0; _i6 < _items6.length; _i6++) {
                  _loop6.index0 = _i6; const _it6 = _items6[_i6]; let [args_name_12, args_value_13] = J.unpack(_it6, 2);
                  out.push(J.str(J.add(J.add("<parameter=", args_name_12), ">\n")));
                  args_value_13 = (J.t(J.test("string", args_value_13)) ? args_value_13 : J.filter("safe", J.filter("tojson", args_value_13)));
                  out.push(J.str(args_value_13), J.str("\n</parameter>\n"));
                }
              }
            }
            out.push(J.str("</function>\n</tool_call>"));
          }
        }
        out.push(J.str("<|im_end|>\n"));
      } else if (J.t(((v) => J.t(v) ? v : (J.eq(J.attr(message_10, "role"), "assistant")))(((v) => J.t(v) ? v : (J.eq(J.attr(message_10, "role"), "system")))((J.eq(J.attr(message_10, "role"), "user")))))) {
        out.push(J.str(J.add(J.add(J.add(J.add(J.add("<|im_start|>", J.attr(message_10, "role")), "\n"), J.attr(message_10, "content")), "<|im_end|>"), "\n")));
      } else if (J.t((J.eq(J.attr(message_10, "role"), "tool")))) {
        if (J.t(((v) => J.t(v) ? (!J.eq(J.attr(J.attr(_loop4, "previtem"), "role"), "tool")) : v)(J.attr(_loop4, "previtem")))) {
          out.push(J.str("<|im_start|>user"));
        }
        out.push(J.str("\n<tool_response>\n"), J.str(J.attr(message_10, "content")), J.str("\n</tool_response>"));
        if (J.t(((v) => J.t(v) ? (!J.eq(J.attr(J.attr(_loop4, "nextitem"), "role"), "tool")) : v)(!J.t(J.attr(_loop4, "last"))))) {
          out.push(J.str("<|im_end|>\n"));
        } else if (J.t(J.attr(_loop4, "last"))) {
          out.push(J.str("<|im_end|>\n"));
        }
      } else {
        out.push(J.str(J.add(J.add(J.add(J.add("<|im_start|>", J.attr(message_10, "role")), "\n"), J.attr(message_10, "content")), "<|im_end|>\n")));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|im_start|>assistant\n"));
  }
  return out.join("");
}
