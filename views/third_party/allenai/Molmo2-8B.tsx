// Converted from the chat template of allenai/Molmo2-8B by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let DEMO_STYLES_0 = ctx["DEMO_STYLES"];
  let image_count_0 = ctx["image_count"];
  let video_count_0 = ctx["video_count"];
  let has_subtitle_0 = ctx["has_subtitle"];
  let messages_0 = ctx["messages"];
  let role_0 = ctx["role"];
  let conv_index_0 = ctx["conv_index"];
  let text_content_0 = ctx["text_content"];
  let m_0 = ctx["m"];
  let seg_0 = ctx["seg"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  DEMO_STYLES_0 = ["point_count", "pointing", "cosyn_point", "user_qa", "long_caption", "short_caption", "video_long_caption", "video_short_caption", "video_point_track_per_frame", "video_point_track_start_end", "video_point_track_all_frames", "video_single_point_track_start_end", "video_transcript", "video_clip_caption_start_end", "video_clip_caption_start_end_in_seconds", "video_clip_transcript_start_end", "video_clip_transcript_start_end_in_seconds", "video_frame_caption_timestamp", "video_frame_caption_timestamp_in_seconds", "correction_qa", "text_sft", "video_point", "video_point_count", "video_count", "video_count_point", "multi_image_pointing", "multi_image_counting", "multi_image_point_then_count", "multi_image_count_then_point", "demo", "a_okvqa_mc", "ai2_diagram_no_letter", "ai2_diagram", "science_qa", "multi_image_mc", "multi_image_mc_exp", "mantis_instruct_mc", "video_multiple_choice", "video_multiple_choice_count_without_pointing", "video_multiple_choice_multiple_correct", "video_multiple_choice_w_subtitle"];
  image_count_0 = J.call(J.namespace, J.kw({"value": 0}));
  video_count_0 = J.call(J.namespace, J.kw({"value": 0}));
  has_subtitle_0 = ((v) => J.t(v) ? (J.eq(J.call(J.attr(J.item(J.item(messages_0, 0), "role"), "lower")), "subtitle")) : v)(messages_0);
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      if (J.t(!J.t(J.test("string", J.item(message_1, "content"))))) {
        {
          let _items2 = J.iter(J.item(message_1, "content"));
          const _loop2 = J.loop(_items2);
          for (let _i2 = 0; _i2 < _items2.length; _i2++) {
            _loop2.index0 = _i2; const _it2 = _items2[_i2]; let content_2 = _it2;
            if (J.t(((v) => J.t(v) ? v : (J.has("image_url", content_2)))(((v) => J.t(v) ? v : (J.has("image", content_2)))((J.eq(J.item(content_2, "type"), "image")))))) {
              J.setattr(image_count_0, "value", J.add(J.attr(image_count_0, "value"), 1));
            } else if (J.t(((v) => J.t(v) ? v : (J.has("video_url", content_2)))(((v) => J.t(v) ? v : (J.has("video", content_2)))((J.eq(J.item(content_2, "type"), "video")))))) {
              J.setattr(video_count_0, "value", J.add(J.attr(video_count_0, "value"), 1));
            }
          }
        }
      }
    }
  }
  if (J.t((J.eq(J.attr(image_count_0, "value"), 1)))) {
    out.push(J.str("<|image|>"));
  } else if (J.t((J.gt(J.attr(image_count_0, "value"), 1)))) {
    {
      let _items3 = J.iter(J.call(J.range, J.attr(image_count_0, "value")));
      const _loop3 = J.loop(_items3);
      for (let _i3 = 0; _i3 < _items3.length; _i3++) {
        _loop3.index0 = _i3; const _it3 = _items3[_i3]; let i_3 = _it3;
        out.push(J.str(J.concat("Image ", J.add(i_3, 1), "<|image|>")));
      }
    }
  }
  {
    let _items4 = J.iter(J.call(J.range, J.attr(video_count_0, "value")));
    const _loop4 = J.loop(_items4);
    for (let _i4 = 0; _i4 < _items4.length; _i4++) {
      _loop4.index0 = _i4; const _it4 = _items4[_i4]; let __4 = _it4;
      out.push(J.str("<|video|>"));
    }
  }
  if (J.t(has_subtitle_0)) {
    out.push(J.str(J.item(J.item(messages_0, 0), "content")));
  }
  {
    let _items5 = J.iter(messages_0);
    const _loop5 = J.loop(_items5);
    for (let _i5 = 0; _i5 < _items5.length; _i5++) {
      _loop5.index0 = _i5; const _it5 = _items5[_i5]; let message_5 = _it5;
      let role_6 = role_0;
      let conv_index_7 = conv_index_0;
      let text_content_8 = text_content_0;
      let m_9 = m_0;
      role_6 = J.call(J.attr(J.item(message_5, "role"), "lower"));
      if (J.t((J.eq(role_6, "subtitle")))) {
        continue;
      }
      conv_index_7 = J.sub(J.attr(_loop5, "index"), (J.t(has_subtitle_0) ? 1 : 0));
      if (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? (!J.eq(role_6, "assistant")) : v)((J.eq(J.mod(conv_index_7, 2), 0))))(((v) => J.t(v) ? (!J.eq(role_6, "user")) : v)((J.eq(J.mod(conv_index_7, 2), 1)))))) {
        out.push(J.str(J.call(J.raise_exception, "Conversation roles must alternate user/assistant/user/assistant/...")));
      }
      if (J.t(J.test("string", J.item(message_5, "content")))) {
        text_content_8 = J.item(message_5, "content");
      } else {
        m_9 = J.call(J.namespace, J.kw({"text": ""}));
        {
          let _items6 = J.iter(J.item(message_5, "content"));
          const _loop6 = J.loop(_items6);
          for (let _i6 = 0; _i6 < _items6.length; _i6++) {
            _loop6.index0 = _i6; const _it6 = _items6[_i6]; let content_10 = _it6;
            let seg_11 = seg_0;
            if (J.t((J.eq(J.item(content_10, "type"), "text")))) {
              if (J.t(((v) => J.t(v) ? (!J.has(J.item(content_10, "style"), DEMO_STYLES_0)) : v)((J.item(content_10, "style") !== undefined)))) {
                seg_11 = J.concat(J.item(content_10, "style"), ": ", J.item(content_10, "text"));
              } else {
                seg_11 = J.item(content_10, "text");
              }
              J.setattr(m_9, "text", J.concat(J.attr(m_9, "text"), (J.t(!J.t(J.attr(m_9, "text"))) ? "" : " "), seg_11));
            }
          }
        }
        text_content_8 = J.attr(m_9, "text");
      }
      if (J.t((J.eq(role_6, "user")))) {
        if (J.t(((v) => J.t(v) ? !J.t(((v) => J.t(v) ? J.attr(_loop5, "first") : v)(!J.t(has_subtitle_0))) : v)(!J.t(((v) => J.t(v) ? (J.eq(J.attr(_loop5, "index"), 2)) : v)(has_subtitle_0))))) {
          out.push(J.str("<|im_end|>\n"));
        }
        out.push(J.str("<|im_start|>user\n"), J.str(text_content_8), J.str("<|im_end|>\n"));
      } else {
        out.push(" ", J.str("<|im_start|>assistant\n"), J.str(text_content_8));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|im_start|>assistant\n"));
  }
  return out.join("");
}
