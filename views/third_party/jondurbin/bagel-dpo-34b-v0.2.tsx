// Converted from the chat template of jondurbin/bagel-dpo-34b-v0.2 by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let messages_0 = ctx["messages"];
  let bos_token_0 = ctx["bos_token"];
  let eos_token_0 = ctx["eos_token"];
  {
    let _items1 = J.iter(J.call(J.range, 0, J.filter("length", messages_0)));
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let idx_1 = _it1;
      if (J.t((J.eq(J.item(J.item(messages_0, idx_1), "role"), "user")))) {
        if (J.t((J.gt(idx_1, 1)))) {
          out.push(J.str(J.add(J.add(J.add(bos_token_0, "[INST] "), J.item(J.item(messages_0, idx_1), "content")), " [/INST]")));
        } else {
          out.push(J.str(J.add(J.item(J.item(messages_0, idx_1), "content"), " [/INST]")));
        }
      } else if (J.t((J.eq(J.item(J.item(messages_0, idx_1), "role"), "system")))) {
        out.push(J.str(J.add(J.add("[INST] <<SYS>>\n", J.item(J.item(messages_0, idx_1), "content")), "\n<</SYS>>\n\n")));
      } else if (J.t((J.eq(J.item(J.item(messages_0, idx_1), "role"), "assistant")))) {
        out.push(J.str(J.add(J.add(J.add(" ", J.item(J.item(messages_0, idx_1), "content")), " "), eos_token_0)));
      }
    }
  }
  return out.join("");
}
