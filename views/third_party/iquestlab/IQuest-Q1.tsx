// Converted from the chat template of IQuestLab/IQuest-Q1 by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let tool_to_json_0 = ctx["tool_to_json"];
  let render_extra_keys_0 = ctx["render_extra_keys"];
  let visible_text_0 = ctx["visible_text"];
  let ns_0 = ctx["ns"];
  let tools_0 = ctx["tools"] ?? null;
  let ns_tool_0 = ctx["ns_tool"];
  let _req_0 = ctx["_req"];
  let _ns_0 = ctx["_ns"];
  let handled_keys_0 = ctx["handled_keys"];
  let media_type_0 = ctx["media_type"];
  let messages_0 = ctx["messages"];
  let content_0 = ctx["content"];
  let reasoning_content_0 = ctx["reasoning_content"];
  let clear_thinking_0 = ctx["clear_thinking"];
  let _args_0 = ctx["_args"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let enable_thinking_0 = ctx["enable_thinking"];
  if (J.t(tools_0)) {
    tool_to_json_0 = (...args) => {
      const [p, k] = J.args(args);
      const out = [];
      const caller_2 = k.caller;
      let tool_1 = p.length > 0 ? p[0] : "tool" in k ? k["tool"] : undefined;
      let ns_tool_3 = ns_tool_0;
      ns_tool_3 = J.call(J.namespace, J.kw({"first": true}));
      out.push(J.str("{"));
      {
        let _items1 = J.iter(J.call(J.attr(tool_1, "items")));
        const _loop1 = J.loop(_items1);
        for (let _i1 = 0; _i1 < _items1.length; _i1++) {
          _loop1.index0 = _i1; const _it1 = _items1[_i1]; let [k_4, v_5] = J.unpack(_it1, 2);
          if (J.t(((v) => J.t(v) ? (!J.eq(k_4, "strict")) : v)((!J.eq(k_4, "defer_loading"))))) {
            if (J.t(!J.t(J.attr(ns_tool_3, "first")))) {
              out.push(J.str(", "));
            }
            J.setattr(ns_tool_3, "first", false);
            out.push("\"", J.str(k_4), "\": ", J.str(J.filter("tojson", v_5, J.kw({"ensure_ascii": false}))));
          }
        }
      }
      out.push(J.str("}"));
      return out.join("");
    };
    render_extra_keys_0 = (...args) => {
      const [p, k] = J.args(args);
      const out = [];
      const caller_8 = k.caller;
      let json_dict_6 = p.length > 0 ? p[0] : "json_dict" in k ? k["json_dict"] : undefined;
      let handled_keys_7 = p.length > 1 ? p[1] : "handled_keys" in k ? k["handled_keys"] : undefined;
      if (J.t(J.test("mapping", json_dict_6))) {
        {
          let _items2 = J.iter(json_dict_6);
          _items2 = _items2.filter(_it2 => { let json_key_9 = _it2; return J.t((!J.has(json_key_9, handled_keys_7))); });
          const _loop2 = J.loop(_items2);
          for (let _i2 = 0; _i2 < _items2.length; _i2++) {
            _loop2.index0 = _i2; const _it2 = _items2[_i2]; let json_key_9 = _it2;
            if (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? !J.t(J.test("string", J.item(json_dict_6, json_key_9))) : v)(J.test("sequence", J.item(json_dict_6, json_key_9))))(J.test("mapping", J.item(json_dict_6, json_key_9))))) {
              out.push(J.str(J.concat("\n<", json_key_9, ">", J.filter("safe", J.filter("tojson", J.item(json_dict_6, json_key_9))), "</", json_key_9, ">")));
            } else {
              out.push(J.str(J.concat("\n<", json_key_9, ">", J.filter("string", J.item(json_dict_6, json_key_9)), "</", json_key_9, ">")));
            }
          }
        }
      }
      return out.join("");
    };
    out.push(J.str("<|iquest_system|>\n# Tools\n\nYou have access to the following functions:\n\n<tools>"));
    {
      let _items3 = J.iter(tools_0);
      const _loop3 = J.loop(_items3);
      for (let _i3 = 0; _i3 < _items3.length; _i3++) {
        _loop3.index0 = _i3; const _it3 = _items3[_i3]; let tool_10 = _it3;
        let _req_11 = _req_0;
        let _ns_12 = _ns_0;
        let handled_keys_13 = handled_keys_0;
        if (J.t((J.attr(tool_10, "function") !== undefined))) {
          tool_10 = J.attr(tool_10, "function");
        }
        _req_11 = (J.t(((v) => J.t(v) ? !J.t(J.test("string", J.attr(J.attr(tool_10, "parameters"), "required"))) : v)(((v) => J.t(v) ? J.test("iterable", J.attr(J.attr(tool_10, "parameters"), "required")) : v)(((v) => J.t(v) ? (J.attr(J.attr(tool_10, "parameters"), "required") !== undefined) : v)(((v) => J.t(v) ? J.test("mapping", J.attr(tool_10, "parameters")) : v)((J.attr(tool_10, "parameters") !== undefined)))))) ? J.attr(J.attr(tool_10, "parameters"), "required") : []);
        out.push(J.str(J.concat("\ndef ", J.attr(tool_10, "name"), "(")));
        if (J.t(((v) => J.t(v) ? J.test("mapping", J.attr(J.attr(tool_10, "parameters"), "properties")) : v)(((v) => J.t(v) ? (J.attr(J.attr(tool_10, "parameters"), "properties") !== undefined) : v)(((v) => J.t(v) ? J.test("mapping", J.attr(tool_10, "parameters")) : v)((J.attr(tool_10, "parameters") !== undefined)))))) {
          _ns_12 = J.call(J.namespace, J.kw({"first": true}));
          {
            let _items4 = J.iter(J.filter("items", J.attr(J.attr(tool_10, "parameters"), "properties")));
            const _loop4 = J.loop(_items4);
            for (let _i4 = 0; _i4 < _items4.length; _i4++) {
              _loop4.index0 = _i4; const _it4 = _items4[_i4]; let [param_name_14, param_fields_15] = J.unpack(_it4, 2);
              if (J.t(!J.t(J.attr(_ns_12, "first")))) {
                out.push(J.str(", "));
              }
              out.push(J.str(param_name_14));
              if (J.t((J.attr(param_fields_15, "type") !== undefined))) {
                out.push(J.str(J.concat(": ", J.filter("string", J.attr(param_fields_15, "type")))));
              }
              if (J.t((!J.has(param_name_14, _req_11)))) {
                out.push(J.str(" = None"));
              }
              J.setattr(_ns_12, "first", false);
            }
          }
        }
        out.push(J.str(") -> any:"), J.str("\n    \"\"\""));
        if (J.t((J.attr(tool_10, "description") !== undefined))) {
          out.push(J.str(J.filter("trim", J.attr(tool_10, "description"))));
        }
        if (J.t(((v) => J.t(v) ? J.test("mapping", J.attr(J.attr(tool_10, "parameters"), "properties")) : v)(((v) => J.t(v) ? (J.attr(J.attr(tool_10, "parameters"), "properties") !== undefined) : v)(((v) => J.t(v) ? J.test("mapping", J.attr(tool_10, "parameters")) : v)((J.attr(tool_10, "parameters") !== undefined)))))) {
          out.push(J.str("\n\n    Args:"));
          {
            let _items5 = J.iter(J.filter("items", J.attr(J.attr(tool_10, "parameters"), "properties")));
            const _loop5 = J.loop(_items5);
            for (let _i5 = 0; _i5 < _items5.length; _i5++) {
              _loop5.index0 = _i5; const _it5 = _items5[_i5]; let [param_name_16, param_fields_17] = J.unpack(_it5, 2);
              let handled_keys_18 = handled_keys_13;
              out.push(J.str(J.concat("\n        ", param_name_16)), J.str(" ("));
              if (J.t((J.attr(param_fields_17, "type") !== undefined))) {
                out.push(J.str(J.filter("string", J.attr(param_fields_17, "type"))));
              } else {
                out.push(J.str("any"));
              }
              if (J.t((!J.has(param_name_16, _req_11)))) {
                out.push(J.str(", optional"));
              }
              out.push(J.str(")"));
              if (J.t((J.attr(param_fields_17, "description") !== undefined))) {
                out.push(J.str(J.concat(": ", J.filter("trim", J.attr(param_fields_17, "description")))));
              }
              handled_keys_18 = ["name", "type", "description"];
              out.push(J.str(J.call(render_extra_keys_0, param_fields_17, handled_keys_18)));
            }
          }
        }
        handled_keys_13 = ["type", "properties", "required"];
        out.push(J.str(J.call(render_extra_keys_0, J.attr(tool_10, "parameters"), handled_keys_13)), J.str("\n    \"\"\""));
        handled_keys_13 = ["type", "name", "description", "parameters"];
        out.push(J.str(J.call(render_extra_keys_0, tool_10, handled_keys_13)));
      }
    }
    out.push(J.str("\n</tools>"), J.str("\n\nFor each function call, output the function name and arguments within the following XML format:\n<iquest_tool_call>{function-name}<arg_key>{arg-key-1}</arg_key><arg_value>{arg-value-1}</arg_value><arg_key>{arg-key-2}</arg_key><arg_value>{arg-value-2}</arg_value>...</iquest_tool_call>"), J.str("<|iquest_end|>"));
  }
  visible_text_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_20 = k.caller;
    let content_19 = p.length > 0 ? p[0] : "content" in k ? k["content"] : undefined;
    if (J.t(((v) => J.t(v) ? v : (content_19 === null))((content_19 === undefined)))) {
      out.push(J.str(""));
    } else if (J.t(J.test("string", content_19))) {
      out.push(J.str(content_19));
    } else if (J.t(((v) => J.t(v) ? !J.t(J.test("mapping", content_19)) : v)(J.test("iterable", content_19)))) {
      {
        let _items6 = J.iter(content_19);
        const _loop6 = J.loop(_items6);
        for (let _i6 = 0; _i6 < _items6.length; _i6++) {
          _loop6.index0 = _i6; const _it6 = _items6[_i6]; let item_21 = _it6;
          let media_type_22 = media_type_0;
          if (J.t(((v) => J.t(v) ? (J.eq(J.attr(item_21, "type"), "text")) : v)(J.test("mapping", item_21)))) {
            out.push(J.str(J.attr(item_21, "text")));
          } else if (J.t(J.test("string", item_21))) {
            out.push(J.str(item_21));
          } else if (J.t(((v) => J.t(v) ? (J.has(J.attr(item_21, "type"), ["image", "image_url", "video", "video_url", "audio", "audio_url", "input_audio"])) : v)(J.test("mapping", item_21)))) {
            media_type_22 = J.filter("replace", J.filter("replace", J.attr(item_21, "type"), "_url", ""), "input_", "");
            out.push(J.str(J.concat("<reminder>You are unable to process this ", media_type_22, " because you don't have multi-modal input ability. Try different methods.</reminder>")));
          }
        }
      }
    } else {
      out.push(J.str(content_19));
    }
    return out.join("");
  };
  ns_0 = J.call(J.namespace, J.kw({"last_user_index": J.neg(1)}));
  {
    let _items7 = J.iter(messages_0);
    const _loop7 = J.loop(_items7);
    for (let _i7 = 0; _i7 < _items7.length; _i7++) {
      _loop7.index0 = _i7; const _it7 = _items7[_i7]; let m_23 = _it7;
      if (J.t((J.eq(J.attr(m_23, "role"), "user")))) {
        J.setattr(ns_0, "last_user_index", J.attr(_loop7, "index0"));
      }
    }
  }
  {
    let _items8 = J.iter(messages_0);
    const _loop8 = J.loop(_items8);
    for (let _i8 = 0; _i8 < _items8.length; _i8++) {
      _loop8.index0 = _i8; const _it8 = _items8[_i8]; let m_24 = _it8;
      let content_25 = content_0;
      let reasoning_content_26 = reasoning_content_0;
      if (J.t((J.eq(J.attr(m_24, "role"), "user")))) {
        out.push(J.str("<|iquest_user|>"), J.str(J.call(visible_text_0, J.attr(m_24, "content"))), J.str("<|iquest_end|>"));
      } else if (J.t((J.eq(J.attr(m_24, "role"), "assistant")))) {
        out.push(J.str("<|iquest_assistant|>"));
        content_25 = J.call(visible_text_0, J.attr(m_24, "content"));
        if (J.t(J.test("string", J.attr(m_24, "reasoning_content")))) {
          reasoning_content_26 = J.attr(m_24, "reasoning_content");
        } else if (J.t((J.attr(m_24, "reasoning_content") === null))) {
          reasoning_content_26 = "";
        } else if (J.t((J.has("</think>", content_25)))) {
          reasoning_content_26 = J.item(J.call(J.attr(J.item(J.call(J.attr(content_25, "split"), "</think>"), 0), "split"), "<think>"), J.neg(1));
          content_25 = J.item(J.call(J.attr(content_25, "split"), "</think>"), J.neg(1));
        }
        if (J.t(((v) => J.t(v) ? (reasoning_content_26 !== undefined) : v)(((v) => J.t(v) ? v : (J.gt(J.attr(_loop8, "index0"), J.attr(ns_0, "last_user_index"))))(((v) => J.t(v) ? v : !J.t(clear_thinking_0))(!J.t((clear_thinking_0 !== undefined))))))) {
          if (J.t(((v) => J.t(v) ? !J.t(J.attr(m_24, "reasoning_content_mask")) : v)((J.attr(m_24, "reasoning_content_mask") !== undefined)))) {
            {
              out.push(J.str(J.add(J.add("<think>", reasoning_content_26), "</think>")));
            }
          } else {
            out.push(J.str("<think>"), J.str(reasoning_content_26));
            if (J.t(((v) => J.t(v) ? !J.t(J.attr(m_24, "boundary_token_mask")) : v)((J.attr(m_24, "boundary_token_mask") !== undefined)))) {
              {
                out.push(J.str("</think>"));
              }
            } else {
              out.push(J.str("</think>"));
            }
          }
        } else {
          out.push(J.str("<think>"));
          if (J.t(((v) => J.t(v) ? !J.t(J.attr(m_24, "boundary_token_mask")) : v)((J.attr(m_24, "boundary_token_mask") !== undefined)))) {
            {
              out.push(J.str("</think>"));
            }
          } else {
            out.push(J.str("</think>"));
          }
        }
        if (J.t(((v) => J.t(v) ? !J.t(J.attr(m_24, "content_mask")) : v)((J.attr(m_24, "content_mask") !== undefined)))) {
          {
            if (J.t(J.call(J.attr(content_25, "strip")))) {
              out.push(J.str(J.call(J.attr(content_25, "strip"))));
            }
            if (J.t(J.attr(m_24, "tool_calls"))) {
              {
                let _items9 = J.iter(J.attr(m_24, "tool_calls"));
                const _loop9 = J.loop(_items9);
                for (let _i9 = 0; _i9 < _items9.length; _i9++) {
                  _loop9.index0 = _i9; const _it9 = _items9[_i9]; let tc_27 = _it9;
                  let _args_28 = _args_0;
                  if (J.t(J.attr(tc_27, "function"))) {
                    tc_27 = J.attr(tc_27, "function");
                  }
                  out.push(J.str(J.add("<iquest_tool_call>", J.attr(tc_27, "name"))));
                  _args_28 = (J.t(J.test("string", J.attr(tc_27, "arguments"))) ? J.filter("fromjson", J.attr(tc_27, "arguments")) : J.attr(tc_27, "arguments"));
                  if (J.t(!J.t(J.test("mapping", _args_28)))) {
                    out.push(J.str(J.call(J.raise_exception, "tool call arguments must be a JSON object")));
                  }
                  {
                    let _items10 = J.iter(J.call(J.attr(_args_28, "items")));
                    const _loop10 = J.loop(_items10);
                    for (let _i10 = 0; _i10 < _items10.length; _i10++) {
                      _loop10.index0 = _i10; const _it10 = _items10[_i10]; let [k_29, v_30] = J.unpack(_it10, 2);
                      out.push("<arg_key>", J.str(k_29), "</arg_key><arg_value>", J.str((J.t(!J.t(J.test("string", v_30))) ? J.filter("tojson", v_30, J.kw({"ensure_ascii": false})) : v_30)), "</arg_value>");
                    }
                  }
                  out.push("</iquest_tool_call>");
                }
              }
            }
            out.push(J.str("<|iquest_end|>"));
          }
        } else {
          if (J.t(J.call(J.attr(content_25, "strip")))) {
            out.push(J.str(J.call(J.attr(content_25, "strip"))));
          }
          if (J.t(J.attr(m_24, "tool_calls"))) {
            {
              let _items11 = J.iter(J.attr(m_24, "tool_calls"));
              const _loop11 = J.loop(_items11);
              for (let _i11 = 0; _i11 < _items11.length; _i11++) {
                _loop11.index0 = _i11; const _it11 = _items11[_i11]; let tc_31 = _it11;
                let _args_32 = _args_0;
                if (J.t(J.attr(tc_31, "function"))) {
                  tc_31 = J.attr(tc_31, "function");
                }
                out.push(J.str(J.add("<iquest_tool_call>", J.attr(tc_31, "name"))));
                _args_32 = (J.t(J.test("string", J.attr(tc_31, "arguments"))) ? J.filter("fromjson", J.attr(tc_31, "arguments")) : J.attr(tc_31, "arguments"));
                if (J.t(!J.t(J.test("mapping", _args_32)))) {
                  out.push(J.str(J.call(J.raise_exception, "tool call arguments must be a JSON object")));
                }
                {
                  let _items12 = J.iter(J.call(J.attr(_args_32, "items")));
                  const _loop12 = J.loop(_items12);
                  for (let _i12 = 0; _i12 < _items12.length; _i12++) {
                    _loop12.index0 = _i12; const _it12 = _items12[_i12]; let [k_33, v_34] = J.unpack(_it12, 2);
                    out.push("<arg_key>", J.str(k_33), "</arg_key><arg_value>", J.str((J.t(!J.t(J.test("string", v_34))) ? J.filter("tojson", v_34, J.kw({"ensure_ascii": false})) : v_34)), "</arg_value>");
                  }
                }
                out.push("</iquest_tool_call>");
              }
            }
          }
          if (J.t(((v) => J.t(v) ? !J.t(J.attr(m_24, "boundary_token_mask")) : v)((J.attr(m_24, "boundary_token_mask") !== undefined)))) {
            {
              out.push(J.str("<|iquest_end|>"));
            }
          } else {
            out.push(J.str("<|iquest_end|>"));
          }
        }
      } else if (J.t((J.eq(J.attr(m_24, "role"), "tool")))) {
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.sub(J.attr(_loop8, "index0"), 1)), "role"), "tool")))(J.attr(_loop8, "first")))) {
          out.push(J.str("<|iquest_tool|>"));
        }
        if (J.t(J.test("string", J.attr(m_24, "content")))) {
          out.push(J.str(J.add(J.add("<tool_response>", J.attr(m_24, "content")), "</tool_response>")));
        } else if (J.t(((v) => J.t(v) ? (J.eq(J.attr(J.item(J.attr(m_24, "content"), 0), "type"), "tool_reference")) : v)(((v) => J.t(v) ? J.attr(m_24, "content") : v)(((v) => J.t(v) ? !J.t(J.test("mapping", J.attr(m_24, "content"))) : v)(J.test("iterable", J.attr(m_24, "content"))))))) {
          out.push(J.str("<tool_response><tools>\n"));
          {
            let _items13 = J.iter(J.attr(m_24, "content"));
            const _loop13 = J.loop(_items13);
            for (let _i13 = 0; _i13 < _items13.length; _i13++) {
              _loop13.index0 = _i13; const _it13 = _items13[_i13]; let tr_35 = _it13;
              {
                let _items14 = J.iter(tools_0);
                const _loop14 = J.loop(_items14);
                for (let _i14 = 0; _i14 < _items14.length; _i14++) {
                  _loop14.index0 = _i14; const _it14 = _items14[_i14]; let tool_36 = _it14;
                  if (J.t((J.has("function", tool_36)))) {
                    tool_36 = J.item(tool_36, "function");
                  }
                  if (J.t((J.eq(J.attr(tool_36, "name"), J.attr(tr_35, "name"))))) {
                    out.push(J.str(J.add(J.call(tool_to_json_0, tool_36), "\n")));
                  }
                }
              }
            }
          }
          out.push(J.str("</tools></tool_response>"));
        } else if (J.t(((v) => J.t(v) ? (J.attr(J.item(J.attr(m_24, "content"), 0), "output") !== undefined) : v)(((v) => J.t(v) ? J.test("mapping", J.item(J.attr(m_24, "content"), 0)) : v)(((v) => J.t(v) ? J.attr(m_24, "content") : v)(((v) => J.t(v) ? !J.t(J.test("mapping", J.attr(m_24, "content"))) : v)(J.test("iterable", J.attr(m_24, "content")))))))) {
          {
            let _items15 = J.iter(J.attr(m_24, "content"));
            const _loop15 = J.loop(_items15);
            for (let _i15 = 0; _i15 < _items15.length; _i15++) {
              _loop15.index0 = _i15; const _it15 = _items15[_i15]; let tr_37 = _it15;
              out.push(J.str(J.add(J.add("<tool_response>", J.attr(tr_37, "output")), "</tool_response>")));
            }
          }
        } else {
          out.push(J.str(J.add(J.add("<tool_response>", J.call(visible_text_0, J.attr(m_24, "content"))), "</tool_response>")));
        }
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.add(J.attr(_loop8, "index0"), 1)), "role"), "tool")))(J.attr(_loop8, "last")))) {
          out.push(J.str("<|iquest_end|>"));
        }
      } else if (J.t((J.eq(J.attr(m_24, "role"), "system")))) {
        out.push(J.str("<|iquest_system|>"), J.str(J.call(visible_text_0, J.attr(m_24, "content"))), J.str("<|iquest_end|>"));
      } else if (J.t((J.eq(J.attr(m_24, "role"), "developer")))) {
        out.push(J.str("<|iquest_developer|>"), J.str(J.call(visible_text_0, J.attr(m_24, "content"))), J.str("<|iquest_end|>"));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|iquest_assistant|>"), J.str((J.t(((v) => J.t(v) ? !J.t(enable_thinking_0) : v)((enable_thinking_0 !== undefined))) ? "<think></think>" : "<think>")));
  }
  return out.join("");
}
