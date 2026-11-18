// Converted from the chat template of tencent/Hunyuan-1.8B-Instruct by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let ns_0 = ctx["ns"];
  let messages_0 = ctx["messages"];
  let bos_token_0 = ctx["bos_token"];
  let tools_0 = ctx["tools"] ?? null;
  let content_0 = ctx["content"];
  let arguments_0 = ctx["arguments"];
  let eos_token_0 = ctx["eos_token"];
  let enable_thinking_0 = ctx["enable_thinking"];
  if (J.t(!J.t((add_generation_prompt_0 !== undefined)))) {
    add_generation_prompt_0 = false;
  }
  ns_0 = J.call(J.namespace, J.kw({"is_first": false, "is_tool": false, "is_output_first": true, "system_prompt": "", "is_first_sp": true, "is_first_user": true, "is_last_user": false}));
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
  if (J.t(tools_0)) {
    if (J.t((!J.eq(J.attr(ns_0, "system_prompt"), "")))) {
      out.push(J.str("\n\n# Tools\n\nYou may call one or more functions to assist with the user query."));
    } else {
      out.push(J.str("# Tools\n\nYou may call one or more functions to assist with the user query."));
    }
    out.push(J.str("\n\nYou are provided with function signatures within <tools></tools> XML tags:"), J.str("\n<tools>\n"));
    {
      let _items2 = J.iter(tools_0);
      const _loop2 = J.loop(_items2);
      for (let _i2 = 0; _i2 < _items2.length; _i2++) {
        _loop2.index0 = _i2; const _it2 = _items2[_i2]; let tool_2 = _it2;
        if (J.t((J.gt(J.attr(_loop2, "index0"), 1)))) {
          out.push(J.str("\n"));
        }
        out.push(J.str(J.filter("tojson", tool_2)));
      }
    }
    out.push(J.str("\n</tools>\n\n"), J.str("For function call returns, you should first print <tool_calls>"), J.str("For each function call, you should return object like:\n"), J.str("<tool_call>function_name\n```json\nfunction_arguments_in_json_format\n```</tool_call>"), J.str("At the end of function call returns, you should print </tool_calls>"));
  }
  if (J.t(((v) => J.t(v) ? v : tools_0)((!J.eq(J.attr(ns_0, "system_prompt"), ""))))) {
    out.push(J.str("<\uff5chy_place\u2581holder\u2581no\u25813\uff5c>"));
  }
  {
    let _items3 = J.iter(messages_0);
    const _loop3 = J.loop(_items3);
    for (let _i3 = 0; _i3 < _items3.length; _i3++) {
      _loop3.index0 = _i3; const _it3 = _items3[_i3]; let message_3 = _it3;
      let content_4 = content_0;
      if (J.t((J.eq(J.item(message_3, "role"), "user")))) {
        J.setattr(ns_0, "is_tool", false);
        J.setattr(ns_0, "is_first", false);
        J.setattr(ns_0, "is_last_user", true);
        out.push(J.str(J.add(J.add("<\uff5chy_User\uff5c>", J.item(message_3, "content")), "<\uff5chy_Assistant\uff5c>")));
      }
      if (J.t(((v) => J.t(v) ? !J.t((J.item(message_3, "tool_calls") === null)) : v)(((v) => J.t(v) ? (J.item(message_3, "tool_calls") !== undefined) : v)((J.eq(J.item(message_3, "role"), "assistant")))))) {
        J.setattr(ns_0, "is_last_user", false);
        if (J.t(J.attr(ns_0, "is_tool"))) {
          out.push(J.str(J.add("</tool_responses>", "<\uff5chy_Assistant\uff5c>")));
        }
        J.setattr(ns_0, "is_first", false);
        J.setattr(ns_0, "is_tool", false);
        J.setattr(ns_0, "is_output_first", true);
        {
          let _items4 = J.iter(J.item(message_3, "tool_calls"));
          const _loop4 = J.loop(_items4);
          for (let _i4 = 0; _i4 < _items4.length; _i4++) {
            _loop4.index0 = _i4; const _it4 = _items4[_i4]; let tool_5 = _it4;
            let arguments_6 = arguments_0;
            arguments_6 = J.item(J.item(tool_5, "function"), "arguments");
            if (J.t(!J.t(J.test("string", arguments_6)))) {
              arguments_6 = J.filter("tojson", arguments_6);
            }
            if (J.t(!J.t(J.attr(ns_0, "is_first")))) {
              if (J.t((J.item(message_3, "content") === null))) {
                out.push(J.str(J.add(J.add(J.add(J.add(J.add(J.add(J.add(J.add("<tool_calls><tool_call>", J.item(J.item(tool_5, "function"), "name")), "\n"), "```json"), "\n"), arguments_6), "\n"), "```"), "</tool_call>")));
              } else {
                out.push(J.str(J.add(J.add(J.add(J.add(J.add(J.add(J.add(J.add(J.add(J.item(message_3, "content"), "<tool_calls><tool_call>"), J.item(J.item(tool_5, "function"), "name")), "\n"), "```json"), "\n"), arguments_6), "\n"), "```"), "</tool_call>")));
              }
              J.setattr(ns_0, "is_first", true);
            } else {
              out.push(J.str(J.add(J.add(J.add(J.add(J.add(J.add(J.add(J.add(J.add("\n", "<tool_call>"), J.item(J.item(tool_5, "function"), "name")), "\n"), "```json"), "\n"), arguments_6), "\n"), "```"), "</tool_call>")));
            }
          }
        }
        out.push(J.str(J.add("</tool_calls>", eos_token_0)));
      }
      if (J.t(((v) => J.t(v) ? ((v) => J.t(v) ? v : (J.item(message_3, "tool_calls") === null))(!J.t((J.item(message_3, "tool_calls") !== undefined))) : v)((J.eq(J.item(message_3, "role"), "assistant"))))) {
        content_4 = J.item(message_3, "content");
        if (J.t(((v) => J.t(v) ? !J.t(J.attr(_loop3, "last")) : v)((J.has("<answer>", content_4))))) {
          content_4 = J.call(J.attr(J.call(J.attr(J.item(J.call(J.attr(content_4, "split"), "<answer>"), J.neg(1)), "strip"), "</answer>"), "strip"));
        }
        J.setattr(ns_0, "is_last_user", false);
        if (J.t(J.attr(ns_0, "is_tool"))) {
          out.push(J.str(J.add(J.add(J.add("</tool_responses>", "<\uff5chy_Assistant\uff5c>"), content_4), eos_token_0)));
          J.setattr(ns_0, "is_tool", false);
        } else {
          out.push(J.str(J.add(content_4, eos_token_0)));
        }
      }
      if (J.t((J.eq(J.item(message_3, "role"), "tool")))) {
        J.setattr(ns_0, "is_last_user", false);
        J.setattr(ns_0, "is_tool", true);
        if (J.t(J.attr(ns_0, "is_output_first"))) {
          out.push(J.str(J.add(J.add(J.add("<\uff5chy_User\uff5c>", "<tool_responses><tool_response>"), J.item(message_3, "content")), "</tool_response>")));
          J.setattr(ns_0, "is_output_first", false);
        } else {
          out.push(J.str(J.add(J.add("\n<tool_response>", J.item(message_3, "content")), "</tool_response>")));
        }
      }
    }
  }
  if (J.t(J.attr(ns_0, "is_tool"))) {
    out.push(J.str(J.add("</tool_responses>", "<\uff5chy_Assistant\uff5c>")));
  }
  if (J.t(((v) => J.t(v) ? !J.t(J.attr(ns_0, "is_tool")) : v)(((v) => J.t(v) ? !J.t(J.attr(ns_0, "is_last_user")) : v)(add_generation_prompt_0)))) {
    out.push(J.str("<\uff5chy_Assistant\uff5c>"));
  }
  if (J.t(((v) => J.t(v) ? !J.t(enable_thinking_0) : v)((enable_thinking_0 !== undefined)))) {
    out.push(J.str("<think>\n\n</think>\n"));
  }
  return out.join("");
}
