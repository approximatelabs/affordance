// Converted from the chat template of nvidia/Llama-3.1-Nemotron-Nano-8B-v1 by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let system_message_0 = ctx["system_message"];
  let messages_0 = ctx["messages"];
  let tools_0 = ctx["tools"] ?? null;
  let tool_response_0 = ctx["tool_response"];
  let tool_calls_0 = ctx["tool_calls"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
    system_message_0 = J.filter("trim", J.item(J.item(messages_0, 0), "content"));
    messages_0 = J.slice(messages_0, 1, undefined, undefined);
  } else {
    system_message_0 = "";
  }
  if (J.t(!J.t((tools_0 === null)))) {
    out.push(J.str(J.add(J.add("<|begin_of_text|><|start_header_id|>system<|end_header_id|>", "\n\n"), system_message_0)), J.str((J.t(system_message_0) ? "\n\n" : "")), J.str("<AVAILABLE_TOOLS>["));
    {
      let _items1 = J.iter(tools_0);
      const _loop1 = J.loop(_items1);
      for (let _i1 = 0; _i1 < _items1.length; _i1++) {
        _loop1.index0 = _i1; const _it1 = _items1[_i1]; let t_1 = _it1;
        out.push(J.str(J.filter("tojson", (J.t((J.attr(t_1, "function") !== undefined)) ? J.attr(t_1, "function") : t_1))), J.str((J.t(!J.t(J.attr(_loop1, "last"))) ? ", " : "")));
      }
    }
    out.push(J.str("]</AVAILABLE_TOOLS>"), J.str("<|eot_id|>"));
  } else {
    out.push(J.str(J.add(J.add(J.add("<|begin_of_text|><|start_header_id|>system<|end_header_id|>", "\n\n"), system_message_0), "<|eot_id|>")));
  }
  {
    let _items2 = J.iter(messages_0);
    const _loop2 = J.loop(_items2);
    for (let _i2 = 0; _i2 < _items2.length; _i2++) {
      _loop2.index0 = _i2; const _it2 = _items2[_i2]; let message_2 = _it2;
      let tool_response_3 = tool_response_0;
      let tool_calls_4 = tool_calls_0;
      if (J.t((!J.eq((J.has(J.item(message_2, "role"), ["user", "tool"])), (J.eq(J.mod(J.attr(_loop2, "index0"), 2), 0)))))) {
        out.push(J.str(J.call(J.raise_exception, "Conversation roles must alternate between user/tool and assistant")));
      } else if (J.t((J.eq(J.item(message_2, "role"), "user")))) {
        out.push(J.str(J.add(J.add(J.add("<|start_header_id|>user<|end_header_id|>", "\n\n"), J.filter("trim", J.item(message_2, "content"))), "<|eot_id|>")));
      } else if (J.t((J.eq(J.item(message_2, "role"), "tool")))) {
        tool_response_3 = J.add(J.add("<TOOL_RESPONSE>[", J.filter("trim", J.item(message_2, "content"))), "]</TOOL_RESPONSE>");
        out.push(J.str(J.add(J.add(J.add("<|start_header_id|>user<|end_header_id|>", "\n\n"), tool_response_3), "<|eot_id|>")));
      } else if (J.t(((v) => J.t(v) ? !J.t((J.call(J.attr(message_2, "get"), "tool_calls") === null)) : v)((J.eq(J.item(message_2, "role"), "assistant"))))) {
        tool_calls_4 = J.item(message_2, "tool_calls");
        out.push(J.str(J.add(J.add("<|start_header_id|>assistant<|end_header_id|>", "\n\n"), "<TOOLCALL>[")));
        {
          let _items3 = J.iter(tool_calls_4);
          const _loop3 = J.loop(_items3);
          for (let _i3 = 0; _i3 < _items3.length; _i3++) {
            _loop3.index0 = _i3; const _it3 = _items3[_i3]; let tool_call_5 = _it3;
            out.push(J.str(J.add(J.add(J.add(J.add(J.add("{", "\"name\": \""), J.attr(J.attr(tool_call_5, "function"), "name")), "\", \"arguments\": "), J.filter("tojson", J.attr(J.attr(tool_call_5, "function"), "arguments"))), "}")));
            if (J.t(!J.t(J.attr(_loop3, "last")))) {
              out.push(J.str(", "));
            } else {
              out.push(J.str(J.add("]</TOOLCALL>", "<|eot_id|>")));
            }
          }
        }
      } else if (J.t((J.eq(J.item(message_2, "role"), "assistant")))) {
        out.push(J.str(J.add(J.add(J.add("<|start_header_id|>assistant<|end_header_id|>", "\n\n"), J.filter("trim", J.item(message_2, "content"))), "<|eot_id|>")));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str(J.add("<|start_header_id|>assistant<|end_header_id|>", "\n\n")));
  }
  return out.join("");
}
