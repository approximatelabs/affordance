// Converted from the chat template of google/gemma-3-4b-it by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let first_user_prefix_0 = ctx["first_user_prefix"];
  let loop_messages_0 = ctx["loop_messages"];
  let bos_token_0 = ctx["bos_token"];
  let messages_0 = ctx["messages"];
  let role_0 = ctx["role"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  out.push(J.str(bos_token_0));
  if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
    if (J.t(J.test("string", J.item(J.item(messages_0, 0), "content")))) {
      first_user_prefix_0 = J.add(J.item(J.item(messages_0, 0), "content"), "\n\n");
    } else {
      first_user_prefix_0 = J.add(J.item(J.item(J.item(J.item(messages_0, 0), "content"), 0), "text"), "\n\n");
    }
    loop_messages_0 = J.slice(messages_0, 1, undefined, undefined);
  } else {
    first_user_prefix_0 = "";
    loop_messages_0 = messages_0;
  }
  {
    let _items1 = J.iter(loop_messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      let role_2 = role_0;
      if (J.t((!J.eq((J.eq(J.item(message_1, "role"), "user")), (J.eq(J.mod(J.attr(_loop1, "index0"), 2), 0)))))) {
        out.push(J.str(J.call(J.raise_exception, "Conversation roles must alternate user/assistant/user/assistant/...")));
      }
      if (J.t((J.eq(J.item(message_1, "role"), "assistant")))) {
        role_2 = "model";
      } else {
        role_2 = J.item(message_1, "role");
      }
      out.push(J.str(J.add(J.add(J.add("<start_of_turn>", role_2), "\n"), (J.t(J.attr(_loop1, "first")) ? first_user_prefix_0 : ""))));
      if (J.t(J.test("string", J.item(message_1, "content")))) {
        out.push(J.str(J.filter("trim", J.item(message_1, "content"))));
      } else if (J.t(J.test("iterable", J.item(message_1, "content")))) {
        {
          let _items2 = J.iter(J.item(message_1, "content"));
          const _loop2 = J.loop(_items2);
          for (let _i2 = 0; _i2 < _items2.length; _i2++) {
            _loop2.index0 = _i2; const _it2 = _items2[_i2]; let item_3 = _it2;
            if (J.t((J.eq(J.item(item_3, "type"), "image")))) {
              out.push(J.str("<start_of_image>"));
            } else if (J.t((J.eq(J.item(item_3, "type"), "text")))) {
              out.push(J.str(J.filter("trim", J.item(item_3, "text"))));
            }
          }
        }
      } else {
        out.push(J.str(J.call(J.raise_exception, "Invalid content type")));
      }
      out.push(J.str("<end_of_turn>\n"));
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<start_of_turn>model\n"));
  }
  return out.join("");
}
