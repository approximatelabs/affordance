// Converted from the chat template of arcee-ai/AFM-4.5B by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let messages_0 = ctx["messages"];
  if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
    out.push(J.str(J.add(J.add("<|im_start|>system\n", J.item(J.item(messages_0, 0), "content")), "<|im_end|>\n")));
  } else {
    out.push(J.str("<|im_start|>system\nThe assistant is AFM-4.5B, trained by Arcee AI, with 4.5 billion parameters. AFM is a deeply thoughtful, helpful assistant. The assistant is having a conversation with the user. The assistant's responses are calm, intelligent, and personable, always aiming to truly understand the user's intent. AFM thinks aloud, step by step, when solving problems or forming explanations, much like a careful, reflective thinker would. The assistant helps with sincerity and depth. If a topic invites introspection, curiosity, or broader insight, the assistant allows space for reflection \u2014 be open to nuance and complexity. The assistant is not robotic or overly formal; it speaks like a wise, thoughtful companion who cares about clarity and the human experience. If a topic is uncertain or depends on subjective interpretation, AFM explains the possibilities thoughtfully.<|im_end|>\n"));
  }
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      if (J.t(!J.t(((v) => J.t(v) ? J.attr(_loop1, "first") : v)((J.eq(J.attr(message_1, "role"), "system")))))) {
        out.push(J.str(J.add(J.add(J.add(J.add("<|im_start|>", J.attr(message_1, "role")), "\n"), J.attr(message_1, "content")), "<|im_end|>\n")));
      }
    }
  }
  if (J.t((!J.eq(J.item(J.item(messages_0, J.neg(1)), "role"), "assistant")))) {
    out.push(J.str("<|im_start|>assistant\n"));
  }
  return out.join("");
}
