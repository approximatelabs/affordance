// Converted from the chat template of utter-project/EuroLLM-1.7B-Instruct by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let messages_0 = ctx["messages"];
  if (J.t(!J.t((add_generation_prompt_0 !== undefined)))) {
    add_generation_prompt_0 = false;
  }
  out.push("<|im_start|>system\n");
  if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
    out.push(J.str(J.item(J.item(messages_0, 0), "content")));
  }
  out.push("<|im_end|>\n");
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      if (J.t((!J.eq(J.item(message_1, "role"), "system")))) {
        out.push("<|im_start|>", J.str(J.item(message_1, "role")), "\n", J.str(J.item(message_1, "content")), "<|im_end|>\n");
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push("<|im_start|>assistant\n");
  }
  return out.join("");
}
