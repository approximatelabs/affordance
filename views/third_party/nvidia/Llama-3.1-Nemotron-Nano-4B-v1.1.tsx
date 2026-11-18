// Converted from the chat template of nvidia/Llama-3.1-Nemotron-Nano-4B-v1.1 by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let tools_0 = ctx["tools"] ?? null;
  let messages_0 = ctx["messages"];
  let content_0 = ctx["content"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  if (J.t(tools_0)) {
    out.push(J.str("<|begin_of_text|><|start_header_id|>system<|end_header_id|>\n\n"));
    if (J.t(((v) => J.t(v) ? (!J.eq(J.attr(J.item(messages_0, 0), "content"), "")) : v)((J.eq(J.attr(J.item(messages_0, 0), "role"), "system"))))) {
      out.push(J.str(J.add(J.attr(J.item(messages_0, 0), "content"), "\n\n")));
    } else {
      out.push(J.str("detailed thinking off\n\n"));
    }
    out.push(J.str("<AVAILABLE_TOOLS>["));
    {
      let _items1 = J.iter(tools_0);
      const _loop1 = J.loop(_items1);
      for (let _i1 = 0; _i1 < _items1.length; _i1++) {
        _loop1.index0 = _i1; const _it1 = _items1[_i1]; let tool_1 = _it1;
        out.push(J.str(J.filter("tojson", (J.t((J.attr(tool_1, "function") !== undefined)) ? J.attr(tool_1, "function") : tool_1))), J.str((J.t(!J.t(J.attr(_loop1, "last"))) ? ", " : "")));
      }
    }
    out.push(J.str("]</AVAILABLE_TOOLS>"), J.str("<|eot_id|>"));
  } else {
    out.push(J.str("<|begin_of_text|><|start_header_id|>system<|end_header_id|>\n\n"));
    if (J.t(((v) => J.t(v) ? (!J.eq(J.attr(J.item(messages_0, 0), "content"), "")) : v)((J.eq(J.attr(J.item(messages_0, 0), "role"), "system"))))) {
      out.push(J.str(J.attr(J.item(messages_0, 0), "content")));
    } else {
      out.push(J.str("detailed thinking off"));
    }
    out.push(J.str("<|eot_id|>"));
  }
  {
    let _items2 = J.iter(messages_0);
    const _loop2 = J.loop(_items2);
    for (let _i2 = 0; _i2 < _items2.length; _i2++) {
      _loop2.index0 = _i2; const _it2 = _items2[_i2]; let message_2 = _it2;
      let content_3 = content_0;
      if (J.t((J.eq(J.attr(message_2, "role"), "user")))) {
        out.push(J.str(J.add(J.add("<|start_header_id|>user<|end_header_id|>\n\n", J.attr(message_2, "content")), "<|eot_id|>")));
      } else if (J.t((J.eq(J.attr(message_2, "role"), "assistant")))) {
        content_3 = J.attr(message_2, "content");
        out.push(J.str(J.add("<|start_header_id|>assistant<|end_header_id|>\n\n", content_3)));
        if (J.t(J.attr(message_2, "tool_calls"))) {
          out.push(J.str("<TOOLCALL>["));
          {
            let _items3 = J.iter(J.attr(message_2, "tool_calls"));
            const _loop3 = J.loop(_items3);
            for (let _i3 = 0; _i3 < _items3.length; _i3++) {
              _loop3.index0 = _i3; const _it3 = _items3[_i3]; let tool_call_4 = _it3;
              if (J.t(J.attr(tool_call_4, "function"))) {
                tool_call_4 = J.attr(tool_call_4, "function");
              }
              out.push(J.str("{\"name\": \""), J.str(J.attr(tool_call_4, "name")), J.str("\", \"arguments\": "));
              if (J.t(J.test("string", J.attr(tool_call_4, "arguments")))) {
                out.push(J.str(J.attr(tool_call_4, "arguments")));
              } else {
                out.push(J.str(J.filter("tojson", J.attr(tool_call_4, "arguments"))));
              }
              out.push(J.str((J.t(!J.t(J.attr(_loop3, "last"))) ? ", " : "")));
            }
          }
          out.push(J.str("]</TOOLCALL>"));
        }
        out.push(J.str("<|eot_id|>"));
      } else if (J.t((J.eq(J.attr(message_2, "role"), "tool")))) {
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.sub(J.attr(_loop2, "index0"), 1)), "role"), "tool")))(J.attr(_loop2, "first")))) {
          out.push(J.str("<|start_header_id|>user<|end_header_id|>\n\n"), J.str("<TOOL_RESPONSE>["));
        }
        out.push(J.str(J.attr(message_2, "content")), J.str((J.t(((v) => J.t(v) ? (J.eq(J.attr(J.item(messages_0, J.add(J.attr(_loop2, "index0"), 1)), "role"), "tool")) : v)(!J.t(J.attr(_loop2, "last")))) ? ", " : "")));
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.add(J.attr(_loop2, "index0"), 1)), "role"), "tool")))(J.attr(_loop2, "last")))) {
          out.push(J.str("]</TOOL_RESPONSE>"), J.str("<|eot_id|>"));
        }
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|start_header_id|>assistant<|end_header_id|>\n\n"));
  }
  return out.join("");
}
