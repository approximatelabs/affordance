// Converted from the chat template of Qwen/Qwen2.5-Omni-7B by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let audio_count_0 = ctx["audio_count"];
  let image_count_0 = ctx["image_count"];
  let video_count_0 = ctx["video_count"];
  let messages_0 = ctx["messages"];
  let add_vision_id_0 = ctx["add_vision_id"];
  let add_audio_id_0 = ctx["add_audio_id"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  audio_count_0 = J.call(J.namespace, J.kw({"value": 0}));
  image_count_0 = J.call(J.namespace, J.kw({"value": 0}));
  video_count_0 = J.call(J.namespace, J.kw({"value": 0}));
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      if (J.t(((v) => J.t(v) ? (!J.eq(J.item(message_1, "role"), "system")) : v)(J.attr(_loop1, "first")))) {
        out.push("<|im_start|>system\nYou are a helpful assistant.<|im_end|>\n");
      }
      out.push("<|im_start|>", J.str(J.item(message_1, "role")), "\n");
      if (J.t(J.test("string", J.item(message_1, "content")))) {
        out.push(J.str(J.item(message_1, "content")), "<|im_end|>\n");
      } else {
        {
          let _items2 = J.iter(J.item(message_1, "content"));
          const _loop2 = J.loop(_items2);
          for (let _i2 = 0; _i2 < _items2.length; _i2++) {
            _loop2.index0 = _i2; const _it2 = _items2[_i2]; let content_2 = _it2;
            if (J.t(((v) => J.t(v) ? v : (J.has("image_url", content_2)))(((v) => J.t(v) ? v : (J.has("image", content_2)))((J.eq(J.item(content_2, "type"), "image")))))) {
              J.setattr(image_count_0, "value", J.add(J.attr(image_count_0, "value"), 1));
              if (J.t(add_vision_id_0)) {
                out.push("Picture ", J.str(J.attr(image_count_0, "value")), ": ");
              }
              out.push("<|vision_bos|><|IMAGE|><|vision_eos|>");
            } else if (J.t(((v) => J.t(v) ? v : (J.has("audio_url", content_2)))(((v) => J.t(v) ? v : (J.has("audio", content_2)))((J.eq(J.item(content_2, "type"), "audio")))))) {
              J.setattr(audio_count_0, "value", J.add(J.attr(audio_count_0, "value"), 1));
              if (J.t(add_audio_id_0)) {
                out.push("Audio ", J.str(J.attr(audio_count_0, "value")), ": ");
              }
              out.push("<|audio_bos|><|AUDIO|><|audio_eos|>");
            } else if (J.t(((v) => J.t(v) ? v : (J.has("video", content_2)))((J.eq(J.item(content_2, "type"), "video"))))) {
              J.setattr(video_count_0, "value", J.add(J.attr(video_count_0, "value"), 1));
              if (J.t(add_vision_id_0)) {
                out.push("Video ", J.str(J.attr(video_count_0, "value")), ": ");
              }
              out.push("<|vision_bos|><|VIDEO|><|vision_eos|>");
            } else if (J.t((J.has("text", content_2)))) {
              out.push(J.str(J.item(content_2, "text")));
            }
          }
        }
        out.push("<|im_end|>\n");
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push("<|im_start|>assistant\n");
  }
  return out.join("");
}
