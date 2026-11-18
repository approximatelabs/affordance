// Converted from the chat template of deepseek-ai/deepseek-vl2-small by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let found_item_0 = ctx["found_item"];
  let messages_0 = ctx["messages"];
  found_item_0 = false;
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      let found_item_2 = found_item_0;
      if (J.t((J.eq(J.item(message_1, "role"), "system")))) {
        found_item_2 = true;
      }
    }
  }
  if (J.t(!J.t(found_item_0))) {
    out.push(J.str("You are an AI programming assistant, utilizing the Deepseek Coder model, developed by Deepseek Company, and you only answer questions related to computer science. For politically sensitive questions, security and privacy issues, and other non-computer science questions, you will refuse to answer.\n"));
  }
  {
    let _items2 = J.iter(messages_0);
    const _loop2 = J.loop(_items2);
    for (let _i2 = 0; _i2 < _items2.length; _i2++) {
      _loop2.index0 = _i2; const _it2 = _items2[_i2]; let message_3 = _it2;
      if (J.t((J.eq(J.item(message_3, "role"), "system")))) {
        out.push(J.str(J.item(message_3, "content")));
      } else {
        if (J.t((J.eq(J.item(message_3, "role"), "user")))) {
          out.push(J.str(J.add(J.add("### Instruction:\n", J.item(message_3, "content")), "\n")));
        } else {
          out.push(J.str(J.add(J.add("### Response:\n", J.item(message_3, "content")), "\n<|EOT|>\n")));
        }
      }
    }
  }
  out.push(J.str("### Response:\n"));
  return out.join("");
}
