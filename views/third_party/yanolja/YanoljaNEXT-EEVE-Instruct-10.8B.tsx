// Converted from the chat template of yanolja/YanoljaNEXT-EEVE-Instruct-10.8B by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let loop_messages_0 = ctx["loop_messages"];
  let system_message_0 = ctx["system_message"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let messages_0 = ctx["messages"];
  if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
    loop_messages_0 = J.slice(messages_0, 1, undefined, undefined);
    system_message_0 = J.item(J.item(messages_0, 0), "content");
  } else {
    loop_messages_0 = messages_0;
    system_message_0 = "You are a helpful assistant.";
  }
  if (J.t(!J.t((add_generation_prompt_0 !== undefined)))) {
    add_generation_prompt_0 = false;
  }
  {
    let _items1 = J.iter(loop_messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      if (J.t((J.eq(J.attr(_loop1, "index0"), 0)))) {
        out.push(J.str(J.add(J.add("<|im_start|>system\n", system_message_0), "<|im_end|>\n")));
      }
      out.push(J.str(J.add(J.add(J.add(J.add(J.add("<|im_start|>", J.item(message_1, "role")), "\n"), J.item(message_1, "content")), "<|im_end|>"), "\n")));
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|im_start|>assistant\n"));
  }
  return out.join("");
}
