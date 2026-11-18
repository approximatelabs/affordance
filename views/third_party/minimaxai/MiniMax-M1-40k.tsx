// Converted from the chat template of MiniMaxAI/MiniMax-M1-40k by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let ns_0 = ctx["ns"];
  let messages_0 = ctx["messages"];
  let tools_0 = ctx["tools"] ?? null;
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  out.push(J.str("<begin_of_document>"));
  ns_0 = J.call(J.namespace, J.kw({"system_prompt": ""}));
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      if (J.t((J.eq(J.item(message_1, "role"), "system")))) {
        J.setattr(ns_0, "system_prompt", J.add(J.attr(ns_0, "system_prompt"), J.item(J.item(J.item(message_1, "content"), 0), "text")));
      }
    }
  }
  if (J.t((!J.eq(J.attr(ns_0, "system_prompt"), "")))) {
    out.push(J.str(J.add(J.add("<beginning_of_sentence>system ai_setting=assistant\n", J.attr(ns_0, "system_prompt")), "<end_of_sentence>\n")));
  }
  if (J.t(tools_0)) {
    out.push(J.str("<beginning_of_sentence>system tool_setting=tools\nYou are provided with these tools:\n<tools>\n"));
    {
      let _items2 = J.iter(tools_0);
      const _loop2 = J.loop(_items2);
      for (let _i2 = 0; _i2 < _items2.length; _i2++) {
        _loop2.index0 = _i2; const _it2 = _items2[_i2]; let tool_2 = _it2;
        out.push(J.str(J.concat(J.filter("tojson", tool_2), "\n")));
      }
    }
    out.push(J.str("</tools>\n\nIf you need to call tools, please respond with <tool_calls></tool_calls> XML tags, and provide tool-name and json-object of arguments, following the format below:\n<tool_calls>\n{\"name\": <tool-name-1>, \"arguments\": <args-json-object-1>}\n...\n</tool_calls><end_of_sentence>\n"));
  }
  {
    let _items3 = J.iter(messages_0);
    const _loop3 = J.loop(_items3);
    for (let _i3 = 0; _i3 < _items3.length; _i3++) {
      _loop3.index0 = _i3; const _it3 = _items3[_i3]; let message_3 = _it3;
      if (J.t((J.eq(J.item(message_3, "role"), "user")))) {
        out.push(J.str(J.add(J.add("<beginning_of_sentence>user name=user\n", J.item(J.item(J.item(message_3, "content"), 0), "text")), "<end_of_sentence>\n")));
      } else if (J.t((J.eq(J.item(message_3, "role"), "assistant")))) {
        out.push(J.str("<beginning_of_sentence>ai name=assistant\n"));
        {
          let _items4 = J.iter(J.filter("selectattr", J.item(message_3, "content"), "type", "equalto", "text"));
          const _loop4 = J.loop(_items4);
          for (let _i4 = 0; _i4 < _items4.length; _i4++) {
            _loop4.index0 = _i4; const _it4 = _items4[_i4]; let content_4 = _it4;
            out.push(J.str(J.item(content_4, "text")));
          }
        }
        out.push(J.str("<end_of_sentence>\n"));
      } else if (J.t((J.eq(J.item(message_3, "role"), "tool")))) {
        out.push(J.str("<beginning_of_sentence>tool name=tools\n"));
        {
          let _items5 = J.iter(J.item(message_3, "content"));
          const _loop5 = J.loop(_items5);
          for (let _i5 = 0; _i5 < _items5.length; _i5++) {
            _loop5.index0 = _i5; const _it5 = _items5[_i5]; let content_5 = _it5;
            out.push(J.str(J.add(J.add(J.add(J.add(J.add("tool name: ", J.item(content_5, "name")), "\n"), "tool result: "), J.item(content_5, "text")), "\n\n")));
          }
        }
        out.push(J.str("<end_of_sentence>\n"));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<beginning_of_sentence>ai name=assistant\n"));
  }
  return out.join("");
}
