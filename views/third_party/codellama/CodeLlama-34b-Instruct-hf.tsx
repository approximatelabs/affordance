// Converted from the chat template of codellama/CodeLlama-34b-Instruct-hf by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let loop_messages_0 = ctx["loop_messages"];
  let system_message_0 = ctx["system_message"];
  let messages_0 = ctx["messages"];
  let content_0 = ctx["content"];
  let bos_token_0 = ctx["bos_token"];
  let eos_token_0 = ctx["eos_token"];
  if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
    loop_messages_0 = J.slice(messages_0, 1, undefined, undefined);
    system_message_0 = J.item(J.item(messages_0, 0), "content");
  } else {
    loop_messages_0 = messages_0;
    system_message_0 = false;
  }
  {
    let _items1 = J.iter(loop_messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      let content_2 = content_0;
      if (J.t((!J.eq((J.eq(J.item(message_1, "role"), "user")), (J.eq(J.mod(J.attr(_loop1, "index0"), 2), 0)))))) {
        out.push(J.str(J.call(J.raise_exception, "Conversation roles must alternate user/assistant/user/assistant/...")));
      }
      if (J.t(((v) => J.t(v) ? (!J.eq(system_message_0, false)) : v)((J.eq(J.attr(_loop1, "index0"), 0))))) {
        content_2 = J.add(J.add(J.add("<<SYS>>\n", system_message_0), "\n<</SYS>>\n\n"), J.item(message_1, "content"));
      } else {
        content_2 = J.item(message_1, "content");
      }
      if (J.t((J.eq(J.item(message_1, "role"), "user")))) {
        out.push(J.str(J.add(J.add(J.add(bos_token_0, "[INST] "), J.filter("trim", content_2)), " [/INST]")));
      } else if (J.t((J.eq(J.item(message_1, "role"), "assistant")))) {
        out.push(J.str(J.add(J.add(J.add(" ", J.filter("trim", content_2)), " "), eos_token_0)));
      }
    }
  }
  return out.join("");
}
