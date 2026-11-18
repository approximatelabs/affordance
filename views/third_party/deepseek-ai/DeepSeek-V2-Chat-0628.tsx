// Converted from the chat template of deepseek-ai/DeepSeek-V2-Chat-0628 by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let bos_token_0 = ctx["bos_token"];
  let messages_0 = ctx["messages"];
  let eos_token_0 = ctx["eos_token"];
  if (J.t(!J.t((add_generation_prompt_0 !== undefined)))) {
    add_generation_prompt_0 = false;
  }
  out.push(J.str(bos_token_0));
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      if (J.t((J.eq(J.item(message_1, "role"), "user")))) {
        out.push(J.str(J.add("<\uff5cUser\uff5c>", J.item(message_1, "content"))));
      } else if (J.t((J.eq(J.item(message_1, "role"), "assistant")))) {
        out.push(J.str(J.add(J.add("<\uff5cAssistant\uff5c>", J.item(message_1, "content")), eos_token_0)));
      } else if (J.t((J.eq(J.item(message_1, "role"), "system")))) {
        out.push(J.str(J.add(J.item(message_1, "content"), "\n\n")));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<\uff5cAssistant\uff5c>"));
  }
  return out.join("");
}
