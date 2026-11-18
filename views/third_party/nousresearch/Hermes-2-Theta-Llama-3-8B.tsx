// Converted from the chat template of NousResearch/Hermes-2-Theta-Llama-3-8B by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

function DefaultTemplate(ctx) {
  const out = [];
  let bos_token_0 = ctx["bos_token"];
  let messages_0 = ctx["messages"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  out.push(J.str(bos_token_0));
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      out.push(J.str(J.add(J.add(J.add(J.add(J.add("<|im_start|>", J.item(message_1, "role")), "\n"), J.item(message_1, "content")), "<|im_end|>"), "\n")));
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|im_start|>assistant\n"));
  }
  return out.join("");
}

function ToolUseTemplate(ctx) {
  const out = [];
  let json_to_python_type_0 = ctx["json_to_python_type"];
  let basic_type_map_0 = ctx["basic_type_map"];
  let bos_token_0 = ctx["bos_token"];
  let tools_0 = ctx["tools"] ?? null;
  let messages_0 = ctx["messages"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  json_to_python_type_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_2 = k.caller;
    let json_spec_1 = p.length > 0 ? p[0] : "json_spec" in k ? k["json_spec"] : undefined;
    let basic_type_map_3 = basic_type_map_0;
    basic_type_map_3 = ({"string": "str", "number": "float", "integer": "int", "boolean": "bool"});
    if (J.t((J.item(basic_type_map_3, J.attr(json_spec_1, "type")) !== undefined))) {
      out.push(J.str(J.item(basic_type_map_3, J.attr(json_spec_1, "type"))));
    } else if (J.t((J.eq(J.attr(json_spec_1, "type"), "array")))) {
      out.push(J.str(J.add(J.add("list[", J.call(json_to_python_type_0, J.filter("items", json_spec_1))), "]")));
    } else if (J.t((J.eq(J.attr(json_spec_1, "type"), "object")))) {
      if (J.t((J.attr(json_spec_1, "additionalProperties") !== undefined))) {
        out.push(J.str(J.add(J.add("dict[str, ", J.call(json_to_python_type_0, J.attr(json_spec_1, "additionalProperties"))), "]")));
      } else {
        out.push(J.str("dict"));
      }
    } else if (J.t(J.test("iterable", J.attr(json_spec_1, "type")))) {
      out.push(J.str("Union["));
      {
        let _items1 = J.iter(J.attr(json_spec_1, "type"));
        const _loop1 = J.loop(_items1);
        for (let _i1 = 0; _i1 < _items1.length; _i1++) {
          _loop1.index0 = _i1; const _it1 = _items1[_i1]; let t_4 = _it1;
          out.push(J.str(J.call(json_to_python_type_0, ({"type": t_4}))));
          if (J.t(!J.t(J.attr(_loop1, "last")))) {
            out.push(J.str(","));
          }
        }
      }
      out.push(J.str("]"));
    } else {
      out.push(J.str("Any"));
    }
    return out.join("");
  };
  out.push(J.str(bos_token_0), J.str("<|im_start|>system\n"), J.str("You are a function calling AI model. You are provided with function signatures within <tools></tools> XML tags. You may call one or more functions to assist with the user query. Don't make assumptions about what values to plug into functions. Here are the available tools: <tools> "));
  {
    let _items2 = J.iter(tools_0);
    const _loop2 = J.loop(_items2);
    for (let _i2 = 0; _i2 < _items2.length; _i2++) {
      _loop2.index0 = _i2; const _it2 = _items2[_i2]; let tool_5 = _it2;
      if (J.t((J.attr(tool_5, "function") !== undefined))) {
        tool_5 = J.attr(tool_5, "function");
      }
      out.push(J.str("{\"type\": \"function\", \"function\": "), J.str(J.add(J.add("{\"name\": \"", J.attr(tool_5, "name")), "\", ")), J.str(J.add(J.add("\"description\": \"", J.attr(tool_5, "name")), "(")));
      {
        let _items3 = J.iter(J.filter("items", J.attr(J.attr(tool_5, "parameters"), "properties")));
        const _loop3 = J.loop(_items3);
        for (let _i3 = 0; _i3 < _items3.length; _i3++) {
          _loop3.index0 = _i3; const _it3 = _items3[_i3]; let [param_name_6, param_fields_7] = J.unpack(_it3, 2);
          out.push(J.str(J.add(J.add(param_name_6, ": "), J.call(json_to_python_type_0, param_fields_7))));
          if (J.t(!J.t(J.attr(_loop3, "last")))) {
            out.push(J.str(", "));
          }
        }
      }
      out.push(J.str(")"));
      if (J.t((J.attr(tool_5, "return") !== undefined))) {
        out.push(J.str(J.add(" -> ", J.call(json_to_python_type_0, J.attr(tool_5, "return")))));
      }
      out.push(J.str(J.add(J.add(" - ", J.attr(tool_5, "description")), "\n\n")));
      {
        let _items4 = J.iter(J.filter("items", J.attr(J.attr(tool_5, "parameters"), "properties")));
        const _loop4 = J.loop(_items4);
        for (let _i4 = 0; _i4 < _items4.length; _i4++) {
          _loop4.index0 = _i4; const _it4 = _items4[_i4]; let [param_name_8, param_fields_9] = J.unpack(_it4, 2);
          if (J.t(J.attr(_loop4, "first"))) {
            out.push(J.str("    Args:\n"));
          }
          out.push(J.str(J.add(J.add(J.add(J.add(J.add("        ", param_name_8), "("), J.call(json_to_python_type_0, param_fields_9)), "): "), J.filter("trim", J.attr(param_fields_9, "description")))));
        }
      }
      if (J.t(((v) => J.t(v) ? (J.attr(J.attr(tool_5, "return"), "description") !== undefined) : v)((J.attr(tool_5, "return") !== undefined)))) {
        out.push(J.str(J.add("\n    Returns:\n        ", J.attr(J.attr(tool_5, "return"), "description"))));
      }
      out.push(J.str("\""), J.str(", \"parameters\": "));
      if (J.t((J.eq(J.filter("length", J.attr(J.attr(tool_5, "parameters"), "properties")), 0)))) {
        out.push(J.str("{}"));
      } else {
        out.push(J.str(J.filter("tojson", J.attr(tool_5, "parameters"))));
      }
      out.push(J.str("}"));
      if (J.t(!J.t(J.attr(_loop2, "last")))) {
        out.push(J.str("\n"));
      }
    }
  }
  out.push(J.str(" </tools>"), J.str("Use the following pydantic model json schema for each tool call you will make: {\"properties\": {\"name\": {\"title\": \"Name\", \"type\": \"string\"}, \"arguments\": {\"title\": \"Arguments\", \"type\": \"object\"}}, \"required\": [\"name\", \"arguments\"], \"title\": \"FunctionCall\", \"type\": \"object\"}}\n"), J.str("For each function call return a json object with function name and arguments within <tool_call></tool_call> XML tags as follows:\n"), J.str("<tool_call>\n"), J.str("{\"name\": <function-name>, \"arguments\": <args-dict>}\n"), J.str("</tool_call><|im_end|>\n"));
  {
    let _items5 = J.iter(messages_0);
    const _loop5 = J.loop(_items5);
    for (let _i5 = 0; _i5 < _items5.length; _i5++) {
      _loop5.index0 = _i5; const _it5 = _items5[_i5]; let message_10 = _it5;
      if (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? !J.t((J.attr(message_10, "tool_calls") !== undefined)) : v)((J.eq(J.attr(message_10, "role"), "assistant"))))(((v) => J.t(v) ? v : (J.eq(J.attr(message_10, "role"), "system")))((J.eq(J.attr(message_10, "role"), "user")))))) {
        out.push(J.str(J.add(J.add(J.add(J.add(J.add("<|im_start|>", J.attr(message_10, "role")), "\n"), J.attr(message_10, "content")), "<|im_end|>"), "\n")));
      } else if (J.t((J.eq(J.attr(message_10, "role"), "assistant")))) {
        out.push(J.str(J.add("<|im_start|>", J.attr(message_10, "role"))));
        {
          let _items6 = J.iter(J.attr(message_10, "tool_calls"));
          const _loop6 = J.loop(_items6);
          for (let _i6 = 0; _i6 < _items6.length; _i6++) {
            _loop6.index0 = _i6; const _it6 = _items6[_i6]; let tool_call_11 = _it6;
            out.push(J.str("\n<tool_call>\n"));
            if (J.t((J.attr(tool_call_11, "function") !== undefined))) {
              tool_call_11 = J.attr(tool_call_11, "function");
            }
            out.push(J.str("{"), J.str("\"name\": \""), J.str(J.attr(tool_call_11, "name")), J.str("\""), J.str(", "));
            if (J.t((J.attr(tool_call_11, "arguments") !== undefined))) {
              out.push(J.str("\"arguments\": "));
              if (J.t(J.test("string", J.attr(tool_call_11, "arguments")))) {
                out.push(J.str(J.attr(tool_call_11, "arguments")));
              } else {
                out.push(J.str(J.filter("tojson", J.attr(tool_call_11, "arguments"))));
              }
            }
            out.push(J.str("}"), J.str("\n</tool_call>"));
          }
        }
        out.push(J.str("<|im_end|>\n"));
      } else if (J.t((J.eq(J.attr(message_10, "role"), "tool")))) {
        if (J.t(((v) => J.t(v) ? (!J.eq(J.attr(J.attr(_loop5, "previtem"), "role"), "tool")) : v)(J.attr(_loop5, "previtem")))) {
          out.push(J.str("<|im_start|>tool\n"));
        }
        out.push(J.str("<tool_response>\n"), J.str(J.attr(message_10, "content")));
        if (J.t(!J.t(J.attr(_loop5, "last")))) {
          out.push(J.str("\n</tool_response>\n"));
        } else {
          out.push(J.str("\n</tool_response>"));
        }
        if (J.t(((v) => J.t(v) ? (!J.eq(J.attr(J.attr(_loop5, "nextitem"), "role"), "tool")) : v)(!J.t(J.attr(_loop5, "last"))))) {
          out.push(J.str("<|im_end|>"));
        } else if (J.t(J.attr(_loop5, "last"))) {
          out.push(J.str("<|im_end|>"));
        }
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|im_start|>assistant\n"));
  }
  return out.join("");
}

export default function Main(ctx) {
  return ctx.tools != null ? ToolUseTemplate(ctx) : DefaultTemplate(ctx);
}
