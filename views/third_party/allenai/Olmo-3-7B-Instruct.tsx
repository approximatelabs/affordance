// Converted from the chat template of allenai/Olmo-3-7B-Instruct by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let has_system_0 = ctx["has_system"];
  let messages_0 = ctx["messages"];
  let tools_0 = ctx["tools"] ?? null;
  let args_0 = ctx["args"];
  let ns_0 = ctx["ns"];
  let arguments_0 = ctx["arguments"];
  let eos_token_0 = ctx["eos_token"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  has_system_0 = (J.gt(J.filter("length", J.filter("list", J.filter("selectattr", messages_0, "role", "equalto", "system"))), 0));
  if (J.t(!J.t(has_system_0))) {
    out.push(J.str("<|im_start|>system\nYou are a helpful function-calling AI assistant. "));
    if (J.t((tools_0 === null))) {
      out.push(J.str("You do not currently have access to any functions. <functions></functions><|im_end|>\n"));
    } else {
      out.push(J.str("You are provided with function signatures within <functions></functions> XML tags. You may call one or more functions to assist with the user query. Output any function calls within <function_calls></function_calls> XML tags. Do not make assumptions about what values to plug into functions."), J.str("<functions>"), J.str(J.filter("tojson", tools_0)), J.str("</functions><|im_end|>\n"));
    }
  }
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      if (J.t((J.eq(J.item(message_1, "role"), "system")))) {
        out.push(J.str(J.add("<|im_start|>system\n", J.item(message_1, "content"))));
        if (J.t(!J.t((tools_0 === null)))) {
          out.push(J.str("<functions>"), J.str(J.filter("tojson", tools_0)), J.str("</functions>"));
        } else if (J.t(!J.t((J.call(J.attr(message_1, "get"), "functions", null) === null)))) {
          out.push(J.str(J.add(J.add(" <functions>", J.item(message_1, "functions")), "</functions>")));
        }
        out.push(J.str("<|im_end|>\n"));
      } else if (J.t((J.eq(J.item(message_1, "role"), "user")))) {
        out.push(J.str(J.add(J.add("<|im_start|>user\n", J.item(message_1, "content")), "<|im_end|>\n")));
      } else if (J.t((J.eq(J.item(message_1, "role"), "assistant")))) {
        out.push(J.str("<|im_start|>assistant\n"));
        if (J.t(!J.t((J.call(J.attr(message_1, "get"), "content", null) === null)))) {
          out.push(J.str(J.item(message_1, "content")));
        }
        if (J.t(!J.t((J.call(J.attr(message_1, "get"), "function_calls", null) === null)))) {
          out.push(J.str(J.add(J.add("<function_calls>", J.item(message_1, "function_calls")), "</function_calls>")));
        } else if (J.t(!J.t((J.call(J.attr(message_1, "get"), "tool_calls", null) === null)))) {
          out.push(J.str("<function_calls>"));
          {
            let _items2 = J.iter(J.item(message_1, "tool_calls"));
            const _loop2 = J.loop(_items2);
            for (let _i2 = 0; _i2 < _items2.length; _i2++) {
              _loop2.index0 = _i2; const _it2 = _items2[_i2]; let tool_call_2 = _it2;
              let args_3 = args_0;
              let ns_4 = ns_0;
              let arguments_5 = arguments_0;
              if (J.t(((v) => J.t(v) ? !J.t((J.call(J.attr(tool_call_2, "get"), "function", null) === null)) : v)(J.test("mapping", tool_call_2)))) {
                args_3 = J.item(J.item(tool_call_2, "function"), "arguments");
                ns_4 = J.call(J.namespace, J.kw({"arguments_list": []}));
                {
                  let _items3 = J.iter(J.call(J.attr(args_3, "items")));
                  const _loop3 = J.loop(_items3);
                  for (let _i3 = 0; _i3 < _items3.length; _i3++) {
                    _loop3.index0 = _i3; const _it3 = _items3[_i3]; let [key_6, value_7] = J.unpack(_it3, 2);
                    J.setattr(ns_4, "arguments_list", J.add(J.attr(ns_4, "arguments_list"), [J.concat(key_6, "=", J.filter("tojson", value_7))]));
                  }
                }
                arguments_5 = J.filter("join", J.attr(ns_4, "arguments_list"), ", ");
                out.push(J.str(J.add(J.add(J.add(J.item(J.item(tool_call_2, "function"), "name"), "("), arguments_5), ")")));
                if (J.t(!J.t(J.attr(_loop2, "last")))) {
                  out.push(J.str("\n"));
                }
              } else {
                out.push(J.str(tool_call_2));
              }
            }
          }
          out.push(J.str("</function_calls>"));
        }
        if (J.t(!J.t(J.attr(_loop1, "last")))) {
          out.push(J.str(J.add("<|im_end|>", "\n")));
        } else {
          out.push(J.str(eos_token_0));
        }
      } else if (J.t((J.eq(J.item(message_1, "role"), "environment")))) {
        out.push(J.str(J.add(J.add("<|im_start|>environment\n", J.item(message_1, "content")), "<|im_end|>\n")));
      } else if (J.t((J.eq(J.item(message_1, "role"), "tool")))) {
        out.push(J.str(J.add(J.add("<|im_start|>environment\n", J.item(message_1, "content")), "<|im_end|>\n")));
      }
      if (J.t(((v) => J.t(v) ? add_generation_prompt_0 : v)(J.attr(_loop1, "last")))) {
        out.push(J.str("<|im_start|>assistant\n"));
      }
    }
  }
  return out.join("");
}
