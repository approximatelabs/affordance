// Converted from the chat template of YannQi/R-4B by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let messages_0 = ctx["messages"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let thinking_mode_0 = ctx["thinking_mode"];
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      out.push(J.str(J.add(J.add("<|im_start|>", J.item(message_1, "role")), "\n")));
      {
        let _items2 = J.iter(J.filter("selectattr", J.item(message_1, "content"), "type", "equalto", "image"));
        const _loop2 = J.loop(_items2);
        for (let _i2 = 0; _i2 < _items2.length; _i2++) {
          _loop2.index0 = _i2; const _it2 = _items2[_i2]; let content_2 = _it2;
          out.push(J.str("<image>\n"));
        }
      }
      {
        let _items3 = J.iter(J.filter("selectattr", J.item(message_1, "content"), "type", "equalto", "video"));
        const _loop3 = J.loop(_items3);
        for (let _i3 = 0; _i3 < _items3.length; _i3++) {
          _loop3.index0 = _i3; const _it3 = _items3[_i3]; let content_3 = _it3;
          out.push(J.str("<video>\n"));
        }
      }
      if (J.t((!J.eq(J.item(message_1, "role"), "assistant")))) {
        {
          let _items4 = J.iter(J.filter("selectattr", J.item(message_1, "content"), "type", "equalto", "text"));
          const _loop4 = J.loop(_items4);
          for (let _i4 = 0; _i4 < _items4.length; _i4++) {
            _loop4.index0 = _i4; const _it4 = _items4[_i4]; let content_4 = _it4;
            out.push(J.str(J.item(content_4, "text")));
          }
        }
      } else {
        {
          let _items5 = J.iter(J.filter("selectattr", J.item(message_1, "content"), "type", "equalto", "text"));
          const _loop5 = J.loop(_items5);
          for (let _i5 = 0; _i5 < _items5.length; _i5++) {
            _loop5.index0 = _i5; const _it5 = _items5[_i5]; let content_5 = _it5;
            {
              out.push(J.str(J.item(content_5, "text")));
            }
          }
        }
      }
      out.push(J.str(J.add("<|im_end|>", "\n")));
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|im_start|>assistant\n<think>"));
  }
  if (J.t(add_generation_prompt_0)) {
    if (J.t(((v) => J.t(v) ? (J.eq(thinking_mode_0, "short")) : v)((thinking_mode_0 !== undefined)))) {
      out.push(J.str("\n\n</think>\n\n"));
    }
    if (J.t(((v) => J.t(v) ? (J.eq(thinking_mode_0, "long")) : v)((thinking_mode_0 !== undefined)))) {
      out.push(J.str("\n"));
    }
  }
  return out.join("");
}
