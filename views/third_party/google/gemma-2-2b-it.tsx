// Converted from the chat template of google/gemma-2-2b-it by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let bos_token_0 = ctx["bos_token"];
  let messages_0 = ctx["messages"];
  let role_0 = ctx["role"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  out.push(J.str(bos_token_0));
  if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
    out.push(J.str(J.call(J.raise_exception, "System role not supported")));
  }
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      let role_2 = role_0;
      if (J.t((!J.eq((J.eq(J.item(message_1, "role"), "user")), (J.eq(J.mod(J.attr(_loop1, "index0"), 2), 0)))))) {
        out.push(J.str(J.call(J.raise_exception, "Conversation roles must alternate user/assistant/user/assistant/...")));
      }
      if (J.t((J.eq(J.item(message_1, "role"), "assistant")))) {
        role_2 = "model";
      } else {
        role_2 = J.item(message_1, "role");
      }
      out.push(J.str(J.add(J.add(J.add(J.add("<start_of_turn>", role_2), "\n"), J.filter("trim", J.item(message_1, "content"))), "<end_of_turn>\n")));
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<start_of_turn>model\n"));
  }
  return out.join("");
}
