// Converted from the chat template of moonshotai/Kimi-K2-Base by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let tools_0 = ctx["tools"] ?? null;
  let messages_0 = ctx["messages"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  if (J.t(tools_0)) {
    out.push(J.str("<|im_system|>tool_declare<|im_middle|>"), J.str(J.filter("tojson", tools_0)), J.str("<|im_end|>"));
  }
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      if (J.t(((v) => J.t(v) ? (!J.eq(J.item(J.item(messages_0, 0), "role"), "system")) : v)(J.attr(_loop1, "first")))) {
        out.push(J.str("<|im_system|>system<|im_middle|>You are a helpful assistant<|im_end|>"));
      }
      if (J.t((J.eq(J.item(message_1, "role"), "system")))) {
        out.push(J.str("<|im_system|>system<|im_middle|>"));
      } else if (J.t((J.eq(J.item(message_1, "role"), "user")))) {
        out.push(J.str("<|im_user|>user<|im_middle|>"));
      } else if (J.t((J.eq(J.item(message_1, "role"), "assistant")))) {
        out.push(J.str("<|im_assistant|>assistant<|im_middle|>"));
      } else if (J.t((J.eq(J.item(message_1, "role"), "tool")))) {
        out.push(J.str("<|im_system|>tool<|im_middle|>"));
      }
      if (J.t(J.test("string", J.item(message_1, "content")))) {
        out.push(J.str(J.add(J.item(message_1, "content"), "<|im_end|>")));
      } else {
        {
          let _items2 = J.iter(J.item(message_1, "content"));
          const _loop2 = J.loop(_items2);
          for (let _i2 = 0; _i2 < _items2.length; _i2++) {
            _loop2.index0 = _i2; const _it2 = _items2[_i2]; let content_2 = _it2;
            if (J.t(((v) => J.t(v) ? v : (J.has("image_url", content_2)))(((v) => J.t(v) ? v : (J.has("image", content_2)))((J.eq(J.item(content_2, "type"), "image")))))) {
              out.push(J.str("<|media_start|>image<|media_content|><|media_pad|><|media_end|>"));
            } else {
              out.push(J.str(J.item(content_2, "text")));
            }
          }
        }
        out.push(J.str("<|im_end|>"));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|im_assistant|>assistant<|im_middle|>"));
  }
  return out.join("");
}
