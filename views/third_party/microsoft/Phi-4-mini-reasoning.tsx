// Converted from the chat template of microsoft/Phi-4-mini-reasoning by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let messages_0 = ctx["messages"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let eos_token_0 = ctx["eos_token"];
  out.push(J.str("<|system|>Your name is Phi, an AI math expert developed by Microsoft."));
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      if (J.t((J.eq(J.item(message_1, "role"), "system")))) {
        out.push(" ", J.str(J.item(message_1, "content")));
        if (J.t(((v) => J.t(v) ? !J.t((J.item(message_1, "tools") === null)) : v)((J.has("tools", message_1))))) {
          out.push(J.str(J.add(J.add("<|tool|>", J.item(message_1, "tools")), "<|/tool|>")));
        }
      }
    }
  }
  out.push(J.str("<|end|>"));
  {
    let _items2 = J.iter(messages_0);
    const _loop2 = J.loop(_items2);
    for (let _i2 = 0; _i2 < _items2.length; _i2++) {
      _loop2.index0 = _i2; const _it2 = _items2[_i2]; let message_2 = _it2;
      if (J.t((!J.eq(J.item(message_2, "role"), "system")))) {
        out.push(J.str(J.add(J.add(J.add(J.add("<|", J.item(message_2, "role")), "|>"), J.item(message_2, "content")), "<|end|>")));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|assistant|>"));
  } else {
    out.push(J.str(eos_token_0));
  }
  return out.join("");
}
