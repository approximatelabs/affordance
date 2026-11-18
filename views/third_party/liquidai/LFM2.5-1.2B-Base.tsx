// Converted from the chat template of LiquidAI/LFM2.5-1.2B-Base by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let system_prompt_0 = ctx["system_prompt"];
  let ns_0 = ctx["ns"];
  let messages_0 = ctx["messages"];
  let bos_token_0 = ctx["bos_token"];
  let tools_0 = ctx["tools"] ?? null;
  let content_0 = ctx["content"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  out.push(J.str(bos_token_0));
  system_prompt_0 = "";
  ns_0 = J.call(J.namespace, J.kw({"system_prompt": ""}));
  if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
    J.setattr(ns_0, "system_prompt", J.item(J.item(messages_0, 0), "content"));
    messages_0 = J.slice(messages_0, 1, undefined, undefined);
  }
  if (J.t(tools_0)) {
    J.setattr(ns_0, "system_prompt", J.add(J.add(J.attr(ns_0, "system_prompt"), (J.t(J.attr(ns_0, "system_prompt")) ? "\n" : "")), "List of tools: <|tool_list_start|>["));
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
    J.setattr(ns_0, "system_prompt", J.add(J.attr(ns_0, "system_prompt"), "]<|tool_list_end|>"));
  }
  if (J.t(J.attr(ns_0, "system_prompt"))) {
    out.push(J.str(J.add(J.add("<|im_start|>system\n", J.attr(ns_0, "system_prompt")), "<|im_end|>\n")));
  }
  {
    let _items2 = J.iter(messages_0);
    const _loop2 = J.loop(_items2);
    for (let _i2 = 0; _i2 < _items2.length; _i2++) {
      _loop2.index0 = _i2; const _it2 = _items2[_i2]; let message_2 = _it2;
      let content_3 = content_0;
      out.push(J.str(J.add(J.add("<|im_start|>", J.item(message_2, "role")), "\n")));
      content_3 = J.item(message_2, "content");
      if (J.t(!J.t(J.test("string", content_3)))) {
        content_3 = J.filter("tojson", content_3);
      }
      if (J.t((J.eq(J.item(message_2, "role"), "tool")))) {
        content_3 = J.add(J.add("<|tool_response_start|>", content_3), "<|tool_response_end|>");
      }
      out.push(J.str(J.add(content_3, "<|im_end|>\n")));
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|im_start|>assistant\n"));
  }
  return out.join("");
}
