// Converted from the chat template of CohereLabs/North-Micro-Vision-Instruct by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let bos_token_0 = ctx["bos_token"];
  let messages_0 = ctx["messages"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  out.push(J.str(bos_token_0));
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      if (J.t((J.eq(J.attr(message_1, "role"), "system")))) {
        out.push(J.str("<|START_OF_TURN_TOKEN|><|SYSTEM_TOKEN|>"));
        if (J.t(J.test("string", J.attr(message_1, "content")))) {
          out.push(J.str(J.attr(message_1, "content")));
        } else {
          {
            let _items2 = J.iter(J.attr(message_1, "content"));
            const _loop2 = J.loop(_items2);
            for (let _i2 = 0; _i2 < _items2.length; _i2++) {
              _loop2.index0 = _i2; const _it2 = _items2[_i2]; let content_2 = _it2;
              if (J.t((J.has("text", content_2)))) {
                out.push(J.str(J.attr(content_2, "text")));
              }
            }
          }
        }
        out.push(J.str("<|END_OF_TURN_TOKEN|>"));
      } else if (J.t((J.eq(J.attr(message_1, "role"), "user")))) {
        out.push(J.str("<|START_OF_TURN_TOKEN|><|USER_TOKEN|>"));
        if (J.t(J.test("string", J.attr(message_1, "content")))) {
          out.push(J.str(J.attr(message_1, "content")));
        } else {
          {
            let _items3 = J.iter(J.attr(message_1, "content"));
            const _loop3 = J.loop(_items3);
            for (let _i3 = 0; _i3 < _items3.length; _i3++) {
              _loop3.index0 = _i3; const _it3 = _items3[_i3]; let content_3 = _it3;
              if (J.t(((v) => J.t(v) ? v : (J.has("image_url", content_3)))(((v) => J.t(v) ? v : (J.has("image", content_3)))((J.eq(J.attr(content_3, "type"), "image")))))) {
                out.push("<|VISION_START|><|IMAGE_PAD|><|VISION_END|>");
              } else if (J.t(((v) => J.t(v) ? v : (J.has("video", content_3)))((J.eq(J.attr(content_3, "type"), "video"))))) {
                out.push("<|VISION_START|><|VIDEO_PAD|><|VISION_END|>");
              } else if (J.t((J.has("text", content_3)))) {
                out.push(J.str(J.attr(content_3, "text")));
              }
            }
          }
        }
        out.push(J.str("<|END_OF_TURN_TOKEN|>"));
      } else if (J.t(((v) => J.t(v) ? v : (J.eq(J.attr(message_1, "role"), "chatbot")))((J.eq(J.attr(message_1, "role"), "assistant"))))) {
        out.push(J.str("<|START_OF_TURN_TOKEN|><|CHATBOT_TOKEN|>"));
        {
          if (J.t(J.test("string", J.attr(message_1, "content")))) {
            out.push(J.str(J.attr(message_1, "content")));
          } else {
            {
              let _items4 = J.iter(J.attr(message_1, "content"));
              const _loop4 = J.loop(_items4);
              for (let _i4 = 0; _i4 < _items4.length; _i4++) {
                _loop4.index0 = _i4; const _it4 = _items4[_i4]; let content_4 = _it4;
                if (J.t((J.has("text", content_4)))) {
                  out.push(J.str(J.attr(content_4, "text")));
                }
              }
            }
          }
          out.push(J.str("<|END_OF_TURN_TOKEN|>"));
        }
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|START_OF_TURN_TOKEN|><|CHATBOT_TOKEN|>"));
  }
  return out.join("");
}
