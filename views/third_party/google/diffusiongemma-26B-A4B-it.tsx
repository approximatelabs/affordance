// Converted from the chat template of google/diffusiongemma-26B-A4B-it by tools/jinja2tsx.py. Regenerate it; do not edit.
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
  let enable_thinking_0 = ctx["enable_thinking"];
  let preserve_thinking_0 = ctx["preserve_thinking"];
  let ns_turn_0 = ctx["ns_turn"];
  let standard_keys_0 = ctx["standard_keys"];
  let add_comma_0 = ctx["add_comma"];
  let ns_items_0 = ctx["ns_items"];
  let params_0 = ctx["params"];
  let response_declaration_0 = ctx["response_declaration"];
  let messages_0 = ctx["messages"];
  let bos_token_0 = ctx["bos_token"];
  let tools_0 = ctx["tools"] ?? null;
  let role_0 = ctx["role"];
  let continue_same_model_turn_0 = ctx["continue_same_model_turn"];
  let thinking_text_0 = ctx["thinking_text"];
  let thinking_gate_0 = ctx["thinking_gate"];
  let ns_tr_out_0 = ctx["ns_tr_out"];
  let ns_tool_scan_0 = ctx["ns_tool_scan"];
  let captured_content_0 = ctx["captured_content"];
  let has_content_0 = ctx["has_content"];
  let next_nt_0 = ctx["next_nt"];
  let continues_into_next_0 = ctx["continues_into_next"];
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
    const caller_4 = k.caller;
    let properties_1 = p.length > 0 ? p[0] : "properties" in k ? k["properties"] : undefined;
    let required_2 = p.length > 1 ? p[1] : "required" in k ? k["required"] : undefined;
    let filter_keys_3 = p.length > 2 ? p[2] : "filter_keys" in k ? k["filter_keys"] : false;
    let standard_keys_5 = standard_keys_0;
    let ns_6 = ns_0;
    standard_keys_5 = ["description", "type", "properties", "required", "nullable"];
    ns_6 = J.call(J.namespace, J.kw({"found_first": false}));
    {
      let _items1 = J.iter(J.filter("dictsort", properties_1));
      const _loop1 = J.loop(_items1);
      for (let _i1 = 0; _i1 < _items1.length; _i1++) {
        _loop1.index0 = _i1; const _it1 = _items1[_i1]; let [key_7, value_8] = J.unpack(_it1, 2);
        let add_comma_9 = add_comma_0;
        let ns_items_10 = ns_items_0;
        add_comma_9 = false;
        if (J.t(((v) => J.t(v) ? v : (!J.has(key_7, standard_keys_5)))(!J.t(filter_keys_3)))) {
          if (J.t(J.attr(ns_6, "found_first"))) {
            out.push(",");
          }
          J.setattr(ns_6, "found_first", true);
          out.push(J.str(key_7), ":{");
          if (J.t(J.item(value_8, "description"))) {
            out.push("description:<|\"|>", J.str(J.item(value_8, "description")), "<|\"|>");
            add_comma_9 = true;
          }
          if (J.t((J.eq(J.filter("upper", J.item(value_8, "type")), "STRING")))) {
            if (J.t(J.item(value_8, "enum"))) {
              if (J.t(add_comma_9)) {
                out.push(",");
              } else {
                add_comma_9 = true;
              }
              out.push("enum:", J.str(J.call(format_argument_0, J.item(value_8, "enum"))));
            }
          } else if (J.t((J.eq(J.filter("upper", J.item(value_8, "type")), "ARRAY")))) {
            if (J.t(((v) => J.t(v) ? J.item(value_8, "items") : v)(J.test("mapping", J.item(value_8, "items"))))) {
              if (J.t(add_comma_9)) {
                out.push(",");
              } else {
                add_comma_9 = true;
              }
              out.push("items:{");
              ns_items_10 = J.call(J.namespace, J.kw({"found_first": false}));
              {
                let _items2 = J.iter(J.filter("dictsort", J.item(value_8, "items")));
                const _loop2 = J.loop(_items2);
                for (let _i2 = 0; _i2 < _items2.length; _i2++) {
                  _loop2.index0 = _i2; const _it2 = _items2[_i2]; let [item_key_11, item_value_12] = J.unpack(_it2, 2);
                  if (J.t(!J.t((item_value_12 === null)))) {
                    if (J.t(J.attr(ns_items_10, "found_first"))) {
                      out.push(",");
                    }
                    J.setattr(ns_items_10, "found_first", true);
                    if (J.t((J.eq(item_key_11, "properties")))) {
                      out.push("properties:{");
                      if (J.t(J.test("mapping", item_value_12))) {
                        out.push(J.str(J.call(format_parameters_0, item_value_12, J.filter("default", J.item(J.item(value_8, "items"), "required"), []))));
                      }
                      out.push("}");
                    } else if (J.t((J.eq(item_key_11, "required")))) {
                      out.push("required:[");
                      {
                        let _items3 = J.iter(item_value_12);
                        const _loop3 = J.loop(_items3);
                        for (let _i3 = 0; _i3 < _items3.length; _i3++) {
                          _loop3.index0 = _i3; const _it3 = _items3[_i3]; let req_item_13 = _it3;
                          out.push("<|\"|>", J.str(req_item_13), "<|\"|>");
                          if (J.t(!J.t(J.attr(_loop3, "last")))) {
                            out.push(",");
                          }
                        }
                      }
                      out.push("]");
                    } else if (J.t((J.eq(item_key_11, "type")))) {
                      if (J.t(J.test("string", item_value_12))) {
                        out.push("type:", J.str(J.call(format_argument_0, J.filter("upper", item_value_12))));
                      } else {
                        out.push("type:", J.str(J.call(format_argument_0, J.filter("list", J.filter("map", item_value_12, "upper")))));
                      }
                    } else {
                      out.push(J.str(item_key_11), ":", J.str(J.call(format_argument_0, item_value_12)));
                    }
                  }
                }
              }
              out.push("}");
            }
          }
          if (J.t(J.item(value_8, "nullable"))) {
            if (J.t(add_comma_9)) {
              out.push(",");
            } else {
              add_comma_9 = true;
            }
            out.push("nullable:true");
          }
          if (J.t((J.eq(J.filter("upper", J.item(value_8, "type")), "OBJECT")))) {
            if (J.t(((v) => J.t(v) ? J.test("mapping", J.item(value_8, "properties")) : v)((J.item(value_8, "properties") !== undefined)))) {
              if (J.t(add_comma_9)) {
                out.push(",");
              } else {
                add_comma_9 = true;
              }
              out.push("properties:{", J.str(J.call(format_parameters_0, J.item(value_8, "properties"), J.filter("default", J.item(value_8, "required"), []))), "}");
            } else if (J.t(J.test("mapping", value_8))) {
              if (J.t(add_comma_9)) {
                out.push(",");
              } else {
                add_comma_9 = true;
              }
              out.push("properties:{", J.str(J.call(format_parameters_0, value_8, J.filter("default", J.item(value_8, "required"), []), J.kw({"filter_keys": true}))), "}");
            }
            if (J.t(J.item(value_8, "required"))) {
              if (J.t(add_comma_9)) {
                out.push(",");
              } else {
                add_comma_9 = true;
              }
              out.push("required:[");
              {
                let _items4 = J.iter(J.filter("default", J.item(value_8, "required"), []));
                const _loop4 = J.loop(_items4);
                for (let _i4 = 0; _i4 < _items4.length; _i4++) {
                  _loop4.index0 = _i4; const _it4 = _items4[_i4]; let item_14 = _it4;
                  out.push("<|\"|>", J.str(item_14), "<|\"|>");
                  if (J.t(!J.t(J.attr(_loop4, "last")))) {
                    out.push(",");
                  }
                }
              }
              out.push("]");
            }
          }
          if (J.t(add_comma_9)) {
            out.push(",");
          } else {
            add_comma_9 = true;
          }
          out.push("type:<|\"|>", J.str(J.filter("upper", J.item(value_8, "type"))), "<|\"|>}");
        }
      }
    }
    return out.join("");
  };
  format_function_declaration_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_16 = k.caller;
    let tool_data_15 = p.length > 0 ? p[0] : "tool_data" in k ? k["tool_data"] : undefined;
    let params_17 = params_0;
    let response_declaration_18 = response_declaration_0;
    out.push("declaration:", J.str(J.item(J.item(tool_data_15, "function"), "name")), "{description:<|\"|>", J.str(J.item(J.item(tool_data_15, "function"), "description")), "<|\"|>");
    params_17 = J.item(J.item(tool_data_15, "function"), "parameters");
    if (J.t(params_17)) {
      out.push(",parameters:{");
      if (J.t(J.item(params_17, "properties"))) {
        out.push("properties:{", J.str(J.call(format_parameters_0, J.item(params_17, "properties"), J.item(params_17, "required"))), "},");
      }
      if (J.t(J.item(params_17, "required"))) {
        out.push("required:[");
        {
          let _items5 = J.iter(J.item(params_17, "required"));
          const _loop5 = J.loop(_items5);
          for (let _i5 = 0; _i5 < _items5.length; _i5++) {
            _loop5.index0 = _i5; const _it5 = _items5[_i5]; let item_19 = _it5;
            out.push("<|\"|>", J.str(item_19), "<|\"|>", J.str((J.t(!J.t(J.attr(_loop5, "last"))) ? "," : undefined)));
          }
        }
        out.push("],");
      }
      if (J.t(J.item(params_17, "type"))) {
        out.push("type:<|\"|>", J.str(J.filter("upper", J.item(params_17, "type"))), "<|\"|>}");
      }
    }
    if (J.t((J.has("response", J.item(tool_data_15, "function"))))) {
      response_declaration_18 = J.item(J.item(tool_data_15, "function"), "response");
      out.push(",response:{");
      if (J.t(J.item(response_declaration_18, "description"))) {
        out.push("description:<|\"|>", J.str(J.item(response_declaration_18, "description")), "<|\"|>,");
      }
      if (J.t((J.eq(J.filter("upper", J.item(response_declaration_18, "type")), "OBJECT")))) {
        out.push("type:<|\"|>", J.str(J.filter("upper", J.item(response_declaration_18, "type"))), "<|\"|>}");
      }
    }
    out.push("}");
    return out.join("");
  };
  format_argument_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_22 = k.caller;
    let argument_20 = p.length > 0 ? p[0] : "argument" in k ? k["argument"] : undefined;
    let escape_keys_21 = p.length > 1 ? p[1] : "escape_keys" in k ? k["escape_keys"] : true;
    let ns_23 = ns_0;
    if (J.t((argument_20 === null))) {
      out.push(J.str("null"));
    } else if (J.t(J.test("string", argument_20))) {
      out.push(J.str(J.add(J.add("<|\"|>", argument_20), "<|\"|>")));
    } else if (J.t(J.test("boolean", argument_20))) {
      out.push(J.str((J.t(argument_20) ? "true" : "false")));
    } else if (J.t(J.test("mapping", argument_20))) {
      out.push(J.str("{"));
      ns_23 = J.call(J.namespace, J.kw({"found_first": false}));
      {
        let _items6 = J.iter(J.filter("dictsort", argument_20));
        const _loop6 = J.loop(_items6);
        for (let _i6 = 0; _i6 < _items6.length; _i6++) {
          _loop6.index0 = _i6; const _it6 = _items6[_i6]; let [key_24, value_25] = J.unpack(_it6, 2);
          if (J.t(J.attr(ns_23, "found_first"))) {
            out.push(",");
          }
          J.setattr(ns_23, "found_first", true);
          if (J.t(escape_keys_21)) {
            out.push(J.str(J.add(J.add("<|\"|>", key_24), "<|\"|>")));
          } else {
            out.push(J.str(key_24));
          }
          out.push(":", J.str(J.call(format_argument_0, value_25, J.kw({"escape_keys": escape_keys_21}))));
        }
      }
      out.push(J.str("}"));
    } else if (J.t(J.test("sequence", argument_20))) {
      out.push(J.str("["));
      {
        let _items7 = J.iter(argument_20);
        const _loop7 = J.loop(_items7);
        for (let _i7 = 0; _i7 < _items7.length; _i7++) {
          _loop7.index0 = _i7; const _it7 = _items7[_i7]; let item_26 = _it7;
          out.push(J.str(J.call(format_argument_0, item_26, J.kw({"escape_keys": escape_keys_21}))));
          if (J.t(!J.t(J.attr(_loop7, "last")))) {
            out.push(",");
          }
        }
      }
      out.push(J.str("]"));
    } else {
      out.push(J.str(argument_20));
    }
    return out.join("");
  };
  strip_thinking_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_28 = k.caller;
    let text_27 = p.length > 0 ? p[0] : "text" in k ? k["text"] : undefined;
    let ns_29 = ns_0;
    ns_29 = J.call(J.namespace, J.kw({"result": ""}));
    {
      let _items8 = J.iter(J.call(J.attr(text_27, "split"), "<channel|>"));
      const _loop8 = J.loop(_items8);
      for (let _i8 = 0; _i8 < _items8.length; _i8++) {
        _loop8.index0 = _i8; const _it8 = _items8[_i8]; let part_30 = _it8;
        if (J.t((J.has("<|channel>", part_30)))) {
          J.setattr(ns_29, "result", J.add(J.attr(ns_29, "result"), J.item(J.call(J.attr(part_30, "split"), "<|channel>"), 0)));
        } else {
          J.setattr(ns_29, "result", J.add(J.attr(ns_29, "result"), part_30));
        }
      }
    }
    out.push(J.str(J.filter("trim", J.attr(ns_29, "result"))));
    return out.join("");
  };
  format_tool_response_block_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_33 = k.caller;
    let tool_name_31 = p.length > 0 ? p[0] : "tool_name" in k ? k["tool_name"] : undefined;
    let response_32 = p.length > 1 ? p[1] : "response" in k ? k["response"] : undefined;
    out.push(J.str("<|tool_response>"));
    if (J.t(J.test("mapping", response_32))) {
      out.push(J.str(J.add(J.add("response:", tool_name_31), "{")));
      {
        let _items9 = J.iter(J.filter("dictsort", response_32));
        const _loop9 = J.loop(_items9);
        for (let _i9 = 0; _i9 < _items9.length; _i9++) {
          _loop9.index0 = _i9; const _it9 = _items9[_i9]; let [key_34, value_35] = J.unpack(_it9, 2);
          out.push(J.str(key_34), ":", J.str(J.call(format_argument_0, value_35, J.kw({"escape_keys": false}))));
          if (J.t(!J.t(J.attr(_loop9, "last")))) {
            out.push(",");
          }
        }
      }
      out.push(J.str("}"));
    } else {
      out.push(J.str(J.add(J.add(J.add(J.add("response:", tool_name_31), "{value:"), J.call(format_argument_0, response_32, J.kw({"escape_keys": false}))), "}")));
    }
    out.push(J.str("<tool_response|>"));
    return out.join("");
  };
  ns_0 = J.call(J.namespace, J.kw({"prev_message_type": null, "prev_non_tool_role": null}));
  loop_messages_0 = messages_0;
  enable_thinking_0 = J.filter("default", enable_thinking_0, false);
  preserve_thinking_0 = J.filter("default", preserve_thinking_0, false);
  out.push(J.str(bos_token_0));
  if (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? (J.has(J.item(J.item(messages_0, 0), "role"), ["system", "developer"])) : v)(messages_0))(((v) => J.t(v) ? v : tools_0)(enable_thinking_0)))) {
    out.push(J.str("<|turn>system\n"));
    if (J.t(enable_thinking_0)) {
      out.push(J.str("<|think|>\n"));
      J.setattr(ns_0, "prev_message_type", "think");
    }
    if (J.t(((v) => J.t(v) ? (J.has(J.item(J.item(messages_0, 0), "role"), ["system", "developer"])) : v)(messages_0))) {
      if (J.t(J.test("string", J.item(J.item(messages_0, 0), "content")))) {
        out.push(J.str(J.filter("trim", J.item(J.item(messages_0, 0), "content"))));
      } else if (J.t(J.test("sequence", J.item(J.item(messages_0, 0), "content")))) {
        {
          let _items10 = J.iter(J.item(J.item(messages_0, 0), "content"));
          const _loop10 = J.loop(_items10);
          for (let _i10 = 0; _i10 < _items10.length; _i10++) {
            _loop10.index0 = _i10; const _it10 = _items10[_i10]; let item_36 = _it10;
            out.push(J.str(J.add(J.filter("trim", J.item(item_36, "text")), " ")));
          }
        }
      }
      loop_messages_0 = J.slice(messages_0, 1, undefined, undefined);
    }
    if (J.t(tools_0)) {
      {
        let _items11 = J.iter(tools_0);
        const _loop11 = J.loop(_items11);
        for (let _i11 = 0; _i11 < _items11.length; _i11++) {
          _loop11.index0 = _i11; const _it11 = _items11[_i11]; let tool_37 = _it11;
          out.push(J.str("<|tool>"), J.str(J.filter("trim", J.call(format_function_declaration_0, tool_37))), J.str("<tool|>"));
        }
      }
      J.setattr(ns_0, "prev_message_type", "tool");
    }
    out.push(J.str("<turn|>\n"));
  }
  ns_turn_0 = J.call(J.namespace, J.kw({"last_user_idx": J.neg(1)}));
  {
    let _items12 = J.iter(J.call(J.range, J.filter("length", loop_messages_0)));
    const _loop12 = J.loop(_items12);
    for (let _i12 = 0; _i12 < _items12.length; _i12++) {
      _loop12.index0 = _i12; const _it12 = _items12[_i12]; let i_38 = _it12;
      if (J.t((J.eq(J.item(J.item(loop_messages_0, i_38), "role"), "user")))) {
        J.setattr(ns_turn_0, "last_user_idx", i_38);
      }
    }
  }
  {
    let _items13 = J.iter(loop_messages_0);
    const _loop13 = J.loop(_items13);
    for (let _i13 = 0; _i13 < _items13.length; _i13++) {
      _loop13.index0 = _i13; const _it13 = _items13[_i13]; let message_39 = _it13;
      let role_40 = role_0;
      let continue_same_model_turn_41 = continue_same_model_turn_0;
      let thinking_text_42 = thinking_text_0;
      let thinking_gate_43 = thinking_gate_0;
      let ns_tr_out_44 = ns_tr_out_0;
      let ns_tool_scan_45 = ns_tool_scan_0;
      let captured_content_46 = captured_content_0;
      let has_content_47 = has_content_0;
      let next_nt_48 = next_nt_0;
      let continues_into_next_49 = continues_into_next_0;
      if (J.t((!J.eq(J.item(message_39, "role"), "tool")))) {
        J.setattr(ns_0, "prev_message_type", null);
        role_40 = (J.t((J.eq(J.item(message_39, "role"), "assistant"))) ? "model" : J.item(message_39, "role"));
        continue_same_model_turn_41 = ((v) => J.t(v) ? (J.eq(J.attr(ns_0, "prev_non_tool_role"), "assistant")) : v)((J.eq(role_40, "model")));
        if (J.t(!J.t(continue_same_model_turn_41))) {
          out.push(J.str(J.add(J.add("<|turn>", role_40), "\n")));
        }
        thinking_text_42 = ((v) => J.t(v) ? v : J.call(J.attr(message_39, "get"), "reasoning_content"))(J.call(J.attr(message_39, "get"), "reasoning"));
        thinking_gate_43 = ((v) => J.t(v) ? v : ((v) => J.t(v) ? J.call(J.attr(message_39, "get"), "tool_calls") : v)(preserve_thinking_0))((J.gt(J.attr(_loop13, "index0"), J.attr(ns_turn_0, "last_user_idx"))));
        if (J.t(((v) => J.t(v) ? thinking_gate_43 : v)(thinking_text_42))) {
          out.push(J.str(J.add(J.add("<|channel>thought\n", thinking_text_42), "\n<channel|>")));
        }
        if (J.t(J.call(J.attr(message_39, "get"), "tool_calls"))) {
          {
            let _items14 = J.iter(J.call(J.attr(message_39, "get"), "tool_calls"));
            const _loop14 = J.loop(_items14);
            for (let _i14 = 0; _i14 < _items14.length; _i14++) {
              _loop14.index0 = _i14; const _it14 = _items14[_i14]; let tool_call_50 = _it14;
              let function_51 = function_0;
              let ns_args_52 = ns_args_0;
              function_51 = J.item(tool_call_50, "function");
              out.push(J.str(J.add(J.add("<|tool_call>call:", J.item(function_51, "name")), "{")));
              if (J.t(J.test("mapping", J.item(function_51, "arguments")))) {
                ns_args_52 = J.call(J.namespace, J.kw({"found_first": false}));
                {
                  let _items15 = J.iter(J.filter("dictsort", J.item(function_51, "arguments")));
                  const _loop15 = J.loop(_items15);
                  for (let _i15 = 0; _i15 < _items15.length; _i15++) {
                    _loop15.index0 = _i15; const _it15 = _items15[_i15]; let [key_53, value_54] = J.unpack(_it15, 2);
                    if (J.t(J.attr(ns_args_52, "found_first"))) {
                      out.push(",");
                    }
                    J.setattr(ns_args_52, "found_first", true);
                    out.push(J.str(key_53), ":", J.str(J.call(format_argument_0, value_54, J.kw({"escape_keys": false}))));
                  }
                }
              } else if (J.t((J.item(function_51, "arguments") === null))) {
              } else {
                out.push(J.str(J.call(J.raise_exception, "chat_template: tool_calls[].function.arguments must be a JSON object (mapping), not a string. Deserialize arguments before passing to the template.")));
              }
              out.push(J.str("}<tool_call|>"));
            }
          }
          J.setattr(ns_0, "prev_message_type", "tool_call");
        }
        ns_tr_out_44 = J.call(J.namespace, J.kw({"flag": false}));
        if (J.t(J.call(J.attr(message_39, "get"), "tool_responses"))) {
          {
            let _items16 = J.iter(J.call(J.attr(message_39, "get"), "tool_responses"));
            const _loop16 = J.loop(_items16);
            for (let _i16 = 0; _i16 < _items16.length; _i16++) {
              _loop16.index0 = _i16; const _it16 = _items16[_i16]; let tool_response_55 = _it16;
              out.push(J.str(J.call(format_tool_response_block_0, J.filter("default", J.item(tool_response_55, "name"), "unknown", true), J.item(tool_response_55, "response"))));
              J.setattr(ns_tr_out_44, "flag", true);
              J.setattr(ns_0, "prev_message_type", "tool_response");
            }
          }
        } else if (J.t(J.call(J.attr(message_39, "get"), "tool_calls"))) {
          ns_tool_scan_45 = J.call(J.namespace, J.kw({"stopped": false}));
          {
            let _items17 = J.iter(J.call(J.range, J.add(J.attr(_loop13, "index0"), 1), J.filter("length", loop_messages_0)));
            const _loop17 = J.loop(_items17);
            for (let _i17 = 0; _i17 < _items17.length; _i17++) {
              _loop17.index0 = _i17; const _it17 = _items17[_i17]; let k_56 = _it17;
              let follow_57 = follow_0;
              let ns_tname_58 = ns_tname_0;
              let tool_body_59 = tool_body_0;
              let ns_txt_60 = ns_txt_0;
              if (J.t(J.attr(ns_tool_scan_45, "stopped"))) {
              } else if (J.t((!J.eq(J.item(J.item(loop_messages_0, k_56), "role"), "tool")))) {
                J.setattr(ns_tool_scan_45, "stopped", true);
              } else {
                follow_57 = J.item(loop_messages_0, k_56);
                ns_tname_58 = J.call(J.namespace, J.kw({"name": ((v) => J.t(v) ? v : "unknown")(J.call(J.attr(follow_57, "get"), "name"))}));
                {
                  let _items18 = J.iter(J.call(J.attr(message_39, "get"), "tool_calls"));
                  const _loop18 = J.loop(_items18);
                  for (let _i18 = 0; _i18 < _items18.length; _i18++) {
                    _loop18.index0 = _i18; const _it18 = _items18[_i18]; let tc_61 = _it18;
                    if (J.t((J.eq(J.call(J.attr(tc_61, "get"), "id"), J.call(J.attr(follow_57, "get"), "tool_call_id"))))) {
                      J.setattr(ns_tname_58, "name", J.item(J.item(tc_61, "function"), "name"));
                    }
                  }
                }
                tool_body_59 = J.call(J.attr(follow_57, "get"), "content");
                if (J.t(J.test("string", tool_body_59))) {
                  out.push(J.str(J.call(format_tool_response_block_0, J.attr(ns_tname_58, "name"), tool_body_59)));
                } else if (J.t(((v) => J.t(v) ? !J.t(J.test("string", tool_body_59)) : v)(J.test("sequence", tool_body_59)))) {
                  ns_txt_60 = J.call(J.namespace, J.kw({"s": ""}));
                  {
                    let _items19 = J.iter(tool_body_59);
                    const _loop19 = J.loop(_items19);
                    for (let _i19 = 0; _i19 < _items19.length; _i19++) {
                      _loop19.index0 = _i19; const _it19 = _items19[_i19]; let part_62 = _it19;
                      if (J.t((J.eq(J.call(J.attr(part_62, "get"), "type"), "text")))) {
                        J.setattr(ns_txt_60, "s", J.add(J.attr(ns_txt_60, "s"), J.filter("default", J.call(J.attr(part_62, "get"), "text"), "")));
                      }
                    }
                  }
                  out.push(J.str(J.call(format_tool_response_block_0, J.attr(ns_tname_58, "name"), J.attr(ns_txt_60, "s"))));
                  {
                    let _items20 = J.iter(tool_body_59);
                    const _loop20 = J.loop(_items20);
                    for (let _i20 = 0; _i20 < _items20.length; _i20++) {
                      _loop20.index0 = _i20; const _it20 = _items20[_i20]; let part_63 = _it20;
                      if (J.t((J.has(J.call(J.attr(part_63, "get"), "type"), ["image", "image_url"])))) {
                        out.push(J.str("<|image|>"));
                      } else if (J.t((J.has(J.call(J.attr(part_63, "get"), "type"), ["audio", "input_audio"])))) {
                        out.push(J.str("<|audio|>"));
                      } else if (J.t((J.eq(J.call(J.attr(part_63, "get"), "type"), "video")))) {
                        out.push(J.str("<|video|>"));
                      }
                    }
                  }
                } else {
                  out.push(J.str(J.call(format_tool_response_block_0, J.attr(ns_tname_58, "name"), tool_body_59)));
                }
                J.setattr(ns_tr_out_44, "flag", true);
                J.setattr(ns_0, "prev_message_type", "tool_response");
              }
            }
          }
        }
        {
          const captured = (() => {
          const out = [];
          if (J.t(J.test("string", J.call(J.attr(message_39, "get"), "content")))) {
            if (J.t((J.eq(role_40, "model")))) {
              out.push(J.str(J.call(strip_thinking_0, J.item(message_39, "content"))));
            } else {
              out.push(J.str(J.filter("trim", J.item(message_39, "content"))));
            }
          } else if (J.t(J.test("sequence", J.call(J.attr(message_39, "get"), "content")))) {
            {
              let _items21 = J.iter(J.item(message_39, "content"));
              const _loop21 = J.loop(_items21);
              for (let _i21 = 0; _i21 < _items21.length; _i21++) {
                _loop21.index0 = _i21; const _it21 = _items21[_i21]; let item_64 = _it21;
                if (J.t((J.eq(J.call(J.attr(item_64, "get"), "type"), "text")))) {
                  if (J.t((J.eq(role_40, "model")))) {
                    out.push(J.str(J.call(strip_thinking_0, J.item(item_64, "text"))));
                  } else {
                    out.push(J.str(J.filter("trim", J.item(item_64, "text"))));
                  }
                } else if (J.t((J.has(J.call(J.attr(item_64, "get"), "type"), ["image", "image_url"])))) {
                  out.push(J.str("<|image|>"));
                } else if (J.t((J.has(J.call(J.attr(item_64, "get"), "type"), ["audio", "input_audio"])))) {
                  out.push(J.str("<|audio|>"));
                } else if (J.t((J.eq(J.call(J.attr(item_64, "get"), "type"), "video")))) {
                  out.push(J.str("<|video|>"));
                }
              }
            }
          }
          return out.join("");
          })();
          captured_content_46 = captured;
        }
        out.push(J.str(captured_content_46));
        has_content_47 = (J.gt(J.filter("length", J.filter("trim", captured_content_46)), 0));
        next_nt_48 = J.call(J.namespace, J.kw({"role": null, "found": false}));
        {
          let _items22 = J.iter(J.call(J.range, J.add(J.attr(_loop13, "index0"), 1), J.filter("length", loop_messages_0)));
          const _loop22 = J.loop(_items22);
          for (let _i22 = 0; _i22 < _items22.length; _i22++) {
            _loop22.index0 = _i22; const _it22 = _items22[_i22]; let j_65 = _it22;
            if (J.t(!J.t(J.attr(next_nt_48, "found")))) {
              if (J.t((!J.eq(J.item(J.item(loop_messages_0, j_65), "role"), "tool")))) {
                J.setattr(next_nt_48, "role", J.item(J.item(loop_messages_0, j_65), "role"));
                J.setattr(next_nt_48, "found", true);
              }
            }
          }
        }
        continues_into_next_49 = ((v) => J.t(v) ? ((v) => J.t(v) ? v : J.attr(ns_tr_out_44, "flag"))(!J.t(J.call(J.attr(message_39, "get"), "tool_calls"))) : v)(((v) => J.t(v) ? (J.eq(J.attr(next_nt_48, "role"), "assistant")) : v)((J.eq(role_40, "model"))));
        if (J.t(((v) => J.t(v) ? !J.t(J.attr(ns_tr_out_44, "flag")) : v)((J.eq(J.attr(ns_0, "prev_message_type"), "tool_call"))))) {
          out.push(J.str("<|tool_response>"));
        } else if (J.t(continues_into_next_49)) {
        } else if (J.t(!J.t(((v) => J.t(v) ? !J.t(J.attr(next_nt_48, "found")) : v)(((v) => J.t(v) ? !J.t(has_content_47) : v)(J.attr(ns_tr_out_44, "flag")))))) {
          out.push(J.str("<turn|>\n"));
        }
      }
      J.setattr(ns_0, "prev_non_tool_role", J.item(message_39, "role"));
    }
  }
  if (J.t(add_generation_prompt_0)) {
    if (J.t(((v) => J.t(v) ? (!J.eq(J.attr(ns_0, "prev_message_type"), "tool_call")) : v)((!J.eq(J.attr(ns_0, "prev_message_type"), "tool_response"))))) {
      out.push(J.str("<|turn>model\n"));
    } else if (J.t(((v) => J.t(v) ? enable_thinking_0 : v)((J.eq(J.attr(ns_0, "prev_message_type"), "tool_response"))))) {
      out.push(J.str("<|channel>thought\n"));
    }
  }
  return out.join("");
}
