// Converted from the chat template of MemTensor/MemT-1.0-Nano by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let image_count_0 = ctx["image_count"];
  let video_count_0 = ctx["video_count"];
  let render_content_0 = ctx["render_content"];
  let content_0 = ctx["content"];
  let ns_0 = ctx["ns"];
  let add_vision_id_0 = ctx["add_vision_id"];
  let messages_0 = ctx["messages"];
  let tools_0 = ctx["tools"] ?? null;
  let index_0 = ctx["index"];
  let reasoning_content_0 = ctx["reasoning_content"];
  let preserve_thinking_0 = ctx["preserve_thinking"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let enable_thinking_0 = ctx["enable_thinking"];
  image_count_0 = J.call(J.namespace, J.kw({"value": 0}));
  video_count_0 = J.call(J.namespace, J.kw({"value": 0}));
  render_content_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_4 = k.caller;
    let content_1 = p.length > 0 ? p[0] : "content" in k ? k["content"] : undefined;
    let do_vision_count_2 = p.length > 1 ? p[1] : "do_vision_count" in k ? k["do_vision_count"] : undefined;
    let is_system_content_3 = p.length > 2 ? p[2] : "is_system_content" in k ? k["is_system_content"] : false;
    if (J.t(J.test("string", content_1))) {
      out.push(J.str(content_1));
    } else if (J.t(((v) => J.t(v) ? !J.t(J.test("mapping", content_1)) : v)(J.test("iterable", content_1)))) {
      {
        let _items1 = J.iter(content_1);
        const _loop1 = J.loop(_items1);
        for (let _i1 = 0; _i1 < _items1.length; _i1++) {
          _loop1.index0 = _i1; const _it1 = _items1[_i1]; let item_5 = _it1;
          if (J.t(((v) => J.t(v) ? v : (J.eq(J.attr(item_5, "type"), "image")))(((v) => J.t(v) ? v : (J.has("image_url", item_5)))((J.has("image", item_5)))))) {
            if (J.t(is_system_content_3)) {
              out.push(J.str(J.call(J.raise_exception, "System message cannot contain images.")));
            }
            if (J.t(do_vision_count_2)) {
              J.setattr(image_count_0, "value", J.add(J.attr(image_count_0, "value"), 1));
            }
            if (J.t(add_vision_id_0)) {
              out.push(J.str(J.concat("Picture ", J.attr(image_count_0, "value"), ": ")));
            }
            out.push(J.str("<|vision_start|><|image_pad|><|vision_end|>"));
          } else if (J.t(((v) => J.t(v) ? v : (J.eq(J.attr(item_5, "type"), "video")))((J.has("video", item_5))))) {
            if (J.t(is_system_content_3)) {
              out.push(J.str(J.call(J.raise_exception, "System message cannot contain videos.")));
            }
            if (J.t(do_vision_count_2)) {
              J.setattr(video_count_0, "value", J.add(J.attr(video_count_0, "value"), 1));
            }
            if (J.t(add_vision_id_0)) {
              out.push(J.str(J.concat("Video ", J.attr(video_count_0, "value"), ": ")));
            }
            out.push(J.str("<|vision_start|><|video_pad|><|vision_end|>"));
          } else if (J.t((J.has("text", item_5)))) {
            out.push(J.str(J.attr(item_5, "text")));
          } else {
            out.push(J.str(J.call(J.raise_exception, "Unexpected item type in content.")));
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
        _loop2.index0 = _i2; const _it2 = _items2[_i2]; let tool_6 = _it2;
        out.push(J.str("\n"), J.str(J.filter("tojson", tool_6)));
      }
    }
    out.push(J.str("\n</tools>"), J.str("\n\nIf you choose to call a function ONLY reply in the following format with NO suffix:\n\n<tool_call>\n<function=example_function_name>\n<parameter=example_parameter_1>\nvalue_1\n</parameter>\n<parameter=example_parameter_2>\nThis is the value for the second parameter\nthat can span\nmultiple lines\n</parameter>\n</function>\n</tool_call>\n\n<IMPORTANT>\nReminder:\n- Function calls MUST follow the specified format: an inner <function=...></function> block must be nested within <tool_call></tool_call> XML tags\n- Required parameters MUST be specified\n- You may provide optional reasoning for your function call in natural language BEFORE the function call, but NOT after\n- If there is no function call available, answer the question like normal with your current knowledge and do not tell the user about function calls\n</IMPORTANT>"));
    if (J.t((J.eq(J.attr(J.item(messages_0, 0), "role"), "system")))) {
      content_0 = J.filter("trim", J.call(render_content_0, J.attr(J.item(messages_0, 0), "content"), false, true));
      if (J.t(content_0)) {
        out.push(J.str(J.add("\n\n", content_0)));
      }
    }
    out.push(J.str("<|im_end|>\n"));
  } else {
    if (J.t((J.eq(J.attr(J.item(messages_0, 0), "role"), "system")))) {
      content_0 = J.filter("trim", J.call(render_content_0, J.attr(J.item(messages_0, 0), "content"), false, true));
      out.push(J.str(J.add(J.add("<|im_start|>system\n", content_0), "<|im_end|>\n")));
    }
  }
  ns_0 = J.call(J.namespace, J.kw({"multi_step_tool": true, "last_query_index": J.sub(J.filter("length", messages_0), 1)}));
  {
    let _items3 = J.iter(J.slice(messages_0, undefined, undefined, J.neg(1)));
    const _loop3 = J.loop(_items3);
    for (let _i3 = 0; _i3 < _items3.length; _i3++) {
      _loop3.index0 = _i3; const _it3 = _items3[_i3]; let message_7 = _it3;
      let index_8 = index_0;
      let content_9 = content_0;
      index_8 = J.sub(J.sub(J.filter("length", messages_0), 1), J.attr(_loop3, "index0"));
      if (J.t(((v) => J.t(v) ? (J.eq(J.attr(message_7, "role"), "user")) : v)(J.attr(ns_0, "multi_step_tool")))) {
        content_9 = J.filter("trim", J.call(render_content_0, J.attr(message_7, "content"), false));
        if (J.t(!J.t(((v) => J.t(v) ? J.call(J.attr(content_9, "endswith"), "</tool_response>") : v)(J.call(J.attr(content_9, "startswith"), "<tool_response>"))))) {
          J.setattr(ns_0, "multi_step_tool", false);
          J.setattr(ns_0, "last_query_index", index_8);
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
      _loop4.index0 = _i4; const _it4 = _items4[_i4]; let message_10 = _it4;
      let content_11 = content_0;
      let reasoning_content_12 = reasoning_content_0;
      content_11 = J.filter("trim", J.call(render_content_0, J.attr(message_10, "content"), true));
      if (J.t((J.eq(J.attr(message_10, "role"), "system")))) {
        if (J.t(!J.t(J.attr(_loop4, "first")))) {
          out.push(J.str(J.call(J.raise_exception, "System message must be at the beginning.")));
        }
      } else if (J.t((J.eq(J.attr(message_10, "role"), "user")))) {
        out.push(J.str(J.add(J.add(J.add(J.add(J.add("<|im_start|>", J.attr(message_10, "role")), "\n"), content_11), "<|im_end|>"), "\n")));
      } else if (J.t((J.eq(J.attr(message_10, "role"), "assistant")))) {
        reasoning_content_12 = "";
        if (J.t(J.test("string", J.attr(message_10, "reasoning_content")))) {
          reasoning_content_12 = J.attr(message_10, "reasoning_content");
        } else {
          if (J.t((J.has("</think>", content_11)))) {
            reasoning_content_12 = J.call(J.attr(J.item(J.call(J.attr(J.call(J.attr(J.item(J.call(J.attr(content_11, "split"), "</think>"), 0), "rstrip"), "\n"), "split"), "<think>"), J.neg(1)), "lstrip"), "\n");
            content_11 = J.call(J.attr(J.item(J.call(J.attr(content_11, "split"), "</think>"), J.neg(1)), "lstrip"), "\n");
          }
        }
        reasoning_content_12 = J.filter("trim", reasoning_content_12);
        if (J.t(((v) => J.t(v) ? (!J.has("</think>", content_11)) : v)(((v) => J.t(v) ? J.call(J.attr(content_11, "startswith"), "<think>") : v)(((v) => J.t(v) ? J.test("true", preserve_thinking_0) : v)((preserve_thinking_0 !== undefined)))))) {
          out.push(J.str(J.add(J.add(J.add("<|im_start|>", J.attr(message_10, "role")), "\n"), content_11)));
        } else if (J.t(((v) => J.t(v) ? v : (J.gt(J.attr(_loop4, "index0"), J.attr(ns_0, "last_query_index"))))(((v) => J.t(v) ? J.test("true", preserve_thinking_0) : v)((preserve_thinking_0 !== undefined))))) {
          out.push(J.str(J.add(J.add(J.add(J.add(J.add("<|im_start|>", J.attr(message_10, "role")), "\n<think>\n"), reasoning_content_12), "\n</think>\n\n"), content_11)));
        } else {
          out.push(J.str(J.add(J.add(J.add("<|im_start|>", J.attr(message_10, "role")), "\n"), content_11)));
        }
        if (J.t(((v) => J.t(v) ? !J.t(J.test("mapping", J.attr(message_10, "tool_calls"))) : v)(((v) => J.t(v) ? J.test("iterable", J.attr(message_10, "tool_calls")) : v)(J.attr(message_10, "tool_calls"))))) {
          {
            let _items5 = J.iter(J.attr(message_10, "tool_calls"));
            const _loop5 = J.loop(_items5);
            for (let _i5 = 0; _i5 < _items5.length; _i5++) {
              _loop5.index0 = _i5; const _it5 = _items5[_i5]; let tool_call_13 = _it5;
              if (J.t((J.attr(tool_call_13, "function") !== undefined))) {
                tool_call_13 = J.attr(tool_call_13, "function");
              }
              if (J.t(J.attr(_loop5, "first"))) {
                if (J.t(J.filter("trim", content_11))) {
                  out.push(J.str(J.add(J.add("\n\n<tool_call>\n<function=", J.attr(tool_call_13, "name")), ">\n")));
                } else {
                  out.push(J.str(J.add(J.add("<tool_call>\n<function=", J.attr(tool_call_13, "name")), ">\n")));
                }
              } else {
                out.push(J.str(J.add(J.add("\n<tool_call>\n<function=", J.attr(tool_call_13, "name")), ">\n")));
              }
              if (J.t((J.attr(tool_call_13, "arguments") !== undefined))) {
                {
                  let _items6 = J.iter(J.filter("items", J.attr(tool_call_13, "arguments")));
                  const _loop6 = J.loop(_items6);
                  for (let _i6 = 0; _i6 < _items6.length; _i6++) {
                    _loop6.index0 = _i6; const _it6 = _items6[_i6]; let [args_name_14, args_value_15] = J.unpack(_it6, 2);
                    out.push(J.str(J.add(J.add("<parameter=", args_name_14), ">\n")));
                    args_value_15 = (J.t(J.test("string", args_value_15)) ? J.filter("string", args_value_15) : J.filter("safe", J.filter("tojson", args_value_15)));
                    out.push(J.str(args_value_15), J.str("\n</parameter>\n"));
                  }
                }
              }
              out.push(J.str("</function>\n</tool_call>"));
            }
          }
        }
        out.push(J.str("<|im_end|>\n"));
      } else if (J.t((J.eq(J.attr(message_10, "role"), "tool")))) {
        if (J.t(((v) => J.t(v) ? (!J.eq(J.attr(J.attr(_loop4, "previtem"), "role"), "tool")) : v)(J.attr(_loop4, "previtem")))) {
          out.push(J.str("<|im_start|>user"));
        }
        out.push(J.str("\n<tool_response>\n"), J.str(content_11), J.str("\n</tool_response>"));
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
