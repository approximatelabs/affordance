// Converted from the chat template of Deci/DeciLM-7B-instruct by tools/jinja2tsx.py. Regenerate it; do not edit.
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
      if (J.t((J.eq(J.item(message_1, "role"), "user")))) {
        out.push(J.str(J.add("### User:\n", J.item(message_1, "content"))), "\n");
      } else if (J.t((J.eq(J.item(message_1, "role"), "system")))) {
        out.push(J.str(J.add("### System:\n", J.item(message_1, "content"))), "\n");
      } else if (J.t((J.eq(J.item(message_1, "role"), "assistant")))) {
        out.push(J.str(J.add("### Assistant:\n", J.item(message_1, "content"))), "\n");
      }
      if (J.t(((v) => J.t(v) ? add_generation_prompt_0 : v)(J.attr(_loop1, "last")))) {
        out.push(J.str("### Assistant:"), "\n");
      }
    }
  }
  return out.join("");
}
