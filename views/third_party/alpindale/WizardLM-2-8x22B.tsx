// Converted from the chat template of alpindale/WizardLM-2-8x22B by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let loop_messages_0 = ctx["loop_messages"];
  let messages_0 = ctx["messages"];
  let eos_token_0 = ctx["eos_token"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
    loop_messages_0 = J.slice(messages_0, 1, undefined, undefined);
    out.push(J.str(J.call(J.attr(J.item(J.item(messages_0, 0), "content"), "strip"))));
  } else {
    loop_messages_0 = messages_0;
    out.push(J.str("A chat between a curious user and an artificial intelligence assistant. The assistant gives helpful, detailed, and polite answers to the user's questions."));
  }
  {
    let _items1 = J.iter(loop_messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      if (J.t((J.eq(J.attr(_loop1, "index0"), 0)))) {
        if (J.t(((v) => J.t(v) ? v : (J.eq(J.item(message_1, "role"), "user")))((J.eq(J.item(message_1, "role"), "system"))))) {
          out.push(J.str(J.add(" USER: ", J.call(J.attr(J.item(message_1, "content"), "strip")))));
        } else {
          out.push(J.str(J.add(J.add(" ASSISTANT: ", J.call(J.attr(J.item(message_1, "content"), "strip"))), eos_token_0)));
        }
      } else {
        if (J.t(((v) => J.t(v) ? v : (J.eq(J.item(message_1, "role"), "user")))((J.eq(J.item(message_1, "role"), "system"))))) {
          out.push(J.str(J.add("\nUSER: ", J.call(J.attr(J.item(message_1, "content"), "strip")))));
        } else {
          out.push(J.str(J.add(J.add(" ASSISTANT: ", J.call(J.attr(J.item(message_1, "content"), "strip"))), eos_token_0)));
        }
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str(" ASSISTANT:"));
  }
  return out.join("");
}
