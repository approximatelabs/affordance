// Converted from the chat template of qnguyen3/nanoLLaVA by tools/jinja2tsx.py. Regenerate it; do not edit.
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
        out.push(J.str("<|im_start|>system\nAnswer the questions.<|im_end|>"));
      }
      out.push(J.str(J.add(J.add(J.add(J.add("<|im_start|>", J.item(message_1, "role")), "\n"), J.item(message_1, "content")), "<|im_end|>")));
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|im_start|>assistant\n"));
  }
  return out.join("");
}
