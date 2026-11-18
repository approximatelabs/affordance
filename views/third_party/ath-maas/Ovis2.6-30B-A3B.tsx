// Converted from the chat template of ATH-MaaS/Ovis2.6-30B-A3B by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let messages_0 = ctx["messages"];
  let content_0 = ctx["content"];
  let ns_0 = ctx["ns"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let enable_thinking_0 = ctx["enable_thinking"];
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      let content_2 = content_0;
      let ns_3 = ns_0;
      out.push(J.str(J.add(J.add("<|im_start|>", J.attr(message_1, "role")), "\n")));
      if (J.t(((v) => J.t(v) ? v : (J.eq(J.attr(message_1, "role"), "user")))((J.eq(J.attr(message_1, "role"), "system"))))) {
        if (J.t(J.test("string", J.attr(message_1, "content")))) {
          out.push(J.str(J.attr(message_1, "content")));
        } else {
          {
            let _items2 = J.iter(J.attr(message_1, "content"));
            const _loop2 = J.loop(_items2);
            for (let _i2 = 0; _i2 < _items2.length; _i2++) {
              _loop2.index0 = _i2; const _it2 = _items2[_i2]; let item_4 = _it2;
              if (J.t(((v) => J.t(v) ? (J.has("text", item_4)) : v)((J.eq(J.attr(item_4, "type"), "text"))))) {
                out.push(J.str(J.attr(item_4, "text")));
              } else if (J.t((J.eq(J.attr(item_4, "type"), "image")))) {
                out.push(J.str("<image>"));
              } else if (J.t((J.eq(J.attr(item_4, "type"), "video")))) {
                out.push(J.str("<video>"));
              } else {
                out.push(J.str(J.call(J.raise_exception, "Invalid content type. Supported types for system and user are text, image, video.")));
              }
              if (J.t(!J.t(J.attr(_loop2, "last")))) {
                out.push(J.str("\n"));
              }
            }
          }
        }
      } else if (J.t((J.eq(J.attr(message_1, "role"), "assistant")))) {
        content_2 = "";
        if (J.t(J.test("string", J.attr(message_1, "content")))) {
          content_2 = J.attr(message_1, "content");
        } else {
          ns_3 = J.call(J.namespace, J.kw({"content": ""}));
          {
            let _items3 = J.iter(J.attr(message_1, "content"));
            const _loop3 = J.loop(_items3);
            for (let _i3 = 0; _i3 < _items3.length; _i3++) {
              _loop3.index0 = _i3; const _it3 = _items3[_i3]; let item_5 = _it3;
              if (J.t(((v) => J.t(v) ? (J.has("text", item_5)) : v)((J.eq(J.attr(item_5, "type"), "text"))))) {
                J.setattr(ns_3, "content", J.concat(J.attr(ns_3, "content"), J.attr(item_5, "text")));
              } else {
                out.push(J.str(J.call(J.raise_exception, "Invalid content type. Supported type for assistant is text.")));
              }
            }
          }
          content_2 = J.attr(ns_3, "content");
        }
        content_2 = J.call(J.attr(J.item(J.call(J.attr(content_2, "split"), "</think>"), J.neg(1)), "lstrip"), "\n");
        out.push(J.str(content_2));
      } else {
        out.push(J.str(J.call(J.raise_exception, "Invalid role. Supported roles are system, user, assistant.")));
      }
      out.push(J.str("<|im_end|>\n"));
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|im_start|>assistant\n"));
    if (J.t(((v) => J.t(v) ? J.test("false", enable_thinking_0) : v)((enable_thinking_0 !== undefined)))) {
      out.push(J.str("<think>\n\n</think>\n\n"));
    }
  }
  return out.join("");
}
