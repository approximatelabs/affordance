// Converted from the chat template of coder3101/gemma-4-31B-it-heretic by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let format_parameters_0 = ctx["format_parameters"];
  let format_function_declaration_0 = ctx["format_function_declaration"];
  let format_argument_0 = ctx["format_argument"];
  let strip_thinking_0 = ctx["strip_thinking"];
  let format_tool_response_block_0 = ctx["format_tool_response_block"];
  let ns_0 = ctx["ns"];
  let loop_messages_0 = ctx["loop_messages"];
  let ns_turn_0 = ctx["ns_turn"];
  let standard_keys_0 = ctx["standard_keys"];
  let add_comma_0 = ctx["add_comma"];
  let ns_items_0 = ctx["ns_items"];
  let params_0 = ctx["params"];
  let response_declaration_0 = ctx["response_declaration"];
  let messages_0 = ctx["messages"];
  let bos_token_0 = ctx["bos_token"];
  let tools_0 = ctx["tools"] ?? null;
  let enable_thinking_0 = ctx["enable_thinking"];
  let role_0 = ctx["role"];
  let prev_nt_0 = ctx["prev_nt"];
  let continue_same_model_turn_0 = ctx["continue_same_model_turn"];
  let thinking_text_0 = ctx["thinking_text"];
  let ns_tr_out_0 = ctx["ns_tr_out"];
  let ns_tool_scan_0 = ctx["ns_tool_scan"];
  let function_0 = ctx["function"];
  let ns_args_0 = ctx["ns_args"];
  let follow_0 = ctx["follow"];
  let ns_tname_0 = ctx["ns_tname"];
  let tool_body_0 = ctx["tool_body"];
  let ns_txt_0 = ctx["ns_txt"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  format_parameters_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_3 = k.caller;
    let properties_1 = p.length > 0 ? p[0] : "properties" in k ? k["properties"] : undefined;
    let required_2 = p.length > 1 ? p[1] : "required" in k ? k["required"] : undefined;
    let standard_keys_4 = standard_keys_0;
    let ns_5 = ns_0;
    standard_keys_4 = ["description", "type", "properties", "required", "nullable"];
    ns_5 = J.call(J.namespace, J.kw({"found_first": false}));
    {
      let _items1 = J.iter(J.filter("dictsort", properties_1));
      const _loop1 = J.loop(_items1);
      for (let _i1 = 0; _i1 < _items1.length; _i1++) {
        _loop1.index0 = _i1; const _it1 = _items1[_i1]; let [key_6, value_7] = J.unpack(_it1, 2);
        let add_comma_8 = add_comma_0;
        let ns_items_9 = ns_items_0;
        add_comma_8 = false;
        if (J.t((!J.has(key_6, standard_keys_4)))) {
          if (J.t(J.attr(ns_5, "found_first"))) {
            out.push(",");
          }
          J.setattr(ns_5, "found_first", true);
          out.push(J.str(key_6), ":{");
          if (J.t(J.item(value_7, "description"))) {
            out.push("description:<|\"|>", J.str(J.item(value_7, "description")), "<|\"|>");
            add_comma_8 = true;
          }
          if (J.t((J.eq(J.filter("upper", J.item(value_7, "type")), "STRING")))) {
            if (J.t(J.item(value_7, "enum"))) {
              if (J.t(add_comma_8)) {
                out.push(",");
              } else {
                add_comma_8 = true;
              }
              out.push("enum:", J.str(J.call(format_argument_0, J.item(value_7, "enum"))));
            }
          } else if (J.t((J.eq(J.filter("upper", J.item(value_7, "type")), "ARRAY")))) {
            if (J.t(((v) => J.t(v) ? J.item(value_7, "items") : v)(J.test("mapping", J.item(value_7, "items"))))) {
              if (J.t(add_comma_8)) {
                out.push(",");
              } else {
                add_comma_8 = true;
              }
              out.push("items:{");
              ns_items_9 = J.call(J.namespace, J.kw({"found_first": false}));
              {
                let _items2 = J.iter(J.filter("dictsort", J.item(value_7, "items")));
                const _loop2 = J.loop(_items2);
                for (let _i2 = 0; _i2 < _items2.length; _i2++) {
                  _loop2.index0 = _i2; const _it2 = _items2[_i2]; let [item_key_10, item_value_11] = J.unpack(_it2, 2);
                  if (J.t(!J.t((item_value_11 === null)))) {
                    if (J.t(J.attr(ns_items_9, "found_first"))) {
                      out.push(",");
                    }
                    J.setattr(ns_items_9, "found_first", true);
                    if (J.t((J.eq(item_key_10, "properties")))) {
                      out.push("properties:{");
                      if (J.t(J.test("mapping", item_value_11))) {
                        out.push(J.str(J.call(format_parameters_0, item_value_11, J.filter("default", J.item(J.item(value_7, "items"), "required"), []))));
                      }
                      out.push("}");
                    } else if (J.t((J.eq(item_key_10, "required")))) {
                      out.push("required:[");
                      {
                        let _items3 = J.iter(item_value_11);
                        const _loop3 = J.loop(_items3);
                        for (let _i3 = 0; _i3 < _items3.length; _i3++) {
                          _loop3.index0 = _i3; const _it3 = _items3[_i3]; let req_item_12 = _it3;
                          out.push("<|\"|>", J.str(req_item_12), "<|\"|>");
                          if (J.t(!J.t(J.attr(_loop3, "last")))) {
                            out.push(",");
                          }
                        }
                      }
                      out.push("]");
                    } else if (J.t((J.eq(item_key_10, "type")))) {
                      if (J.t(J.test("string", item_value_11))) {
                        out.push("type:", J.str(J.call(format_argument_0, J.filter("upper", item_value_11))));
                      } else {
                        out.push("type:", J.str(J.call(format_argument_0, J.filter("list", J.filter("map", item_value_11, "upper")))));
                      }
                    } else {
                      out.push(J.str(item_key_10), ":", J.str(J.call(format_argument_0, item_value_11)));
                    }
                  }
                }
              }
              out.push("}");
            }
          }
          if (J.t(J.item(value_7, "nullable"))) {
            if (J.t(add_comma_8)) {
              out.push(",");
            } else {
              add_comma_8 = true;
            }
            out.push("nullable:true");
          }
          if (J.t((J.eq(J.filter("upper", J.item(value_7, "type")), "OBJECT")))) {
            if (J.t(((v) => J.t(v) ? J.test("mapping", J.item(value_7, "properties")) : v)((J.item(value_7, "properties") !== undefined)))) {
              if (J.t(add_comma_8)) {
                out.push(",");
              } else {
                add_comma_8 = true;
              }
              out.push("properties:{", J.str(J.call(format_parameters_0, J.item(value_7, "properties"), J.filter("default", J.item(value_7, "required"), []))), "}");
            } else if (J.t(J.test("mapping", value_7))) {
              if (J.t(add_comma_8)) {
                out.push(",");
              } else {
                add_comma_8 = true;
              }
              out.push("properties:{", J.str(J.call(format_parameters_0, value_7, J.filter("default", J.item(value_7, "required"), []))), "}");
            }
            if (J.t(J.item(value_7, "required"))) {
              if (J.t(add_comma_8)) {
                out.push(",");
              } else {
                add_comma_8 = true;
              }
              out.push("required:[");
              {
                let _items4 = J.iter(J.filter("default", J.item(value_7, "required"), []));
                const _loop4 = J.loop(_items4);
                for (let _i4 = 0; _i4 < _items4.length; _i4++) {
                  _loop4.index0 = _i4; const _it4 = _items4[_i4]; let item_13 = _it4;
                  out.push("<|\"|>", J.str(item_13), "<|\"|>");
                  if (J.t(!J.t(J.attr(_loop4, "last")))) {
                    out.push(",");
                  }
                }
              }
              out.push("]");
            }
          }
          if (J.t(add_comma_8)) {
            out.push(",");
          } else {
            add_comma_8 = true;
          }
          out.push("type:<|\"|>", J.str(J.filter("upper", J.item(value_7, "type"))), "<|\"|>}");
        }
      }
    }
    return out.join("");
  };
  format_function_declaration_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_15 = k.caller;
    let tool_data_14 = p.length > 0 ? p[0] : "tool_data" in k ? k["tool_data"] : undefined;
    let params_16 = params_0;
    let response_declaration_17 = response_declaration_0;
    out.push("declaration:", J.str(J.item(J.item(tool_data_14, "function"), "name")), "{description:<|\"|>", J.str(J.item(J.item(tool_data_14, "function"), "description")), "<|\"|>");
    params_16 = J.item(J.item(tool_data_14, "function"), "parameters");
    if (J.t(params_16)) {
      out.push(",parameters:{");
      if (J.t(J.item(params_16, "properties"))) {
        out.push("properties:{", J.str(J.call(format_parameters_0, J.item(params_16, "properties"), J.item(params_16, "required"))), "},");
      }
      if (J.t(J.item(params_16, "required"))) {
        out.push("required:[");
        {
          let _items5 = J.iter(J.item(params_16, "required"));
          const _loop5 = J.loop(_items5);
          for (let _i5 = 0; _i5 < _items5.length; _i5++) {
            _loop5.index0 = _i5; const _it5 = _items5[_i5]; let item_18 = _it5;
            out.push("<|\"|>", J.str(item_18), "<|\"|>", J.str((J.t(!J.t(J.attr(_loop5, "last"))) ? "," : undefined)));
          }
        }
        out.push("],");
      }
      if (J.t(J.item(params_16, "type"))) {
        out.push("type:<|\"|>", J.str(J.filter("upper", J.item(params_16, "type"))), "<|\"|>}");
      }
    }
    if (J.t((J.has("response", J.item(tool_data_14, "function"))))) {
      response_declaration_17 = J.item(J.item(tool_data_14, "function"), "response");
      out.push(",response:{");
      if (J.t(J.item(response_declaration_17, "description"))) {
        out.push("description:<|\"|>", J.str(J.item(response_declaration_17, "description")), "<|\"|>,");
      }
      if (J.t((J.eq(J.filter("upper", J.item(response_declaration_17, "type")), "OBJECT")))) {
        out.push("type:<|\"|>", J.str(J.filter("upper", J.item(response_declaration_17, "type"))), "<|\"|>}");
      }
    }
    out.push("}");
    return out.join("");
  };
  format_argument_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_21 = k.caller;
    let argument_19 = p.length > 0 ? p[0] : "argument" in k ? k["argument"] : undefined;
    let escape_keys_20 = p.length > 1 ? p[1] : "escape_keys" in k ? k["escape_keys"] : true;
    let ns_22 = ns_0;
    if (J.t(J.test("string", argument_19))) {
      out.push(J.str(J.add(J.add("<|\"|>", argument_19), "<|\"|>")));
    } else if (J.t(J.test("boolean", argument_19))) {
      out.push(J.str((J.t(argument_19) ? "true" : "false")));
    } else if (J.t(J.test("mapping", argument_19))) {
      out.push(J.str("{"));
      ns_22 = J.call(J.namespace, J.kw({"found_first": false}));
      {
        let _items6 = J.iter(J.filter("dictsort", argument_19));
        const _loop6 = J.loop(_items6);
        for (let _i6 = 0; _i6 < _items6.length; _i6++) {
          _loop6.index0 = _i6; const _it6 = _items6[_i6]; let [key_23, value_24] = J.unpack(_it6, 2);
          if (J.t(J.attr(ns_22, "found_first"))) {
            out.push(",");
          }
          J.setattr(ns_22, "found_first", true);
          if (J.t(escape_keys_20)) {
            out.push(J.str(J.add(J.add("<|\"|>", key_23), "<|\"|>")));
          } else {
            out.push(J.str(key_23));
          }
          out.push(":", J.str(J.call(format_argument_0, value_24, J.kw({"escape_keys": escape_keys_20}))));
        }
      }
      out.push(J.str("}"));
    } else if (J.t(J.test("sequence", argument_19))) {
      out.push(J.str("["));
      {
        let _items7 = J.iter(argument_19);
        const _loop7 = J.loop(_items7);
        for (let _i7 = 0; _i7 < _items7.length; _i7++) {
          _loop7.index0 = _i7; const _it7 = _items7[_i7]; let item_25 = _it7;
          out.push(J.str(J.call(format_argument_0, item_25, J.kw({"escape_keys": escape_keys_20}))));
          if (J.t(!J.t(J.attr(_loop7, "last")))) {
            out.push(",");
          }
        }
      }
      out.push(J.str("]"));
    } else {
      out.push(J.str(argument_19));
    }
    return out.join("");
  };
  strip_thinking_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_27 = k.caller;
    let text_26 = p.length > 0 ? p[0] : "text" in k ? k["text"] : undefined;
    let ns_28 = ns_0;
    ns_28 = J.call(J.namespace, J.kw({"result": ""}));
    {
      let _items8 = J.iter(J.call(J.attr(text_26, "split"), "<channel|>"));
      const _loop8 = J.loop(_items8);
      for (let _i8 = 0; _i8 < _items8.length; _i8++) {
        _loop8.index0 = _i8; const _it8 = _items8[_i8]; let part_29 = _it8;
        if (J.t((J.has("<|channel>", part_29)))) {
          J.setattr(ns_28, "result", J.add(J.attr(ns_28, "result"), J.item(J.call(J.attr(part_29, "split"), "<|channel>"), 0)));
        } else {
          J.setattr(ns_28, "result", J.add(J.attr(ns_28, "result"), part_29));
        }
      }
    }
    out.push(J.str(J.filter("trim", J.attr(ns_28, "result"))));
    return out.join("");
  };
  format_tool_response_block_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_32 = k.caller;
    let tool_name_30 = p.length > 0 ? p[0] : "tool_name" in k ? k["tool_name"] : undefined;
    let response_31 = p.length > 1 ? p[1] : "response" in k ? k["response"] : undefined;
    out.push(J.str("<|tool_response>"));
    if (J.t(J.test("mapping", response_31))) {
      out.push(J.str(J.add(J.add("response:", tool_name_30), "{")));
      {
        let _items9 = J.iter(J.filter("dictsort", response_31));
        const _loop9 = J.loop(_items9);
        for (let _i9 = 0; _i9 < _items9.length; _i9++) {
          _loop9.index0 = _i9; const _it9 = _items9[_i9]; let [key_33, value_34] = J.unpack(_it9, 2);
          out.push(J.str(key_33), ":", J.str(J.call(format_argument_0, value_34, J.kw({"escape_keys": false}))));
          if (J.t(!J.t(J.attr(_loop9, "last")))) {
            out.push(",");
          }
        }
      }
      out.push(J.str("}"));
    } else {
      out.push(J.str(J.add(J.add(J.add(J.add("response:", tool_name_30), "{value:"), J.call(format_argument_0, response_31, J.kw({"escape_keys": false}))), "}")));
    }
    out.push(J.str("<tool_response|>"));
    return out.join("");
  };
  ns_0 = J.call(J.namespace, J.kw({"prev_message_type": null}));
  loop_messages_0 = messages_0;
  out.push(J.str(bos_token_0));
  if (J.t(((v) => J.t(v) ? v : (J.has(J.item(J.item(messages_0, 0), "role"), ["system", "developer"])))(((v) => J.t(v) ? v : tools_0)(((v) => J.t(v) ? enable_thinking_0 : v)((enable_thinking_0 !== undefined)))))) {
    out.push(J.str("<|turn>system\n"));
    if (J.t(((v) => J.t(v) ? enable_thinking_0 : v)((enable_thinking_0 !== undefined)))) {
      out.push(J.str("<|think|>\n"));
      J.setattr(ns_0, "prev_message_type", "think");
    }
    if (J.t((J.has(J.item(J.item(messages_0, 0), "role"), ["system", "developer"])))) {
      out.push(J.str(J.filter("trim", J.item(J.item(messages_0, 0), "content"))));
      loop_messages_0 = J.slice(messages_0, 1, undefined, undefined);
    }
    if (J.t(tools_0)) {
      {
        let _items10 = J.iter(tools_0);
        const _loop10 = J.loop(_items10);
        for (let _i10 = 0; _i10 < _items10.length; _i10++) {
          _loop10.index0 = _i10; const _it10 = _items10[_i10]; let tool_35 = _it10;
          out.push(J.str("<|tool>"), J.str(J.filter("trim", J.call(format_function_declaration_0, tool_35))), J.str("<tool|>"));
        }
      }
      J.setattr(ns_0, "prev_message_type", "tool");
    }
    out.push(J.str("<turn|>\n"));
  }
  ns_turn_0 = J.call(J.namespace, J.kw({"last_user_idx": J.neg(1)}));
  {
    let _items11 = J.iter(J.call(J.range, J.filter("length", loop_messages_0)));
    const _loop11 = J.loop(_items11);
    for (let _i11 = 0; _i11 < _items11.length; _i11++) {
      _loop11.index0 = _i11; const _it11 = _items11[_i11]; let i_36 = _it11;
      if (J.t((J.eq(J.item(J.item(loop_messages_0, i_36), "role"), "user")))) {
        J.setattr(ns_turn_0, "last_user_idx", i_36);
      }
    }
  }
  {
    let _items12 = J.iter(loop_messages_0);
    const _loop12 = J.loop(_items12);
    for (let _i12 = 0; _i12 < _items12.length; _i12++) {
      _loop12.index0 = _i12; const _it12 = _items12[_i12]; let message_37 = _it12;
      let role_38 = role_0;
      let prev_nt_39 = prev_nt_0;
      let continue_same_model_turn_40 = continue_same_model_turn_0;
      let thinking_text_41 = thinking_text_0;
      let ns_tr_out_42 = ns_tr_out_0;
      let ns_tool_scan_43 = ns_tool_scan_0;
      if (J.t((!J.eq(J.item(message_37, "role"), "tool")))) {
        J.setattr(ns_0, "prev_message_type", null);
        role_38 = (J.t((J.eq(J.item(message_37, "role"), "assistant"))) ? "model" : J.item(message_37, "role"));
        prev_nt_39 = J.call(J.namespace, J.kw({"role": null, "found": false}));
        if (J.t((J.gt(J.attr(_loop12, "index0"), 0)))) {
          {
            let _items13 = J.iter(J.call(J.range, J.sub(J.attr(_loop12, "index0"), 1), J.neg(1), J.neg(1)));
            const _loop13 = J.loop(_items13);
            for (let _i13 = 0; _i13 < _items13.length; _i13++) {
              _loop13.index0 = _i13; const _it13 = _items13[_i13]; let j_44 = _it13;
              if (J.t(!J.t(J.attr(prev_nt_39, "found")))) {
                if (J.t((!J.eq(J.item(J.item(loop_messages_0, j_44), "role"), "tool")))) {
                  J.setattr(prev_nt_39, "role", J.item(J.item(loop_messages_0, j_44), "role"));
                  J.setattr(prev_nt_39, "found", true);
                }
              }
            }
          }
        }
        continue_same_model_turn_40 = ((v) => J.t(v) ? (J.eq(J.attr(prev_nt_39, "role"), "assistant")) : v)((J.eq(role_38, "model")));
        if (J.t(!J.t(continue_same_model_turn_40))) {
          out.push(J.str(J.add(J.add("<|turn>", role_38), "\n")));
        }
        thinking_text_41 = ((v) => J.t(v) ? v : J.call(J.attr(message_37, "get"), "reasoning_content"))(J.call(J.attr(message_37, "get"), "reasoning"));
        if (J.t(((v) => J.t(v) ? J.call(J.attr(message_37, "get"), "tool_calls") : v)(((v) => J.t(v) ? (J.gt(J.attr(_loop12, "index0"), J.attr(ns_turn_0, "last_user_idx"))) : v)(thinking_text_41)))) {
          out.push(J.str(J.add(J.add("<|channel>thought\n", thinking_text_41), "\n<channel|>")));
        }
        if (J.t(J.item(message_37, "tool_calls"))) {
          {
            let _items14 = J.iter(J.item(message_37, "tool_calls"));
            const _loop14 = J.loop(_items14);
            for (let _i14 = 0; _i14 < _items14.length; _i14++) {
              _loop14.index0 = _i14; const _it14 = _items14[_i14]; let tool_call_45 = _it14;
              let function_46 = function_0;
              let ns_args_47 = ns_args_0;
              function_46 = J.item(tool_call_45, "function");
              out.push(J.str(J.add(J.add("<|tool_call>call:", J.item(function_46, "name")), "{")));
              if (J.t(J.test("mapping", J.item(function_46, "arguments")))) {
                ns_args_47 = J.call(J.namespace, J.kw({"found_first": false}));
                {
                  let _items15 = J.iter(J.filter("dictsort", J.item(function_46, "arguments")));
                  const _loop15 = J.loop(_items15);
                  for (let _i15 = 0; _i15 < _items15.length; _i15++) {
                    _loop15.index0 = _i15; const _it15 = _items15[_i15]; let [key_48, value_49] = J.unpack(_it15, 2);
                    if (J.t(J.attr(ns_args_47, "found_first"))) {
                      out.push(",");
                    }
                    J.setattr(ns_args_47, "found_first", true);
                    out.push(J.str(key_48), ":", J.str(J.call(format_argument_0, value_49, J.kw({"escape_keys": false}))));
                  }
                }
              } else if (J.t(J.test("string", J.item(function_46, "arguments")))) {
                out.push(J.str(J.item(function_46, "arguments")));
              }
              out.push(J.str("}<tool_call|>"));
            }
          }
          J.setattr(ns_0, "prev_message_type", "tool_call");
        }
        ns_tr_out_42 = J.call(J.namespace, J.kw({"flag": false}));
        if (J.t(J.call(J.attr(message_37, "get"), "tool_responses"))) {
          {
            let _items16 = J.iter(J.item(message_37, "tool_responses"));
            const _loop16 = J.loop(_items16);
            for (let _i16 = 0; _i16 < _items16.length; _i16++) {
              _loop16.index0 = _i16; const _it16 = _items16[_i16]; let tool_response_50 = _it16;
              out.push(J.str(J.call(format_tool_response_block_0, J.filter("default", J.item(tool_response_50, "name"), "unknown"), J.item(tool_response_50, "response"))));
              J.setattr(ns_tr_out_42, "flag", true);
              J.setattr(ns_0, "prev_message_type", "tool_response");
            }
          }
        } else if (J.t(J.call(J.attr(message_37, "get"), "tool_calls"))) {
          ns_tool_scan_43 = J.call(J.namespace, J.kw({"stopped": false}));
          {
            let _items17 = J.iter(J.call(J.range, J.add(J.attr(_loop12, "index0"), 1), J.filter("length", loop_messages_0)));
            const _loop17 = J.loop(_items17);
            for (let _i17 = 0; _i17 < _items17.length; _i17++) {
              _loop17.index0 = _i17; const _it17 = _items17[_i17]; let k_51 = _it17;
              let follow_52 = follow_0;
              let ns_tname_53 = ns_tname_0;
              let tool_body_54 = tool_body_0;
              let ns_txt_55 = ns_txt_0;
              if (J.t(J.attr(ns_tool_scan_43, "stopped"))) {
              } else if (J.t((!J.eq(J.item(J.item(loop_messages_0, k_51), "role"), "tool")))) {
                J.setattr(ns_tool_scan_43, "stopped", true);
              } else {
                follow_52 = J.item(loop_messages_0, k_51);
                ns_tname_53 = J.call(J.namespace, J.kw({"name": J.filter("default", J.call(J.attr(follow_52, "get"), "name"), "unknown")}));
                {
                  let _items18 = J.iter(J.item(message_37, "tool_calls"));
                  const _loop18 = J.loop(_items18);
                  for (let _i18 = 0; _i18 < _items18.length; _i18++) {
                    _loop18.index0 = _i18; const _it18 = _items18[_i18]; let tc_56 = _it18;
                    if (J.t((J.eq(J.call(J.attr(tc_56, "get"), "id"), J.call(J.attr(follow_52, "get"), "tool_call_id"))))) {
                      J.setattr(ns_tname_53, "name", J.item(J.item(tc_56, "function"), "name"));
                    }
                  }
                }
                tool_body_54 = J.call(J.attr(follow_52, "get"), "content");
                if (J.t(J.test("string", tool_body_54))) {
                  out.push(J.str(J.call(format_tool_response_block_0, J.attr(ns_tname_53, "name"), tool_body_54)));
                } else if (J.t(((v) => J.t(v) ? !J.t(J.test("string", tool_body_54)) : v)(J.test("sequence", tool_body_54)))) {
                  ns_txt_55 = J.call(J.namespace, J.kw({"s": ""}));
                  {
                    let _items19 = J.iter(tool_body_54);
                    const _loop19 = J.loop(_items19);
                    for (let _i19 = 0; _i19 < _items19.length; _i19++) {
                      _loop19.index0 = _i19; const _it19 = _items19[_i19]; let part_57 = _it19;
                      if (J.t((J.eq(J.call(J.attr(part_57, "get"), "type"), "text")))) {
                        J.setattr(ns_txt_55, "s", J.add(J.attr(ns_txt_55, "s"), J.filter("default", J.call(J.attr(part_57, "get"), "text"), "")));
                      }
                    }
                  }
                  out.push(J.str(J.call(format_tool_response_block_0, J.attr(ns_tname_53, "name"), J.attr(ns_txt_55, "s"))));
                } else {
                  out.push(J.str(J.call(format_tool_response_block_0, J.attr(ns_tname_53, "name"), tool_body_54)));
                }
                J.setattr(ns_tr_out_42, "flag", true);
                J.setattr(ns_0, "prev_message_type", "tool_response");
              }
            }
          }
        }
        if (J.t(J.test("string", J.item(message_37, "content")))) {
          if (J.t((J.eq(role_38, "model")))) {
            out.push(J.str(J.call(strip_thinking_0, J.item(message_37, "content"))));
          } else {
            out.push(J.str(J.filter("trim", J.item(message_37, "content"))));
          }
        } else if (J.t(J.test("sequence", J.item(message_37, "content")))) {
          {
            let _items20 = J.iter(J.item(message_37, "content"));
            const _loop20 = J.loop(_items20);
            for (let _i20 = 0; _i20 < _items20.length; _i20++) {
              _loop20.index0 = _i20; const _it20 = _items20[_i20]; let item_58 = _it20;
              if (J.t((J.eq(J.item(item_58, "type"), "text")))) {
                if (J.t((J.eq(role_38, "model")))) {
                  out.push(J.str(J.call(strip_thinking_0, J.item(item_58, "text"))));
                } else {
                  out.push(J.str(J.filter("trim", J.item(item_58, "text"))));
                }
              } else if (J.t((J.eq(J.item(item_58, "type"), "image")))) {
                out.push(J.str("<|image|>"));
                J.setattr(ns_0, "prev_message_type", "image");
              } else if (J.t((J.eq(J.item(item_58, "type"), "audio")))) {
                out.push(J.str("<|audio|>"));
                J.setattr(ns_0, "prev_message_type", "audio");
              } else if (J.t((J.eq(J.item(item_58, "type"), "video")))) {
                out.push(J.str("<|video|>"));
                J.setattr(ns_0, "prev_message_type", "video");
              }
            }
          }
        }
        if (J.t(((v) => J.t(v) ? !J.t(J.attr(ns_tr_out_42, "flag")) : v)((J.eq(J.attr(ns_0, "prev_message_type"), "tool_call"))))) {
          out.push(J.str("<|tool_response>"));
        } else if (J.t(!J.t(((v) => J.t(v) ? !J.t(J.call(J.attr(message_37, "get"), "content")) : v)(J.attr(ns_tr_out_42, "flag"))))) {
          out.push(J.str("<turn|>\n"));
        }
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    if (J.t(((v) => J.t(v) ? (!J.eq(J.attr(ns_0, "prev_message_type"), "tool_call")) : v)((!J.eq(J.attr(ns_0, "prev_message_type"), "tool_response"))))) {
      out.push(J.str("<|turn>model\n"));
      if (J.t(!J.t(J.filter("default", enable_thinking_0, false)))) {
        out.push(J.str("<|channel>thought\n<channel|>"));
      }
    }
  }
  return out.join("");
}
