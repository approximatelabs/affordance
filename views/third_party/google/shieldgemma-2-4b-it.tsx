// Converted from the chat template of google/shieldgemma-2-4b-it by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let bos_token_0 = ctx["bos_token"];
  let messages_0 = ctx["messages"];
  out.push(J.str(bos_token_0));
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      if (J.t(J.test("iterable", J.item(message_1, "content")))) {
        {
          let _items2 = J.iter(J.item(message_1, "content"));
          const _loop2 = J.loop(_items2);
          for (let _i2 = 0; _i2 < _items2.length; _i2++) {
            _loop2.index0 = _i2; const _it2 = _items2[_i2]; let item_2 = _it2;
            out.push(J.str("<start_of_turn>user\n"));
            if (J.t((J.eq(J.item(item_2, "type"), "image")))) {
              out.push(J.str("<start_of_image>"));
            } else if (J.t((J.eq(J.item(item_2, "type"), "text")))) {
              out.push(J.str("You are a policy expert trying to help determine whether a generated or real image is in"), "\n                ", J.str(" violation of the safety policy below. Your decision must very clearly follow the specific"), "\n                ", J.str(" language used in the safety policy and should not include any interpretation outside of the"), "\n                ", J.str(" safety policy.\n\n"), "\n                ", J.str("<BEGIN_SAFETY_POLICY>\n"), "\n                ", J.str(J.add(J.item(item_2, "text"), "\n")), "\n                ", J.str("</END_SAFETY_POLICY>\n\n"), "\n                ", J.str("Does the image violate the above policy? Your Answer must start with 'Yes' or 'No'."), "\n                ", J.str("<end_of_turn>\n"));
            }
          }
        }
        out.push(J.str("<start_of_turn>model\n"));
      } else {
        out.push(J.str(J.call(J.raise_exception, "Conversation messages must contain iterable content containing images and policy definitions in text.")));
      }
    }
  }
  return out.join("");
}
