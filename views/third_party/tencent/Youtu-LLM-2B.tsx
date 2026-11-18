// Converted from the chat template of tencent/Youtu-LLM-2B by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let ns_0 = ctx["ns"];
  let tool_ns_0 = ctx["tool_ns"];
  let messages_0 = ctx["messages"];
  let tools_0 = ctx["tools"] ?? null;
  let bos_token_0 = ctx["bos_token"];
  let content_0 = ctx["content"];
  let tool_call_str_0 = ctx["tool_call_str"];
  let enable_thinking_0 = ctx["enable_thinking"];
  let forced_thinking_0 = ctx["forced_thinking"];
  if (J.t(!J.t((add_generation_prompt_0 !== undefined)))) {
    add_generation_prompt_0 = false;
  }
  ns_0 = J.call(J.namespace, J.kw({"is_first": false, "is_tool": false, "is_tool_message": false, "first_tool_index": J.filter("length", messages_0), "is_output_first": true, "system_prompt": "", "is_first_sp": true, "is_last_user": false}));
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
      if (J.t(((v) => J.t(v) ? ((v) => J.t(v) ? v : ((v) => J.t(v) ? J.call(J.attr(J.item(message_1, "content"), "endswith"), "</tool_response>") : v)(((v) => J.t(v) ? J.call(J.attr(J.item(message_1, "content"), "startswith"), "<tool_response>") : v)((J.eq(J.item(message_1, "role"), "user")))))((J.eq(J.item(message_1, "role"), "tool"))) : v)(!J.t(J.attr(ns_0, "is_tool_message"))))) {
        J.setattr(ns_0, "is_tool_message", true);
        J.setattr(ns_0, "first_tool_index", J.attr(_loop1, "index0"));
      }
    }
  }
  if (J.t(((v) => J.t(v) ? !J.t((tools_0 === null)) : v)((tools_0 !== undefined)))) {
    tool_ns_0 = J.call(J.namespace, J.kw({"text": "<|begin_of_tool_description|>Tool calling capabilities.\nYou may call one or more functions to assist with the user query. You have the following functions available:", "return_text": "For tool call returns, you MUST use the following format:\n<tool_call>{\"name\": \"function-name\", \"arguments\": {\"param1\": \"value1\", \"param2\": \"value2\"}}</tool_call>\n<|end_of_tool_description|>"}));
    {
      let _items2 = J.iter(tools_0);
      const _loop2 = J.loop(_items2);
      for (let _i2 = 0; _i2 < _items2.length; _i2++) {
        _loop2.index0 = _i2; const _it2 = _items2[_i2]; let tool_2 = _it2;
        J.setattr(tool_ns_0, "text", J.add(J.add(J.add(J.attr(tool_ns_0, "text"), "\n```json\n"), J.filter("tojson", tool_2)), "\n```"));
      }
    }
    J.setattr(tool_ns_0, "text", J.add(J.add(J.attr(tool_ns_0, "text"), "\n"), J.attr(tool_ns_0, "return_text")));
    if (J.t((J.eq(J.attr(ns_0, "system_prompt"), "")))) {
      J.setattr(ns_0, "system_prompt", J.attr(tool_ns_0, "text"));
    } else {
      J.setattr(ns_0, "system_prompt", J.add(J.add(J.attr(ns_0, "system_prompt"), "\n\n"), J.attr(tool_ns_0, "text")));
    }
  }
  out.push(J.str(bos_token_0), J.str(J.attr(ns_0, "system_prompt")));
  {
    let _items3 = J.iter(messages_0);
    const _loop3 = J.loop(_items3);
    for (let _i3 = 0; _i3 < _items3.length; _i3++) {
      _loop3.index0 = _i3; const _it3 = _items3[_i3]; let message_3 = _it3;
      let content_4 = content_0;
      content_4 = J.item(message_3, "content");
      if (J.t((J.eq(J.item(message_3, "role"), "user")))) {
        J.setattr(ns_0, "is_tool", false);
        J.setattr(ns_0, "is_first", false);
        J.setattr(ns_0, "is_last_user", true);
        out.push(J.str(J.add("<|User|>", content_4)));
      }
      if (J.t((J.eq(J.item(message_3, "role"), "assistant")))) {
        if (J.t(((v) => J.t(v) ? (J.lt(J.attr(_loop3, "index0"), J.sub(J.attr(ns_0, "first_tool_index"), 1))) : v)(((v) => J.t(v) ? !J.t(J.attr(_loop3, "last")) : v)((J.has("</think>", content_4)))))) {
          content_4 = J.call(J.attr(J.item(J.call(J.attr(content_4, "rsplit"), "</think>", 1), J.neg(1)), "lstrip"), "\n");
        }
        if (J.t(((v) => J.t(v) ? J.attr(_loop3, "last") : v)(((v) => J.t(v) ? (!J.has("</think>", content_4)) : v)((!J.has("<think>", content_4)))))) {
          content_4 = J.add("<think>\n\n</think>\n\n", content_4);
        }
      }
      if (J.t(((v) => J.t(v) ? !J.t((J.item(message_3, "tool_calls") === null)) : v)(((v) => J.t(v) ? (J.item(message_3, "tool_calls") !== undefined) : v)((J.eq(J.item(message_3, "role"), "assistant")))))) {
        J.setattr(ns_0, "is_last_user", false);
        out.push(J.str("<|Assistant|>"));
        if (J.t(!J.t((content_4 === null)))) {
          out.push(J.str(content_4));
        }
        J.setattr(ns_0, "is_first", false);
        J.setattr(ns_0, "is_tool", false);
        J.setattr(ns_0, "is_output_first", true);
        {
          let _items4 = J.iter(J.item(message_3, "tool_calls"));
          const _loop4 = J.loop(_items4);
          for (let _i4 = 0; _i4 < _items4.length; _i4++) {
            _loop4.index0 = _i4; const _it4 = _items4[_i4]; let tool_5 = _it4;
            let tool_call_str_6 = tool_call_str_0;
            if (J.t(J.test("string", J.item(J.item(tool_5, "function"), "arguments")))) {
              tool_call_str_6 = J.add(J.add(J.add(J.add("{\"name\": \"", J.item(J.item(tool_5, "function"), "name")), "\", \"arguments\": "), J.item(J.item(tool_5, "function"), "arguments")), "}");
            } else {
              tool_call_str_6 = J.add(J.add(J.add(J.add("{\"name\": \"", J.item(J.item(tool_5, "function"), "name")), "\", \"arguments\": "), J.filter("tojson", J.item(J.item(tool_5, "function"), "arguments"))), "}");
            }
            if (J.t(!J.t(J.attr(ns_0, "is_first")))) {
              out.push(J.str(J.add(J.add("<tool_call>", tool_call_str_6), "</tool_call>")));
              J.setattr(ns_0, "is_first", true);
            } else {
              out.push(J.str(J.add(J.add(J.add("\n", "<tool_call>"), tool_call_str_6), "</tool_call>")));
            }
          }
        }
        out.push(J.str("<|end_of_text|>"));
      }
      if (J.t(((v) => J.t(v) ? ((v) => J.t(v) ? v : (J.item(message_3, "tool_calls") === null))(!J.t((J.item(message_3, "tool_calls") !== undefined))) : v)((J.eq(J.item(message_3, "role"), "assistant"))))) {
        J.setattr(ns_0, "is_last_user", false);
        J.setattr(ns_0, "is_tool", false);
        J.setattr(ns_0, "is_output_first", true);
        out.push(J.str(J.add(J.add("<|Assistant|>", content_4), "<|end_of_text|>")));
      }
      if (J.t((J.eq(J.item(message_3, "role"), "tool")))) {
        J.setattr(ns_0, "is_last_user", false);
        J.setattr(ns_0, "is_tool", true);
        if (J.t(J.attr(ns_0, "is_output_first"))) {
          out.push(J.str(J.add(J.add("<|User|><tool_response>", content_4), "</tool_response>")));
          J.setattr(ns_0, "is_output_first", false);
        } else {
          out.push(J.str(J.add(J.add("\n<tool_response>", content_4), "</tool_response>")));
        }
      }
    }
  }
  if (J.t(((v) => J.t(v) ? ((v) => J.t(v) ? v : J.attr(ns_0, "is_tool"))(J.attr(ns_0, "is_last_user")) : v)(add_generation_prompt_0))) {
    out.push(J.str("<|Assistant|>"));
    if (J.t(((v) => J.t(v) ? J.test("false", enable_thinking_0) : v)((enable_thinking_0 !== undefined)))) {
      out.push(J.str("<think>\n\n</think>\n\n"));
    } else if (J.t(((v) => J.t(v) ? J.test("true", forced_thinking_0) : v)((forced_thinking_0 !== undefined)))) {
      out.push(J.str("<think>\n"));
    }
  }
  return out.join("");
}
