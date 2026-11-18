// Converted from the chat template of tiiuae/falcon-7b-instruct by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let loop_messages_0 = ctx["loop_messages"];
  let system_message_0 = ctx["system_message"];
  let messages_0 = ctx["messages"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
    loop_messages_0 = J.slice(messages_0, 1, undefined, undefined);
    system_message_0 = J.item(J.item(messages_0, 0), "content");
  } else {
    loop_messages_0 = messages_0;
    system_message_0 = "";
  }
  {
    let _items1 = J.iter(loop_messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      if (J.t((!J.eq((J.eq(J.item(message_1, "role"), "user")), (J.eq(J.mod(J.attr(_loop1, "index0"), 2), 0)))))) {
        out.push(J.str(J.call(J.raise_exception, "Conversation roles must alternate user/assistant/user/assistant/...")));
      }
      if (J.t((J.eq(J.attr(_loop1, "index0"), 0)))) {
        out.push(J.str(J.call(J.attr(system_message_0, "strip"))));
      }
      if (J.t((J.eq(J.item(message_1, "role"), "user")))) {
        out.push(J.str(J.add("\n\nUser: ", J.call(J.attr(J.call(J.attr(J.call(J.attr(J.item(message_1, "content"), "strip")), "replace"), "\n", "\n"), "replace"), "\n\n", "\n"))));
      } else if (J.t((J.eq(J.item(message_1, "role"), "assistant")))) {
        out.push(J.str(J.add("\n\nAssistant: ", J.call(J.attr(J.call(J.attr(J.call(J.attr(J.item(message_1, "content"), "strip")), "replace"), "\n", "\n"), "replace"), "\n\n", "\n"))));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("\n\nAssistant:"));
  }
  return out.join("");
}
