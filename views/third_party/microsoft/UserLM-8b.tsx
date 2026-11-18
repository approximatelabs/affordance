// Converted from the chat template of microsoft/UserLM-8b by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let messages_0 = ctx["messages"];
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      out.push(J.str(J.add(J.add("<|start_header_id|>", J.item(message_1, "role")), "<|end_header_id|>")), "\n", J.str(J.item(message_1, "content")), "<|eot_id|>");
    }
  }
  out.push(J.str("<|start_header_id|>user<|end_header_id|>"));
  return out.join("");
}
