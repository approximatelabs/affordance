// Converted from the chat template of LGAI-EXAONE/K-EXAONE-236B-A23B by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let image_count_0 = ctx["image_count"];
  let video_count_0 = ctx["video_count"];
  let role_indicators_0 = ctx["role_indicators"];
  let end_of_turn_0 = ctx["end_of_turn"];
  let declare_available_tools_0 = ctx["declare_available_tools"];
  let ns_0 = ctx["ns"];
  let messages_0 = ctx["messages"];
  let index_0 = ctx["index"];
  let tools_0 = ctx["tools"] ?? null;
  let msg_0 = ctx["msg"];
  let role_0 = ctx["role"];
  let content_0 = ctx["content"];
  let reasoning_0 = ctx["reasoning"];
  let _parts_0 = ctx["_parts"];
  let skip_think_0 = ctx["skip_think"];
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
    out.push(J.str("# Tools"), J.str("\n"));
    {
      let _items1 = J.iter(tools_1);
      const _loop1 = J.loop(_items1);
      for (let _i1 = 0; _i1 < _items1.length; _i1++) {
        _loop1.index0 = _i1; const _it1 = _items1[_i1]; let tool_3 = _it1;
        out.push(J.str("<tool>"), J.str(J.filter("safe", J.filter("tojson", tool_3, J.kw({"ensure_ascii": false})))), J.str("</tool>\n"));
      }
    }
    return out.join("");
  };
  ns_0 = J.call(J.namespace, J.kw({"last_query_index": J.sub(J.filter("length", messages_0), 1), "last_query_index_not_yet_determined": true}));
  {
    let _items2 = J.iter(J.slice(messages_0, undefined, undefined, J.neg(1)));
    const _loop2 = J.loop(_items2);
    for (let _i2 = 0; _i2 < _items2.length; _i2++) {
      _loop2.index0 = _i2; const _it2 = _items2[_i2]; let message_4 = _it2;
      let index_5 = index_0;
      index_5 = J.sub(J.sub(J.filter("length", messages_0), 1), J.attr(_loop2, "index0"));
      if (J.t(((v) => J.t(v) ? J.test("string", J.attr(message_4, "content")) : v)(((v) => J.t(v) ? (J.eq(J.attr(message_4, "role"), "user")) : v)(J.attr(ns_0, "last_query_index_not_yet_determined"))))) {
        J.setattr(ns_0, "last_query_index", index_5);
        J.setattr(ns_0, "last_query_index_not_yet_determined", false);
      }
    }
  }
  if (J.t(((v) => J.t(v) ? tools_0 : v)((tools_0 !== undefined)))) {
    out.push(J.str(J.item(role_indicators_0, "tool_declare")), J.str(J.call(declare_available_tools_0, tools_0)), J.str(end_of_turn_0));
  }
  {
    let _items3 = J.iter(J.call(J.range, J.filter("length", messages_0)));
    const _loop3 = J.loop(_items3);
    for (let _i3 = 0; _i3 < _items3.length; _i3++) {
      _loop3.index0 = _i3; const _it3 = _items3[_i3]; let i_6 = _it3;
      let msg_7 = msg_0;
      let role_8 = role_0;
      let content_9 = content_0;
      let reasoning_10 = reasoning_0;
      let _parts_11 = _parts_0;
      msg_7 = J.item(messages_0, i_6);
      role_8 = J.attr(msg_7, "role");
      if (J.t((!J.has(role_8, role_indicators_0)))) {
        out.push(J.str(J.call(J.raise_exception, J.concat("Unknown role: ", role_8))));
      }
      if (J.t((J.eq(i_6, 0)))) {
        if (J.t((J.eq(role_8, "system")))) {
          out.push(J.str(J.item(role_indicators_0, "system")), J.str(J.attr(msg_7, "content")), J.str(end_of_turn_0));
          continue;
        }
      }
      if (J.t((J.eq(role_8, "assistant")))) {
        out.push(J.str(J.item(role_indicators_0, "assistant")));
        content_9 = (J.t(((v) => J.t(v) ? J.attr(msg_7, "content") : v)((J.attr(msg_7, "content") !== undefined))) ? J.attr(msg_7, "content") : "");
        reasoning_10 = null;
        if (J.t(((v) => J.t(v) ? J.attr(msg_7, "reasoning_content") : v)((J.attr(msg_7, "reasoning_content") !== undefined)))) {
          reasoning_10 = J.call(J.attr(J.attr(msg_7, "reasoning_content"), "strip"));
        } else if (J.t(((v) => J.t(v) ? (J.has("</think>", content_9)) : v)(content_9))) {
          _parts_11 = J.call(J.attr(content_9, "split"), "</think>");
          reasoning_10 = J.call(J.attr(J.call(J.attr(J.item(_parts_11, 0), "lstrip"), "<think>"), "strip"));
          content_9 = J.call(J.attr(J.item(_parts_11, J.neg(1)), "strip"));
        }
        if (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? skip_think_0 : v)((skip_think_0 !== undefined)))(!J.t(((v) => J.t(v) ? (J.gt(i_6, J.attr(ns_0, "last_query_index"))) : v)(reasoning_10))))) {
          reasoning_10 = null;
        }
        content_9 = J.call(J.attr(content_9, "strip"));
        out.push(J.str("<think>\n"), J.str((J.t(!J.t((reasoning_10 === null))) ? reasoning_10 : "")), J.str("\n</think>\n\n"), J.str(content_9));
        if (J.t(J.attr(msg_7, "tool_calls"))) {
          if (J.t(((v) => J.t(v) ? content_9 : v)((content_9 !== undefined)))) {
            out.push(J.str("\n"));
          }
          {
            let _items4 = J.iter(J.attr(msg_7, "tool_calls"));
            const _loop4 = J.loop(_items4);
            for (let _i4 = 0; _i4 < _items4.length; _i4++) {
              _loop4.index0 = _i4; const _it4 = _items4[_i4]; let tool_call_12 = _it4;
              let arguments_13 = arguments_0;
              if (J.t((J.attr(tool_call_12, "function") !== undefined))) {
                tool_call_12 = J.attr(tool_call_12, "function");
              }
              if (J.t((J.attr(tool_call_12, "arguments") !== undefined))) {
                arguments_13 = J.attr(tool_call_12, "arguments");
              } else if (J.t((J.attr(tool_call_12, "parameters") !== undefined))) {
                arguments_13 = J.attr(tool_call_12, "parameters");
              } else {
                out.push(J.str(J.call(J.raise_exception, J.concat("arguments or parameters are mandatory: ", tool_call_12))));
              }
              if (J.t(J.test("string", arguments_13))) {
                out.push(J.str("<tool_call>"), "{\"name\": \"", J.str(J.attr(tool_call_12, "name")), "\", \"arguments\": ", J.str(arguments_13), "}", J.str("</tool_call>"));
              } else {
                out.push(J.str("<tool_call>"), "{\"name\": \"", J.str(J.attr(tool_call_12, "name")), "\", \"arguments\": ", J.str(J.filter("safe", J.filter("tojson", arguments_13, J.kw({"ensure_ascii": false})))), "}", J.str("</tool_call>"));
              }
              if (J.t(!J.t(J.attr(_loop4, "last")))) {
                out.push(J.str("\n"));
              }
            }
          }
        }
        out.push(J.str(end_of_turn_0));
      } else if (J.t((J.eq(role_8, "tool")))) {
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.sub(i_6, 1)), "role"), "tool")))((J.eq(i_6, 0))))) {
          out.push(J.str(J.item(role_indicators_0, "tool")));
        }
        if (J.t((J.attr(msg_7, "content") !== undefined))) {
          if (J.t(J.test("string", J.attr(msg_7, "content")))) {
            out.push(J.str("<tool_result>"), J.str(J.attr(msg_7, "content")), J.str("</tool_result>"));
          } else {
            out.push(J.str("<tool_result>"), J.str(J.filter("safe", J.filter("tojson", J.attr(msg_7, "content"), J.kw({"ensure_ascii": false})))), J.str("</tool_result>"));
          }
        }
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.add(i_6, 1)), "role"), "tool")))(J.attr(_loop3, "last")))) {
          out.push(J.str(end_of_turn_0));
        } else {
          out.push(J.str("\n"));
        }
      } else {
        out.push(J.str(J.item(role_indicators_0, role_8)));
        if (J.t(J.test("string", J.attr(msg_7, "content")))) {
          out.push(J.str(J.attr(msg_7, "content")));
        } else {
          {
            let _items5 = J.iter(J.attr(msg_7, "content"));
            const _loop5 = J.loop(_items5);
            for (let _i5 = 0; _i5 < _items5.length; _i5++) {
              _loop5.index0 = _i5; const _it5 = _items5[_i5]; let content_14 = _it5;
              if (J.t((J.eq(J.attr(content_14, "type"), "image")))) {
                J.setattr(image_count_0, "value", J.add(J.attr(image_count_0, "value"), 1));
                if (J.t(add_vision_id_0)) {
                  out.push("Picture ", J.str(J.attr(image_count_0, "value")), ": ");
                }
                out.push("<vision><image_pad></vision>");
              } else if (J.t((J.eq(J.attr(content_14, "type"), "video")))) {
                J.setattr(video_count_0, "value", J.add(J.attr(video_count_0, "value"), 1));
                if (J.t(add_vision_id_0)) {
                  out.push("Video ", J.str(J.attr(video_count_0, "value")), ": ");
                }
                out.push("<vision><video_pad></vision>");
              } else if (J.t((J.eq(J.attr(content_14, "type"), "text")))) {
                out.push(J.str(J.attr(content_14, "text")));
              } else {
                out.push(J.str(J.attr(content_14, "text")));
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
