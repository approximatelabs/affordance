// Converted from the chat template of ByteDance-Seed/Seed-Coder-8B-Instruct by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let has_system_0 = ctx["has_system"];
  let messages_0 = ctx["messages"];
  let bos_token_0 = ctx["bos_token"];
  let eos_token_0 = ctx["eos_token"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  has_system_0 = (J.eq(J.item(J.item(messages_0, 0), "role"), "system"));
  if (J.t(!J.t(has_system_0))) {
    out.push(J.str(J.add(J.add(bos_token_0, "system\nYou are an AI programming assistant, utilizing the Seed-Coder model, developed by ByteDance Seed, and you only answer questions related to computer science. For politically sensitive questions, security and privacy issues, and other non-computer science questions, you will refuse to answer.\n\n"), eos_token_0)));
  }
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      out.push(J.str(J.add(J.add(J.add(J.add(bos_token_0, J.item(message_1, "role")), "\n"), J.filter("trim", J.item(message_1, "content"))), eos_token_0)));
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str(J.add(bos_token_0, "assistant\n")));
  }
  return out.join("");
}
