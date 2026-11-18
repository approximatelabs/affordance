// Converted from the chat template of tencent/HunyuanOCR by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let loop_messages_0 = ctx["loop_messages"];
  let system_message_0 = ctx["system_message"];
  let messages_0 = ctx["messages"];
  if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
    loop_messages_0 = J.slice(messages_0, 1, undefined, undefined);
    if (J.t(J.test("string", J.item(J.item(messages_0, 0), "content")))) {
      system_message_0 = J.item(J.item(messages_0, 0), "content");
    } else {
      system_message_0 = J.item(J.item(J.item(messages_0, 0), "content"), "text");
    }
    out.push("<\uff5chy_begin\u2581of\u2581sentence\uff5c>", J.str(system_message_0), "<\uff5chy_place\u2581holder\u2581no\u25813\uff5c>");
  } else {
    loop_messages_0 = messages_0;
    out.push("<\uff5chy_begin\u2581of\u2581sentence\uff5c>");
  }
  {
    let _items1 = J.iter(loop_messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      if (J.t((J.eq(J.item(message_1, "role"), "user")))) {
        if (J.t(J.test("string", J.item(message_1, "content")))) {
          out.push(J.str(J.item(message_1, "content")));
        } else {
          {
            let _items2 = J.iter(J.item(message_1, "content"));
            const _loop2 = J.loop(_items2);
            for (let _i2 = 0; _i2 < _items2.length; _i2++) {
              _loop2.index0 = _i2; const _it2 = _items2[_i2]; let content_2 = _it2;
              if (J.t(((v) => J.t(v) ? v : (J.has("image_url", content_2)))(((v) => J.t(v) ? v : (J.has("image", content_2)))((J.eq(J.item(content_2, "type"), "image")))))) {
                out.push("<\uff5chy_place\u2581holder\u2581no\u2581100\uff5c><\uff5chy_place\u2581holder\u2581no\u2581102\uff5c><\uff5chy_place\u2581holder\u2581no\u2581101\uff5c>");
              } else if (J.t((J.has("text", content_2)))) {
                out.push(J.str(J.item(content_2, "text")));
              }
            }
          }
        }
        out.push("<\uff5chy_User\uff5c>");
      } else if (J.t((J.eq(J.item(message_1, "role"), "assistant")))) {
        out.push(J.str(J.item(message_1, "content")), "<\uff5chy_Assistant\uff5c>");
      }
    }
  }
  return out.join("");
}
