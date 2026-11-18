// Converted from the chat template of ByteDance/Ouro-2.6B-Thinking by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let messages_0 = ctx["messages"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let enable_thinking_0 = ctx["enable_thinking"];
  if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
    out.push(J.str(J.add(J.add("<|im_start|>system\n", J.item(J.item(messages_0, 0), "content")), "<|im_end|>\n")));
  } else {
    out.push(J.str("<|im_start|>system\nYou are a helpful assistant.<|im_end|>\n"));
  }
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      if (J.t(((v) => J.t(v) ? J.attr(_loop1, "first") : v)((J.eq(J.attr(message_1, "role"), "system"))))) {
      } else {
        out.push(J.str(J.add(J.add(J.add(J.add(J.add("<|im_start|>", J.item(message_1, "role")), "\n"), J.item(message_1, "content")), "<|im_end|>"), "\n")));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|im_start|>assistant\n"));
    if (J.t(((v) => J.t(v) ? J.test("true", enable_thinking_0) : v)((enable_thinking_0 !== undefined)))) {
      out.push(J.str("<think>\n"));
    }
  }
  return out.join("");
}
