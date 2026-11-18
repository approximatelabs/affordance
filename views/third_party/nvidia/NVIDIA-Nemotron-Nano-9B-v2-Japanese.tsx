// Converted from the chat template of nvidia/NVIDIA-Nemotron-Nano-9B-v2-Japanese by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let ns_0 = ctx["ns"];
  let messages_0 = ctx["messages"];
  let tools_0 = ctx["tools"] ?? null;
  let content_0 = ctx["content"];
  let fn_0 = ctx["fn"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let enable_thinking_0 = ctx["enable_thinking"];
  let continue_final_message_0 = ctx["continue_final_message"];
  ns_0 = J.call(J.namespace);
  if (J.t((!J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
    J.setattr(ns_0, "non_tool_system_content", "");
    out.push(J.str("<SPECIAL_10>System\n"));
  } else {
    J.setattr(ns_0, "non_tool_system_content", J.call(J.attr(J.item(J.item(messages_0, 0), "content"), "strip")));
    out.push(J.str(J.add("<SPECIAL_10>System\n", J.attr(ns_0, "non_tool_system_content"))));
  }
  if (J.t(tools_0)) {
    if (J.t(((v) => J.t(v) ? (!J.eq(J.attr(ns_0, "non_tool_system_content"), "")) : v)((J.attr(ns_0, "non_tool_system_content") !== undefined)))) {
      out.push(J.str("\n\n"));
    }
    out.push(J.str("You can use the following tools to assist the user if required:"), J.str("\n<AVAILABLE_TOOLS>["));
    {
      let _items1 = J.iter(tools_0);
      const _loop1 = J.loop(_items1);
      for (let _i1 = 0; _i1 < _items1.length; _i1++) {
        _loop1.index0 = _i1; const _it1 = _items1[_i1]; let tool_1 = _it1;
        out.push(J.str(J.filter("tojson", (J.t((J.attr(tool_1, "function") !== undefined)) ? J.attr(tool_1, "function") : tool_1))), J.str((J.t(!J.t(J.attr(_loop1, "last"))) ? ", " : "")));
      }
    }
    out.push(J.str("]</AVAILABLE_TOOLS>\n\n"), J.str("If you decide to call any tool(s), use the following format:\n"), J.str("<TOOLCALL>[{{\"name\": \"tool_name1\", \"arguments\": \"tool_args1\"}}, "), J.str("{{\"name\": \"tool_name2\", \"arguments\": \"tool_args2\"}}]</TOOLCALL>\n\n"), J.str("The user will execute tool-calls and return responses from tool(s) in this format:\n"), J.str("<TOOL_RESPONSE>[{{\"tool_response1\"}}, {{\"tool_response2\"}}]</TOOL_RESPONSE>\n\n"), J.str("Based on the tool responses, you can call additional tools if needed, correct tool calls if any errors are found, or just respond to the user."));
  }
  out.push(J.str("\n"));
  messages_0 = (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system"))) ? J.slice(messages_0, 1, undefined, undefined) : messages_0);
  if (J.t((J.eq(J.item(J.item(messages_0, J.neg(1)), "role"), "assistant")))) {
    J.setattr(ns_0, "last_turn_assistant_content", J.call(J.attr(J.item(J.item(messages_0, J.neg(1)), "content"), "strip")));
    messages_0 = J.slice(messages_0, undefined, J.neg(1), undefined);
  }
  {
    let _items2 = J.iter(messages_0);
    const _loop2 = J.loop(_items2);
    for (let _i2 = 0; _i2 < _items2.length; _i2++) {
      _loop2.index0 = _i2; const _it2 = _items2[_i2]; let message_2 = _it2;
      let content_3 = content_0;
      content_3 = (J.t((J.has("content", message_2))) ? J.item(message_2, "content") : "");
      if (J.t((J.eq(J.item(message_2, "role"), "user")))) {
        out.push(J.str(J.add(J.add("<SPECIAL_11>User\n", J.call(J.attr(content_3, "strip"))), "\n")));
      } else if (J.t((J.eq(J.item(message_2, "role"), "tool")))) {
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.sub(J.attr(_loop2, "index0"), 1)), "role"), "tool")))(J.attr(_loop2, "first")))) {
          out.push(J.str(J.add("<SPECIAL_11>User\n", "<TOOL_RESPONSE>[")));
        }
        out.push(J.str(J.item(message_2, "content")), J.str((J.t(((v) => J.t(v) ? (J.eq(J.attr(J.item(messages_0, J.add(J.attr(_loop2, "index0"), 1)), "role"), "tool")) : v)(!J.t(J.attr(_loop2, "last")))) ? ", " : "")));
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.add(J.attr(_loop2, "index0"), 1)), "role"), "tool")))(J.attr(_loop2, "last")))) {
          out.push(J.str("]</TOOL_RESPONSE>\n"));
        }
      } else if (J.t((J.eq(J.item(message_2, "role"), "assistant")))) {
        if (J.t((J.has("</think>", content_3)))) {
          content_3 = J.call(J.attr(J.item(J.call(J.attr(content_3, "split"), "</think>"), 1), "strip"));
        }
        out.push(J.str(J.add("<SPECIAL_11>Assistant\n", J.call(J.attr(content_3, "strip")))));
        if (J.t(J.attr(message_2, "tool_calls"))) {
          if (J.t((!J.eq(J.call(J.attr(content_3, "strip")), "")))) {
            out.push(J.str("\n\n"));
          }
          out.push(J.str("<TOOLCALL>["));
          {
            let _items3 = J.iter(J.attr(message_2, "tool_calls"));
            const _loop3 = J.loop(_items3);
            for (let _i3 = 0; _i3 < _items3.length; _i3++) {
              _loop3.index0 = _i3; const _it3 = _items3[_i3]; let call_4 = _it3;
              let fn_5 = fn_0;
              fn_5 = (J.t((J.attr(call_4, "function") !== undefined)) ? J.attr(call_4, "function") : call_4);
              out.push(J.str(J.add(J.add("{\"name\": \"", J.attr(fn_5, "name")), "\", \"arguments\": ")));
              if (J.t(J.test("string", J.attr(fn_5, "arguments")))) {
                out.push(J.str(J.attr(fn_5, "arguments")));
              } else {
                out.push(J.str(J.filter("tojson", J.attr(fn_5, "arguments"))));
              }
              out.push(J.str(J.add("}", (J.t(!J.t(J.attr(_loop3, "last"))) ? ", " : ""))));
            }
          }
          out.push(J.str("]</TOOLCALL>"));
        }
        out.push(J.str("\n<SPECIAL_12>\n"));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<SPECIAL_11>Assistant\n"));
    if (J.t(((v) => J.t(v) ? !J.t(enable_thinking_0) : v)((enable_thinking_0 !== undefined)))) {
      out.push(J.str("<think></think>"));
    } else {
      out.push(J.str("<think>\n"));
    }
    if (J.t(((v) => J.t(v) ? (!J.eq(J.attr(ns_0, "last_turn_assistant_content"), "")) : v)((J.attr(ns_0, "last_turn_assistant_content") !== undefined)))) {
      out.push(J.str(J.attr(ns_0, "last_turn_assistant_content")));
    }
  } else {
    if (J.t(((v) => J.t(v) ? (!J.eq(J.attr(ns_0, "last_turn_assistant_content"), "")) : v)((J.attr(ns_0, "last_turn_assistant_content") !== undefined)))) {
      out.push(J.str("<SPECIAL_11>Assistant\n"));
      if (J.t(((v) => J.t(v) ? !J.t(enable_thinking_0) : v)((enable_thinking_0 !== undefined)))) {
        out.push(J.str("<think></think>"));
      } else {
        out.push(J.str("<think>\n"));
      }
      out.push(J.str(J.attr(ns_0, "last_turn_assistant_content")));
      if (J.t((continue_final_message_0 !== undefined))) {
        if (J.t(J.test("false", continue_final_message_0))) {
          out.push(J.str("\n<SPECIAL_12>\n"));
        }
      } else {
        out.push(J.str("\n<SPECIAL_12>\n"));
      }
    }
  }
  return out.join("");
}
