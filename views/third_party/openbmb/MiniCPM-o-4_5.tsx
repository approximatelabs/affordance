// Converted from the chat template of openbmb/MiniCPM-o-4_5 by tools/jinja2tsx.py. Regenerate it; do not edit.
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
  let enable_thinking_0 = ctx["enable_thinking"];
  let use_tts_template_0 = ctx["use_tts_template"];
  if (J.t(tools_0)) {
    out.push(J.str("<|im_start|>system\n"));
    if (J.t((J.eq(J.attr(J.item(messages_0, 0), "role"), "system")))) {
      out.push(J.str(J.add(J.attr(J.item(messages_0, 0), "content"), "\n\n")));
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
      if (J.t(((v) => J.t(v) ? !J.t(((v) => J.t(v) ? J.call(J.attr(J.attr(message_2, "content"), "endswith"), "</tool_response>") : v)(J.call(J.attr(J.attr(message_2, "content"), "startswith"), "<tool_response>"))) : v)(((v) => J.t(v) ? (J.eq(J.attr(message_2, "role"), "user")) : v)(J.attr(ns_0, "multi_step_tool"))))) {
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
      if (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? !J.t(J.attr(_loop3, "first")) : v)((J.eq(J.attr(message_4, "role"), "system"))))((J.eq(J.attr(message_4, "role"), "user"))))) {
        out.push(J.str(J.add(J.add(J.add(J.add(J.add("<|im_start|>", J.attr(message_4, "role")), "\n"), J.attr(message_4, "content")), "<|im_end|>"), "\n")));
      } else if (J.t((J.eq(J.attr(message_4, "role"), "assistant")))) {
        content_5 = J.attr(message_4, "content");
        reasoning_content_6 = "";
        if (J.t(((v) => J.t(v) ? !J.t((J.attr(message_4, "reasoning_content") === null)) : v)((J.attr(message_4, "reasoning_content") !== undefined)))) {
          reasoning_content_6 = J.attr(message_4, "reasoning_content");
        } else {
          if (J.t((J.has("</think>", J.attr(message_4, "content"))))) {
            content_5 = J.call(J.attr(J.item(J.call(J.attr(J.attr(message_4, "content"), "split"), "</think>"), J.neg(1)), "lstrip"), "\n");
            reasoning_content_6 = J.call(J.attr(J.item(J.call(J.attr(J.call(J.attr(J.item(J.call(J.attr(J.attr(message_4, "content"), "split"), "</think>"), 0), "rstrip"), "\n"), "split"), "<think>"), J.neg(1)), "lstrip"), "\n");
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
        out.push(J.str("\n<tool_response>\n"), J.str(J.attr(message_4, "content")), J.str("\n</tool_response>"));
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.add(J.attr(_loop3, "index0"), 1)), "role"), "tool")))(J.attr(_loop3, "last")))) {
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
    if (J.t(((v) => J.t(v) ? J.test("true", use_tts_template_0) : v)((use_tts_template_0 !== undefined)))) {
      out.push(J.str("<|tts_bos|>"));
    }
  }
  return out.join("");
}
