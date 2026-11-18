// Converted from the chat template of deepseek-ai/DeepSeek-R1-0528-Qwen3-8B by tools/jinja2tsx.py. Regenerate it; do not edit.
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
  ns_0 = J.call(J.namespace, J.kw({"is_first": false, "is_tool": false, "is_output_first": true, "system_prompt": "", "is_first_sp": true, "is_last_user": false}));
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      if (J.t((J.eq(J.item(message_1, "role"), "system")))) {
        if (J.t(J.attr(ns_0, "is_first_sp"))) {
          J.setattr(ns_0, "system_prompt", J.add(J.attr(ns_0, "system_prompt"), J.item(message_1, "content")));
          J.setattr(ns_0, "is_first_sp", false);
        } else {
          J.setattr(ns_0, "system_prompt", J.add(J.add(J.attr(ns_0, "system_prompt"), "\n\n"), J.item(message_1, "content")));
        }
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
      content_3 = J.item(message_2, "content");
      if (J.t((J.eq(J.item(message_2, "role"), "user")))) {
        J.setattr(ns_0, "is_tool", false);
        J.setattr(ns_0, "is_first", false);
        J.setattr(ns_0, "is_last_user", true);
        out.push(J.str(J.add(J.add("<\uff5cUser\uff5c>", content_3), "<\uff5cAssistant\uff5c>")));
      }
      if (J.t((J.eq(J.item(message_2, "role"), "assistant")))) {
        if (J.t((J.has("</think>", content_3)))) {
          content_3 = J.item(J.call(J.attr(content_3, "split"), "</think>"), J.neg(1));
        }
      }
      if (J.t(((v) => J.t(v) ? !J.t((J.item(message_2, "tool_calls") === null)) : v)(((v) => J.t(v) ? (J.item(message_2, "tool_calls") !== undefined) : v)((J.eq(J.item(message_2, "role"), "assistant")))))) {
        J.setattr(ns_0, "is_last_user", false);
        if (J.t(J.attr(ns_0, "is_tool"))) {
          out.push(J.str("<\uff5ctool\u2581outputs\u2581end\uff5c>"));
        }
        J.setattr(ns_0, "is_first", false);
        J.setattr(ns_0, "is_tool", false);
        J.setattr(ns_0, "is_output_first", true);
        {
          let _items3 = J.iter(J.item(message_2, "tool_calls"));
          const _loop3 = J.loop(_items3);
          for (let _i3 = 0; _i3 < _items3.length; _i3++) {
            _loop3.index0 = _i3; const _it3 = _items3[_i3]; let tool_4 = _it3;
            if (J.t(!J.t(J.attr(ns_0, "is_first")))) {
              if (J.t((content_3 === null))) {
                out.push(J.str(J.add(J.add(J.add(J.add(J.add(J.add(J.add(J.add(J.add(J.add("<\uff5ctool\u2581calls\u2581begin\uff5c><\uff5ctool\u2581call\u2581begin\uff5c>", J.item(tool_4, "type")), "<\uff5ctool\u2581sep\uff5c>"), J.item(J.item(tool_4, "function"), "name")), "\n"), "```json"), "\n"), J.item(J.item(tool_4, "function"), "arguments")), "\n"), "```"), "<\uff5ctool\u2581call\u2581end\uff5c>")));
              } else {
                out.push(J.str(J.add(J.add(J.add(J.add(J.add(J.add(J.add(J.add(J.add(J.add(J.add(content_3, "<\uff5ctool\u2581calls\u2581begin\uff5c><\uff5ctool\u2581call\u2581begin\uff5c>"), J.item(tool_4, "type")), "<\uff5ctool\u2581sep\uff5c>"), J.item(J.item(tool_4, "function"), "name")), "\n"), "```json"), "\n"), J.item(J.item(tool_4, "function"), "arguments")), "\n"), "```"), "<\uff5ctool\u2581call\u2581end\uff5c>")));
              }
              J.setattr(ns_0, "is_first", true);
            } else {
              out.push(J.str(J.add(J.add(J.add(J.add(J.add(J.add(J.add(J.add(J.add(J.add(J.add("\n", "<\uff5ctool\u2581call\u2581begin\uff5c>"), J.item(tool_4, "type")), "<\uff5ctool\u2581sep\uff5c>"), J.item(J.item(tool_4, "function"), "name")), "\n"), "```json"), "\n"), J.item(J.item(tool_4, "function"), "arguments")), "\n"), "```"), "<\uff5ctool\u2581call\u2581end\uff5c>")));
            }
          }
        }
        out.push(J.str("<\uff5ctool\u2581calls\u2581end\uff5c><\uff5cend\u2581of\u2581sentence\uff5c>"));
      }
      if (J.t(((v) => J.t(v) ? ((v) => J.t(v) ? v : (J.item(message_2, "tool_calls") === null))(!J.t((J.item(message_2, "tool_calls") !== undefined))) : v)((J.eq(J.item(message_2, "role"), "assistant"))))) {
        J.setattr(ns_0, "is_last_user", false);
        if (J.t(J.attr(ns_0, "is_tool"))) {
          out.push(J.str(J.add(J.add("<\uff5ctool\u2581outputs\u2581end\uff5c>", content_3), "<\uff5cend\u2581of\u2581sentence\uff5c>")));
          J.setattr(ns_0, "is_tool", false);
        } else {
          out.push(J.str(J.add(content_3, "<\uff5cend\u2581of\u2581sentence\uff5c>")));
        }
      }
      if (J.t((J.eq(J.item(message_2, "role"), "tool")))) {
        J.setattr(ns_0, "is_last_user", false);
        J.setattr(ns_0, "is_tool", true);
        if (J.t(J.attr(ns_0, "is_output_first"))) {
          out.push(J.str(J.add(J.add("<\uff5ctool\u2581outputs\u2581begin\uff5c><\uff5ctool\u2581output\u2581begin\uff5c>", content_3), "<\uff5ctool\u2581output\u2581end\uff5c>")));
          J.setattr(ns_0, "is_output_first", false);
        } else {
          out.push(J.str(J.add(J.add("\n<\uff5ctool\u2581output\u2581begin\uff5c>", content_3), "<\uff5ctool\u2581output\u2581end\uff5c>")));
        }
      }
    }
  }
  if (J.t(J.attr(ns_0, "is_tool"))) {
    out.push(J.str("<\uff5ctool\u2581outputs\u2581end\uff5c>"));
  }
  if (J.t(((v) => J.t(v) ? !J.t(J.attr(ns_0, "is_tool")) : v)(((v) => J.t(v) ? !J.t(J.attr(ns_0, "is_last_user")) : v)(add_generation_prompt_0)))) {
    out.push(J.str("<\uff5cAssistant\uff5c>"));
  }
  return out.join("");
}
