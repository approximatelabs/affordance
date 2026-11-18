// Converted from the chat template of nvidia/Llama3-ChatQA-1.5-8B by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let loop_messages_0 = ctx["loop_messages"];
  let bos_token_0 = ctx["bos_token"];
  let messages_0 = ctx["messages"];
  out.push(J.str(bos_token_0));
  if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
    loop_messages_0 = J.slice(messages_0, 1, undefined, undefined);
  } else {
    loop_messages_0 = messages_0;
  }
  out.push("System: This is a chat between a user and an artificial intelligence assistant. The assistant gives helpful, detailed, and polite answers to the user's questions based on the context. The assistant should also indicate when the answer cannot be found in the context.\n\n");
  {
    let _items1 = J.iter(loop_messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      if (J.t((J.eq(J.item(message_1, "role"), "user")))) {
        out.push("User: ", J.str(J.add(J.call(J.attr(J.item(message_1, "content"), "strip")), "\n\n")));
      } else {
        out.push("Assistant: ", J.str(J.add(J.call(J.attr(J.item(message_1, "content"), "strip")), "\n\n")));
      }
      if (J.t(((v) => J.t(v) ? (J.eq(J.item(message_1, "role"), "user")) : v)(J.attr(_loop1, "last")))) {
        out.push("Assistant:");
      }
    }
  }
  return out.join("");
}
