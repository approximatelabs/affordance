// Converted from the chat template of baidu/ERNIE-4.5-21B-A3B-PT by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let cls_token_0 = ctx["cls_token"];
  let sep_token_0 = ctx["sep_token"];
  let messages_0 = ctx["messages"];
  if (J.t(!J.t((add_generation_prompt_0 !== undefined)))) {
    add_generation_prompt_0 = true;
  }
  if (J.t(!J.t((cls_token_0 !== undefined)))) {
    cls_token_0 = "<|begin_of_sentence|>";
  }
  if (J.t(!J.t((sep_token_0 !== undefined)))) {
    sep_token_0 = "<|end_of_sentence|>";
  }
  out.push(J.str(cls_token_0));
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      if (J.t((J.eq(J.item(message_1, "role"), "user")))) {
        out.push(J.str(J.add(J.add("User: ", J.item(message_1, "content")), "\n")));
      } else if (J.t((J.eq(J.item(message_1, "role"), "assistant")))) {
        out.push(J.str(J.add(J.add("Assistant: ", J.item(message_1, "content")), sep_token_0)));
      } else if (J.t((J.eq(J.item(message_1, "role"), "system")))) {
        out.push(J.str(J.add(J.item(message_1, "content"), "\n")));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("Assistant: "));
  }
  return out.join("");
}
