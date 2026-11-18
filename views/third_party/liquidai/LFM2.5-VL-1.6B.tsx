// Converted from the chat template of LiquidAI/LFM2.5-VL-1.6B by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let keep_past_thinking_0 = ctx["keep_past_thinking"];
  let ns_0 = ctx["ns"];
  let sys_content_0 = ctx["sys_content"];
  let messages_0 = ctx["messages"];
  let bos_token_0 = ctx["bos_token"];
  let tools_0 = ctx["tools"] ?? null;
  let content_0 = ctx["content"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  out.push(J.str(bos_token_0));
  keep_past_thinking_0 = J.filter("default", keep_past_thinking_0, false);
  ns_0 = J.call(J.namespace, J.kw({"system_prompt": ""}));
  if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
    sys_content_0 = J.item(J.item(messages_0, 0), "content");
    if (J.t(!J.t(J.test("string", sys_content_0)))) {
      {
        let _items1 = J.iter(sys_content_0);
        const _loop1 = J.loop(_items1);
        for (let _i1 = 0; _i1 < _items1.length; _i1++) {
          _loop1.index0 = _i1; const _it1 = _items1[_i1]; let item_1 = _it1;
          if (J.t((J.eq(J.item(item_1, "type"), "text")))) {
            J.setattr(ns_0, "system_prompt", J.add(J.attr(ns_0, "system_prompt"), J.item(item_1, "text")));
          }
        }
      }
    } else {
      J.setattr(ns_0, "system_prompt", sys_content_0);
    }
    messages_0 = J.slice(messages_0, 1, undefined, undefined);
  }
  if (J.t(tools_0)) {
    J.setattr(ns_0, "system_prompt", J.add(J.add(J.attr(ns_0, "system_prompt"), (J.t(J.attr(ns_0, "system_prompt")) ? "\n" : "")), "List of tools: ["));
    {
      let _items2 = J.iter(tools_0);
      const _loop2 = J.loop(_items2);
      for (let _i2 = 0; _i2 < _items2.length; _i2++) {
        _loop2.index0 = _i2; const _it2 = _items2[_i2]; let tool_2 = _it2;
        if (J.t(!J.t(J.test("string", tool_2)))) {
          tool_2 = J.filter("tojson", tool_2);
        }
        J.setattr(ns_0, "system_prompt", J.add(J.attr(ns_0, "system_prompt"), tool_2));
        if (J.t(!J.t(J.attr(_loop2, "last")))) {
          J.setattr(ns_0, "system_prompt", J.add(J.attr(ns_0, "system_prompt"), ", "));
        }
      }
    }
    J.setattr(ns_0, "system_prompt", J.add(J.attr(ns_0, "system_prompt"), "]"));
  }
  if (J.t(J.attr(ns_0, "system_prompt"))) {
    out.push(J.str(J.add(J.add("<|im_start|>system\n", J.attr(ns_0, "system_prompt")), "<|im_end|>\n")));
  }
  J.setattr(ns_0, "last_assistant_index", J.neg(1));
  {
    let _items3 = J.iter(messages_0);
    const _loop3 = J.loop(_items3);
    for (let _i3 = 0; _i3 < _items3.length; _i3++) {
      _loop3.index0 = _i3; const _it3 = _items3[_i3]; let message_3 = _it3;
      if (J.t((J.eq(J.item(message_3, "role"), "assistant")))) {
        J.setattr(ns_0, "last_assistant_index", J.attr(_loop3, "index0"));
      }
    }
  }
  {
    let _items4 = J.iter(messages_0);
    const _loop4 = J.loop(_items4);
    for (let _i4 = 0; _i4 < _items4.length; _i4++) {
      _loop4.index0 = _i4; const _it4 = _items4[_i4]; let message_4 = _it4;
      let content_5 = content_0;
      out.push(J.str(J.add(J.add("<|im_start|>", J.item(message_4, "role")), "\n")));
      if (J.t(!J.t(J.test("string", J.item(message_4, "content"))))) {
        J.setattr(ns_0, "content", "");
        {
          let _items5 = J.iter(J.item(message_4, "content"));
          const _loop5 = J.loop(_items5);
          for (let _i5 = 0; _i5 < _items5.length; _i5++) {
            _loop5.index0 = _i5; const _it5 = _items5[_i5]; let item_6 = _it5;
            if (J.t((J.eq(J.item(item_6, "type"), "image")))) {
              J.setattr(ns_0, "content", J.add(J.attr(ns_0, "content"), "<image>"));
            } else if (J.t((J.eq(J.item(item_6, "type"), "text")))) {
              J.setattr(ns_0, "content", J.add(J.attr(ns_0, "content"), J.item(item_6, "text")));
            } else {
              J.setattr(ns_0, "content", J.add(J.attr(ns_0, "content"), J.filter("tojson", item_6)));
            }
          }
        }
        content_5 = J.attr(ns_0, "content");
      } else {
        content_5 = J.item(message_4, "content");
      }
      if (J.t(((v) => J.t(v) ? (!J.eq(J.attr(_loop4, "index0"), J.attr(ns_0, "last_assistant_index"))) : v)(((v) => J.t(v) ? !J.t(keep_past_thinking_0) : v)((J.eq(J.item(message_4, "role"), "assistant")))))) {
        if (J.t((J.has("</think>", content_5)))) {
          content_5 = J.filter("trim", J.item(J.call(J.attr(content_5, "split"), "</think>"), J.neg(1)));
        }
      }
      out.push(J.str(J.add(content_5, "<|im_end|>\n")));
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|im_start|>assistant\n"));
  }
  return out.join("");
}
