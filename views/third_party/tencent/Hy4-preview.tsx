// Converted from the chat template of tencent/Hy4-preview by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let HYTK_0 = ctx["HYTK"];
  let hy_start_token_0 = ctx["hy_start_token"];
  let hy_middle_token_0 = ctx["hy_middle_token"];
  let hy_end_token_0 = ctx["hy_end_token"];
  let think_begin_token_0 = ctx["think_begin_token"];
  let think_end_token_0 = ctx["think_end_token"];
  let toolcalls_begin_token_0 = ctx["toolcalls_begin_token"];
  let toolcalls_end_token_0 = ctx["toolcalls_end_token"];
  let toolcall_begin_token_0 = ctx["toolcall_begin_token"];
  let toolcall_end_token_0 = ctx["toolcall_end_token"];
  let argkey_begin_token_0 = ctx["argkey_begin_token"];
  let argkey_end_token_0 = ctx["argkey_end_token"];
  let argvalue_begin_token_0 = ctx["argvalue_begin_token"];
  let argvalue_end_token_0 = ctx["argvalue_end_token"];
  let toolresponse_begin_token_0 = ctx["toolresponse_begin_token"];
  let toolresponse_end_token_0 = ctx["toolresponse_end_token"];
  let reasoning_mode_token_0 = ctx["reasoning_mode_token"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let preserved_thinking_0 = ctx["preserved_thinking"];
  let reasoning_effort_0 = ctx["reasoning_effort"];
  let raw_last_assistant_0 = ctx["raw_last_assistant"];
  let tool_to_json_0 = ctx["tool_to_json"];
  let render_content_0 = ctx["render_content"];
  let render_tools_prompt_0 = ctx["render_tools_prompt"];
  let render_tool_response_0 = ctx["render_tool_response"];
  let merged_ns_0 = ctx["merged_ns"];
  let merge_tool_responses_0 = ctx["merge_tool_responses"];
  let render_tool_responses_0 = ctx["render_tool_responses"];
  let __0 = ctx["_"];
  let normed_messages_0 = ctx["normed_messages"];
  let ns_0 = ctx["ns"];
  let last_ns_0 = ctx["last_ns"];
  let prev_ns_0 = ctx["prev_ns"];
  let tools_0 = ctx["tools"] ?? null;
  let fallback_strategy_0 = ctx["fallback_strategy"];
  let ns_tool_0 = ctx["ns_tool"];
  let tool_ns_0 = ctx["tool_ns"];
  let t_0 = ctx["t"];
  let content_0 = ctx["content"];
  let merge_ns_0 = ctx["merge_ns"];
  let order_ns_0 = ctx["order_ns"];
  let hit_ns_0 = ctx["hit_ns"];
  let message_id_0 = ctx["message_id"];
  let messages_0 = ctx["messages"];
  let content_body_0 = ctx["content_body"];
  let reasoning_text_0 = ctx["reasoning_text"];
  let ids_ns_0 = ctx["ids_ns"];
  let tool_call_id_0 = ctx["tool_call_id"];
  let func_0 = ctx["func"];
  let arguments_0 = ctx["arguments"];
  HYTK_0 = ":opensource";
  hy_start_token_0 = J.call(J.attr("<\uff5chy_start{}\uff5c>", "format"), HYTK_0);
  hy_middle_token_0 = J.call(J.attr("<\uff5chy_middle{}\uff5c>", "format"), HYTK_0);
  hy_end_token_0 = J.call(J.attr("<\uff5chy_end{}\uff5c>", "format"), HYTK_0);
  think_begin_token_0 = J.call(J.attr("<think{}>", "format"), HYTK_0);
  think_end_token_0 = J.call(J.attr("</think{}>", "format"), HYTK_0);
  toolcalls_begin_token_0 = J.call(J.attr("<tool_calls{}>", "format"), HYTK_0);
  toolcalls_end_token_0 = J.call(J.attr("</tool_calls{}>", "format"), HYTK_0);
  toolcall_begin_token_0 = J.call(J.attr("<tool_call{}>", "format"), HYTK_0);
  toolcall_end_token_0 = J.call(J.attr("</tool_call{}>", "format"), HYTK_0);
  argkey_begin_token_0 = J.call(J.attr("<arg_key{}>", "format"), HYTK_0);
  argkey_end_token_0 = J.call(J.attr("</arg_key{}>", "format"), HYTK_0);
  argvalue_begin_token_0 = J.call(J.attr("<arg_value{}>", "format"), HYTK_0);
  argvalue_end_token_0 = J.call(J.attr("</arg_value{}>", "format"), HYTK_0);
  toolresponse_begin_token_0 = J.call(J.attr("<tool_response{}>", "format"), HYTK_0);
  toolresponse_end_token_0 = J.call(J.attr("</tool_response{}>", "format"), HYTK_0);
  reasoning_mode_token_0 = J.call(J.attr("<\uff5creasoning_mode{}\uff5c>", "format"), HYTK_0);
  if (J.t(!J.t((add_generation_prompt_0 !== undefined)))) {
    add_generation_prompt_0 = false;
  }
  if (J.t(!J.t((preserved_thinking_0 !== undefined)))) {
    if (J.t(!J.t(tools_0))) {
      preserved_thinking_0 = false;
    } else {
      preserved_thinking_0 = true;
    }
  }
  if (J.t(!J.t((reasoning_effort_0 !== undefined)))) {
    reasoning_effort_0 = "high";
  } else if (J.t((!J.has(reasoning_effort_0, ["high", "no_think"])))) {
    if (J.t((reasoning_effort_0 === null))) {
      out.push(J.str(J.call(J.raise_exception, "reasoning_effort error : None, should be no_think/high")));
    } else {
      out.push(J.str(J.call(J.raise_exception, J.add(J.add("reasoning_effort error : ", reasoning_effort_0), ", should be no_think/high"))));
    }
  }
  if (J.t(((v) => J.t(v) ? (J.eq(fallback_strategy_0, "reasoning_toolcall_retry")) : v)((fallback_strategy_0 !== undefined)))) {
    reasoning_effort_0 = "high";
    add_generation_prompt_0 = false;
  }
  if (J.t(!J.t((raw_last_assistant_0 !== undefined)))) {
    raw_last_assistant_0 = false;
  }
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
          out.push(J.str(J.concat("\"", k_4, "\": ", J.filter("tojson", v_5, J.kw({"ensure_ascii": false})))));
        }
      }
    }
    out.push(J.str("}"));
    return out.join("");
  };
  render_content_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_7 = k.caller;
    let content_6 = p.length > 0 ? p[0] : "content" in k ? k["content"] : undefined;
    if (J.t(J.test("string", content_6))) {
      out.push(J.str(content_6));
    } else if (J.t(((v) => J.t(v) ? !J.t(J.test("mapping", content_6)) : v)(J.test("iterable", content_6)))) {
      {
        let _items2 = J.iter(content_6);
        const _loop2 = J.loop(_items2);
        for (let _i2 = 0; _i2 < _items2.length; _i2++) {
          _loop2.index0 = _i2; const _it2 = _items2[_i2]; let item_8 = _it2;
          if (J.t(((v) => J.t(v) ? (J.eq(J.attr(item_8, "type"), "text")) : v)(J.test("mapping", item_8)))) {
            out.push(J.str(J.attr(item_8, "text")));
          } else if (J.t(J.test("string", item_8))) {
            out.push(J.str(item_8));
          } else {
            out.push(J.str(J.filter("string", item_8)));
          }
        }
      }
    } else if (J.t((content_6 === null))) {
      out.push(J.str(""));
    } else {
      out.push(J.str(J.filter("string", content_6)));
    }
    return out.join("");
  };
  render_tools_prompt_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_9 = k.caller;
    let tool_ns_10 = tool_ns_0;
    out.push(J.str("# Tools\n\nYou may call one or more functions to assist with the user query."), J.str("\n\nYou are provided with function signatures within <tools></tools> XML tags:"), J.str("\n<tools>\n"));
    tool_ns_10 = J.call(J.namespace, J.kw({"first": true}));
    {
      let _items3 = J.iter(tools_0);
      const _loop3 = J.loop(_items3);
      for (let _i3 = 0; _i3 < _items3.length; _i3++) {
        _loop3.index0 = _i3; const _it3 = _items3[_i3]; let tool_11 = _it3;
        let t_12 = t_0;
        t_12 = (J.t(((v) => J.t(v) ? (J.has("function", tool_11)) : v)(J.test("mapping", tool_11))) ? J.item(tool_11, "function") : tool_11);
        if (J.t(((v) => J.t(v) ? v : !J.t(J.attr(t_12, "defer_loading")))(!J.t((J.attr(t_12, "defer_loading") !== undefined))))) {
          if (J.t(!J.t(J.attr(tool_ns_10, "first")))) {
            out.push(J.str("\n"));
          }
          J.setattr(tool_ns_10, "first", false);
          out.push(J.str(J.call(tool_to_json_0, t_12)));
        }
      }
    }
    out.push(J.str("\n</tools>\n\n"), J.str(J.concat("For function call returns, you should first print ", toolcalls_begin_token_0)), J.str("\nFor each function call, you should return object like:\n"), J.str(J.concat(toolcall_begin_token_0, "{function-name}")), J.str(J.concat(argkey_begin_token_0, "{arg-key-1}", argkey_end_token_0)), J.str(J.concat(argvalue_begin_token_0, "{arg-value-1}", argvalue_end_token_0)), J.str(J.concat(argkey_begin_token_0, "{arg-key-2}", argkey_end_token_0)), J.str(J.concat(argvalue_begin_token_0, "{arg-value-2}", argvalue_end_token_0)), J.str("..."), J.str(toolcall_end_token_0), J.str(J.concat("\nAt the end of function call returns, you should print ", toolcalls_end_token_0)));
    return out.join("");
  };
  render_tool_response_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_14 = k.caller;
    let message_13 = p.length > 0 ? p[0] : "message" in k ? k["message"] : undefined;
    let content_15 = content_0;
    content_15 = J.item(message_13, "content");
    if (J.t(J.test("string", content_15))) {
      out.push(J.str(J.concat(toolresponse_begin_token_0, content_15, toolresponse_end_token_0)));
    } else if (J.t(((v) => J.t(v) ? (J.eq(J.attr(J.item(content_15, 0), "type"), "tool_reference")) : v)(((v) => J.t(v) ? J.test("mapping", J.item(content_15, 0)) : v)(((v) => J.t(v) ? content_15 : v)(((v) => J.t(v) ? !J.t(J.test("mapping", content_15)) : v)(J.test("iterable", content_15))))))) {
      out.push(J.str(toolresponse_begin_token_0), J.str("<tools>\n"));
      {
        let _items4 = J.iter(content_15);
        const _loop4 = J.loop(_items4);
        for (let _i4 = 0; _i4 < _items4.length; _i4++) {
          _loop4.index0 = _i4; const _it4 = _items4[_i4]; let tr_16 = _it4;
          {
            let _items5 = J.iter(tools_0);
            const _loop5 = J.loop(_items5);
            for (let _i5 = 0; _i5 < _items5.length; _i5++) {
              _loop5.index0 = _i5; const _it5 = _items5[_i5]; let tool_17 = _it5;
              let t_18 = t_0;
              t_18 = (J.t(((v) => J.t(v) ? (J.has("function", tool_17)) : v)(J.test("mapping", tool_17))) ? J.item(tool_17, "function") : tool_17);
              if (J.t((J.eq(J.attr(t_18, "name"), J.attr(tr_16, "name"))))) {
                out.push(J.str(J.concat(J.call(tool_to_json_0, t_18), "\n")));
              }
            }
          }
        }
      }
      out.push(J.str("</tools>"), J.str(toolresponse_end_token_0));
    } else if (J.t(((v) => J.t(v) ? (J.attr(J.item(content_15, 0), "output") !== undefined) : v)(((v) => J.t(v) ? J.test("mapping", J.item(content_15, 0)) : v)(((v) => J.t(v) ? content_15 : v)(((v) => J.t(v) ? !J.t(J.test("mapping", content_15)) : v)(J.test("iterable", content_15))))))) {
      {
        let _items6 = J.iter(content_15);
        const _loop6 = J.loop(_items6);
        for (let _i6 = 0; _i6 < _items6.length; _i6++) {
          _loop6.index0 = _i6; const _it6 = _items6[_i6]; let tr_19 = _it6;
          out.push(J.str(J.concat(toolresponse_begin_token_0, J.attr(tr_19, "output"), toolresponse_end_token_0)));
        }
      }
    } else {
      out.push(J.str(J.concat(toolresponse_begin_token_0, J.call(render_content_0, content_15), toolresponse_end_token_0)));
    }
    return out.join("");
  };
  merged_ns_0 = J.call(J.namespace, J.kw({"messages": []}));
  merge_tool_responses_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_21 = k.caller;
    let messages_20 = p.length > 0 ? p[0] : "messages" in k ? k["messages"] : undefined;
    let merge_ns_22 = merge_ns_0;
    merge_ns_22 = J.call(J.namespace, J.kw({"result": [], "cluster": []}));
    {
      let _items7 = J.iter(messages_20);
      const _loop7 = J.loop(_items7);
      for (let _i7 = 0; _i7 < _items7.length; _i7++) {
        _loop7.index0 = _i7; const _it7 = _items7[_i7]; let message_23 = _it7;
        if (J.t((J.eq(J.item(message_23, "role"), "tool")))) {
          J.setattr(merge_ns_22, "cluster", J.add(J.attr(merge_ns_22, "cluster"), [message_23]));
        } else {
          if (J.t(J.attr(merge_ns_22, "cluster"))) {
            J.setattr(merge_ns_22, "result", J.add(J.attr(merge_ns_22, "result"), [({"role": "tool", "content": J.attr(merge_ns_22, "cluster")})]));
            J.setattr(merge_ns_22, "cluster", []);
          }
          J.setattr(merge_ns_22, "result", J.add(J.attr(merge_ns_22, "result"), [message_23]));
        }
      }
    }
    if (J.t(J.attr(merge_ns_22, "cluster"))) {
      J.setattr(merge_ns_22, "result", J.add(J.attr(merge_ns_22, "result"), [({"role": "tool", "content": J.attr(merge_ns_22, "cluster")})]));
    }
    J.setattr(merged_ns_0, "messages", J.attr(merge_ns_22, "result"));
    return out.join("");
  };
  render_tool_responses_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_26 = k.caller;
    let tool_messages_24 = p.length > 0 ? p[0] : "tool_messages" in k ? k["tool_messages"] : undefined;
    let tool_call_ids_25 = p.length > 1 ? p[1] : "tool_call_ids" in k ? k["tool_call_ids"] : undefined;
    let order_ns_27 = order_ns_0;
    order_ns_27 = J.call(J.namespace, J.kw({"ordered": [], "matched": true}));
    if (J.t(((v) => J.t(v) ? (J.eq(J.filter("length", tool_call_ids_25), J.filter("length", tool_messages_24))) : v)(tool_call_ids_25))) {
      {
        let _items8 = J.iter(tool_call_ids_25);
        const _loop8 = J.loop(_items8);
        for (let _i8 = 0; _i8 < _items8.length; _i8++) {
          _loop8.index0 = _i8; const _it8 = _items8[_i8]; let tool_call_id_28 = _it8;
          let hit_ns_29 = hit_ns_0;
          hit_ns_29 = J.call(J.namespace, J.kw({"count": 0, "message": null}));
          {
            let _items9 = J.iter(tool_messages_24);
            const _loop9 = J.loop(_items9);
            for (let _i9 = 0; _i9 < _items9.length; _i9++) {
              _loop9.index0 = _i9; const _it9 = _items9[_i9]; let tool_message_30 = _it9;
              let message_id_31 = message_id_0;
              message_id_31 = (J.t((J.item(tool_message_30, "tool_call_id") !== undefined)) ? J.item(tool_message_30, "tool_call_id") : J.item(tool_message_30, "id"));
              if (J.t(((v) => J.t(v) ? (J.eq(message_id_31, tool_call_id_28)) : v)((message_id_31 !== undefined)))) {
                J.setattr(hit_ns_29, "count", J.add(J.attr(hit_ns_29, "count"), 1));
                J.setattr(hit_ns_29, "message", tool_message_30);
              }
            }
          }
          if (J.t((J.eq(J.attr(hit_ns_29, "count"), 1)))) {
            J.setattr(order_ns_27, "ordered", J.add(J.attr(order_ns_27, "ordered"), [J.attr(hit_ns_29, "message")]));
          } else {
            J.setattr(order_ns_27, "matched", false);
          }
        }
      }
    } else {
      J.setattr(order_ns_27, "matched", false);
    }
    {
      let _items10 = J.iter((J.t(J.attr(order_ns_27, "matched")) ? J.attr(order_ns_27, "ordered") : tool_messages_24));
      const _loop10 = J.loop(_items10);
      for (let _i10 = 0; _i10 < _items10.length; _i10++) {
        _loop10.index0 = _i10; const _it10 = _items10[_i10]; let tool_message_32 = _it10;
        out.push(J.str(J.call(render_tool_response_0, tool_message_32)));
      }
    }
    return out.join("");
  };
  __0 = J.call(merge_tool_responses_0, messages_0);
  normed_messages_0 = J.attr(merged_ns_0, "messages");
  ns_0 = J.call(J.namespace, J.kw({"last_user_index": J.neg(1), "has_leading_system": false}));
  if (J.t(((v) => J.t(v) ? (J.eq(J.attr(J.item(normed_messages_0, 0), "role"), "system")) : v)(normed_messages_0))) {
    J.setattr(ns_0, "has_leading_system", true);
  }
  {
    let _items11 = J.iter(normed_messages_0);
    const _loop11 = J.loop(_items11);
    for (let _i11 = 0; _i11 < _items11.length; _i11++) {
      _loop11.index0 = _i11; const _it11 = _items11[_i11]; let message_33 = _it11;
      if (J.t((J.eq(J.item(message_33, "role"), "user")))) {
        J.setattr(ns_0, "last_user_index", J.attr(_loop11, "index0"));
      }
    }
  }
  if (J.t(!J.t(J.attr(ns_0, "has_leading_system")))) {
    out.push(J.str(J.concat(hy_start_token_0, "system", hy_middle_token_0)));
    if (J.t(tools_0)) {
      out.push(J.str(J.call(render_tools_prompt_0)));
    }
    out.push(J.str(J.concat(reasoning_mode_token_0, "reasoning_effort:", reasoning_effort_0)), J.str(hy_end_token_0));
  }
  last_ns_0 = J.call(J.namespace, J.kw({"last_is_assistant": false}));
  prev_ns_0 = J.call(J.namespace, J.kw({"tool_call_ids": []}));
  {
    let _items12 = J.iter(normed_messages_0);
    const _loop12 = J.loop(_items12);
    for (let _i12 = 0; _i12 < _items12.length; _i12++) {
      _loop12.index0 = _i12; const _it12 = _items12[_i12]; let message_34 = _it12;
      let content_35 = content_0;
      let content_body_36 = content_body_0;
      let reasoning_text_37 = reasoning_text_0;
      let ids_ns_38 = ids_ns_0;
      if (J.t((J.eq(J.item(message_34, "role"), "system")))) {
        out.push(J.str(J.concat(hy_start_token_0, "system", hy_middle_token_0)));
        if (J.t(J.attr(_loop12, "first"))) {
          if (J.t(tools_0)) {
            out.push(J.str(J.call(render_tools_prompt_0)));
          }
          content_35 = J.call(render_content_0, J.item(message_34, "content"));
          if (J.t(((v) => J.t(v) ? content_35 : v)(tools_0))) {
            out.push(J.str("\n\n"));
          }
          out.push(J.str(content_35), J.str(J.concat(reasoning_mode_token_0, "reasoning_effort:", reasoning_effort_0)));
        } else {
          out.push(J.str(J.call(render_content_0, J.item(message_34, "content"))));
        }
        out.push(J.str(hy_end_token_0));
      } else if (J.t((J.eq(J.item(message_34, "role"), "user")))) {
        out.push(J.str(J.concat(hy_start_token_0, "user", hy_middle_token_0)), J.str(J.call(render_content_0, J.item(message_34, "content"))), J.str(hy_end_token_0));
      } else if (J.t((J.eq(J.item(message_34, "role"), "assistant")))) {
        content_body_36 = J.call(render_content_0, J.item(message_34, "content"));
        reasoning_text_37 = "";
        if (J.t(((v) => J.t(v) ? J.item(message_34, "reasoning") : v)(((v) => J.t(v) ? J.test("string", J.item(message_34, "reasoning")) : v)((J.item(message_34, "reasoning") !== undefined))))) {
          reasoning_text_37 = J.item(message_34, "reasoning");
        } else if (J.t(((v) => J.t(v) ? J.item(message_34, "reasoning_content") : v)(((v) => J.t(v) ? J.test("string", J.item(message_34, "reasoning_content")) : v)((J.item(message_34, "reasoning_content") !== undefined))))) {
          reasoning_text_37 = J.item(message_34, "reasoning_content");
        }
        if (J.t(((v) => J.t(v) ? reasoning_text_37 : v)(((v) => J.t(v) ? ((v) => J.t(v) ? v : (J.gt(J.attr(_loop12, "index0"), J.attr(ns_0, "last_user_index"))))(preserved_thinking_0) : v)((!J.eq(reasoning_effort_0, "no_think")))))) {
          content_35 = J.concat(think_begin_token_0, reasoning_text_37, think_end_token_0, content_body_36);
        } else {
          content_35 = J.concat(think_begin_token_0, think_end_token_0, content_body_36);
        }
        out.push(J.str(J.concat(hy_start_token_0, "assistant", hy_middle_token_0)));
        if (J.t(((v) => J.t(v) ? J.item(message_34, "tool_calls") : v)((J.item(message_34, "tool_calls") !== undefined)))) {
          ids_ns_38 = J.call(J.namespace, J.kw({"ids": [], "complete": true}));
          {
            let _items13 = J.iter(J.item(message_34, "tool_calls"));
            const _loop13 = J.loop(_items13);
            for (let _i13 = 0; _i13 < _items13.length; _i13++) {
              _loop13.index0 = _i13; const _it13 = _items13[_i13]; let tool_39 = _it13;
              let tool_call_id_40 = tool_call_id_0;
              tool_call_id_40 = (J.t(((v) => J.t(v) ? J.item(tool_39, "id") : v)((J.item(tool_39, "id") !== undefined))) ? J.item(tool_39, "id") : J.item(tool_39, "tool_call_id"));
              if (J.t(((v) => J.t(v) ? tool_call_id_40 : v)((tool_call_id_40 !== undefined)))) {
                J.setattr(ids_ns_38, "ids", J.add(J.attr(ids_ns_38, "ids"), [tool_call_id_40]));
              } else {
                J.setattr(ids_ns_38, "complete", false);
              }
            }
          }
          J.setattr(prev_ns_0, "tool_call_ids", (J.t(J.attr(ids_ns_38, "complete")) ? J.attr(ids_ns_38, "ids") : []));
          out.push(J.str(content_35), J.str(toolcalls_begin_token_0));
          {
            let _items14 = J.iter(J.item(message_34, "tool_calls"));
            const _loop14 = J.loop(_items14);
            for (let _i14 = 0; _i14 < _items14.length; _i14++) {
              _loop14.index0 = _i14; const _it14 = _items14[_i14]; let tool_41 = _it14;
              let func_42 = func_0;
              let arguments_43 = arguments_0;
              func_42 = (J.t(((v) => J.t(v) ? (J.has("function", tool_41)) : v)(J.test("mapping", tool_41))) ? J.item(tool_41, "function") : tool_41);
              arguments_43 = J.item(func_42, "arguments");
              out.push(J.str(J.concat(toolcall_begin_token_0, J.item(func_42, "name"))));
              {
                let _items15 = J.iter(J.call(J.attr(arguments_43, "items")));
                const _loop15 = J.loop(_items15);
                for (let _i15 = 0; _i15 < _items15.length; _i15++) {
                  _loop15.index0 = _i15; const _it15 = _items15[_i15]; let [key_44, value_45] = J.unpack(_it15, 2);
                  out.push(J.str(J.concat(argkey_begin_token_0, key_44, argkey_end_token_0)));
                  if (J.t(!J.t(J.test("string", value_45)))) {
                    value_45 = J.filter("tojson", value_45, J.kw({"ensure_ascii": false}));
                  }
                  out.push(J.str(J.concat(argvalue_begin_token_0, value_45, argvalue_end_token_0)));
                }
              }
              out.push(J.str(toolcall_end_token_0));
            }
          }
          out.push(J.str(toolcalls_end_token_0));
        } else {
          if (J.t(((v) => J.t(v) ? raw_last_assistant_0 : v)(J.attr(_loop12, "last")))) {
            out.push(J.str(content_body_36));
          } else {
            out.push(J.str(content_35));
          }
        }
        if (J.t(!J.t(((v) => J.t(v) ? raw_last_assistant_0 : v)(J.attr(_loop12, "last"))))) {
          out.push(J.str(hy_end_token_0));
        }
      } else if (J.t((J.eq(J.item(message_34, "role"), "tool")))) {
        out.push(J.str(J.concat(hy_start_token_0, "tool", hy_middle_token_0)), J.str(J.call(render_tool_responses_0, J.item(message_34, "content"), J.attr(prev_ns_0, "tool_call_ids"))), J.str(hy_end_token_0));
      } else {
        out.push(J.str(J.concat(hy_start_token_0, J.item(message_34, "role"), hy_middle_token_0)), J.str(J.call(render_content_0, J.item(message_34, "content"))), J.str(hy_end_token_0));
      }
      if (J.t(((v) => J.t(v) ? v : !J.t(((v) => J.t(v) ? J.item(message_34, "tool_calls") : v)((J.item(message_34, "tool_calls") !== undefined))))((!J.eq(J.item(message_34, "role"), "assistant"))))) {
        J.setattr(prev_ns_0, "tool_call_ids", []);
      }
      if (J.t(((v) => J.t(v) ? (J.eq(J.item(message_34, "role"), "assistant")) : v)(J.attr(_loop12, "last")))) {
        J.setattr(last_ns_0, "last_is_assistant", true);
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    if (J.t(!J.t(J.attr(last_ns_0, "last_is_assistant")))) {
      if (J.t((J.eq(reasoning_effort_0, "no_think")))) {
        out.push(J.str(J.concat(hy_start_token_0, "assistant", hy_middle_token_0, think_begin_token_0, think_end_token_0)));
      } else {
        out.push(J.str(J.concat(hy_start_token_0, "assistant", hy_middle_token_0, think_begin_token_0)));
      }
    }
  }
  return out.join("");
}
