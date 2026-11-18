// Converted from the chat template of HuggingFaceTB/SmolVLM2-2.2B-Instruct by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let messages_0 = ctx["messages"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  out.push("<|im_start|>");
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      out.push(J.str(J.filter("capitalize", J.item(message_1, "role"))));
      if (J.t(J.test("string", J.item(message_1, "content")))) {
        out.push(J.str(J.add(": ", J.item(message_1, "content"))));
      } else {
        if (J.t((J.eq(J.item(J.item(J.item(message_1, "content"), 0), "type"), "image")))) {
          out.push(J.str(":"));
        } else {
          out.push(J.str(": "));
        }
        {
          let _items2 = J.iter(J.item(message_1, "content"));
          const _loop2 = J.loop(_items2);
          for (let _i2 = 0; _i2 < _items2.length; _i2++) {
            _loop2.index0 = _i2; const _it2 = _items2[_i2]; let line_2 = _it2;
            if (J.t((J.eq(J.item(line_2, "type"), "text")))) {
              out.push(J.str(J.item(line_2, "text")));
            } else if (J.t((J.eq(J.item(line_2, "type"), "image")))) {
              out.push(J.str("<image>"));
            }
          }
        }
      }
      out.push("<end_of_utterance>\n");
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("Assistant:"));
  }
  return out.join("");
}
