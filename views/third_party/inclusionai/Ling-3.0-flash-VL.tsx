// Converted from the chat template of inclusionAI/Ling-3.0-flash-VL by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let enable_thinking_0 = ctx["enable_thinking"];
  let thinking_instruction_0 = ctx["thinking_instruction"];
  let preserve_thinking_0 = ctx["preserve_thinking"];
  let image_count_0 = ctx["image_count"];
  let video_count_0 = ctx["video_count"];
  let render_content_0 = ctx["render_content"];
  let has_thinking_instruction_0 = ctx["has_thinking_instruction"];
  let system_content_0 = ctx["system_content"];
  let ns_0 = ctx["ns"];
  let item_type_0 = ctx["item_type"];
  let add_vision_id_0 = ctx["add_vision_id"];
  let messages_0 = ctx["messages"];
  let tools_0 = ctx["tools"] ?? null;
  let index_0 = ctx["index"];
  let user_content_0 = ctx["user_content"];
  let content_0 = ctx["content"];
  let reasoning_content_0 = ctx["reasoning_content"];
  let tc_0 = ctx["tc"];
  let _args_0 = ctx["_args"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  enable_thinking_0 = (J.t((enable_thinking_0 !== undefined)) ? enable_thinking_0 : true);
  if (J.t(enable_thinking_0)) {
    thinking_instruction_0 = "detailed thinking on";
  } else {
    thinking_instruction_0 = "detailed thinking off";
  }
  if (J.t(!J.t((preserve_thinking_0 !== undefined)))) {
    preserve_thinking_0 = true;
  }
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
          let item_type_6 = item_type_0;
          item_type_6 = (J.t((J.attr(item_5, "type") !== undefined)) ? J.attr(item_5, "type") : "");
          if (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? ((v) => J.t(v) ? v : (J.has("image_url", item_5)))((J.has("image", item_5))) : v)(!J.t(item_type_6)))((J.eq(item_type_6, "image"))))) {
            if (J.t(do_vision_count_2)) {
              J.setattr(image_count_0, "value", J.add(J.attr(image_count_0, "value"), 1));
            }
            if (J.t(((v) => J.t(v) ? add_vision_id_0 : v)((add_vision_id_0 !== undefined)))) {
              out.push(J.str(J.concat("Picture ", J.attr(image_count_0, "value"), ": ")));
            }
            out.push(J.str("<|vision_start|><|image_pad|><|vision_end|>"));
          } else if (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? ((v) => J.t(v) ? v : (J.has("audio_url", item_5)))((J.has("audio", item_5))) : v)(!J.t(item_type_6)))((J.eq(item_type_6, "audio"))))) {
            out.push(J.str("<|audio_start|><|audio_pad|><|audio_end|>"));
          } else if (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? (J.has("video", item_5)) : v)(!J.t(item_type_6)))((J.eq(item_type_6, "video"))))) {
            if (J.t(do_vision_count_2)) {
              J.setattr(video_count_0, "value", J.add(J.attr(video_count_0, "value"), 1));
            }
            if (J.t(((v) => J.t(v) ? add_vision_id_0 : v)((add_vision_id_0 !== undefined)))) {
              out.push(J.str(J.concat("Video ", J.attr(video_count_0, "value"), ": ")));
            }
            out.push(J.str("<|video_start|><|video_pad|><|video_end|>"));
          } else if (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? (J.has("text", item_5)) : v)(!J.t(item_type_6)))((J.eq(item_type_6, "text"))))) {
            out.push(J.str(J.attr(item_5, "text")));
          } else {
            out.push(J.str(J.call(J.raise_exception, "Unexpected item type in content list: each item must have type image/audio/video/text.")));
          }
        }
      }
    } else if (J.t(((v) => J.t(v) ? v : (content_1 === undefined))((content_1 === null)))) {
      out.push(J.str(""));
    } else {
      out.push(J.str(J.call(J.raise_exception, "Unexpected content type: expected string, list, or none.")));
    }
    return out.join("");
  };
  if (J.t(!J.t(messages_0))) {
    out.push(J.str(J.call(J.raise_exception, "No messages provided.")));
  }
  out.push(J.str("<role>SYSTEM</role>"));
  has_thinking_instruction_0 = false;
  if (J.t((J.eq(J.attr(J.item(messages_0, 0), "role"), "system")))) {
    system_content_0 = J.call(render_content_0, J.attr(J.item(messages_0, 0), "content"), false, true);
    if (J.t(((v) => J.t(v) ? v : (J.has("detailed thinking off", system_content_0)))((J.has("detailed thinking on", system_content_0))))) {
      has_thinking_instruction_0 = true;
    }
    out.push(J.str(system_content_0));
    if (J.t(((v) => J.t(v) ? (!J.eq(J.item(system_content_0, J.neg(1)), "\n")) : v)(((v) => J.t(v) ? system_content_0 : v)(((v) => J.t(v) ? v : !J.t(has_thinking_instruction_0))(tools_0))))) {
      out.push(J.str("\n"));
    }
  }
  if (J.t(tools_0)) {
    out.push(J.str("# Tools\n\nYou may call one or more functions to assist with the user query.\n\nYou are provided with function signatures within <tools></tools> XML tags:\n<tools>"));
    {
      let _items2 = J.iter(tools_0);
      const _loop2 = J.loop(_items2);
      for (let _i2 = 0; _i2 < _items2.length; _i2++) {
        _loop2.index0 = _i2; const _it2 = _items2[_i2]; let tool_7 = _it2;
        out.push(J.str("\n"), J.str(J.filter("tojson", tool_7)));
      }
    }
    out.push(J.str("\n</tools>\n\nIf none of the functions can be used, point it out. If the given question lacks the parameters required by the function, also point it out.\nIf you need to use a function, for each function call, output the function name and arguments within the following XML format:\n<tool_call>{function-name}\n<arg_key>{arg-key-1}</arg_key>\n<arg_value>{arg-value-1}</arg_value>\n<arg_key>{arg-key-2}</arg_key>\n<arg_value>{arg-value-2}</arg_value>\n...\n</tool_call>\n"));
  }
  if (J.t(!J.t(has_thinking_instruction_0))) {
    out.push(J.str(thinking_instruction_0));
  }
  out.push(J.str("<|role_end|>"));
  ns_0 = J.call(J.namespace, J.kw({"multi_step_tool": true, "last_query_index": J.sub(J.filter("length", messages_0), 1)}));
  {
    let _items3 = J.iter(J.slice(messages_0, undefined, undefined, J.neg(1)));
    const _loop3 = J.loop(_items3);
    for (let _i3 = 0; _i3 < _items3.length; _i3++) {
      _loop3.index0 = _i3; const _it3 = _items3[_i3]; let message_8 = _it3;
      let index_9 = index_0;
      let user_content_10 = user_content_0;
      index_9 = J.sub(J.sub(J.filter("length", messages_0), 1), J.attr(_loop3, "index0"));
      if (J.t(((v) => J.t(v) ? (J.eq(J.attr(message_8, "role"), "user")) : v)(J.attr(ns_0, "multi_step_tool")))) {
        user_content_10 = J.call(render_content_0, J.attr(message_8, "content"), false);
        if (J.t(!J.t(((v) => J.t(v) ? J.call(J.attr(user_content_10, "endswith"), "</tool_response>") : v)(J.call(J.attr(user_content_10, "startswith"), "<tool_response>"))))) {
          J.setattr(ns_0, "multi_step_tool", false);
          J.setattr(ns_0, "last_query_index", index_9);
        }
      }
    }
  }
  {
    let _items4 = J.iter(messages_0);
    const _loop4 = J.loop(_items4);
    for (let _i4 = 0; _i4 < _items4.length; _i4++) {
      _loop4.index0 = _i4; const _it4 = _items4[_i4]; let message_11 = _it4;
      let content_12 = content_0;
      let reasoning_content_13 = reasoning_content_0;
      if (J.t((J.eq(J.attr(message_11, "role"), "user")))) {
        content_12 = J.call(render_content_0, J.attr(message_11, "content"), true);
        out.push(J.str(J.add(J.add("<role>HUMAN</role>", content_12), "<|role_end|>")));
      } else if (J.t((J.eq(J.attr(message_11, "role"), "system")))) {
        if (J.t(!J.t(J.attr(_loop4, "first")))) {
          content_12 = J.call(render_content_0, J.attr(message_11, "content"), true, true);
          out.push(J.str(J.add(J.add("<role>SYSTEM</role>", content_12), "<|role_end|>")));
        }
      } else if (J.t((J.eq(J.attr(message_11, "role"), "assistant")))) {
        content_12 = J.call(render_content_0, J.attr(message_11, "content"), true);
        reasoning_content_13 = "";
        if (J.t(((v) => J.t(v) ? (!J.eq(J.attr(message_11, "reasoning_content"), "")) : v)(J.test("string", J.attr(message_11, "reasoning_content"))))) {
          reasoning_content_13 = J.attr(message_11, "reasoning_content");
        } else {
          if (J.t((J.has("</think>", content_12)))) {
            reasoning_content_13 = J.call(J.attr(J.item(J.call(J.attr(J.call(J.attr(J.item(J.call(J.attr(content_12, "split"), "</think>"), 0), "rstrip"), "\n"), "split"), "<think>"), J.neg(1)), "lstrip"), "\n");
            content_12 = J.call(J.attr(J.item(J.call(J.attr(content_12, "split"), "</think>"), J.neg(1)), "lstrip"), "\n");
          }
        }
        if (J.t(((v) => J.t(v) ? v : (J.gt(J.attr(_loop4, "index0"), J.attr(ns_0, "last_query_index"))))(preserve_thinking_0))) {
          if (J.t((!J.eq(reasoning_content_13, "")))) {
            out.push(J.str(J.add(J.add(J.add(J.add("<role>ASSISTANT</role>", "\n<think>"), J.call(J.attr(reasoning_content_13, "strip"), "\n")), "</think>"), J.call(J.attr(content_12, "lstrip"), "\n"))));
          } else {
            out.push(J.str(J.add("<role>ASSISTANT</role>\n<think></think>", content_12)));
          }
        } else {
          out.push(J.str(J.add("<role>ASSISTANT</role>\n<think></think>", content_12)));
        }
        if (J.t(J.attr(message_11, "tool_calls"))) {
          {
            let _items5 = J.iter(J.attr(message_11, "tool_calls"));
            const _loop5 = J.loop(_items5);
            for (let _i5 = 0; _i5 < _items5.length; _i5++) {
              _loop5.index0 = _i5; const _it5 = _items5[_i5]; let tool_call_14 = _it5;
              let tc_15 = tc_0;
              let _args_16 = _args_0;
              if (J.t(((v) => J.t(v) ? v : !J.t(J.attr(_loop5, "first")))(((v) => J.t(v) ? content_12 : v)(J.attr(_loop5, "first"))))) {
                out.push(J.str("\n"));
              }
              tc_15 = tool_call_14;
              if (J.t(J.attr(tool_call_14, "function"))) {
                tc_15 = J.attr(tool_call_14, "function");
              }
              out.push(J.str(J.add("<tool_call>", J.attr(tc_15, "name"))));
              _args_16 = J.attr(tc_15, "arguments");
              {
                let _items6 = J.iter(J.call(J.attr(_args_16, "items")));
                const _loop6 = J.loop(_items6);
                for (let _i6 = 0; _i6 < _items6.length; _i6++) {
                  _loop6.index0 = _i6; const _it6 = _items6[_i6]; let [k_17, v_18] = J.unpack(_it6, 2);
                  out.push(J.str(J.add(J.add("\n<arg_key>", k_17), "</arg_key>")), J.str("\n<arg_value>"));
                  if (J.t(J.test("string", v_18))) {
                    out.push(J.str(v_18));
                  } else {
                    out.push(J.str(J.filter("tojson", v_18, J.kw({"ensure_ascii": false}))));
                  }
                  out.push(J.str("</arg_value>"));
                }
              }
              out.push(J.str("\n</tool_call>"));
            }
          }
        }
        out.push(J.str("<|role_end|>"));
      } else if (J.t((J.eq(J.attr(message_11, "role"), "tool")))) {
        content_12 = J.call(render_content_0, J.attr(message_11, "content"), true);
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.sub(J.attr(_loop4, "index0"), 1)), "role"), "tool")))(J.attr(_loop4, "first")))) {
          out.push(J.str("<role>OBSERVATION</role>"));
        }
        out.push(J.str("\n<tool_response>\n"), J.str(content_12), J.str("\n</tool_response>"));
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.add(J.attr(_loop4, "index0"), 1)), "role"), "tool")))(J.attr(_loop4, "last")))) {
          out.push(J.str("<|role_end|>"));
        }
      } else {
        content_12 = J.call(render_content_0, J.attr(message_11, "content"), true);
        out.push(J.str(J.add(J.add(J.add(J.add("<role>", J.attr(message_11, "role")), "</role>"), content_12), "<|role_end|>")));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<role>ASSISTANT</role>"));
    if (J.t(enable_thinking_0)) {
      out.push(J.str("\n<think>"));
    } else {
      out.push(J.str("\n<think></think>"));
    }
  }
  return out.join("");
}
