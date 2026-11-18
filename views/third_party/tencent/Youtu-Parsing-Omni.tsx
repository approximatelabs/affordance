// Converted from the chat template of tencent/Youtu-Parsing-Omni by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let enable_parsing_0 = ctx["enable_parsing"];
  let messages_0 = ctx["messages"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let enable_thinking_0 = ctx["enable_thinking"];
  if (J.t(((v) => J.t(v) ? J.test("true", enable_parsing_0) : v)((enable_parsing_0 !== undefined)))) {
    {
      let _items1 = J.iter(messages_0);
      const _loop1 = J.loop(_items1);
      for (let _i1 = 0; _i1 < _items1.length; _i1++) {
        _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
        if (J.t((J.eq(J.attr(message_1, "role"), "user")))) {
          if (J.t(J.test("string", J.attr(message_1, "content")))) {
            out.push(J.str(J.attr(message_1, "content")));
          } else {
            {
              let _items2 = J.iter(J.attr(message_1, "content"));
              const _loop2 = J.loop(_items2);
              for (let _i2 = 0; _i2 < _items2.length; _i2++) {
                _loop2.index0 = _i2; const _it2 = _items2[_i2]; let item_2 = _it2;
                if (J.t(((v) => J.t(v) ? v : (J.has("video_url", item_2)))(((v) => J.t(v) ? v : (J.has("video", item_2)))(((v) => J.t(v) ? v : (J.eq(J.attr(item_2, "type"), "video_url")))((J.eq(J.attr(item_2, "type"), "video"))))))) {
                  out.push(J.str("<|video|>"));
                } else if (J.t(((v) => J.t(v) ? v : (J.has("image_url", item_2)))(((v) => J.t(v) ? v : (J.has("image", item_2)))(((v) => J.t(v) ? v : (J.eq(J.attr(item_2, "type"), "image_url")))((J.eq(J.attr(item_2, "type"), "image"))))))) {
                  out.push(J.str("<|image|>"));
                } else if (J.t(((v) => J.t(v) ? v : (J.has("audio_url", item_2)))(((v) => J.t(v) ? v : (J.has("audio", item_2)))(((v) => J.t(v) ? v : (J.eq(J.attr(item_2, "type"), "audio_url")))((J.eq(J.attr(item_2, "type"), "audio"))))))) {
                  out.push(J.str("<|audio|>"));
                } else if (J.t((J.eq(J.attr(item_2, "type"), "text")))) {
                  out.push(J.str(J.attr(item_2, "text")));
                }
              }
            }
          }
        } else if (J.t((J.eq(J.attr(message_1, "role"), "assistant")))) {
          if (J.t(J.test("string", J.attr(message_1, "content")))) {
            out.push(J.str(J.attr(message_1, "content")));
          } else {
            {
              let _items3 = J.iter(J.attr(message_1, "content"));
              const _loop3 = J.loop(_items3);
              for (let _i3 = 0; _i3 < _items3.length; _i3++) {
                _loop3.index0 = _i3; const _it3 = _items3[_i3]; let item_3 = _it3;
                if (J.t(((v) => J.t(v) ? v : (J.has("video_url", item_3)))(((v) => J.t(v) ? v : (J.has("video", item_3)))(((v) => J.t(v) ? v : (J.eq(J.attr(item_3, "type"), "video_url")))((J.eq(J.attr(item_3, "type"), "video"))))))) {
                  out.push(J.str("<|video|>"));
                } else if (J.t(((v) => J.t(v) ? v : (J.has("image_url", item_3)))(((v) => J.t(v) ? v : (J.has("image", item_3)))(((v) => J.t(v) ? v : (J.eq(J.attr(item_3, "type"), "image_url")))((J.eq(J.attr(item_3, "type"), "image"))))))) {
                  out.push(J.str("<|image|>"));
                } else if (J.t(((v) => J.t(v) ? v : (J.has("audio_url", item_3)))(((v) => J.t(v) ? v : (J.has("audio", item_3)))(((v) => J.t(v) ? v : (J.eq(J.attr(item_3, "type"), "audio_url")))((J.eq(J.attr(item_3, "type"), "audio"))))))) {
                  out.push(J.str("<|audio|>"));
                } else if (J.t((J.eq(J.attr(item_3, "type"), "text")))) {
                  out.push(J.str(J.attr(item_3, "text")));
                }
              }
            }
          }
        } else if (J.t((J.eq(J.attr(message_1, "role"), "system")))) {
          if (J.t(J.test("string", J.attr(message_1, "content")))) {
            out.push(J.str(J.attr(message_1, "content")));
          } else {
            {
              let _items4 = J.iter(J.attr(message_1, "content"));
              const _loop4 = J.loop(_items4);
              for (let _i4 = 0; _i4 < _items4.length; _i4++) {
                _loop4.index0 = _i4; const _it4 = _items4[_i4]; let item_4 = _it4;
                if (J.t(((v) => J.t(v) ? v : (J.has("video_url", item_4)))(((v) => J.t(v) ? v : (J.has("video", item_4)))(((v) => J.t(v) ? v : (J.eq(J.attr(item_4, "type"), "video_url")))((J.eq(J.attr(item_4, "type"), "video"))))))) {
                  out.push(J.str("<|video|>"));
                } else if (J.t(((v) => J.t(v) ? v : (J.has("image_url", item_4)))(((v) => J.t(v) ? v : (J.has("image", item_4)))(((v) => J.t(v) ? v : (J.eq(J.attr(item_4, "type"), "image_url")))((J.eq(J.attr(item_4, "type"), "image"))))))) {
                  out.push(J.str("<|image|>"));
                } else if (J.t(((v) => J.t(v) ? v : (J.has("audio_url", item_4)))(((v) => J.t(v) ? v : (J.has("audio", item_4)))(((v) => J.t(v) ? v : (J.eq(J.attr(item_4, "type"), "audio_url")))((J.eq(J.attr(item_4, "type"), "audio"))))))) {
                  out.push(J.str("<|audio|>"));
                } else if (J.t((J.eq(J.attr(item_4, "type"), "text")))) {
                  out.push(J.str(J.attr(item_4, "text")));
                }
              }
            }
          }
        }
      }
    }
  } else {
    {
      let _items5 = J.iter(messages_0);
      const _loop5 = J.loop(_items5);
      for (let _i5 = 0; _i5 < _items5.length; _i5++) {
        _loop5.index0 = _i5; const _it5 = _items5[_i5]; let message_5 = _it5;
        if (J.t((J.eq(J.attr(message_5, "role"), "user")))) {
          out.push(J.str("<|begin_of_text|>user\n"));
          if (J.t(J.test("string", J.attr(message_5, "content")))) {
            out.push(J.str(J.attr(message_5, "content")));
          } else {
            {
              let _items6 = J.iter(J.attr(message_5, "content"));
              const _loop6 = J.loop(_items6);
              for (let _i6 = 0; _i6 < _items6.length; _i6++) {
                _loop6.index0 = _i6; const _it6 = _items6[_i6]; let item_6 = _it6;
                if (J.t(((v) => J.t(v) ? v : (J.has("video_url", item_6)))(((v) => J.t(v) ? v : (J.has("video", item_6)))(((v) => J.t(v) ? v : (J.eq(J.attr(item_6, "type"), "video_url")))((J.eq(J.attr(item_6, "type"), "video"))))))) {
                  out.push(J.str("<|video|>"));
                } else if (J.t(((v) => J.t(v) ? v : (J.has("image_url", item_6)))(((v) => J.t(v) ? v : (J.has("image", item_6)))(((v) => J.t(v) ? v : (J.eq(J.attr(item_6, "type"), "image_url")))((J.eq(J.attr(item_6, "type"), "image"))))))) {
                  out.push(J.str("<|image|>"));
                } else if (J.t(((v) => J.t(v) ? v : (J.has("audio_url", item_6)))(((v) => J.t(v) ? v : (J.has("audio", item_6)))(((v) => J.t(v) ? v : (J.eq(J.attr(item_6, "type"), "audio_url")))((J.eq(J.attr(item_6, "type"), "audio"))))))) {
                  out.push(J.str("<|audio|>"));
                } else if (J.t((J.eq(J.attr(item_6, "type"), "text")))) {
                  out.push(J.str(J.attr(item_6, "text")));
                }
              }
            }
          }
        } else if (J.t((J.eq(J.attr(message_5, "role"), "assistant")))) {
          out.push(J.str("<|begin_of_text|>assistant\n"));
          if (J.t(J.test("string", J.attr(message_5, "content")))) {
            out.push(J.str(J.attr(message_5, "content")));
          } else {
            {
              let _items7 = J.iter(J.attr(message_5, "content"));
              const _loop7 = J.loop(_items7);
              for (let _i7 = 0; _i7 < _items7.length; _i7++) {
                _loop7.index0 = _i7; const _it7 = _items7[_i7]; let item_7 = _it7;
                if (J.t(((v) => J.t(v) ? v : (J.has("video_url", item_7)))(((v) => J.t(v) ? v : (J.has("video", item_7)))(((v) => J.t(v) ? v : (J.eq(J.attr(item_7, "type"), "video_url")))((J.eq(J.attr(item_7, "type"), "video"))))))) {
                  out.push(J.str("<|video|>"));
                } else if (J.t(((v) => J.t(v) ? v : (J.has("image_url", item_7)))(((v) => J.t(v) ? v : (J.has("image", item_7)))(((v) => J.t(v) ? v : (J.eq(J.attr(item_7, "type"), "image_url")))((J.eq(J.attr(item_7, "type"), "image"))))))) {
                  out.push(J.str("<|image|>"));
                } else if (J.t(((v) => J.t(v) ? v : (J.has("audio_url", item_7)))(((v) => J.t(v) ? v : (J.has("audio", item_7)))(((v) => J.t(v) ? v : (J.eq(J.attr(item_7, "type"), "audio_url")))((J.eq(J.attr(item_7, "type"), "audio"))))))) {
                  out.push(J.str("<|audio|>"));
                } else if (J.t((J.eq(J.attr(item_7, "type"), "text")))) {
                  out.push(J.str(J.attr(item_7, "text")));
                }
              }
            }
          }
        } else if (J.t((J.eq(J.attr(message_5, "role"), "system")))) {
          out.push(J.str("<|begin_of_text|>system\n"));
          if (J.t(J.test("string", J.attr(message_5, "content")))) {
            out.push(J.str(J.attr(message_5, "content")));
          } else {
            {
              let _items8 = J.iter(J.attr(message_5, "content"));
              const _loop8 = J.loop(_items8);
              for (let _i8 = 0; _i8 < _items8.length; _i8++) {
                _loop8.index0 = _i8; const _it8 = _items8[_i8]; let item_8 = _it8;
                if (J.t(((v) => J.t(v) ? v : (J.has("video_url", item_8)))(((v) => J.t(v) ? v : (J.has("video", item_8)))(((v) => J.t(v) ? v : (J.eq(J.attr(item_8, "type"), "video_url")))((J.eq(J.attr(item_8, "type"), "video"))))))) {
                  out.push(J.str("<|video|>"));
                } else if (J.t(((v) => J.t(v) ? v : (J.has("image_url", item_8)))(((v) => J.t(v) ? v : (J.has("image", item_8)))(((v) => J.t(v) ? v : (J.eq(J.attr(item_8, "type"), "image_url")))((J.eq(J.attr(item_8, "type"), "image"))))))) {
                  out.push(J.str("<|image|>"));
                } else if (J.t(((v) => J.t(v) ? v : (J.has("audio_url", item_8)))(((v) => J.t(v) ? v : (J.has("audio", item_8)))(((v) => J.t(v) ? v : (J.eq(J.attr(item_8, "type"), "audio_url")))((J.eq(J.attr(item_8, "type"), "audio"))))))) {
                  out.push(J.str("<|audio|>"));
                } else if (J.t((J.eq(J.attr(item_8, "type"), "text")))) {
                  out.push(J.str(J.attr(item_8, "text")));
                }
              }
            }
          }
        }
        out.push(J.str("<|end_of_text|>\n"));
      }
    }
    if (J.t(add_generation_prompt_0)) {
      out.push(J.str("<|begin_of_text|>assistant\n"));
      if (J.t(((v) => J.t(v) ? J.test("false", enable_thinking_0) : v)((enable_thinking_0 !== undefined)))) {
        out.push(J.str("<think>\n\n</think>\n\n"));
      }
    }
  }
  return out.join("");
}
