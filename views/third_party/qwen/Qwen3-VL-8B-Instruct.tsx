// Converted from the chat template of Qwen/Qwen3-VL-8B-Instruct by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let image_count_0 = ctx["image_count"];
  let video_count_0 = ctx["video_count"];
  let tools_0 = ctx["tools"] ?? null;
  let messages_0 = ctx["messages"];
  let add_vision_id_0 = ctx["add_vision_id"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  if (J.t(tools_0)) {
    out.push(J.str("<|im_start|>system\n"));
    if (J.t((J.eq(J.attr(J.item(messages_0, 0), "role"), "system")))) {
      if (J.t(J.test("string", J.attr(J.item(messages_0, 0), "content")))) {
        out.push(J.str(J.attr(J.item(messages_0, 0), "content")));
      } else {
        {
          let _items1 = J.iter(J.attr(J.item(messages_0, 0), "content"));
          const _loop1 = J.loop(_items1);
          for (let _i1 = 0; _i1 < _items1.length; _i1++) {
            _loop1.index0 = _i1; const _it1 = _items1[_i1]; let content_1 = _it1;
            if (J.t((J.has("text", content_1)))) {
              out.push(J.str(J.attr(content_1, "text")));
            }
          }
        }
      }
      out.push(J.str("\n\n"));
    }
    out.push(J.str("# Tools\n\nYou may call one or more functions to assist with the user query.\n\nYou are provided with function signatures within <tools></tools> XML tags:\n<tools>"));
    {
      let _items2 = J.iter(tools_0);
      const _loop2 = J.loop(_items2);
      for (let _i2 = 0; _i2 < _items2.length; _i2++) {
        _loop2.index0 = _i2; const _it2 = _items2[_i2]; let tool_2 = _it2;
        out.push(J.str("\n"), J.str(J.filter("tojson", tool_2)));
      }
    }
    out.push(J.str("\n</tools>\n\nFor each function call, return a json object with function name and arguments within <tool_call></tool_call> XML tags:\n<tool_call>\n{\"name\": <function-name>, \"arguments\": <args-json-object>}\n</tool_call><|im_end|>\n"));
  } else {
    if (J.t((J.eq(J.attr(J.item(messages_0, 0), "role"), "system")))) {
      out.push(J.str("<|im_start|>system\n"));
      if (J.t(J.test("string", J.attr(J.item(messages_0, 0), "content")))) {
        out.push(J.str(J.attr(J.item(messages_0, 0), "content")));
      } else {
        {
          let _items3 = J.iter(J.attr(J.item(messages_0, 0), "content"));
          const _loop3 = J.loop(_items3);
          for (let _i3 = 0; _i3 < _items3.length; _i3++) {
            _loop3.index0 = _i3; const _it3 = _items3[_i3]; let content_3 = _it3;
            if (J.t((J.has("text", content_3)))) {
              out.push(J.str(J.attr(content_3, "text")));
            }
          }
        }
      }
      out.push(J.str("<|im_end|>\n"));
    }
  }
  image_count_0 = J.call(J.namespace, J.kw({"value": 0}));
  video_count_0 = J.call(J.namespace, J.kw({"value": 0}));
  {
    let _items4 = J.iter(messages_0);
    const _loop4 = J.loop(_items4);
    for (let _i4 = 0; _i4 < _items4.length; _i4++) {
      _loop4.index0 = _i4; const _it4 = _items4[_i4]; let message_4 = _it4;
      if (J.t((J.eq(J.attr(message_4, "role"), "user")))) {
        out.push(J.str(J.add(J.add("<|im_start|>", J.attr(message_4, "role")), "\n")));
        if (J.t(J.test("string", J.attr(message_4, "content")))) {
          out.push(J.str(J.attr(message_4, "content")));
        } else {
          {
            let _items5 = J.iter(J.attr(message_4, "content"));
            const _loop5 = J.loop(_items5);
            for (let _i5 = 0; _i5 < _items5.length; _i5++) {
              _loop5.index0 = _i5; const _it5 = _items5[_i5]; let content_5 = _it5;
              if (J.t(((v) => J.t(v) ? v : (J.has("image_url", content_5)))(((v) => J.t(v) ? v : (J.has("image", content_5)))((J.eq(J.attr(content_5, "type"), "image")))))) {
                J.setattr(image_count_0, "value", J.add(J.attr(image_count_0, "value"), 1));
                if (J.t(add_vision_id_0)) {
                  out.push("Picture ", J.str(J.attr(image_count_0, "value")), ": ");
                }
                out.push("<|vision_start|><|image_pad|><|vision_end|>");
              } else if (J.t(((v) => J.t(v) ? v : (J.has("video", content_5)))((J.eq(J.attr(content_5, "type"), "video"))))) {
                J.setattr(video_count_0, "value", J.add(J.attr(video_count_0, "value"), 1));
                if (J.t(add_vision_id_0)) {
                  out.push("Video ", J.str(J.attr(video_count_0, "value")), ": ");
                }
                out.push("<|vision_start|><|video_pad|><|vision_end|>");
              } else if (J.t((J.has("text", content_5)))) {
                out.push(J.str(J.attr(content_5, "text")));
              }
            }
          }
        }
        out.push(J.str("<|im_end|>\n"));
      } else if (J.t((J.eq(J.attr(message_4, "role"), "assistant")))) {
        out.push(J.str(J.add(J.add("<|im_start|>", J.attr(message_4, "role")), "\n")));
        if (J.t(J.test("string", J.attr(message_4, "content")))) {
          out.push(J.str(J.attr(message_4, "content")));
        } else {
          {
            let _items6 = J.iter(J.attr(message_4, "content"));
            const _loop6 = J.loop(_items6);
            for (let _i6 = 0; _i6 < _items6.length; _i6++) {
              _loop6.index0 = _i6; const _it6 = _items6[_i6]; let content_item_6 = _it6;
              if (J.t((J.has("text", content_item_6)))) {
                out.push(J.str(J.attr(content_item_6, "text")));
              }
            }
          }
        }
        if (J.t(J.attr(message_4, "tool_calls"))) {
          {
            let _items7 = J.iter(J.attr(message_4, "tool_calls"));
            const _loop7 = J.loop(_items7);
            for (let _i7 = 0; _i7 < _items7.length; _i7++) {
              _loop7.index0 = _i7; const _it7 = _items7[_i7]; let tool_call_7 = _it7;
              if (J.t(((v) => J.t(v) ? v : !J.t(J.attr(_loop7, "first")))(((v) => J.t(v) ? J.attr(message_4, "content") : v)(J.attr(_loop7, "first"))))) {
                out.push(J.str("\n"));
              }
              if (J.t(J.attr(tool_call_7, "function"))) {
                tool_call_7 = J.attr(tool_call_7, "function");
              }
              out.push(J.str("<tool_call>\n{\"name\": \""), J.str(J.attr(tool_call_7, "name")), J.str("\", \"arguments\": "));
              if (J.t(J.test("string", J.attr(tool_call_7, "arguments")))) {
                out.push(J.str(J.attr(tool_call_7, "arguments")));
              } else {
                out.push(J.str(J.filter("tojson", J.attr(tool_call_7, "arguments"))));
              }
              out.push(J.str("}\n</tool_call>"));
            }
          }
        }
        out.push(J.str("<|im_end|>\n"));
      } else if (J.t((J.eq(J.attr(message_4, "role"), "tool")))) {
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.sub(J.attr(_loop4, "index0"), 1)), "role"), "tool")))(J.attr(_loop4, "first")))) {
          out.push(J.str("<|im_start|>user"));
        }
        out.push(J.str("\n<tool_response>\n"));
        if (J.t(J.test("string", J.attr(message_4, "content")))) {
          out.push(J.str(J.attr(message_4, "content")));
        } else {
          {
            let _items8 = J.iter(J.attr(message_4, "content"));
            const _loop8 = J.loop(_items8);
            for (let _i8 = 0; _i8 < _items8.length; _i8++) {
              _loop8.index0 = _i8; const _it8 = _items8[_i8]; let content_8 = _it8;
              if (J.t(((v) => J.t(v) ? v : (J.has("image_url", content_8)))(((v) => J.t(v) ? v : (J.has("image", content_8)))((J.eq(J.attr(content_8, "type"), "image")))))) {
                J.setattr(image_count_0, "value", J.add(J.attr(image_count_0, "value"), 1));
                if (J.t(add_vision_id_0)) {
                  out.push("Picture ", J.str(J.attr(image_count_0, "value")), ": ");
                }
                out.push("<|vision_start|><|image_pad|><|vision_end|>");
              } else if (J.t(((v) => J.t(v) ? v : (J.has("video", content_8)))((J.eq(J.attr(content_8, "type"), "video"))))) {
                J.setattr(video_count_0, "value", J.add(J.attr(video_count_0, "value"), 1));
                if (J.t(add_vision_id_0)) {
                  out.push("Video ", J.str(J.attr(video_count_0, "value")), ": ");
                }
                out.push("<|vision_start|><|video_pad|><|vision_end|>");
              } else if (J.t((J.has("text", content_8)))) {
                out.push(J.str(J.attr(content_8, "text")));
              }
            }
          }
        }
        out.push(J.str("\n</tool_response>"));
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.add(J.attr(_loop4, "index0"), 1)), "role"), "tool")))(J.attr(_loop4, "last")))) {
          out.push(J.str("<|im_end|>\n"));
        }
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|im_start|>assistant\n"));
  }
  return out.join("");
}
