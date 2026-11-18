// Converted from the chat template of JetBrains/Mellum2-12B-A2.5B-Thinking by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let normalize_content_0 = ctx["normalize_content"];
  let ns_0 = ctx["ns"];
  let ns_c_0 = ctx["ns_c"];
  let tools_0 = ctx["tools"] ?? null;
  let messages_0 = ctx["messages"];
  let index_0 = ctx["index"];
  let msg_content_0 = ctx["msg_content"];
  let content_0 = ctx["content"];
  let reasoning_content_0 = ctx["reasoning_content"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let enable_thinking_0 = ctx["enable_thinking"];
  normalize_content_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_2 = k.caller;
    let content_1 = p.length > 0 ? p[0] : "content" in k ? k["content"] : undefined;
    let ns_c_3 = ns_c_0;
    if (J.t(J.test("string", content_1))) {
      out.push(J.str(content_1));
    } else if (J.t(((v) => J.t(v) ? !J.t(J.test("mapping", content_1)) : v)(J.test("iterable", content_1)))) {
      ns_c_3 = J.call(J.namespace, J.kw({"text": ""}));
      {
        let _items1 = J.iter(content_1);
        const _loop1 = J.loop(_items1);
        for (let _i1 = 0; _i1 < _items1.length; _i1++) {
          _loop1.index0 = _i1; const _it1 = _items1[_i1]; let part_4 = _it1;
          if (J.t(J.test("mapping", part_4))) {
            if (J.t((J.eq(J.attr(part_4, "type"), "text")))) {
              J.setattr(ns_c_3, "text", J.add(J.attr(ns_c_3, "text"), ((v) => J.t(v) ? v : "")(J.attr(part_4, "text"))));
            } else if (J.t(((v) => J.t(v) ? (J.eq(J.attr(J.attr(part_4, "output"), "type"), "text")) : v)(((v) => J.t(v) ? J.test("mapping", J.attr(part_4, "output")) : v)((J.eq(J.attr(part_4, "type"), "tool-result")))))) {
              J.setattr(ns_c_3, "text", J.add(J.attr(ns_c_3, "text"), ((v) => J.t(v) ? v : "")(J.attr(J.attr(part_4, "output"), "value"))));
            }
          }
        }
      }
      out.push(J.str(J.attr(ns_c_3, "text")));
    }
    return out.join("");
  };
  if (J.t(tools_0)) {
    out.push(J.str("<|im_start|>system\n"));
    if (J.t((J.eq(J.attr(J.item(messages_0, 0), "role"), "system")))) {
      out.push(J.str(J.add(J.call(normalize_content_0, J.attr(J.item(messages_0, 0), "content")), "\n\n")));
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
    out.push(J.str("\n</tools>\n\nFor each function call, return a json object with function name and arguments within <tool_call></tool_call> XML tags:\n<tool_call>\n{\"name\": <function-name>, \"arguments\": <args-json-object>}\n</tool_call><|im_end|>\n"));
  } else {
    if (J.t((J.eq(J.attr(J.item(messages_0, 0), "role"), "system")))) {
      out.push(J.str(J.add(J.add("<|im_start|>system\n", J.call(normalize_content_0, J.attr(J.item(messages_0, 0), "content"))), "<|im_end|>\n")));
    }
  }
  ns_0 = J.call(J.namespace, J.kw({"multi_step_tool": true, "last_query_index": J.sub(J.filter("length", messages_0), 1)}));
  {
    let _items3 = J.iter(J.slice(messages_0, undefined, undefined, J.neg(1)));
    const _loop3 = J.loop(_items3);
    for (let _i3 = 0; _i3 < _items3.length; _i3++) {
      _loop3.index0 = _i3; const _it3 = _items3[_i3]; let message_6 = _it3;
      let index_7 = index_0;
      let msg_content_8 = msg_content_0;
      index_7 = J.sub(J.sub(J.filter("length", messages_0), 1), J.attr(_loop3, "index0"));
      msg_content_8 = J.call(normalize_content_0, J.attr(message_6, "content"));
      if (J.t(((v) => J.t(v) ? !J.t(((v) => J.t(v) ? J.call(J.attr(msg_content_8, "endswith"), "</tool_response>") : v)(J.call(J.attr(msg_content_8, "startswith"), "<tool_response>"))) : v)(((v) => J.t(v) ? (J.eq(J.attr(message_6, "role"), "user")) : v)(J.attr(ns_0, "multi_step_tool"))))) {
        J.setattr(ns_0, "multi_step_tool", false);
        J.setattr(ns_0, "last_query_index", index_7);
      }
    }
  }
  {
    let _items4 = J.iter(messages_0);
    const _loop4 = J.loop(_items4);
    for (let _i4 = 0; _i4 < _items4.length; _i4++) {
      _loop4.index0 = _i4; const _it4 = _items4[_i4]; let message_9 = _it4;
      let content_10 = content_0;
      let reasoning_content_11 = reasoning_content_0;
      content_10 = J.call(normalize_content_0, J.attr(message_9, "content"));
      if (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? !J.t(J.attr(_loop4, "first")) : v)((J.eq(J.attr(message_9, "role"), "system"))))((J.eq(J.attr(message_9, "role"), "user"))))) {
        out.push(J.str(J.add(J.add(J.add(J.add(J.add("<|im_start|>", J.attr(message_9, "role")), "\n"), content_10), "<|im_end|>"), "\n")));
      } else if (J.t((J.eq(J.attr(message_9, "role"), "assistant")))) {
        reasoning_content_11 = "";
        if (J.t(J.test("string", J.attr(message_9, "reasoning_content")))) {
          reasoning_content_11 = J.attr(message_9, "reasoning_content");
        } else {
          if (J.t((J.has("</think>", content_10)))) {
            reasoning_content_11 = J.call(J.attr(J.item(J.call(J.attr(J.call(J.attr(J.item(J.call(J.attr(content_10, "split"), "</think>"), 0), "rstrip"), "\n"), "split"), "<think>"), J.neg(1)), "lstrip"), "\n");
            content_10 = J.call(J.attr(J.item(J.call(J.attr(content_10, "split"), "</think>"), J.neg(1)), "lstrip"), "\n");
          }
        }
        if (J.t((J.gt(J.attr(_loop4, "index0"), J.attr(ns_0, "last_query_index"))))) {
          if (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? reasoning_content_11 : v)(!J.t(J.attr(_loop4, "last"))))(J.attr(_loop4, "last")))) {
            out.push(J.str(J.add(J.add(J.add(J.add(J.add("<|im_start|>", J.attr(message_9, "role")), "\n<think>\n"), J.call(J.attr(reasoning_content_11, "strip"), "\n")), "\n</think>\n\n"), J.call(J.attr(content_10, "lstrip"), "\n"))));
          } else {
            out.push(J.str(J.add(J.add(J.add("<|im_start|>", J.attr(message_9, "role")), "\n"), content_10)));
          }
        } else {
          out.push(J.str(J.add(J.add(J.add("<|im_start|>", J.attr(message_9, "role")), "\n"), content_10)));
        }
        if (J.t(J.attr(message_9, "tool_calls"))) {
          {
            let _items5 = J.iter(J.attr(message_9, "tool_calls"));
            const _loop5 = J.loop(_items5);
            for (let _i5 = 0; _i5 < _items5.length; _i5++) {
              _loop5.index0 = _i5; const _it5 = _items5[_i5]; let tool_call_12 = _it5;
              if (J.t(((v) => J.t(v) ? v : !J.t(J.attr(_loop5, "first")))(((v) => J.t(v) ? content_10 : v)(J.attr(_loop5, "first"))))) {
                out.push(J.str("\n"));
              }
              if (J.t(J.attr(tool_call_12, "function"))) {
                tool_call_12 = J.attr(tool_call_12, "function");
              }
              out.push(J.str("<tool_call>\n{\"name\": \""), J.str(J.attr(tool_call_12, "name")), J.str("\", \"arguments\": "));
              if (J.t(J.test("string", J.attr(tool_call_12, "arguments")))) {
                out.push(J.str(J.attr(tool_call_12, "arguments")));
              } else {
                out.push(J.str(J.filter("tojson", J.attr(tool_call_12, "arguments"))));
              }
              out.push(J.str("}\n</tool_call>"));
            }
          }
        }
        out.push(J.str("<|im_end|>\n"));
      } else if (J.t((J.eq(J.attr(message_9, "role"), "tool")))) {
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.sub(J.attr(_loop4, "index0"), 1)), "role"), "tool")))(J.attr(_loop4, "first")))) {
          out.push(J.str("<|im_start|>user"));
        }
        out.push(J.str("\n<tool_response>\n"), J.str(content_10), J.str("\n</tool_response>"));
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.add(J.attr(_loop4, "index0"), 1)), "role"), "tool")))(J.attr(_loop4, "last")))) {
          out.push(J.str("<|im_end|>\n"));
        }
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|im_start|>assistant\n"));
    if (J.t(((v) => J.t(v) ? J.test("false", enable_thinking_0) : v)((enable_thinking_0 !== undefined)))) {
      out.push(J.str("<think>\n\n</think>\n\n"));
    }
  }
  return out.join("");
}
