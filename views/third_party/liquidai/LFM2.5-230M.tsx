// Converted from the chat template of LiquidAI/LFM2.5-230M by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let preserve_thinking_0 = ctx["preserve_thinking"];
  let format_arg_value_0 = ctx["format_arg_value"];
  let parse_content_0 = ctx["parse_content"];
  let render_tool_calls_0 = ctx["render_tool_calls"];
  let ns_0 = ctx["ns"];
  let messages_0 = ctx["messages"];
  let bos_token_0 = ctx["bos_token"];
  let _ns_0 = ctx["_ns"];
  let tool_calls_ns_0 = ctx["tool_calls_ns"];
  let func_name_0 = ctx["func_name"];
  let func_args_0 = ctx["func_args"];
  let args_ns_0 = ctx["args_ns"];
  let tools_0 = ctx["tools"] ?? null;
  let thinking_0 = ctx["thinking"];
  let _cfm_tag_0 = ctx["_cfm_tag"];
  let _has_cfm_0 = ctx["_has_cfm"];
  let content_0 = ctx["content"];
  let _trunc_len_0 = ctx["_trunc_len"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  out.push(J.str(bos_token_0));
  preserve_thinking_0 = J.filter("default", preserve_thinking_0, false);
  format_arg_value_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_2 = k.caller;
    let arg_value_1 = p.length > 0 ? p[0] : "arg_value" in k ? k["arg_value"] : undefined;
    if (J.t(J.test("string", arg_value_1))) {
      out.push(J.str(J.add(J.add("'", arg_value_1), "'")));
    } else if (J.t(J.test("mapping", arg_value_1))) {
      out.push(J.str(J.filter("tojson", arg_value_1)));
    } else {
      out.push(J.str(J.filter("string", arg_value_1)));
    }
    return out.join("");
  };
  parse_content_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_4 = k.caller;
    let content_3 = p.length > 0 ? p[0] : "content" in k ? k["content"] : undefined;
    let _ns_5 = _ns_0;
    if (J.t(J.test("string", content_3))) {
      out.push(J.str(content_3));
    } else {
      _ns_5 = J.call(J.namespace, J.kw({"result": ""}));
      {
        let _items1 = J.iter(content_3);
        const _loop1 = J.loop(_items1);
        for (let _i1 = 0; _i1 < _items1.length; _i1++) {
          _loop1.index0 = _i1; const _it1 = _items1[_i1]; let item_6 = _it1;
          if (J.t((J.eq(J.item(item_6, "type"), "image")))) {
            J.setattr(_ns_5, "result", J.add(J.attr(_ns_5, "result"), "<image>"));
          } else if (J.t((J.eq(J.item(item_6, "type"), "text")))) {
            J.setattr(_ns_5, "result", J.add(J.attr(_ns_5, "result"), J.item(item_6, "text")));
          } else {
            J.setattr(_ns_5, "result", J.add(J.attr(_ns_5, "result"), J.filter("tojson", item_6)));
          }
        }
      }
      out.push(J.str(J.attr(_ns_5, "result")));
    }
    return out.join("");
  };
  render_tool_calls_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_8 = k.caller;
    let tool_calls_7 = p.length > 0 ? p[0] : "tool_calls" in k ? k["tool_calls"] : undefined;
    let tool_calls_ns_9 = tool_calls_ns_0;
    tool_calls_ns_9 = J.call(J.namespace, J.kw({"tool_calls": []}));
    {
      let _items2 = J.iter(tool_calls_7);
      const _loop2 = J.loop(_items2);
      for (let _i2 = 0; _i2 < _items2.length; _i2++) {
        _loop2.index0 = _i2; const _it2 = _items2[_i2]; let tool_call_10 = _it2;
        let func_name_11 = func_name_0;
        let func_args_12 = func_args_0;
        let args_ns_13 = args_ns_0;
        func_name_11 = J.item(J.item(tool_call_10, "function"), "name");
        func_args_12 = J.item(J.item(tool_call_10, "function"), "arguments");
        args_ns_13 = J.call(J.namespace, J.kw({"arg_strings": []}));
        {
          let _items3 = J.iter(J.call(J.attr(func_args_12, "items")));
          const _loop3 = J.loop(_items3);
          for (let _i3 = 0; _i3 < _items3.length; _i3++) {
            _loop3.index0 = _i3; const _it3 = _items3[_i3]; let [arg_name_14, arg_value_15] = J.unpack(_it3, 2);
            J.setattr(args_ns_13, "arg_strings", J.add(J.attr(args_ns_13, "arg_strings"), [J.add(J.add(arg_name_14, "="), J.call(format_arg_value_0, arg_value_15))]));
          }
        }
        J.setattr(tool_calls_ns_9, "tool_calls", J.add(J.attr(tool_calls_ns_9, "tool_calls"), [J.add(J.add(J.add(func_name_11, "("), J.filter("join", J.attr(args_ns_13, "arg_strings"), ", ")), ")")]));
      }
    }
    out.push(J.str(J.add(J.add("<|tool_call_start|>[", J.filter("join", J.attr(tool_calls_ns_9, "tool_calls"), ", ")), "]<|tool_call_end|>")));
    return out.join("");
  };
  ns_0 = J.call(J.namespace, J.kw({"system_prompt": "", "last_user_index": J.neg(1)}));
  if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
    if (J.t(J.call(J.attr(J.item(messages_0, 0), "get"), "content"))) {
      J.setattr(ns_0, "system_prompt", J.call(parse_content_0, J.item(J.item(messages_0, 0), "content")));
    }
    messages_0 = J.slice(messages_0, 1, undefined, undefined);
  }
  if (J.t(tools_0)) {
    J.setattr(ns_0, "system_prompt", J.add(J.add(J.attr(ns_0, "system_prompt"), (J.t(J.attr(ns_0, "system_prompt")) ? "\n" : "")), "List of tools: ["));
    {
      let _items4 = J.iter(tools_0);
      const _loop4 = J.loop(_items4);
      for (let _i4 = 0; _i4 < _items4.length; _i4++) {
        _loop4.index0 = _i4; const _it4 = _items4[_i4]; let tool_16 = _it4;
        if (J.t(!J.t(J.test("string", tool_16)))) {
          tool_16 = J.filter("tojson", tool_16);
        }
        J.setattr(ns_0, "system_prompt", J.add(J.attr(ns_0, "system_prompt"), tool_16));
        if (J.t(!J.t(J.attr(_loop4, "last")))) {
          J.setattr(ns_0, "system_prompt", J.add(J.attr(ns_0, "system_prompt"), ", "));
        }
      }
    }
    J.setattr(ns_0, "system_prompt", J.add(J.attr(ns_0, "system_prompt"), "]"));
  }
  if (J.t(J.attr(ns_0, "system_prompt"))) {
    out.push(J.str(J.add(J.add("<|im_start|>system\n", J.attr(ns_0, "system_prompt")), "<|im_end|>\n")));
  }
  {
    let _items5 = J.iter(messages_0);
    const _loop5 = J.loop(_items5);
    for (let _i5 = 0; _i5 < _items5.length; _i5++) {
      _loop5.index0 = _i5; const _it5 = _items5[_i5]; let message_17 = _it5;
      if (J.t((J.eq(J.item(message_17, "role"), "user")))) {
        J.setattr(ns_0, "last_user_index", J.attr(_loop5, "index0"));
      }
    }
  }
  {
    let _items6 = J.iter(messages_0);
    const _loop6 = J.loop(_items6);
    for (let _i6 = 0; _i6 < _items6.length; _i6++) {
      _loop6.index0 = _i6; const _it6 = _items6[_i6]; let message_18 = _it6;
      out.push(J.str(J.add(J.add("<|im_start|>", J.attr(message_18, "role")), "\n")));
      if (J.t((J.eq(J.attr(message_18, "role"), "assistant")))) {
        {
          thinking_0 = ((v) => J.t(v) ? v : J.attr(message_18, "reasoning_content"))(((v) => J.t(v) ? v : J.attr(message_18, "reasoning"))(J.attr(message_18, "thinking")));
          thinking_0 = (J.t(J.test("string", thinking_0)) ? thinking_0 : "");
          if (J.t(((v) => J.t(v) ? ((v) => J.t(v) ? v : (J.gt(J.attr(_loop6, "index0"), J.attr(ns_0, "last_user_index"))))(preserve_thinking_0) : v)(thinking_0))) {
            out.push(J.str(J.add(J.add("<think>", thinking_0), "</think>")));
          }
          _cfm_tag_0 = "CONTINUE_FINAL_MESSAGE_TAG ";
          _has_cfm_0 = false;
          if (J.t((J.attr(message_18, "content") !== undefined))) {
            content_0 = J.call(parse_content_0, J.attr(message_18, "content"));
            if (J.t(!J.t(((v) => J.t(v) ? v : (J.gt(J.attr(_loop6, "index0"), J.attr(ns_0, "last_user_index"))))(preserve_thinking_0)))) {
              if (J.t((J.has("</think>", content_0)))) {
                content_0 = J.filter("trim", J.item(J.call(J.attr(content_0, "split"), "</think>"), J.neg(1)));
              }
            }
            if (J.t(((v) => J.t(v) ? J.call(J.attr(content_0, "endswith"), _cfm_tag_0) : v)((J.attr(message_18, "tool_calls") !== undefined)))) {
              _has_cfm_0 = true;
              _trunc_len_0 = J.sub(J.filter("length", content_0), J.filter("length", _cfm_tag_0));
              out.push(J.str(J.slice(content_0, undefined, _trunc_len_0, undefined)));
            } else {
              out.push(J.str(content_0));
            }
          }
          if (J.t((J.attr(message_18, "tool_calls") !== undefined))) {
            out.push(J.str(J.call(render_tool_calls_0, J.attr(message_18, "tool_calls"))));
          }
          if (J.t(_has_cfm_0)) {
            out.push(J.str(_cfm_tag_0));
          }
          out.push(J.str("<|im_end|>\n"));
        }
      } else {
        if (J.t(J.call(J.attr(message_18, "get"), "content"))) {
          out.push(J.str(J.call(parse_content_0, J.item(message_18, "content"))));
        }
        out.push(J.str("<|im_end|>\n"));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|im_start|>assistant\n"));
  }
  return out.join("");
}
