// Converted from the chat template of shenzhi-wang/Llama3-8B-Chinese-Chat by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let system_message_0 = ctx["system_message"];
  let loop_messages_0 = ctx["loop_messages"];
  let messages_0 = ctx["messages"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  out.push(J.str("<|begin_of_text|>"));
  system_message_0 = "You are a helpful assistant.";
  if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
    system_message_0 = J.item(J.item(messages_0, 0), "content");
    loop_messages_0 = J.slice(messages_0, 1, undefined, undefined);
  } else {
    loop_messages_0 = messages_0;
  }
  if (J.t((system_message_0 !== undefined))) {
    out.push(J.str(J.add(J.add("<|start_header_id|>system<|end_header_id|>\n\n", J.filter("trim", system_message_0)), "<|eot_id|>")));
  }
  {
    let _items1 = J.iter(loop_messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      out.push(J.str(J.add(J.add(J.add(J.add("<|start_header_id|>", J.item(message_1, "role")), "<|end_header_id|>\n\n"), J.filter("trim", J.item(message_1, "content"))), "<|eot_id|>")));
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|start_header_id|>assistant<|end_header_id|>\n\n"));
  }
  return out.join("");
}
