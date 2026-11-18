// Converted from the chat template of tencent/Hunyuan-A13B-Instruct by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let context_0 = ctx["context"];
  let loop_messages_0 = ctx["loop_messages"];
  let messages_0 = ctx["messages"];
  let content_0 = ctx["content"];
  let __0 = ctx["_"];
  context_0 = ({"has_head": true});
  loop_messages_0 = messages_0;
  {
    let _items1 = J.iter(loop_messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      let content_2 = content_0;
      let __3 = __0;
      content_2 = J.item(message_1, "content");
      if (J.t((J.eq(J.attr(_loop1, "index0"), 0)))) {
        if (J.t((J.eq(content_2, "")))) {
          __3 = J.call(J.attr(context_0, "update"), ({"has_head": false}));
        } else if (J.t((J.eq(J.item(message_1, "role"), "system")))) {
          content_2 = J.add(J.add("<|startoftext|>", content_2), "<|extra_4|>");
        }
      }
      if (J.t((J.eq(J.item(message_1, "role"), "user")))) {
        if (J.t(((v) => J.t(v) ? !J.t(J.attr(context_0, "has_head")) : v)((J.eq(J.attr(_loop1, "index0"), 1))))) {
          content_2 = J.add("<|startoftext|>", content_2);
        }
        if (J.t(((v) => J.t(v) ? J.attr(context_0, "has_head") : v)((J.eq(J.attr(_loop1, "index0"), 1))))) {
          content_2 = J.add(content_2, "<|extra_0|>");
        } else {
          content_2 = J.add(J.add("<|startoftext|>", content_2), "<|extra_0|>");
        }
      } else if (J.t((J.eq(J.item(message_1, "role"), "assistant")))) {
        content_2 = J.add(content_2, "<|eos|>");
      }
      out.push(J.str(content_2));
    }
  }
  return out.join("");
}
