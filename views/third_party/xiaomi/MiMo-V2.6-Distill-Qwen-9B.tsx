// Converted from the chat template of XiaomiMiMo/MiMo-V2.6-Distill-Qwen-9B by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let render_value_0 = ctx["render_value"];
  let render_content_0 = ctx["render_content"];
  let render_tools_0 = ctx["render_tools"];
  let render_tool_calls_0 = ctx["render_tool_calls"];
  let render_assistant_message_0 = ctx["render_assistant_message"];
  let content_0 = ctx["content"];
  let reasoning_0 = ctx["reasoning"];
  let tools_0 = ctx["tools"] ?? null;
  let messages_0 = ctx["messages"];
  let body_0 = ctx["body"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let enable_thinking_0 = ctx["enable_thinking"];
  render_value_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_2 = k.caller;
    let value_1 = p.length > 0 ? p[0] : "value" in k ? k["value"] : undefined;
    if (J.t(J.test("string", value_1))) {
      out.push(J.str(value_1));
    } else {
      out.push(J.str(J.filter("tojson", value_1, J.kw({"ensure_ascii": false}))));
    }
    return out.join("");
  };
  render_content_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_4 = k.caller;
    let message_content_3 = p.length > 0 ? p[0] : "message_content" in k ? k["message_content"] : undefined;
    if (J.t(J.test("string", message_content_3))) {
      out.push(J.str(message_content_3));
    } else if (J.t(J.test("iterable", message_content_3))) {
      {
        let _items1 = J.iter(message_content_3);
        const _loop1 = J.loop(_items1);
        for (let _i1 = 0; _i1 < _items1.length; _i1++) {
          _loop1.index0 = _i1; const _it1 = _items1[_i1]; let part_5 = _it1;
          if (J.t(!J.t(J.test("mapping", part_5)))) {
            out.push(J.str(part_5));
          } else if (J.t(((v) => J.t(v) ? v : (J.has("image_url", part_5)))(((v) => J.t(v) ? v : (J.has("image", part_5)))((J.eq(J.item(part_5, "type"), "image")))))) {
            out.push(J.str("<|vision_start|><|image_pad|><|vision_end|>"));
          } else if (J.t(((v) => J.t(v) ? v : (J.has("input_audio", part_5)))(((v) => J.t(v) ? v : (J.has("audio_url", part_5)))(((v) => J.t(v) ? v : (J.has("audio", part_5)))(((v) => J.t(v) ? v : (J.eq(J.item(part_5, "type"), "input_audio")))((J.eq(J.item(part_5, "type"), "audio")))))))) {
            out.push(J.str("<|mimo_audio_start|><|audio_pad|><|mimo_audio_end|>"));
          } else if (J.t(((v) => J.t(v) ? v : (J.has("video_url", part_5)))(((v) => J.t(v) ? v : (J.has("video", part_5)))((J.eq(J.item(part_5, "type"), "video")))))) {
            out.push(J.str("<|vision_start|><|video_pad|><|vision_end|>"));
          } else if (J.t((J.has("text", part_5)))) {
            out.push(J.str(J.item(part_5, "text")));
          }
        }
      }
    }
    return out.join("");
  };
  render_tools_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_7 = k.caller;
    let tools_6 = p.length > 0 ? p[0] : "tools" in k ? k["tools"] : undefined;
    out.push(J.str("You are provided with the following tools:\n\n<tools>"));
    {
      let _items2 = J.iter(tools_6);
      const _loop2 = J.loop(_items2);
      for (let _i2 = 0; _i2 < _items2.length; _i2++) {
        _loop2.index0 = _i2; const _it2 = _items2[_i2]; let tool_8 = _it2;
        out.push(J.str(J.concat("\n", J.filter("tojson", tool_8, J.kw({"ensure_ascii": false})))));
      }
    }
    out.push(J.str("\n</tools>"));
    return out.join("");
  };
  render_tool_calls_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_10 = k.caller;
    let tool_calls_9 = p.length > 0 ? p[0] : "tool_calls" in k ? k["tool_calls"] : undefined;
    {
      let _items3 = J.iter(tool_calls_9);
      const _loop3 = J.loop(_items3);
      for (let _i3 = 0; _i3 < _items3.length; _i3++) {
        _loop3.index0 = _i3; const _it3 = _items3[_i3]; let tool_call_11 = _it3;
        if (J.t((J.attr(tool_call_11, "function") !== undefined))) {
          tool_call_11 = J.attr(tool_call_11, "function");
        } else if (J.t((J.attr(tool_call_11, "custom") !== undefined))) {
          tool_call_11 = J.attr(tool_call_11, "custom");
        }
        out.push(J.str(J.concat("<tool_call><function=", J.attr(tool_call_11, "name"), ">")));
        if (J.t(((v) => J.t(v) ? J.test("string", J.attr(tool_call_11, "input")) : v)((J.attr(tool_call_11, "input") !== undefined)))) {
          out.push(J.str(J.attr(tool_call_11, "input")));
        } else if (J.t(J.attr(tool_call_11, "arguments"))) {
          if (J.t(J.test("string", J.attr(tool_call_11, "arguments")))) {
            out.push(J.str(J.attr(tool_call_11, "arguments")));
          } else {
            {
              let _items4 = J.iter(J.filter("items", J.attr(tool_call_11, "arguments")));
              const _loop4 = J.loop(_items4);
              for (let _i4 = 0; _i4 < _items4.length; _i4++) {
                _loop4.index0 = _i4; const _it4 = _items4[_i4]; let [args_name_12, args_value_13] = J.unpack(_it4, 2);
                out.push(J.str(J.concat("<parameter=", args_name_12, ">", J.call(render_value_0, args_value_13), "</parameter>")));
              }
            }
          }
        }
        out.push(J.str("</function></tool_call>"));
      }
    }
    return out.join("");
  };
  render_assistant_message_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_15 = k.caller;
    let message_14 = p.length > 0 ? p[0] : "message" in k ? k["message"] : undefined;
    {
      content_0 = J.call(render_content_0, J.attr(message_14, "content"));
      reasoning_0 = (J.t(J.test("string", J.attr(message_14, "reasoning_content"))) ? J.attr(message_14, "reasoning_content") : "");
      out.push(J.str(J.concat("<|im_start|>assistant\n<think>", reasoning_0, "</think>", content_0)));
      if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", J.attr(message_14, "tool_calls")), 0)) : v)(((v) => J.t(v) ? J.test("iterable", J.attr(message_14, "tool_calls")) : v)((J.attr(message_14, "tool_calls") !== undefined))))) {
        out.push(J.str(J.call(render_tool_calls_0, J.attr(message_14, "tool_calls"))));
      }
      out.push(J.str("<|im_end|>"));
    }
    return out.join("");
  };
  if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", tools_0), 0)) : v)(((v) => J.t(v) ? J.test("iterable", tools_0) : v)((tools_0 !== undefined))))) {
    out.push(J.str(J.concat("<|im_start|>system\n", J.call(render_tools_0, tools_0), "<|im_end|>")));
  }
  {
    let _items5 = J.iter(messages_0);
    const _loop5 = J.loop(_items5);
    for (let _i5 = 0; _i5 < _items5.length; _i5++) {
      _loop5.index0 = _i5; const _it5 = _items5[_i5]; let message_16 = _it5;
      let body_17 = body_0;
      if (J.t((J.eq(J.attr(message_16, "role"), "assistant")))) {
        out.push(J.str(J.call(render_assistant_message_0, message_16)));
      } else {
        body_17 = J.call(render_content_0, J.attr(message_16, "content"));
        out.push(J.str(J.concat("<|im_start|>", J.attr(message_16, "role"), "\n", body_17)));
        if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", J.attr(message_16, "tools")), 0)) : v)(((v) => J.t(v) ? J.test("iterable", J.attr(message_16, "tools")) : v)((J.attr(message_16, "tools") !== undefined))))) {
          if (J.t(body_17)) {
            out.push(J.str("\n\n"));
          }
          out.push(J.str(J.call(render_tools_0, J.attr(message_16, "tools"))));
        }
        out.push(J.str("<|im_end|>"));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|im_start|>assistant\n"));
    if (J.t(J.test("false", enable_thinking_0))) {
      out.push(J.str("<think></think>"));
    }
  }
  return out.join("");
}
