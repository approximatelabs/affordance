// Converted from the chat template of microsoft/Phi-3.5-vision-instruct by tools/jinja2tsx.py. Regenerate it; do not edit.
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
      out.push(J.str(J.add(J.add(J.add(J.add(J.add("<|", J.item(message_1, "role")), "|>"), "\n"), J.item(message_1, "content")), "<|end|>\n")));
    }
  }
  if (J.t(((v) => J.t(v) ? (!J.eq(J.item(J.item(messages_0, J.neg(1)), "role"), "assistant")) : v)(add_generation_prompt_0))) {
    out.push(J.str("<|assistant|>\n"));
  }
  return out.join("");
}
