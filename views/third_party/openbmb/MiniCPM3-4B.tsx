// Converted from the chat template of openbmb/MiniCPM3-4B by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let json_to_python_type_0 = ctx["json_to_python_type"];
  let object_to_fields_0 = ctx["object_to_fields"];
  let tool_parser_0 = ctx["tool_parser"];
  let loop_messages_0 = ctx["loop_messages"];
  let system_message_0 = ctx["system_message"];
  let basic_type_map_0 = ctx["basic_type_map"];
  let o_ns_0 = ctx["o_ns"];
  let param_default_0 = ctx["param_default"];
  let tool_params_0 = ctx["tool_params"];
  let messages_0 = ctx["messages"];
  let tools_0 = ctx["tools"] ?? null;
  let content_0 = ctx["content"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  json_to_python_type_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_3 = k.caller;
    let param_name_1 = p.length > 0 ? p[0] : "param_name" in k ? k["param_name"] : undefined;
    let json_spec_2 = p.length > 1 ? p[1] : "json_spec" in k ? k["json_spec"] : undefined;
    let basic_type_map_4 = basic_type_map_0;
    basic_type_map_4 = ({"string": "str", "number": "float", "integer": "int", "boolean": "bool", "null": "None"});
    if (J.t(J.attr(json_spec_2, "enum"))) {
      out.push(J.str(J.filter("title", param_name_1)));
    } else if (J.t((J.item(basic_type_map_4, J.attr(json_spec_2, "type")) !== undefined))) {
      out.push(J.str(J.item(basic_type_map_4, J.attr(json_spec_2, "type"))));
    } else if (J.t((J.eq(J.attr(json_spec_2, "type"), "array")))) {
      out.push(J.str(J.add(J.add("List[", J.call(json_to_python_type_0, param_name_1, J.item(json_spec_2, "items"))), "]")));
    } else if (J.t((J.eq(J.attr(json_spec_2, "type"), "object")))) {
      out.push(J.str((J.t(!J.t(J.attr(json_spec_2, "properties"))) ? J.add(J.add("Dict[str, ", J.call(json_to_python_type_0, param_name_1, (J.t(J.attr(json_spec_2, "additionalProperties")) ? J.attr(json_spec_2, "additionalProperties") : "Any"))), "]") : J.filter("title", param_name_1))));
    } else if (J.t(J.test("iterable", J.attr(json_spec_2, "type")))) {
      out.push(J.str("Union["));
      {
        let _items1 = J.iter(J.attr(json_spec_2, "type"));
        const _loop1 = J.loop(_items1);
        for (let _i1 = 0; _i1 < _items1.length; _i1++) {
          _loop1.index0 = _i1; const _it1 = _items1[_i1]; let t_5 = _it1;
          out.push(J.str(J.call(json_to_python_type_0, param_name_1, ({"type": t_5}))), J.str((J.t(!J.t(J.attr(_loop1, "last"))) ? ", " : undefined)));
        }
      }
      out.push(J.str("]"));
    } else {
      out.push(J.str("Any"));
    }
    return out.join("");
  };
  object_to_fields_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_8 = k.caller;
    let json_spec_6 = p.length > 0 ? p[0] : "json_spec" in k ? k["json_spec"] : undefined;
    let field_indent_7 = p.length > 1 ? p[1] : "field_indent" in k ? k["field_indent"] : undefined;
    let o_ns_9 = o_ns_0;
    o_ns_9 = J.call(J.namespace, J.kw({"f": J.call(caller_8)}));
    {
      let _items2 = J.iter(J.filter("items", J.attr(json_spec_6, "properties")));
      const _loop2 = J.loop(_items2);
      for (let _i2 = 0; _i2 < _items2.length; _i2++) {
        _loop2.index0 = _i2; const _it2 = _items2[_i2]; let [param_name_10, param_fields_11] = J.unpack(_it2, 2);
        let param_default_12 = param_default_0;
        if (J.t(J.attr(param_fields_11, "enum"))) {
          out.push(J.str(J.add(J.add("\n\nclass ", J.filter("title", param_name_10)), "(Enum):\n")));
          {
            let _items3 = J.iter(J.attr(param_fields_11, "enum"));
            const _loop3 = J.loop(_items3);
            for (let _i3 = 0; _i3 < _items3.length; _i3++) {
              _loop3.index0 = _i3; const _it3 = _items3[_i3]; let enum_option_13 = _it3;
              out.push(J.str(J.add(J.add(J.add(J.add("    enum_", J.filter("string", J.attr(_loop3, "index0"))), " = "), J.filter("tojson", enum_option_13)), "\n")));
            }
          }
        } else if (J.t(((v) => J.t(v) ? J.attr(param_fields_11, "properties") : v)((J.eq(J.attr(param_fields_11, "type"), "object"))))) {
          {
            const caller = () => (() => {
            const out = [];
            out.push(J.str(J.add(J.add("\n\nclass ", J.filter("title", param_name_10)), "(BaseModel):\n")));
            return out.join("");
            })();
            out.push(J.str(J.call(object_to_fields_0, param_fields_11, "    ", J.kw({caller: caller}))));
          }
        } else if (J.t(((v) => J.t(v) ? J.attr(J.item(param_fields_11, "items"), "properties") : v)(((v) => J.t(v) ? (J.eq(J.attr(J.item(param_fields_11, "items"), "type"), "object")) : v)(((v) => J.t(v) ? J.item(param_fields_11, "items") : v)((J.eq(J.attr(param_fields_11, "type"), "array"))))))) {
          {
            const caller = () => (() => {
            const out = [];
            out.push(J.str(J.add(J.add("\n\nclass ", J.filter("title", param_name_10)), "(BaseModel):\n")));
            return out.join("");
            })();
            out.push(J.str(J.call(object_to_fields_0, J.item(param_fields_11, "items"), "    ", J.kw({caller: caller}))));
          }
        }
        param_default_12 = (J.t(J.test("string", J.attr(param_fields_11, "default"))) ? J.filter("tojson", J.attr(param_fields_11, "default")) : (J.t((J.attr(param_fields_11, "default") !== undefined)) ? J.filter("string", J.attr(param_fields_11, "default")) : "None"));
        J.setattr(o_ns_9, "f", J.add(J.add(J.add(J.attr(o_ns_9, "f"), field_indent_7), param_name_10), ": "));
        J.setattr(o_ns_9, "f", J.add(J.attr(o_ns_9, "f"), (J.t((!J.has(param_name_10, J.attr(json_spec_6, "required")))) ? J.add(J.add("Optional[", J.call(json_to_python_type_0, param_name_10, param_fields_11)), "]") : J.call(json_to_python_type_0, param_name_10, param_fields_11))));
        if (J.t(((v) => J.t(v) ? !J.t(J.attr(param_fields_11, "pattern")) : v)(((v) => J.t(v) ? !J.t(J.attr(param_fields_11, "description")) : v)(!J.t(J.attr(param_fields_11, "title")))))) {
          J.setattr(o_ns_9, "f", J.add(J.attr(o_ns_9, "f"), (J.t((!J.has(param_name_10, J.attr(json_spec_6, "required")))) ? J.add(" = ", param_default_12) : "")));
        } else {
          J.setattr(o_ns_9, "f", J.add(J.attr(o_ns_9, "f"), (J.t((J.has(param_name_10, J.attr(json_spec_6, "required")))) ? " = Field(..." : J.add(" = Field(", param_default_12))));
          J.setattr(o_ns_9, "f", J.add(J.attr(o_ns_9, "f"), (J.t(J.attr(param_fields_11, "description")) ? J.add(", description=", J.filter("tojson", J.attr(param_fields_11, "description"))) : "")));
          J.setattr(o_ns_9, "f", J.add(J.attr(o_ns_9, "f"), (J.t(J.attr(param_fields_11, "pattern")) ? J.add(", regex=", J.filter("tojson", J.attr(param_fields_11, "pattern"))) : "")));
          J.setattr(o_ns_9, "f", J.add(J.attr(o_ns_9, "f"), (J.t(J.attr(param_fields_11, "title")) ? J.add(", title=", J.filter("tojson", J.attr(param_fields_11, "title"))) : "")));
          J.setattr(o_ns_9, "f", J.add(J.attr(o_ns_9, "f"), ")"));
        }
        J.setattr(o_ns_9, "f", J.add(J.attr(o_ns_9, "f"), "\n"));
      }
    }
    out.push(J.str(J.attr(o_ns_9, "f")));
    return out.join("");
  };
  tool_parser_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_15 = k.caller;
    let tools_14 = p.length > 0 ? p[0] : "tools" in k ? k["tools"] : undefined;
    {
      let _items4 = J.iter(tools_14);
      const _loop4 = J.loop(_items4);
      for (let _i4 = 0; _i4 < _items4.length; _i4++) {
        _loop4.index0 = _i4; const _it4 = _items4[_i4]; let tool_16 = _it4;
        let tool_params_17 = tool_params_0;
        if (J.t(((v) => J.t(v) ? v : (J.eq(J.attr(tool_16, "type"), "function")))(!J.t((J.attr(tool_16, "type") !== undefined))))) {
          if (J.t((J.attr(tool_16, "function") !== undefined))) {
            tool_16 = J.attr(tool_16, "function");
          }
          tool_params_17 = (J.t((J.attr(tool_16, "parameters") !== undefined)) ? J.attr(tool_16, "parameters") : null);
          {
            const caller = () => (() => {
            const out = [];
            out.push(J.str(J.add(J.add("\n\ndef ", J.attr(tool_16, "name")), "(")));
            if (J.t(tool_params_17)) {
              {
                let _items5 = J.iter(J.filter("items", J.attr(tool_params_17, "properties")));
                const _loop5 = J.loop(_items5);
                for (let _i5 = 0; _i5 < _items5.length; _i5++) {
                  _loop5.index0 = _i5; const _it5 = _items5[_i5]; let [param_name_18, param_fields_19] = J.unpack(_it5, 2);
                  let param_default_20 = param_default_0;
                  param_default_20 = (J.t(J.test("string", J.attr(param_fields_19, "default"))) ? J.filter("tojson", J.attr(param_fields_19, "default")) : (J.t((J.attr(param_fields_19, "default") !== undefined)) ? J.filter("string", J.attr(param_fields_19, "default")) : "None"));
                  out.push(J.str((J.t((!J.eq(J.attr(_loop5, "index0"), 0))) ? ", " : undefined)), J.str(param_name_18), J.str((J.t((!J.has(param_name_18, J.attr(tool_params_17, "required")))) ? J.add("=", param_default_20) : undefined)));
                }
              }
            }
            out.push(J.str("):\n    \"\"\""), J.str(J.attr(tool_16, "description")), J.str((J.t(tool_params_17) ? "\n\n    Args:\n" : "\n")));
            return out.join("");
            })();
            out.push(J.str(J.call(object_to_fields_0, tool_params_17, "        ", J.kw({caller: caller}))));
          }
          out.push(J.str("    \"\"\"\n"));
        }
      }
    }
    return out.join("");
  };
  if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
    loop_messages_0 = J.slice(messages_0, 1, undefined, undefined);
    system_message_0 = J.item(J.item(messages_0, 0), "content");
  } else {
    loop_messages_0 = messages_0;
    system_message_0 = "";
  }
  out.push(J.str((J.t(((v) => J.t(v) ? v : tools_0)(system_message_0)) ? J.add("<|im_start|>system\n", system_message_0) : undefined)));
  if (J.t(tools_0)) {
    out.push(J.str("\n# Functions\nHere is a list of functions that you can invoke:\n```python\nfrom enum import Enum\nfrom typing import List, Dict, Optional\nfrom pydantic import BaseModel, Field\n\n"), J.str(J.call(tool_parser_0, tools_0)), J.str("\n```\n\n# Function Call Rule and Output Format\n- If the user's question can be answered without calling any function, please answer the user's question directly. In this situation, you should return your thought and answer the user's question directly.\n- If the user cannot be answered without calling any function, and the user does not provide enough information to call functions, please ask the user for more information. In this situation, you should return your thought and ask the user for more information.\n- If the user's question cannot be answered without calling any function, and the user has provided enough information to call functions to solve it, you should call the functions. In this situation, the assistant should return your thought and call the functions.\n- Use default parameters unless the user has specified otherwise.\n- You should answer in the following format:\n\n<|thought_start|>\n{explain why the user's question can be answered without calling a function or why you should ask the user for more information or why you should call one or more functions and your plan to solve the user's question.}\n<|thought_end|>\n<|tool_call_start|>\n```python\nfunc1(params_name=params_value, params_name2=params_value2...)\nfunc2(params)\n```\n<|tool_call_end|>\n{answer the user's question directly or ask the user for more information}"));
  }
  out.push(J.str((J.t(((v) => J.t(v) ? v : tools_0)(system_message_0)) ? "<|im_end|>\n" : undefined)));
  {
    let _items6 = J.iter(loop_messages_0);
    const _loop6 = J.loop(_items6);
    for (let _i6 = 0; _i6 < _items6.length; _i6++) {
      _loop6.index0 = _i6; const _it6 = _items6[_i6]; let message_21 = _it6;
      let content_22 = content_0;
      content_22 = J.attr(message_21, "content");
      if (J.t(((v) => J.t(v) ? J.attr(message_21, "tool_calls") : v)((J.eq(J.attr(message_21, "role"), "assistant"))))) {
        out.push(J.str(J.add(J.add("<|im_start|>", J.attr(message_21, "role")), "\n")), J.str((J.t(J.attr(message_21, "thought")) ? J.add(J.add("<|thought_start|>\n", J.attr(message_21, "thought")), "\n<|thought_end|>\n") : undefined)), J.str("<|tool_call_start|>\n```python\n"));
        {
          let _items7 = J.iter(J.attr(message_21, "tool_calls"));
          const _loop7 = J.loop(_items7);
          for (let _i7 = 0; _i7 < _items7.length; _i7++) {
            _loop7.index0 = _i7; const _it7 = _items7[_i7]; let tool_call_23 = _it7;
            if (J.t((J.attr(tool_call_23, "function") !== undefined))) {
              tool_call_23 = J.attr(tool_call_23, "function");
            }
            out.push(J.str(J.add(J.attr(tool_call_23, "name"), "(")));
            if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", J.attr(tool_call_23, "arguments")), 0)) : v)((J.attr(tool_call_23, "arguments") !== undefined)))) {
              {
                let _items8 = J.iter(J.filter("items", J.attr(tool_call_23, "arguments")));
                const _loop8 = J.loop(_items8);
                for (let _i8 = 0; _i8 < _items8.length; _i8++) {
                  _loop8.index0 = _i8; const _it8 = _items8[_i8]; let [param_name_24, param_value_25] = J.unpack(_it8, 2);
                  out.push(J.str(J.add(J.add(param_name_24, "="), J.filter("tojson", param_value_25))), J.str((J.t(!J.t(J.attr(_loop8, "last"))) ? "," : undefined)));
                }
              }
            }
            out.push(J.str(")\n"));
          }
        }
        out.push(J.str("```\n<|tool_call_end|>\n"), J.str((J.t(((v) => J.t(v) ? !J.t(J.call(J.attr(content_22, "startswith"), "<|tool_call_start|>")) : v)(content_22)) ? content_22 : undefined)), J.str("<|im_end|>\n"));
      } else if (J.t(((v) => J.t(v) ? J.attr(message_21, "thought") : v)((J.eq(J.attr(message_21, "role"), "assistant"))))) {
        out.push(J.str(J.add(J.add(J.add(J.add(J.add(J.add(J.add("<|im_start|>", J.attr(message_21, "role")), "\n"), "<|thought_start|>\n"), J.attr(message_21, "thought")), "\n<|thought_end|>\n"), content_22), "<|im_end|>\n")));
      } else {
        out.push(J.str(J.add(J.add(J.add(J.add("<|im_start|>", J.attr(message_21, "role")), "\n"), content_22), "<|im_end|>\n")));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|im_start|>assistant\n"));
  }
  return out.join("");
}
