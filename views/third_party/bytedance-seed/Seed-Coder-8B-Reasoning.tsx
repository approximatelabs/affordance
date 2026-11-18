// Converted from the chat template of ByteDance-Seed/Seed-Coder-8B-Reasoning by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let messages_0 = ctx["messages"];
  let role_0 = ctx["role"];
  let bos_token_0 = ctx["bos_token"];
  let eos_token_0 = ctx["eos_token"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
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
      role_2 = J.item(message_1, "role");
      out.push(J.str(J.add(J.add(J.add(J.add(bos_token_0, role_2), "\n"), J.filter("trim", J.item(message_1, "content"))), eos_token_0)));
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str(J.add(bos_token_0, "assistant\n")));
  }
  return out.join("");
}
