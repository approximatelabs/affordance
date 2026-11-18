// Converted from the chat template of LGAI-EXAONE/EXAONE-3.5-7.8B-Instruct by tools/jinja2tsx.py. Regenerate it; do not edit.
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
      if (J.t(((v) => J.t(v) ? (!J.eq(J.item(message_1, "role"), "system")) : v)(J.attr(_loop1, "first")))) {
        out.push(J.str("[|system|][|endofturn|]\n"));
      }
      out.push(J.str(J.add(J.add(J.add("[|", J.item(message_1, "role")), "|]"), J.item(message_1, "content"))));
      if (J.t((J.eq(J.item(message_1, "role"), "user")))) {
        out.push(J.str("\n"));
      } else {
        out.push(J.str("[|endofturn|]\n"));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("[|assistant|]"));
  }
  return out.join("");
}
