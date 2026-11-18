// Converted from the chat template of MiniMaxAI/MiniMax-VL-01 by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let messages_0 = ctx["messages"];
  let tools_0 = ctx["tools"] ?? null;
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      if (J.t((J.eq(J.item(message_1, "role"), "system")))) {
        out.push(J.str("<beginning_of_sentence>system ai_setting=assistant\n"));
        {
          let _items2 = J.iter(J.item(message_1, "content"));
          const _loop2 = J.loop(_items2);
          for (let _i2 = 0; _i2 < _items2.length; _i2++) {
            _loop2.index0 = _i2; const _it2 = _items2[_i2]; let item_2 = _it2;
            if (J.t((J.eq(J.attr(item_2, "type"), "image")))) {
              out.push("<image>");
            } else if (J.t((J.eq(J.attr(item_2, "type"), "text")))) {
              out.push(J.str(J.attr(item_2, "text")));
            }
          }
        }
        out.push(J.str("<end_of_sentence>\n"));
      }
      if (J.t((J.eq(J.item(message_1, "role"), "assistant")))) {
        out.push(J.str("<beginning_of_sentence>ai name=assistant\n"));
        {
          let _items3 = J.iter(J.item(message_1, "content"));
          const _loop3 = J.loop(_items3);
          for (let _i3 = 0; _i3 < _items3.length; _i3++) {
            _loop3.index0 = _i3; const _it3 = _items3[_i3]; let item_3 = _it3;
            if (J.t((J.eq(J.attr(item_3, "type"), "image")))) {
              out.push("<image>");
            } else if (J.t((J.eq(J.attr(item_3, "type"), "text")))) {
              out.push(J.str(J.attr(item_3, "text")));
            }
          }
        }
        out.push(J.str("<end_of_sentence>\n"));
      }
      if (J.t((J.eq(J.item(message_1, "role"), "user")))) {
        out.push(J.str("<beginning_of_sentence>user name=user\n"));
        {
          let _items4 = J.iter(J.item(message_1, "content"));
          const _loop4 = J.loop(_items4);
          for (let _i4 = 0; _i4 < _items4.length; _i4++) {
            _loop4.index0 = _i4; const _it4 = _items4[_i4]; let item_4 = _it4;
            if (J.t((J.eq(J.attr(item_4, "type"), "image")))) {
              out.push("<image>");
            } else if (J.t((J.eq(J.attr(item_4, "type"), "text")))) {
              out.push(J.str(J.attr(item_4, "text")));
            }
          }
        }
        out.push(J.str("<end_of_sentence>\n"));
      }
      if (J.t((J.eq(J.item(message_1, "role"), "function")))) {
        out.push(J.str(J.add(J.add(J.add(J.add(J.add(J.add("<beginning_of_sentence>system function_response=functions\n", "{\"name\": \""), J.item(message_1, "name")), "\", \"response\": "), J.item(J.item(J.item(message_1, "content"), 0), "text")), "}"), "<end_of_sentence>\n")));
      }
    }
  }
  if (J.t(tools_0)) {
    {
      let _items5 = J.iter(tools_0);
      const _loop5 = J.loop(_items5);
      for (let _i5 = 0; _i5 < _items5.length; _i5++) {
        _loop5.index0 = _i5; const _it5 = _items5[_i5]; let function_5 = _it5;
        out.push(J.str(J.add(J.add("<beginning_of_sentence>system function_setting=functions\n", J.filter("tojson", function_5)), "<end_of_sentence>\n")));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<beginning_of_sentence>ai name=assistant\n"));
    {
    }
  }
  return out.join("");
}
