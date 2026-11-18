// Converted from the chat template of 01-ai/Yi-1.5-9B-Chat by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let system_message_0 = ctx["system_message"];
  let messages_0 = ctx["messages"];
  let content_0 = ctx["content"];
  if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
    system_message_0 = J.item(J.item(messages_0, 0), "content");
  }
  if (J.t((system_message_0 !== undefined))) {
    out.push(J.str(system_message_0));
  }
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      let content_2 = content_0;
      content_2 = J.item(message_1, "content");
      if (J.t((J.eq(J.item(message_1, "role"), "user")))) {
        out.push(J.str(J.add(J.add("<|im_start|>user\n", content_2), "<|im_end|>\n<|im_start|>assistant\n")));
      } else if (J.t((J.eq(J.item(message_1, "role"), "assistant")))) {
        out.push(J.str(J.add(J.add(content_2, "<|im_end|>"), "\n")));
      }
    }
  }
  return out.join("");
}
