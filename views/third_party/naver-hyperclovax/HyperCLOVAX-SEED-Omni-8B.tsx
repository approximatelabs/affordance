// Converted from the chat template of naver-hyperclovax/HyperCLOVAX-SEED-Omni-8B by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let ns_img_0 = ctx["ns_img"];
  let ns_aud_0 = ctx["ns_aud"];
  let ns_vid_0 = ctx["ns_vid"];
  let ns_0 = ctx["ns"];
  let tools_0 = ctx["tools"] ?? null;
  let messages_0 = ctx["messages"];
  let content_0 = ctx["content"];
  let reasoning_content_0 = ctx["reasoning_content"];
  let media_url_0 = ctx["media_url"];
  let url_lower_0 = ctx["url_lower"];
  let image_extensions_0 = ctx["image_extensions"];
  let video_extensions_0 = ctx["video_extensions"];
  let ns_check_0 = ctx["ns_check"];
  let video_id_0 = ctx["video_id"];
  let image_id_0 = ctx["image_id"];
  let audio_id_0 = ctx["audio_id"];
  let input_audio_0 = ctx["input_audio"];
  let _args_0 = ctx["_args"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let skip_reasoning_0 = ctx["skip_reasoning"];
  ns_img_0 = J.call(J.namespace, J.kw({"count": 0}));
  ns_aud_0 = J.call(J.namespace, J.kw({"count": 0}));
  ns_vid_0 = J.call(J.namespace, J.kw({"count": 0}));
  if (J.t(tools_0)) {
    out.push(J.str("<|im_start|>system\n"));
    if (J.t((J.eq(J.attr(J.item(messages_0, 0), "role"), "system")))) {
      if (J.t(J.test("string", J.attr(J.item(messages_0, 0), "content")))) {
        out.push(J.str(J.add(J.attr(J.item(messages_0, 0), "content"), "\n\n")));
      } else if (J.t(J.test("sequence", J.attr(J.item(messages_0, 0), "content")))) {
        {
          let _items1 = J.iter(J.attr(J.item(messages_0, 0), "content"));
          const _loop1 = J.loop(_items1);
          for (let _i1 = 0; _i1 < _items1.length; _i1++) {
            _loop1.index0 = _i1; const _it1 = _items1[_i1]; let part_1 = _it1;
            if (J.t((J.eq(J.attr(part_1, "type"), "text")))) {
              out.push(J.str(J.attr(part_1, "text")));
            }
          }
        }
        out.push(J.str("\n\n"));
      }
    }
    out.push(J.str("# Tools\n\n"), J.str("You may call one or more functions to assist with the user query.\n\n"), J.str("You are provided with function signatures within <tools></tools> XML tags:\n"), J.str("<tools>\n"));
    {
      let _items2 = J.iter(tools_0);
      const _loop2 = J.loop(_items2);
      for (let _i2 = 0; _i2 < _items2.length; _i2++) {
        _loop2.index0 = _i2; const _it2 = _items2[_i2]; let tool_2 = _it2;
        out.push(J.str(J.filter("tojson", tool_2)));
      }
    }
    out.push(J.str("\n</tools>\n\n"), J.str("For each function call, output the function name and arguments within the following XML format:\n"), J.str("<tool_call>{function-name}\n"), J.str("<arg_key>{arg-key-1}</arg_key>\n"), J.str("<arg_value>{arg-value-1}</arg_value>\n"), J.str("<arg_key>{arg-key-2}</arg_key>\n"), J.str("<arg_value>{arg-value-2}</arg_value>\n"), J.str("...\n"), J.str("</tool_call><|im_end|>\n"));
  } else {
    if (J.t((J.eq(J.attr(J.item(messages_0, 0), "role"), "system")))) {
      out.push(J.str("<|im_start|>system\n"));
      if (J.t(J.test("string", J.attr(J.item(messages_0, 0), "content")))) {
        out.push(J.str(J.attr(J.item(messages_0, 0), "content")));
      } else if (J.t(J.test("sequence", J.attr(J.item(messages_0, 0), "content")))) {
        {
          let _items3 = J.iter(J.attr(J.item(messages_0, 0), "content"));
          const _loop3 = J.loop(_items3);
          for (let _i3 = 0; _i3 < _items3.length; _i3++) {
            _loop3.index0 = _i3; const _it3 = _items3[_i3]; let part_3 = _it3;
            if (J.t((J.eq(J.attr(part_3, "type"), "text")))) {
              out.push(J.str(J.attr(part_3, "text")));
            }
          }
        }
      }
      out.push(J.str("<|im_end|>\n"));
    }
  }
  ns_0 = J.call(J.namespace, J.kw({"last_user_index": J.neg(1)}));
  {
    let _items4 = J.iter(messages_0);
    const _loop4 = J.loop(_items4);
    for (let _i4 = 0; _i4 < _items4.length; _i4++) {
      _loop4.index0 = _i4; const _it4 = _items4[_i4]; let m_4 = _it4;
      if (J.t((J.eq(J.attr(m_4, "role"), "user")))) {
        J.setattr(ns_0, "last_user_index", J.attr(_loop4, "index0"));
      }
    }
  }
  {
    let _items5 = J.iter(messages_0);
    const _loop5 = J.loop(_items5);
    for (let _i5 = 0; _i5 < _items5.length; _i5++) {
      _loop5.index0 = _i5; const _it5 = _items5[_i5]; let message_5 = _it5;
      let content_6 = content_0;
      let reasoning_content_7 = reasoning_content_0;
      content_6 = J.attr(message_5, "content");
      if (J.t(((v) => J.t(v) ? !J.t(J.attr(_loop5, "first")) : v)((J.eq(J.attr(message_5, "role"), "system"))))) {
        out.push(J.str(J.add(J.add("<|im_start|>", J.attr(message_5, "role")), "\n")));
        if (J.t(J.test("string", content_6))) {
          out.push(J.str(content_6));
        } else if (J.t(J.test("sequence", content_6))) {
          {
            let _items6 = J.iter(content_6);
            const _loop6 = J.loop(_items6);
            for (let _i6 = 0; _i6 < _items6.length; _i6++) {
              _loop6.index0 = _i6; const _it6 = _items6[_i6]; let part_8 = _it6;
              if (J.t((J.eq(J.attr(part_8, "type"), "text")))) {
                out.push(J.str(J.attr(part_8, "text")));
              }
            }
          }
        }
        out.push(J.str(J.add("<|im_end|>", "\n")));
      } else if (J.t((J.eq(J.attr(message_5, "role"), "user")))) {
        out.push(J.str("<|im_start|>user\n"));
        if (J.t(J.test("string", J.item(message_5, "content")))) {
          out.push(J.str(J.add(J.item(message_5, "content"), "<|im_end|>\n")));
        } else if (J.t(J.test("sequence", J.item(message_5, "content")))) {
          {
            let _items7 = J.iter(J.item(message_5, "content"));
            const _loop7 = J.loop(_items7);
            for (let _i7 = 0; _i7 < _items7.length; _i7++) {
              _loop7.index0 = _i7; const _it7 = _items7[_i7]; let content_9 = _it7;
              let media_url_10 = media_url_0;
              let url_lower_11 = url_lower_0;
              let image_extensions_12 = image_extensions_0;
              let video_extensions_13 = video_extensions_0;
              let ns_check_14 = ns_check_0;
              let video_id_15 = video_id_0;
              let image_id_16 = image_id_0;
              let audio_id_17 = audio_id_0;
              let input_audio_18 = input_audio_0;
              if (J.t(!J.t(J.attr(_loop7, "first")))) {
                out.push(J.str("\n"));
              }
              if (J.t((J.eq(J.item(content_9, "type"), "image_url")))) {
                media_url_10 = J.call(J.attr(J.call(J.attr(content_9, "get"), "image_url", ({})), "get"), "url", "");
                url_lower_11 = J.call(J.attr(media_url_10, "lower"));
                image_extensions_12 = [".jpg", ".jpeg", ".png", ".gif", ".bmp", ".webp", ".tiff", ".svg"];
                video_extensions_13 = [".mp4", ".avi", ".mov", ".mkv", ".webm", ".flv", ".wmv", ".m4v"];
                ns_check_14 = J.call(J.namespace, J.kw({"is_video": false}));
                {
                  let _items8 = J.iter(video_extensions_13);
                  const _loop8 = J.loop(_items8);
                  for (let _i8 = 0; _i8 < _items8.length; _i8++) {
                    _loop8.index0 = _i8; const _it8 = _items8[_i8]; let ext_19 = _it8;
                    if (J.t(J.call(J.attr(url_lower_11, "endswith"), ext_19))) {
                      J.setattr(ns_check_14, "is_video", true);
                    }
                  }
                }
                if (J.t(J.attr(ns_check_14, "is_video"))) {
                  video_id_15 = J.mod("video_%02d", J.attr(ns_vid_0, "count"));
                  J.setattr(ns_vid_0, "count", J.add(J.attr(ns_vid_0, "count"), 1));
                  out.push(J.str(J.add(J.add("<|mime_start|>{\"id\": \"", video_id_15), "\", \"type\": \"video/mp4\", \"filename\": \"video.mp4\"}<|mime_end|>\n")), J.str("<|video_aux_start|>\ub2e4\uc74c \uc911 video_duration\uc740 \ube44\ub514\uc624 \uae38\uc774 \uc815\ubcf4\uc785\ub2c8\ub2e4. \ucc38\uace0\ud558\uc5ec \ub2f5\ubcc0\ud558\uc138\uc694. {\"video_duration\": \"<|video_meta_duration|>\"}<|video_aux_end|>\n"), J.str("<|video_start|><|VIDEO_PAD|><|video_end|>"));
                } else {
                  image_id_16 = J.mod("image_%02d", J.attr(ns_img_0, "count"));
                  J.setattr(ns_img_0, "count", J.add(J.attr(ns_img_0, "count"), 1));
                  out.push(J.str(J.add(J.add("<|mime_start|>{\"id\": \"", image_id_16), "\", \"type\": \"image/jpeg\", \"filename\": \"image.jpg\"}<|mime_end|>\n")), J.str("<|discrete_image_start|><|DISCRETE_IMAGE_PAD|><|discrete_image_end|>\n"), J.str("<|image_start|><|IMAGE_PAD|><|image_end|>"));
                }
              } else if (J.t((J.eq(J.item(content_9, "type"), "input_audio")))) {
                audio_id_17 = J.mod("audio_%02d", J.attr(ns_aud_0, "count"));
                J.setattr(ns_aud_0, "count", J.add(J.attr(ns_aud_0, "count"), 1));
                input_audio_18 = J.call(J.attr(content_9, "get"), "input_audio", ({}));
                out.push(J.str(J.add(J.add("<|mime_start|>{\"id\": \"", audio_id_17), "\", \"type\": \"audio/mpeg\", \"filename\": \"user_query.wav\"}<|mime_end|>\n")), J.str("<|audio_aux_start|>\ub2e4\uc74c \uc911 audio_duration\uc740 \uc624\ub514\uc624 \uae38\uc774 \uc815\ubcf4\uc785\ub2c8\ub2e4. \ucc38\uace0\ud558\uc5ec \ub2f5\ubcc0\ud558\uc138\uc694. {\"audio_duration\": \"<|audio_meta_duration|>\"}<|audio_aux_end|>\n"), J.str("<|discrete_audio_start|><|DISCRETE_AUDIO_PAD|><|discrete_audio_end|>\n"), J.str("<|audio_start|><|AUDIO_PAD|><|audio_end|>"));
              } else if (J.t((J.eq(J.item(content_9, "type"), "text")))) {
                out.push(J.str(J.item(content_9, "text")));
              }
            }
          }
          out.push(J.str("<|im_end|>\n"));
        }
      } else if (J.t((J.eq(J.attr(message_5, "role"), "assistant")))) {
        reasoning_content_7 = "";
        if (J.t(J.test("string", J.call(J.attr(message_5, "get"), "reasoning_content")))) {
          reasoning_content_7 = J.call(J.attr(message_5, "get"), "reasoning_content");
        } else {
          if (J.t((J.has("</think>", content_6)))) {
            reasoning_content_7 = J.call(J.attr(J.item(J.call(J.attr(J.call(J.attr(J.item(J.call(J.attr(content_6, "split"), "</think>"), 0), "rstrip"), "\n"), "split"), "<think>"), J.neg(1)), "lstrip"), "\n");
            content_6 = J.call(J.attr(J.item(J.call(J.attr(content_6, "split"), "</think>"), J.neg(1)), "lstrip"), "\n");
          }
        }
        if (J.t((J.gt(J.attr(_loop5, "index0"), J.attr(ns_0, "last_user_index"))))) {
          if (J.t(((v) => J.t(v) ? v : reasoning_content_7)(J.attr(_loop5, "last")))) {
            out.push(J.str(J.add(J.add(J.add(J.add("<|im_start|>", J.attr(message_5, "role")), "\n<think>\n"), J.call(J.attr(reasoning_content_7, "strip"), "\n")), "\n</think>\n\n")));
          } else {
            out.push(J.str(J.add(J.add("<|im_start|>", J.attr(message_5, "role")), "\n")));
          }
        } else {
          out.push(J.str(J.add(J.add("<|im_start|>", J.attr(message_5, "role")), "\n")));
        }
        out.push(J.str(content_6));
        if (J.t(J.call(J.attr(message_5, "get"), "tool_calls"))) {
          {
            let _items9 = J.iter(J.call(J.attr(message_5, "get"), "tool_calls", []));
            const _loop9 = J.loop(_items9);
            for (let _i9 = 0; _i9 < _items9.length; _i9++) {
              _loop9.index0 = _i9; const _it9 = _items9[_i9]; let tool_call_20 = _it9;
              let _args_21 = _args_0;
              if (J.t(((v) => J.t(v) ? v : content_6)(!J.t(J.attr(_loop9, "first"))))) {
                out.push(J.str("\n"));
              }
              if (J.t(J.attr(tool_call_20, "function"))) {
                tool_call_20 = J.attr(tool_call_20, "function");
              }
              out.push(J.str(J.add(J.add("<tool_call>", J.attr(tool_call_20, "name")), "\n")));
              _args_21 = J.attr(tool_call_20, "arguments");
              {
                let _items10 = J.iter(J.call(J.attr(_args_21, "items")));
                const _loop10 = J.loop(_items10);
                for (let _i10 = 0; _i10 < _items10.length; _i10++) {
                  _loop10.index0 = _i10; const _it10 = _items10[_i10]; let [k_22, v_23] = J.unpack(_it10, 2);
                  out.push(J.str(J.add(J.add("<arg_key>", k_22), "</arg_key>\n")), J.str(J.add(J.add("<arg_value>", (J.t(!J.t(J.test("string", v_23))) ? J.filter("tojson", v_23) : v_23)), "</arg_value>\n")));
                }
              }
              out.push(J.str("</tool_call>"));
            }
          }
        }
        out.push(J.str("<|im_end|>\n"));
      } else if (J.t((J.eq(J.attr(message_5, "role"), "tool")))) {
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.sub(J.attr(_loop5, "index0"), 1)), "role"), "tool")))(J.attr(_loop5, "first")))) {
          out.push(J.str("<|im_start|>tool"));
        }
        out.push(J.str(J.add(J.add("\n<tool_response>", J.call(J.attr(message_5, "get"), "name", "")), "\n")));
        if (J.t(J.test("string", J.item(message_5, "content")))) {
          out.push(J.str(content_6));
        }
        out.push(J.str("\n</tool_response>"));
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.add(J.attr(_loop5, "index0"), 1)), "role"), "tool")))(J.attr(_loop5, "last")))) {
          out.push(J.str("<|im_end|>\n"));
        }
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|im_start|>assistant\n<think>\n"));
    if (J.t(((v) => J.t(v) ? J.test("true", skip_reasoning_0) : v)((skip_reasoning_0 !== undefined)))) {
      out.push(J.str("\n</think>\n\n"));
    }
  }
  return out.join("");
}
