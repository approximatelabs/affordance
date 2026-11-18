// Converted from the chat template of zai-org/AutoGLM-Phone-9B by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let messages_0 = ctx["messages"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  out.push("[gMASK]<sop>");
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let msg_1 = _it1;
      if (J.t((J.eq(J.attr(msg_1, "role"), "system")))) {
        out.push("<|system|>\n", J.str(J.attr(msg_1, "content")));
      } else if (J.t((J.eq(J.attr(msg_1, "role"), "user")))) {
        out.push("<|user|>", J.str("\n"));
        if (J.t(J.test("string", J.attr(msg_1, "content")))) {
          out.push(J.str(J.attr(msg_1, "content")));
        } else {
          {
            let _items2 = J.iter(J.attr(msg_1, "content"));
            const _loop2 = J.loop(_items2);
            for (let _i2 = 0; _i2 < _items2.length; _i2++) {
              _loop2.index0 = _i2; const _it2 = _items2[_i2]; let item_2 = _it2;
              if (J.t(((v) => J.t(v) ? v : (J.has("video", item_2)))((J.eq(J.attr(item_2, "type"), "video"))))) {
                out.push("<|begin_of_video|><|video|><|end_of_video|>");
              } else if (J.t(((v) => J.t(v) ? v : (J.has("image", item_2)))((J.eq(J.attr(item_2, "type"), "image"))))) {
                out.push("<|begin_of_image|><|image|><|end_of_image|>");
              } else if (J.t((J.eq(J.attr(item_2, "type"), "text")))) {
                out.push(J.str(J.attr(item_2, "text")));
              }
            }
          }
        }
      } else if (J.t((J.eq(J.attr(msg_1, "role"), "observation")))) {
        out.push(J.str("\n"));
        if (J.t(J.test("string", J.attr(msg_1, "content")))) {
          out.push(J.str(J.attr(msg_1, "content")));
        } else {
          {
            let _items3 = J.iter(J.attr(msg_1, "content"));
            const _loop3 = J.loop(_items3);
            for (let _i3 = 0; _i3 < _items3.length; _i3++) {
              _loop3.index0 = _i3; const _it3 = _items3[_i3]; let item_3 = _it3;
              if (J.t(((v) => J.t(v) ? v : (J.has("video", item_3)))((J.eq(J.attr(item_3, "type"), "video"))))) {
                out.push("<|begin_of_video|><|video|><|end_of_video|>");
              } else if (J.t(((v) => J.t(v) ? v : (J.has("image", item_3)))((J.eq(J.attr(item_3, "type"), "image"))))) {
                out.push("<|begin_of_image|><|image|><|end_of_image|>");
              } else if (J.t((J.eq(J.attr(item_3, "type"), "text")))) {
                out.push(J.str(J.attr(item_3, "text")));
              }
            }
          }
        }
      } else if (J.t((J.eq(J.attr(msg_1, "role"), "assistant")))) {
        if (J.t(J.attr(msg_1, "metadata"))) {
          out.push("<|assistant|>", J.str(J.attr(msg_1, "metadata")), "\n", J.str(J.attr(msg_1, "content")));
        } else {
          out.push("<|assistant|>\n", J.str(J.attr(msg_1, "content")));
        }
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push("<|assistant|>\n");
  }
  return out.join("");
}
