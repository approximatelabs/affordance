// Converted from the chat template of dots-studio/dots3-note-prev by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let enable_thinking_0 = ctx["enable_thinking"];
  let clear_thinking_0 = ctx["clear_thinking"];
  let image_count_0 = ctx["image_count"];
  let audio_count_0 = ctx["audio_count"];
  let video_count_0 = ctx["video_count"];
  let last_user_index_0 = ctx["last_user_index"];
  let render_content_0 = ctx["render_content"];
  let add_vision_id_0 = ctx["add_vision_id"];
  let tools_0 = ctx["tools"] ?? null;
  let messages_0 = ctx["messages"];
  let content_0 = ctx["content"];
  let reasoning_content_0 = ctx["reasoning_content"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  if (J.t(!J.t((enable_thinking_0 !== undefined)))) {
    enable_thinking_0 = true;
  }
  if (J.t(!J.t((clear_thinking_0 !== undefined)))) {
    clear_thinking_0 = false;
  }
  image_count_0 = J.call(J.namespace, J.kw({"value": 0}));
  audio_count_0 = J.call(J.namespace, J.kw({"value": 0}));
  video_count_0 = J.call(J.namespace, J.kw({"value": 0}));
  last_user_index_0 = J.call(J.namespace, J.kw({"value": J.neg(1)}));
  render_content_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_2 = k.caller;
    let content_1 = p.length > 0 ? p[0] : "content" in k ? k["content"] : undefined;
    if (J.t(!J.t(content_1))) {
      out.push(J.str(""));
    } else if (J.t(J.test("string", content_1))) {
      out.push(J.str(content_1));
    } else {
      {
        let _items1 = J.iter(content_1);
        const _loop1 = J.loop(_items1);
        for (let _i1 = 0; _i1 < _items1.length; _i1++) {
          _loop1.index0 = _i1; const _it1 = _items1[_i1]; let item_3 = _it1;
          if (J.t(((v) => J.t(v) ? v : (J.has("image_url", item_3)))(((v) => J.t(v) ? v : (J.has("image", item_3)))((J.eq(J.item(item_3, "type"), "image")))))) {
            J.setattr(image_count_0, "value", J.add(J.attr(image_count_0, "value"), 1));
            if (J.t(!J.t(J.attr(_loop1, "first")))) {
              out.push(J.str("\n"));
            }
            if (J.t(add_vision_id_0)) {
              out.push("Picture ", J.str(J.attr(image_count_0, "value")), ": ");
            }
            out.push("<|img|><|imgpad|><|endofimg|>");
          } else if (J.t(((v) => J.t(v) ? v : (J.has("video_url", item_3)))(((v) => J.t(v) ? v : (J.has("video", item_3)))((J.eq(J.item(item_3, "type"), "video")))))) {
            J.setattr(video_count_0, "value", J.add(J.attr(video_count_0, "value"), 1));
            if (J.t(!J.t(J.attr(_loop1, "first")))) {
              out.push(J.str("\n"));
            }
            if (J.t(add_vision_id_0)) {
              out.push("Video ", J.str(J.attr(video_count_0, "value")), ": ");
            }
            out.push("<|video_pad|>");
          } else if (J.t(((v) => J.t(v) ? v : (J.has("audio_url", item_3)))(((v) => J.t(v) ? v : (J.has("audio", item_3)))((J.eq(J.item(item_3, "type"), "audio")))))) {
            J.setattr(audio_count_0, "value", J.add(J.attr(audio_count_0, "value"), 1));
            if (J.t(!J.t(J.attr(_loop1, "first")))) {
              out.push(J.str("\n"));
            }
            if (J.t(add_vision_id_0)) {
              out.push("Audio ", J.str(J.attr(audio_count_0, "value")), ": ");
            }
            out.push("<|audio_comp_start|><|audio_comp_pad|><|audio_comp_end|>");
          } else if (J.t((J.has("text", item_3)))) {
            out.push(J.str(J.item(item_3, "text")));
          }
        }
      }
    }
    return out.join("");
  };
  if (J.t(tools_0)) {
    out.push(J.str("<|system|>"));
    if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
      out.push(J.str(J.call(render_content_0, J.item(J.item(messages_0, 0), "content"))));
    } else {
      out.push(J.str("You are a helpful assistant."));
    }
    out.push(J.str("\n\n# Tools\n\nYou may call one or more functions to assist with the user query.\n\nYou are provided with function signatures within <tools></tools> XML tags:\n<tools>"));
    {
      let _items2 = J.iter(tools_0);
      const _loop2 = J.loop(_items2);
      for (let _i2 = 0; _i2 < _items2.length; _i2++) {
        _loop2.index0 = _i2; const _it2 = _items2[_i2]; let tool_4 = _it2;
        out.push(J.str("\n"), J.str(J.filter("tojson", tool_4)));
      }
    }
    out.push(J.str("\n</tools>\n\nWhen making tool calls, use XML format to invoke tools and pass parameters:\n\n<dots_function_call>\n<invoke name=\"tool-name-1\">\n<parameter name=\"param-key-1\">\nparam-value-1\n</parameter>\n<parameter name=\"param-key-2\">\nparam-value-2\n</parameter>\n...\n</invoke>\n</dots_function_call><|endofsystem|>"));
  } else {
    if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
      out.push(J.str("<|system|>"), J.str(J.call(render_content_0, J.item(J.item(messages_0, 0), "content"))), J.str("<|endofsystem|>"));
    } else {
      out.push(J.str("<|system|>You are a helpful assistant.<|endofsystem|>"));
    }
  }
  {
    let _items3 = J.iter(messages_0);
    const _loop3 = J.loop(_items3);
    for (let _i3 = 0; _i3 < _items3.length; _i3++) {
      _loop3.index0 = _i3; const _it3 = _items3[_i3]; let m_5 = _it3;
      if (J.t((J.eq(J.item(m_5, "role"), "user")))) {
        J.setattr(last_user_index_0, "value", J.attr(_loop3, "index0"));
      }
    }
  }
  {
    let _items4 = J.iter(messages_0);
    const _loop4 = J.loop(_items4);
    for (let _i4 = 0; _i4 < _items4.length; _i4++) {
      _loop4.index0 = _i4; const _it4 = _items4[_i4]; let message_6 = _it4;
      let content_7 = content_0;
      let reasoning_content_8 = reasoning_content_0;
      content_7 = J.call(render_content_0, J.item(message_6, "content"));
      if (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? !J.t(J.attr(_loop4, "first")) : v)((J.eq(J.item(message_6, "role"), "system"))))((J.eq(J.item(message_6, "role"), "user"))))) {
        out.push(J.str("<|user|>"), J.str(content_7));
        if (J.t(((v) => J.t(v) ? !J.t(J.call(J.attr(content_7, "endswith"), "<no_think>")) : v)(((v) => J.t(v) ? J.test("false", enable_thinking_0) : v)((J.eq(J.item(message_6, "role"), "user")))))) {
          out.push(J.str("<no_think>"));
        }
        out.push(J.str("<|endofuser|>"));
      } else if (J.t((J.eq(J.item(message_6, "role"), "assistant")))) {
        reasoning_content_8 = "";
        if (J.t(J.test("string", J.attr(message_6, "reasoning_content")))) {
          reasoning_content_8 = J.attr(message_6, "reasoning_content");
        } else {
          if (J.t((J.has("</think>", content_7)))) {
            reasoning_content_8 = J.call(J.attr(J.item(J.call(J.attr(J.call(J.attr(J.item(J.call(J.attr(content_7, "split"), "</think>"), 0), "rstrip"), "\n"), "split"), "<think>"), J.neg(1)), "lstrip"), "\n");
            content_7 = J.call(J.attr(J.item(J.call(J.attr(content_7, "split"), "</think>"), J.neg(1)), "lstrip"), "\n");
          }
        }
        reasoning_content_8 = J.filter("trim", reasoning_content_8);
        out.push(J.str("<|assistant|>"));
        if (J.t(J.test("false", enable_thinking_0))) {
          out.push(J.str(J.add("<think>\n\n</think>\n\n", content_7)));
        } else if (J.t(((v) => J.t(v) ? (J.lt(J.attr(_loop4, "index0"), J.attr(last_user_index_0, "value"))) : v)(clear_thinking_0))) {
          out.push(J.str(J.add("<think>\n\n</think>\n\n", content_7)));
        } else if (J.t(reasoning_content_8)) {
          out.push(J.str(J.add(J.add(J.add("<think>\n", reasoning_content_8), "\n</think>\n\n"), content_7)));
        } else {
          out.push(J.str(content_7));
        }
        if (J.t(J.call(J.attr(message_6, "get"), "tool_calls"))) {
          {
            let _items5 = J.iter(J.item(message_6, "tool_calls"));
            const _loop5 = J.loop(_items5);
            for (let _i5 = 0; _i5 < _items5.length; _i5++) {
              _loop5.index0 = _i5; const _it5 = _items5[_i5]; let tool_call_9 = _it5;
              if (J.t((J.attr(tool_call_9, "function") !== undefined))) {
                tool_call_9 = J.attr(tool_call_9, "function");
              }
              out.push(J.str("\n<dots_function_call>\n<invoke name=\""), J.str(J.attr(tool_call_9, "name")), J.str("\">"));
              {
                let _items6 = J.iter(J.call(J.attr(J.attr(tool_call_9, "arguments"), "items")));
                const _loop6 = J.loop(_items6);
                for (let _i6 = 0; _i6 < _items6.length; _i6++) {
                  _loop6.index0 = _i6; const _it6 = _items6[_i6]; let [arg_name_10, arg_value_11] = J.unpack(_it6, 2);
                  out.push(J.str("\n<parameter name=\""), J.str(arg_name_10), J.str("\">"), J.str("\n"));
                  if (J.t(J.test("string", arg_value_11))) {
                    out.push(J.str(arg_value_11));
                  } else {
                    out.push(J.str(J.filter("tojson", arg_value_11)));
                  }
                  out.push(J.str("\n"), J.str("</parameter>"));
                }
              }
              out.push(J.str("\n</invoke>\n</dots_function_call>"));
            }
          }
        }
        out.push(J.str("<|endofassistant|>"));
      } else if (J.t((J.eq(J.item(message_6, "role"), "tool")))) {
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.item(J.item(messages_0, J.sub(J.attr(_loop4, "index0"), 1)), "role"), "tool")))((J.eq(J.attr(_loop4, "index0"), 0))))) {
          out.push(J.str("<|user|>"));
        }
        out.push(J.str("\n<dots_function_response>\n"), J.str(content_7), J.str("\n</dots_function_response>"));
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.item(J.item(messages_0, J.add(J.attr(_loop4, "index0"), 1)), "role"), "tool")))(J.attr(_loop4, "last")))) {
          out.push(J.str("<|endofuser|>"));
        }
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|assistant|>"));
    if (J.t(J.test("false", enable_thinking_0))) {
      out.push(J.str("<think>\n\n</think>\n\n"));
    }
  }
  return out.join("");
}
