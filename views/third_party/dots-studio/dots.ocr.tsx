// Converted from the chat template of dots-studio/dots.ocr by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let messages_0 = ctx["messages"];
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let m_1 = _it1;
      if (J.t((J.eq(J.attr(m_1, "role"), "system")))) {
        out.push(J.str(J.add(J.add("<|system|>", J.attr(m_1, "content")), "<|endofsystem|>\n")));
      } else if (J.t((J.eq(J.attr(m_1, "role"), "user")))) {
        out.push(J.str(J.add(J.add("<|user|>", J.attr(m_1, "content")), "<|endofuser|>")));
      } else if (J.t((J.eq(J.attr(m_1, "role"), "assistant")))) {
        out.push(J.str(J.add("<|assistant|>", J.attr(m_1, "content"))));
        if (J.t(!J.t(J.attr(_loop1, "last")))) {
          out.push(J.str("<|endofassistant|>"));
        }
      }
    }
  }
  if (J.t((!J.eq(J.attr(J.item(messages_0, J.neg(1)), "role"), "assistant")))) {
    out.push(J.str("<|assistant|>"));
  }
  return out.join("");
}
