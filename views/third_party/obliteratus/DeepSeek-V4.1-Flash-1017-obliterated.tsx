// Converted from the chat template of OBLITERATUS/DeepSeek-V4.1-Flash-1017-obliterated by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let bos_0 = ctx["bos"];
  let eos_0 = ctx["eos"];
  let system_token_0 = ctx["system_token"];
  let user_token_0 = ctx["user_token"];
  let assistant_token_0 = ctx["assistant_token"];
  let messages_0 = ctx["messages"];
  if (J.t(!J.t((add_generation_prompt_0 !== undefined)))) {
    add_generation_prompt_0 = false;
  }
  bos_0 = "<\uff5cbegin\u2581of\u2581sentence\uff5c>";
  eos_0 = "<\uff5cend\u2581of\u2581sentence\uff5c>";
  system_token_0 = "<\uff5cSystem\uff5c>";
  user_token_0 = "<\uff5cUser\uff5c>";
  assistant_token_0 = "<\uff5cAssistant\uff5c>";
  out.push(J.str(bos_0));
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      if (J.t((J.eq(J.item(message_1, "role"), "system")))) {
        if (J.t(J.attr(_loop1, "first"))) {
          out.push(J.str(system_token_0));
        }
        out.push(J.str(J.item(message_1, "content")));
      } else if (J.t((J.eq(J.item(message_1, "role"), "user")))) {
        out.push(J.str(user_token_0), J.str(J.item(message_1, "content")));
      } else if (J.t((J.eq(J.item(message_1, "role"), "assistant")))) {
        out.push(J.str(assistant_token_0), "</think>", J.str(J.item(message_1, "content")), J.str(eos_0));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str(assistant_token_0), "</think>");
  }
  return out.join("");
}
