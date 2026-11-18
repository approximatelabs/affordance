// Converted from the chat template of mlabonne/AlphaMonarch-7B by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let messages_0 = ctx["messages"];
  let bos_token_0 = ctx["bos_token"];
  let eos_token_0 = ctx["eos_token"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      out.push(J.str(J.add(J.add(J.add(J.add(J.add(bos_token_0, J.item(message_1, "role")), "\n"), J.item(message_1, "content")), eos_token_0), "\n")));
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str(J.add(bos_token_0, "assistant\n")));
  }
  return out.join("");
}
