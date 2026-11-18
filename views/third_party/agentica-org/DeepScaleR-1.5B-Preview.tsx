// Converted from the chat template of agentica-org/DeepScaleR-1.5B-Preview by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let ns_0 = ctx["ns"];
  let messages_0 = ctx["messages"];
  let bos_token_0 = ctx["bos_token"];
  let content_0 = ctx["content"];
  if (J.t(!J.t((add_generation_prompt_0 !== undefined)))) {
    add_generation_prompt_0 = false;
  }
  ns_0 = J.call(J.namespace, J.kw({"is_first": false, "is_tool": false, "is_output_first": true, "system_prompt": ""}));
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      if (J.t((J.eq(J.item(message_1, "role"), "system")))) {
        J.setattr(ns_0, "system_prompt", J.item(message_1, "content"));
      }
    }
  }
  out.push(J.str(bos_token_0), J.str(J.attr(ns_0, "system_prompt")));
  {
    let _items2 = J.iter(messages_0);
    const _loop2 = J.loop(_items2);
    for (let _i2 = 0; _i2 < _items2.length; _i2++) {
      _loop2.index0 = _i2; const _it2 = _items2[_i2]; let message_2 = _it2;
      let content_3 = content_0;
      if (J.t((J.eq(J.item(message_2, "role"), "user")))) {
        J.setattr(ns_0, "is_tool", false);
        out.push(J.str(J.add("<\uff5cUser\uff5c>", J.item(message_2, "content"))));
      }
      if (J.t(((v) => J.t(v) ? (J.item(message_2, "content") === null) : v)((J.eq(J.item(message_2, "role"), "assistant"))))) {
        J.setattr(ns_0, "is_tool", false);
        {
          let _items3 = J.iter(J.item(message_2, "tool_calls"));
          const _loop3 = J.loop(_items3);
          for (let _i3 = 0; _i3 < _items3.length; _i3++) {
            _loop3.index0 = _i3; const _it3 = _items3[_i3]; let tool_4 = _it3;
            if (J.t(!J.t(J.attr(ns_0, "is_first")))) {
              out.push(J.str(J.add(J.add(J.add(J.add(J.add(J.add(J.add(J.add(J.add(J.add("<\uff5cAssistant\uff5c><\uff5ctool\u2581calls\u2581begin\uff5c><\uff5ctool\u2581call\u2581begin\uff5c>", J.item(tool_4, "type")), "<\uff5ctool\u2581sep\uff5c>"), J.item(J.item(tool_4, "function"), "name")), "\n"), "```json"), "\n"), J.item(J.item(tool_4, "function"), "arguments")), "\n"), "```"), "<\uff5ctool\u2581call\u2581end\uff5c>")));
              J.setattr(ns_0, "is_first", true);
            } else {
              out.push(J.str(J.add(J.add(J.add(J.add(J.add(J.add(J.add(J.add(J.add(J.add(J.add("\n", "<\uff5ctool\u2581call\u2581begin\uff5c>"), J.item(tool_4, "type")), "<\uff5ctool\u2581sep\uff5c>"), J.item(J.item(tool_4, "function"), "name")), "\n"), "```json"), "\n"), J.item(J.item(tool_4, "function"), "arguments")), "\n"), "```"), "<\uff5ctool\u2581call\u2581end\uff5c>")), J.str("<\uff5ctool\u2581calls\u2581end\uff5c><\uff5cend\u2581of\u2581sentence\uff5c>"));
            }
          }
        }
      }
      if (J.t(((v) => J.t(v) ? !J.t((J.item(message_2, "content") === null)) : v)((J.eq(J.item(message_2, "role"), "assistant"))))) {
        if (J.t(J.attr(ns_0, "is_tool"))) {
          out.push(J.str(J.add(J.add("<\uff5ctool\u2581outputs\u2581end\uff5c>", J.item(message_2, "content")), "<\uff5cend\u2581of\u2581sentence\uff5c>")));
          J.setattr(ns_0, "is_tool", false);
        } else {
          content_3 = J.item(message_2, "content");
          if (J.t((J.has("</think>", content_3)))) {
            content_3 = J.item(J.call(J.attr(content_3, "split"), "</think>"), J.neg(1));
          }
          out.push(J.str(J.add(J.add("<\uff5cAssistant\uff5c>", content_3), "<\uff5cend\u2581of\u2581sentence\uff5c>")));
        }
      }
      if (J.t((J.eq(J.item(message_2, "role"), "tool")))) {
        J.setattr(ns_0, "is_tool", true);
        if (J.t(J.attr(ns_0, "is_output_first"))) {
          out.push(J.str(J.add(J.add("<\uff5ctool\u2581outputs\u2581begin\uff5c><\uff5ctool\u2581output\u2581begin\uff5c>", J.item(message_2, "content")), "<\uff5ctool\u2581output\u2581end\uff5c>")));
          J.setattr(ns_0, "is_output_first", false);
        } else {
          out.push(J.str(J.add(J.add("\n<\uff5ctool\u2581output\u2581begin\uff5c>", J.item(message_2, "content")), "<\uff5ctool\u2581output\u2581end\uff5c>")));
        }
      }
    }
  }
  if (J.t(J.attr(ns_0, "is_tool"))) {
    out.push(J.str("<\uff5ctool\u2581outputs\u2581end\uff5c>"));
  }
  if (J.t(((v) => J.t(v) ? !J.t(J.attr(ns_0, "is_tool")) : v)(add_generation_prompt_0))) {
    out.push(J.str("<\uff5cAssistant\uff5c>"));
  }
  return out.join("");
}
