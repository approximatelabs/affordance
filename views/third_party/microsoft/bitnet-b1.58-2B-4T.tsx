// Converted from the chat template of microsoft/bitnet-b1.58-2B-4T by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let loop_messages_0 = ctx["loop_messages"];
  let messages_0 = ctx["messages"];
  let content_0 = ctx["content"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  loop_messages_0 = messages_0;
  {
    let _items1 = J.iter(loop_messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      let content_2 = content_0;
      content_2 = J.add(J.add(J.add(J.filter("capitalize", J.item(message_1, "role")), ": "), J.filter("trim", J.item(message_1, "content"))), "<|eot_id|>");
      out.push(J.str(content_2));
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("Assistant: "));
  }
  return out.join("");
}
