// Converted from the chat template of stepfun-ai/Step-3.7-Flash by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let render_message_content_0 = ctx["render_message_content"];
  let ns_0 = ctx["ns"];
  let bos_token_0 = ctx["bos_token"];
  let tools_0 = ctx["tools"] ?? null;
  let reasoning_effort_0 = ctx["reasoning_effort"];
  let messages_0 = ctx["messages"];
  let index_0 = ctx["index"];
  let content_0 = ctx["content"];
  let role_name_0 = ctx["role_name"];
  let reasoning_content_0 = ctx["reasoning_content"];
  let arguments_0 = ctx["arguments"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  render_message_content_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_2 = k.caller;
    let message_1 = p.length > 0 ? p[0] : "message" in k ? k["message"] : undefined;
    let ns_3 = ns_0;
    if (J.t((J.attr(message_1, "content") === null))) {
      out.push(J.str(""));
    } else if (J.t(J.test("string", J.attr(message_1, "content")))) {
      out.push(J.str(J.attr(message_1, "content")));
    } else if (J.t(J.test("mapping", J.attr(message_1, "content")))) {
      out.push(J.str((J.t((J.has("value", J.attr(message_1, "content")))) ? J.item(J.attr(message_1, "content"), "value") : J.item(J.attr(message_1, "content"), "text"))));
    } else if (J.t(J.test("iterable", J.attr(message_1, "content")))) {
      ns_3 = J.call(J.namespace, J.kw({"needs_text_separator": false}));
      {
        let _items1 = J.iter(J.attr(message_1, "content"));
        const _loop1 = J.loop(_items1);
        for (let _i1 = 0; _i1 < _items1.length; _i1++) {
          _loop1.index0 = _i1; const _it1 = _items1[_i1]; let item_4 = _it1;
          if (J.t((J.eq(J.attr(item_4, "type"), "text")))) {
            if (J.t(J.attr(ns_3, "needs_text_separator"))) {
              out.push(J.str(" "));
            }
            out.push(J.str((J.t((J.has("value", item_4))) ? J.item(item_4, "value") : J.item(item_4, "text"))));
            J.setattr(ns_3, "needs_text_separator", true);
          } else if (J.t((J.eq(J.attr(item_4, "type"), "image")))) {
            out.push("<im_patch>");
            J.setattr(ns_3, "needs_text_separator", false);
          }
        }
      }
    }
    return out.join("");
  };
  out.push(J.str(bos_token_0));
  if (J.t(tools_0)) {
    out.push(J.str("<|im_start|>system\n"));
    if (J.t((reasoning_effort_0 !== undefined))) {
      out.push(J.str(J.add(J.add("Reasoning: ", reasoning_effort_0), "\n\n")));
    }
    if (J.t((J.eq(J.attr(J.item(messages_0, 0), "role"), "system")))) {
      out.push(J.str(J.add(J.call(render_message_content_0, J.item(messages_0, 0)), "\n\n")));
    }
    out.push(J.str("# Tools\n\nYou have access to the following functions in JSONSchema format:\n\n<tools>"));
    {
      let _items2 = J.iter(tools_0);
      const _loop2 = J.loop(_items2);
      for (let _i2 = 0; _i2 < _items2.length; _i2++) {
        _loop2.index0 = _i2; const _it2 = _items2[_i2]; let tool_5 = _it2;
        out.push(J.str("\n"), J.str(J.filter("tojson", tool_5, J.kw({"ensure_ascii": false}))));
      }
    }
    out.push(J.str("\n</tools>\n\nIf you choose to call a function ONLY reply in the following format with NO suffix:\n\n<tool_call>\n<function=example_function_name>\n<parameter=example_parameter_1>\nvalue_1\n</parameter>\n<parameter=example_parameter_2>\nThis is the value for the second parameter\nthat can span\nmultiple lines\n</parameter>\n</function>\n</tool_call>\n\n<IMPORTANT>\nReminder:\n- Function calls MUST follow the specified format: an inner <function=...>\n...\n</function> block must be nested within <tool_call>\n...\n</tool_call> XML tags\n- Required parameters MUST be specified\n</IMPORTANT><|im_end|>\n"));
  } else {
    if (J.t((J.eq(J.attr(J.item(messages_0, 0), "role"), "system")))) {
      out.push(J.str("<|im_start|>system\n"));
      if (J.t((reasoning_effort_0 !== undefined))) {
        out.push(J.str(J.add(J.add("Reasoning: ", reasoning_effort_0), "\n\n")));
      }
      out.push(J.str(J.add(J.call(render_message_content_0, J.item(messages_0, 0)), "<|im_end|>\n")));
    } else if (J.t((reasoning_effort_0 !== undefined))) {
      out.push(J.str(J.add(J.add(J.add(J.add("<|im_start|>system\n", "Reasoning: "), reasoning_effort_0), "\n\n"), "<|im_end|>\n")));
    }
  }
  ns_0 = J.call(J.namespace, J.kw({"multi_step_tool": true, "last_query_index": J.sub(J.filter("length", messages_0), 1)}));
  {
    let _items3 = J.iter(J.slice(messages_0, undefined, undefined, J.neg(1)));
    const _loop3 = J.loop(_items3);
    for (let _i3 = 0; _i3 < _items3.length; _i3++) {
      _loop3.index0 = _i3; const _it3 = _items3[_i3]; let message_6 = _it3;
      let index_7 = index_0;
      index_7 = J.sub(J.sub(J.filter("length", messages_0), 1), J.attr(_loop3, "index0"));
      if (J.t(((v) => J.t(v) ? !J.t(((v) => J.t(v) ? J.call(J.attr(J.call(render_message_content_0, message_6), "endswith"), "</tool_response>") : v)(J.call(J.attr(J.call(render_message_content_0, message_6), "startswith"), "<tool_response>"))) : v)(((v) => J.t(v) ? J.test("string", J.call(render_message_content_0, message_6)) : v)(((v) => J.t(v) ? (J.eq(J.attr(message_6, "role"), "user")) : v)(J.attr(ns_0, "multi_step_tool")))))) {
        J.setattr(ns_0, "multi_step_tool", false);
        J.setattr(ns_0, "last_query_index", index_7);
      }
    }
  }
  {
    let _items4 = J.iter(messages_0);
    const _loop4 = J.loop(_items4);
    for (let _i4 = 0; _i4 < _items4.length; _i4++) {
      _loop4.index0 = _i4; const _it4 = _items4[_i4]; let message_8 = _it4;
      let content_9 = content_0;
      let role_name_10 = role_name_0;
      let reasoning_content_11 = reasoning_content_0;
      content_9 = J.call(render_message_content_0, message_8);
      if (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? !J.t(J.attr(_loop4, "first")) : v)((J.eq(J.attr(message_8, "role"), "system"))))((J.eq(J.attr(message_8, "role"), "user"))))) {
        role_name_10 = (J.t(((v) => J.t(v) ? (J.eq(J.attr(message_8, "name"), "observation")) : v)(((v) => J.t(v) ? !J.t(J.attr(_loop4, "first")) : v)((J.eq(J.attr(message_8, "role"), "system"))))) ? "observation" : J.attr(message_8, "role"));
        out.push(J.str(J.add(J.add(J.add(J.add(J.add("<|im_start|>", role_name_10), "\n"), content_9), "<|im_end|>"), "\n")));
      } else if (J.t((J.eq(J.attr(message_8, "role"), "assistant")))) {
        if (J.t(J.test("string", J.attr(message_8, "reasoning_content")))) {
          reasoning_content_11 = J.attr(message_8, "reasoning_content");
        } else {
          if (J.t((J.has("</think>", content_9)))) {
            reasoning_content_11 = J.call(J.attr(J.item(J.call(J.attr(J.call(J.attr(J.item(J.call(J.attr(content_9, "split"), "</think>"), 0), "rstrip"), "\n"), "split"), "<think>"), J.neg(1)), "lstrip"), "\n");
            content_9 = J.call(J.attr(J.item(J.call(J.attr(content_9, "split"), "</think>"), J.neg(1)), "lstrip"), "\n");
          } else {
            reasoning_content_11 = "";
          }
        }
        if (J.t((J.gt(J.attr(_loop4, "index0"), J.attr(ns_0, "last_query_index"))))) {
          out.push(J.str(J.add(J.add(J.add(J.add(J.add("<|im_start|>", J.attr(message_8, "role")), "\n<think>\n"), reasoning_content_11), "\n</think>\n"), content_9)));
        } else {
          out.push(J.str(J.add(J.add(J.add("<|im_start|>", J.attr(message_8, "role")), "\n"), content_9)));
        }
        if (J.t(J.attr(message_8, "tool_calls"))) {
          {
            let _items5 = J.iter(J.attr(message_8, "tool_calls"));
            const _loop5 = J.loop(_items5);
            for (let _i5 = 0; _i5 < _items5.length; _i5++) {
              _loop5.index0 = _i5; const _it5 = _items5[_i5]; let tool_call_12 = _it5;
              let arguments_13 = arguments_0;
              if (J.t((J.attr(tool_call_12, "function") !== undefined))) {
                tool_call_12 = J.attr(tool_call_12, "function");
              }
              out.push(J.str(J.add(J.add("<tool_call>\n<function=", J.attr(tool_call_12, "name")), ">\n")));
              if (J.t((J.attr(tool_call_12, "arguments") !== undefined))) {
                arguments_13 = (J.t(J.test("string", J.attr(tool_call_12, "arguments"))) ? J.filter("fromjson", J.attr(tool_call_12, "arguments")) : J.attr(tool_call_12, "arguments"));
                {
                  let _items6 = J.iter(J.filter("items", arguments_13));
                  const _loop6 = J.loop(_items6);
                  for (let _i6 = 0; _i6 < _items6.length; _i6++) {
                    _loop6.index0 = _i6; const _it6 = _items6[_i6]; let [args_name_14, args_value_15] = J.unpack(_it6, 2);
                    out.push(J.str(J.add(J.add("<parameter=", args_name_14), ">\n")));
                    args_value_15 = (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? !J.t(J.test("string", args_value_15)) : v)(J.test("sequence", args_value_15)))(J.test("mapping", args_value_15))) ? J.filter("safe", J.filter("tojson", args_value_15, J.kw({"ensure_ascii": false}))) : J.filter("string", args_value_15));
                    out.push(J.str(args_value_15), J.str("\n</parameter>\n"));
                  }
                }
              }
              out.push(J.str("</function>\n</tool_call>"));
            }
          }
        }
        out.push(J.str("<|im_end|>\n"));
      } else if (J.t((J.eq(J.attr(message_8, "role"), "tool")))) {
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.sub(J.attr(_loop4, "index0"), 1)), "role"), "tool")))(J.attr(_loop4, "first")))) {
          out.push(J.str("<|im_start|>tool_response\n"));
        }
        out.push(J.str("<tool_response>"), J.str(content_9), J.str("</tool_response>"));
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.add(J.attr(_loop4, "index0"), 1)), "role"), "tool")))(J.attr(_loop4, "last")))) {
          out.push(J.str("<|im_end|>\n"));
        }
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|im_start|>assistant\n<think>\n"));
  }
  return out.join("");
}
