// Converted from the chat template of poolside/Laguna-S-2.1 by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let enable_thinking_0 = ctx["enable_thinking"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let preserve_thinking_0 = ctx["preserve_thinking"];
  let system_message_0 = ctx["system_message"];
  let messages_0 = ctx["messages"];
  let has_sys_0 = ctx["has_sys"];
  let tools_0 = ctx["tools"] ?? null;
  let content_0 = ctx["content"];
  let reasoning_content_0 = ctx["reasoning_content"];
  let function_data_0 = ctx["function_data"];
  let _args_0 = ctx["_args"];
  out.push(J.str("\u3008|EOS|\u3009"));
  enable_thinking_0 = J.filter("default", enable_thinking_0, true);
  add_generation_prompt_0 = J.filter("default", add_generation_prompt_0, false);
  preserve_thinking_0 = J.filter("default", preserve_thinking_0, false);
  system_message_0 = "You are a helpful, conversationally-fluent assistant made by Poolside. You are here to be helpful to users through natural language conversations.";
  if (J.t(((v) => J.t(v) ? (J.eq(J.attr(J.item(messages_0, 0), "role"), "system")) : v)(messages_0))) {
    system_message_0 = J.attr(J.item(messages_0, 0), "content");
    messages_0 = J.slice(messages_0, 1, undefined, undefined);
  }
  has_sys_0 = ((v) => J.t(v) ? J.call(J.attr(system_message_0, "strip")) : v)(system_message_0);
  if (J.t(((v) => J.t(v) ? v : enable_thinking_0)(((v) => J.t(v) ? v : tools_0)(has_sys_0)))) {
    out.push(J.str("<system>"));
    if (J.t(has_sys_0)) {
      out.push(J.str(J.call(J.attr(system_message_0, "rstrip"))));
      if (J.t(tools_0)) {
        out.push(J.str("\n\n"));
      }
    }
    if (J.t(tools_0)) {
      out.push(J.str("### Tools\n\n"), J.str("You may call functions to assist with the user query.\n"), J.str("All available function signatures are listed below:\n"), J.str("<available_tools>\n"));
      {
        let _items1 = J.iter(tools_0);
        const _loop1 = J.loop(_items1);
        for (let _i1 = 0; _i1 < _items1.length; _i1++) {
          _loop1.index0 = _i1; const _it1 = _items1[_i1]; let tool_1 = _it1;
          out.push(J.str(J.concat(J.filter("tojson", tool_1), "\n")));
        }
      }
      out.push(J.str("</available_tools>"));
    }
    out.push(J.str("</system>\n"));
  }
  {
    let _items2 = J.iter(messages_0);
    const _loop2 = J.loop(_items2);
    for (let _i2 = 0; _i2 < _items2.length; _i2++) {
      _loop2.index0 = _i2; const _it2 = _items2[_i2]; let message_2 = _it2;
      let content_3 = content_0;
      content_3 = (J.t(J.test("string", J.attr(message_2, "content"))) ? J.attr(message_2, "content") : "");
      if (J.t((J.eq(J.attr(message_2, "role"), "user")))) {
        out.push(J.str(J.add(J.add("<user>", content_3), "</user>\n")));
      } else if (J.t((J.eq(J.attr(message_2, "role"), "assistant")))) {
        {
          out.push(J.str("<assistant>"));
          reasoning_content_0 = "";
          if (J.t(J.test("string", J.attr(message_2, "reasoning")))) {
            reasoning_content_0 = J.attr(message_2, "reasoning");
          } else if (J.t(J.test("string", J.attr(message_2, "reasoning_content")))) {
            reasoning_content_0 = J.attr(message_2, "reasoning_content");
          }
          if (J.t(((v) => J.t(v) ? v : preserve_thinking_0)(enable_thinking_0))) {
            out.push(J.str(J.add(J.add("<think>", reasoning_content_0), "</think>")));
          } else {
            out.push(J.str("</think>"));
          }
          if (J.t(content_3)) {
            out.push(J.str(content_3));
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
                out.push(J.str(J.add("<tool_call>", J.attr(function_data_5, "name"))));
                _args_6 = J.attr(function_data_5, "arguments");
                {
                  let _items4 = J.iter(J.call(J.attr(_args_6, "items")));
                  const _loop4 = J.loop(_items4);
                  for (let _i4 = 0; _i4 < _items4.length; _i4++) {
                    _loop4.index0 = _i4; const _it4 = _items4[_i4]; let [k_7, v_8] = J.unpack(_it4, 2);
                    out.push(J.str(J.concat("<arg_key>", k_7, "</arg_key>")), J.str("<arg_value>"), J.str((J.t(!J.t(J.test("string", v_8))) ? J.filter("tojson", v_8, J.kw({"ensure_ascii": false})) : v_8)), J.str("</arg_value>"));
                  }
                }
                out.push(J.str("</tool_call>"));
              }
            }
          }
          out.push(J.str("</assistant>\n"));
        }
      } else if (J.t((J.eq(J.attr(message_2, "role"), "tool")))) {
        out.push(J.str(J.add(J.add("<tool_response>", content_3), "</tool_response>\n")));
      } else if (J.t((J.eq(J.attr(message_2, "role"), "system")))) {
        out.push(J.str(J.add(J.add("<system>", content_3), "</system>\n")));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<assistant>"));
    if (J.t(enable_thinking_0)) {
      out.push(J.str("<think>"));
    } else {
      out.push(J.str("</think>"));
    }
  }
  return out.join("");
}
