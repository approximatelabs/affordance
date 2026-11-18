// Converted from the chat template of moonshotai/Moonlight-16B-A3B-Instruct by tools/jinja2tsx.py. Regenerate it; do not edit.
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
      if (J.t(((v) => J.t(v) ? (!J.eq(J.item(J.item(messages_0, 0), "role"), "system")) : v)(J.attr(_loop1, "first")))) {
        out.push("<|im_system|>system<|im_middle|>You are a helpful assistant<|im_end|>");
      }
      if (J.t((J.eq(J.item(message_1, "role"), "system")))) {
        out.push("<|im_system|>");
      }
      if (J.t((J.eq(J.item(message_1, "role"), "user")))) {
        out.push("<|im_user|>");
      }
      if (J.t((J.eq(J.item(message_1, "role"), "assistant")))) {
        out.push("<|im_assistant|>");
      }
      out.push(J.str(J.item(message_1, "role")), "<|im_middle|>", J.str(J.item(message_1, "content")), "<|im_end|>");
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push("<|im_assistant|>assistant<|im_middle|>");
  }
  return out.join("");
}
