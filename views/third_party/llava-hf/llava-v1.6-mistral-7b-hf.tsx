// Converted from the chat template of llava-hf/llava-v1.6-mistral-7b-hf by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let bos_token_0 = ctx["bos_token"];
  let messages_0 = ctx["messages"];
  let eos_token_0 = ctx["eos_token"];
  out.push(J.str(bos_token_0));
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      if (J.t((!J.eq((J.eq(J.item(message_1, "role"), "user")), (J.eq(J.mod(J.attr(_loop1, "index0"), 2), 0)))))) {
        out.push(J.str(J.call(J.raise_exception, "Conversation roles must alternate user/assistant/user/assistant/...")));
      }
      if (J.t((J.eq(J.item(message_1, "role"), "user")))) {
        out.push(J.str(J.add(J.add("[INST] ", J.item(message_1, "content")), " [/INST]")));
      } else if (J.t((J.eq(J.item(message_1, "role"), "assistant")))) {
        out.push(J.str(J.add(J.item(message_1, "content"), eos_token_0)));
      } else {
        out.push(J.str(J.call(J.raise_exception, "Only user and assistant roles are supported!")));
      }
    }
  }
  return out.join("");
}
