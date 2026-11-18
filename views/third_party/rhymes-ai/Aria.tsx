// Converted from the chat template of rhymes-ai/Aria by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let messages_0 = ctx["messages"];
  if (J.t(!J.t((add_generation_prompt_0 !== undefined)))) {
    add_generation_prompt_0 = false;
  }
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      out.push("<|im_start|>", J.str(J.item(message_1, "role")), "\n");
      if (J.t(J.test("string", J.item(message_1, "content")))) {
        out.push(J.str(J.item(message_1, "content")));
      } else if (J.t(J.test("iterable", J.item(message_1, "content")))) {
        {
          let _items2 = J.iter(J.item(message_1, "content"));
          const _loop2 = J.loop(_items2);
          for (let _i2 = 0; _i2 < _items2.length; _i2++) {
            _loop2.index0 = _i2; const _it2 = _items2[_i2]; let item_2 = _it2;
            if (J.t((J.eq(J.item(item_2, "type"), "text")))) {
              out.push(J.str(J.item(item_2, "text")));
            } else if (J.t((J.eq(J.item(item_2, "type"), "image")))) {
              out.push("<fim_prefix><|img|><fim_suffix>");
            }
          }
        }
      }
      out.push("<|im_end|>\n");
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push("<|im_start|>assistant\n");
  }
  return out.join("");
}
