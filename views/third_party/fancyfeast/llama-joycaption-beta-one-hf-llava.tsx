// Converted from the chat template of fancyfeast/llama-joycaption-beta-one-hf-llava by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let date_string_0 = ctx["date_string"];
  let system_message_0 = ctx["system_message"];
  let messages_0 = ctx["messages"];
  let first_user_message_0 = ctx["first_user_message"];
  let bos_token_0 = ctx["bos_token"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  out.push(J.str(bos_token_0));
  if (J.t(!J.t((date_string_0 !== undefined)))) {
    date_string_0 = "26 July 2024";
  }
  if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
    system_message_0 = J.item(J.item(messages_0, 0), "content");
    messages_0 = J.slice(messages_0, 1, undefined, undefined);
  } else {
    system_message_0 = "";
  }
  out.push(J.str("<|start_header_id|>system<|end_header_id|>\n\n"), J.str("Cutting Knowledge Date: December 2023\n"), J.str(J.add(J.add("Today Date: ", date_string_0), "\n\n")), J.str(system_message_0), J.str("<|eot_id|>"));
  first_user_message_0 = true;
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      let first_user_message_2 = first_user_message_0;
      if (J.t(((v) => J.t(v) ? (J.eq(J.item(message_1, "role"), "user")) : v)(first_user_message_2))) {
        first_user_message_2 = false;
        out.push(J.str(J.add(J.add(J.add(J.add("<|start_header_id|>", J.item(message_1, "role")), "<|end_header_id|>\n\n<|reserved_special_token_70|><|reserved_special_token_69|><|reserved_special_token_71|>"), J.item(message_1, "content")), "<|eot_id|>")));
      } else {
        out.push(J.str(J.add(J.add(J.add(J.add("<|start_header_id|>", J.item(message_1, "role")), "<|end_header_id|>\n\n"), J.item(message_1, "content")), "<|eot_id|>")));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|start_header_id|>assistant<|end_header_id|>\n\n"));
  }
  return out.join("");
}
