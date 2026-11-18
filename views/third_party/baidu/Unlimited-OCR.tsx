// Converted from the chat template of baidu/Unlimited-OCR by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let messages_0 = ctx["messages"];
  let eos_token_0 = ctx["eos_token"];
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      if (J.t(J.test("string", J.item(message_1, "content")))) {
        out.push(J.str(J.item(message_1, "content")));
      } else {
        {
          let _items2 = J.iter(J.filter("selectattr", J.item(message_1, "content"), "type", "equalto", "image"));
          const _loop2 = J.loop(_items2);
          for (let _i2 = 0; _i2 < _items2.length; _i2++) {
            _loop2.index0 = _i2; const _it2 = _items2[_i2]; let content_2 = _it2;
            out.push(J.str("<image>"));
          }
        }
        {
          let _items3 = J.iter(J.filter("selectattr", J.item(message_1, "content"), "type", "equalto", "text"));
          const _loop3 = J.loop(_items3);
          for (let _i3 = 0; _i3 < _items3.length; _i3++) {
            _loop3.index0 = _i3; const _it3 = _items3[_i3]; let content_3 = _it3;
            out.push(J.str(J.item(content_3, "text")));
          }
        }
      }
      if (J.t((J.eq(J.item(message_1, "role"), "assistant")))) {
        out.push(J.str(eos_token_0));
      }
    }
  }
  return out.join("");
}
