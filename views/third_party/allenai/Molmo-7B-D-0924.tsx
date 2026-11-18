// Converted from the chat template of allenai/Molmo-7B-D-0924 by tools/jinja2tsx.py. Regenerate it; do not edit.
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
      if (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? (!J.eq(J.call(J.attr(J.item(message_1, "role"), "lower")), "assistant")) : v)((J.eq(J.mod(J.attr(_loop1, "index"), 2), 0))))(((v) => J.t(v) ? (!J.eq(J.item(message_1, "role"), "user")) : v)((J.eq(J.mod(J.attr(_loop1, "index"), 2), 1)))))) {
        out.push(J.str(J.call(J.raise_exception, "Conversation roles must alternate user/assistant/user/assistant/...")));
      }
      out.push(J.str(J.add(J.add(J.call(J.attr(J.item(message_1, "role"), "capitalize")), ": "), J.item(message_1, "content"))));
      if (J.t(!J.t(J.attr(_loop1, "last")))) {
        out.push(J.str(" "));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str(" Assistant:"));
  }
  return out.join("");
}
