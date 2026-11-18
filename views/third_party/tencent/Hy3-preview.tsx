// Converted from the chat template of tencent/Hy3-preview by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let bos_token_0 = ctx["bos_token"];
  let pad_token_0 = ctx["pad_token"];
  let user_token_0 = ctx["user_token"];
  let assistant_token_0 = ctx["assistant_token"];
  let eos_token_0 = ctx["eos_token"];
  let think_begin_token_0 = ctx["think_begin_token"];
  let think_end_token_0 = ctx["think_end_token"];
  let toolcalls_begin_token_0 = ctx["toolcalls_begin_token"];
  let toolcalls_end_token_0 = ctx["toolcalls_end_token"];
  let toolcall_begin_token_0 = ctx["toolcall_begin_token"];
  let toolcall_end_token_0 = ctx["toolcall_end_token"];
  let toolsep_token_0 = ctx["toolsep_token"];
  let argkey_begin_token_0 = ctx["argkey_begin_token"];
  let argkey_end_token_0 = ctx["argkey_end_token"];
  let argvalue_begin_token_0 = ctx["argvalue_begin_token"];
  let argvalue_end_token_0 = ctx["argvalue_end_token"];
  let toolresponses_begin_token_0 = ctx["toolresponses_begin_token"];
  let toolresponses_end_token_0 = ctx["toolresponses_end_token"];
  let toolresponse_begin_token_0 = ctx["toolresponse_begin_token"];
  let toolresponse_end_token_0 = ctx["toolresponse_end_token"];
  let reasoning_mode_token_0 = ctx["reasoning_mode_token"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let interleaved_thinking_0 = ctx["interleaved_thinking"];
  let is_training_0 = ctx["is_training"];
  let reasoning_effort_0 = ctx["reasoning_effort"];
  let visible_text_0 = ctx["visible_text"];
  let ns_0 = ctx["ns"];
  let sp_ns_0 = ctx["sp_ns"];
  let prev_ns_0 = ctx["prev_ns"];
  let last_ns_0 = ctx["last_ns"];
  let tools_0 = ctx["tools"] ?? null;
  let messages_0 = ctx["messages"];
  let rc_0 = ctx["rc"];
  let content_0 = ctx["content"];
  let arguments_0 = ctx["arguments"];
  bos_token_0 = "<\uff5chy_begin\u2581of\u2581sentence\uff5c>";
  pad_token_0 = "<\uff5chy_\u2581pad\u2581\uff5c>";
  user_token_0 = "<\uff5chy_User\uff5c>";
  assistant_token_0 = "<\uff5chy_Assistant\uff5c>";
  eos_token_0 = "<\uff5chy_eos\uff5c>";
  think_begin_token_0 = "<think>";
  think_end_token_0 = "</think>";
  toolcalls_begin_token_0 = "<tool_calls>";
  toolcalls_end_token_0 = "</tool_calls>";
  toolcall_begin_token_0 = "<tool_call>";
  toolcall_end_token_0 = "</tool_call>";
  toolsep_token_0 = "<tool_sep>";
  argkey_begin_token_0 = "<arg_key>";
  argkey_end_token_0 = "</arg_key>";
  argvalue_begin_token_0 = "<arg_value>";
  argvalue_end_token_0 = "</arg_value>";
  toolresponses_begin_token_0 = "<tool_responses>";
  toolresponses_end_token_0 = "</tool_responses>";
  toolresponse_begin_token_0 = "<tool_response>";
  toolresponse_end_token_0 = "</tool_response>";
  reasoning_mode_token_0 = "<\uff5creasoning_mode\uff5c>";
  if (J.t(!J.t((add_generation_prompt_0 !== undefined)))) {
    add_generation_prompt_0 = false;
  }
  if (J.t(!J.t((interleaved_thinking_0 !== undefined)))) {
    interleaved_thinking_0 = false;
  }
  if (J.t(!J.t(tools_0))) {
    interleaved_thinking_0 = false;
  }
  if (J.t(!J.t((is_training_0 !== undefined)))) {
    is_training_0 = false;
  }
  if (J.t(((v) => J.t(v) ? v : (!J.has(reasoning_effort_0, ["high", "low", "no_think"])))(!J.t((reasoning_effort_0 !== undefined))))) {
    reasoning_effort_0 = "no_think";
  }
  visible_text_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_2 = k.caller;
    let content_1 = p.length > 0 ? p[0] : "content" in k ? k["content"] : undefined;
    if (J.t(J.test("string", content_1))) {
      out.push(J.str(content_1));
    } else if (J.t(((v) => J.t(v) ? !J.t(J.test("mapping", content_1)) : v)(J.test("iterable", content_1)))) {
      {
        let _items1 = J.iter(content_1);
        const _loop1 = J.loop(_items1);
        for (let _i1 = 0; _i1 < _items1.length; _i1++) {
          _loop1.index0 = _i1; const _it1 = _items1[_i1]; let item_3 = _it1;
          if (J.t(((v) => J.t(v) ? (J.eq(J.attr(item_3, "type"), "text")) : v)(J.test("mapping", item_3)))) {
            out.push(J.str(J.attr(item_3, "text")));
          } else if (J.t(J.test("string", item_3))) {
            out.push(J.str(item_3));
          }
        }
      }
    } else if (J.t((content_1 === null))) {
      out.push(J.str(""));
    } else {
      out.push(J.str(content_1));
    }
    return out.join("");
  };
  ns_0 = J.call(J.namespace, J.kw({"last_user_index": J.neg(1)}));
  sp_ns_0 = J.call(J.namespace, J.kw({"system_prompt": "", "is_first_sp": true}));
  {
    let _items2 = J.iter(messages_0);
    const _loop2 = J.loop(_items2);
    for (let _i2 = 0; _i2 < _items2.length; _i2++) {
      _loop2.index0 = _i2; const _it2 = _items2[_i2]; let message_4 = _it2;
      if (J.t((J.eq(J.item(message_4, "role"), "system")))) {
        J.setattr(sp_ns_0, "system_prompt", J.add(J.attr(sp_ns_0, "system_prompt"), J.call(visible_text_0, J.item(message_4, "content"))));
      }
      if (J.t((J.eq(J.item(message_4, "role"), "user")))) {
        J.setattr(ns_0, "last_user_index", J.attr(_loop2, "index0"));
      }
    }
  }
  if (J.t(((v) => J.t(v) ? !J.t(tools_0) : v)(((v) => J.t(v) ? (!J.eq(reasoning_effort_0, "")) : v)(((v) => J.t(v) ? J.test("string", reasoning_effort_0) : v)((reasoning_effort_0 !== undefined)))))) {
    J.setattr(sp_ns_0, "system_prompt", J.add(J.add(J.add(J.attr(sp_ns_0, "system_prompt"), reasoning_mode_token_0), "reasoning_effort:"), reasoning_effort_0));
  }
  out.push(J.str(bos_token_0), J.str(J.attr(sp_ns_0, "system_prompt")));
  if (J.t(tools_0)) {
    if (J.t((!J.eq(J.attr(sp_ns_0, "system_prompt"), "")))) {
      out.push(J.str("\n\n# Tools\n\nYou may call one or more functions to assist with the user query."));
    } else {
      out.push(J.str("# Tools\n\nYou may call one or more functions to assist with the user query."));
    }
    out.push(J.str("\n\nYou are provided with function signatures within <tools></tools> XML tags:"), J.str("\n<tools>\n"));
    {
      let _items3 = J.iter(tools_0);
      const _loop3 = J.loop(_items3);
      for (let _i3 = 0; _i3 < _items3.length; _i3++) {
        _loop3.index0 = _i3; const _it3 = _items3[_i3]; let tool_5 = _it3;
        if (J.t((J.gt(J.attr(_loop3, "index0"), 0)))) {
          out.push(J.str("\n"));
        }
        out.push(J.str(J.filter("tojson", tool_5)));
      }
    }
    out.push(J.str("\n</tools>\n\n"), J.str(J.add(J.add("For function call returns, you should first print ", toolcalls_begin_token_0), "\n")), J.str("For each function call, you should return object like:\n"), J.str(J.add(J.add(J.add(toolcall_begin_token_0, "{function-name}"), toolsep_token_0), "\n")), J.str(J.add(J.add(J.add(argkey_begin_token_0, "{arg-key-1}"), argkey_end_token_0), "\n")), J.str(J.add(J.add(J.add(argvalue_begin_token_0, "{arg-value-1}"), argvalue_end_token_0), "\n")), J.str(J.add(J.add(J.add(argkey_begin_token_0, "{arg-key-2}"), argkey_end_token_0), "\n")), J.str(J.add(J.add(J.add(argvalue_begin_token_0, "{arg-value-2}"), argvalue_end_token_0), "\n")), J.str("...\n"), J.str(J.add(toolcall_end_token_0, "\n")));
    if (J.t(((v) => J.t(v) ? (!J.eq(reasoning_effort_0, "")) : v)(((v) => J.t(v) ? J.test("string", reasoning_effort_0) : v)((reasoning_effort_0 !== undefined))))) {
      out.push(J.str(J.add(J.add(J.add(J.add("At the end of function call returns, you should print ", toolcalls_end_token_0), reasoning_mode_token_0), "reasoning_effort:"), reasoning_effort_0)));
    } else {
      out.push(J.str(J.add("At the end of function call returns, you should print ", toolcalls_end_token_0)));
    }
  }
  prev_ns_0 = J.call(J.namespace, J.kw({"is_tool": false, "is_tool_first": true}));
  last_ns_0 = J.call(J.namespace, J.kw({"last_is_assistant": false}));
  {
    let _items4 = J.iter(messages_0);
    const _loop4 = J.loop(_items4);
    for (let _i4 = 0; _i4 < _items4.length; _i4++) {
      _loop4.index0 = _i4; const _it4 = _items4[_i4]; let message_6 = _it4;
      let rc_7 = rc_0;
      let content_8 = content_0;
      if (J.t((J.eq(J.item(message_6, "role"), "user")))) {
        if (J.t(J.attr(prev_ns_0, "is_tool"))) {
          out.push(J.str(toolresponses_end_token_0));
        }
        out.push(J.str(J.add(user_token_0, J.call(visible_text_0, J.item(message_6, "content")))));
        J.setattr(prev_ns_0, "is_tool", false);
      }
      if (J.t((J.eq(J.item(message_6, "role"), "assistant")))) {
        if (J.t(((v) => J.t(v) ? J.test("string", J.item(message_6, "reasoning_content")) : v)((J.has("reasoning_content", message_6))))) {
          rc_7 = J.item(message_6, "reasoning_content");
        } else if (J.t(((v) => J.t(v) ? J.test("string", J.item(message_6, "reasoning")) : v)((J.has("reasoning", message_6))))) {
          rc_7 = J.item(message_6, "reasoning");
        } else {
          rc_7 = null;
        }
        if (J.t(is_training_0)) {
          if (J.t(!J.t((rc_7 === null)))) {
            content_8 = J.add(J.add(J.add(think_begin_token_0, rc_7), think_end_token_0), J.call(visible_text_0, J.item(message_6, "content")));
          } else {
            content_8 = J.add(J.add(think_begin_token_0, think_end_token_0), J.call(visible_text_0, J.item(message_6, "content")));
          }
        } else {
          if (J.t(interleaved_thinking_0)) {
            if (J.t(((v) => J.t(v) ? !J.t((rc_7 === null)) : v)((J.gt(J.attr(_loop4, "index0"), J.attr(ns_0, "last_user_index")))))) {
              content_8 = J.add(J.add(J.add(think_begin_token_0, rc_7), think_end_token_0), J.call(visible_text_0, J.item(message_6, "content")));
            } else {
              content_8 = J.add(J.add(think_begin_token_0, think_end_token_0), J.call(visible_text_0, J.item(message_6, "content")));
            }
          } else {
            content_8 = J.add(J.add(think_begin_token_0, think_end_token_0), J.call(visible_text_0, J.item(message_6, "content")));
          }
        }
        if (J.t(J.attr(prev_ns_0, "is_tool"))) {
          out.push(J.str(toolresponses_end_token_0));
        }
        out.push(J.str(assistant_token_0));
        if (J.t(((v) => J.t(v) ? J.item(message_6, "tool_calls") : v)((J.item(message_6, "tool_calls") !== undefined)))) {
          J.setattr(prev_ns_0, "is_tool_first", true);
          out.push(J.str(content_8), J.str(J.add(toolcalls_begin_token_0, "\n")));
          {
            let _items5 = J.iter(J.item(message_6, "tool_calls"));
            const _loop5 = J.loop(_items5);
            for (let _i5 = 0; _i5 < _items5.length; _i5++) {
              _loop5.index0 = _i5; const _it5 = _items5[_i5]; let tool_9 = _it5;
              let arguments_10 = arguments_0;
              arguments_10 = J.item(J.item(tool_9, "function"), "arguments");
              out.push(J.str(J.add(J.add(J.add(toolcall_begin_token_0, J.item(J.item(tool_9, "function"), "name")), toolsep_token_0), "\n")));
              {
                let _items6 = J.iter(J.call(J.attr(arguments_10, "items")));
                const _loop6 = J.loop(_items6);
                for (let _i6 = 0; _i6 < _items6.length; _i6++) {
                  _loop6.index0 = _i6; const _it6 = _items6[_i6]; let [key_11, value_12] = J.unpack(_it6, 2);
                  out.push(J.str(J.add(J.add(J.add(argkey_begin_token_0, key_11), argkey_end_token_0), "\n")));
                  if (J.t(!J.t(J.test("string", value_12)))) {
                    value_12 = J.filter("tojson", value_12, J.kw({"ensure_ascii": false}));
                  }
                  out.push(J.str(J.add(J.add(J.add(argvalue_begin_token_0, value_12), argvalue_end_token_0), "\n")));
                }
              }
              out.push(J.str(J.add(toolcall_end_token_0, "\n")));
            }
          }
          out.push(J.str(J.add(toolcalls_end_token_0, eos_token_0)));
        } else {
          if (J.t(((v) => J.t(v) ? v : is_training_0)(!J.t(J.attr(_loop4, "last"))))) {
            out.push(J.str(J.add(content_8, eos_token_0)));
          } else {
            out.push(J.str(content_8));
          }
        }
        J.setattr(prev_ns_0, "is_tool", false);
      }
      if (J.t((J.eq(J.item(message_6, "role"), "tool")))) {
        J.setattr(prev_ns_0, "is_tool", true);
        if (J.t(J.attr(prev_ns_0, "is_tool_first"))) {
          out.push(J.str(J.add(toolresponses_begin_token_0, "\n")));
          J.setattr(prev_ns_0, "is_tool_first", false);
        }
        out.push(J.str(J.add(J.add(J.add(J.add(J.add(toolresponse_begin_token_0, "\n"), J.call(visible_text_0, J.item(message_6, "content"))), "\n"), toolresponse_end_token_0), "\n")));
      }
      if (J.t(((v) => J.t(v) ? (J.eq(J.item(message_6, "role"), "assistant")) : v)(J.attr(_loop4, "last")))) {
        J.setattr(last_ns_0, "last_is_assistant", true);
      }
    }
  }
  if (J.t(J.attr(prev_ns_0, "is_tool"))) {
    out.push(J.str(toolresponses_end_token_0));
  }
  if (J.t(add_generation_prompt_0)) {
    if (J.t(!J.t(J.attr(last_ns_0, "last_is_assistant")))) {
      if (J.t(((v) => J.t(v) ? (J.has(reasoning_effort_0, ["low", "high"])) : v)((reasoning_effort_0 !== undefined)))) {
        out.push(J.str(J.add(assistant_token_0, think_begin_token_0)));
      } else if (J.t(((v) => J.t(v) ? (J.eq(reasoning_effort_0, "no_think")) : v)((reasoning_effort_0 !== undefined)))) {
        out.push(J.str(J.add(J.add(assistant_token_0, think_begin_token_0), think_end_token_0)));
      } else {
        out.push(J.str(assistant_token_0));
      }
    }
  }
  return out.join("");
}
