// Converted from the chat template of zai-org/GLM-5.3-Flash by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let effective_reasoning_effort_0 = ctx["effective_reasoning_effort"];
  let clear_thinking_0 = ctx["clear_thinking"];
  let tool_to_json_0 = ctx["tool_to_json"];
  let tool_references_to_response_0 = ctx["tool_references_to_response"];
  let emit_image_0 = ctx["emit_image"];
  let emit_video_0 = ctx["emit_video"];
  let emit_audio_0 = ctx["emit_audio"];
  let visible_text_0 = ctx["visible_text"];
  let tool_response_0 = ctx["tool_response"];
  let render_tool_response_0 = ctx["render_tool_response"];
  let id_of_0 = ctx["id_of"];
  let is_list_of_outputs_0 = ctx["is_list_of_outputs"];
  let has_dup_tool_result_id_0 = ctx["has_dup_tool_result_id"];
  let tc_id_exists_0 = ctx["tc_id_exists"];
  let ns_0 = ctx["ns"];
  let reasoning_effort_0 = ctx["reasoning_effort"];
  let tools_0 = ctx["tools"] ?? null;
  let ns_tool_0 = ctx["ns_tool"];
  let ns_cnt_0 = ctx["ns_cnt"];
  let m_0 = ctx["m"];
  let messages_0 = ctx["messages"];
  let ns_f_0 = ctx["ns_f"];
  let content_0 = ctx["content"];
  let reasoning_content_0 = ctx["reasoning_content"];
  let block_start_0 = ctx["block_start"];
  let ns_blk_0 = ctx["ns_blk"];
  let ns_a_0 = ctx["ns_a"];
  let ns_chk_0 = ctx["ns_chk"];
  let ns_ord_0 = ctx["ns_ord"];
  let _args_0 = ctx["_args"];
  let entry_0 = ctx["entry"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  out.push("[gMASK]<sop>");
  effective_reasoning_effort_0 = (J.t(((v) => J.t(v) ? (J.has(reasoning_effort_0, ["low", "high"])) : v)((reasoning_effort_0 !== undefined))) ? reasoning_effort_0 : "max");
  if (J.t(!J.t((effective_reasoning_effort_0 === null)))) {
    out.push("<|system|>Reasoning Effort: ", J.str(J.filter("capitalize", effective_reasoning_effort_0)));
  }
  clear_thinking_0 = (J.t((clear_thinking_0 !== undefined)) ? clear_thinking_0 : false);
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
    tool_references_to_response_0 = (...args) => {
      const [p, k] = J.args(args);
      const out = [];
      const caller_7 = k.caller;
      let refs_6 = p.length > 0 ? p[0] : "refs" in k ? k["refs"] : undefined;
      out.push(J.str("<tool_response><tools>\n"));
      {
        let _items2 = J.iter(refs_6);
        const _loop2 = J.loop(_items2);
        for (let _i2 = 0; _i2 < _items2.length; _i2++) {
          _loop2.index0 = _i2; const _it2 = _items2[_i2]; let tr_8 = _it2;
          {
            let _items3 = J.iter(tools_0);
            const _loop3 = J.loop(_items3);
            for (let _i3 = 0; _i3 < _items3.length; _i3++) {
              _loop3.index0 = _i3; const _it3 = _items3[_i3]; let tool_9 = _it3;
              if (J.t((J.has("function", tool_9)))) {
                tool_9 = J.item(tool_9, "function");
              }
              if (J.t((J.eq(J.attr(tool_9, "name"), J.attr(tr_8, "name"))))) {
                out.push(J.str(J.add(J.call(tool_to_json_0, tool_9), "\n")));
              }
            }
          }
        }
      }
      out.push(J.str("</tools></tool_response>"));
      return out.join("");
    };
    out.push("<|system|>\n# Tools\n\nYou may call one or more functions to assist with the user query.\n\nYou are provided with function signatures within <tools></tools> XML tags:\n<tools>\n");
    {
      let _items4 = J.iter(tools_0);
      const _loop4 = J.loop(_items4);
      for (let _i4 = 0; _i4 < _items4.length; _i4++) {
        _loop4.index0 = _i4; const _it4 = _items4[_i4]; let tool_10 = _it4;
        if (J.t((J.has("function", tool_10)))) {
          tool_10 = J.item(tool_10, "function");
        }
        if (J.t(((v) => J.t(v) ? v : !J.t(J.attr(tool_10, "defer_loading")))(!J.t((J.attr(tool_10, "defer_loading") !== undefined))))) {
          out.push(J.str(J.call(tool_to_json_0, tool_10)), "\n");
        }
      }
    }
    out.push("</tools>\n\nFor each function call, output the function name and arguments within the following XML format:\n<tool_call>{function-name}<arg_key>{arg-key-1}</arg_key><arg_value>{arg-value-1}</arg_value><arg_key>{arg-key-2}</arg_key><arg_value>{arg-value-2}</arg_value>...</tool_call>");
  }
  emit_image_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_11 = k.caller;
    out.push("<|begin_of_image|><|image|><|end_of_image|>");
    return out.join("");
  };
  emit_video_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_12 = k.caller;
    out.push("<|begin_of_video|><|video|><|end_of_video|>");
    return out.join("");
  };
  emit_audio_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_13 = k.caller;
    out.push("<|begin_of_audio|><|end_of_audio|>");
    return out.join("");
  };
  visible_text_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_15 = k.caller;
    let content_14 = p.length > 0 ? p[0] : "content" in k ? k["content"] : undefined;
    if (J.t(J.test("string", content_14))) {
      out.push(J.str(content_14));
    } else if (J.t(((v) => J.t(v) ? !J.t(J.test("mapping", content_14)) : v)(J.test("iterable", content_14)))) {
      {
        let _items5 = J.iter(content_14);
        const _loop5 = J.loop(_items5);
        for (let _i5 = 0; _i5 < _items5.length; _i5++) {
          _loop5.index0 = _i5; const _it5 = _items5[_i5]; let item_16 = _it5;
          if (J.t(((v) => J.t(v) ? (J.eq(J.attr(item_16, "type"), "text")) : v)(J.test("mapping", item_16)))) {
            out.push(J.str(J.attr(item_16, "text")));
          } else if (J.t(J.test("string", item_16))) {
            out.push(J.str(item_16));
          } else if (J.t(((v) => J.t(v) ? (J.has(J.attr(item_16, "type"), ["image", "image_url"])) : v)(J.test("mapping", item_16)))) {
            out.push(J.str(J.call(emit_image_0)));
          } else if (J.t(((v) => J.t(v) ? (J.has(J.attr(item_16, "type"), ["video", "video_url"])) : v)(J.test("mapping", item_16)))) {
            out.push(J.str(J.call(emit_video_0)));
          } else if (J.t(((v) => J.t(v) ? (J.has(J.attr(item_16, "type"), ["audio", "audio_url", "input_audio"])) : v)(J.test("mapping", item_16)))) {
            out.push(J.str(J.call(emit_audio_0)));
          }
        }
      }
    } else if (J.t(!J.t((content_14 === null)))) {
      out.push(J.str(content_14));
    }
    return out.join("");
  };
  tool_response_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_18 = k.caller;
    let text_17 = p.length > 0 ? p[0] : "text" in k ? k["text"] : undefined;
    out.push(J.str(J.add(J.add("<tool_response>", text_17), "</tool_response>")));
    return out.join("");
  };
  render_tool_response_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_20 = k.caller;
    let m_19 = p.length > 0 ? p[0] : "m" in k ? k["m"] : undefined;
    if (J.t(J.test("string", J.attr(m_19, "content")))) {
      out.push(J.str(J.call(tool_response_0, J.attr(m_19, "content"))));
    } else if (J.t(((v) => J.t(v) ? (J.eq(J.attr(J.item(J.attr(m_19, "content"), 0), "type"), "tool_reference")) : v)(((v) => J.t(v) ? !J.t(J.test("mapping", J.attr(m_19, "content"))) : v)(J.attr(m_19, "content"))))) {
      out.push(J.str(J.call(tool_references_to_response_0, J.attr(m_19, "content"))));
    } else if (J.t(J.call(is_list_of_outputs_0, m_19))) {
      {
        let _items6 = J.iter(J.attr(m_19, "content"));
        const _loop6 = J.loop(_items6);
        for (let _i6 = 0; _i6 < _items6.length; _i6++) {
          _loop6.index0 = _i6; const _it6 = _items6[_i6]; let tr_21 = _it6;
          if (J.t(((v) => J.t(v) ? (J.eq(J.attr(J.item(J.attr(tr_21, "output"), 0), "type"), "tool_reference")) : v)(((v) => J.t(v) ? J.attr(tr_21, "output") : v)(((v) => J.t(v) ? !J.t(J.test("mapping", J.attr(tr_21, "output"))) : v)(((v) => J.t(v) ? !J.t(J.test("string", J.attr(tr_21, "output"))) : v)(J.test("iterable", J.attr(tr_21, "output")))))))) {
            out.push(J.str(J.call(tool_references_to_response_0, J.attr(tr_21, "output"))));
          } else {
            out.push(J.str(J.call(tool_response_0, J.call(visible_text_0, J.attr(tr_21, "output")))));
          }
        }
      }
    } else {
      out.push(J.str(J.call(tool_response_0, J.call(visible_text_0, J.attr(m_19, "content")))));
    }
    return out.join("");
  };
  id_of_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_23 = k.caller;
    let obj_22 = p.length > 0 ? p[0] : "obj" in k ? k["obj"] : undefined;
    if (J.t(J.attr(obj_22, "tool_call_id"))) {
      out.push(J.str(J.attr(obj_22, "tool_call_id")));
    } else if (J.t(J.attr(obj_22, "id"))) {
      out.push(J.str(J.attr(obj_22, "id")));
    }
    return out.join("");
  };
  is_list_of_outputs_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_25 = k.caller;
    let m_24 = p.length > 0 ? p[0] : "m" in k ? k["m"] : undefined;
    if (J.t(((v) => J.t(v) ? (J.attr(J.item(J.attr(m_24, "content"), 0), "output") !== undefined) : v)(J.attr(m_24, "content")))) {
      out.push("1");
    }
    return out.join("");
  };
  has_dup_tool_result_id_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_29 = k.caller;
    let lo_26 = p.length > 0 ? p[0] : "lo" in k ? k["lo"] : undefined;
    let hi_27 = p.length > 1 ? p[1] : "hi" in k ? k["hi"] : undefined;
    let target_28 = p.length > 2 ? p[2] : "target" in k ? k["target"] : undefined;
    let ns_cnt_30 = ns_cnt_0;
    ns_cnt_30 = J.call(J.namespace, J.kw({"n": 0}));
    {
      let _items7 = J.iter(J.call(J.range, lo_26, J.add(hi_27, 1)));
      const _loop7 = J.loop(_items7);
      for (let _i7 = 0; _i7 < _items7.length; _i7++) {
        _loop7.index0 = _i7; const _it7 = _items7[_i7]; let k_31 = _it7;
        let m_32 = m_0;
        m_32 = J.item(messages_0, k_31);
        if (J.t(J.call(is_list_of_outputs_0, m_32))) {
          {
            let _items8 = J.iter(J.attr(m_32, "content"));
            const _loop8 = J.loop(_items8);
            for (let _i8 = 0; _i8 < _items8.length; _i8++) {
              _loop8.index0 = _i8; const _it8 = _items8[_i8]; let entry_33 = _it8;
              if (J.t((J.eq(J.call(id_of_0, entry_33), target_28)))) {
                J.setattr(ns_cnt_30, "n", J.add(J.attr(ns_cnt_30, "n"), 1));
              }
            }
          }
        } else if (J.t((J.eq(J.call(id_of_0, m_32), target_28)))) {
          J.setattr(ns_cnt_30, "n", J.add(J.attr(ns_cnt_30, "n"), 1));
        }
        if (J.t((J.gt(J.attr(ns_cnt_30, "n"), 1)))) {
          break;
        }
      }
    }
    if (J.t((J.gt(J.attr(ns_cnt_30, "n"), 1)))) {
      out.push("1");
    }
    return out.join("");
  };
  tc_id_exists_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_36 = k.caller;
    let tcs_34 = p.length > 0 ? p[0] : "tcs" in k ? k["tcs"] : undefined;
    let target_35 = p.length > 1 ? p[1] : "target" in k ? k["target"] : undefined;
    let ns_f_37 = ns_f_0;
    ns_f_37 = J.call(J.namespace, J.kw({"found": false}));
    {
      let _items9 = J.iter(tcs_34);
      const _loop9 = J.loop(_items9);
      for (let _i9 = 0; _i9 < _items9.length; _i9++) {
        _loop9.index0 = _i9; const _it9 = _items9[_i9]; let tc_38 = _it9;
        if (J.t((J.eq(J.call(id_of_0, tc_38), target_35)))) {
          J.setattr(ns_f_37, "found", true);
          break;
        }
      }
    }
    if (J.t(J.attr(ns_f_37, "found"))) {
      out.push("1");
    }
    return out.join("");
  };
  ns_0 = J.call(J.namespace, J.kw({"last_user_index": J.neg(1)}));
  {
    let _items10 = J.iter(messages_0);
    const _loop10 = J.loop(_items10);
    for (let _i10 = 0; _i10 < _items10.length; _i10++) {
      _loop10.index0 = _i10; const _it10 = _items10[_i10]; let m_39 = _it10;
      if (J.t((J.eq(J.attr(m_39, "role"), "user")))) {
        J.setattr(ns_0, "last_user_index", J.attr(_loop10, "index0"));
      }
    }
  }
  {
    let _items11 = J.iter(messages_0);
    const _loop11 = J.loop(_items11);
    for (let _i11 = 0; _i11 < _items11.length; _i11++) {
      _loop11.index0 = _i11; const _it11 = _items11[_i11]; let m_40 = _it11;
      let content_41 = content_0;
      let reasoning_content_42 = reasoning_content_0;
      let block_start_43 = block_start_0;
      let ns_blk_44 = ns_blk_0;
      let ns_a_45 = ns_a_0;
      let ns_chk_46 = ns_chk_0;
      let ns_ord_47 = ns_ord_0;
      if (J.t((J.eq(J.attr(m_40, "role"), "user")))) {
        out.push("<|user|>", J.str(J.call(visible_text_0, J.attr(m_40, "content"))));
      } else if (J.t((J.eq(J.attr(m_40, "role"), "assistant")))) {
        out.push("<|assistant|>");
        content_41 = J.call(visible_text_0, J.attr(m_40, "content"));
        if (J.t(J.test("string", J.attr(m_40, "reasoning_content")))) {
          reasoning_content_42 = J.attr(m_40, "reasoning_content");
        } else if (J.t((J.has("</think>", content_41)))) {
          reasoning_content_42 = J.item(J.call(J.attr(J.item(J.call(J.attr(content_41, "split"), "</think>"), 0), "split"), "<think>"), J.neg(1));
          content_41 = J.item(J.call(J.attr(content_41, "split"), "</think>"), J.neg(1));
        }
        if (J.t(((v) => J.t(v) ? (reasoning_content_42 !== undefined) : v)(((v) => J.t(v) ? v : (J.gt(J.attr(_loop11, "index0"), J.attr(ns_0, "last_user_index"))))(!J.t(clear_thinking_0))))) {
          out.push(J.str(J.add(J.add("<think>", reasoning_content_42), "</think>")));
        } else {
          out.push(J.str("<think></think>"));
        }
        if (J.t(J.call(J.attr(content_41, "strip")))) {
          out.push(J.str(J.call(J.attr(content_41, "strip"))));
        }
        if (J.t(J.attr(m_40, "tool_calls"))) {
          {
            let _items12 = J.iter(J.attr(m_40, "tool_calls"));
            const _loop12 = J.loop(_items12);
            for (let _i12 = 0; _i12 < _items12.length; _i12++) {
              _loop12.index0 = _i12; const _it12 = _items12[_i12]; let tc_48 = _it12;
              let _args_49 = _args_0;
              if (J.t(J.attr(tc_48, "function"))) {
                tc_48 = J.attr(tc_48, "function");
              }
              out.push(J.str(J.concat("<tool_call>", J.attr(tc_48, "name"))));
              _args_49 = J.attr(tc_48, "arguments");
              {
                let _items13 = J.iter(J.call(J.attr(_args_49, "items")));
                const _loop13 = J.loop(_items13);
                for (let _i13 = 0; _i13 < _items13.length; _i13++) {
                  _loop13.index0 = _i13; const _it13 = _items13[_i13]; let [k_50, v_51] = J.unpack(_it13, 2);
                  out.push("<arg_key>", J.str(k_50), "</arg_key><arg_value>", J.str((J.t(!J.t(J.test("string", v_51))) ? J.filter("tojson", v_51, J.kw({"ensure_ascii": false})) : v_51)), "</arg_value>");
                }
              }
              out.push("</tool_call>");
            }
          }
        }
      } else if (J.t((J.eq(J.attr(m_40, "role"), "tool")))) {
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.sub(J.attr(_loop11, "index0"), 1)), "role"), "tool")))(J.attr(_loop11, "first")))) {
          out.push(J.str("<|observation|>"));
          block_start_43 = J.attr(_loop11, "index0");
          ns_blk_44 = J.call(J.namespace, J.kw({"end": block_start_43}));
          {
            let _items14 = J.iter(J.call(J.range, block_start_43, J.filter("length", messages_0)));
            const _loop14 = J.loop(_items14);
            for (let _i14 = 0; _i14 < _items14.length; _i14++) {
              _loop14.index0 = _i14; const _it14 = _items14[_i14]; let j_52 = _it14;
              if (J.t((J.eq(J.attr(J.item(messages_0, j_52), "role"), "tool")))) {
                J.setattr(ns_blk_44, "end", j_52);
              } else {
                break;
              }
            }
          }
          ns_a_45 = J.call(J.namespace, J.kw({"tool_calls": null}));
          if (J.t(((v) => J.t(v) ? J.attr(J.item(messages_0, J.sub(block_start_43, 1)), "tool_calls") : v)(((v) => J.t(v) ? (J.eq(J.attr(J.item(messages_0, J.sub(block_start_43, 1)), "role"), "assistant")) : v)((J.gt(block_start_43, 0)))))) {
            J.setattr(ns_a_45, "tool_calls", J.attr(J.item(messages_0, J.sub(block_start_43, 1)), "tool_calls"));
          }
          ns_chk_46 = J.call(J.namespace, J.kw({"can_sort": true, "res": [], "mk": [], "ek": [], "tcs": []}));
          {
            let _items15 = J.iter(J.call(J.range, block_start_43, J.add(J.attr(ns_blk_44, "end"), 1)));
            const _loop15 = J.loop(_items15);
            for (let _i15 = 0; _i15 < _items15.length; _i15++) {
              _loop15.index0 = _i15; const _it15 = _items15[_i15]; let k_53 = _it15;
              let m_54 = m_40;
              m_54 = J.item(messages_0, k_53);
              if (J.t(J.call(is_list_of_outputs_0, m_54))) {
                {
                  let _items16 = J.iter(J.attr(m_54, "content"));
                  const _loop16 = J.loop(_items16);
                  for (let _i16 = 0; _i16 < _items16.length; _i16++) {
                    _loop16.index0 = _i16; const _it16 = _items16[_i16]; let entry_55 = _it16;
                    J.setattr(ns_chk_46, "res", J.add(J.attr(ns_chk_46, "res"), [J.call(id_of_0, entry_55)]));
                    J.setattr(ns_chk_46, "mk", J.add(J.attr(ns_chk_46, "mk"), [k_53]));
                    J.setattr(ns_chk_46, "ek", J.add(J.attr(ns_chk_46, "ek"), [J.attr(_loop16, "index0")]));
                  }
                }
              } else {
                J.setattr(ns_chk_46, "res", J.add(J.attr(ns_chk_46, "res"), [J.call(id_of_0, m_54)]));
                J.setattr(ns_chk_46, "mk", J.add(J.attr(ns_chk_46, "mk"), [k_53]));
                J.setattr(ns_chk_46, "ek", J.add(J.attr(ns_chk_46, "ek"), [J.neg(1)]));
              }
            }
          }
          if (J.t(!J.t(J.attr(ns_a_45, "tool_calls")))) {
            J.setattr(ns_chk_46, "can_sort", false);
          } else {
            {
              let _items17 = J.iter(J.attr(ns_a_45, "tool_calls"));
              const _loop17 = J.loop(_items17);
              for (let _i17 = 0; _i17 < _items17.length; _i17++) {
                _loop17.index0 = _i17; const _it17 = _items17[_i17]; let tc_56 = _it17;
                J.setattr(ns_chk_46, "tcs", J.add(J.attr(ns_chk_46, "tcs"), [J.call(id_of_0, tc_56)]));
              }
            }
            if (J.t(((v) => J.t(v) ? v : (J.has("", J.attr(ns_chk_46, "tcs"))))((J.has("", J.attr(ns_chk_46, "res")))))) {
              J.setattr(ns_chk_46, "can_sort", false);
            } else if (J.t((!J.eq(J.filter("length", J.filter("list", J.filter("unique", J.attr(ns_chk_46, "res"), J.kw({"case_sensitive": true})))), J.filter("length", J.attr(ns_chk_46, "res")))))) {
              J.setattr(ns_chk_46, "can_sort", false);
            } else if (J.t((!J.eq(J.filter("length", J.filter("list", J.filter("unique", J.attr(ns_chk_46, "tcs"), J.kw({"case_sensitive": true})))), J.filter("length", J.attr(ns_chk_46, "tcs")))))) {
              J.setattr(ns_chk_46, "can_sort", false);
            } else {
              {
                let _items18 = J.iter(J.attr(ns_chk_46, "res"));
                const _loop18 = J.loop(_items18);
                for (let _i18 = 0; _i18 < _items18.length; _i18++) {
                  _loop18.index0 = _i18; const _it18 = _items18[_i18]; let eid_57 = _it18;
                  if (J.t((!J.has(eid_57, J.attr(ns_chk_46, "tcs"))))) {
                    J.setattr(ns_chk_46, "can_sort", false);
                    break;
                  }
                }
              }
            }
          }
          if (J.t(J.attr(ns_chk_46, "can_sort"))) {
            ns_ord_47 = J.call(J.namespace, J.kw({"res": [], "tcs": [], "j": 0, "pairs": []}));
            {
              let _items19 = J.iter(J.attr(ns_chk_46, "res"));
              const _loop19 = J.loop(_items19);
              for (let _i19 = 0; _i19 < _items19.length; _i19++) {
                _loop19.index0 = _i19; const _it19 = _items19[_i19]; let eid_58 = _it19;
                J.setattr(ns_ord_47, "res", J.add(J.attr(ns_ord_47, "res"), [({"id": eid_58, "p": J.attr(_loop19, "index0")})]));
              }
            }
            {
              let _items20 = J.iter(J.attr(ns_chk_46, "tcs"));
              const _loop20 = J.loop(_items20);
              for (let _i20 = 0; _i20 < _items20.length; _i20++) {
                _loop20.index0 = _i20; const _it20 = _items20[_i20]; let tc_id_59 = _it20;
                J.setattr(ns_ord_47, "tcs", J.add(J.attr(ns_ord_47, "tcs"), [({"id": tc_id_59, "i": J.attr(_loop20, "index0")})]));
              }
            }
            J.setattr(ns_ord_47, "res", J.filter("sort", J.attr(ns_ord_47, "res"), J.kw({"attribute": "id", "case_sensitive": true})));
            {
              let _items21 = J.iter(J.filter("sort", J.attr(ns_ord_47, "tcs"), J.kw({"attribute": "id", "case_sensitive": true})));
              const _loop21 = J.loop(_items21);
              for (let _i21 = 0; _i21 < _items21.length; _i21++) {
                _loop21.index0 = _i21; const _it21 = _items21[_i21]; let tc_60 = _it21;
                if (J.t(((v) => J.t(v) ? (J.eq(J.attr(J.item(J.attr(ns_ord_47, "res"), J.attr(ns_ord_47, "j")), "id"), J.attr(tc_60, "id"))) : v)((J.lt(J.attr(ns_ord_47, "j"), J.filter("length", J.attr(ns_ord_47, "res"))))))) {
                  J.setattr(ns_ord_47, "pairs", J.add(J.attr(ns_ord_47, "pairs"), [({"i": J.attr(tc_60, "i"), "p": J.attr(J.item(J.attr(ns_ord_47, "res"), J.attr(ns_ord_47, "j")), "p")})]));
                  J.setattr(ns_ord_47, "j", J.add(J.attr(ns_ord_47, "j"), 1));
                }
              }
            }
            {
              let _items22 = J.iter(J.filter("sort", J.attr(ns_ord_47, "pairs"), J.kw({"attribute": "i"})));
              const _loop22 = J.loop(_items22);
              for (let _i22 = 0; _i22 < _items22.length; _i22++) {
                _loop22.index0 = _i22; const _it22 = _items22[_i22]; let pair_61 = _it22;
                let m_62 = m_40;
                let entry_63 = entry_0;
                m_62 = J.item(messages_0, J.item(J.attr(ns_chk_46, "mk"), J.attr(pair_61, "p")));
                if (J.t((J.ge(J.item(J.attr(ns_chk_46, "ek"), J.attr(pair_61, "p")), 0)))) {
                  entry_63 = J.item(J.attr(m_62, "content"), J.item(J.attr(ns_chk_46, "ek"), J.attr(pair_61, "p")));
                  if (J.t(((v) => J.t(v) ? (J.eq(J.attr(J.item(J.attr(entry_63, "output"), 0), "type"), "tool_reference")) : v)(((v) => J.t(v) ? J.attr(entry_63, "output") : v)(((v) => J.t(v) ? !J.t(J.test("mapping", J.attr(entry_63, "output"))) : v)(((v) => J.t(v) ? !J.t(J.test("string", J.attr(entry_63, "output"))) : v)(J.test("iterable", J.attr(entry_63, "output")))))))) {
                    out.push(J.str(J.call(tool_references_to_response_0, J.attr(entry_63, "output"))));
                  } else {
                    out.push(J.str(J.call(tool_response_0, J.call(visible_text_0, J.attr(entry_63, "output")))));
                  }
                } else {
                  out.push(J.str(J.call(render_tool_response_0, m_62)));
                }
              }
            }
          } else {
            {
              let _items23 = J.iter(J.call(J.range, block_start_43, J.add(J.attr(ns_blk_44, "end"), 1)));
              const _loop23 = J.loop(_items23);
              for (let _i23 = 0; _i23 < _items23.length; _i23++) {
                _loop23.index0 = _i23; const _it23 = _items23[_i23]; let k_64 = _it23;
                out.push(J.str(J.call(render_tool_response_0, J.item(messages_0, k_64))));
              }
            }
          }
        }
      } else if (J.t((J.eq(J.attr(m_40, "role"), "system")))) {
        out.push("<|system|>", J.str(J.call(visible_text_0, J.attr(m_40, "content"))));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push("<|assistant|>", J.str("<think>"));
  }
  return out.join("");
}
