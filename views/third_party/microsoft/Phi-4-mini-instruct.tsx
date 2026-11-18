// Converted from the chat template of microsoft/Phi-4-mini-instruct by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let messages_0 = ctx["messages"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let eos_token_0 = ctx["eos_token"];
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      if (J.t(((v) => J.t(v) ? !J.t((J.item(message_1, "tools") === null)) : v)(((v) => J.t(v) ? (J.has("tools", message_1)) : v)((J.eq(J.item(message_1, "role"), "system")))))) {
        out.push(J.str(J.add(J.add(J.add(J.add(J.add(J.add(J.add("<|", J.item(message_1, "role")), "|>"), J.item(message_1, "content")), "<|tool|>"), J.item(message_1, "tools")), "<|/tool|>"), "<|end|>")));
      } else {
        out.push(J.str(J.add(J.add(J.add(J.add("<|", J.item(message_1, "role")), "|>"), J.item(message_1, "content")), "<|end|>")));
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
