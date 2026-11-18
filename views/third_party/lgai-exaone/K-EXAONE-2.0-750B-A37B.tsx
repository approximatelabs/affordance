// Converted from the chat template of LGAI-EXAONE/K-EXAONE-2.0-750B-A37B by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let image_count_0 = ctx["image_count"];
  let video_count_0 = ctx["video_count"];
  let role_indicators_0 = ctx["role_indicators"];
  let end_of_turn_0 = ctx["end_of_turn"];
  let declare_available_tools_0 = ctx["declare_available_tools"];
  let render_tool_call_0 = ctx["render_tool_call"];
  let render_tool_response_0 = ctx["render_tool_response"];
  let ns_0 = ctx["ns"];
  let messages_0 = ctx["messages"];
  let index_0 = ctx["index"];
  let tools_0 = ctx["tools"] ?? null;
  let msg_0 = ctx["msg"];
  let role_0 = ctx["role"];
  let content_0 = ctx["content"];
  let reasoning_0 = ctx["reasoning"];
  let _parts_0 = ctx["_parts"];
  let preserve_thinking_0 = ctx["preserve_thinking"];
  let arguments_0 = ctx["arguments"];
  let add_vision_id_0 = ctx["add_vision_id"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let enable_thinking_0 = ctx["enable_thinking"];
  image_count_0 = J.call(J.namespace, J.kw({"value": 0}));
  video_count_0 = J.call(J.namespace, J.kw({"value": 0}));
  role_indicators_0 = ({"user": "<|user|>\n", "assistant": "<|assistant|>\n", "system": "<|system|>\n", "tool": "<|tool|>\n", "tool_declare": "<|tool_declare|>\n"});
  end_of_turn_0 = "<|endofturn|>\n";
  declare_available_tools_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_2 = k.caller;
    let tools_1 = p.length > 0 ? p[0] : "tools" in k ? k["tools"] : undefined;
    out.push(J.str("# Tools\n"), J.str("The available tools are defined below in JSON format.\n"), J.str("When calling a tool, use XML with <function=...> and one <parameter=...> block per argument.\n"), J.str("\n"));
    {
      let _items1 = J.iter(tools_1);
      const _loop1 = J.loop(_items1);
      for (let _i1 = 0; _i1 < _items1.length; _i1++) {
        _loop1.index0 = _i1; const _it1 = _items1[_i1]; let tool_3 = _it1;
        out.push(J.str("<tool>"), J.str(J.filter("safe", J.filter("tojson", tool_3, J.kw({"ensure_ascii": false})))), J.str("</tool>\n"));
      }
    }
    out.push(J.str("\n# Tool Call Format\n"), J.str("<tool_call>\n"), J.str("<function=example_tool_name>\n"), J.str("<parameter=arg1>\n"), J.str("value1\n"), J.str("</parameter>\n"), J.str("<parameter=arg2>\n"), J.str("2\n"), J.str("</parameter>\n"), J.str("</function>\n"), J.str("</tool_call>"));
    return out.join("");
  };
  render_tool_call_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_6 = k.caller;
    let tool_call_4 = p.length > 0 ? p[0] : "tool_call" in k ? k["tool_call"] : undefined;
    let arguments_5 = p.length > 1 ? p[1] : "arguments" in k ? k["arguments"] : undefined;
    out.push(J.str("<tool_call>\n"), J.str("<function="), J.str(J.attr(tool_call_4, "name")), J.str(">\n"));
    {
      let _items2 = J.iter(J.call(J.attr(arguments_5, "items")));
      const _loop2 = J.loop(_items2);
      for (let _i2 = 0; _i2 < _items2.length; _i2++) {
        _loop2.index0 = _i2; const _it2 = _items2[_i2]; let [args_name_7, args_value_8] = J.unpack(_it2, 2);
        out.push(J.str("<parameter="), J.str(args_name_7), J.str(">\n"));
        if (J.t(J.test("string", args_value_8))) {
          out.push(J.str(args_value_8));
        } else {
          out.push(J.str(J.filter("safe", J.filter("tojson", args_value_8, J.kw({"ensure_ascii": false})))));
        }
        out.push(J.str("\n</parameter>\n"));
      }
    }
    out.push(J.str("</function>\n"), J.str("</tool_call>"));
    return out.join("");
  };
  render_tool_response_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_10 = k.caller;
    let msg_9 = p.length > 0 ? p[0] : "msg" in k ? k["msg"] : undefined;
    out.push(J.str("<tool_result>"));
    if (J.t((J.attr(msg_9, "content") !== undefined))) {
      if (J.t(J.test("string", J.attr(msg_9, "content")))) {
        out.push(J.str(J.attr(msg_9, "content")));
      } else {
        out.push(J.str(J.filter("safe", J.filter("tojson", J.attr(msg_9, "content"), J.kw({"ensure_ascii": false})))));
      }
    }
    out.push(J.str("</tool_result>"));
    return out.join("");
  };
  ns_0 = J.call(J.namespace, J.kw({"last_query_index": J.sub(J.filter("length", messages_0), 1), "last_query_index_not_yet_determined": true}));
  {
    let _items3 = J.iter(J.slice(messages_0, undefined, undefined, J.neg(1)));
    const _loop3 = J.loop(_items3);
    for (let _i3 = 0; _i3 < _items3.length; _i3++) {
      _loop3.index0 = _i3; const _it3 = _items3[_i3]; let message_11 = _it3;
      let index_12 = index_0;
      index_12 = J.sub(J.sub(J.filter("length", messages_0), 1), J.attr(_loop3, "index0"));
      if (J.t(((v) => J.t(v) ? J.test("string", J.attr(message_11, "content")) : v)(((v) => J.t(v) ? (J.eq(J.attr(message_11, "role"), "user")) : v)(J.attr(ns_0, "last_query_index_not_yet_determined"))))) {
        J.setattr(ns_0, "last_query_index", index_12);
        J.setattr(ns_0, "last_query_index_not_yet_determined", false);
      }
    }
  }
  if (J.t(((v) => J.t(v) ? tools_0 : v)((tools_0 !== undefined)))) {
    out.push(J.str(J.item(role_indicators_0, "tool_declare")), J.str(J.call(declare_available_tools_0, tools_0)), J.str(end_of_turn_0));
  }
  {
    let _items4 = J.iter(J.call(J.range, J.filter("length", messages_0)));
    const _loop4 = J.loop(_items4);
    for (let _i4 = 0; _i4 < _items4.length; _i4++) {
      _loop4.index0 = _i4; const _it4 = _items4[_i4]; let i_13 = _it4;
      let msg_14 = msg_0;
      let role_15 = role_0;
      let content_16 = content_0;
      let reasoning_17 = reasoning_0;
      let _parts_18 = _parts_0;
      msg_14 = J.item(messages_0, i_13);
      role_15 = J.attr(msg_14, "role");
      if (J.t((!J.has(role_15, role_indicators_0)))) {
        out.push(J.str(J.call(J.raise_exception, J.concat("Unknown role: ", role_15))));
      }
      if (J.t((J.eq(i_13, 0)))) {
        if (J.t((J.eq(role_15, "system")))) {
          out.push(J.str(J.item(role_indicators_0, "system")), J.str(J.attr(msg_14, "content")), J.str(end_of_turn_0));
          continue;
        }
      }
      if (J.t((J.eq(role_15, "assistant")))) {
        out.push(J.str(J.item(role_indicators_0, "assistant")));
        content_16 = (J.t(((v) => J.t(v) ? J.attr(msg_14, "content") : v)((J.attr(msg_14, "content") !== undefined))) ? J.attr(msg_14, "content") : "");
        reasoning_17 = null;
        if (J.t(((v) => J.t(v) ? J.attr(msg_14, "reasoning_content") : v)((J.attr(msg_14, "reasoning_content") !== undefined)))) {
          reasoning_17 = J.call(J.attr(J.attr(msg_14, "reasoning_content"), "strip"));
        } else if (J.t(((v) => J.t(v) ? (J.has("</think>", content_16)) : v)(content_16))) {
          _parts_18 = J.call(J.attr(content_16, "split"), "</think>");
          reasoning_17 = J.call(J.attr(J.call(J.attr(J.item(_parts_18, 0), "lstrip"), "<think>"), "strip"));
          content_16 = J.call(J.attr(J.item(_parts_18, J.neg(1)), "strip"));
        }
        if (J.t(!J.t(((v) => J.t(v) ? ((v) => J.t(v) ? v : (J.gt(i_13, J.attr(ns_0, "last_query_index"))))(((v) => J.t(v) ? preserve_thinking_0 : v)((preserve_thinking_0 !== undefined))) : v)(reasoning_17)))) {
          reasoning_17 = null;
        }
        if (J.t(J.test("string", content_16))) {
          content_16 = J.call(J.attr(content_16, "strip"));
        }
        out.push(J.str("<think>\n"), J.str((J.t(!J.t((reasoning_17 === null))) ? reasoning_17 : "")), J.str("\n</think>\n\n"), J.str(content_16));
        if (J.t(J.attr(msg_14, "tool_calls"))) {
          if (J.t(((v) => J.t(v) ? content_16 : v)((content_16 !== undefined)))) {
            out.push(J.str("\n"));
          }
          {
            let _items5 = J.iter(J.attr(msg_14, "tool_calls"));
            const _loop5 = J.loop(_items5);
            for (let _i5 = 0; _i5 < _items5.length; _i5++) {
              _loop5.index0 = _i5; const _it5 = _items5[_i5]; let tool_call_19 = _it5;
              let arguments_20 = arguments_0;
              if (J.t((J.attr(tool_call_19, "function") !== undefined))) {
                tool_call_19 = J.attr(tool_call_19, "function");
              }
              if (J.t((J.attr(tool_call_19, "arguments") !== undefined))) {
                arguments_20 = J.attr(tool_call_19, "arguments");
              } else if (J.t((J.attr(tool_call_19, "parameters") !== undefined))) {
                arguments_20 = J.attr(tool_call_19, "parameters");
              } else {
                out.push(J.str(J.call(J.raise_exception, J.concat("arguments or parameters are mandatory: ", tool_call_19))));
              }
              out.push(J.str(J.call(render_tool_call_0, tool_call_19, arguments_20)));
              if (J.t(!J.t(J.attr(_loop5, "last")))) {
                out.push(J.str("\n"));
              }
            }
          }
        }
        out.push(J.str(end_of_turn_0));
      } else if (J.t((J.eq(role_15, "tool")))) {
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.sub(i_13, 1)), "role"), "tool")))((J.eq(i_13, 0))))) {
          out.push(J.str(J.item(role_indicators_0, "tool")));
        }
        out.push(J.str(J.call(render_tool_response_0, msg_14)));
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.add(i_13, 1)), "role"), "tool")))(J.attr(_loop4, "last")))) {
          out.push(J.str(end_of_turn_0));
        } else {
          out.push(J.str("\n"));
        }
      } else {
        out.push(J.str(J.item(role_indicators_0, role_15)));
        if (J.t(J.test("string", J.attr(msg_14, "content")))) {
          out.push(J.str(J.attr(msg_14, "content")));
        } else {
          {
            let _items6 = J.iter(J.attr(msg_14, "content"));
            const _loop6 = J.loop(_items6);
            for (let _i6 = 0; _i6 < _items6.length; _i6++) {
              _loop6.index0 = _i6; const _it6 = _items6[_i6]; let content_21 = _it6;
              if (J.t((J.eq(J.attr(content_21, "type"), "image")))) {
                J.setattr(image_count_0, "value", J.add(J.attr(image_count_0, "value"), 1));
                if (J.t(add_vision_id_0)) {
                  out.push("Picture ", J.str(J.attr(image_count_0, "value")), ": ");
                }
                out.push("<vision><image_pad></vision>");
              } else if (J.t((J.eq(J.attr(content_21, "type"), "video")))) {
                J.setattr(video_count_0, "value", J.add(J.attr(video_count_0, "value"), 1));
                if (J.t(add_vision_id_0)) {
                  out.push("Video ", J.str(J.attr(video_count_0, "value")), ": ");
                }
                out.push("<vision><video_pad></vision>");
              } else if (J.t((J.eq(J.attr(content_21, "type"), "text")))) {
                out.push(J.str(J.attr(content_21, "text")));
              } else {
                out.push(J.str(J.attr(content_21, "text")));
              }
            }
          }
        }
        out.push(J.str(end_of_turn_0));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str(J.item(role_indicators_0, "assistant")));
    if (J.t(((v) => J.t(v) ? v : J.test("true", enable_thinking_0))(!J.t((enable_thinking_0 !== undefined))))) {
      out.push(J.str("<think>\n"));
    } else {
      out.push(J.str("<think>\n\n</think>\n\n"));
    }
  }
  return out.join("");
}
