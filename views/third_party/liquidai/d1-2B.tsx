// Converted from the chat template of LiquidAI/d1-2B by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let preserve_thinking_0 = ctx["preserve_thinking"];
  let enable_thinking_0 = ctx["enable_thinking"];
  let render_value_0 = ctx["render_value"];
  let parse_content_0 = ctx["parse_content"];
  let render_tool_calls_0 = ctx["render_tool_calls"];
  let tool_to_json_0 = ctx["tool_to_json"];
  let is_tool_reference_0 = ctx["is_tool_reference"];
  let referenced_tool_to_json_0 = ctx["referenced_tool_to_json"];
  let render_tool_response_0 = ctx["render_tool_response"];
  let ns_0 = ctx["ns"];
  let messages_0 = ctx["messages"];
  let tools_ns_0 = ctx["tools_ns"];
  let carry_0 = ctx["carry"];
  let bos_token_0 = ctx["bos_token"];
  let _ns_0 = ctx["_ns"];
  let func_0 = ctx["func"];
  let func_args_0 = ctx["func_args"];
  let fn_0 = ctx["fn"];
  let ns_t_0 = ctx["ns_t"];
  let tools_0 = ctx["tools"] ?? null;
  let _tl_0 = ctx["_tl"];
  let _dyn_0 = ctx["_dyn"];
  let _next_0 = ctx["_next"];
  let _merge_0 = ctx["_merge"];
  let _items_0 = ctx["_items"];
  let _c_0 = ctx["_c"];
  let _at_0 = ctx["_at"];
  let _src_0 = ctx["_src"];
  let thinking_0 = ctx["thinking"];
  let keep_thinking_0 = ctx["keep_thinking"];
  let _cfm_tag_0 = ctx["_cfm_tag"];
  let _has_cfm_0 = ctx["_has_cfm"];
  let content_0 = ctx["content"];
  let _trunc_len_0 = ctx["_trunc_len"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  out.push(J.str(bos_token_0));
  preserve_thinking_0 = J.filter("default", preserve_thinking_0, false);
  enable_thinking_0 = J.filter("default", enable_thinking_0, true);
  render_value_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_2 = k.caller;
    let v_1 = p.length > 0 ? p[0] : "v" in k ? k["v"] : undefined;
    if (J.t(J.test("string", v_1))) {
      out.push(J.str(v_1));
    } else if (J.t(J.test("boolean", v_1))) {
      out.push(J.str((J.t(v_1) ? "true" : "false")));
    } else if (J.t((v_1 === null))) {
      out.push(J.str("null"));
    } else if (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? !J.t(J.test("string", v_1)) : v)(J.test("sequence", v_1)))(J.test("mapping", v_1)))) {
      out.push(J.str(J.filter("tojson", v_1)));
    } else {
      out.push(J.str(v_1));
    }
    return out.join("");
  };
  parse_content_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_4 = k.caller;
    let content_3 = p.length > 0 ? p[0] : "content" in k ? k["content"] : undefined;
    let _ns_5 = _ns_0;
    if (J.t(J.test("string", content_3))) {
      out.push(J.str(content_3));
    } else if (J.t(((v) => J.t(v) ? (J.eq(J.call(J.attr(content_3, "get"), "type"), "image")) : v)(J.test("mapping", content_3)))) {
      out.push(J.str("<|vision_start|><|image|><|vision_end|>"));
    } else if (J.t(((v) => J.t(v) ? (J.eq(J.call(J.attr(content_3, "get"), "type"), "video")) : v)(J.test("mapping", content_3)))) {
      out.push(J.str("<|vision_start|><|video|><|vision_end|>"));
    } else if (J.t(((v) => J.t(v) ? (J.eq(J.call(J.attr(content_3, "get"), "type"), "text")) : v)(J.test("mapping", content_3)))) {
      out.push(J.str(J.filter("string", ((v) => J.t(v) ? v : "")(J.call(J.attr(content_3, "get"), "text")))));
    } else if (J.t(J.test("mapping", content_3))) {
      out.push(J.str(J.filter("tojson", content_3)));
    } else if (J.t(J.test("iterable", content_3))) {
      _ns_5 = J.call(J.namespace, J.kw({"result": ""}));
      {
        let _items1 = J.iter(content_3);
        const _loop1 = J.loop(_items1);
        for (let _i1 = 0; _i1 < _items1.length; _i1++) {
          _loop1.index0 = _i1; const _it1 = _items1[_i1]; let item_6 = _it1;
          if (J.t(J.test("string", item_6))) {
            J.setattr(_ns_5, "result", J.add(J.attr(_ns_5, "result"), item_6));
          } else if (J.t(((v) => J.t(v) ? (J.eq(J.call(J.attr(item_6, "get"), "type"), "image")) : v)(J.test("mapping", item_6)))) {
            J.setattr(_ns_5, "result", J.add(J.attr(_ns_5, "result"), "<|vision_start|><|image|><|vision_end|>"));
          } else if (J.t(((v) => J.t(v) ? (J.eq(J.call(J.attr(item_6, "get"), "type"), "video")) : v)(J.test("mapping", item_6)))) {
            J.setattr(_ns_5, "result", J.add(J.attr(_ns_5, "result"), "<|vision_start|><|video|><|vision_end|>"));
          } else if (J.t(((v) => J.t(v) ? (J.eq(J.call(J.attr(item_6, "get"), "type"), "text")) : v)(J.test("mapping", item_6)))) {
            J.setattr(_ns_5, "result", J.add(J.attr(_ns_5, "result"), J.filter("string", ((v) => J.t(v) ? v : "")(J.call(J.attr(item_6, "get"), "text")))));
          } else {
            J.setattr(_ns_5, "result", J.add(J.attr(_ns_5, "result"), J.filter("tojson", item_6)));
          }
        }
      }
      out.push(J.str(J.attr(_ns_5, "result")));
    }
    return out.join("");
  };
  render_tool_calls_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_8 = k.caller;
    let tool_calls_7 = p.length > 0 ? p[0] : "tool_calls" in k ? k["tool_calls"] : undefined;
    {
      let _items2 = J.iter(tool_calls_7);
      const _loop2 = J.loop(_items2);
      for (let _i2 = 0; _i2 < _items2.length; _i2++) {
        _loop2.index0 = _i2; const _it2 = _items2[_i2]; let tool_call_9 = _it2;
        let func_10 = func_0;
        let func_args_11 = func_args_0;
        if (J.t(!J.t(J.attr(_loop2, "first")))) {
          out.push(J.str("\n\n"));
        }
        func_10 = (J.t((J.has("function", tool_call_9))) ? J.item(tool_call_9, "function") : tool_call_9);
        func_args_11 = J.call(J.attr(func_10, "get"), "arguments");
        if (J.t(((v) => J.t(v) ? (!J.has(J.filter("trim", func_args_11), ["", "{}", "null"])) : v)(J.test("string", func_args_11)))) {
          out.push(J.str(J.call(J.raise_exception, "Tool call arguments must be a mapping, got a JSON-encoded string: parse arguments with json.loads() before applying the chat template")));
        }
        out.push(J.str(J.add(J.add("<tool_call>\n<function=", J.item(func_10, "name")), ">\n")));
        if (J.t(J.test("mapping", func_args_11))) {
          {
            let _items3 = J.iter(J.call(J.attr(func_args_11, "items")));
            const _loop3 = J.loop(_items3);
            for (let _i3 = 0; _i3 < _items3.length; _i3++) {
              _loop3.index0 = _i3; const _it3 = _items3[_i3]; let [arg_name_12, arg_value_13] = J.unpack(_it3, 2);
              out.push(J.str(J.add(J.add(J.add(J.add("<parameter=", arg_name_12), ">\n"), J.call(render_value_0, arg_value_13)), "\n</parameter>\n")));
            }
          }
        }
        out.push(J.str("</function>\n</tool_call>"));
      }
    }
    return out.join("");
  };
  tool_to_json_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_15 = k.caller;
    let tool_14 = p.length > 0 ? p[0] : "tool" in k ? k["tool"] : undefined;
    let fn_16 = fn_0;
    let ns_t_17 = ns_t_0;
    fn_16 = (J.t((J.attr(tool_14, "function") !== undefined)) ? J.attr(tool_14, "function") : tool_14);
    ns_t_17 = J.call(J.namespace, J.kw({"first": true}));
    out.push(J.str("{"));
    {
      let _items4 = J.iter(J.call(J.attr(fn_16, "items")));
      const _loop4 = J.loop(_items4);
      for (let _i4 = 0; _i4 < _items4.length; _i4++) {
        _loop4.index0 = _i4; const _it4 = _items4[_i4]; let [k_18, v_19] = J.unpack(_it4, 2);
        if (J.t((!J.eq(k_18, "defer_loading")))) {
          if (J.t(!J.t(J.attr(ns_t_17, "first")))) {
            out.push(J.str(", "));
          }
          J.setattr(ns_t_17, "first", false);
          out.push(J.str(J.filter("tojson", k_18)), J.str(": "), J.str(J.filter("tojson", v_19)));
        }
      }
    }
    out.push(J.str("}"));
    return out.join("");
  };
  is_tool_reference_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_21 = k.caller;
    let item_20 = p.length > 0 ? p[0] : "item" in k ? k["item"] : undefined;
    out.push(J.str((J.t(((v) => J.t(v) ? (J.eq(J.call(J.attr(item_20, "get"), "type"), "tool_reference")) : v)(J.test("mapping", item_20))) ? "1" : "")));
    return out.join("");
  };
  referenced_tool_to_json_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_23 = k.caller;
    let name_22 = p.length > 0 ? p[0] : "name" in k ? k["name"] : undefined;
    let _ns_24 = _ns_0;
    _ns_24 = J.call(J.namespace, J.kw({"found": null}));
    {
      let _items5 = J.iter(((v) => J.t(v) ? v : [])(tools_0));
      const _loop5 = J.loop(_items5);
      for (let _i5 = 0; _i5 < _items5.length; _i5++) {
        _loop5.index0 = _i5; const _it5 = _items5[_i5]; let tool_25 = _it5;
        let fn_26 = fn_0;
        fn_26 = (J.t((J.attr(tool_25, "function") !== undefined)) ? J.attr(tool_25, "function") : tool_25);
        if (J.t((J.eq(J.attr(fn_26, "name"), name_22)))) {
          J.setattr(_ns_24, "found", fn_26);
        }
      }
    }
    if (J.t((J.attr(_ns_24, "found") === null))) {
      out.push(J.str(J.call(J.raise_exception, J.add("tool_reference names a tool that is not in tools: ", J.filter("string", name_22)))));
    }
    out.push(J.str(J.call(tool_to_json_0, J.attr(_ns_24, "found"))));
    return out.join("");
  };
  render_tool_response_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_28 = k.caller;
    let content_27 = p.length > 0 ? p[0] : "content" in k ? k["content"] : undefined;
    if (J.t(((v) => J.t(v) ? (J.has("<tool_response>", content_27)) : v)(J.test("string", content_27)))) {
      out.push(J.str(content_27));
    } else {
      out.push(J.str("<tool_response>\n"));
      if (J.t(J.test("string", content_27))) {
        out.push(J.str(content_27));
      } else if (J.t(J.test("mapping", content_27))) {
        out.push(J.str(J.call(parse_content_0, content_27)));
      } else if (J.t(J.test("iterable", content_27))) {
        {
          let _items6 = J.iter(content_27);
          const _loop6 = J.loop(_items6);
          for (let _i6 = 0; _i6 < _items6.length; _i6++) {
            _loop6.index0 = _i6; const _it6 = _items6[_i6]; let item_29 = _it6;
            let _tl_30 = _tl_0;
            if (J.t(!J.t(J.attr(_loop6, "first")))) {
              out.push(J.str("\n"));
            }
            if (J.t(J.call(is_tool_reference_0, item_29))) {
              if (J.t(((v) => J.t(v) ? v : !J.t(J.call(is_tool_reference_0, J.attr(_loop6, "previtem"))))(J.attr(_loop6, "first")))) {
                out.push(J.str("<additional_tools>\n"));
              }
              out.push(J.str(J.call(referenced_tool_to_json_0, J.attr(item_29, "name"))));
              if (J.t(((v) => J.t(v) ? v : !J.t(J.call(is_tool_reference_0, J.attr(_loop6, "nextitem"))))(J.attr(_loop6, "last")))) {
                out.push(J.str("\n</additional_tools>"));
              }
            } else if (J.t(((v) => J.t(v) ? (J.attr(J.attr(item_29, "result"), "tools") !== undefined) : v)(((v) => J.t(v) ? J.test("mapping", J.attr(item_29, "result")) : v)(((v) => J.t(v) ? (J.attr(item_29, "result") !== undefined) : v)(J.test("mapping", item_29)))))) {
              out.push(J.str("<additional_tools>\n"));
              _tl_30 = J.attr(J.attr(item_29, "result"), "tools");
              if (J.t(J.test("string", _tl_30))) {
                out.push(J.str(_tl_30));
              } else if (J.t(J.test("mapping", _tl_30))) {
                out.push(J.str(J.call(tool_to_json_0, _tl_30)));
              } else {
                {
                  let _items7 = J.iter(_tl_30);
                  const _loop7 = J.loop(_items7);
                  for (let _i7 = 0; _i7 < _items7.length; _i7++) {
                    _loop7.index0 = _i7; const _it7 = _items7[_i7]; let t_31 = _it7;
                    if (J.t(!J.t(J.attr(_loop7, "first")))) {
                      out.push(J.str("\n"));
                    }
                    out.push(J.str(J.call(tool_to_json_0, t_31)));
                  }
                }
              }
              out.push(J.str("\n</additional_tools>"));
            } else {
              out.push(J.str(J.call(parse_content_0, item_29)));
            }
          }
        }
      }
      out.push(J.str("\n</tool_response>"));
    }
    return out.join("");
  };
  ns_0 = J.call(J.namespace, J.kw({"system_prompt": "", "last_user_index": J.neg(1)}));
  if (J.t(((v) => J.t(v) ? (J.eq(J.item(J.item(messages_0, 0), "role"), "system")) : v)(messages_0))) {
    if (J.t(J.call(J.attr(J.item(messages_0, 0), "get"), "content"))) {
      J.setattr(ns_0, "system_prompt", J.call(parse_content_0, J.item(J.item(messages_0, 0), "content")));
    }
    messages_0 = J.slice(messages_0, 1, undefined, undefined);
  }
  if (J.t(tools_0)) {
    tools_ns_0 = J.call(J.namespace, J.kw({"lines": []}));
    {
      let _items8 = J.iter(tools_0);
      const _loop8 = J.loop(_items8);
      for (let _i8 = 0; _i8 < _items8.length; _i8++) {
        _loop8.index0 = _i8; const _it8 = _items8[_i8]; let tool_32 = _it8;
        let fn_33 = fn_0;
        let _dyn_34 = _dyn_0;
        fn_33 = (J.t((J.attr(tool_32, "function") !== undefined)) ? J.attr(tool_32, "function") : tool_32);
        _dyn_34 = ((v) => J.t(v) ? v : ((v) => J.t(v) ? J.attr(tool_32, "defer_loading") : v)((J.attr(tool_32, "defer_loading") !== undefined)))(((v) => J.t(v) ? J.attr(fn_33, "defer_loading") : v)((J.attr(fn_33, "defer_loading") !== undefined)));
        if (J.t(!J.t(_dyn_34))) {
          J.setattr(tools_ns_0, "lines", J.add(J.attr(tools_ns_0, "lines"), [J.call(tool_to_json_0, fn_33)]));
        }
      }
    }
    if (J.t(J.attr(tools_ns_0, "lines"))) {
      J.setattr(ns_0, "system_prompt", J.add(J.add(J.add(J.add(J.attr(ns_0, "system_prompt"), (J.t(J.attr(ns_0, "system_prompt")) ? "\n\n" : "")), "<tools>\n"), J.filter("join", J.attr(tools_ns_0, "lines"), "\n")), "\n</tools>"));
    }
  }
  if (J.t(J.attr(ns_0, "system_prompt"))) {
    out.push(J.str(J.add(J.add("<|im_start|>system\n", J.attr(ns_0, "system_prompt")), "<|im_end|>\n")));
  }
  {
    let _items9 = J.iter(messages_0);
    const _loop9 = J.loop(_items9);
    for (let _i9 = 0; _i9 < _items9.length; _i9++) {
      _loop9.index0 = _i9; const _it9 = _items9[_i9]; let message_35 = _it9;
      if (J.t((J.eq(J.item(message_35, "role"), "user")))) {
        J.setattr(ns_0, "last_user_index", J.attr(_loop9, "index0"));
      }
    }
  }
  carry_0 = J.call(J.namespace, J.kw({"pending": []}));
  {
    let _items10 = J.iter(messages_0);
    const _loop10 = J.loop(_items10);
    for (let _i10 = 0; _i10 < _items10.length; _i10++) {
      _loop10.index0 = _i10; const _it10 = _items10[_i10]; let message_36 = _it10;
      let _next_37 = _next_0;
      let _merge_38 = _merge_0;
      let _items_39 = _items_0;
      let _c_40 = _c_0;
      let _at_41 = _at_0;
      let _src_42 = _src_0;
      _next_37 = (J.t(!J.t(J.attr(_loop10, "last"))) ? J.item(messages_0, J.add(J.attr(_loop10, "index0"), 1)) : null);
      _merge_38 = ((v) => J.t(v) ? (J.eq(J.call(J.attr(_next_37, "get"), "tool_call_id"), J.call(J.attr(message_36, "get"), "tool_call_id"))) : v)(((v) => J.t(v) ? J.call(J.attr(message_36, "get"), "tool_call_id") : v)(((v) => J.t(v) ? !J.t(J.call(J.attr(_next_37, "get"), "additional_tools")) : v)(((v) => J.t(v) ? (J.eq(J.attr(_next_37, "role"), "tool")) : v)(((v) => J.t(v) ? !J.t((_next_37 === null)) : v)(((v) => J.t(v) ? !J.t(J.call(J.attr(message_36, "get"), "additional_tools")) : v)((J.eq(J.attr(message_36, "role"), "tool"))))))));
      _items_39 = [];
      if (J.t(((v) => J.t(v) ? !J.t(J.call(J.attr(message_36, "get"), "additional_tools")) : v)((J.eq(J.attr(message_36, "role"), "tool"))))) {
        _c_40 = J.call(J.attr(message_36, "get"), "content");
        if (J.t(J.test("string", _c_40))) {
          _items_39 = (J.t(_c_40) ? [({"type": "text", "text": _c_40})] : []);
        } else if (J.t(J.test("mapping", _c_40))) {
          _items_39 = [_c_40];
        } else if (J.t(_c_40)) {
          _items_39 = J.filter("list", _c_40);
        }
      }
      if (J.t(_merge_38)) {
        J.setattr(carry_0, "pending", J.add(J.attr(carry_0, "pending"), _items_39));
      } else {
        out.push(J.str(J.add(J.add("<|im_start|>", J.attr(message_36, "role")), "\n")));
        if (J.t((J.eq(J.attr(message_36, "role"), "assistant")))) {
          {
            thinking_0 = (J.t((J.attr(message_36, "thinking") !== undefined)) ? J.attr(message_36, "thinking") : (J.t((J.attr(message_36, "reasoning") !== undefined)) ? J.attr(message_36, "reasoning") : J.attr(message_36, "reasoning_content")));
            keep_thinking_0 = ((v) => J.t(v) ? v : (J.gt(J.attr(_loop10, "index0"), J.attr(ns_0, "last_user_index"))))(preserve_thinking_0);
            if (J.t(((v) => J.t(v) ? keep_thinking_0 : v)((thinking_0 !== undefined)))) {
              out.push(J.str(J.add(J.add("<think>", ((v) => J.t(v) ? v : "")(thinking_0)), "</think>")));
            }
            _cfm_tag_0 = "CONTINUE_FINAL_MESSAGE_TAG ";
            _has_cfm_0 = false;
            content_0 = "";
            if (J.t(J.call(J.attr(message_36, "get"), "content"))) {
              content_0 = J.call(parse_content_0, J.attr(message_36, "content"));
            }
            if (J.t(!J.t(keep_thinking_0))) {
              if (J.t((J.has("</think>", content_0)))) {
                content_0 = J.filter("trim", J.item(J.call(J.attr(content_0, "split"), "</think>"), J.neg(1)));
              }
              content_0 = J.add("<think></think>", content_0);
            }
            if (J.t(J.call(J.attr(content_0, "endswith"), _cfm_tag_0))) {
              _has_cfm_0 = true;
              _trunc_len_0 = J.sub(J.filter("length", content_0), J.filter("length", _cfm_tag_0));
              content_0 = J.slice(content_0, undefined, _trunc_len_0, undefined);
            }
            out.push(J.str(content_0));
            if (J.t(J.attr(message_36, "tool_calls"))) {
              out.push(J.str(J.call(render_tool_calls_0, J.attr(message_36, "tool_calls"))));
            }
            if (J.t(_has_cfm_0)) {
              out.push(J.str(_cfm_tag_0));
            }
            out.push(J.str("<|im_end|>\n"));
          }
        } else if (J.t((J.eq(J.attr(message_36, "role"), "tool")))) {
          if (J.t(J.call(J.attr(message_36, "get"), "additional_tools"))) {
            _at_41 = J.item(message_36, "additional_tools");
            if (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? !J.t(J.test("string", _at_41)) : v)(J.test("sequence", _at_41)))(J.test("mapping", _at_41)))) {
              _src_42 = _at_41;
            } else {
              _src_42 = J.call(J.attr(message_36, "get"), "content");
            }
            if (J.t(((v) => J.t(v) ? (J.has("<tool_response>", _src_42)) : v)(J.test("string", _src_42)))) {
              out.push(J.str(_src_42));
            } else if (J.t(((v) => J.t(v) ? (J.has("<additional_tools>", _src_42)) : v)(J.test("string", _src_42)))) {
              out.push(J.str(J.add(J.add("<tool_response>\n", _src_42), "\n</tool_response>")));
            } else if (J.t(_src_42)) {
              out.push(J.str("<tool_response>\n<additional_tools>\n"));
              if (J.t(J.test("string", _src_42))) {
                out.push(J.str(_src_42));
              } else if (J.t(J.test("mapping", _src_42))) {
                out.push(J.str(J.call(tool_to_json_0, _src_42)));
              } else {
                {
                  let _items11 = J.iter(_src_42);
                  const _loop11 = J.loop(_items11);
                  for (let _i11 = 0; _i11 < _items11.length; _i11++) {
                    _loop11.index0 = _i11; const _it11 = _items11[_i11]; let t_43 = _it11;
                    if (J.t(!J.t(J.attr(_loop11, "first")))) {
                      out.push(J.str("\n"));
                    }
                    out.push(J.str(J.call(tool_to_json_0, t_43)));
                  }
                }
              }
              out.push(J.str("\n</additional_tools>\n</tool_response>"));
            }
          } else if (J.t(J.attr(carry_0, "pending"))) {
            out.push(J.str(J.call(render_tool_response_0, J.add(J.attr(carry_0, "pending"), _items_39))));
            J.setattr(carry_0, "pending", []);
          } else {
            out.push(J.str(J.call(render_tool_response_0, ((v) => J.t(v) ? v : "")(J.call(J.attr(message_36, "get"), "content")))));
          }
          out.push(J.str("<|im_end|>\n"));
        } else {
          if (J.t(J.call(J.attr(message_36, "get"), "content"))) {
            out.push(J.str(J.call(parse_content_0, J.item(message_36, "content"))));
          }
          out.push(J.str("<|im_end|>\n"));
        }
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|im_start|>assistant\n"));
    if (J.t(enable_thinking_0)) {
      out.push(J.str("<think>"));
    } else {
      out.push(J.str("<think></think>"));
    }
  }
  return out.join("");
}
