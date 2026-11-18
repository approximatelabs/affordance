// Converted from the chat template of LiquidAI/LFM2.5-1.2B-JP by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let keep_past_thinking_0 = ctx["keep_past_thinking"];
  let ns_0 = ctx["ns"];
  let messages_0 = ctx["messages"];
  let bos_token_0 = ctx["bos_token"];
  let tools_0 = ctx["tools"] ?? null;
  let content_0 = ctx["content"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  out.push(J.str(bos_token_0));
  keep_past_thinking_0 = J.filter("default", keep_past_thinking_0, false);
  ns_0 = J.call(J.namespace, J.kw({"system_prompt": ""}));
  if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
    J.setattr(ns_0, "system_prompt", J.item(J.item(messages_0, 0), "content"));
    messages_0 = J.slice(messages_0, 1, undefined, undefined);
  }
  if (J.t(tools_0)) {
    J.setattr(ns_0, "system_prompt", J.add(J.add(J.attr(ns_0, "system_prompt"), (J.t(J.attr(ns_0, "system_prompt")) ? "\n" : "")), "List of tools: ["));
    {
      let _items1 = J.iter(tools_0);
      const _loop1 = J.loop(_items1);
      for (let _i1 = 0; _i1 < _items1.length; _i1++) {
        _loop1.index0 = _i1; const _it1 = _items1[_i1]; let tool_1 = _it1;
        if (J.t(!J.t(J.test("string", tool_1)))) {
          tool_1 = J.filter("tojson", tool_1);
        }
        J.setattr(ns_0, "system_prompt", J.add(J.attr(ns_0, "system_prompt"), tool_1));
        if (J.t(!J.t(J.attr(_loop1, "last")))) {
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
    let _items2 = J.iter(messages_0);
    const _loop2 = J.loop(_items2);
    for (let _i2 = 0; _i2 < _items2.length; _i2++) {
      _loop2.index0 = _i2; const _it2 = _items2[_i2]; let message_2 = _it2;
      if (J.t((J.eq(J.item(message_2, "role"), "assistant")))) {
        J.setattr(ns_0, "last_assistant_index", J.attr(_loop2, "index0"));
      }
    }
  }
  {
    let _items3 = J.iter(messages_0);
    const _loop3 = J.loop(_items3);
    for (let _i3 = 0; _i3 < _items3.length; _i3++) {
      _loop3.index0 = _i3; const _it3 = _items3[_i3]; let message_3 = _it3;
      let content_4 = content_0;
      out.push(J.str(J.add(J.add("<|im_start|>", J.item(message_3, "role")), "\n")));
      content_4 = J.item(message_3, "content");
      if (J.t(!J.t(J.test("string", content_4)))) {
        content_4 = J.filter("tojson", content_4);
      }
      if (J.t(((v) => J.t(v) ? (!J.eq(J.attr(_loop3, "index0"), J.attr(ns_0, "last_assistant_index"))) : v)(((v) => J.t(v) ? !J.t(keep_past_thinking_0) : v)((J.eq(J.item(message_3, "role"), "assistant")))))) {
        if (J.t((J.has("</think>", content_4)))) {
          content_4 = J.filter("trim", J.item(J.call(J.attr(content_4, "split"), "</think>"), J.neg(1)));
        }
      }
      out.push(J.str(J.add(content_4, "<|im_end|>\n")));
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|im_start|>assistant\n"));
  }
  return out.join("");
}
