// Converted from the chat template of meta-llama/Llama-Guard-4-12B by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let role_0 = ctx["role"];
  let ns_0 = ctx["ns"];
  let categories_0 = ctx["categories"];
  let messages_0 = ctx["messages"];
  let excluded_category_keys_0 = ctx["excluded_category_keys"];
  let content_0 = ctx["content"];
  if (J.t((J.eq(J.mod(J.filter("length", messages_0), 2), 0)))) {
    role_0 = "Agent";
  } else {
    role_0 = "User";
  }
  if (J.t(!J.t((categories_0 !== undefined)))) {
    ns_0 = J.call(J.namespace, J.kw({"contains_image": false}));
    {
      let _items1 = J.iter(messages_0);
      const _loop1 = J.loop(_items1);
      for (let _i1 = 0; _i1 < _items1.length; _i1++) {
        _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
        {
          let _items2 = J.iter(J.filter("selectattr", J.item(message_1, "content"), "type", "equalto", "image"));
          const _loop2 = J.loop(_items2);
          for (let _i2 = 0; _i2 < _items2.length; _i2++) {
            _loop2.index0 = _i2; const _it2 = _items2[_i2]; let content_2 = _it2;
            J.setattr(ns_0, "contains_image", true);
          }
        }
      }
    }
    if (J.t(J.attr(ns_0, "contains_image"))) {
      categories_0 = ({"S1": "Violent Crimes.", "S2": "Non-Violent Crimes.", "S3": "Sex Crimes.", "S4": "Child Exploitation.", "S5": "Defamation.", "S6": "Specialized Advice.", "S7": "Privacy.", "S8": "Intellectual Property.", "S9": "Indiscriminate Weapons.", "S10": "Hate.", "S11": "Self-Harm.", "S12": "Sexual Content.", "S13": "Elections."});
    } else {
      categories_0 = ({"S1": "Violent Crimes.", "S2": "Non-Violent Crimes.", "S3": "Sex Crimes.", "S4": "Child Exploitation.", "S5": "Defamation.", "S6": "Specialized Advice.", "S7": "Privacy.", "S8": "Intellectual Property.", "S9": "Indiscriminate Weapons.", "S10": "Hate.", "S11": "Self-Harm.", "S12": "Sexual Content.", "S13": "Elections.", "S14": "Code Interpreter Abuse."});
    }
  }
  out.push(J.str("<|begin_of_text|><|header_start|>user<|header_end|>\n"), "\n", J.str(J.add(J.add("Task: Check if there is unsafe content in '", role_0), "' messages in conversations according to our safety policy with the below categories.\n")), "\n", J.str("<BEGIN UNSAFE CONTENT CATEGORIES>\n"));
  {
    let _items3 = J.iter(categories_0);
    const _loop3 = J.loop(_items3);
    for (let _i3 = 0; _i3 < _items3.length; _i3++) {
      _loop3.index0 = _i3; const _it3 = _items3[_i3]; let key_3 = _it3;
      if (J.t((!J.has(key_3, excluded_category_keys_0)))) {
        out.push(J.str(J.add(J.add(J.add(key_3, ": "), J.item(categories_0, key_3)), "\n")));
      }
    }
  }
  out.push(J.str("<END UNSAFE CONTENT CATEGORIES>\n"), "\n", J.str("<BEGIN CONVERSATION>\n"));
  {
    let _items4 = J.iter(messages_0);
    const _loop4 = J.loop(_items4);
    for (let _i4 = 0; _i4 < _items4.length; _i4++) {
      _loop4.index0 = _i4; const _it4 = _items4[_i4]; let message_4 = _it4;
      let content_5 = content_0;
      let role_6 = role_0;
      let ns_7 = ns_0;
      if (J.t((!J.eq((J.eq(J.item(message_4, "role"), "user")), (J.eq(J.mod(J.attr(_loop4, "index0"), 2), 0)))))) {
        out.push(J.str(J.call(J.raise_exception, "Conversation roles must alternate user/assistant/user/assistant/...")));
      }
      content_5 = J.item(message_4, "content");
      if (J.t((J.eq(J.item(message_4, "role"), "user")))) {
        role_6 = "User";
      } else if (J.t((J.eq(J.item(message_4, "role"), "assistant")))) {
        role_6 = "Agent";
      }
      ns_7 = J.call(J.namespace, J.kw({"img_tokens": ""}));
      {
        let _items5 = J.iter(J.filter("selectattr", J.attr(message_4, "content"), "type", "equalto", "image"));
        const _loop5 = J.loop(_items5);
        for (let _i5 = 0; _i5 < _items5.length; _i5++) {
          _loop5.index0 = _i5; const _it5 = _items5[_i5]; let img_8 = _it5;
          J.setattr(ns_7, "img_tokens", J.add(J.attr(ns_7, "img_tokens"), "<|image|>"));
        }
      }
      {
        let _items6 = J.iter(J.filter("selectattr", J.attr(message_4, "content"), "type", "equalto", "text"));
        const _loop6 = J.loop(_items6);
        for (let _i6 = 0; _i6 < _items6.length; _i6++) {
          _loop6.index0 = _i6; const _it6 = _items6[_i6]; let txt_9 = _it6;
          out.push(J.str(J.add(role_6, ": ")), J.str(J.attr(ns_7, "img_tokens")), J.str(J.filter("trim", J.attr(txt_9, "text"))), "\\n\\n\n\n");
          if (J.t(J.attr(_loop6, "first"))) {
            J.setattr(ns_7, "img_tokens", "");
          }
        }
      }
    }
  }
  out.push(J.str("<END CONVERSATION>\n"), "\n", J.str(J.add(J.add("Provide your safety assessment for ONLY THE LAST ", role_0), " message in the above conversation:")), "\n", J.str(" - First line must read 'safe' or 'unsafe'."), "\n", J.str(" - If unsafe, a second line must include a comma-separated list of violated categories. <|eot|><|header_start|>assistant<|header_end|>"));
  return out.join("");
}
