// Converted from the chat template of project-numina/NuminaMath-7B-TIR by tools/jinja2tsx.py. Regenerate it; do not edit.
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
        out.push(J.str(""));
      } else if (J.t((J.eq(J.item(message_1, "role"), "user")))) {
        out.push(J.str(J.add(J.add("### Problem: ", J.item(message_1, "content")), "\n")));
      } else if (J.t((J.eq(J.item(message_1, "role"), "assistant")))) {
        out.push(J.str(J.add(J.add("### Solution: ", J.item(message_1, "content")), "\n")));
      }
      if (J.t(((v) => J.t(v) ? add_generation_prompt_0 : v)(((v) => J.t(v) ? (J.eq(J.item(message_1, "role"), "user")) : v)(J.attr(_loop1, "last"))))) {
        out.push(J.str("### Solution: "));
      }
    }
  }
  return out.join("");
}
