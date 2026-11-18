// Converted from the chat template of microsoft/phi-4 by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let messages_0 = ctx["messages"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      if (J.t((J.eq(J.item(message_1, "role"), "system")))) {
        out.push(J.str(J.add(J.add("<|im_start|>system<|im_sep|>", J.item(message_1, "content")), "<|im_end|>")));
      } else if (J.t((J.eq(J.item(message_1, "role"), "user")))) {
        out.push(J.str(J.add(J.add("<|im_start|>user<|im_sep|>", J.item(message_1, "content")), "<|im_end|>")));
      } else if (J.t((J.eq(J.item(message_1, "role"), "assistant")))) {
        out.push(J.str(J.add(J.add("<|im_start|>assistant<|im_sep|>", J.item(message_1, "content")), "<|im_end|>")));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|im_start|>assistant<|im_sep|>"));
  }
  return out.join("");
}
