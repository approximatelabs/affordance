// Converted from the chat template of Nanbeige/Nanbeige4-3B-Thinking-2511 by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let ns_0 = ctx["ns"];
  let tools_0 = ctx["tools"] ?? null;
  let messages_0 = ctx["messages"];
  let index_0 = ctx["index"];
  let content_0 = ctx["content"];
  let reasoning_content_0 = ctx["reasoning_content"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  if (J.t(tools_0)) {
    out.push(J.str("<|im_start|>system\n"));
    if (J.t((J.eq(J.attr(J.item(messages_0, 0), "role"), "system")))) {
      out.push(J.str(J.add(J.attr(J.item(messages_0, 0), "content"), "\n\n")));
    } else {
      out.push(J.str("\u4f60\u662f\u4e00\u4f4d\u5de5\u5177\u51fd\u6570\u8c03\u7528\u4e13\u5bb6\uff0c\u4f60\u4f1a\u5f97\u5230\u4e00\u4e2a\u95ee\u9898\u548c\u4e00\u7ec4\u53ef\u80fd\u7684\u5de5\u5177\u51fd\u6570\u3002\u6839\u636e\u95ee\u9898\uff0c\u4f60\u9700\u8981\u8fdb\u884c\u4e00\u4e2a\u6216\u591a\u4e2a\u51fd\u6570/\u5de5\u5177\u8c03\u7528\u4ee5\u5b9e\u73b0\u76ee\u7684\uff0c\u8bf7\u5c3d\u91cf\u5c1d\u8bd5\u63a2\u7d22\u901a\u8fc7\u5de5\u5177\u89e3\u51b3\u95ee\u9898\u3002\n\u5982\u679c\u6ca1\u6709\u4e00\u4e2a\u51fd\u6570\u53ef\u4ee5\u4f7f\u7528\uff0c\u8bf7\u76f4\u63a5\u4f7f\u7528\u81ea\u7136\u8bed\u8a00\u56de\u590d\u7528\u6237\u3002\n\u5982\u679c\u7ed9\u5b9a\u7684\u95ee\u9898\u7f3a\u5c11\u51fd\u6570\u6240\u9700\u7684\u53c2\u6570\uff0c\u8bf7\u4f7f\u7528\u81ea\u7136\u8bed\u8a00\u8fdb\u884c\u63d0\u95ee\uff0c\u5411\u7528\u6237\u8be2\u95ee\u5fc5\u8981\u4fe1\u606f\u3002\n\u5982\u679c\u8c03\u7528\u7ed3\u679c\u5df2\u7ecf\u8db3\u591f\u56de\u7b54\u7528\u6237\u95ee\u9898\uff0c\u8bf7\u5bf9\u5386\u53f2\u7ed3\u679c\u8fdb\u884c\u603b\u7ed3\uff0c\u4f7f\u7528\u81ea\u7136\u8bed\u8a00\u56de\u590d\u7528\u6237\u3002"));
    }
    out.push(J.str("# Tools\n\nYou may call one or more functions to assist with the user query.\n\nYou are provided with function signatures within <tools></tools> XML tags:\n<tools>"));
    {
      let _items1 = J.iter(tools_0);
      const _loop1 = J.loop(_items1);
      for (let _i1 = 0; _i1 < _items1.length; _i1++) {
        _loop1.index0 = _i1; const _it1 = _items1[_i1]; let tool_1 = _it1;
        out.push(J.str("\n"), J.str(J.filter("tojson", tool_1)));
      }
    }
    out.push(J.str("\n</tools>\n\nFor each function call, return a json object with function name and arguments within <tool_call></tool_call> XML tags:\n<tool_call>\n{\"name\": <function-name>, \"arguments\": <args-json-object>}\n</tool_call><|im_end|>\n"));
  } else {
    if (J.t((J.eq(J.attr(J.item(messages_0, 0), "role"), "system")))) {
      out.push(J.str(J.add(J.add("<|im_start|>system\n", J.attr(J.item(messages_0, 0), "content")), "<|im_end|>\n")));
    } else {
      out.push(J.str("<|im_start|>system\n\u4f60\u662f\u5357\u5317\u9601\uff0c\u4e00\u6b3e\u7531BOSS\u76f4\u8058\u81ea\u4e3b\u7814\u53d1\u5e76\u8bad\u7ec3\u7684\u4e13\u4e1a\u5927\u8bed\u8a00\u6a21\u578b\u3002<|im_end|>\n"));
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
        if (J.t((J.gt(J.attr(_loop3, "index0"), J.attr(ns_0, "last_query_index"))))) {
          if (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? reasoning_content_6 : v)(!J.t(J.attr(_loop3, "last"))))(J.attr(_loop3, "last")))) {
            out.push(J.str(J.add(J.add(J.add(J.add(J.add("<|im_start|>", J.attr(message_4, "role")), "\n<think>\n"), J.call(J.attr(reasoning_content_6, "strip"), "\n")), "\n</think>\n\n"), J.call(J.attr(content_5, "lstrip"), "\n"))));
          } else {
            out.push(J.str(J.add(J.add(J.add("<|im_start|>", J.attr(message_4, "role")), "\n"), content_5)));
          }
        } else {
          out.push(J.str(J.add(J.add(J.add("<|im_start|>", J.attr(message_4, "role")), "\n"), content_5)));
        }
        if (J.t(J.attr(message_4, "tool_calls"))) {
          {
            let _items4 = J.iter(J.attr(message_4, "tool_calls"));
            const _loop4 = J.loop(_items4);
            for (let _i4 = 0; _i4 < _items4.length; _i4++) {
              _loop4.index0 = _i4; const _it4 = _items4[_i4]; let tool_call_7 = _it4;
              if (J.t(((v) => J.t(v) ? v : !J.t(J.attr(_loop4, "first")))(((v) => J.t(v) ? content_5 : v)(J.attr(_loop4, "first"))))) {
                out.push(J.str("\n"));
              }
              if (J.t(J.attr(tool_call_7, "function"))) {
                tool_call_7 = J.attr(tool_call_7, "function");
              }
              out.push(J.str("<tool_call>\n{\"name\": \""), J.str(J.attr(tool_call_7, "name")), J.str("\", \"arguments\": "));
              if (J.t(J.test("string", J.attr(tool_call_7, "arguments")))) {
                out.push(J.str(J.attr(tool_call_7, "arguments")));
              } else {
                out.push(J.str(J.filter("tojson", J.attr(tool_call_7, "arguments"))));
              }
              out.push(J.str("}\n</tool_call>"));
            }
          }
        }
        out.push(J.str("<|im_end|>\n"));
      } else if (J.t((J.eq(J.attr(message_4, "role"), "tool")))) {
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.sub(J.attr(_loop3, "index0"), 1)), "role"), "tool")))(J.attr(_loop3, "first")))) {
          out.push(J.str("<|im_start|>user"));
        }
        out.push(J.str("\n<tool_response>\n"), J.str(content_5), J.str("\n</tool_response>"));
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.add(J.attr(_loop3, "index0"), 1)), "role"), "tool")))(J.attr(_loop3, "last")))) {
          out.push(J.str("<|im_end|>\n"));
        }
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|im_start|>assistant\n"));
  }
  return out.join("");
}
