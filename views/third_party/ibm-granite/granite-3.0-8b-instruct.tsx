// Converted from the chat template of ibm-granite/granite-3.0-8b-instruct by tools/jinja2tsx.py. Regenerate it; do not edit.
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
    const _loop2 = J.loop(_items2);
    for (let _i2 = 0; _i2 < _items2.length; _i2++) {
      _loop2.index0 = _i2; const _it2 = _items2[_i2]; let message_2 = _it2;
      if (J.t((J.eq(J.item(message_2, "role"), "system")))) {
        out.push(J.str(J.add(J.add("<|start_of_role|>system<|end_of_role|>", J.item(message_2, "content")), "<|end_of_text|>\n")));
      } else if (J.t((J.eq(J.item(message_2, "role"), "user")))) {
        out.push(J.str(J.add(J.add("<|start_of_role|>user<|end_of_role|>", J.item(message_2, "content")), "<|end_of_text|>\n")));
      } else if (J.t((J.eq(J.item(message_2, "role"), "assistant")))) {
        out.push(J.str(J.add(J.add("<|start_of_role|>assistant<|end_of_role|>", J.item(message_2, "content")), "<|end_of_text|>\n")));
      } else if (J.t((J.eq(J.item(message_2, "role"), "assistant_tool_call")))) {
        out.push(J.str(J.add(J.add("<|start_of_role|>assistant<|end_of_role|><|tool_call|>", J.item(message_2, "content")), "<|end_of_text|>\n")));
      } else if (J.t((J.eq(J.item(message_2, "role"), "tool_response")))) {
        out.push(J.str(J.add(J.add("<|start_of_role|>tool_response<|end_of_role|>", J.item(message_2, "content")), "<|end_of_text|>\n")));
      }
      if (J.t(((v) => J.t(v) ? add_generation_prompt_0 : v)(J.attr(_loop2, "last")))) {
        out.push(J.str("<|start_of_role|>assistant<|end_of_role|>"));
      }
    }
  }
  return out.join("");
}
