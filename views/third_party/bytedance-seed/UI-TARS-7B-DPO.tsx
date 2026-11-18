// Converted from the chat template of ByteDance-Seed/UI-TARS-7B-DPO by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let messages_0 = ctx["messages"];
  let eos_token_0 = ctx["eos_token"];
  if (J.t(!J.t((add_generation_prompt_0 !== undefined)))) {
    add_generation_prompt_0 = false;
  }
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      out.push(J.str(J.add(J.add(J.add(J.add(J.call(J.attr(J.item(message_1, "role"), "title")), ": "), J.filter("trim", J.item(message_1, "content"))), eos_token_0), "\n")));
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("Assistant: "));
  }
  return out.join("");
}
