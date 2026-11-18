// Converted from the chat template of nvidia/Llama-3_1-Nemotron-Ultra-253B-v1 by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let system_message_0 = ctx["system_message"];
  let messages_0 = ctx["messages"];
  let bos_token_0 = ctx["bos_token"];
  let content_0 = ctx["content"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  out.push(J.str(bos_token_0));
  if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
    system_message_0 = J.filter("trim", J.item(J.item(messages_0, 0), "content"));
    messages_0 = J.slice(messages_0, 1, undefined, undefined);
  } else {
    system_message_0 = "detailed thinking on";
  }
  out.push(J.str("<|start_header_id|>system<|end_header_id|>\n\n"), J.str(system_message_0), J.str("<|eot_id|>"));
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      let content_2 = content_0;
      if (J.t(((v) => J.t(v) ? (J.has("</think>", J.item(message_1, "content"))) : v)((J.eq(J.item(message_1, "role"), "assistant"))))) {
        content_2 = J.call(J.attr(J.item(J.call(J.attr(J.item(message_1, "content"), "split"), "</think>"), J.neg(1)), "lstrip"));
      } else {
        content_2 = J.item(message_1, "content");
      }
      out.push(J.str(J.add(J.add(J.add(J.add("<|start_header_id|>", J.item(message_1, "role")), "<|end_header_id|>\n\n"), J.filter("trim", content_2)), "<|eot_id|>")));
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|start_header_id|>assistant<|end_header_id|>\n\n"));
  }
  return out.join("");
}
