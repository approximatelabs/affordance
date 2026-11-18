// Converted from the chat template of datalab-to/surya-ocr-2 by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let image_count_0 = ctx["image_count"];
  let video_count_0 = ctx["video_count"];
  let render_content_0 = ctx["render_content"];
  let content_0 = ctx["content"];
  let add_vision_id_0 = ctx["add_vision_id"];
  let messages_0 = ctx["messages"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  image_count_0 = J.call(J.namespace, J.kw({"value": 0}));
  video_count_0 = J.call(J.namespace, J.kw({"value": 0}));
  render_content_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_4 = k.caller;
    let content_1 = p.length > 0 ? p[0] : "content" in k ? k["content"] : undefined;
    let do_vision_count_2 = p.length > 1 ? p[1] : "do_vision_count" in k ? k["do_vision_count"] : undefined;
    let is_system_content_3 = p.length > 2 ? p[2] : "is_system_content" in k ? k["is_system_content"] : false;
    if (J.t(J.test("string", content_1))) {
      out.push(J.str(content_1));
    } else if (J.t(((v) => J.t(v) ? !J.t(J.test("mapping", content_1)) : v)(J.test("iterable", content_1)))) {
      {
        let _items1 = J.iter(content_1);
        const _loop1 = J.loop(_items1);
        for (let _i1 = 0; _i1 < _items1.length; _i1++) {
          _loop1.index0 = _i1; const _it1 = _items1[_i1]; let item_5 = _it1;
          if (J.t(((v) => J.t(v) ? v : (J.eq(J.attr(item_5, "type"), "image")))(((v) => J.t(v) ? v : (J.has("image_url", item_5)))((J.has("image", item_5)))))) {
            if (J.t(is_system_content_3)) {
              out.push(J.str(J.call(J.raise_exception, "System message cannot contain images.")));
            }
            if (J.t(do_vision_count_2)) {
              J.setattr(image_count_0, "value", J.add(J.attr(image_count_0, "value"), 1));
            }
            if (J.t(add_vision_id_0)) {
              out.push(J.str(J.concat("Picture ", J.attr(image_count_0, "value"), ": ")));
            }
            out.push(J.str("<|vision_start|><|image_pad|><|vision_end|>"));
          } else if (J.t(((v) => J.t(v) ? v : (J.eq(J.attr(item_5, "type"), "video")))((J.has("video", item_5))))) {
            if (J.t(is_system_content_3)) {
              out.push(J.str(J.call(J.raise_exception, "System message cannot contain videos.")));
            }
            if (J.t(do_vision_count_2)) {
              J.setattr(video_count_0, "value", J.add(J.attr(video_count_0, "value"), 1));
            }
            if (J.t(add_vision_id_0)) {
              out.push(J.str(J.concat("Video ", J.attr(video_count_0, "value"), ": ")));
            }
            out.push(J.str("<|vision_start|><|video_pad|><|vision_end|>"));
          } else if (J.t((J.has("text", item_5)))) {
            out.push(J.str(J.attr(item_5, "text")));
          } else {
            out.push(J.str(J.call(J.raise_exception, "Unexpected item type in content.")));
          }
        }
      }
    } else if (J.t(((v) => J.t(v) ? v : (content_1 === undefined))((content_1 === null)))) {
      out.push(J.str(""));
    } else {
      out.push(J.str(J.call(J.raise_exception, "Unexpected content type.")));
    }
    return out.join("");
  };
  if (J.t(!J.t(messages_0))) {
    out.push(J.str(J.call(J.raise_exception, "No messages provided.")));
  }
  if (J.t((J.eq(J.attr(J.item(messages_0, 0), "role"), "system")))) {
    content_0 = J.filter("trim", J.call(render_content_0, J.attr(J.item(messages_0, 0), "content"), false, true));
    out.push(J.str(J.add(J.add("<|im_start|>system\n", content_0), "<|im_end|>\n")));
  }
  {
    let _items2 = J.iter(messages_0);
    const _loop2 = J.loop(_items2);
    for (let _i2 = 0; _i2 < _items2.length; _i2++) {
      _loop2.index0 = _i2; const _it2 = _items2[_i2]; let message_6 = _it2;
      let content_7 = content_0;
      content_7 = J.filter("trim", J.call(render_content_0, J.attr(message_6, "content"), true));
      if (J.t((J.eq(J.attr(message_6, "role"), "system")))) {
        if (J.t(!J.t(J.attr(_loop2, "first")))) {
          out.push(J.str(J.call(J.raise_exception, "System message must be at the beginning.")));
        }
      } else if (J.t((J.eq(J.attr(message_6, "role"), "user")))) {
        out.push(J.str(J.add(J.add(J.add(J.add(J.add("<|im_start|>", J.attr(message_6, "role")), "\n"), content_7), "<|im_end|>"), "\n")));
      } else if (J.t((J.eq(J.attr(message_6, "role"), "assistant")))) {
        out.push(J.str(J.add(J.add(J.add("<|im_start|>", J.attr(message_6, "role")), "\n"), content_7)), J.str("<|im_end|>\n"));
      } else {
        out.push(J.str(J.call(J.raise_exception, "Unexpected message role.")));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|im_start|>assistant\n"));
  }
  return out.join("");
}
