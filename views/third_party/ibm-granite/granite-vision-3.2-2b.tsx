// Converted from the chat template of ibm-granite/granite-vision-3.2-2b by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let tools_0 = ctx["tools"] ?? null;
  let messages_0 = ctx["messages"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  if (J.t(tools_0)) {
    out.push(J.str("<|start_of_role|>available_tools<|end_of_role|>\n"));
    {
      let _items1 = J.iter(tools_0);
      const _loop1 = J.loop(_items1);
      for (let _i1 = 0; _i1 < _items1.length; _i1++) {
        _loop1.index0 = _i1; const _it1 = _items1[_i1]; let tool_1 = _it1;
        out.push(J.str(J.filter("tojson", tool_1, J.kw({"indent": 4}))));
        if (J.t(!J.t(J.attr(_loop1, "last")))) {
          out.push(J.str("\n\n"));
        }
      }
    }
    out.push(J.str("<|end_of_text|>\n"));
  }
  {
    let _items2 = J.iter(messages_0);
    _items2 = _items2.filter(_it2 => { let message_2 = _it2; return J.t((J.eq(J.item(message_2, "role"), "system"))); });
    const _loop2 = J.loop(_items2);
    if (_items2.length === 0) {
      out.push("<|system|>\nA chat between a curious user and an artificial intelligence assistant. The assistant gives helpful, detailed, and polite answers to the user's questions.\n");
    }
    for (let _i2 = 0; _i2 < _items2.length; _i2++) {
      _loop2.index0 = _i2; const _it2 = _items2[_i2]; let message_2 = _it2;
    }
  }
  {
    let _items3 = J.iter(messages_0);
    const _loop3 = J.loop(_items3);
    for (let _i3 = 0; _i3 < _items3.length; _i3++) {
      _loop3.index0 = _i3; const _it3 = _items3[_i3]; let message_3 = _it3;
      if (J.t((J.eq(J.item(message_3, "role"), "system")))) {
        out.push(J.str(J.add(J.add("<|system|>\n", J.item(message_3, "content")), "\n")));
      } else if (J.t((J.eq(J.item(message_3, "role"), "user")))) {
        out.push(J.str(J.add(J.add("<|user|>\n", J.item(message_3, "content")), "\n")));
      } else if (J.t((J.eq(J.item(message_3, "role"), "assistant")))) {
        out.push(J.str(J.add(J.add("<|assistant|>\n", J.item(message_3, "content")), "<|end_of_text|>")));
      } else if (J.t((J.eq(J.item(message_3, "role"), "assistant_tool_call")))) {
        out.push(J.str(J.add(J.add("<|start_of_role|>assistant<|end_of_role|><|tool_call|>", J.item(message_3, "content")), "<|end_of_text|>\n")));
      } else if (J.t((J.eq(J.item(message_3, "role"), "tool_response")))) {
        out.push(J.str(J.add(J.add("<|start_of_role|>tool_response<|end_of_role|>", J.item(message_3, "content")), "<|end_of_text|>\n")));
      }
      if (J.t(((v) => J.t(v) ? add_generation_prompt_0 : v)(J.attr(_loop3, "last")))) {
        out.push(J.str("<|assistant|>\n"));
      }
    }
  }
  return out.join("");
}
