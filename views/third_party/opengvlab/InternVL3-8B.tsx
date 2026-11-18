// Converted from the chat template of OpenGVLab/InternVL3-8B by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let messages_0 = ctx["messages"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
    out.push(J.str(J.add(J.add("<|im_start|>system\n", J.item(J.item(messages_0, 0), "content")), "<|im_end|>\n")));
  } else {
    out.push(J.str("<|im_start|>system\n\u4f60\u662f\u4e66\u751f\u00b7\u4e07\u8c61\uff0c\u82f1\u6587\u540d\u662fInternVL\uff0c\u662f\u7531\u4e0a\u6d77\u4eba\u5de5\u667a\u80fd\u5b9e\u9a8c\u5ba4\u3001\u6e05\u534e\u5927\u5b66\u53ca\u591a\u5bb6\u5408\u4f5c\u5355\u4f4d\u8054\u5408\u5f00\u53d1\u7684\u591a\u6a21\u6001\u5927\u8bed\u8a00\u6a21\u578b\u3002<|im_end|>\n"));
  }
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      if (J.t(((v) => J.t(v) ? v : !J.t(J.attr(_loop1, "first")))((!J.eq(J.item(J.item(messages_0, 0), "role"), "system"))))) {
        out.push(J.str(J.add(J.add("<|im_start|>", J.item(message_1, "role")), "\n")));
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
              } else if (J.t((J.eq(J.item(content_2, "type"), "video")))) {
                out.push(J.str("<video>\n"));
              } else if (J.t((J.eq(J.item(content_2, "type"), "text")))) {
                out.push(J.str(J.item(content_2, "text")));
              }
            }
          }
        }
        out.push(J.str("<|im_end|>\n"));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|im_start|>assistant\n"));
  }
  return out.join("");
}
