// Converted from the chat template of poolside/Laguna-XS.2 by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let enable_thinking_0 = ctx["enable_thinking"];
  let render_assistant_messages_raw_0 = ctx["render_assistant_messages_raw"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let system_message_0 = ctx["system_message"];
  let ns_0 = ctx["ns"];
  let tool_string_0 = ctx["tool_string"];
  let messages_0 = ctx["messages"];
  let tools_0 = ctx["tools"] ?? null;
  let content_0 = ctx["content"];
  let reasoning_content_0 = ctx["reasoning_content"];
  let function_data_0 = ctx["function_data"];
  let _args_0 = ctx["_args"];
  out.push(J.str("\u3008|EOS|\u3009"));
  enable_thinking_0 = J.filter("default", enable_thinking_0, false);
  render_assistant_messages_raw_0 = J.filter("default", render_assistant_messages_raw_0, false);
  add_generation_prompt_0 = J.filter("default", add_generation_prompt_0, false);
  system_message_0 = "You are a helpful, conversationally-fluent assistant made by Poolside. You are here to be helpful to users through natural language conversations.";
  if (J.t(((v) => J.t(v) ? (J.eq(J.attr(J.item(messages_0, 0), "role"), "system")) : v)(messages_0))) {
    system_message_0 = J.attr(J.item(messages_0, 0), "content");
  }
  if (J.t(((v) => J.t(v) ? v : tools_0)(((v) => J.t(v) ? J.call(J.attr(system_message_0, "strip")) : v)(system_message_0)))) {
    out.push(J.str("<system>\n"));
    if (J.t(((v) => J.t(v) ? J.call(J.attr(system_message_0, "strip")) : v)(system_message_0))) {
      out.push(J.str("\n"), J.str(J.call(J.attr(system_message_0, "rstrip"))));
    }
    if (J.t(tools_0)) {
      out.push(J.str("\n\n### Tools\n\n"));
      ns_0 = J.call(J.namespace, J.kw({"tool_string": J.concat("You may call functions to assist with the user query.\n", "All available function signatures are listed below:\n", "<available_tools>\n")}));
      {
        let _items1 = J.iter(tools_0);
        const _loop1 = J.loop(_items1);
        for (let _i1 = 0; _i1 < _items1.length; _i1++) {
          _loop1.index0 = _i1; const _it1 = _items1[_i1]; let tool_1 = _it1;
          J.setattr(ns_0, "tool_string", J.concat(J.attr(ns_0, "tool_string"), J.filter("tojson", tool_1), "\n"));
        }
      }
      if (J.t(enable_thinking_0)) {
        tool_string_0 = J.add(J.attr(ns_0, "tool_string"), J.concat("</available_tools>\n\n", "Wrap your thinking in '<think>', '</think>' tags, followed by a function call. For each function call, return an unescaped XML-like object with function name and arguments within '<tool_call>' and '</tool_call>' tags, like here:\n", "<think> your thoughts here </think>\n", "<tool_call>function-name\n<arg_key>argument-key</arg_key>\n<arg_value>value-of-argument-key</arg_value>\n", "</tool_call>"));
      } else {
        tool_string_0 = J.add(J.attr(ns_0, "tool_string"), J.concat("</available_tools>\n\n", "For each function call, return an unescaped XML-like object ", "with function name and arguments within '<tool_call>' and '</tool_call>' tags, like here:\n", "<tool_call>function-name\n<arg_key>argument-key</arg_key>\n<arg_value>value-of-argument-key</arg_value>\n", "</tool_call>"));
      }
      out.push(J.str(tool_string_0));
    }
    out.push(J.str("\n</system>\n"));
  }
  {
    let _items2 = J.iter(messages_0);
    const _loop2 = J.loop(_items2);
    for (let _i2 = 0; _i2 < _items2.length; _i2++) {
      _loop2.index0 = _i2; const _it2 = _items2[_i2]; let message_2 = _it2;
      let content_3 = content_0;
      content_3 = (J.t(J.test("string", J.attr(message_2, "content"))) ? J.attr(message_2, "content") : "");
      if (J.t((J.eq(J.attr(message_2, "role"), "user")))) {
        out.push(J.str(J.add(J.add("<user>\n", content_3), "\n</user>\n")));
      } else if (J.t((J.eq(J.attr(message_2, "role"), "assistant")))) {
        {
          out.push(J.str("<assistant>\n"));
          if (J.t(render_assistant_messages_raw_0)) {
            if (J.t(enable_thinking_0)) {
              if (J.t(!J.t(J.call(J.attr(content_3, "startswith"), "<think>")))) {
                out.push(J.str("<think>"));
              }
            } else {
              if (J.t(!J.t(J.call(J.attr(content_3, "startswith"), "</think>")))) {
                out.push(J.str("</think>"));
              }
            }
            out.push(J.str(content_3));
            if (J.t(((v) => J.t(v) ? !J.t(J.call(J.attr(content_3, "endswith"), "</assistant>")) : v)(!J.t(J.call(J.attr(content_3, "endswith"), "</assistant>\n"))))) {
              out.push(J.str("\n</assistant>"));
            }
            out.push(J.str("\n"));
          } else {
            reasoning_content_0 = "";
            if (J.t(J.test("string", J.attr(message_2, "reasoning")))) {
              reasoning_content_0 = J.attr(message_2, "reasoning");
            } else if (J.t(J.test("string", J.attr(message_2, "reasoning_content")))) {
              reasoning_content_0 = J.attr(message_2, "reasoning_content");
            }
            if (J.t((J.has("</think>", content_3)))) {
              if (J.t(!J.t(reasoning_content_0))) {
                reasoning_content_0 = J.call(J.attr(J.item(J.call(J.attr(J.call(J.attr(J.item(J.call(J.attr(content_3, "split"), "</think>"), 0), "rstrip"), "\n"), "split"), "<think>"), J.neg(1)), "lstrip"), "\n");
              }
              content_3 = J.call(J.attr(J.item(J.call(J.attr(content_3, "split"), "</think>"), J.neg(1)), "lstrip"), "\n");
            }
            if (J.t(reasoning_content_0)) {
              out.push(J.str(J.add(J.add("<think>\n", J.call(J.attr(reasoning_content_0, "strip"))), "\n</think>\n")));
            } else {
              out.push(J.str("</think>\n"));
            }
            if (J.t(J.call(J.attr(content_3, "strip")))) {
              out.push(J.str(J.concat(J.call(J.attr(content_3, "strip")), "\n")));
            }
            if (J.t(J.attr(message_2, "tool_calls"))) {
              {
                let _items3 = J.iter(J.attr(message_2, "tool_calls"));
                const _loop3 = J.loop(_items3);
                for (let _i3 = 0; _i3 < _items3.length; _i3++) {
                  _loop3.index0 = _i3; const _it3 = _items3[_i3]; let tool_call_4 = _it3;
                  let function_data_5 = function_data_0;
                  let _args_6 = _args_0;
                  function_data_5 = J.attr(tool_call_4, "function");
                  out.push(J.str(J.add("<tool_call>", J.attr(function_data_5, "name"))), "\n");
                  _args_6 = J.attr(function_data_5, "arguments");
                  {
                    let _items4 = J.iter(J.call(J.attr(_args_6, "items")));
                    const _loop4 = J.loop(_items4);
                    for (let _i4 = 0; _i4 < _items4.length; _i4++) {
                      _loop4.index0 = _i4; const _it4 = _items4[_i4]; let [k_7, v_8] = J.unpack(_it4, 2);
                      out.push(J.str(J.concat("<arg_key>", k_7, "</arg_key>\n")), J.str("<arg_value>"), J.str((J.t(!J.t(J.test("string", v_8))) ? J.filter("tojson", v_8, J.kw({"ensure_ascii": false})) : v_8)), J.str("</arg_value>\n"));
                    }
                  }
                  out.push(J.str("</tool_call>\n"));
                }
              }
            }
            out.push(J.str("</assistant>\n"));
          }
        }
      } else if (J.t((J.eq(J.attr(message_2, "role"), "tool")))) {
        out.push(J.str(J.add(J.add("<tool_response>\n", content_3), "\n</tool_response>\n")));
      } else if (J.t(((v) => J.t(v) ? (!J.eq(J.attr(_loop2, "index0"), 0)) : v)((J.eq(J.attr(message_2, "role"), "system"))))) {
        out.push(J.str(J.add(J.add("<system>\n", content_3), "\n</system>\n")));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<assistant>\n"));
    if (J.t(!J.t(enable_thinking_0))) {
      out.push(J.str("</think>"));
    } else {
      out.push(J.str("<think>"));
    }
  }
  return out.join("");
}
