// Converted from the chat template of tomg-group-umd/huginn-0125 by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let loop_messages_0 = ctx["loop_messages"];
  let messages_0 = ctx["messages"];
  let start_content_0 = ctx["start_content"];
  let end_content_0 = ctx["end_content"];
  let bos_token_0 = ctx["bos_token"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  loop_messages_0 = messages_0;
  {
    let _items1 = J.iter(loop_messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      let start_content_2 = start_content_0;
      let end_content_3 = end_content_0;
      start_content_2 = "<|begin_header|>";
      end_content_3 = J.add(J.filter("trim", J.item(message_1, "content")), "<|end_turn|>");
      if (J.t((J.eq(J.attr(_loop1, "index0"), 0)))) {
        start_content_2 = J.add(bos_token_0, start_content_2);
      }
      if (J.t(((v) => J.t(v) ? v : (J.eq(J.item(message_1, "role"), "assistant")))((J.eq(J.item(message_1, "role"), "Huginn"))))) {
        start_content_2 = J.add(start_content_2, "Huginn<|end_header|>\n\n");
        out.push(J.str(start_content_2));
        {
          out.push(J.str(end_content_3));
        }
      } else {
        start_content_2 = J.add(J.add(start_content_2, J.item(message_1, "role")), "<|end_header|>\n\n");
        out.push(J.str(start_content_2), J.str(end_content_3));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|begin_header|>Huginn<|end_header|>\n\n"));
  } else {
    out.push(J.str("<|end_text|>"));
  }
  return out.join("");
}
