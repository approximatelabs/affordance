// Converted from the chat template of jzhang533/PaddleOCR-VL-For-Manga by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let cls_token_0 = ctx["cls_token"];
  let eos_token_0 = ctx["eos_token"];
  let image_token_0 = ctx["image_token"];
  let messages_0 = ctx["messages"];
  if (J.t(!J.t((add_generation_prompt_0 !== undefined)))) {
    add_generation_prompt_0 = true;
  }
  if (J.t(!J.t((cls_token_0 !== undefined)))) {
    cls_token_0 = "<|begin_of_sentence|>";
  }
  if (J.t(!J.t((eos_token_0 !== undefined)))) {
    eos_token_0 = "</s>";
  }
  if (J.t(!J.t((image_token_0 !== undefined)))) {
    image_token_0 = "<|IMAGE_START|><|IMAGE_PLACEHOLDER|><|IMAGE_END|>";
  }
  out.push(J.str(cls_token_0));
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      if (J.t((J.eq(J.item(message_1, "role"), "user")))) {
        out.push(J.str("User: "));
        {
          let _items2 = J.iter(J.item(message_1, "content"));
          const _loop2 = J.loop(_items2);
          for (let _i2 = 0; _i2 < _items2.length; _i2++) {
            _loop2.index0 = _i2; const _it2 = _items2[_i2]; let content_2 = _it2;
            if (J.t((J.eq(J.item(content_2, "type"), "image")))) {
              out.push(J.str(image_token_0));
            }
          }
        }
        {
          let _items3 = J.iter(J.item(message_1, "content"));
          const _loop3 = J.loop(_items3);
          for (let _i3 = 0; _i3 < _items3.length; _i3++) {
            _loop3.index0 = _i3; const _it3 = _items3[_i3]; let content_3 = _it3;
            if (J.t((J.eq(J.item(content_3, "type"), "text")))) {
              out.push(J.str(J.item(content_3, "text")));
            }
          }
        }
        out.push(J.str("\n"));
      } else if (J.t((J.eq(J.item(message_1, "role"), "assistant")))) {
        out.push(J.str("Assistant: "));
        {
          let _items4 = J.iter(J.item(message_1, "content"));
          const _loop4 = J.loop(_items4);
          for (let _i4 = 0; _i4 < _items4.length; _i4++) {
            _loop4.index0 = _i4; const _it4 = _items4[_i4]; let content_4 = _it4;
            if (J.t((J.eq(J.item(content_4, "type"), "text")))) {
              out.push(J.str(J.item(content_4, "text")));
            }
          }
        }
        out.push(J.str(eos_token_0));
      } else if (J.t((J.eq(J.item(message_1, "role"), "system")))) {
        {
          let _items5 = J.iter(J.item(message_1, "content"));
          const _loop5 = J.loop(_items5);
          for (let _i5 = 0; _i5 < _items5.length; _i5++) {
            _loop5.index0 = _i5; const _it5 = _items5[_i5]; let content_5 = _it5;
            if (J.t((J.eq(J.item(content_5, "type"), "text")))) {
              out.push(J.str(J.add(J.item(content_5, "text"), "\n")));
            }
          }
        }
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("Assistant: "));
  }
  return out.join("");
}
