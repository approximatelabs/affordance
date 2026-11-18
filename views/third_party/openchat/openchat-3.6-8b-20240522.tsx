// Converted from the chat template of openchat/openchat-3.6-8b-20240522 by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let bos_token_0 = ctx["bos_token"];
  let messages_0 = ctx["messages"];
  let content_0 = ctx["content"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  out.push(J.str(bos_token_0));
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      let content_2 = content_0;
      if (J.t((J.has(J.item(message_1, "role"), ["user", "assistant"])))) {
        content_2 = J.add(J.add(J.add(J.add("<|start_header_id|>GPT4 Correct ", J.call(J.attr(J.item(message_1, "role"), "title"))), "<|end_header_id|>\n\n"), J.filter("trim", J.item(message_1, "content"))), "<|eot_id|>");
      } else if (J.t((J.eq(J.item(message_1, "role"), "system")))) {
        content_2 = J.add(J.add("<|start_header_id|>System<|end_header_id|>\n\n", J.filter("trim", J.item(message_1, "content"))), "<|eot_id|>");
      } else {
        out.push(J.str(J.call(J.raise_exception, "Only user, assistant and system roles are supported!")));
      }
      out.push(J.str(content_2));
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|start_header_id|>GPT4 Correct Assistant<|end_header_id|>\n\n"));
  }
  return out.join("");
}
