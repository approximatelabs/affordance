// Converted from the chat template of nvidia/NVIDIA-Nemotron-Nano-9B-v2 by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let ns_0 = ctx["ns"];
  let messages_0 = ctx["messages"];
  let content_0 = ctx["content"];
  let tools_0 = ctx["tools"] ?? null;
  let fn_0 = ctx["fn"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let continue_final_message_0 = ctx["continue_final_message"];
  ns_0 = J.call(J.namespace, J.kw({"enable_thinking": true}));
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      let content_2 = content_0;
      content_2 = J.item(message_1, "content");
      if (J.t(((v) => J.t(v) ? v : (J.eq(J.item(message_1, "role"), "system")))((J.eq(J.item(message_1, "role"), "user"))))) {
        if (J.t((J.has("/think", content_2)))) {
          J.setattr(ns_0, "enable_thinking", true);
        } else if (J.t((J.has("/no_think", content_2)))) {
          J.setattr(ns_0, "enable_thinking", false);
        }
      }
    }
  }
  if (J.t((!J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
    J.setattr(ns_0, "non_tool_system_content", "");
    out.push(J.str("<SPECIAL_10>System\n"));
  } else {
    J.setattr(ns_0, "non_tool_system_content", J.call(J.attr(J.call(J.attr(J.call(J.attr(J.item(J.item(messages_0, 0), "content"), "replace"), "/think", ""), "replace"), "/no_think", ""), "strip")));
    out.push(J.str(J.add("<SPECIAL_10>System\n", J.attr(ns_0, "non_tool_system_content"))));
  }
  if (J.t(tools_0)) {
    if (J.t(((v) => J.t(v) ? (!J.eq(J.attr(ns_0, "non_tool_system_content"), "")) : v)((J.attr(ns_0, "non_tool_system_content") !== undefined)))) {
      out.push(J.str("\n\n"));
    }
    out.push(J.str("You can use the following tools to assist the user if required:"), J.str("\n<AVAILABLE_TOOLS>["));
    {
      let _items2 = J.iter(tools_0);
      const _loop2 = J.loop(_items2);
      for (let _i2 = 0; _i2 < _items2.length; _i2++) {
        _loop2.index0 = _i2; const _it2 = _items2[_i2]; let tool_3 = _it2;
        out.push(J.str(J.filter("tojson", (J.t((J.attr(tool_3, "function") !== undefined)) ? J.attr(tool_3, "function") : tool_3))), J.str((J.t(!J.t(J.attr(_loop2, "last"))) ? ", " : "")));
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
    let _items3 = J.iter(messages_0);
    const _loop3 = J.loop(_items3);
    for (let _i3 = 0; _i3 < _items3.length; _i3++) {
      _loop3.index0 = _i3; const _it3 = _items3[_i3]; let message_4 = _it3;
      let content_5 = content_0;
      content_5 = J.item(message_4, "content");
      if (J.t((J.eq(J.item(message_4, "role"), "user")))) {
        out.push(J.str(J.add(J.add("<SPECIAL_11>User\n", J.call(J.attr(J.call(J.attr(J.call(J.attr(content_5, "replace"), "/think", ""), "replace"), "/no_think", ""), "strip"))), "\n")));
      } else if (J.t((J.eq(J.item(message_4, "role"), "tool")))) {
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.sub(J.attr(_loop3, "index0"), 1)), "role"), "tool")))(J.attr(_loop3, "first")))) {
          out.push(J.str(J.add("<SPECIAL_11>User\n", "<TOOL_RESPONSE>[")));
        }
        out.push(J.str(J.item(message_4, "content")), J.str((J.t(((v) => J.t(v) ? (J.eq(J.attr(J.item(messages_0, J.add(J.attr(_loop3, "index0"), 1)), "role"), "tool")) : v)(!J.t(J.attr(_loop3, "last")))) ? ", " : "")));
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.add(J.attr(_loop3, "index0"), 1)), "role"), "tool")))(J.attr(_loop3, "last")))) {
          out.push(J.str("]</TOOL_RESPONSE>\n"));
        }
      } else if (J.t((J.eq(J.item(message_4, "role"), "assistant")))) {
        if (J.t((J.has("</think>", content_5)))) {
          content_5 = J.call(J.attr(J.item(J.call(J.attr(content_5, "split"), "</think>"), 1), "strip"));
        }
        out.push(J.str(J.add("<SPECIAL_11>Assistant\n", J.call(J.attr(content_5, "strip")))));
        if (J.t(J.attr(message_4, "tool_calls"))) {
          if (J.t((!J.eq(J.call(J.attr(content_5, "strip")), "")))) {
            out.push(J.str("\n\n"));
          }
          out.push(J.str("<TOOLCALL>["));
          {
            let _items4 = J.iter(J.attr(message_4, "tool_calls"));
            const _loop4 = J.loop(_items4);
            for (let _i4 = 0; _i4 < _items4.length; _i4++) {
              _loop4.index0 = _i4; const _it4 = _items4[_i4]; let call_6 = _it4;
              let fn_7 = fn_0;
              fn_7 = (J.t((J.attr(call_6, "function") !== undefined)) ? J.attr(call_6, "function") : call_6);
              out.push(J.str(J.add(J.add("{\"name\": \"", J.attr(fn_7, "name")), "\", \"arguments\": ")));
              if (J.t(J.test("string", J.attr(fn_7, "arguments")))) {
                out.push(J.str(J.attr(fn_7, "arguments")));
              } else {
                out.push(J.str(J.filter("tojson", J.attr(fn_7, "arguments"))));
              }
              out.push(J.str(J.add("}", (J.t(!J.t(J.attr(_loop4, "last"))) ? ", " : ""))));
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
    if (J.t(((v) => J.t(v) ? J.test("false", J.attr(ns_0, "enable_thinking")) : v)((J.attr(ns_0, "enable_thinking") !== undefined)))) {
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
      if (J.t(((v) => J.t(v) ? J.test("false", J.attr(ns_0, "enable_thinking")) : v)((J.attr(ns_0, "enable_thinking") !== undefined)))) {
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
