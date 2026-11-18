// Converted from the chat template of Qwen/Qwen3-Omni-30B-A3B-Instruct by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let ns_0 = ctx["ns"];
  let tools_0 = ctx["tools"] ?? null;
  let messages_0 = ctx["messages"];
  let index_0 = ctx["index"];
  let content_0 = ctx["content"];
  let reasoning_content_0 = ctx["reasoning_content"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let enable_thinking_0 = ctx["enable_thinking"];
  if (J.t(tools_0)) {
    out.push(J.str("<|im_start|>system\n"));
    if (J.t((J.eq(J.attr(J.item(messages_0, 0), "role"), "system")))) {
      if (J.t(J.test("string", J.attr(J.item(messages_0, 0), "content")))) {
        out.push(J.str(J.attr(J.item(messages_0, 0), "content")));
      } else {
        {
          let _items1 = J.iter(J.attr(J.item(messages_0, 0), "content"));
          const _loop1 = J.loop(_items1);
          for (let _i1 = 0; _i1 < _items1.length; _i1++) {
            _loop1.index0 = _i1; const _it1 = _items1[_i1]; let content_1 = _it1;
            if (J.t(((v) => J.t(v) ? v : (J.has("image_url", content_1)))(((v) => J.t(v) ? v : (J.has("image", content_1)))((J.eq(J.attr(content_1, "type"), "image")))))) {
              out.push(J.str("<|vision_start|><|image_pad|><|vision_end|>"));
            } else if (J.t(((v) => J.t(v) ? v : (J.has("audio_url", content_1)))(((v) => J.t(v) ? v : (J.has("audio", content_1)))((J.eq(J.attr(content_1, "type"), "audio")))))) {
              out.push(J.str("<|audio_start|><|audio_pad|><|audio_end|>"));
            } else if (J.t(((v) => J.t(v) ? v : (J.has("video", content_1)))((J.eq(J.attr(content_1, "type"), "video"))))) {
              out.push(J.str("<|vision_start|><|video_pad|><|vision_end|>"));
            } else if (J.t((J.eq(J.attr(content_1, "type"), "text")))) {
              out.push(J.str(J.attr(content_1, "text")));
            }
          }
        }
      }
    }
    out.push(J.str("\n\n"), J.str("# Tools\n\nYou may call one or more functions to assist with the user query.\n\nYou are provided with function signatures within <tools></tools> XML tags:\n<tools>"));
    {
      let _items2 = J.iter(tools_0);
      const _loop2 = J.loop(_items2);
      for (let _i2 = 0; _i2 < _items2.length; _i2++) {
        _loop2.index0 = _i2; const _it2 = _items2[_i2]; let tool_2 = _it2;
        out.push(J.str("\n"), J.str(J.filter("tojson", tool_2)));
      }
    }
    out.push(J.str("\n</tools>\n\nFor each function call, return a json object with function name and arguments within <tool_call></tool_call> XML tags:\n<tool_call>\n{\"name\": <function-name>, \"arguments\": <args-json-object>}\n</tool_call><|im_end|>\n"));
  } else {
    if (J.t((J.eq(J.attr(J.item(messages_0, 0), "role"), "system")))) {
      if (J.t(J.test("string", J.attr(J.item(messages_0, 0), "content")))) {
        out.push(J.str(J.add(J.add("<|im_start|>system\n", J.attr(J.item(messages_0, 0), "content")), "<|im_end|>\n")));
      } else {
        {
          let _items3 = J.iter(J.attr(J.item(messages_0, 0), "content"));
          const _loop3 = J.loop(_items3);
          for (let _i3 = 0; _i3 < _items3.length; _i3++) {
            _loop3.index0 = _i3; const _it3 = _items3[_i3]; let content_3 = _it3;
            if (J.t(((v) => J.t(v) ? v : (J.has("image_url", content_3)))(((v) => J.t(v) ? v : (J.has("image", content_3)))((J.eq(J.attr(content_3, "type"), "image")))))) {
              out.push(J.str(J.add(J.add("<|im_start|>system\n", "<|vision_start|><|image_pad|><|vision_end|>"), "<|im_end|>\n")));
            } else if (J.t(((v) => J.t(v) ? v : (J.has("audio_url", content_3)))(((v) => J.t(v) ? v : (J.has("audio", content_3)))((J.eq(J.attr(content_3, "type"), "audio")))))) {
              out.push(J.str(J.add(J.add("<|im_start|>system\n", "<|audio_start|><|audio_pad|><|audio_end|>"), "<|im_end|>\n")));
            } else if (J.t(((v) => J.t(v) ? v : (J.has("video", content_3)))((J.eq(J.attr(content_3, "type"), "video"))))) {
              out.push(J.str(J.add(J.add("<|im_start|>system\n", "<|vision_start|><|video_pad|><|vision_end|>"), "<|im_end|>\n")));
            } else if (J.t((J.eq(J.attr(content_3, "type"), "text")))) {
              out.push(J.str(J.add(J.add("<|im_start|>system\n", J.attr(content_3, "text")), "<|im_end|>\n")));
            }
          }
        }
      }
    }
  }
  ns_0 = J.call(J.namespace, J.kw({"multi_step_tool": true, "last_query_index": J.sub(J.filter("length", messages_0), 1)}));
  {
    let _items4 = J.iter(J.slice(messages_0, undefined, undefined, J.neg(1)));
    const _loop4 = J.loop(_items4);
    for (let _i4 = 0; _i4 < _items4.length; _i4++) {
      _loop4.index0 = _i4; const _it4 = _items4[_i4]; let message_4 = _it4;
      let index_5 = index_0;
      index_5 = J.sub(J.sub(J.filter("length", messages_0), 1), J.attr(_loop4, "index0"));
      if (J.t(((v) => J.t(v) ? !J.t(((v) => J.t(v) ? J.call(J.attr(J.attr(message_4, "content"), "endswith"), "</tool_response>") : v)(J.call(J.attr(J.attr(message_4, "content"), "startswith"), "<tool_response>"))) : v)(((v) => J.t(v) ? J.test("string", J.attr(message_4, "content")) : v)(((v) => J.t(v) ? (J.eq(J.attr(message_4, "role"), "user")) : v)(J.attr(ns_0, "multi_step_tool")))))) {
        J.setattr(ns_0, "multi_step_tool", false);
        J.setattr(ns_0, "last_query_index", index_5);
      }
    }
  }
  {
    let _items5 = J.iter(messages_0);
    const _loop5 = J.loop(_items5);
    for (let _i5 = 0; _i5 < _items5.length; _i5++) {
      _loop5.index0 = _i5; const _it5 = _items5[_i5]; let message_6 = _it5;
      let content_7 = content_0;
      let reasoning_content_8 = reasoning_content_0;
      if (J.t(J.test("string", J.attr(message_6, "content")))) {
        content_7 = J.attr(message_6, "content");
      } else {
        content_7 = J.call(J.namespace, J.kw({"text": ""}));
        {
          let _items6 = J.iter(J.attr(message_6, "content"));
          const _loop6 = J.loop(_items6);
          for (let _i6 = 0; _i6 < _items6.length; _i6++) {
            _loop6.index0 = _i6; const _it6 = _items6[_i6]; let mcontent_9 = _it6;
            if (J.t(((v) => J.t(v) ? v : (J.has("image_url", mcontent_9)))(((v) => J.t(v) ? v : (J.has("image", mcontent_9)))((J.eq(J.attr(mcontent_9, "type"), "image")))))) {
              J.setattr(content_7, "text", J.concat(J.attr(content_7, "text"), "<|vision_start|><|image_pad|><|vision_end|>"));
            } else if (J.t(((v) => J.t(v) ? v : (J.has("audio_url", mcontent_9)))(((v) => J.t(v) ? v : (J.has("audio", mcontent_9)))((J.eq(J.attr(mcontent_9, "type"), "audio")))))) {
              J.setattr(content_7, "text", J.concat(J.attr(content_7, "text"), "<|audio_start|><|audio_pad|><|audio_end|>"));
            } else if (J.t(((v) => J.t(v) ? v : (J.has("video", mcontent_9)))((J.eq(J.attr(mcontent_9, "type"), "video"))))) {
              J.setattr(content_7, "text", J.concat(J.attr(content_7, "text"), "<|vision_start|><|video_pad|><|vision_end|>"));
            } else if (J.t((J.eq(J.attr(mcontent_9, "type"), "text")))) {
              J.setattr(content_7, "text", J.concat(J.attr(content_7, "text"), J.attr(mcontent_9, "text")));
            }
          }
        }
        content_7 = J.attr(content_7, "text");
      }
      if (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? !J.t(J.attr(_loop5, "first")) : v)((J.eq(J.attr(message_6, "role"), "system"))))((J.eq(J.attr(message_6, "role"), "user"))))) {
        out.push(J.str(J.add(J.add(J.add(J.add(J.add("<|im_start|>", J.attr(message_6, "role")), "\n"), content_7), "<|im_end|>"), "\n")));
      } else if (J.t((J.eq(J.attr(message_6, "role"), "assistant")))) {
        reasoning_content_8 = "";
        if (J.t(J.test("string", J.attr(message_6, "reasoning_content")))) {
          reasoning_content_8 = J.attr(message_6, "reasoning_content");
        } else {
          if (J.t((J.has("</think>", content_7)))) {
            reasoning_content_8 = J.call(J.attr(J.item(J.call(J.attr(J.call(J.attr(J.item(J.call(J.attr(content_7, "split"), "</think>"), 0), "rstrip"), "\n"), "split"), "<think>"), J.neg(1)), "lstrip"), "\n");
            content_7 = J.call(J.attr(J.item(J.call(J.attr(content_7, "split"), "</think>"), J.neg(1)), "lstrip"), "\n");
          }
        }
        if (J.t((J.gt(J.attr(_loop5, "index0"), J.attr(ns_0, "last_query_index"))))) {
          if (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? reasoning_content_8 : v)(!J.t(J.attr(_loop5, "last"))))(J.attr(_loop5, "last")))) {
            out.push(J.str(J.add(J.add(J.add(J.add(J.add("<|im_start|>", J.attr(message_6, "role")), "\n<think>\n"), J.call(J.attr(reasoning_content_8, "strip"), "\n")), "\n</think>\n\n"), J.call(J.attr(content_7, "lstrip"), "\n"))));
          } else {
            out.push(J.str(J.add(J.add(J.add("<|im_start|>", J.attr(message_6, "role")), "\n"), content_7)));
          }
        } else {
          out.push(J.str(J.add(J.add(J.add("<|im_start|>", J.attr(message_6, "role")), "\n"), content_7)));
        }
        if (J.t(J.attr(message_6, "tool_calls"))) {
          {
            let _items7 = J.iter(J.attr(message_6, "tool_calls"));
            const _loop7 = J.loop(_items7);
            for (let _i7 = 0; _i7 < _items7.length; _i7++) {
              _loop7.index0 = _i7; const _it7 = _items7[_i7]; let tool_call_10 = _it7;
              if (J.t(((v) => J.t(v) ? v : !J.t(J.attr(_loop7, "first")))(((v) => J.t(v) ? content_7 : v)(J.attr(_loop7, "first"))))) {
                out.push(J.str("\n"));
              }
              if (J.t(J.attr(tool_call_10, "function"))) {
                tool_call_10 = J.attr(tool_call_10, "function");
              }
              out.push(J.str("<tool_call>\n{\"name\": \""), J.str(J.attr(tool_call_10, "name")), J.str("\", \"arguments\": "));
              if (J.t(J.test("string", J.attr(tool_call_10, "arguments")))) {
                out.push(J.str(J.attr(tool_call_10, "arguments")));
              } else {
                out.push(J.str(J.filter("tojson", J.attr(tool_call_10, "arguments"))));
              }
              out.push(J.str("}\n</tool_call>"));
            }
          }
        }
        out.push(J.str("<|im_end|>\n"));
      } else if (J.t((J.eq(J.attr(message_6, "role"), "tool")))) {
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.sub(J.attr(_loop5, "index0"), 1)), "role"), "tool")))(J.attr(_loop5, "first")))) {
          out.push(J.str("<|im_start|>user"));
        }
        out.push(J.str("\n<tool_response>\n"), J.str(content_7), J.str("\n</tool_response>"));
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.add(J.attr(_loop5, "index0"), 1)), "role"), "tool")))(J.attr(_loop5, "last")))) {
          out.push(J.str("<|im_end|>\n"));
        }
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|im_start|>assistant\n"));
    if (J.t(((v) => J.t(v) ? J.test("false", enable_thinking_0) : v)((enable_thinking_0 !== undefined)))) {
      out.push(J.str("<think>\n\n</think>\n\n"));
    }
  }
  return out.join("");
}
