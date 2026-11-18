// Converted from the chat template of nvidia/Llama-3.1-Nemotron-Nano-VL-8B-V1 by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let system_message_0 = ctx["system_message"];
  let messages_0 = ctx["messages"];
  let bos_token_0 = ctx["bos_token"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
    if (J.t(J.test("string", J.item(J.item(messages_0, 0), "content")))) {
      system_message_0 = J.item(J.item(messages_0, 0), "content");
    } else {
      system_message_0 = J.item(J.item(J.item(J.item(messages_0, 0), "content"), 0), "text");
    }
    messages_0 = J.slice(messages_0, 1, undefined, undefined);
  } else {
    system_message_0 = "Answer the questions.";
  }
  out.push(J.str(bos_token_0), "<|start_header_id|>system<|end_header_id|>\n\n", J.str(system_message_0), "<|eot_id|>");
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      out.push("<|start_header_id|>", J.str(J.item(message_1, "role")), "<|end_header_id|>\n\n");
      if (J.t(J.test("string", J.item(message_1, "content")))) {
        out.push(J.str(J.item(message_1, "content")));
      } else {
        {
          let _items2 = J.iter(J.item(message_1, "content"));
          const _loop2 = J.loop(_items2);
          for (let _i2 = 0; _i2 < _items2.length; _i2++) {
            _loop2.index0 = _i2; const _it2 = _items2[_i2]; let content_2 = _it2;
            if (J.t((J.eq(J.item(content_2, "type"), "image")))) {
              out.push(J.str("<image>\n"));
            } else if (J.t((J.eq(J.item(content_2, "type"), "text")))) {
              out.push(J.str(J.item(content_2, "text")));
            }
          }
        }
      }
      out.push("<|eot_id|>");
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push("<|start_header_id|>assistant<|end_header_id|>\n\n");
  }
  return out.join("");
}
