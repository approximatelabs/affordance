// Converted from the chat template of inclusionAI/Ling-3.0-tiny by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let thinking_option_0 = ctx["thinking_option"];
  let preserved_thinking_0 = ctx["preserved_thinking"];
  let ns_0 = ctx["ns"];
  let enable_thinking_0 = ctx["enable_thinking"];
  let tools_0 = ctx["tools"] ?? null;
  let messages_0 = ctx["messages"];
  let index_0 = ctx["index"];
  let content_0 = ctx["content"];
  let reasoning_content_0 = ctx["reasoning_content"];
  let tc_0 = ctx["tc"];
  let _args_0 = ctx["_args"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  if (J.t((enable_thinking_0 !== undefined))) {
    if (J.t(enable_thinking_0)) {
      thinking_option_0 = "on";
    } else {
      thinking_option_0 = "off";
    }
  } else if (J.t(!J.t((thinking_option_0 !== undefined)))) {
    thinking_option_0 = "on";
  }
  preserved_thinking_0 = true;
  out.push(J.str("<role>SYSTEM</role>"));
  if (J.t(tools_0)) {
    if (J.t((J.eq(J.attr(J.item(messages_0, 0), "role"), "system")))) {
      out.push(J.str(J.add(J.attr(J.item(messages_0, 0), "content"), "\n")));
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
    out.push(J.str("\n</tools>\n\nIf none of the functions can be used, point it out. If the given question lacks the parameters required by the function, also point it out.\nIf you need to use a function, for each function call, output the function name and arguments within the following XML format:\n<tool_call>{function-name}\n<arg_key>{arg-key-1}</arg_key>\n<arg_value>{arg-value-1}</arg_value>\n<arg_key>{arg-key-2}</arg_key>\n<arg_value>{arg-value-2}</arg_value>\n...\n</tool_call>\n"));
    if (J.t(((v) => J.t(v) ? ((v) => J.t(v) ? v : (J.has("detailed thinking off", J.attr(J.item(messages_0, 0), "content"))))((J.has("detailed thinking on", J.attr(J.item(messages_0, 0), "content")))) : v)(((v) => J.t(v) ? J.test("string", J.attr(J.item(messages_0, 0), "content")) : v)((J.eq(J.attr(J.item(messages_0, 0), "role"), "system")))))) {
      out.push(J.str("<|role_end|>"));
    } else {
      out.push(J.str(J.add(J.add("detailed thinking ", thinking_option_0), "<|role_end|>")));
    }
  } else {
    if (J.t((J.eq(J.attr(J.item(messages_0, 0), "role"), "system")))) {
      if (J.t(((v) => J.t(v) ? v : (J.has("detailed thinking off", J.attr(J.item(messages_0, 0), "content"))))((J.has("detailed thinking on", J.attr(J.item(messages_0, 0), "content")))))) {
        out.push(J.str(J.add(J.attr(J.item(messages_0, 0), "content"), "<|role_end|>")));
      } else {
        out.push(J.str(J.add(J.attr(J.item(messages_0, 0), "content"), "\n")), J.str(J.add(J.add("detailed thinking ", thinking_option_0), "<|role_end|>")));
      }
    } else {
      out.push(J.str(J.add(J.add("detailed thinking ", thinking_option_0), "<|role_end|>")));
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
      if (J.t((J.eq(J.attr(message_4, "role"), "user")))) {
        out.push(J.str(J.add(J.add("<role>HUMAN</role>", J.attr(message_4, "content")), "<|role_end|>")));
      } else if (J.t(((v) => J.t(v) ? !J.t(J.attr(_loop3, "first")) : v)((J.eq(J.attr(message_4, "role"), "system"))))) {
        out.push(J.str(J.add(J.add("<role>SYSTEM</role>", J.attr(message_4, "content")), "<|role_end|>")));
      } else if (J.t((J.eq(J.attr(message_4, "role"), "assistant")))) {
        reasoning_content_6 = "";
        if (J.t(((v) => J.t(v) ? (!J.eq(J.attr(message_4, "reasoning_content"), "")) : v)(J.test("string", J.attr(message_4, "reasoning_content"))))) {
          reasoning_content_6 = J.attr(message_4, "reasoning_content");
        } else {
          if (J.t((J.has("</think>", content_5)))) {
            reasoning_content_6 = J.call(J.attr(J.item(J.call(J.attr(J.call(J.attr(J.item(J.call(J.attr(content_5, "split"), "</think>"), 0), "rstrip"), "\n"), "split"), "<think>"), J.neg(1)), "lstrip"), "\n");
            content_5 = J.call(J.attr(J.item(J.call(J.attr(content_5, "split"), "</think>"), J.neg(1)), "lstrip"), "\n");
          }
        }
        if (J.t(((v) => J.t(v) ? v : (J.gt(J.attr(_loop3, "index0"), J.attr(ns_0, "last_query_index"))))(preserved_thinking_0))) {
          if (J.t((!J.eq(reasoning_content_6, "")))) {
            out.push(J.str(J.add(J.add(J.add(J.add("<role>ASSISTANT</role>", "\n<think>"), J.call(J.attr(reasoning_content_6, "strip"), "\n")), "</think>"), J.call(J.attr(content_5, "lstrip"), "\n"))));
          } else {
            out.push(J.str(J.add("<role>ASSISTANT</role>\n<think></think>", content_5)));
          }
        } else {
          out.push(J.str(J.add("<role>ASSISTANT</role>\n<think></think>", content_5)));
        }
        if (J.t(J.attr(message_4, "tool_calls"))) {
          {
            let _items4 = J.iter(J.attr(message_4, "tool_calls"));
            const _loop4 = J.loop(_items4);
            for (let _i4 = 0; _i4 < _items4.length; _i4++) {
              _loop4.index0 = _i4; const _it4 = _items4[_i4]; let tool_call_7 = _it4;
              let tc_8 = tc_0;
              let _args_9 = _args_0;
              if (J.t(((v) => J.t(v) ? v : !J.t(J.attr(_loop4, "first")))(((v) => J.t(v) ? content_5 : v)(J.attr(_loop4, "first"))))) {
                out.push(J.str("\n"));
              }
              tc_8 = tool_call_7;
              if (J.t(J.attr(tool_call_7, "function"))) {
                tc_8 = J.attr(tool_call_7, "function");
              }
              out.push(J.str(J.add("<tool_call>", J.attr(tc_8, "name"))), "\n");
              _args_9 = J.attr(tc_8, "arguments");
              {
                let _items5 = J.iter(J.call(J.attr(_args_9, "items")));
                const _loop5 = J.loop(_items5);
                for (let _i5 = 0; _i5 < _items5.length; _i5++) {
                  _loop5.index0 = _i5; const _it5 = _items5[_i5]; let [k_10, v_11] = J.unpack(_it5, 2);
                  out.push(J.str(J.add(J.add("<arg_key>", k_10), "</arg_key>")), J.str("\n<arg_value>"));
                  if (J.t(J.test("string", v_11))) {
                    out.push(J.str(v_11));
                  } else {
                    out.push(J.str(J.filter("tojson", v_11, J.kw({"ensure_ascii": false}))));
                  }
                  out.push(J.str("</arg_value>"));
                }
              }
              out.push(J.str("\n</tool_call>"));
            }
          }
        }
        out.push(J.str("<|role_end|>"));
      } else if (J.t((J.eq(J.attr(message_4, "role"), "tool")))) {
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.sub(J.attr(_loop3, "index0"), 1)), "role"), "tool")))(J.attr(_loop3, "first")))) {
          out.push(J.str("<role>OBSERVATION</role>"));
        }
        out.push(J.str("\n<tool_response>\n"), J.str(content_5), J.str("\n</tool_response>"));
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.add(J.attr(_loop3, "index0"), 1)), "role"), "tool")))(J.attr(_loop3, "last")))) {
          out.push(J.str("<|role_end|>"));
        }
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<role>ASSISTANT</role>"));
    if (J.t((J.eq(thinking_option_0, "on")))) {
      out.push(J.str("\n<think>"));
    } else if (J.t((J.eq(thinking_option_0, "off")))) {
      out.push(J.str("\n<think></think>"));
    }
  }
  return out.join("");
}
