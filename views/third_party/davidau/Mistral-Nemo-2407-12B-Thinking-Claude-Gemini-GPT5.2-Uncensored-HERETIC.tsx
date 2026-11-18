// Converted from the chat template of DavidAU/Mistral-Nemo-2407-12B-Thinking-Claude-Gemini-GPT5.2-Uncensored-HERETIC by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let system_message_0 = ctx["system_message"];
  let loop_messages_0 = ctx["loop_messages"];
  let messages_0 = ctx["messages"];
  let bos_token_0 = ctx["bos_token"];
  let eos_token_0 = ctx["eos_token"];
  if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
    system_message_0 = J.item(J.item(messages_0, 0), "content");
    loop_messages_0 = J.slice(messages_0, 1, undefined, undefined);
  } else {
    loop_messages_0 = messages_0;
  }
  out.push(J.str(bos_token_0));
  {
    let _items1 = J.iter(loop_messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      if (J.t((!J.eq((J.eq(J.item(message_1, "role"), "user")), (J.eq(J.mod(J.attr(_loop1, "index0"), 2), 0)))))) {
        out.push(J.str(J.call(J.raise_exception, "After the optional system message, conversation roles must alternate user/assistant/user/assistant/...")));
      }
      if (J.t((J.eq(J.item(message_1, "role"), "user")))) {
        if (J.t(((v) => J.t(v) ? (system_message_0 !== undefined) : v)(J.attr(_loop1, "last")))) {
          out.push(J.str(J.add(J.add(J.add(J.add("[INST] ", system_message_0), "\n\n"), J.item(message_1, "content")), "[/INST]")));
        } else {
          out.push(J.str(J.add(J.add("[INST] ", J.item(message_1, "content")), "[/INST]")));
        }
      } else if (J.t((J.eq(J.item(message_1, "role"), "assistant")))) {
        out.push(J.str(J.add(J.add(" ", J.item(message_1, "content")), eos_token_0)));
      } else {
        out.push(J.str(J.call(J.raise_exception, "Only user and assistant roles are supported, with the exception of an initial optional system message!")));
      }
    }
  }
  return out.join("");
}
