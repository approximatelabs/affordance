// Converted from the chat template of nvidia/Llama-3_3-Nemotron-Super-49B-v1_5 by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let bos_0 = ctx["bos"];
  let enable_thinking_0 = ctx["enable_thinking"];
  let system_start_header_0 = ctx["system_start_header"];
  let system_end_header_0 = ctx["system_end_header"];
  let start_header_0 = ctx["start_header"];
  let end_header_0 = ctx["end_header"];
  let eot_0 = ctx["eot"];
  let system_token_0 = ctx["system_token"];
  let user_token_0 = ctx["user_token"];
  let assistant_token_0 = ctx["assistant_token"];
  let tool_token_0 = ctx["tool_token"];
  let system_content_0 = ctx["system_content"];
  let messages_0 = ctx["messages"];
  let tools_0 = ctx["tools"] ?? null;
  let content_0 = ctx["content"];
  let fn_0 = ctx["fn"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  bos_0 = "<|begin_of_text|>";
  enable_thinking_0 = true;
  system_start_header_0 = "<|start_header_id|>";
  system_end_header_0 = "<|end_header_id|>\n\n";
  start_header_0 = "<|start_header_id|>";
  end_header_0 = "<|end_header_id|>\n\n";
  eot_0 = "<|eot_id|>";
  system_token_0 = "system";
  user_token_0 = "user";
  assistant_token_0 = "assistant";
  tool_token_0 = "tool";
  out.push(J.str(J.concat(bos_0, system_start_header_0, system_token_0, system_end_header_0)));
  if (J.t(((v) => J.t(v) ? (!J.eq(J.attr(J.item(messages_0, 0), "content"), "")) : v)((J.eq(J.attr(J.item(messages_0, 0), "role"), "system"))))) {
    system_content_0 = J.attr(J.item(messages_0, 0), "content");
    if (J.t((J.has("/no_think", system_content_0)))) {
      system_content_0 = J.filter("trim", J.call(J.attr(system_content_0, "replace"), "/no_think", ""));
      enable_thinking_0 = false;
    } else if (J.t((J.has("/think", system_content_0)))) {
      system_content_0 = J.filter("trim", J.call(J.attr(system_content_0, "replace"), "/think", ""));
      enable_thinking_0 = true;
    }
    out.push(J.str(J.add(system_content_0, "\n\n")));
  }
  if (J.t(tools_0)) {
    out.push(J.str("You can use the following tools to assist the user if required:\n<AVAILABLE_TOOLS>["));
    {
      let _items1 = J.iter(tools_0);
      const _loop1 = J.loop(_items1);
      for (let _i1 = 0; _i1 < _items1.length; _i1++) {
        _loop1.index0 = _i1; const _it1 = _items1[_i1]; let tool_1 = _it1;
        out.push(J.str(J.filter("tojson", (J.t((J.attr(tool_1, "function") !== undefined)) ? J.attr(tool_1, "function") : tool_1))), J.str((J.t(!J.t(J.attr(_loop1, "last"))) ? ", " : "")));
      }
    }
    out.push(J.str("]</AVAILABLE_TOOLS>\n\nIf you decide to call any tool(s), use the following format:\n<TOOLCALL>[{{\"name\": \"tool_name1\", \"arguments\": \"tool_args1\"}}, {{\"name\": \"tool_name2\", \"arguments\": \"tool_args2\"}}]</TOOLCALL>\n\nResponse from tool(s) will be returned in this format:\n<TOOL_RESPONSE>[{{\"response\": \"tool_response1\"}}, {{\"response\": \"tool_response2\"}}]</TOOL_RESPONSE>\n\nBased on the results returned by the tool(s), you can call additional tools if needed, correct tool calls if any errors are found, or just respond with the answer to the user."));
  }
  out.push(J.str(eot_0));
  {
    let _items2 = J.iter(messages_0);
    const _loop2 = J.loop(_items2);
    for (let _i2 = 0; _i2 < _items2.length; _i2++) {
      _loop2.index0 = _i2; const _it2 = _items2[_i2]; let message_2 = _it2;
      let content_3 = content_0;
      if (J.t((J.eq(J.attr(message_2, "role"), user_token_0)))) {
        out.push(J.str(J.concat(start_header_0, user_token_0, end_header_0)), J.str(J.attr(message_2, "content")), J.str(eot_0));
      } else if (J.t((J.eq(J.attr(message_2, "role"), assistant_token_0)))) {
        if (J.t((J.has("</think>", J.attr(message_2, "content"))))) {
          content_3 = J.call(J.attr(J.item(J.call(J.attr(J.attr(message_2, "content"), "split"), "</think>"), J.neg(1)), "lstrip"));
        } else {
          content_3 = J.attr(message_2, "content");
        }
        out.push(J.str(J.concat(start_header_0, assistant_token_0, end_header_0)), J.str(content_3));
        if (J.t(J.attr(message_2, "tool_calls"))) {
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
        out.push(J.str(eot_0));
      } else if (J.t((J.eq(J.attr(message_2, "role"), tool_token_0)))) {
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.sub(J.attr(_loop2, "index0"), 1)), "role"), tool_token_0)))(J.attr(_loop2, "first")))) {
          out.push(J.str(J.concat(start_header_0, tool_token_0, end_header_0)), J.str("<TOOL_RESPONSE>["));
        }
        out.push(J.str(J.attr(message_2, "content")), J.str((J.t(((v) => J.t(v) ? (J.eq(J.attr(J.item(messages_0, J.add(J.attr(_loop2, "index0"), 1)), "role"), tool_token_0)) : v)(!J.t(J.attr(_loop2, "last")))) ? ", " : "")));
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.add(J.attr(_loop2, "index0"), 1)), "role"), tool_token_0)))(J.attr(_loop2, "last")))) {
          out.push(J.str("]</TOOL_RESPONSE>"), J.str(eot_0));
        }
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str(J.concat(start_header_0, assistant_token_0, end_header_0)));
    if (J.t(!J.t(enable_thinking_0))) {
      out.push(J.str("<think>\n\n</think>\n\n"));
    }
  }
  return out.join("");
}
