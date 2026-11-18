// Converted from the chat template of PleIAs/Baguettotron by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let messages_0 = ctx["messages"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let m_1 = _it1;
      out.push("<|im_start|>", J.str(J.item(m_1, "role")), "\n", J.str(J.item(m_1, "content")), "<|im_end|>\n");
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push("<|im_start|>assistant\n<think>\n");
  }
  return out.join("");
}
