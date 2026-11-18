// Converted from the chat template of dealignai/Gemma-4-31B-JANG_4M-CRACK by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let format_parameters_0 = ctx["format_parameters"];
  let format_function_declaration_0 = ctx["format_function_declaration"];
  let format_argument_0 = ctx["format_argument"];
  let strip_thinking_0 = ctx["strip_thinking"];
  let ns_0 = ctx["ns"];
  let loop_messages_0 = ctx["loop_messages"];
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
  let function_0 = ctx["function"];
  let ns_args_0 = ctx["ns_args"];
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
          if (J.t(J.item(value_7, "nullable"))) {
            if (J.t(add_comma_8)) {
              out.push(",");
            } else {
              add_comma_8 = true;
            }
            out.push("nullable:true");
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
                  _loop2.index0 = _i2; const _it2 = _items2[_i2]; let item_10 = _it2;
                  out.push("<|\"|>", J.str(item_10), "<|\"|>");
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
              ns_items_9 = J.call(J.namespace, J.kw({"found_first": false}));
              {
                let _items3 = J.iter(J.filter("dictsort", J.item(value_7, "items")));
                const _loop3 = J.loop(_items3);
                for (let _i3 = 0; _i3 < _items3.length; _i3++) {
                  _loop3.index0 = _i3; const _it3 = _items3[_i3]; let [item_key_11, item_value_12] = J.unpack(_it3, 2);
                  if (J.t(!J.t((item_value_12 === null)))) {
                    if (J.t(J.attr(ns_items_9, "found_first"))) {
                      out.push(",");
                    }
                    J.setattr(ns_items_9, "found_first", true);
                    if (J.t((J.eq(item_key_11, "properties")))) {
                      out.push("properties:{");
                      if (J.t(J.test("mapping", item_value_12))) {
                        out.push(J.str(J.call(format_parameters_0, item_value_12, J.filter("default", J.item(J.item(value_7, "items"), "required"), []))));
                      }
                      out.push("}");
                    } else if (J.t((J.eq(item_key_11, "required")))) {
                      out.push("required:[");
                      {
                        let _items4 = J.iter(item_value_12);
                        const _loop4 = J.loop(_items4);
                        for (let _i4 = 0; _i4 < _items4.length; _i4++) {
                          _loop4.index0 = _i4; const _it4 = _items4[_i4]; let req_item_13 = _it4;
                          out.push("<|\"|>", J.str(req_item_13), "<|\"|>");
                          if (J.t(!J.t(J.attr(_loop4, "last")))) {
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
  ns_0 = J.call(J.namespace, J.kw({"prev_message_type": null}));
  loop_messages_0 = messages_0;
  out.push(J.str(bos_token_0));
  if (J.t(((v) => J.t(v) ? v : (J.has(J.item(J.item(messages_0, 0), "role"), ["system", "developer"])))(((v) => J.t(v) ? v : tools_0)(((v) => J.t(v) ? enable_thinking_0 : v)((enable_thinking_0 !== undefined)))))) {
    out.push(J.str("<|turn>system\n"));
    if (J.t(((v) => J.t(v) ? enable_thinking_0 : v)((enable_thinking_0 !== undefined)))) {
      out.push(J.str("<|think|>"));
      J.setattr(ns_0, "prev_message_type", "think");
    }
    if (J.t((J.has(J.item(J.item(messages_0, 0), "role"), ["system", "developer"])))) {
      out.push(J.str(J.filter("trim", J.item(J.item(messages_0, 0), "content"))));
      loop_messages_0 = J.slice(messages_0, 1, undefined, undefined);
    }
    if (J.t(tools_0)) {
      {
        let _items9 = J.iter(tools_0);
        const _loop9 = J.loop(_items9);
        for (let _i9 = 0; _i9 < _items9.length; _i9++) {
          _loop9.index0 = _i9; const _it9 = _items9[_i9]; let tool_30 = _it9;
          out.push(J.str("<|tool>"), J.str(J.filter("trim", J.call(format_function_declaration_0, tool_30))), J.str("<tool|>"));
        }
      }
      J.setattr(ns_0, "prev_message_type", "tool");
    }
    out.push(J.str("<turn|>\n"));
  }
  {
    let _items10 = J.iter(loop_messages_0);
    const _loop10 = J.loop(_items10);
    for (let _i10 = 0; _i10 < _items10.length; _i10++) {
      _loop10.index0 = _i10; const _it10 = _items10[_i10]; let message_31 = _it10;
      let role_32 = role_0;
      J.setattr(ns_0, "prev_message_type", null);
      role_32 = (J.t((J.eq(J.item(message_31, "role"), "assistant"))) ? "model" : J.item(message_31, "role"));
      out.push(J.str(J.add(J.add("<|turn>", role_32), "\n")));
      if (J.t(J.item(message_31, "tool_calls"))) {
        {
          let _items11 = J.iter(J.item(message_31, "tool_calls"));
          const _loop11 = J.loop(_items11);
          for (let _i11 = 0; _i11 < _items11.length; _i11++) {
            _loop11.index0 = _i11; const _it11 = _items11[_i11]; let tool_call_33 = _it11;
            let function_34 = function_0;
            let ns_args_35 = ns_args_0;
            function_34 = J.item(tool_call_33, "function");
            out.push(J.str(J.add(J.add("<|tool_call>call:", J.item(function_34, "name")), "{")));
            if (J.t(J.test("mapping", J.item(function_34, "arguments")))) {
              ns_args_35 = J.call(J.namespace, J.kw({"found_first": false}));
              {
                let _items12 = J.iter(J.filter("dictsort", J.item(function_34, "arguments")));
                const _loop12 = J.loop(_items12);
                for (let _i12 = 0; _i12 < _items12.length; _i12++) {
                  _loop12.index0 = _i12; const _it12 = _items12[_i12]; let [key_36, value_37] = J.unpack(_it12, 2);
                  if (J.t(J.attr(ns_args_35, "found_first"))) {
                    out.push(",");
                  }
                  J.setattr(ns_args_35, "found_first", true);
                  out.push(J.str(key_36), ":", J.str(J.call(format_argument_0, value_37, J.kw({"escape_keys": false}))));
                }
              }
            } else if (J.t(J.test("string", J.item(function_34, "arguments")))) {
              out.push(J.str(J.item(function_34, "arguments")));
            }
            out.push(J.str("}<tool_call|>"));
          }
        }
        J.setattr(ns_0, "prev_message_type", "tool_call");
      }
      if (J.t(J.item(message_31, "tool_responses"))) {
        {
          let _items13 = J.iter(J.item(message_31, "tool_responses"));
          const _loop13 = J.loop(_items13);
          for (let _i13 = 0; _i13 < _items13.length; _i13++) {
            _loop13.index0 = _i13; const _it13 = _items13[_i13]; let tool_response_38 = _it13;
            out.push(J.str("<|tool_response>"));
            if (J.t(J.test("mapping", J.item(tool_response_38, "response")))) {
              out.push(J.str(J.add(J.add("response:", J.filter("default", J.item(tool_response_38, "name"), "unknown")), "{")));
              {
                let _items14 = J.iter(J.filter("dictsort", J.item(tool_response_38, "response")));
                const _loop14 = J.loop(_items14);
                for (let _i14 = 0; _i14 < _items14.length; _i14++) {
                  _loop14.index0 = _i14; const _it14 = _items14[_i14]; let [key_39, value_40] = J.unpack(_it14, 2);
                  out.push(J.str(key_39), ":", J.str(J.call(format_argument_0, value_40, J.kw({"escape_keys": false}))));
                  if (J.t(!J.t(J.attr(_loop14, "last")))) {
                    out.push(",");
                  }
                }
              }
              out.push(J.str("}"));
            } else {
              out.push(J.str(J.add(J.add(J.add(J.add("response:", J.filter("default", J.item(tool_response_38, "name"), "unknown")), "{value:"), J.call(format_argument_0, J.item(tool_response_38, "response"), J.kw({"escape_keys": false}))), "}")));
            }
            out.push(J.str("<tool_response|>"));
          }
        }
        J.setattr(ns_0, "prev_message_type", "tool_response");
      }
      if (J.t(J.test("string", J.item(message_31, "content")))) {
        if (J.t((J.eq(role_32, "model")))) {
          out.push(J.str(J.call(strip_thinking_0, J.item(message_31, "content"))));
        } else {
          out.push(J.str(J.filter("trim", J.item(message_31, "content"))));
        }
      } else if (J.t(J.test("sequence", J.item(message_31, "content")))) {
        {
          let _items15 = J.iter(J.item(message_31, "content"));
          const _loop15 = J.loop(_items15);
          for (let _i15 = 0; _i15 < _items15.length; _i15++) {
            _loop15.index0 = _i15; const _it15 = _items15[_i15]; let item_41 = _it15;
            if (J.t((J.eq(J.item(item_41, "type"), "text")))) {
              if (J.t((J.eq(role_32, "model")))) {
                out.push(J.str(J.call(strip_thinking_0, J.item(item_41, "text"))));
              } else {
                out.push(J.str(J.filter("trim", J.item(item_41, "text"))));
              }
            } else if (J.t((J.eq(J.item(item_41, "type"), "image")))) {
              out.push(J.str("\n\n<|image|>\n\n"));
              J.setattr(ns_0, "prev_message_type", "image");
            } else if (J.t((J.eq(J.item(item_41, "type"), "audio")))) {
              out.push(J.str("<|audio|>"));
              J.setattr(ns_0, "prev_message_type", "audio");
            } else if (J.t((J.eq(J.item(item_41, "type"), "video")))) {
              out.push(J.str("\n\n<|video|>\n\n"));
              J.setattr(ns_0, "prev_message_type", "video");
            }
          }
        }
      }
      if (J.t(!J.t(((v) => J.t(v) ? !J.t(J.item(message_31, "content")) : v)(J.item(message_31, "tool_responses"))))) {
        out.push(J.str("<turn|>\n"));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    if (J.t((!J.eq(J.attr(ns_0, "prev_message_type"), "tool_response")))) {
      out.push(J.str("<|turn>model\n"));
    }
    if (J.t(!J.t(J.filter("default", enable_thinking_0, false)))) {
      out.push(J.str("<|channel>thought\n<channel|>"));
    }
  }
  return out.join("");
}
