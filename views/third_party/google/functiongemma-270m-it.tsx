// Converted from the chat template of google/functiongemma-270m-it by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let format_parameters_0 = ctx["format_parameters"];
  let format_function_declaration_0 = ctx["format_function_declaration"];
  let format_argument_0 = ctx["format_argument"];
  let ns_0 = ctx["ns"];
  let loop_messages_0 = ctx["loop_messages"];
  let standard_keys_0 = ctx["standard_keys"];
  let ns_items_0 = ctx["ns_items"];
  let params_0 = ctx["params"];
  let bos_token_0 = ctx["bos_token"];
  let messages_0 = ctx["messages"];
  let tools_0 = ctx["tools"] ?? null;
  let role_0 = ctx["role"];
  let response_ns_0 = ctx["response_ns"];
  let function_0 = ctx["function"];
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
        let ns_items_8 = ns_items_0;
        if (J.t((!J.has(key_6, standard_keys_4)))) {
          if (J.t(J.attr(ns_5, "found_first"))) {
            out.push(",");
          }
          J.setattr(ns_5, "found_first", true);
          out.push(J.str(key_6), ":{description:<escape>", J.str(J.item(value_7, "description")), "<escape>");
          if (J.t((J.eq(J.filter("upper", J.item(value_7, "type")), "STRING")))) {
            if (J.t(J.item(value_7, "enum"))) {
              out.push(",enum:", J.str(J.call(format_argument_0, J.item(value_7, "enum"))));
            }
          } else if (J.t((J.eq(J.filter("upper", J.item(value_7, "type")), "OBJECT")))) {
            out.push(",properties:{");
            if (J.t(((v) => J.t(v) ? J.test("mapping", J.item(value_7, "properties")) : v)((J.item(value_7, "properties") !== undefined)))) {
              out.push(J.str(J.call(format_parameters_0, J.item(value_7, "properties"), J.filter("default", J.item(value_7, "required"), []))));
            } else if (J.t(J.test("mapping", value_7))) {
              out.push(J.str(J.call(format_parameters_0, value_7, J.filter("default", J.item(value_7, "required"), []))));
            }
            out.push("}");
            if (J.t(J.item(value_7, "required"))) {
              out.push(",required:[");
              {
                let _items2 = J.iter(J.filter("default", J.item(value_7, "required"), []));
                const _loop2 = J.loop(_items2);
                for (let _i2 = 0; _i2 < _items2.length; _i2++) {
                  _loop2.index0 = _i2; const _it2 = _items2[_i2]; let item_9 = _it2;
                  out.push("<escape>", J.str(item_9), "<escape>");
                  if (J.t(!J.t(J.attr(_loop2, "last")))) {
                    out.push(",");
                  }
                }
              }
              out.push("]");
            }
          } else if (J.t((J.eq(J.filter("upper", J.item(value_7, "type")), "ARRAY")))) {
            if (J.t(((v) => J.t(v) ? J.item(value_7, "items") : v)(J.test("mapping", J.item(value_7, "items"))))) {
              out.push(",items:{");
              ns_items_8 = J.call(J.namespace, J.kw({"found_first": false}));
              {
                let _items3 = J.iter(J.filter("dictsort", J.item(value_7, "items")));
                const _loop3 = J.loop(_items3);
                for (let _i3 = 0; _i3 < _items3.length; _i3++) {
                  _loop3.index0 = _i3; const _it3 = _items3[_i3]; let [item_key_10, item_value_11] = J.unpack(_it3, 2);
                  if (J.t(!J.t((item_value_11 === null)))) {
                    if (J.t(J.attr(ns_items_8, "found_first"))) {
                      out.push(",");
                    }
                    J.setattr(ns_items_8, "found_first", true);
                    if (J.t((J.eq(item_key_10, "properties")))) {
                      out.push("properties:{");
                      if (J.t(J.test("mapping", item_value_11))) {
                        out.push(J.str(J.call(format_parameters_0, item_value_11, J.filter("default", J.item(J.item(value_7, "items"), "required"), []))));
                      }
                      out.push("}");
                    } else if (J.t((J.eq(item_key_10, "required")))) {
                      out.push("required:[");
                      {
                        let _items4 = J.iter(item_value_11);
                        const _loop4 = J.loop(_items4);
                        for (let _i4 = 0; _i4 < _items4.length; _i4++) {
                          _loop4.index0 = _i4; const _it4 = _items4[_i4]; let req_item_12 = _it4;
                          out.push("<escape>", J.str(req_item_12), "<escape>");
                          if (J.t(!J.t(J.attr(_loop4, "last")))) {
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
          out.push(",type:<escape>", J.str(J.filter("upper", J.item(value_7, "type"))), "<escape>}");
        }
      }
    }
    return out.join("");
  };
  format_function_declaration_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_14 = k.caller;
    let tool_data_13 = p.length > 0 ? p[0] : "tool_data" in k ? k["tool_data"] : undefined;
    let params_15 = params_0;
    out.push("declaration:", J.str(J.item(J.item(tool_data_13, "function"), "name")), "{description:<escape>", J.str(J.item(J.item(tool_data_13, "function"), "description")), "<escape>");
    params_15 = J.item(J.item(tool_data_13, "function"), "parameters");
    if (J.t(params_15)) {
      out.push(",parameters:{");
      if (J.t(J.item(params_15, "properties"))) {
        out.push("properties:{", J.str(J.call(format_parameters_0, J.item(params_15, "properties"), J.item(params_15, "required"))), "},");
      }
      if (J.t(J.item(params_15, "required"))) {
        out.push("required:[");
        {
          let _items5 = J.iter(J.item(params_15, "required"));
          const _loop5 = J.loop(_items5);
          for (let _i5 = 0; _i5 < _items5.length; _i5++) {
            _loop5.index0 = _i5; const _it5 = _items5[_i5]; let item_16 = _it5;
            out.push("<escape>", J.str(item_16), "<escape>", J.str((J.t(!J.t(J.attr(_loop5, "last"))) ? "," : undefined)));
          }
        }
        out.push("],");
      }
      if (J.t(J.item(params_15, "type"))) {
        out.push("type:<escape>", J.str(J.filter("upper", J.item(params_15, "type"))), "<escape>}");
      }
    }
    out.push("}");
    return out.join("");
  };
  format_argument_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_19 = k.caller;
    let argument_17 = p.length > 0 ? p[0] : "argument" in k ? k["argument"] : undefined;
    let escape_keys_18 = p.length > 1 ? p[1] : "escape_keys" in k ? k["escape_keys"] : true;
    let ns_20 = ns_0;
    if (J.t(J.test("string", argument_17))) {
      out.push(J.str(J.add(J.add("<escape>", argument_17), "<escape>")));
    } else if (J.t(J.test("boolean", argument_17))) {
      if (J.t(argument_17)) {
        out.push(J.str("true"));
      } else {
        out.push(J.str("false"));
      }
    } else if (J.t(J.test("mapping", argument_17))) {
      out.push(J.str("{"));
      ns_20 = J.call(J.namespace, J.kw({"found_first": false}));
      {
        let _items6 = J.iter(J.filter("dictsort", argument_17));
        const _loop6 = J.loop(_items6);
        for (let _i6 = 0; _i6 < _items6.length; _i6++) {
          _loop6.index0 = _i6; const _it6 = _items6[_i6]; let [key_21, value_22] = J.unpack(_it6, 2);
          if (J.t(J.attr(ns_20, "found_first"))) {
            out.push(",");
          }
          J.setattr(ns_20, "found_first", true);
          if (J.t(escape_keys_18)) {
            out.push(J.str(J.add(J.add("<escape>", key_21), "<escape>")));
          } else {
            out.push(J.str(key_21));
          }
          out.push(":", J.str(J.call(format_argument_0, value_22, J.kw({"escape_keys": escape_keys_18}))));
        }
      }
      out.push(J.str("}"));
    } else if (J.t(J.test("sequence", argument_17))) {
      out.push(J.str("["));
      {
        let _items7 = J.iter(argument_17);
        const _loop7 = J.loop(_items7);
        for (let _i7 = 0; _i7 < _items7.length; _i7++) {
          _loop7.index0 = _i7; const _it7 = _items7[_i7]; let item_23 = _it7;
          out.push(J.str(J.call(format_argument_0, item_23, J.kw({"escape_keys": escape_keys_18}))));
          if (J.t(!J.t(J.attr(_loop7, "last")))) {
            out.push(",");
          }
        }
      }
      out.push(J.str("]"));
    } else {
      out.push(J.str(argument_17));
    }
    return out.join("");
  };
  out.push(J.str(bos_token_0));
  ns_0 = J.call(J.namespace, J.kw({"prev_message_type": null}));
  loop_messages_0 = messages_0;
  if (J.t(((v) => J.t(v) ? v : (J.eq(J.item(J.item(messages_0, 0), "role"), "developer")))(((v) => J.t(v) ? v : (J.eq(J.item(J.item(messages_0, 0), "role"), "system")))(tools_0)))) {
    out.push(J.str("<start_of_turn>developer\n"));
    if (J.t(((v) => J.t(v) ? v : (J.eq(J.item(J.item(messages_0, 0), "role"), "developer")))((J.eq(J.item(J.item(messages_0, 0), "role"), "system"))))) {
      if (J.t(J.test("string", J.item(J.item(messages_0, 0), "content")))) {
        out.push(J.str(J.filter("trim", J.item(J.item(messages_0, 0), "content"))));
      } else if (J.t(J.test("sequence", J.item(J.item(messages_0, 0), "content")))) {
        {
          let _items8 = J.iter(J.item(J.item(messages_0, 0), "content"));
          const _loop8 = J.loop(_items8);
          for (let _i8 = 0; _i8 < _items8.length; _i8++) {
            _loop8.index0 = _i8; const _it8 = _items8[_i8]; let item_24 = _it8;
            if (J.t((J.eq(J.item(item_24, "type"), "text")))) {
              out.push(J.str(J.filter("trim", J.item(item_24, "text"))));
            }
          }
        }
      }
      loop_messages_0 = J.slice(messages_0, 1, undefined, undefined);
    }
    if (J.t(tools_0)) {
      {
        let _items9 = J.iter(tools_0);
        const _loop9 = J.loop(_items9);
        for (let _i9 = 0; _i9 < _items9.length; _i9++) {
          _loop9.index0 = _i9; const _it9 = _items9[_i9]; let tool_25 = _it9;
          out.push(J.str("<start_function_declaration>"), J.str(J.filter("trim", J.call(format_function_declaration_0, tool_25))), J.str("<end_function_declaration>"));
        }
      }
    }
    out.push(J.str("<end_of_turn>\n"));
  }
  {
    let _items10 = J.iter(loop_messages_0);
    const _loop10 = J.loop(_items10);
    for (let _i10 = 0; _i10 < _items10.length; _i10++) {
      _loop10.index0 = _i10; const _it10 = _items10[_i10]; let message_26 = _it10;
      let role_27 = role_0;
      let response_ns_28 = response_ns_0;
      if (J.t((J.eq(J.item(message_26, "role"), "assistant")))) {
        role_27 = "model";
      } else {
        role_27 = J.item(message_26, "role");
      }
      if (J.t((!J.eq(role_27, "tool")))) {
        if (J.t((!J.eq(J.attr(ns_0, "prev_message_type"), "tool_response")))) {
          out.push(J.str(J.add(J.add("<start_of_turn>", role_27), "\n")));
        }
        J.setattr(ns_0, "prev_message_type", null);
        if (J.t(((v) => J.t(v) ? !J.t((J.item(message_26, "content") === null)) : v)((J.has("content", message_26))))) {
          if (J.t(J.test("string", J.item(message_26, "content")))) {
            out.push(J.str(J.filter("trim", J.item(message_26, "content"))));
          } else if (J.t(J.test("sequence", J.item(message_26, "content")))) {
            {
              let _items11 = J.iter(J.item(message_26, "content"));
              const _loop11 = J.loop(_items11);
              for (let _i11 = 0; _i11 < _items11.length; _i11++) {
                _loop11.index0 = _i11; const _it11 = _items11[_i11]; let item_29 = _it11;
                if (J.t((J.eq(J.item(item_29, "type"), "image")))) {
                  out.push(J.str("<start_of_image>"));
                } else if (J.t((J.eq(J.item(item_29, "type"), "text")))) {
                  out.push(J.str(J.filter("trim", J.item(item_29, "text"))));
                }
              }
            }
          } else {
            out.push(J.str(J.call(J.raise_exception, "Invalid content type in user/assistant message")));
          }
          J.setattr(ns_0, "prev_message_type", "content");
        }
        if (J.t(((v) => J.t(v) ? J.test("iterable", J.item(message_26, "tool_calls")) : v)(((v) => J.t(v) ? J.item(message_26, "tool_calls") : v)((J.has("tool_calls", message_26)))))) {
          {
            let _items12 = J.iter(J.item(message_26, "tool_calls"));
            const _loop12 = J.loop(_items12);
            for (let _i12 = 0; _i12 < _items12.length; _i12++) {
              _loop12.index0 = _i12; const _it12 = _items12[_i12]; let tool_call_30 = _it12;
              let function_31 = function_0;
              let ns_32 = ns_0;
              function_31 = J.item(tool_call_30, "function");
              out.push(J.str(J.add(J.add("<start_function_call>call:", J.item(function_31, "name")), "{")));
              if (J.t((J.has("arguments", function_31)))) {
                if (J.t(J.test("mapping", J.item(function_31, "arguments")))) {
                  ns_32 = J.call(J.namespace, J.kw({"found_first": false}));
                  {
                    let _items13 = J.iter(J.filter("dictsort", J.item(function_31, "arguments")));
                    const _loop13 = J.loop(_items13);
                    for (let _i13 = 0; _i13 < _items13.length; _i13++) {
                      _loop13.index0 = _i13; const _it13 = _items13[_i13]; let [key_33, value_34] = J.unpack(_it13, 2);
                      if (J.t(J.attr(ns_32, "found_first"))) {
                        out.push(",");
                      }
                      J.setattr(ns_32, "found_first", true);
                      out.push(J.str(key_33), ":", J.str(J.call(format_argument_0, value_34, J.kw({"escape_keys": false}))));
                    }
                  }
                } else if (J.t(J.test("string", J.item(function_31, "arguments")))) {
                  out.push("                    ", J.str(J.item(function_31, "arguments")));
                }
              }
              out.push(J.str("}<end_function_call>"));
            }
          }
          if (J.t(J.attr(_loop10, "last"))) {
            out.push(J.str("<start_function_response>"));
          }
          J.setattr(ns_0, "prev_message_type", "tool_call");
        }
      } else {
        if (J.t(((v) => J.t(v) ? J.item(message_26, "content") : v)((J.has("content", message_26))))) {
          if (J.t(J.test("mapping", J.item(message_26, "content")))) {
            if (J.t(((v) => J.t(v) ? (J.has("response", J.item(message_26, "content"))) : v)((J.has("name", J.item(message_26, "content")))))) {
              out.push(J.str(J.add(J.add("<start_function_response>response:", J.filter("trim", J.item(J.item(message_26, "content"), "name"))), "{")));
              response_ns_28 = J.call(J.namespace, J.kw({"found_first": false}));
              {
                let _items14 = J.iter(J.filter("dictsort", J.item(J.item(message_26, "content"), "response")));
                const _loop14 = J.loop(_items14);
                for (let _i14 = 0; _i14 < _items14.length; _i14++) {
                  _loop14.index0 = _i14; const _it14 = _items14[_i14]; let [key_35, value_36] = J.unpack(_it14, 2);
                  if (J.t(J.attr(response_ns_28, "found_first"))) {
                    out.push(",");
                  }
                  J.setattr(response_ns_28, "found_first", true);
                  out.push(J.str(key_35), ":", J.str(J.call(format_argument_0, value_36, J.kw({"escape_keys": false}))));
                }
              }
              out.push(J.str("}<end_function_response>"));
            } else if (J.t((J.has("name", message_26)))) {
              out.push(J.str(J.add(J.add("<start_function_response>response:", J.filter("trim", J.item(message_26, "name"))), "{")));
              response_ns_28 = J.call(J.namespace, J.kw({"found_first": false}));
              {
                let _items15 = J.iter(J.filter("dictsort", J.item(message_26, "content")));
                const _loop15 = J.loop(_items15);
                for (let _i15 = 0; _i15 < _items15.length; _i15++) {
                  _loop15.index0 = _i15; const _it15 = _items15[_i15]; let [key_37, value_38] = J.unpack(_it15, 2);
                  if (J.t(J.attr(response_ns_28, "found_first"))) {
                    out.push(",");
                  }
                  J.setattr(response_ns_28, "found_first", true);
                  out.push(J.str(key_37), ":", J.str(J.call(format_argument_0, value_38, J.kw({"escape_keys": false}))));
                }
              }
              out.push(J.str("}<end_function_response>"));
            } else {
              out.push(J.str(J.call(J.raise_exception, "Invalid tool response mapping: must contain 'name' and 'response' keys, or 'name' must be in the message.")));
            }
          } else if (J.t(J.test("string", J.item(message_26, "content")))) {
            if (J.t((J.has("name", message_26)))) {
              out.push(J.str(J.add(J.add(J.add(J.add("<start_function_response>response:", J.filter("trim", J.item(message_26, "name"))), "{value:"), J.call(format_argument_0, J.item(message_26, "content"), J.kw({"escape_keys": false}))), "}<end_function_response>")));
            } else {
              out.push(J.str(J.call(J.raise_exception, "Invalid tool response: 'name' must be provided.")));
            }
          } else if (J.t(J.test("sequence", J.item(message_26, "content")))) {
            {
              let _items16 = J.iter(J.item(message_26, "content"));
              const _loop16 = J.loop(_items16);
              for (let _i16 = 0; _i16 < _items16.length; _i16++) {
                _loop16.index0 = _i16; const _it16 = _items16[_i16]; let item_39 = _it16;
                let response_ns_40 = response_ns_28;
                if (J.t(J.test("mapping", item_39))) {
                  if (J.t(((v) => J.t(v) ? (J.has("response", item_39)) : v)((J.has("name", item_39))))) {
                    out.push(J.str(J.add(J.add("<start_function_response>response:", J.filter("trim", J.item(item_39, "name"))), "{")));
                    response_ns_40 = J.call(J.namespace, J.kw({"found_first": false}));
                    {
                      let _items17 = J.iter(J.filter("dictsort", J.item(item_39, "response")));
                      const _loop17 = J.loop(_items17);
                      for (let _i17 = 0; _i17 < _items17.length; _i17++) {
                        _loop17.index0 = _i17; const _it17 = _items17[_i17]; let [key_41, value_42] = J.unpack(_it17, 2);
                        if (J.t(J.attr(response_ns_40, "found_first"))) {
                          out.push(",");
                        }
                        J.setattr(response_ns_40, "found_first", true);
                        out.push(J.str(key_41), ":", J.str(J.call(format_argument_0, value_42, J.kw({"escape_keys": false}))));
                      }
                    }
                    out.push(J.str("}<end_function_response>"));
                  } else if (J.t((J.has("name", message_26)))) {
                    out.push(J.str(J.add(J.add("<start_function_response>response:", J.filter("trim", J.item(message_26, "name"))), "{")));
                    response_ns_40 = J.call(J.namespace, J.kw({"found_first": false}));
                    {
                      let _items18 = J.iter(J.filter("dictsort", item_39));
                      const _loop18 = J.loop(_items18);
                      for (let _i18 = 0; _i18 < _items18.length; _i18++) {
                        _loop18.index0 = _i18; const _it18 = _items18[_i18]; let [key_43, value_44] = J.unpack(_it18, 2);
                        if (J.t(J.attr(response_ns_40, "found_first"))) {
                          out.push(",");
                        }
                        J.setattr(response_ns_40, "found_first", true);
                        out.push(J.str(key_43), ":", J.str(J.call(format_argument_0, value_44, J.kw({"escape_keys": false}))));
                      }
                    }
                    out.push(J.str("}<end_function_response>"));
                  } else {
                    out.push(J.str(J.call(J.raise_exception, "Invalid tool response mapping: must contain 'name' and 'response' keys, or 'name' must be in the message.")));
                  }
                } else {
                  out.push(J.str(J.call(J.raise_exception, "Invalid tool response message: multiple responses must all be mappings")));
                }
              }
            }
          } else {
            out.push(J.str(J.call(J.raise_exception, "Invalid content type in tool message: must be mapping, sequence of mappings, or string.")));
          }
        }
        J.setattr(ns_0, "prev_message_type", "tool_response");
      }
      if (J.t((!J.has(J.attr(ns_0, "prev_message_type"), ["tool_call", "tool_response"])))) {
        out.push(J.str("<end_of_turn>\n"));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    if (J.t((!J.eq(J.attr(ns_0, "prev_message_type"), "tool_response")))) {
      out.push(J.str("<start_of_turn>model\n"));
    }
  }
  return out.join("");
}
