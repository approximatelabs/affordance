// Converted from the chat template of openbmb/MiniCPM5-1B by tools/jinja2tsx.py. Regenerate it; do not edit.
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
  let content_parts_0 = ctx["content_parts"];
  let processed_content_0 = ctx["processed_content"];
  let tool_calls_count_0 = ctx["tool_calls_count"];
  let tool_sep_count_0 = ctx["tool_sep_count"];
  let min_count_0 = ctx["min_count"];
  let tool_index_0 = ctx["tool_index"];
  let tool_call_0 = ctx["tool_call"];
  let single_tool_xml_0 = ctx["single_tool_xml"];
  let args_dict_0 = ctx["args_dict"];
  let remaining_tool_xml_0 = ctx["remaining_tool_xml"];
  let has_tool_sep_0 = ctx["has_tool_sep"];
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
      let content_parts_7 = content_parts_0;
      let processed_content_8 = processed_content_0;
      let tool_calls_count_9 = tool_calls_count_0;
      let tool_sep_count_10 = tool_sep_count_0;
      let min_count_11 = min_count_0;
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
        if (J.t(J.attr(message_4, "tool_calls"))) {
          content_parts_7 = J.call(J.attr(content_5, "split"), "<tool_sep>");
          processed_content_8 = J.item(content_parts_7, 0);
          tool_calls_count_9 = J.filter("length", J.attr(message_4, "tool_calls"));
          tool_sep_count_10 = J.sub(J.filter("length", content_parts_7), 1);
          min_count_11 = J.filter("min", [tool_calls_count_9, tool_sep_count_10]);
          {
            let _items4 = J.iter(J.call(J.range, 1, J.filter("length", content_parts_7)));
            const _loop4 = J.loop(_items4);
            for (let _i4 = 0; _i4 < _items4.length; _i4++) {
              _loop4.index0 = _i4; const _it4 = _items4[_i4]; let i_12 = _it4;
              let tool_index_13 = tool_index_0;
              let tool_call_14 = tool_call_0;
              let single_tool_xml_15 = single_tool_xml_0;
              let processed_content_16 = processed_content_8;
              tool_index_13 = J.sub(i_12, 1);
              if (J.t((J.lt(tool_index_13, tool_calls_count_9)))) {
                tool_call_14 = J.item(J.attr(message_4, "tool_calls"), tool_index_13);
                if (J.t(J.attr(tool_call_14, "function"))) {
                  tool_call_14 = J.attr(tool_call_14, "function");
                }
                {
                  const captured = (() => {
                  const out = [];
                  let args_dict_17 = args_dict_0;
                  out.push(J.str(J.concat("<function name=\"", J.attr(tool_call_14, "name"), "\">")));
                  if (J.t(J.attr(tool_call_14, "arguments"))) {
                    args_dict_17 = J.attr(tool_call_14, "arguments");
                    {
                      let _items5 = J.iter(J.call(J.attr(args_dict_17, "items")));
                      const _loop5 = J.loop(_items5);
                      for (let _i5 = 0; _i5 < _items5.length; _i5++) {
                        _loop5.index0 = _i5; const _it5 = _items5[_i5]; let [param_name_18, param_value_19] = J.unpack(_it5, 2);
                        out.push(J.str(J.concat("<param name=\"", param_name_18, "\">")));
                        if (J.t(((v) => J.t(v) ? ((v) => J.t(v) ? v : (J.has("\n", param_value_19)))(((v) => J.t(v) ? v : (J.has("&", param_value_19)))((J.has("<", param_value_19)))) : v)(J.test("string", param_value_19)))) {
                          out.push(J.str(J.add(J.add("<![CDATA[", param_value_19), "]]>")));
                        } else {
                          out.push(J.str(param_value_19));
                        }
                        out.push(J.str("</param>"));
                      }
                    }
                  }
                  out.push(J.str("</function>"));
                  return out.join("");
                  })();
                  single_tool_xml_15 = captured;
                }
                processed_content_16 = J.add(J.add(processed_content_16, single_tool_xml_15), J.item(content_parts_7, i_12));
              } else {
                processed_content_16 = J.add(processed_content_16, J.item(content_parts_7, i_12));
              }
            }
          }
          if (J.t((J.gt(tool_calls_count_9, tool_sep_count_10)))) {
            {
              let _items6 = J.iter(J.call(J.range, tool_sep_count_10, tool_calls_count_9));
              const _loop6 = J.loop(_items6);
              for (let _i6 = 0; _i6 < _items6.length; _i6++) {
                _loop6.index0 = _i6; const _it6 = _items6[_i6]; let remaining_index_20 = _it6;
                let tool_call_21 = tool_call_0;
                let remaining_tool_xml_22 = remaining_tool_xml_0;
                let processed_content_23 = processed_content_8;
                tool_call_21 = J.item(J.attr(message_4, "tool_calls"), remaining_index_20);
                if (J.t(J.attr(tool_call_21, "function"))) {
                  tool_call_21 = J.attr(tool_call_21, "function");
                }
                {
                  const captured = (() => {
                  const out = [];
                  let args_dict_24 = args_dict_0;
                  out.push(J.str(J.concat("<function name=\"", J.attr(tool_call_21, "name"), "\">")));
                  if (J.t(J.attr(tool_call_21, "arguments"))) {
                    args_dict_24 = J.attr(tool_call_21, "arguments");
                    {
                      let _items7 = J.iter(J.call(J.attr(args_dict_24, "items")));
                      const _loop7 = J.loop(_items7);
                      for (let _i7 = 0; _i7 < _items7.length; _i7++) {
                        _loop7.index0 = _i7; const _it7 = _items7[_i7]; let [param_name_25, param_value_26] = J.unpack(_it7, 2);
                        out.push(J.str(J.concat("<param name=\"", param_name_25, "\">")));
                        if (J.t(((v) => J.t(v) ? ((v) => J.t(v) ? v : (J.has("\n", param_value_26)))(((v) => J.t(v) ? v : (J.has("&", param_value_26)))((J.has("<", param_value_26)))) : v)(J.test("string", param_value_26)))) {
                          out.push(J.str(J.add(J.add("<![CDATA[", param_value_26), "]]>")));
                        } else {
                          out.push(J.str(param_value_26));
                        }
                        out.push(J.str("</param>"));
                      }
                    }
                  }
                  out.push(J.str("</function>"));
                  return out.join("");
                  })();
                  remaining_tool_xml_22 = captured;
                }
                processed_content_23 = J.add(processed_content_23, remaining_tool_xml_22);
              }
            }
          }
          content_5 = processed_content_8;
        }
        if (J.t((J.gt(J.attr(_loop3, "index0"), J.attr(ns_0, "last_query_index"))))) {
          if (J.t(reasoning_content_6)) {
            out.push(J.str(J.add(J.add(J.add(J.add(J.add("<|im_start|>", J.attr(message_4, "role")), "\n<think>\n"), J.call(J.attr(reasoning_content_6, "strip"), "\n")), "\n</think>\n\n"), J.call(J.attr(content_5, "lstrip"), "\n"))));
          } else {
            out.push(J.str(J.add(J.add(J.add("<|im_start|>", J.attr(message_4, "role")), "\n"), content_5)));
          }
        } else {
          out.push(J.str(J.add(J.add(J.add("<|im_start|>", J.attr(message_4, "role")), "\n"), content_5)));
        }
        if (J.t(((v) => J.t(v) ? !J.t(has_tool_sep_0) : v)(J.attr(message_4, "tool_calls")))) {
          {
            let _items8 = J.iter(J.attr(message_4, "tool_calls"));
            const _loop8 = J.loop(_items8);
            for (let _i8 = 0; _i8 < _items8.length; _i8++) {
              _loop8.index0 = _i8; const _it8 = _items8[_i8]; let tool_call_27 = _it8;
              let args_dict_28 = args_dict_0;
              if (J.t(((v) => J.t(v) ? v : !J.t(J.attr(_loop8, "first")))(((v) => J.t(v) ? content_5 : v)(J.attr(_loop8, "first"))))) {
                out.push(J.str("\n"));
              }
              if (J.t(J.attr(tool_call_27, "function"))) {
                tool_call_27 = J.attr(tool_call_27, "function");
              }
              out.push(J.str(J.concat("<function name=\"", J.attr(tool_call_27, "name"), "\">")));
              if (J.t(J.attr(tool_call_27, "arguments"))) {
                args_dict_28 = J.attr(tool_call_27, "arguments");
                {
                  let _items9 = J.iter(J.call(J.attr(args_dict_28, "items")));
                  const _loop9 = J.loop(_items9);
                  for (let _i9 = 0; _i9 < _items9.length; _i9++) {
                    _loop9.index0 = _i9; const _it9 = _items9[_i9]; let [param_name_29, param_value_30] = J.unpack(_it9, 2);
                    out.push(J.str(J.concat("<param name=\"", param_name_29, "\">")));
                    if (J.t(((v) => J.t(v) ? ((v) => J.t(v) ? v : (J.has("\n", param_value_30)))(((v) => J.t(v) ? v : (J.has("&", param_value_30)))((J.has("<", param_value_30)))) : v)(J.test("string", param_value_30)))) {
                      out.push(J.str(J.add(J.add("<![CDATA[", param_value_30), "]]>")));
                    } else {
                      out.push(J.str(param_value_30));
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
