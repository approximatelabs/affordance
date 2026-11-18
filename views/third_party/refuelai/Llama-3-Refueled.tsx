// Converted from the chat template of refuelai/Llama-3-Refueled by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let loop_messages_0 = ctx["loop_messages"];
  let messages_0 = ctx["messages"];
  let content_0 = ctx["content"];
  let bos_token_0 = ctx["bos_token"];
  let eos_token_0 = ctx["eos_token"];
  if (J.t(!J.t((add_generation_prompt_0 !== undefined)))) {
    add_generation_prompt_0 = false;
  }
  loop_messages_0 = messages_0;
  {
    let _items1 = J.iter(loop_messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      let content_2 = content_0;
      content_2 = J.add(J.add(J.add(J.add("<|start_header_id|>", J.item(message_1, "role")), "<|end_header_id|>\n\n"), J.filter("trim", J.item(message_1, "content"))), "<|eot_id|>");
      if (J.t((J.eq(J.attr(_loop1, "index0"), 0)))) {
        content_2 = J.add(bos_token_0, content_2);
      }
      out.push(J.str(content_2));
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|start_header_id|>assistant<|end_header_id|>\n\n"));
  } else {
    out.push(J.str(eos_token_0));
  }
  return out.join("");
}
