// Converted from the chat template of Diffuspace/Clover-35B-A3B-Preview by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let render_content_0 = ctx["render_content"];
  let content_0 = ctx["content"];
  let ns_0 = ctx["ns"];
  let messages_0 = ctx["messages"];
  let tools_0 = ctx["tools"] ?? null;
  let index_0 = ctx["index"];
  let reasoning_content_0 = ctx["reasoning_content"];
  let preserve_0 = ctx["preserve"];
  let add_non_think_0 = ctx["add_non_think"];
  let has_reasoning_0 = ctx["has_reasoning"];
  let after_last_query_0 = ctx["after_last_query"];
  let wrap_thinking_0 = ctx["wrap_thinking"];
  let think_prefix_0 = ctx["think_prefix"];
  let assistant_body_0 = ctx["assistant_body"];
  let preserve_thinking_0 = ctx["preserve_thinking"];
  let add_non_thinking_prefix_0 = ctx["add_non_thinking_prefix"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let enable_thinking_0 = ctx["enable_thinking"];
  render_content_0 = (...args) => {
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
          if (J.t(((v) => J.t(v) ? (J.has("text", item_3)) : v)((J.eq(J.attr(item_3, "type"), "text"))))) {
            out.push(J.str(J.attr(item_3, "text")));
          } else {
            out.push(J.str(J.call(J.raise_exception, "This model accepts text content only.")));
          }
        }
      }
    } else if (J.t(((v) => J.t(v) ? v : (content_1 === undefined))((content_1 === null)))) {
      out.push(J.str(""));
    } else {
      out.push(J.str(J.call(J.raise_exception, "Unexpected content type.")));
    }
    return out.join("");
  };
  if (J.t(!J.t(messages_0))) {
    out.push(J.str(J.call(J.raise_exception, "No messages provided.")));
  }
  if (J.t(((v) => J.t(v) ? !J.t(J.test("mapping", tools_0)) : v)(((v) => J.t(v) ? J.test("iterable", tools_0) : v)(tools_0)))) {
    out.push(J.str("<|im_start|>system\n"), J.str("# Tools\n\nYou have access to the following functions:\n\n<tools>"));
    {
      let _items2 = J.iter(tools_0);
      const _loop2 = J.loop(_items2);
      for (let _i2 = 0; _i2 < _items2.length; _i2++) {
        _loop2.index0 = _i2; const _it2 = _items2[_i2]; let tool_4 = _it2;
        out.push(J.str("\n"), J.str(J.filter("tojson", tool_4)));
      }
    }
    out.push(J.str("\n</tools>"), J.str("\n\nIf you choose to call a function ONLY reply in the following format with NO suffix:\n\n<tool_call>\n<function=example_function_name>\n<parameter=example_parameter_1>\nvalue_1\n</parameter>\n<parameter=example_parameter_2>\nThis is the value for the second parameter\nthat can span\nmultiple lines\n</parameter>\n</function>\n</tool_call>\n\n<IMPORTANT>\nReminder:\n- Function calls MUST follow the specified format: an inner <function=...></function> block must be nested within <tool_call></tool_call> XML tags\n- Required parameters MUST be specified\n- You may provide optional reasoning for your function call in natural language BEFORE the function call, but NOT after\n- If there is no function call available, answer the question like normal with your current knowledge and do not tell the user about function calls\n</IMPORTANT>"));
    if (J.t((J.eq(J.attr(J.item(messages_0, 0), "role"), "system")))) {
      content_0 = J.filter("trim", J.call(render_content_0, J.attr(J.item(messages_0, 0), "content")));
      if (J.t(content_0)) {
        out.push(J.str(J.add("\n\n", content_0)));
      }
    }
    out.push(J.str("<|im_end|>\n"));
  } else {
    if (J.t((J.eq(J.attr(J.item(messages_0, 0), "role"), "system")))) {
      content_0 = J.filter("trim", J.call(render_content_0, J.attr(J.item(messages_0, 0), "content")));
      out.push(J.str(J.add(J.add("<|im_start|>system\n", content_0), "<|im_end|>\n")));
    }
  }
  ns_0 = J.call(J.namespace, J.kw({"multi_step_tool": true, "last_query_index": J.sub(J.filter("length", messages_0), 1)}));
  {
    let _items3 = J.iter(J.slice(messages_0, undefined, undefined, J.neg(1)));
    const _loop3 = J.loop(_items3);
    for (let _i3 = 0; _i3 < _items3.length; _i3++) {
      _loop3.index0 = _i3; const _it3 = _items3[_i3]; let message_5 = _it3;
      let index_6 = index_0;
      let content_7 = content_0;
      index_6 = J.sub(J.sub(J.filter("length", messages_0), 1), J.attr(_loop3, "index0"));
      if (J.t(((v) => J.t(v) ? (J.eq(J.attr(message_5, "role"), "user")) : v)(J.attr(ns_0, "multi_step_tool")))) {
        content_7 = J.filter("trim", J.call(render_content_0, J.attr(message_5, "content")));
        if (J.t(!J.t(((v) => J.t(v) ? J.call(J.attr(content_7, "endswith"), "</tool_response>") : v)(J.call(J.attr(content_7, "startswith"), "<tool_response>"))))) {
          J.setattr(ns_0, "multi_step_tool", false);
          J.setattr(ns_0, "last_query_index", index_6);
        }
      }
    }
  }
  if (J.t(J.attr(ns_0, "multi_step_tool"))) {
    out.push(J.str(J.call(J.raise_exception, "No user query found in messages.")));
  }
  {
    let _items4 = J.iter(messages_0);
    const _loop4 = J.loop(_items4);
    for (let _i4 = 0; _i4 < _items4.length; _i4++) {
      _loop4.index0 = _i4; const _it4 = _items4[_i4]; let message_8 = _it4;
      let content_9 = content_0;
      let reasoning_content_10 = reasoning_content_0;
      let preserve_11 = preserve_0;
      let add_non_think_12 = add_non_think_0;
      let has_reasoning_13 = has_reasoning_0;
      let after_last_query_14 = after_last_query_0;
      let wrap_thinking_15 = wrap_thinking_0;
      let think_prefix_16 = think_prefix_0;
      let assistant_body_17 = assistant_body_0;
      content_9 = J.filter("trim", J.call(render_content_0, J.attr(message_8, "content")));
      if (J.t((J.eq(J.attr(message_8, "role"), "system")))) {
        if (J.t(!J.t(J.attr(_loop4, "first")))) {
          out.push(J.str(J.call(J.raise_exception, "System message must be at the beginning.")));
        }
      } else if (J.t((J.eq(J.attr(message_8, "role"), "user")))) {
        out.push(J.str(J.add(J.add(J.add(J.add(J.add("<|im_start|>", J.attr(message_8, "role")), "\n"), content_9), "<|im_end|>"), "\n")));
      } else if (J.t((J.eq(J.attr(message_8, "role"), "assistant")))) {
        reasoning_content_10 = "";
        if (J.t(J.test("string", J.attr(message_8, "reasoning_content")))) {
          reasoning_content_10 = J.attr(message_8, "reasoning_content");
        } else {
          if (J.t((J.has("</think>", content_9)))) {
            reasoning_content_10 = J.call(J.attr(J.item(J.call(J.attr(J.call(J.attr(J.item(J.call(J.attr(content_9, "split"), "</think>"), 0), "rstrip"), "\n"), "split"), "<think>"), J.neg(1)), "lstrip"), "\n");
            content_9 = J.call(J.attr(J.item(J.call(J.attr(content_9, "split"), "</think>"), J.neg(1)), "lstrip"), "\n");
          }
        }
        reasoning_content_10 = J.filter("trim", reasoning_content_10);
        preserve_11 = ((v) => J.t(v) ? v : J.test("true", preserve_thinking_0))(!J.t((preserve_thinking_0 !== undefined)));
        add_non_think_12 = ((v) => J.t(v) ? v : J.test("true", add_non_thinking_prefix_0))(!J.t((add_non_thinking_prefix_0 !== undefined)));
        has_reasoning_13 = (J.gt(J.filter("length", reasoning_content_10), 0));
        after_last_query_14 = (J.gt(J.attr(_loop4, "index0"), J.attr(ns_0, "last_query_index")));
        wrap_thinking_15 = ((v) => J.t(v) ? ((v) => J.t(v) ? v : after_last_query_14)(preserve_11) : v)(((v) => J.t(v) ? v : add_non_think_12)(has_reasoning_13));
        if (J.t(wrap_thinking_15)) {
          if (J.t(has_reasoning_13)) {
            think_prefix_16 = J.add(J.add("<think>\n", reasoning_content_10), "\n</think>\n\n");
          } else {
            think_prefix_16 = "<think>\n\n</think>\n\n";
          }
        } else {
          think_prefix_16 = "";
        }
        assistant_body_17 = J.add(think_prefix_16, J.call(J.attr(content_9, "lstrip"), "\n"));
        out.push(J.str(J.add(J.add("<|im_start|>", J.attr(message_8, "role")), "\n")), J.str(assistant_body_17));
        if (J.t(((v) => J.t(v) ? !J.t(J.test("mapping", J.attr(message_8, "tool_calls"))) : v)(((v) => J.t(v) ? J.test("iterable", J.attr(message_8, "tool_calls")) : v)(J.attr(message_8, "tool_calls"))))) {
          {
            let _items5 = J.iter(J.attr(message_8, "tool_calls"));
            const _loop5 = J.loop(_items5);
            for (let _i5 = 0; _i5 < _items5.length; _i5++) {
              _loop5.index0 = _i5; const _it5 = _items5[_i5]; let tool_call_18 = _it5;
              if (J.t((J.attr(tool_call_18, "function") !== undefined))) {
                tool_call_18 = J.attr(tool_call_18, "function");
              }
              if (J.t(J.attr(_loop5, "first"))) {
                if (J.t(J.filter("trim", content_9))) {
                  out.push(J.str(J.add(J.add("\n\n<tool_call>\n<function=", J.attr(tool_call_18, "name")), ">\n")));
                } else {
                  out.push(J.str(J.add(J.add("<tool_call>\n<function=", J.attr(tool_call_18, "name")), ">\n")));
                }
              } else {
                out.push(J.str(J.add(J.add("\n<tool_call>\n<function=", J.attr(tool_call_18, "name")), ">\n")));
              }
              if (J.t((J.attr(tool_call_18, "arguments") !== undefined))) {
                {
                  let _items6 = J.iter(J.filter("items", J.attr(tool_call_18, "arguments")));
                  const _loop6 = J.loop(_items6);
                  for (let _i6 = 0; _i6 < _items6.length; _i6++) {
                    _loop6.index0 = _i6; const _it6 = _items6[_i6]; let [args_name_19, args_value_20] = J.unpack(_it6, 2);
                    out.push(J.str(J.add(J.add("<parameter=", args_name_19), ">\n")));
                    args_value_20 = (J.t(J.test("string", args_value_20)) ? J.filter("string", args_value_20) : J.filter("safe", J.filter("tojson", args_value_20)));
                    out.push(J.str(args_value_20), J.str("\n</parameter>\n"));
                  }
                }
              }
              out.push(J.str("</function>\n</tool_call>"));
            }
          }
        }
        out.push(J.str("<|im_end|>\n"));
      } else if (J.t((J.eq(J.attr(message_8, "role"), "tool")))) {
        if (J.t(((v) => J.t(v) ? (!J.eq(J.attr(J.attr(_loop4, "previtem"), "role"), "tool")) : v)(J.attr(_loop4, "previtem")))) {
          out.push(J.str("<|im_start|>user"));
        }
        out.push(J.str("\n<tool_response>\n"), J.str(content_9), J.str("\n</tool_response>"));
        if (J.t(((v) => J.t(v) ? (!J.eq(J.attr(J.attr(_loop4, "nextitem"), "role"), "tool")) : v)(!J.t(J.attr(_loop4, "last"))))) {
          out.push(J.str("<|im_end|>\n"));
        } else if (J.t(J.attr(_loop4, "last"))) {
          out.push(J.str("<|im_end|>\n"));
        }
      } else {
        out.push(J.str(J.call(J.raise_exception, "Unexpected message role.")));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|im_start|>assistant\n"));
    if (J.t(((v) => J.t(v) ? J.test("false", enable_thinking_0) : v)((enable_thinking_0 !== undefined)))) {
      out.push(J.str("<think>\n\n</think>\n\n"));
    } else {
      out.push(J.str("<think>\n"));
    }
  }
  return out.join("");
}
