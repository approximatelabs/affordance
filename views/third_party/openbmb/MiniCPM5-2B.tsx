// Converted from the chat template of openbmb/MiniCPM5-2B by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let tool_definitions_0 = ctx["tool_definitions"];
  let ns_0 = ctx["ns"];
  let bos_token_0 = ctx["bos_token"];
  let tools_0 = ctx["tools"] ?? null;
  let messages_0 = ctx["messages"];
  let index_0 = ctx["index"];
  let content_0 = ctx["content"];
  let reasoning_content_0 = ctx["reasoning_content"];
  let args_dict_0 = ctx["args_dict"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let enable_thinking_0 = ctx["enable_thinking"];
  out.push(J.str(bos_token_0));
  if (J.t(tools_0)) {
    {
      const captured = (() => {
      const out = [];
      out.push(J.str("# Tools\n\nYou are provided with function signatures within <tools></tools> XML tags:\n<tools>"));
      {
        let _items1 = J.iter(tools_0);
        const _loop1 = J.loop(_items1);
        for (let _i1 = 0; _i1 < _items1.length; _i1++) {
          _loop1.index0 = _i1; const _it1 = _items1[_i1]; let tool_1 = _it1;
          out.push(J.str("\n"), J.str(J.filter("tojson", tool_1, J.kw({"ensure_ascii": false}))));
        }
      }
      out.push(J.str("\n</tools>\n\nTool usage guidelines:\n- You may call zero or more functions. If no function calls are needed, just answer normally and do not include any <function ... </function>.\n- When calling a function, return an XML object within <function ... </function> using:\n<function name=\"function-name\"><param name=\"param-name\">param-value</param></function>\n- param-value may be multi-line. If it contains <, & or newline characters, wrap it in a CDATA block: <param name=\"param-name\"><![CDATA[...multi-line value...]]></param>"));
      return out.join("");
      })();
      tool_definitions_0 = captured;
    }
    out.push(J.str("<|im_start|>system\n"));
    if (J.t((J.eq(J.attr(J.item(messages_0, 0), "role"), "system")))) {
      if (J.t((J.has("<tool_def_sep>", J.attr(J.item(messages_0, 0), "content"))))) {
        out.push(J.str(J.call(J.attr(J.attr(J.item(messages_0, 0), "content"), "replace"), "<tool_def_sep>", tool_definitions_0)));
      } else {
        out.push(J.str(J.add(J.add(J.attr(J.item(messages_0, 0), "content"), "\n\n"), tool_definitions_0)));
      }
    } else {
      out.push(J.str(J.call(J.attr(tool_definitions_0, "lstrip"))));
    }
    out.push(J.str("<|im_end|>\n"));
  } else {
    if (J.t((J.eq(J.attr(J.item(messages_0, 0), "role"), "system")))) {
      out.push(J.str(J.add(J.add("<|im_start|>system\n", J.attr(J.item(messages_0, 0), "content")), "<|im_end|>\n")));
    }
  }
  ns_0 = J.call(J.namespace, J.kw({"multi_step_tool": true, "last_query_index": J.sub(J.filter("length", messages_0), 1)}));
  {
    let _items2 = J.iter(J.slice(messages_0, undefined, undefined, J.neg(1)));
    const _loop2 = J.loop(_items2);
    for (let _i2 = 0; _i2 < _items2.length; _i2++) {
      _loop2.index0 = _i2; const _it2 = _items2[_i2]; let message_2 = _it2;
      let index_3 = index_0;
      index_3 = J.sub(J.sub(J.filter("length", messages_0), 1), J.attr(_loop2, "index0"));
      if (J.t(((v) => J.t(v) ? !J.t(((v) => J.t(v) ? J.call(J.attr(J.attr(message_2, "content"), "endswith"), "</tool_response>") : v)(J.call(J.attr(J.attr(message_2, "content"), "startswith"), "<tool_response>"))) : v)(((v) => J.t(v) ? J.test("string", J.attr(message_2, "content")) : v)(((v) => J.t(v) ? (J.eq(J.attr(message_2, "role"), "user")) : v)(J.attr(ns_0, "multi_step_tool")))))) {
        J.setattr(ns_0, "multi_step_tool", false);
        J.setattr(ns_0, "last_query_index", index_3);
      }
    }
  }
  {
    let _items3 = J.iter(messages_0);
    const _loop3 = J.loop(_items3);
    for (let _i3 = 0; _i3 < _items3.length; _i3++) {
      _loop3.index0 = _i3; const _it3 = _items3[_i3]; let message_4 = _it3;
      let content_5 = content_0;
      let reasoning_content_6 = reasoning_content_0;
      if (J.t(J.test("string", J.attr(message_4, "content")))) {
        content_5 = J.attr(message_4, "content");
      } else {
        content_5 = "";
      }
      if (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? !J.t(J.attr(_loop3, "first")) : v)((J.eq(J.attr(message_4, "role"), "system"))))((J.eq(J.attr(message_4, "role"), "user"))))) {
        out.push(J.str(J.add(J.add(J.add(J.add(J.add("<|im_start|>", J.attr(message_4, "role")), "\n"), content_5), "<|im_end|>"), "\n")));
      } else if (J.t((J.eq(J.attr(message_4, "role"), "assistant")))) {
        reasoning_content_6 = "";
        if (J.t(J.test("string", J.attr(message_4, "reasoning_content")))) {
          reasoning_content_6 = J.attr(message_4, "reasoning_content");
        } else {
          if (J.t((J.has("</think>", content_5)))) {
            reasoning_content_6 = J.call(J.attr(J.item(J.call(J.attr(J.call(J.attr(J.item(J.call(J.attr(content_5, "split"), "</think>"), 0), "rstrip"), "\n"), "split"), "<think>"), J.neg(1)), "lstrip"), "\n");
            content_5 = J.call(J.attr(J.item(J.call(J.attr(content_5, "split"), "</think>"), J.neg(1)), "lstrip"), "\n");
          }
        }
        if (J.t(reasoning_content_6)) {
          out.push(J.str(J.add(J.add(J.add(J.add(J.add("<|im_start|>", J.attr(message_4, "role")), "\n<think>\n"), J.call(J.attr(reasoning_content_6, "strip"), "\n")), "\n</think>\n\n"), J.call(J.attr(content_5, "lstrip"), "\n"))));
        } else if (J.t(((v) => J.t(v) ? (!J.has("</think>", content_5)) : v)((!J.has("<think>", content_5))))) {
          out.push(J.str(J.add(J.add(J.add("<|im_start|>", J.attr(message_4, "role")), "\n<think>\n\n</think>\n\n"), J.call(J.attr(content_5, "lstrip"), "\n"))));
        } else {
          out.push(J.str(J.add(J.add(J.add("<|im_start|>", J.attr(message_4, "role")), "\n"), content_5)));
        }
        if (J.t(J.attr(message_4, "tool_calls"))) {
          {
            let _items4 = J.iter(J.attr(message_4, "tool_calls"));
            const _loop4 = J.loop(_items4);
            for (let _i4 = 0; _i4 < _items4.length; _i4++) {
              _loop4.index0 = _i4; const _it4 = _items4[_i4]; let tool_call_7 = _it4;
              let args_dict_8 = args_dict_0;
              if (J.t(((v) => J.t(v) ? v : !J.t(J.attr(_loop4, "first")))(((v) => J.t(v) ? content_5 : v)(J.attr(_loop4, "first"))))) {
                out.push(J.str("\n"));
              }
              if (J.t(J.attr(tool_call_7, "function"))) {
                tool_call_7 = J.attr(tool_call_7, "function");
              }
              out.push(J.str(J.concat("<function name=\"", J.attr(tool_call_7, "name"), "\">")));
              if (J.t(J.attr(tool_call_7, "arguments"))) {
                args_dict_8 = J.attr(tool_call_7, "arguments");
                {
                  let _items5 = J.iter(J.call(J.attr(args_dict_8, "items")));
                  const _loop5 = J.loop(_items5);
                  for (let _i5 = 0; _i5 < _items5.length; _i5++) {
                    _loop5.index0 = _i5; const _it5 = _items5[_i5]; let [param_name_9, param_value_10] = J.unpack(_it5, 2);
                    out.push(J.str(J.concat("<param name=\"", param_name_9, "\">")));
                    if (J.t(((v) => J.t(v) ? ((v) => J.t(v) ? v : (J.has("\n", param_value_10)))(((v) => J.t(v) ? v : (J.has("&", param_value_10)))((J.has("<", param_value_10)))) : v)(J.test("string", param_value_10)))) {
                      out.push(J.str(J.add(J.add("<![CDATA[", param_value_10), "]]>")));
                    } else {
                      out.push(J.str(param_value_10));
                    }
                    out.push(J.str("</param>"));
                  }
                }
              }
              out.push(J.str("</function>"));
            }
          }
        }
        out.push(J.str("<|im_end|>\n"));
      } else if (J.t((J.eq(J.attr(message_4, "role"), "tool")))) {
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.sub(J.attr(_loop3, "index0"), 1)), "role"), "tool")))(J.attr(_loop3, "first")))) {
          out.push(J.str("<|im_start|>user"));
        }
        out.push(J.str("\n<tool_response>\n"));
        if (J.t(J.test("string", J.attr(message_4, "content")))) {
          out.push(J.str(content_5));
        } else {
          out.push(J.str(J.filter("tojson", J.attr(message_4, "content"), J.kw({"ensure_ascii": false}))));
        }
        out.push(J.str("\n</tool_response>"));
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.add(J.attr(_loop3, "index0"), 1)), "role"), "tool")))(J.attr(_loop3, "last")))) {
          out.push(J.str("<|im_end|>\n"));
        }
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|im_start|>assistant\n"));
    if (J.t((enable_thinking_0 !== undefined))) {
      if (J.t(J.test("false", enable_thinking_0))) {
        out.push(J.str("<think>\n\n</think>\n\n"));
      } else if (J.t(J.test("true", enable_thinking_0))) {
        out.push(J.str("<think>\n"));
      }
    }
  }
  return out.join("");
}
