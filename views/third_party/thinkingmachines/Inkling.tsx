// Converted from the chat template of thinkingmachines/Inkling by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let effort_map_0 = ctx["effort_map"];
  let role_token_0 = ctx["role_token"];
  let emit_thinking_effort_0 = ctx["emit_thinking_effort"];
  let tool_state_0 = ctx["tool_state"];
  let state_0 = ctx["state"];
  let eff_0 = ctx["eff"];
  let key_0 = ctx["key"];
  let num_0 = ctx["num"];
  let reasoning_effort_0 = ctx["reasoning_effort"];
  let tools_0 = ctx["tools"] ?? null;
  let fn_0 = ctx["fn"];
  let spec_0 = ctx["spec"];
  let messages_0 = ctx["messages"];
  let rtok_0 = ctx["rtok"];
  let tool_name_state_0 = ctx["tool_name_state"];
  let text_part_0 = ctx["text_part"];
  let args_0 = ctx["args"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  effort_map_0 = ({"none": 0.0, "minimal": 0.1, "low": 0.2, "medium": 0.7, "high": 0.9, "max": 0.99});
  role_token_0 = ({"user": "<|message_user|>", "assistant": "<|message_model|>", "system": "<|message_system|>", "tool": "<|message_tool|>"});
  emit_thinking_effort_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_1 = k.caller;
    let eff_2 = eff_0;
    let key_3 = key_0;
    let num_4 = num_0;
    eff_2 = (J.t(((v) => J.t(v) ? !J.t((reasoning_effort_0 === null)) : v)((reasoning_effort_0 !== undefined))) ? reasoning_effort_0 : 0.9);
    if (J.t(J.test("string", eff_2))) {
      key_3 = J.filter("trim", eff_2);
      if (J.t((!J.has(key_3, effort_map_0)))) {
        out.push(J.str(J.call(J.raise_exception, J.concat("Unknown reasoning_effort: ", eff_2))));
      }
      num_4 = J.item(effort_map_0, key_3);
    } else {
      num_4 = J.filter("float", eff_2);
    }
    if (J.t(((v) => J.t(v) ? v : (J.gt(num_4, 0.99)))((J.lt(num_4, 0.0))))) {
      out.push(J.str(J.call(J.raise_exception, "reasoning_effort must be in [0.0, 0.99]")));
    }
    out.push(J.str("<|message_system|><|content_text|>Thinking effort level: "));
    if (J.t((J.eq(num_4, 0.0)))) {
      out.push("0");
    } else {
      out.push(J.str(num_4));
    }
    out.push(J.str("<|end_message|>"));
    return out.join("");
  };
  if (J.t(tools_0)) {
    tool_state_0 = J.call(J.namespace, J.kw({"specs": []}));
    {
      let _items1 = J.iter(tools_0);
      const _loop1 = J.loop(_items1);
      for (let _i1 = 0; _i1 < _items1.length; _i1++) {
        _loop1.index0 = _i1; const _it1 = _items1[_i1]; let tool_5 = _it1;
        let fn_6 = fn_0;
        let spec_7 = spec_0;
        fn_6 = (J.t((J.attr(tool_5, "function") !== undefined)) ? J.attr(tool_5, "function") : tool_5);
        spec_7 = ({"description": (J.t(((v) => J.t(v) ? J.attr(fn_6, "description") : v)((J.attr(fn_6, "description") !== undefined))) ? J.attr(fn_6, "description") : ""), "name": J.attr(fn_6, "name"), "parameters": (J.t(((v) => J.t(v) ? J.attr(fn_6, "parameters") : v)((J.attr(fn_6, "parameters") !== undefined))) ? J.attr(fn_6, "parameters") : ({})), "type": (J.t(((v) => J.t(v) ? J.attr(tool_5, "type") : v)((J.attr(tool_5, "type") !== undefined))) ? J.attr(tool_5, "type") : "function")});
        J.setattr(tool_state_0, "specs", J.add(J.attr(tool_state_0, "specs"), [spec_7]));
      }
    }
    out.push(J.str("<|message_system|>tool_declare<|content_xml|>"), J.str(J.filter("tojson", J.attr(tool_state_0, "specs"), J.kw({"sort_keys": true, "separators": [",", ":"]}))), J.str("<|end_message|>"));
  }
  state_0 = J.call(J.namespace, J.kw({"effort_emitted": false}));
  {
    let _items2 = J.iter(messages_0);
    const _loop2 = J.loop(_items2);
    for (let _i2 = 0; _i2 < _items2.length; _i2++) {
      _loop2.index0 = _i2; const _it2 = _items2[_i2]; let message_8 = _it2;
      let rtok_9 = rtok_0;
      let tool_name_state_10 = tool_name_state_0;
      if (J.t((!J.has(J.attr(message_8, "role"), role_token_0)))) {
        out.push(J.str(J.call(J.raise_exception, J.concat("Unknown message role: ", J.attr(message_8, "role")))));
      }
      if (J.t(((v) => J.t(v) ? (!J.eq(J.attr(message_8, "role"), "system")) : v)(!J.t(J.attr(state_0, "effort_emitted"))))) {
        out.push(J.str(J.call(emit_thinking_effort_0)));
        J.setattr(state_0, "effort_emitted", true);
      }
      rtok_9 = J.item(role_token_0, J.attr(message_8, "role"));
      if (J.t((J.eq(J.attr(message_8, "role"), "tool")))) {
        tool_name_state_10 = J.call(J.namespace, J.kw({"name": ""}));
        if (J.t(((v) => J.t(v) ? J.attr(message_8, "name") : v)((J.attr(message_8, "name") !== undefined)))) {
          J.setattr(tool_name_state_10, "name", J.attr(message_8, "name"));
        } else if (J.t(((v) => J.t(v) ? J.attr(message_8, "tool_call_id") : v)((J.attr(message_8, "tool_call_id") !== undefined)))) {
          {
            let _items3 = J.iter(messages_0);
            const _loop3 = J.loop(_items3);
            for (let _i3 = 0; _i3 < _items3.length; _i3++) {
              _loop3.index0 = _i3; const _it3 = _items3[_i3]; let prev_11 = _it3;
              if (J.t(((v) => J.t(v) ? J.attr(prev_11, "tool_calls") : v)((J.eq(J.attr(prev_11, "role"), "assistant"))))) {
                {
                  let _items4 = J.iter(J.attr(prev_11, "tool_calls"));
                  const _loop4 = J.loop(_items4);
                  for (let _i4 = 0; _i4 < _items4.length; _i4++) {
                    _loop4.index0 = _i4; const _it4 = _items4[_i4]; let tc_12 = _it4;
                    if (J.t(((v) => J.t(v) ? (J.attr(J.attr(tc_12, "function"), "name") !== undefined) : v)(((v) => J.t(v) ? (J.eq(J.attr(tc_12, "id"), J.attr(message_8, "tool_call_id"))) : v)((J.attr(tc_12, "id") !== undefined))))) {
                      J.setattr(tool_name_state_10, "name", J.attr(J.attr(tc_12, "function"), "name"));
                    }
                  }
                }
              }
            }
          }
        }
        out.push(J.str(rtok_9));
        if (J.t(J.attr(tool_name_state_10, "name"))) {
          out.push(J.str(J.attr(tool_name_state_10, "name")));
        }
        out.push(J.str("<|content_text|>"));
        if (J.t(J.test("string", J.attr(message_8, "content")))) {
          out.push(J.str(J.attr(message_8, "content")));
        }
        out.push(J.str("<|end_message|>"));
      } else {
        if (J.t(((v) => J.t(v) ? J.attr(message_8, "reasoning_content") : v)(((v) => J.t(v) ? (J.attr(message_8, "reasoning_content") !== undefined) : v)((J.eq(J.attr(message_8, "role"), "assistant")))))) {
          out.push(J.str(J.concat("<|message_model|><|content_thinking|>", J.attr(message_8, "reasoning_content"), "<|end_message|>")));
        }
        if (J.t(J.test("string", J.attr(message_8, "content")))) {
          out.push(J.str(J.concat(rtok_9, "<|content_text|>", J.attr(message_8, "content"), "<|end_message|>")));
        } else if (J.t(J.attr(message_8, "content"))) {
          {
            let _items5 = J.iter(J.attr(message_8, "content"));
            const _loop5 = J.loop(_items5);
            for (let _i5 = 0; _i5 < _items5.length; _i5++) {
              _loop5.index0 = _i5; const _it5 = _items5[_i5]; let part_13 = _it5;
              let text_part_14 = text_part_0;
              if (J.t(J.test("string", part_13))) {
                out.push(J.str(J.concat(rtok_9, "<|content_text|>", part_13, "<|end_message|>")));
              } else if (J.t(((v) => J.t(v) ? v : (J.has(J.attr(part_13, "type"), ["text", "input_text"])))(!J.t((J.attr(part_13, "type") !== undefined))))) {
                text_part_14 = (J.t(((v) => J.t(v) ? J.test("string", J.attr(part_13, "text")) : v)((J.attr(part_13, "text") !== undefined))) ? J.attr(part_13, "text") : "");
                out.push(J.str(J.concat(rtok_9, "<|content_text|>", text_part_14, "<|end_message|>")));
              } else if (J.t((J.has(J.attr(part_13, "type"), ["image", "input_image", "image_url"])))) {
                out.push(J.str(J.concat(rtok_9, "<|content_image|><|unused_200054|><|end_message|>")));
              } else if (J.t((J.has(J.attr(part_13, "type"), ["audio", "input_audio", "audio_url"])))) {
                out.push(J.str(J.concat(rtok_9, "<|content_audio_input|><|unused_200053|><|audio_end|><|end_message|>")));
              } else {
                out.push(J.str(J.call(J.raise_exception, J.concat("Unsupported content part type: ", J.attr(part_13, "type")))));
              }
            }
          }
        }
        if (J.t(((v) => J.t(v) ? J.attr(message_8, "tool_calls") : v)((J.eq(J.attr(message_8, "role"), "assistant"))))) {
          {
            let _items6 = J.iter(J.attr(message_8, "tool_calls"));
            const _loop6 = J.loop(_items6);
            for (let _i6 = 0; _i6 < _items6.length; _i6++) {
              _loop6.index0 = _i6; const _it6 = _items6[_i6]; let tc_15 = _it6;
              let fn_16 = fn_0;
              let args_17 = args_0;
              fn_16 = J.attr(tc_15, "function");
              if (J.t(((v) => J.t(v) ? v : !J.t(J.test("string", J.attr(fn_16, "name"))))(!J.t((J.attr(fn_16, "name") !== undefined))))) {
                out.push(J.str(J.call(J.raise_exception, "tool call function name must be a string")));
              }
              args_17 = (J.t(((v) => J.t(v) ? J.attr(fn_16, "arguments") : v)((J.attr(fn_16, "arguments") !== undefined))) ? J.attr(fn_16, "arguments") : ({}));
              if (J.t(J.test("string", args_17))) {
                out.push(J.str(J.call(J.raise_exception, "tool call arguments must be a parsed object, not a JSON string; canonicalize upstream")));
              }
              if (J.t(!J.t(J.test("mapping", args_17)))) {
                out.push(J.str(J.call(J.raise_exception, "tool call arguments must be an object")));
              }
              out.push(J.str(J.concat("<|message_model|>", J.attr(fn_16, "name"), "<|content_invoke_tool_json|>")), J.str(J.concat("{\"name\":", J.filter("tojson", J.attr(fn_16, "name"), J.kw({"sort_keys": true, "separators": [",", ":"]})), ",\"args\":")), J.str(J.filter("tojson", args_17, J.kw({"sort_keys": true, "separators": [",", ":"]}))), J.str("}<|end_message|>"));
            }
          }
        }
        if (J.t((J.eq(J.attr(message_8, "role"), "assistant")))) {
          out.push(J.str("<|content_model_end_sampling|>"));
        }
      }
    }
  }
  if (J.t(!J.t(J.attr(state_0, "effort_emitted")))) {
    out.push(J.str(J.call(emit_thinking_effort_0)));
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|message_model|>"));
  }
  return out.join("");
}
