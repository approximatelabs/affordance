// Converted from the chat template of baidu/ERNIE-4.5-VL-28B-A3B-PT by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let image_count_0 = ctx["image_count"];
  let video_count_0 = ctx["video_count"];
  let messages_0 = ctx["messages"];
  let extract_text_content_0 = ctx["extract_text_content"];
  let reasoning_content_0 = ctx["reasoning_content"];
  let content_0 = ctx["content"];
  let ns_0 = ctx["ns"];
  let text_parts_0 = ctx["text_parts"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let enable_thinking_0 = ctx["enable_thinking"];
  image_count_0 = J.call(J.namespace, J.kw({"value": 0}));
  video_count_0 = J.call(J.namespace, J.kw({"value": 0}));
  out.push(J.str("<|begin_of_sentence|>"));
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      let extract_text_content_2 = extract_text_content_0;
      let reasoning_content_3 = reasoning_content_0;
      let content_4 = content_0;
      if (J.t((J.has(J.attr(message_1, "role"), ["system", "user"])))) {
        if (J.t((J.eq(J.attr(message_1, "role"), "user")))) {
          out.push(J.str("User: "));
        }
        if (J.t(J.test("string", J.attr(message_1, "content")))) {
          out.push(J.str(J.attr(message_1, "content")));
        } else {
          {
            let _items2 = J.iter(J.attr(message_1, "content"));
            const _loop2 = J.loop(_items2);
            for (let _i2 = 0; _i2 < _items2.length; _i2++) {
              _loop2.index0 = _i2; const _it2 = _items2[_i2]; let content_item_5 = _it2;
              if (J.t((J.eq(J.attr(content_item_5, "type"), "text")))) {
                out.push(J.str(J.attr(content_item_5, "text")));
              } else if (J.t((J.has(J.attr(content_item_5, "type"), ["image_url", "image"])))) {
                J.setattr(image_count_0, "value", J.add(J.attr(image_count_0, "value"), 1));
                out.push("Picture ", J.str(J.attr(image_count_0, "value")), ":<|IMAGE_START|><|image@placeholder|><|IMAGE_END|>");
              } else if (J.t((J.has(J.attr(content_item_5, "type"), ["video_url", "video"])))) {
                J.setattr(video_count_0, "value", J.add(J.attr(video_count_0, "value"), 1));
                out.push("Video ", J.str(J.attr(video_count_0, "value")), ":<|VIDEO_START|><|video@placeholder|><|VIDEO_END|>");
              }
            }
          }
        }
        if (J.t((J.eq(J.attr(message_1, "role"), "system")))) {
          out.push(J.str("\n"));
        }
      } else if (J.t((J.eq(J.attr(message_1, "role"), "assistant")))) {
        extract_text_content_2 = (...args) => {
          const [p, k] = J.args(args);
          const out = [];
          const caller_7 = k.caller;
          let content_field_6 = p.length > 0 ? p[0] : "content_field" in k ? k["content_field"] : undefined;
          let ns_8 = ns_0;
          let text_parts_9 = text_parts_0;
          if (J.t(J.test("string", content_field_6))) {
            out.push(J.str(content_field_6));
          } else if (J.t(((v) => J.t(v) ? !J.t(J.test("string", content_field_6)) : v)(J.test("iterable", content_field_6)))) {
            ns_8 = J.call(J.namespace, J.kw({"text_parts": []}));
            text_parts_9 = [];
            {
              let _items3 = J.iter(content_field_6);
              const _loop3 = J.loop(_items3);
              for (let _i3 = 0; _i3 < _items3.length; _i3++) {
                _loop3.index0 = _i3; const _it3 = _items3[_i3]; let item_10 = _it3;
                if (J.t((J.eq(J.attr(item_10, "type"), "text")))) {
                  J.setattr(ns_8, "text_parts", J.add(J.attr(ns_8, "text_parts"), [J.attr(item_10, "text")]));
                }
              }
            }
            out.push(J.str(J.filter("join", J.attr(ns_8, "text_parts"), "")));
          } else {
            out.push(J.str(""));
          }
          return out.join("");
        };
        reasoning_content_3 = J.call(extract_text_content_2, J.attr(message_1, "reasoning_content"));
        content_4 = J.call(extract_text_content_2, J.attr(message_1, "content"));
        if (J.t((J.has("</think>", content_4)))) {
          reasoning_content_3 = J.call(J.attr(J.item(J.call(J.attr(J.call(J.attr(J.item(J.call(J.attr(content_4, "split"), "</think>"), 0), "rstrip"), "\n"), "split"), "<think>"), J.neg(1)), "lstrip"), "\n");
          content_4 = J.call(J.attr(J.item(J.call(J.attr(content_4, "split"), "</think>"), J.neg(1)), "lstrip"), "\n");
        }
        if (J.t(reasoning_content_3)) {
          out.push(J.str(J.add(J.add(J.add(J.add(J.add("\n", "Assistant: "), "<think>\n"), J.call(J.attr(reasoning_content_3, "strip"), "\n")), "\n</think>\n\n"), J.call(J.attr(content_4, "lstrip"), "\n"))));
        } else {
          out.push(J.str(J.add(J.add("\n", "Assistant: "), content_4)));
        }
        out.push(J.str("<|end_of_sentence|>"));
      }
    }
  }
  if (J.t(((v) => J.t(v) ? v : J.test("true", add_generation_prompt_0))(!J.t((add_generation_prompt_0 !== undefined))))) {
    out.push(J.str("\nAssistant: "));
    if (J.t(((v) => J.t(v) ? J.test("false", enable_thinking_0) : v)((enable_thinking_0 !== undefined)))) {
      out.push(J.str("<think>\n\n</think>\n\n"));
    }
    if (J.t(((v) => J.t(v) ? v : J.test("true", enable_thinking_0))(!J.t((enable_thinking_0 !== undefined))))) {
      out.push(J.str("<think>"));
    }
  }
  return out.join("");
}
