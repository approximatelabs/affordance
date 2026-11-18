// Converted from the chat template of Nanbeige/Nanbeige4.2-3B by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let visible_text_0 = ctx["visible_text"];
  let tool_call_format_0 = ctx["tool_call_format"];
  let ns_0 = ctx["ns"];
  let media_type_0 = ctx["media_type"];
  let tools_0 = ctx["tools"] ?? null;
  let messages_0 = ctx["messages"];
  let index_0 = ctx["index"];
  let content_0 = ctx["content"];
  let reasoning_content_0 = ctx["reasoning_content"];
  let preserve_thinking_0 = ctx["preserve_thinking"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let enable_thinking_0 = ctx["enable_thinking"];
  visible_text_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_2 = k.caller;
    let content_1 = p.length > 0 ? p[0] : "content" in k ? k["content"] : undefined;
    if (J.t(J.test("string", content_1))) {
      out.push(J.str(content_1));
    } else if (J.t(((v) => J.t(v) ? !J.t(J.test("mapping", content_1)) : v)(J.test("iterable", content_1)))) {
      {
        let _items1 = J.iter(content_1);
        const _loop1 = J.loop(_items1);
        for (let _i1 = 0; _i1 < _items1.length; _i1++) {
          _loop1.index0 = _i1; const _it1 = _items1[_i1]; let item_3 = _it1;
          let media_type_4 = media_type_0;
          if (J.t(((v) => J.t(v) ? (J.eq(J.attr(item_3, "type"), "text")) : v)(J.test("mapping", item_3)))) {
            out.push(J.str(J.attr(item_3, "text")));
          } else if (J.t(J.test("string", item_3))) {
            out.push(J.str(item_3));
          } else if (J.t(((v) => J.t(v) ? (J.has(J.attr(item_3, "type"), ["image", "image_url", "video", "video_url", "audio", "audio_url", "input_audio"])) : v)(J.test("mapping", item_3)))) {
            media_type_4 = J.filter("replace", J.filter("replace", J.attr(item_3, "type"), "_url", ""), "input_", "");
            out.push(J.str(J.concat("<reminder>You are unable to process this ", media_type_4, " because you don't have multi-modal input ability. Try different methods.</reminder>")));
          }
        }
      }
    } else {
      out.push(J.str(content_1));
    }
    return out.join("");
  };
  tool_call_format_0 = (J.t((tool_call_format_0 !== undefined)) ? tool_call_format_0 : "xml");
  if (J.t(tools_0)) {
    out.push(J.str("<|im_start|>system\n"));
    if (J.t(((v) => J.t(v) ? (J.eq(J.call(J.attr(J.item(messages_0, 0), "get"), "role", ""), "system")) : v)((J.gt(J.filter("length", messages_0), 0))))) {
      out.push(J.str(J.add(J.call(visible_text_0, J.attr(J.item(messages_0, 0), "content")), "\n\n")));
    } else {
      out.push(J.str("\u4f60\u662f\u4e00\u4f4d\u5de5\u5177\u51fd\u6570\u8c03\u7528\u4e13\u5bb6\uff0c\u4f60\u4f1a\u5f97\u5230\u4e00\u4e2a\u95ee\u9898\u548c\u4e00\u7ec4\u53ef\u80fd\u7684\u5de5\u5177\u51fd\u6570\u3002\u6839\u636e\u95ee\u9898\uff0c\u4f60\u9700\u8981\u8fdb\u884c\u4e00\u4e2a\u6216\u591a\u4e2a\u51fd\u6570/\u5de5\u5177\u8c03\u7528\u4ee5\u5b9e\u73b0\u76ee\u7684\uff0c\u8bf7\u5c3d\u91cf\u5c1d\u8bd5\u63a2\u7d22\u901a\u8fc7\u5de5\u5177\u89e3\u51b3\u95ee\u9898\u3002\n\u5982\u679c\u6ca1\u6709\u4e00\u4e2a\u51fd\u6570\u53ef\u4ee5\u4f7f\u7528\uff0c\u8bf7\u76f4\u63a5\u4f7f\u7528\u81ea\u7136\u8bed\u8a00\u56de\u590d\u7528\u6237\u3002\n\u5982\u679c\u7ed9\u5b9a\u7684\u95ee\u9898\u7f3a\u5c11\u51fd\u6570\u6240\u9700\u7684\u53c2\u6570\uff0c\u8bf7\u4f7f\u7528\u81ea\u7136\u8bed\u8a00\u8fdb\u884c\u63d0\u95ee\uff0c\u5411\u7528\u6237\u8be2\u95ee\u5fc5\u8981\u4fe1\u606f\u3002\n\u5982\u679c\u8c03\u7528\u7ed3\u679c\u5df2\u7ecf\u8db3\u591f\u56de\u7b54\u7528\u6237\u95ee\u9898\uff0c\u8bf7\u5bf9\u5386\u53f2\u7ed3\u679c\u8fdb\u884c\u603b\u7ed3\uff0c\u4f7f\u7528\u81ea\u7136\u8bed\u8a00\u56de\u590d\u7528\u6237\u3002"));
    }
    out.push(J.str("# Tools\n\nYou may call one or more functions to assist with the user query.\n\nYou are provided with function signatures within <tools></tools> XML tags:\n<tools>"));
    {
      let _items2 = J.iter(tools_0);
      const _loop2 = J.loop(_items2);
      for (let _i2 = 0; _i2 < _items2.length; _i2++) {
        _loop2.index0 = _i2; const _it2 = _items2[_i2]; let tool_5 = _it2;
        out.push(J.str("\n"), J.str(J.filter("tojson", tool_5)));
      }
    }
    if (J.t((J.eq(tool_call_format_0, "json")))) {
      out.push(J.str("\n</tools>\n\nFor each function call, return a json object with function name and arguments within <tool_call></tool_call> XML tags:\n"), J.str("<tool_call>\n{\"name\": <function-name>, \"arguments\": <args-json-object>}\n</tool_call><|im_end|>\n"));
    } else {
      out.push(J.str("\n</tools>\n\nFor each function call, output the function name and arguments within the following XML format:\n"), J.str("<tool_call>\n<function=example_function_name>\n<parameter=example_parameter_1>\nvalue_1\n</parameter>\n<parameter=example_parameter_2>\nThis is the value for the second parameter\nthat can span\nmultiple lines\n</parameter>\n</function>\n</tool_call><|im_end|>\n"));
    }
  } else {
    if (J.t(((v) => J.t(v) ? (J.eq(J.call(J.attr(J.item(messages_0, 0), "get"), "role", ""), "system")) : v)((J.gt(J.filter("length", messages_0), 0))))) {
      out.push(J.str(J.add(J.add("<|im_start|>system\n", J.call(visible_text_0, J.attr(J.item(messages_0, 0), "content"))), "<|im_end|>\n")));
    } else {
      out.push(J.str("<|im_start|>system\n\u4f60\u662f\u5357\u5317\u9601\uff0c\u4e00\u6b3e\u7531BOSS\u76f4\u8058\u81ea\u4e3b\u7814\u53d1\u5e76\u8bad\u7ec3\u7684\u4e13\u4e1a\u5927\u8bed\u8a00\u6a21\u578b\u3002<|im_end|>\n"));
    }
  }
  ns_0 = J.call(J.namespace, J.kw({"multi_step_tool": true, "last_query_index": J.sub(J.filter("length", messages_0), 1)}));
  {
    let _items3 = J.iter(J.slice(messages_0, undefined, undefined, J.neg(1)));
    const _loop3 = J.loop(_items3);
    for (let _i3 = 0; _i3 < _items3.length; _i3++) {
      _loop3.index0 = _i3; const _it3 = _items3[_i3]; let message_6 = _it3;
      let index_7 = index_0;
      index_7 = J.sub(J.sub(J.filter("length", messages_0), 1), J.attr(_loop3, "index0"));
      if (J.t(((v) => J.t(v) ? !J.t(((v) => J.t(v) ? J.call(J.attr(J.call(visible_text_0, J.attr(message_6, "content")), "endswith"), "</tool_response>") : v)(J.call(J.attr(J.call(visible_text_0, J.attr(message_6, "content")), "startswith"), "<tool_response>"))) : v)(((v) => J.t(v) ? J.test("string", J.call(visible_text_0, J.attr(message_6, "content"))) : v)(((v) => J.t(v) ? (J.eq(J.call(J.attr(message_6, "get"), "role", ""), "user")) : v)(J.attr(ns_0, "multi_step_tool")))))) {
        J.setattr(ns_0, "multi_step_tool", false);
        J.setattr(ns_0, "last_query_index", index_7);
      }
    }
  }
  {
    let _items4 = J.iter(messages_0);
    const _loop4 = J.loop(_items4);
    for (let _i4 = 0; _i4 < _items4.length; _i4++) {
      _loop4.index0 = _i4; const _it4 = _items4[_i4]; let message_8 = _it4;
      let content_9 = content_0;
      let reasoning_content_10 = reasoning_content_0;
      if (J.t(J.test("string", J.call(visible_text_0, J.attr(message_8, "content"))))) {
        content_9 = J.call(visible_text_0, J.attr(message_8, "content"));
      } else {
        content_9 = "";
      }
      if (J.t((J.eq(J.call(J.attr(message_8, "get"), "role", ""), "system")))) {
        if (J.t(!J.t(J.attr(_loop4, "first")))) {
          out.push(J.str(J.call(J.raise_exception, "System message must be at the beginning.")));
        }
      } else if (J.t((J.eq(J.call(J.attr(message_8, "get"), "role", ""), "assistant")))) {
        reasoning_content_10 = "";
        if (J.t(J.test("string", J.attr(message_8, "reasoning_content")))) {
          reasoning_content_10 = J.attr(message_8, "reasoning_content");
        } else {
          if (J.t((J.has("</think>", content_9)))) {
            reasoning_content_10 = J.call(J.attr(J.item(J.call(J.attr(J.call(J.attr(J.item(J.call(J.attr(content_9, "split"), "</think>"), 0), "rstrip"), "\n"), "split"), "<think>"), J.neg(1)), "lstrip"), "\n");
            content_9 = J.call(J.attr(J.call(J.attr(J.item(J.call(J.attr(content_9, "split"), "</think>"), J.neg(1)), "lstrip"), "\n"), "rstrip"), "\n");
          }
        }
        reasoning_content_10 = J.filter("trim", reasoning_content_10);
        if (J.t(((v) => J.t(v) ? (J.lt(J.attr(_loop4, "index0"), J.attr(ns_0, "last_query_index"))) : v)(((v) => J.t(v) ? J.test("false", preserve_thinking_0) : v)((preserve_thinking_0 !== undefined))))) {
          out.push(J.str(J.add(J.add(J.add("<|im_start|>", J.call(J.attr(message_8, "get"), "role", "")), "\n<think>\n\n</think>\n\n"), content_9)));
        } else {
          out.push(J.str(J.add(J.add(J.add(J.add(J.add("<|im_start|>", J.call(J.attr(message_8, "get"), "role", "")), "\n<think>\n"), reasoning_content_10), "\n</think>\n\n"), content_9)));
        }
        if (J.t(((v) => J.t(v) ? !J.t(J.test("mapping", J.attr(message_8, "tool_calls"))) : v)(((v) => J.t(v) ? J.test("iterable", J.attr(message_8, "tool_calls")) : v)(J.attr(message_8, "tool_calls"))))) {
          if (J.t((J.eq(tool_call_format_0, "json")))) {
            {
              let _items5 = J.iter(J.attr(message_8, "tool_calls"));
              const _loop5 = J.loop(_items5);
              for (let _i5 = 0; _i5 < _items5.length; _i5++) {
                _loop5.index0 = _i5; const _it5 = _items5[_i5]; let tool_call_11 = _it5;
                if (J.t(((v) => J.t(v) ? v : !J.t(J.attr(_loop5, "first")))(((v) => J.t(v) ? content_9 : v)(J.attr(_loop5, "first"))))) {
                  out.push(J.str("\n"));
                }
                if (J.t(J.attr(tool_call_11, "function"))) {
                  tool_call_11 = J.attr(tool_call_11, "function");
                }
                out.push(J.str("<tool_call>\n{\"name\": \""), J.str(J.attr(tool_call_11, "name")), J.str("\", \"arguments\": "));
                if (J.t(J.test("string", J.attr(tool_call_11, "arguments")))) {
                  out.push(J.str(J.attr(tool_call_11, "arguments")));
                } else {
                  out.push(J.str(J.filter("tojson", J.attr(tool_call_11, "arguments"))));
                }
                out.push(J.str("}\n</tool_call>"));
              }
            }
          } else {
            {
              let _items6 = J.iter(J.attr(message_8, "tool_calls"));
              const _loop6 = J.loop(_items6);
              for (let _i6 = 0; _i6 < _items6.length; _i6++) {
                _loop6.index0 = _i6; const _it6 = _items6[_i6]; let tool_call_12 = _it6;
                if (J.t((J.attr(tool_call_12, "function") !== undefined))) {
                  tool_call_12 = J.attr(tool_call_12, "function");
                }
                if (J.t(J.attr(_loop6, "first"))) {
                  if (J.t(J.filter("trim", content_9))) {
                    out.push(J.str(J.add(J.add("\n\n<tool_call>\n<function=", J.attr(tool_call_12, "name")), ">\n")));
                  } else {
                    out.push(J.str(J.add(J.add("<tool_call>\n<function=", J.attr(tool_call_12, "name")), ">\n")));
                  }
                } else {
                  out.push(J.str(J.add(J.add("\n<tool_call>\n<function=", J.attr(tool_call_12, "name")), ">\n")));
                }
                if (J.t((J.attr(tool_call_12, "arguments") !== undefined))) {
                  {
                    let _items7 = J.iter(J.filter("items", J.attr(tool_call_12, "arguments")));
                    const _loop7 = J.loop(_items7);
                    for (let _i7 = 0; _i7 < _items7.length; _i7++) {
                      _loop7.index0 = _i7; const _it7 = _items7[_i7]; let [args_name_13, args_value_14] = J.unpack(_it7, 2);
                      out.push(J.str(J.add(J.add("<parameter=", args_name_13), ">\n")));
                      args_value_14 = (J.t(J.test("string", args_value_14)) ? J.filter("string", args_value_14) : J.filter("safe", J.filter("tojson", args_value_14)));
                      out.push(J.str(args_value_14), J.str("\n</parameter>\n"));
                    }
                  }
                }
                out.push(J.str("</function>\n</tool_call>"));
              }
            }
          }
        }
        out.push(J.str("<|im_end|>\n"));
      } else if (J.t((J.eq(J.call(J.attr(message_8, "get"), "role", ""), "tool")))) {
        if (J.t(((v) => J.t(v) ? (!J.eq(J.call(J.attr(J.attr(_loop4, "previtem"), "get"), "role", ""), "tool")) : v)(J.attr(_loop4, "previtem")))) {
          out.push(J.str("<|im_start|>user"));
        }
        out.push(J.str("\n<tool_response>\n"), J.str(content_9), J.str("\n</tool_response>"));
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.call(J.attr(J.attr(_loop4, "nextitem"), "get"), "role", ""), "tool")))(J.attr(_loop4, "last")))) {
          out.push(J.str("<|im_end|>\n"));
        }
      } else if (J.t((!J.eq(J.call(J.attr(message_8, "get"), "role", ""), "")))) {
        out.push(J.str(J.add(J.add(J.add(J.add(J.add("<|im_start|>", J.call(J.attr(message_8, "get"), "role", "")), "\n"), content_9), "<|im_end|>"), "\n")));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|im_start|>assistant\n"));
    if (J.t(((v) => J.t(v) ? J.test("false", enable_thinking_0) : v)((enable_thinking_0 !== undefined)))) {
      out.push(J.str("<think>\n\n</think>\n\n"));
    } else {
      out.push(J.str("<think>\n"));
    }
  }
  return out.join("");
}
