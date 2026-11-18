// Converted from the chat template of baidu/ERNIE-4.5-21B-A3B-Thinking by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let ns_0 = ctx["ns"];
  let system_settings_0 = ctx["system_settings"];
  let messages_0 = ctx["messages"];
  let tools_0 = ctx["tools"] ?? null;
  let index_0 = ctx["index"];
  let content_0 = ctx["content"];
  let reasoning_content_0 = ctx["reasoning_content"];
  out.push(J.str("<|im_start|>system\n"));
  if (J.t(((v) => J.t(v) ? !J.t(system_settings_0) : v)((!J.eq(J.attr(J.item(messages_0, 0), "role"), "system"))))) {
    out.push(J.str("<global_setting>\nthink_mode=True\n</global_setting>"));
  } else {
    out.push(J.str("<system_setting>\n"), J.str((J.t(system_settings_0) ? J.add(system_settings_0, "\n") : "")), J.str(J.add((J.t((J.eq(J.attr(J.item(messages_0, 0), "role"), "system"))) ? J.add(J.attr(J.item(messages_0, 0), "content"), "\n") : ""), "</system_setting>\n\n<global_setting>\nthink_mode=True\n</global_setting>")));
  }
  if (J.t(tools_0)) {
    out.push(J.str("\n\n<tool_list>"), J.str("\n"), J.str("["));
    {
      let _items1 = J.iter(tools_0);
      const _loop1 = J.loop(_items1);
      for (let _i1 = 0; _i1 < _items1.length; _i1++) {
        _loop1.index0 = _i1; const _it1 = _items1[_i1]; let tool_1 = _it1;
        out.push(J.str("{\"type\": \"function\", \"function\": "), J.str(J.filter("tojson", J.attr(tool_1, "function"))), "}");
        if (J.t(!J.t(J.attr(_loop1, "last")))) {
          out.push(",");
        }
      }
    }
    out.push(J.str("]"), J.str("\n</tool_list>"));
  }
  out.push(J.str("<|im_end|>\n\n"));
  ns_0 = J.call(J.namespace, J.kw({"multi_step_tool": true, "last_query_index": J.sub(J.filter("length", messages_0), 1)}));
  {
    let _items2 = J.iter(J.slice(messages_0, undefined, undefined, J.neg(1)));
    const _loop2 = J.loop(_items2);
    for (let _i2 = 0; _i2 < _items2.length; _i2++) {
      _loop2.index0 = _i2; const _it2 = _items2[_i2]; let message_2 = _it2;
      let index_3 = index_0;
      index_3 = J.sub(J.sub(J.filter("length", messages_0), 1), J.attr(_loop2, "index0"));
      if (J.t(((v) => J.t(v) ? !J.t(((v) => J.t(v) ? J.call(J.attr(J.attr(message_2, "content"), "endswith"), "</tool_output>") : v)(J.call(J.attr(J.attr(message_2, "content"), "startswith"), "<tool_output>"))) : v)(((v) => J.t(v) ? J.test("string", J.attr(message_2, "content")) : v)(((v) => J.t(v) ? (J.eq(J.attr(message_2, "role"), "user")) : v)(J.attr(ns_0, "multi_step_tool")))))) {
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
        out.push(J.str(J.add(J.add(J.add(J.add(J.add("<|im_start|>", J.attr(message_4, "role")), "\n"), content_5), "<|im_end|>"), "\n\n")));
      } else if (J.t((J.eq(J.attr(message_4, "role"), "assistant")))) {
        reasoning_content_6 = "";
        if (J.t(J.test("string", J.attr(message_4, "thoughts")))) {
          reasoning_content_6 = J.attr(message_4, "thoughts");
        } else {
          if (J.t((J.has("</think>", content_5)))) {
            reasoning_content_6 = J.call(J.attr(J.item(J.call(J.attr(J.call(J.attr(J.item(J.call(J.attr(content_5, "split"), "</think>"), 0), "rstrip"), "\n"), "split"), "<think>"), J.neg(1)), "lstrip"), "\n");
            content_5 = J.call(J.attr(J.item(J.call(J.attr(content_5, "split"), "</think>"), J.neg(1)), "lstrip"), "\n");
          }
        }
        if (J.t(((v) => J.t(v) ? ((v) => J.t(v) ? v : ((v) => J.t(v) ? reasoning_content_6 : v)(!J.t(J.attr(_loop3, "last"))))(J.attr(_loop3, "last")) : v)((J.gt(J.attr(_loop3, "index0"), J.attr(ns_0, "last_query_index")))))) {
          out.push(J.str(J.add(J.add(J.add(J.add("<|im_start|>", J.attr(message_4, "role")), "\n<think>\n"), J.call(J.attr(reasoning_content_6, "strip"), "\n")), "\n</think>\n")));
        } else {
          out.push(J.str(J.add(J.add("<|im_start|>", J.attr(message_4, "role")), "\n")));
        }
        if (J.t((J.gt(J.filter("length", content_5), 0)))) {
          out.push(J.str(J.add(J.add("<response>\n", content_5), "\n</response>\n")));
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
              out.push(J.str("\n<tool_call>\n{\"name\": \""), J.str(J.attr(tool_call_7, "name")), J.str("\", \"arguments\": "));
              if (J.t(J.test("string", J.attr(tool_call_7, "arguments")))) {
                out.push(J.str(J.attr(tool_call_7, "arguments")));
              } else {
                out.push(J.str(J.filter("tojson", J.attr(tool_call_7, "arguments"))));
              }
              out.push(J.str("}\n</tool_call>\n"));
            }
          }
        }
        out.push(J.str("<|im_end|>\n\n"));
      } else if (J.t((J.eq(J.attr(message_4, "role"), "tool")))) {
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.sub(J.attr(_loop3, "index0"), 1)), "role"), "tool")))(J.attr(_loop3, "first")))) {
          out.push(J.str("<|im_start|>tool"));
        }
        out.push(J.str("\n<tool_output>"), J.str(J.filter("tojson", J.attr(message_4, "content"))), J.str("</tool_output>"));
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.add(J.attr(_loop3, "index0"), 1)), "role"), "tool")))(J.attr(_loop3, "last")))) {
          out.push(J.str("<|im_end|>\n\n"));
        }
      }
    }
  }
  out.push(J.str("<|im_start|>assistant\n<think>\n"));
  return out.join("");
}
