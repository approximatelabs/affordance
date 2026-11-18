// Converted from the chat template of lightonai/LightOnOCR-2-1B by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let image_count_0 = ctx["image_count"];
  let messages_0 = ctx["messages"];
  let add_vision_id_0 = ctx["add_vision_id"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  image_count_0 = J.call(J.namespace, J.kw({"value": 0}));
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      if (J.t(((v) => J.t(v) ? (!J.eq(J.item(message_1, "role"), "system")) : v)(J.attr(_loop1, "first")))) {
        out.push("<|im_start|>system<|im_end|>\n");
      }
      out.push("<|im_start|>", J.str(J.item(message_1, "role")), "\n");
      if (J.t(J.test("string", J.item(message_1, "content")))) {
        out.push(J.str(J.item(message_1, "content")), "<|im_end|>\n");
      } else {
        {
          let _items2 = J.iter(J.item(message_1, "content"));
          const _loop2 = J.loop(_items2);
          for (let _i2 = 0; _i2 < _items2.length; _i2++) {
            _loop2.index0 = _i2; const _it2 = _items2[_i2]; let content_2 = _it2;
            if (J.t(((v) => J.t(v) ? v : (J.has("image_url", content_2)))(((v) => J.t(v) ? v : (J.has("image", content_2)))((J.eq(J.item(content_2, "type"), "image")))))) {
              J.setattr(image_count_0, "value", J.add(J.attr(image_count_0, "value"), 1));
              if (J.t(add_vision_id_0)) {
                out.push("Picture ", J.str(J.attr(image_count_0, "value")), ":\n");
              }
              out.push("<|image_pad|>");
            } else if (J.t((J.has("text", content_2)))) {
              out.push(J.str(J.item(content_2, "text")), "\n");
            }
          }
        }
        out.push("<|im_end|>\n");
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push("<|im_start|>assistant\n");
  }
  return out.join("");
}
