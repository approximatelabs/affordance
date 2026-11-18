// Converted from the chat template of deepseek-ai/DeepSeek-V4.1-Flash-1017 by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let IMG_0 = ctx["IMG"];
  let DSML_0 = ctx["DSML"];
  let thinking_mode_0 = ctx["thinking_mode"];
  let drop_thinking_0 = ctx["drop_thinking"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let tools_0 = ctx["tools"] ?? null;
  let reasoning_effort_0 = ctx["reasoning_effort"];
  let effort_map_0 = ctx["effort_map"];
  let budget_0 = ctx["budget"];
  let joined_text_0 = ctx["joined_text"];
  let qualified_name_0 = ctx["qualified_name"];
  let render_tools_0 = ctx["render_tools"];
  let mns_0 = ctx["mns"];
  let sns_0 = ctx["sns"];
  let msgs_0 = ctx["msgs"];
  let state_0 = ctx["state"];
  let effective_drop_0 = ctx["effective_drop"];
  let thinking_0 = ctx["thinking"];
  let enable_thinking_0 = ctx["enable_thinking"];
  let jt_0 = ctx["jt"];
  let parts_0 = ctx["parts"];
  let ts_0 = ctx["ts"];
  let fn_0 = ctx["fn"];
  let ns_0 = ctx["ns"];
  let schema_0 = ctx["schema"];
  let messages_0 = ctx["messages"];
  let content_0 = ctx["content"];
  let is_blocks_0 = ctx["is_blocks"];
  let rc_0 = ctx["rc"];
  let prev_0 = ctx["prev"];
  let tblock_0 = ctx["tblock"];
  let blocks_0 = ctx["blocks"];
  let tb_0 = ctx["tb"];
  let sorted_blocks_0 = ctx["sorted_blocks"];
  let rb_0 = ctx["rb"];
  let order_0 = ctx["order"];
  let starts_turn_0 = ctx["starts_turn"];
  let turn_0 = ctx["turn"];
  let idx_0 = ctx["idx"];
  let up_0 = ctx["up"];
  let prev_has_task_0 = ctx["prev_has_task"];
  let tcs_0 = ctx["tcs"];
  let task_0 = ctx["task"];
  let task_tokens_0 = ctx["task_tokens"];
  let tc_0 = ctx["tc"];
  let sub_0 = ctx["sub"];
  let params_0 = ctx["params"];
  let args_0 = ctx["args"];
  let name_0 = ctx["name"];
  IMG_0 = "<\uff5cdeepseek_image\uff5c>";
  DSML_0 = "\uff5cDSML\uff5c";
  if (J.t(!J.t((thinking_mode_0 !== undefined)))) {
    if (J.t(((v) => J.t(v) ? !J.t((thinking_0 === null)) : v)((thinking_0 !== undefined)))) {
      thinking_mode_0 = (J.t(thinking_0) ? "thinking" : "chat");
    } else if (J.t(((v) => J.t(v) ? !J.t((enable_thinking_0 === null)) : v)((enable_thinking_0 !== undefined)))) {
      thinking_mode_0 = (J.t(enable_thinking_0) ? "thinking" : "chat");
    } else {
      thinking_mode_0 = (J.t(((v) => J.t(v) ? (J.eq(reasoning_effort_0, "none")) : v)((reasoning_effort_0 !== undefined))) ? "chat" : "thinking");
    }
  }
  if (J.t((!J.has(thinking_mode_0, ["chat", "thinking"])))) {
    out.push(J.str(J.call(J.raise_exception, J.concat("Invalid thinking_mode `", thinking_mode_0, "`, should be \"chat\" or \"thinking\""))));
  }
  if (J.t(!J.t((drop_thinking_0 !== undefined)))) {
    drop_thinking_0 = true;
  }
  if (J.t(!J.t((add_generation_prompt_0 !== undefined)))) {
    add_generation_prompt_0 = false;
  }
  if (J.t(!J.t((tools_0 !== undefined)))) {
    tools_0 = null;
  }
  if (J.t(((v) => J.t(v) ? v : (reasoning_effort_0 === null))(!J.t((reasoning_effort_0 !== undefined))))) {
    reasoning_effort_0 = "high";
  }
  effort_map_0 = ({"none": 75, "minimal": 50, "low": 50, "medium": 75, "high": 75, "xhigh": 75, "max": 100});
  if (J.t(((v) => J.t(v) ? (J.has(reasoning_effort_0, effort_map_0)) : v)(J.test("string", reasoning_effort_0)))) {
    budget_0 = J.item(effort_map_0, reasoning_effort_0);
  } else if (J.t(((v) => J.t(v) ? ((c0, c1, c2) => J.le(c0, c1) && J.le(c1, c2))(1, reasoning_effort_0, 100) : v)(J.test("integer", reasoning_effort_0)))) {
    budget_0 = reasoning_effort_0;
  } else {
    out.push(J.str(J.call(J.raise_exception, J.concat("Invalid reasoning effort: ", reasoning_effort_0, ", should be int within [1,100] or one of \"low\", \"high\", \"max\""))));
  }
  joined_text_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_2 = k.caller;
    let blocks_1 = p.length > 0 ? p[0] : "blocks" in k ? k["blocks"] : undefined;
    let jt_3 = jt_0;
    jt_3 = J.call(J.namespace, J.kw({"parts": []}));
    {
      let _items1 = J.iter(blocks_1);
      const _loop1 = J.loop(_items1);
      for (let _i1 = 0; _i1 < _items1.length; _i1++) {
        _loop1.index0 = _i1; const _it1 = _items1[_i1]; let b_4 = _it1;
        if (J.t((J.eq(J.attr(b_4, "type"), "text")))) {
          if (J.t((J.has(IMG_0, ((v) => J.t(v) ? v : "")(J.attr(b_4, "text")))))) {
            out.push(J.str(J.call(J.raise_exception, J.concat("Text block contains image placeholder ", IMG_0, ". Images should be separate content blocks."))));
          }
          J.setattr(jt_3, "parts", J.add(J.attr(jt_3, "parts"), [((v) => J.t(v) ? v : "")(J.attr(b_4, "text"))]));
        } else if (J.t((J.has(J.attr(b_4, "type"), ["image", "image_url"])))) {
          J.setattr(jt_3, "parts", J.add(J.attr(jt_3, "parts"), [IMG_0]));
        }
      }
    }
    out.push(J.str(J.filter("join", J.attr(jt_3, "parts"), "\n\n")));
    return out.join("");
  };
  qualified_name_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_7 = k.caller;
    let name_5 = p.length > 0 ? p[0] : "name" in k ? k["name"] : undefined;
    let ns_6 = p.length > 1 ? p[1] : "ns" in k ? k["ns"] : undefined;
    let parts_8 = parts_0;
    if (J.t(J.test("mapping", ns_6))) {
      ns_6 = J.attr(ns_6, "name");
    }
    parts_8 = J.call(J.attr(name_5, "split"), "::", 1);
    if (J.t((J.eq(J.filter("length", parts_8), 2)))) {
      if (J.t(((v) => J.t(v) ? (!J.eq(ns_6, J.item(parts_8, 0))) : v)(!J.t((ns_6 === null))))) {
        out.push(J.str(J.call(J.raise_exception, J.concat("Conflicting tool namespaces: ", ns_6, " != ", J.item(parts_8, 0)))));
      }
      ns_6 = J.item(parts_8, 0);
      name_5 = J.item(parts_8, 1);
    }
    if (J.t((J.has("::", name_5)))) {
      out.push(J.str(J.call(J.raise_exception, J.concat("Tool name must not contain '::': ", name_5))));
    }
    if (J.t(((v) => J.t(v) ? (J.has("::", ns_6)) : v)(!J.t((ns_6 === null))))) {
      out.push(J.str(J.call(J.raise_exception, J.concat("Tool namespace must not contain '::': ", ns_6))));
    }
    out.push(J.str((J.t((ns_6 === null)) ? name_5 : J.concat(ns_6, "::", name_5))));
    return out.join("");
  };
  render_tools_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_10 = k.caller;
    let tool_list_9 = p.length > 0 ? p[0] : "tool_list" in k ? k["tool_list"] : undefined;
    let ts_11 = ts_0;
    ts_11 = J.call(J.namespace, J.kw({"schemas": []}));
    {
      let _items2 = J.iter(tool_list_9);
      const _loop2 = J.loop(_items2);
      for (let _i2 = 0; _i2 < _items2.length; _i2++) {
        _loop2.index0 = _i2; const _it2 = _items2[_i2]; let tool_12 = _it2;
        let fn_13 = fn_0;
        let ns_14 = ns_0;
        let schema_15 = schema_0;
        fn_13 = (J.t((J.attr(tool_12, "function") !== undefined)) ? J.attr(tool_12, "function") : tool_12);
        ns_14 = (J.t(((v) => J.t(v) ? !J.t((J.attr(tool_12, "namespace") === null)) : v)((J.attr(tool_12, "namespace") !== undefined))) ? J.attr(tool_12, "namespace") : J.call(J.attr(fn_13, "get"), "namespace"));
        schema_15 = J.call(J.namespace, J.kw({"d": ({})}));
        {
          let _items3 = J.iter(J.call(J.attr(fn_13, "items")));
          _items3 = _items3.filter(_it3 => { let [k_16, v_17] = J.unpack(_it3, 2); return J.t((!J.eq(k_16, "namespace"))); });
          const _loop3 = J.loop(_items3);
          for (let _i3 = 0; _i3 < _items3.length; _i3++) {
            _loop3.index0 = _i3; const _it3 = _items3[_i3]; let [k_16, v_17] = J.unpack(_it3, 2);
            J.setattr(schema_15, "d", J.call(J.dict, J.attr(schema_15, "d"), J.kw({...({[k_16]: v_17})})));
          }
        }
        J.setattr(schema_15, "d", J.call(J.dict, J.attr(schema_15, "d"), J.kw({"name": J.call(qualified_name_0, J.attr(fn_13, "name"), ns_14)})));
        if (J.t(((v) => J.t(v) ? J.attr(ns_14, "description") : v)(J.test("mapping", ns_14)))) {
          J.setattr(schema_15, "d", J.call(J.dict, J.attr(schema_15, "d"), J.kw({"description": J.concat(J.attr(ns_14, "description"), "\n", ((v) => J.t(v) ? v : "")(J.attr(fn_13, "description")))})));
        }
        J.setattr(ts_11, "schemas", J.add(J.attr(ts_11, "schemas"), [J.filter("tojson", J.attr(schema_15, "d"))]));
      }
    }
    out.push(J.str("## Tools\n\nYou have access to a set of tools to help answer the user's question. You can invoke tools by writing a \"<\uff5cDSML\uff5c calls>\" block like the following:\n\n<\uff5cDSML\uff5c calls>\n<\uff5cDSML\uff5c invoke name=\"$TOOL_NAME\">\n<\uff5cDSML\uff5c parameter name=\"$PARAMETER_NAME\" string=\"true|false\">$PARAMETER_VALUE</\uff5cDSML\uff5c parameter>\n...\n</\uff5cDSML\uff5c invoke>\n<\uff5cDSML\uff5c invoke name=\"$TOOL_NAME2\">\n...\n</\uff5cDSML\uff5c invoke>\n</\uff5cDSML\uff5c calls>\n\nString parameters should be specified as is and set `string=\"true\"`. For all other types (numbers, booleans, arrays, objects), pass the value in JSON format and set `string=\"false\"`.\n\nIf thinking_mode is enabled (triggered by <think>), you MUST output your complete reasoning inside <think>...</think> BEFORE any tool calls or final response.\n\nOtherwise, output directly after </think> with tool calls or final response.\n\n### Available Tool Schemas\n\n"), J.str(J.filter("join", J.attr(ts_11, "schemas"), "\n")), J.str("\n\nYou MUST strictly follow the above defined tool name and parameter schemas to invoke tool calls.\n"));
    return out.join("");
  };
  mns_0 = J.call(J.namespace, J.kw({"list": []}));
  {
    let _items4 = J.iter(messages_0);
    const _loop4 = J.loop(_items4);
    for (let _i4 = 0; _i4 < _items4.length; _i4++) {
      _loop4.index0 = _i4; const _it4 = _items4[_i4]; let msg_18 = _it4;
      let content_19 = content_0;
      let is_blocks_20 = is_blocks_0;
      let rc_21 = rc_0;
      let prev_22 = prev_0;
      let tblock_23 = tblock_0;
      let blocks_24 = blocks_0;
      content_19 = J.call(J.attr(msg_18, "get"), "content");
      if (J.t(((v) => J.t(v) ? !J.t((J.call(J.attr(msg_18, "get"), "content_blocks") === null)) : v)((J.eq(J.attr(msg_18, "role"), "user"))))) {
        content_19 = J.attr(msg_18, "content_blocks");
      } else if (J.t(((v) => J.t(v) ? !J.t(J.test("string", content_19)) : v)(((v) => J.t(v) ? J.call(J.attr(msg_18, "get"), "content_blocks") : v)((J.eq(J.attr(msg_18, "role"), "tool")))))) {
        content_19 = J.attr(msg_18, "content_blocks");
      }
      is_blocks_20 = ((v) => J.t(v) ? !J.t(J.test("mapping", content_19)) : v)(((v) => J.t(v) ? !J.t(J.test("string", content_19)) : v)(J.test("iterable", content_19)));
      if (J.t(((v) => J.t(v) ? (J.has(IMG_0, content_19)) : v)(J.test("string", content_19)))) {
        out.push(J.str(J.call(J.raise_exception, J.concat("Message content contains image special token ", IMG_0, ". Images should be provided as image content blocks."))));
      }
      rc_21 = (J.t(!J.t((J.call(J.attr(msg_18, "get"), "reasoning_content") === null))) ? J.attr(msg_18, "reasoning_content") : J.call(J.attr(msg_18, "get"), "thinking"));
      if (J.t(((v) => J.t(v) ? (J.has(IMG_0, rc_21)) : v)(J.test("string", rc_21)))) {
        out.push(J.str(J.call(J.raise_exception, J.concat("reasoning_content contains image special token ", IMG_0))));
      }
      prev_22 = (J.t(J.attr(mns_0, "list")) ? J.item(J.attr(mns_0, "list"), J.neg(1)) : null);
      if (J.t((J.eq(J.attr(msg_18, "role"), "tool")))) {
        tblock_23 = ({"type": "tool_result", "tool_use_id": J.call(J.attr(msg_18, "get"), "tool_call_id", ""), "content": (J.t(is_blocks_20) ? J.call(joined_text_0, content_19) : ((v) => J.t(v) ? v : "")(content_19))});
        if (J.t(((v) => J.t(v) ? (J.eq(J.attr(prev_22, "role"), "user")) : v)(!J.t((prev_22 === null))))) {
          J.setattr(mns_0, "list", J.add(J.slice(J.attr(mns_0, "list"), undefined, J.neg(1), undefined), [J.call(J.dict, prev_22, J.kw({"content_blocks": J.add(J.attr(prev_22, "content_blocks"), [tblock_23])}))]));
        } else {
          J.setattr(mns_0, "list", J.add(J.attr(mns_0, "list"), [({"role": "user", "content_blocks": [tblock_23]})]));
        }
      } else if (J.t((J.eq(J.attr(msg_18, "role"), "user")))) {
        blocks_24 = (J.t(is_blocks_20) ? content_19 : [({"type": "text", "text": ((v) => J.t(v) ? v : "")(content_19)})]);
        if (J.t(((v) => J.t(v) ? (J.call(J.attr(prev_22, "get"), "task") === null) : v)(((v) => J.t(v) ? (J.eq(J.attr(prev_22, "role"), "user")) : v)(!J.t((prev_22 === null)))))) {
          J.setattr(mns_0, "list", J.add(J.slice(J.attr(mns_0, "list"), undefined, J.neg(1), undefined), [J.call(J.dict, prev_22, J.kw({"content_blocks": J.add(J.attr(prev_22, "content_blocks"), blocks_24)}))]));
        } else {
          J.setattr(mns_0, "list", J.add(J.attr(mns_0, "list"), [J.call(J.dict, msg_18, J.kw({"content_blocks": blocks_24}))]));
        }
      } else {
        J.setattr(mns_0, "list", J.add(J.attr(mns_0, "list"), [msg_18]));
      }
    }
  }
  if (J.t(tools_0)) {
    if (J.t(((v) => J.t(v) ? (J.eq(J.attr(J.item(J.attr(mns_0, "list"), 0), "role"), "system")) : v)(J.attr(mns_0, "list")))) {
      if (J.t(!J.t(J.call(J.attr(J.item(J.attr(mns_0, "list"), 0), "get"), "tools")))) {
        J.setattr(mns_0, "list", J.add([J.call(J.dict, J.item(J.attr(mns_0, "list"), 0), J.kw({"tools": tools_0}))], J.slice(J.attr(mns_0, "list"), 1, undefined, undefined)));
      }
    } else {
      J.setattr(mns_0, "list", J.add([({"role": "system", "content": "", "tools": tools_0})], J.attr(mns_0, "list")));
    }
  }
  sns_0 = J.call(J.namespace, J.kw({"list": [], "ids": []}));
  {
    let _items5 = J.iter(J.attr(mns_0, "list"));
    const _loop5 = J.loop(_items5);
    for (let _i5 = 0; _i5 < _items5.length; _i5++) {
      _loop5.index0 = _i5; const _it5 = _items5[_i5]; let m_25 = _it5;
      let tb_26 = tb_0;
      let sorted_blocks_27 = sorted_blocks_0;
      let rb_28 = rb_0;
      if (J.t(((v) => J.t(v) ? J.call(J.attr(m_25, "get"), "tool_calls") : v)((J.eq(J.attr(m_25, "role"), "assistant"))))) {
        J.setattr(sns_0, "ids", []);
        {
          let _items6 = J.iter(J.attr(m_25, "tool_calls"));
          const _loop6 = J.loop(_items6);
          for (let _i6 = 0; _i6 < _items6.length; _i6++) {
            _loop6.index0 = _i6; const _it6 = _items6[_i6]; let tc_29 = _it6;
            J.setattr(sns_0, "ids", J.add(J.attr(sns_0, "ids"), [((v) => J.t(v) ? v : J.call(J.attr(J.call(J.attr(tc_29, "get"), "function", ({})), "get"), "id", ""))(J.call(J.attr(tc_29, "get"), "id"))]));
          }
        }
        J.setattr(sns_0, "list", J.add(J.attr(sns_0, "list"), [m_25]));
      } else if (J.t(((v) => J.t(v) ? J.attr(m_25, "content_blocks") : v)((J.eq(J.attr(m_25, "role"), "user"))))) {
        tb_26 = J.call(J.namespace, J.kw({"blocks": []}));
        {
          let _items7 = J.iter(J.attr(m_25, "content_blocks"));
          _items7 = _items7.filter(_it7 => { let b_30 = _it7; return J.t((J.eq(J.attr(b_30, "type"), "tool_result"))); });
          const _loop7 = J.loop(_items7);
          for (let _i7 = 0; _i7 < _items7.length; _i7++) {
            _loop7.index0 = _i7; const _it7 = _items7[_i7]; let b_30 = _it7;
            let order_31 = order_0;
            order_31 = J.call(J.namespace, J.kw({"idx": 0}));
            {
              let _items8 = J.iter(J.attr(sns_0, "ids"));
              const _loop8 = J.loop(_items8);
              for (let _i8 = 0; _i8 < _items8.length; _i8++) {
                _loop8.index0 = _i8; const _it8 = _items8[_i8]; let tc_id_32 = _it8;
                if (J.t(((v) => J.t(v) ? (J.eq(tc_id_32, J.call(J.attr(b_30, "get"), "tool_use_id", ""))) : v)(tc_id_32))) {
                  J.setattr(order_31, "idx", J.attr(_loop8, "index0"));
                }
              }
            }
            J.setattr(tb_26, "blocks", J.add(J.attr(tb_26, "blocks"), [J.call(J.dict, b_30, J.kw({"_order": J.attr(order_31, "idx")}))]));
          }
        }
        if (J.t((J.gt(J.filter("length", J.attr(tb_26, "blocks")), 1)))) {
          sorted_blocks_27 = J.filter("sort", J.attr(tb_26, "blocks"), J.kw({"attribute": "_order"}));
          rb_28 = J.call(J.namespace, J.kw({"blocks": [], "i": 0}));
          {
            let _items9 = J.iter(J.attr(m_25, "content_blocks"));
            const _loop9 = J.loop(_items9);
            for (let _i9 = 0; _i9 < _items9.length; _i9++) {
              _loop9.index0 = _i9; const _it9 = _items9[_i9]; let b_33 = _it9;
              if (J.t((J.eq(J.attr(b_33, "type"), "tool_result")))) {
                J.setattr(rb_28, "blocks", J.add(J.attr(rb_28, "blocks"), [J.item(sorted_blocks_27, J.attr(rb_28, "i"))]));
                J.setattr(rb_28, "i", J.add(J.attr(rb_28, "i"), 1));
              } else {
                J.setattr(rb_28, "blocks", J.add(J.attr(rb_28, "blocks"), [b_33]));
              }
            }
          }
          J.setattr(sns_0, "list", J.add(J.attr(sns_0, "list"), [J.call(J.dict, m_25, J.kw({"content_blocks": J.attr(rb_28, "blocks")}))]));
        } else {
          J.setattr(sns_0, "list", J.add(J.attr(sns_0, "list"), [m_25]));
        }
      } else {
        J.setattr(sns_0, "list", J.add(J.attr(sns_0, "list"), [m_25]));
      }
    }
  }
  msgs_0 = J.attr(sns_0, "list");
  state_0 = J.call(J.namespace, J.kw({"last_user": J.neg(1), "has_tools": false}));
  {
    let _items10 = J.iter(msgs_0);
    const _loop10 = J.loop(_items10);
    for (let _i10 = 0; _i10 < _items10.length; _i10++) {
      _loop10.index0 = _i10; const _it10 = _items10[_i10]; let m_34 = _it10;
      let starts_turn_35 = starts_turn_0;
      let turn_36 = turn_0;
      starts_turn_35 = ((v) => J.t(v) ? (J.gt(J.attr(_loop10, "index0"), 0)) : v)((J.eq(J.attr(m_34, "role"), "system")));
      if (J.t((J.eq(J.attr(m_34, "role"), "user")))) {
        turn_36 = J.call(J.namespace, J.kw({"found": !J.t(J.attr(m_34, "content_blocks"))}));
        {
          let _items11 = J.iter(((v) => J.t(v) ? v : [])(J.attr(m_34, "content_blocks")));
          const _loop11 = J.loop(_items11);
          for (let _i11 = 0; _i11 < _items11.length; _i11++) {
            _loop11.index0 = _i11; const _it11 = _items11[_i11]; let b_37 = _it11;
            if (J.t((J.has(J.attr(b_37, "type"), ["text", "image", "image_url"])))) {
              J.setattr(turn_36, "found", true);
            }
          }
        }
        starts_turn_35 = J.attr(turn_36, "found");
      }
      if (J.t(starts_turn_35)) {
        J.setattr(state_0, "last_user", J.attr(_loop10, "index0"));
      }
      if (J.t(J.call(J.attr(m_34, "get"), "tools"))) {
        J.setattr(state_0, "has_tools", true);
      }
    }
  }
  effective_drop_0 = ((v) => J.t(v) ? !J.t(J.attr(state_0, "has_tools")) : v)(drop_thinking_0);
  thinking_0 = (J.eq(thinking_mode_0, "thinking"));
  out.push(J.str("<\uff5cbegin\u2581of\u2581sentence\uff5c>"));
  {
    let _items12 = J.iter(msgs_0);
    const _loop12 = J.loop(_items12);
    for (let _i12 = 0; _i12 < _items12.length; _i12++) {
      _loop12.index0 = _i12; const _it12 = _items12[_i12]; let m_38 = _it12;
      let idx_39 = idx_0;
      let content_40 = content_0;
      let up_41 = up_0;
      let prev_has_task_42 = prev_has_task_0;
      let tcs_43 = tcs_0;
      let task_44 = task_0;
      let task_tokens_45 = task_tokens_0;
      idx_39 = J.attr(_loop12, "index0");
      content_40 = J.call(J.attr(m_38, "get"), "content");
      if (J.t(((v) => J.t(v) ? !J.t(J.test("mapping", content_40)) : v)(((v) => J.t(v) ? !J.t(J.test("string", content_40)) : v)(J.test("iterable", content_40))))) {
        content_40 = J.call(joined_text_0, content_40);
      }
      if (J.t((J.eq(idx_39, 0)))) {
        if (J.t(((v) => J.t(v) ? v : (J.eq(J.attr(m_38, "role"), "system")))(thinking_0))) {
          out.push(J.str("<\uff5cSystem\uff5c>"));
        }
        if (J.t(thinking_0)) {
          out.push(J.str(J.concat("Reasoning Effort: ", budget_0, " (range 1-100, the higher the value, the more thorough the reasoning)\n\n")));
        }
      }
      if (J.t((J.eq(J.attr(m_38, "role"), "system")))) {
        if (J.t((J.gt(idx_39, 0)))) {
          out.push(J.str("<\uff5cSystem\uff5c>"));
        }
        out.push(J.str(((v) => J.t(v) ? v : "")(content_40)));
        if (J.t(J.call(J.attr(m_38, "get"), "tools"))) {
          out.push(J.str(J.concat("\n\n", J.call(render_tools_0, J.attr(m_38, "tools")))));
        }
        if (J.t(J.call(J.attr(m_38, "get"), "response_format"))) {
          out.push(J.str(J.concat("\n\n## Response Format:\n\nYou MUST strictly adhere to the following schema to reply:\n", J.filter("tojson", J.attr(m_38, "response_format")))));
        }
      } else if (J.t((J.eq(J.attr(m_38, "role"), "user")))) {
        out.push(J.str("<\uff5cUser\uff5c>"));
        up_41 = J.call(J.namespace, J.kw({"parts": []}));
        {
          let _items13 = J.iter(J.attr(m_38, "content_blocks"));
          const _loop13 = J.loop(_items13);
          for (let _i13 = 0; _i13 < _items13.length; _i13++) {
            _loop13.index0 = _i13; const _it13 = _items13[_i13]; let b_46 = _it13;
            let tc_47 = tc_0;
            let sub_48 = sub_0;
            if (J.t((J.eq(J.attr(b_46, "type"), "text")))) {
              if (J.t((J.has(IMG_0, ((v) => J.t(v) ? v : "")(J.attr(b_46, "text")))))) {
                out.push(J.str(J.call(J.raise_exception, J.concat("Text block contains image placeholder ", IMG_0, ". Images should be separate content blocks."))));
              }
              J.setattr(up_41, "parts", J.add(J.attr(up_41, "parts"), [((v) => J.t(v) ? v : "")(J.attr(b_46, "text"))]));
            } else if (J.t((J.has(J.attr(b_46, "type"), ["image", "image_url"])))) {
              J.setattr(up_41, "parts", J.add(J.attr(up_41, "parts"), [IMG_0]));
            } else if (J.t((J.eq(J.attr(b_46, "type"), "tool_result")))) {
              tc_47 = J.call(J.attr(b_46, "get"), "content", "");
              if (J.t(((v) => J.t(v) ? !J.t(J.test("mapping", tc_47)) : v)(((v) => J.t(v) ? !J.t(J.test("string", tc_47)) : v)(J.test("iterable", tc_47))))) {
                sub_48 = J.call(J.namespace, J.kw({"parts": []}));
                {
                  let _items14 = J.iter(tc_47);
                  const _loop14 = J.loop(_items14);
                  for (let _i14 = 0; _i14 < _items14.length; _i14++) {
                    _loop14.index0 = _i14; const _it14 = _items14[_i14]; let s_49 = _it14;
                    if (J.t((J.eq(J.attr(s_49, "type"), "text")))) {
                      J.setattr(sub_48, "parts", J.add(J.attr(sub_48, "parts"), [J.call(J.attr(s_49, "get"), "text", "")]));
                    } else if (J.t((J.has(J.attr(s_49, "type"), ["image", "image_url"])))) {
                      J.setattr(sub_48, "parts", J.add(J.attr(sub_48, "parts"), [IMG_0]));
                    } else {
                      J.setattr(sub_48, "parts", J.add(J.attr(sub_48, "parts"), [J.concat("[Unsupported ", J.attr(s_49, "type"), "]")]));
                    }
                  }
                }
                tc_47 = J.filter("join", J.attr(sub_48, "parts"), "\n\n");
              }
              J.setattr(up_41, "parts", J.add(J.attr(up_41, "parts"), [J.concat("<tool_result>", tc_47, "</tool_result>")]));
            } else {
              J.setattr(up_41, "parts", J.add(J.attr(up_41, "parts"), [J.concat("[Unsupported ", J.attr(b_46, "type"), "]")]));
            }
          }
        }
        out.push(J.str(J.filter("join", J.attr(up_41, "parts"), "\n\n")));
      } else if (J.t((J.eq(J.attr(m_38, "role"), "latest_reminder")))) {
        out.push(J.str(J.concat("<\uff5clatest_reminder\uff5c>", content_40)));
      } else if (J.t((J.eq(J.attr(m_38, "role"), "assistant")))) {
        prev_has_task_42 = ((v) => J.t(v) ? !J.t((J.call(J.attr(J.item(msgs_0, J.sub(idx_39, 1)), "get"), "task") === null)) : v)((J.gt(idx_39, 0)));
        if (J.t(((v) => J.t(v) ? ((v) => J.t(v) ? v : (J.gt(idx_39, J.attr(state_0, "last_user"))))(!J.t(effective_drop_0)) : v)(((v) => J.t(v) ? !J.t(prev_has_task_42) : v)(thinking_0)))) {
          out.push(J.str(J.concat(((v) => J.t(v) ? v : "")((J.t(!J.t((J.call(J.attr(m_38, "get"), "reasoning_content") === null))) ? J.attr(m_38, "reasoning_content") : J.call(J.attr(m_38, "get"), "thinking"))), "</think>")));
        }
        out.push(J.str(((v) => J.t(v) ? v : "")(content_40)));
        if (J.t(J.call(J.attr(m_38, "get"), "tool_calls"))) {
          tcs_43 = J.call(J.namespace, J.kw({"lines": []}));
          {
            let _items15 = J.iter(J.attr(m_38, "tool_calls"));
            const _loop15 = J.loop(_items15);
            for (let _i15 = 0; _i15 < _items15.length; _i15++) {
              _loop15.index0 = _i15; const _it15 = _items15[_i15]; let tc_50 = _it15;
              let fn_51 = fn_0;
              let params_52 = params_0;
              let args_53 = args_0;
              let name_54 = name_0;
              fn_51 = J.attr(tc_50, "function");
              params_52 = J.call(J.namespace, J.kw({"lines": []}));
              if (J.t(J.test("string", J.attr(fn_51, "arguments")))) {
                out.push(J.str(J.call(J.raise_exception, "Tool call arguments must be a dict, not a JSON string")));
              }
              args_53 = (J.t(J.test("mapping", J.attr(fn_51, "arguments"))) ? J.attr(fn_51, "arguments") : ({"arguments": J.attr(fn_51, "arguments")}));
              {
                let _items16 = J.iter(J.call(J.attr(args_53, "items")));
                const _loop16 = J.loop(_items16);
                for (let _i16 = 0; _i16 < _items16.length; _i16++) {
                  _loop16.index0 = _i16; const _it16 = _items16[_i16]; let [k_55, v_56] = J.unpack(_it16, 2);
                  J.setattr(params_52, "lines", J.add(J.attr(params_52, "lines"), [J.concat("<", DSML_0, " parameter name=\"", k_55, "\" string=\"", (J.t(J.test("string", v_56)) ? "true" : "false"), "\">", (J.t(J.test("string", v_56)) ? v_56 : J.filter("tojson", v_56)), "</", DSML_0, " parameter>")]));
                }
              }
              name_54 = J.call(qualified_name_0, J.attr(fn_51, "name"), ((v) => J.t(v) ? v : J.call(J.attr(fn_51, "get"), "namespace"))(J.call(J.attr(tc_50, "get"), "namespace")));
              J.setattr(tcs_43, "lines", J.add(J.attr(tcs_43, "lines"), [J.concat("<", DSML_0, " invoke name=\"", name_54, "\">\n", J.filter("join", J.attr(params_52, "lines"), "\n"), "\n</", DSML_0, " invoke>")]));
            }
          }
          out.push(J.str(J.concat("\n\n<", DSML_0, " calls>\n", J.filter("join", J.attr(tcs_43, "lines"), "\n"), "\n</", DSML_0, " calls>")));
        }
        if (J.t(!J.t(J.call(J.attr(m_38, "get"), "wo_eos", false)))) {
          out.push(J.str("<\uff5cend\u2581of\u2581sentence\uff5c>"));
        }
      } else {
        out.push(J.str(J.call(J.raise_exception, J.concat("Unknown role: ", J.attr(m_38, "role")))));
      }
      if (J.t(((v) => J.t(v) ? v : (J.has(J.attr(J.item(msgs_0, J.add(idx_39, 1)), "role"), ["assistant", "latest_reminder"])))(J.attr(_loop12, "last")))) {
        task_44 = J.call(J.attr(m_38, "get"), "task");
        if (J.t(!J.t((task_44 === null)))) {
          task_tokens_45 = ({"action": "<\uff5caction\uff5c>", "query": "<\uff5cquery\uff5c>", "authority": "<\uff5cauthority\uff5c>", "domain": "<\uff5cdomain\uff5c>", "title": "<\uff5ctitle\uff5c>", "read_url": "<\uff5cread_url\uff5c>"});
          if (J.t((!J.has(task_44, task_tokens_45)))) {
            out.push(J.str(J.call(J.raise_exception, J.concat("Invalid task: '", task_44, "'"))));
          }
          if (J.t((J.eq(task_44, "action")))) {
            out.push(J.str(J.concat("<\uff5cAssistant\uff5c>", (J.t(thinking_0) ? "<think>" : "</think>"))));
          }
          out.push(J.str(J.item(task_tokens_45, task_44)));
        } else if (J.t(((v) => J.t(v) ? ((v) => J.t(v) ? v : !J.t(J.attr(_loop12, "last")))(add_generation_prompt_0) : v)(((v) => J.t(v) ? v : ((v) => J.t(v) ? (J.gt(idx_39, 0)) : v)((J.eq(J.attr(m_38, "role"), "system"))))((J.eq(J.attr(m_38, "role"), "user")))))) {
          out.push(J.str(J.concat("<\uff5cAssistant\uff5c>", (J.t(((v) => J.t(v) ? ((v) => J.t(v) ? v : (J.ge(idx_39, J.attr(state_0, "last_user"))))(!J.t(effective_drop_0)) : v)(thinking_0)) ? "<think>" : "</think>"))));
        }
      }
    }
  }
  return out.join("");
}
