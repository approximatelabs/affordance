// Converted from the chat template of ByteDance-Seed/Seed-OSS-36B-Instruct by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let bos_token_0 = ctx["bos_token"];
  let eos_token_0 = ctx["eos_token"];
  let pad_token_0 = ctx["pad_token"];
  let toolcall_begin_token_0 = ctx["toolcall_begin_token"];
  let toolcall_end_token_0 = ctx["toolcall_end_token"];
  let think_begin_token_0 = ctx["think_begin_token"];
  let think_end_token_0 = ctx["think_end_token"];
  let budget_begin_token_0 = ctx["budget_begin_token"];
  let budget_end_token_0 = ctx["budget_end_token"];
  let thinking_budget_0 = ctx["thinking_budget"];
  let budget_reflections_v05_0 = ctx["budget_reflections_v05"];
  let ns_0 = ctx["ns"];
  let system_message_0 = ctx["system_message"];
  let loop_messages_0 = ctx["loop_messages"];
  let tools_0 = ctx["tools"] ?? null;
  let py_type_0 = ctx["py_type"];
  let messages_0 = ctx["messages"];
  let use_json_tooldef_0 = ctx["use_json_tooldef"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  bos_token_0 = "<seed:bos>";
  eos_token_0 = "<seed:eos>";
  pad_token_0 = "<seed:pad>";
  toolcall_begin_token_0 = "<seed:tool_call>";
  toolcall_end_token_0 = "</seed:tool_call>";
  think_begin_token_0 = "<seed:think>";
  think_end_token_0 = "</seed:think>";
  budget_begin_token_0 = "<seed:cot_budget_reflect>";
  budget_end_token_0 = "</seed:cot_budget_reflect>";
  if (J.t(!J.t((thinking_budget_0 !== undefined)))) {
    thinking_budget_0 = J.neg(1);
  }
  budget_reflections_v05_0 = J.pairs([[0, 0], [512, 128], [1024, 256], [2048, 512], [4096, 512], [8192, 1024], [16384, 1024]]);
  ns_0 = J.call(J.namespace, J.kw({"interval": null}));
  {
    let _items1 = J.iter(J.filter("dictsort", budget_reflections_v05_0));
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let [k_1, v_2] = J.unpack(_it1, 2);
      if (J.t(((v) => J.t(v) ? (J.le(thinking_budget_0, k_1)) : v)((J.attr(ns_0, "interval") === null)))) {
        J.setattr(ns_0, "interval", v_2);
      }
    }
  }
  if (J.t((J.attr(ns_0, "interval") === null))) {
    J.setattr(ns_0, "interval", J.item(budget_reflections_v05_0, 16384));
  }
  if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
    system_message_0 = J.item(J.item(messages_0, 0), "content");
    loop_messages_0 = J.slice(messages_0, 1, undefined, undefined);
  } else {
    loop_messages_0 = messages_0;
  }
  if (J.t(((v) => J.t(v) ? v : (tools_0 === null))(!J.t((tools_0 !== undefined))))) {
    tools_0 = [];
  }
  py_type_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_4 = k.caller;
    let t_3 = p.length > 0 ? p[0] : "t" in k ? k["t"] : undefined;
    if (J.t((J.eq(t_3, "string")))) {
      out.push("str");
    } else if (J.t((J.has(t_3, ["number", "integer"])))) {
      out.push("int");
    } else if (J.t((J.eq(t_3, "boolean")))) {
      out.push("bool");
    } else if (J.t((J.eq(t_3, "array")))) {
      out.push("list");
    } else {
      out.push("Any");
    }
    return out.join("");
  };
  if (J.t((system_message_0 !== undefined))) {
    out.push(J.str(J.add(J.add(bos_token_0, "system\n"), system_message_0)));
  } else {
    if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", tools_0), 0)) : v)(J.test("iterable", tools_0)))) {
      out.push(J.str(J.add(bos_token_0, "system\nYou are Doubao, a helpful AI assistant. You may call one or more functions to assist with the user query.")));
    }
  }
  if (J.t(((v) => J.t(v) ? use_json_tooldef_0 : v)((use_json_tooldef_0 !== undefined)))) {
    out.push("\n", J.str("Tool List:\nYou are authorized to use the following tools (described in JSON Schema format). Before performing any task, you must decide how to call them based on the descriptions and parameters of these tools."), "\n", J.str(J.filter("tojson", tools_0, J.kw({"ensure_ascii": false}))));
  } else {
    {
      let _items2 = J.iter(tools_0);
      _items2 = _items2.filter(_it2 => { let item_5 = _it2; return J.t((J.eq(J.attr(item_5, "type"), "function"))); });
      const _loop2 = J.loop(_items2);
      for (let _i2 = 0; _i2 < _items2.length; _i2++) {
        _loop2.index0 = _i2; const _it2 = _items2[_i2]; let item_5 = _it2;
        out.push("\n\nFunction:\ndef ", J.str(J.attr(J.attr(item_5, "function"), "name")), "(");
        {
          let _items3 = J.iter(J.call(J.attr(J.attr(J.attr(J.attr(item_5, "function"), "parameters"), "properties"), "items")));
          const _loop3 = J.loop(_items3);
          for (let _i3 = 0; _i3 < _items3.length; _i3++) {
            _loop3.index0 = _i3; const _it3 = _items3[_i3]; let [name_6, spec_7] = J.unpack(_it3, 2);
            out.push(J.str(name_6), ": ", J.str(J.call(py_type_0, J.attr(spec_7, "type"))));
            if (J.t(!J.t(J.attr(_loop3, "last")))) {
              out.push(",");
            }
          }
        }
        out.push("):\n    \"\"\"\n    ", J.str(J.filter("trim", J.attr(J.attr(item_5, "function"), "description"))), "\n\n");
        if (J.t(J.attr(J.attr(J.attr(item_5, "function"), "parameters"), "properties"))) {
          out.push("    Args:");
          {
            let _items4 = J.iter(J.call(J.attr(J.attr(J.attr(J.attr(item_5, "function"), "parameters"), "properties"), "items")));
            const _loop4 = J.loop(_items4);
            for (let _i4 = 0; _i4 < _items4.length; _i4++) {
              _loop4.index0 = _i4; const _it4 = _items4[_i4]; let [name_8, spec_9] = J.unpack(_it4, 2);
              out.push("\n    - ", J.str(name_8), " (", J.str(J.call(py_type_0, J.attr(spec_9, "type"))), ")");
              if (J.t((J.has(name_8, J.attr(J.attr(J.attr(item_5, "function"), "parameters"), "required"))))) {
                out.push(" [\u5fc5\u586b]");
              } else {
                out.push(" [\u9009\u586b]");
              }
              out.push(":", J.str(J.concat(" ", ((v) => J.t(v) ? v : "")(J.attr(spec_9, "description")))));
            }
          }
        }
        out.push("\n");
        if (J.t(((v) => J.t(v) ? J.attr(J.attr(J.attr(item_5, "function"), "returns"), "properties") : v)(((v) => J.t(v) ? (J.attr(J.attr(J.attr(item_5, "function"), "returns"), "properties") !== undefined) : v)((J.attr(J.attr(item_5, "function"), "returns") !== undefined))))) {
          out.push("    Returns:");
          {
            let _items5 = J.iter(J.call(J.attr(J.attr(J.attr(J.attr(item_5, "function"), "returns"), "properties"), "items")));
            const _loop5 = J.loop(_items5);
            for (let _i5 = 0; _i5 < _items5.length; _i5++) {
              _loop5.index0 = _i5; const _it5 = _items5[_i5]; let [name_10, spec_11] = J.unpack(_it5, 2);
              out.push("\n    - ", J.str(name_10), " (", J.str(J.call(py_type_0, J.attr(spec_11, "type"))), "):", J.str(J.concat(" ", ((v) => J.t(v) ? v : "")(J.attr(spec_11, "description")))));
            }
          }
        }
        out.push("\n    \"\"\"");
      }
    }
  }
  if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", tools_0), 0)) : v)(J.test("iterable", tools_0)))) {
    out.push("\n", J.str("\u5de5\u5177\u8c03\u7528\u8bf7\u9075\u5faa\u5982\u4e0b\u683c\u5f0f:\n<seed:tool_call>\n<function=example_function_name>\n<parameter=example_parameter_1>value_1</parameter>\n<parameter=example_parameter_2>This is the value for the second parameter\nthat can span\nmultiple lines</parameter>\n</function>\n</seed:tool_call>\n"));
  }
  if (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? (J.gt(J.filter("length", tools_0), 0)) : v)(J.test("iterable", tools_0)))((system_message_0 !== undefined)))) {
    out.push(J.str(eos_token_0));
  }
  if (J.t((thinking_budget_0 !== undefined))) {
    if (J.t((J.eq(thinking_budget_0, 0)))) {
      out.push(J.str(J.add(bos_token_0, "system")), "\n", J.str("You are an intelligent assistant that can answer questions in one step without the need for reasoning and thinking, that is, your thinking budget is 0. Next, please skip the thinking process and directly start answering the user's questions."), "\n", J.str(eos_token_0));
    } else if (J.t(!J.t((J.eq(thinking_budget_0, J.neg(1)))))) {
      out.push(J.str(J.add(bos_token_0, "system")), "\n", J.str("You are an intelligent assistant with reflective ability. In the process of thinking and reasoning, you need to strictly follow the thinking budget, which is "), J.str(thinking_budget_0), J.str(". That is, you need to complete your thinking within "), J.str(thinking_budget_0), J.str(" tokens and start answering the user's questions. You will reflect on your thinking process every "), J.str(J.attr(ns_0, "interval")), J.str(" tokens, stating how many tokens have been used and how many are left."), "\n", J.str(eos_token_0));
    }
  }
  {
    let _items6 = J.iter(loop_messages_0);
    const _loop6 = J.loop(_items6);
    for (let _i6 = 0; _i6 < _items6.length; _i6++) {
      _loop6.index0 = _i6; const _it6 = _items6[_i6]; let message_12 = _it6;
      if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", J.attr(message_12, "tool_calls")), 0)) : v)(((v) => J.t(v) ? J.test("iterable", J.attr(message_12, "tool_calls")) : v)(((v) => J.t(v) ? (J.attr(message_12, "tool_calls") !== undefined) : v)((J.eq(J.attr(message_12, "role"), "assistant"))))))) {
        out.push(J.str(J.add(bos_token_0, J.attr(message_12, "role"))));
        if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", J.filter("trim", J.attr(message_12, "reasoning_content"))), 0)) : v)(((v) => J.t(v) ? J.test("string", J.attr(message_12, "reasoning_content")) : v)((J.attr(message_12, "reasoning_content") !== undefined))))) {
          out.push(J.str(J.add(J.add(J.add("\n", think_begin_token_0), J.filter("trim", J.attr(message_12, "reasoning_content"))), think_end_token_0)));
        }
        if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", J.filter("trim", J.attr(message_12, "content"))), 0)) : v)(((v) => J.t(v) ? J.test("string", J.attr(message_12, "content")) : v)((J.attr(message_12, "content") !== undefined))))) {
          out.push(J.str(J.add(J.add("\n", J.filter("trim", J.attr(message_12, "content"))), "\n")));
        }
        {
          let _items7 = J.iter(J.attr(message_12, "tool_calls"));
          const _loop7 = J.loop(_items7);
          for (let _i7 = 0; _i7 < _items7.length; _i7++) {
            _loop7.index0 = _i7; const _it7 = _items7[_i7]; let tool_call_13 = _it7;
            if (J.t((J.attr(tool_call_13, "function") !== undefined))) {
              tool_call_13 = J.attr(tool_call_13, "function");
            }
            out.push(J.str(J.add(J.add(J.add(J.add("\n", toolcall_begin_token_0), "\n<function="), J.attr(tool_call_13, "name")), ">\n")));
            if (J.t((J.attr(tool_call_13, "arguments") !== undefined))) {
              {
                let _items8 = J.iter(J.filter("items", J.attr(tool_call_13, "arguments")));
                const _loop8 = J.loop(_items8);
                for (let _i8 = 0; _i8 < _items8.length; _i8++) {
                  _loop8.index0 = _i8; const _it8 = _items8[_i8]; let [arg_name_14, arg_value_15] = J.unpack(_it8, 2);
                  out.push(J.str(J.add(J.add("<parameter=", arg_name_14), ">")));
                  arg_value_15 = (J.t(J.test("string", arg_value_15)) ? arg_value_15 : J.filter("string", arg_value_15));
                  out.push(J.str(J.add(arg_value_15, "</parameter>\n")));
                }
              }
            }
            out.push(J.str(J.add("</function>\n", toolcall_end_token_0)));
          }
        }
        out.push(J.str(eos_token_0));
      } else if (J.t((J.has(J.attr(message_12, "role"), ["user", "system"])))) {
        out.push(J.str(J.add(J.add(J.add(J.add(bos_token_0, J.attr(message_12, "role")), "\n"), J.attr(message_12, "content")), eos_token_0)));
      } else if (J.t((J.eq(J.attr(message_12, "role"), "assistant")))) {
        out.push(J.str(J.add(bos_token_0, J.attr(message_12, "role"))));
        if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", J.filter("trim", J.attr(message_12, "reasoning_content"))), 0)) : v)(((v) => J.t(v) ? J.test("string", J.attr(message_12, "reasoning_content")) : v)((J.attr(message_12, "reasoning_content") !== undefined))))) {
          out.push(J.str(J.add(J.add(J.add("\n", think_begin_token_0), J.filter("trim", J.attr(message_12, "reasoning_content"))), think_end_token_0)));
        }
        if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", J.filter("trim", J.attr(message_12, "content"))), 0)) : v)(((v) => J.t(v) ? J.test("string", J.attr(message_12, "content")) : v)((J.attr(message_12, "content") !== undefined))))) {
          out.push(J.str(J.add(J.add("\n", J.filter("trim", J.attr(message_12, "content"))), eos_token_0)));
        }
      } else {
        out.push(J.str(J.add(J.add(J.add(J.add(bos_token_0, J.attr(message_12, "role")), "\n"), J.attr(message_12, "content")), eos_token_0)));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str(J.add(bos_token_0, "assistant\n")));
    if (J.t((J.eq(thinking_budget_0, 0)))) {
      out.push(J.str(J.add(J.add(J.add(J.add(J.add(J.add(think_begin_token_0, "\n"), budget_begin_token_0), "The current thinking budget is 0, so I will directly start answering the question."), budget_end_token_0), "\n"), think_end_token_0)));
    }
  }
  return out.join("");
}
