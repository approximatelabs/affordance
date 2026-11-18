// Converted from the chat template of KRAFTON/Raon-1-mini-preview by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let truncate_history_thinking_0 = ctx["truncate_history_thinking"];
  let render_content_0 = ctx["render_content"];
  let render_assistant_body_0 = ctx["render_assistant_body"];
  let content_0 = ctx["content"];
  let ns_0 = ctx["ns"];
  let messages_0 = ctx["messages"];
  let tools_0 = ctx["tools"] ?? null;
  let index_0 = ctx["index"];
  let reasoning_content_0 = ctx["reasoning_content"];
  let thinking_parts_0 = ctx["thinking_parts"];
  let preserve_reasoning_0 = ctx["preserve_reasoning"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let enable_thinking_0 = ctx["enable_thinking"];
  truncate_history_thinking_0 = (J.t((truncate_history_thinking_0 !== undefined)) ? truncate_history_thinking_0 : false);
  render_content_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_3 = k.caller;
    let content_1 = p.length > 0 ? p[0] : "content" in k ? k["content"] : undefined;
    let is_system_content_2 = p.length > 1 ? p[1] : "is_system_content" in k ? k["is_system_content"] : false;
    if (J.t(J.test("string", content_1))) {
      out.push(J.str(content_1));
    } else if (J.t(((v) => J.t(v) ? !J.t(J.test("mapping", content_1)) : v)(J.test("iterable", content_1)))) {
      {
        let _items1 = J.iter(content_1);
        const _loop1 = J.loop(_items1);
        for (let _i1 = 0; _i1 < _items1.length; _i1++) {
          _loop1.index0 = _i1; const _it1 = _items1[_i1]; let item_4 = _it1;
          if (J.t((J.has("text", item_4)))) {
            out.push(J.str(J.attr(item_4, "text")));
          } else {
            out.push(J.str(J.call(J.raise_exception, "raon-1-mini supports text content only.")));
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
  render_assistant_body_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_9 = k.caller;
    let message_5 = p.length > 0 ? p[0] : "message" in k ? k["message"] : undefined;
    let content_6 = p.length > 1 ? p[1] : "content" in k ? k["content"] : undefined;
    let reasoning_content_7 = p.length > 2 ? p[2] : "reasoning_content" in k ? k["reasoning_content"] : undefined;
    let preserve_reasoning_8 = p.length > 3 ? p[3] : "preserve_reasoning" in k ? k["preserve_reasoning"] : undefined;
    if (J.t(((v) => J.t(v) ? J.filter("trim", reasoning_content_7) : v)(preserve_reasoning_8))) {
      out.push(J.str(J.add(J.add("<think>\n", J.filter("trim", reasoning_content_7)), "\n</think>\n\n")));
    } else {
      out.push(J.str("<think></think>\n\n"));
    }
    out.push(J.str(content_6));
    if (J.t(((v) => J.t(v) ? !J.t(J.test("mapping", J.attr(message_5, "tool_calls"))) : v)(((v) => J.t(v) ? J.test("iterable", J.attr(message_5, "tool_calls")) : v)(J.attr(message_5, "tool_calls"))))) {
      {
        let _items2 = J.iter(J.attr(message_5, "tool_calls"));
        const _loop2 = J.loop(_items2);
        for (let _i2 = 0; _i2 < _items2.length; _i2++) {
          _loop2.index0 = _i2; const _it2 = _items2[_i2]; let tool_call_10 = _it2;
          if (J.t((J.attr(tool_call_10, "function") !== undefined))) {
            tool_call_10 = J.attr(tool_call_10, "function");
          }
          if (J.t(J.attr(_loop2, "first"))) {
            if (J.t(J.filter("trim", content_6))) {
              out.push(J.str(J.add(J.add("\n\n<tool_call>\n<function=", J.attr(tool_call_10, "name")), ">\n")));
            } else {
              out.push(J.str(J.add(J.add("<tool_call>\n<function=", J.attr(tool_call_10, "name")), ">\n")));
            }
          } else {
            out.push(J.str(J.add(J.add("\n<tool_call>\n<function=", J.attr(tool_call_10, "name")), ">\n")));
          }
          if (J.t((J.attr(tool_call_10, "arguments") !== undefined))) {
            {
              let _items3 = J.iter(J.filter("items", J.attr(tool_call_10, "arguments")));
              const _loop3 = J.loop(_items3);
              for (let _i3 = 0; _i3 < _items3.length; _i3++) {
                _loop3.index0 = _i3; const _it3 = _items3[_i3]; let [args_name_11, args_value_12] = J.unpack(_it3, 2);
                out.push(J.str(J.add(J.add("<parameter=", args_name_11), ">\n")));
                args_value_12 = (J.t(J.test("string", args_value_12)) ? J.filter("string", args_value_12) : J.filter("safe", J.filter("tojson", args_value_12)));
                out.push(J.str(args_value_12), J.str("\n</parameter>\n"));
              }
            }
          }
          out.push(J.str("</function>\n</tool_call>"));
        }
      }
    }
    out.push(J.str("<|im_end|>"));
    return out.join("");
  };
  if (J.t(!J.t(messages_0))) {
    out.push(J.str(J.call(J.raise_exception, "No messages provided.")));
  }
  if (J.t(((v) => J.t(v) ? !J.t(J.test("mapping", tools_0)) : v)(((v) => J.t(v) ? J.test("iterable", tools_0) : v)(tools_0)))) {
    out.push(J.str("<|im_start|>system\n"), J.str("# Tools\n\nYou have access to the following functions:\n\n<tools>"));
    {
      let _items4 = J.iter(tools_0);
      const _loop4 = J.loop(_items4);
      for (let _i4 = 0; _i4 < _items4.length; _i4++) {
        _loop4.index0 = _i4; const _it4 = _items4[_i4]; let tool_13 = _it4;
        out.push(J.str("\n"), J.str(J.filter("tojson", tool_13)));
      }
    }
    out.push(J.str("\n</tools>"), J.str("\n\nIf you choose to call a function ONLY reply in the following format with NO suffix:\n\n<tool_call>\n<function=example_function_name>\n<parameter=example_parameter_1>\nvalue_1\n</parameter>\n<parameter=example_parameter_2>\nThis is the value for the second parameter\nthat can span\nmultiple lines\n</parameter>\n</function>\n</tool_call>\n\n<IMPORTANT>\nReminder:\n- Function calls MUST follow the specified format: an inner <function=...></function> block must be nested within <tool_call></tool_call> XML tags\n- Required parameters MUST be specified\n- You may provide optional reasoning for your function call in natural language BEFORE the function call, but NOT after\n- If there is no function call available, answer the question like normal with your current knowledge and do not tell the user about function calls\n</IMPORTANT>"));
    if (J.t((J.eq(J.attr(J.item(messages_0, 0), "role"), "system")))) {
      content_0 = J.filter("trim", J.call(render_content_0, J.attr(J.item(messages_0, 0), "content"), true));
      if (J.t(content_0)) {
        out.push(J.str(J.add("\n\n", content_0)));
      }
    }
    out.push(J.str("<|im_end|>\n"));
  } else if (J.t((J.eq(J.attr(J.item(messages_0, 0), "role"), "system")))) {
    content_0 = J.filter("trim", J.call(render_content_0, J.attr(J.item(messages_0, 0), "content"), true));
    out.push(J.str(J.add(J.add("<|im_start|>system\n", content_0), "<|im_end|>\n")));
  }
  ns_0 = J.call(J.namespace, J.kw({"multi_step_tool": true, "last_query_index": J.sub(J.filter("length", messages_0), 1)}));
  {
    let _items5 = J.iter(J.slice(messages_0, undefined, undefined, J.neg(1)));
    const _loop5 = J.loop(_items5);
    for (let _i5 = 0; _i5 < _items5.length; _i5++) {
      _loop5.index0 = _i5; const _it5 = _items5[_i5]; let message_14 = _it5;
      let index_15 = index_0;
      let content_16 = content_0;
      index_15 = J.sub(J.sub(J.filter("length", messages_0), 1), J.attr(_loop5, "index0"));
      if (J.t(((v) => J.t(v) ? (J.eq(J.attr(message_14, "role"), "user")) : v)(J.attr(ns_0, "multi_step_tool")))) {
        content_16 = J.filter("trim", J.call(render_content_0, J.attr(message_14, "content")));
        if (J.t(!J.t(((v) => J.t(v) ? J.call(J.attr(content_16, "endswith"), "</tool_response>") : v)(J.call(J.attr(content_16, "startswith"), "<tool_response>"))))) {
          J.setattr(ns_0, "multi_step_tool", false);
          J.setattr(ns_0, "last_query_index", index_15);
        }
      }
    }
  }
  {
    let _items6 = J.iter(messages_0);
    const _loop6 = J.loop(_items6);
    for (let _i6 = 0; _i6 < _items6.length; _i6++) {
      _loop6.index0 = _i6; const _it6 = _items6[_i6]; let message_17 = _it6;
      let content_18 = content_0;
      let reasoning_content_19 = reasoning_content_0;
      let thinking_parts_20 = thinking_parts_0;
      let preserve_reasoning_21 = preserve_reasoning_0;
      content_18 = J.filter("trim", J.call(render_content_0, J.attr(message_17, "content")));
      if (J.t((J.eq(J.attr(message_17, "role"), "system")))) {
        if (J.t(!J.t(J.attr(_loop6, "first")))) {
          out.push(J.str(J.add(J.add("<|im_start|>system\n", content_18), "<|im_end|>\n")));
        }
      } else if (J.t((J.eq(J.attr(message_17, "role"), "user")))) {
        out.push(J.str(J.add(J.add("<|im_start|>user\n", content_18), "<|im_end|>\n")));
      } else if (J.t((J.eq(J.attr(message_17, "role"), "assistant")))) {
        reasoning_content_19 = "";
        if (J.t(J.test("string", J.attr(message_17, "reasoning_content")))) {
          reasoning_content_19 = J.attr(message_17, "reasoning_content");
        } else if (J.t(((v) => J.t(v) ? (J.has("</think>", content_18)) : v)(J.call(J.attr(content_18, "startswith"), "<think>")))) {
          thinking_parts_20 = J.call(J.attr(content_18, "split"), "</think>", 1);
          reasoning_content_19 = J.call(J.attr(J.slice(J.item(thinking_parts_20, 0), 7, undefined, undefined), "strip"), "\n");
          content_18 = J.call(J.attr(J.item(thinking_parts_20, 1), "lstrip"), "\n");
        }
        out.push(J.str("<|im_start|>assistant\n"));
        preserve_reasoning_21 = ((v) => J.t(v) ? v : (J.gt(J.attr(_loop6, "index0"), J.attr(ns_0, "last_query_index"))))(!J.t(truncate_history_thinking_0));
        if (J.t(((v) => J.t(v) ? J.test("false", J.attr(message_17, "loss")) : v)((J.attr(message_17, "loss") !== undefined)))) {
          out.push(J.str(J.call(render_assistant_body_0, message_17, content_18, reasoning_content_19, preserve_reasoning_21)));
        } else {
          {
            out.push(J.str(J.call(render_assistant_body_0, message_17, content_18, reasoning_content_19, preserve_reasoning_21)));
          }
        }
        out.push(J.str("\n"));
      } else if (J.t((J.eq(J.attr(message_17, "role"), "tool")))) {
        if (J.t(((v) => J.t(v) ? (!J.eq(J.attr(J.attr(_loop6, "previtem"), "role"), "tool")) : v)(J.attr(_loop6, "previtem")))) {
          out.push(J.str("<|im_start|>user"));
        }
        out.push(J.str(J.add(J.add("\n<tool_response>\n", content_18), "\n</tool_response>")));
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.attr(_loop6, "nextitem"), "role"), "tool")))(J.attr(_loop6, "last")))) {
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
      out.push(J.str("<think></think>\n\n"));
    } else {
      out.push(J.str("<think>\n"));
    }
  }
  return out.join("");
}
