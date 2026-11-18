// Converted from the chat template of ZJU-AI4H/Hulu-Med-7B by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let identifier_0 = ctx["identifier"];
  let messages_0 = ctx["messages"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  identifier_0 = "im";
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      let identifier_2 = identifier_0;
      if (J.t((J.eq(J.item(message_1, "role"), "stream")))) {
        identifier_2 = "stream";
      } else {
        identifier_2 = "im";
      }
      out.push(J.str(J.add(J.add(J.add(J.add("<|", identifier_2), "_start|>"), J.item(message_1, "role")), "\n")));
      if (J.t(J.test("string", J.item(message_1, "content")))) {
        out.push(J.str(J.add(J.add(J.add(J.item(message_1, "content"), "<|"), identifier_2), "_end|>\n")));
      } else {
        {
          let _items2 = J.iter(J.item(message_1, "content"));
          const _loop2 = J.loop(_items2);
          for (let _i2 = 0; _i2 < _items2.length; _i2++) {
            _loop2.index0 = _i2; const _it2 = _items2[_i2]; let content_3 = _it2;
            if (J.t(((v) => J.t(v) ? v : (J.has("image_url", content_3)))(((v) => J.t(v) ? v : (J.has("image", content_3)))((J.eq(J.item(content_3, "type"), "image")))))) {
              if (J.t((J.has("time", content_3)))) {
                out.push(J.str(J.add(J.add("Time ", J.filter("string", J.filter("round", J.item(content_3, "time"), 1))), "s: ")));
              }
              out.push(J.str("<image>\n"));
            } else if (J.t(((v) => J.t(v) ? v : (J.has("video_url", content_3)))(((v) => J.t(v) ? v : (J.has("video", content_3)))((J.eq(J.item(content_3, "type"), "video")))))) {
              {
                let _items3 = J.iter(J.call(J.range, J.item(content_3, "num_frames")));
                const _loop3 = J.loop(_items3);
                for (let _i3 = 0; _i3 < _items3.length; _i3++) {
                  _loop3.index0 = _i3; const _it3 = _items3[_i3]; let i_4 = _it3;
                  if (J.t((J.has("timestamps", content_3)))) {
                    out.push(J.str(J.add(J.add("Time ", J.filter("string", J.filter("round", J.item(J.item(content_3, "timestamps"), i_4), 1))), "s:")));
                  }
                  if (J.t((J.lt(i_4, J.sub(J.item(content_3, "num_frames"), 1))))) {
                    out.push(J.str("<image>,"));
                  } else {
                    out.push(J.str("<image>\n"));
                  }
                }
              }
            } else if (J.t(((v) => J.t(v) ? v : (J.has("text", content_3)))((J.eq(J.item(content_3, "type"), "text"))))) {
              out.push(J.str(J.item(content_3, "text")));
            }
          }
        }
        out.push(J.str(J.add(J.add("<|", identifier_2), "_end|>\n")));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|im_start|>assistant\n"));
  }
  return out.join("");
}
