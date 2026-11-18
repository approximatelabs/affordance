// Converted from the chat template of Motif-Technologies/Motif-3 by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let visible_text_0 = ctx["visible_text"];
  let ns_0 = ctx["ns"];
  let messages_0 = ctx["messages"];
  let tools_0 = ctx["tools"] ?? null;
  let _props_0 = ctx["_props"];
  let _keys_0 = ctx["_keys"];
  let _content_0 = ctx["_content"];
  let _reasoning_0 = ctx["_reasoning"];
  let _has_tools_0 = ctx["_has_tools"];
  let _emit_think_0 = ctx["_emit_think"];
  let _is_last_assistant_0 = ctx["_is_last_assistant"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let _tc_id_0 = ctx["_tc_id"];
  let _id_suffix_0 = ctx["_id_suffix"];
  let null_0 = ctx["null"];
  let enable_thinking_0 = ctx["enable_thinking"];
  visible_text_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_2 = k.caller;
    let content_1 = p.length > 0 ? p[0] : "content" in k ? k["content"] : undefined;
    if (J.t(J.test("string", content_1))) {
      out.push(J.str(content_1));
    } else if (J.t(((v) => J.t(v) ? !J.t(J.test("mapping", content_1)) : v)(J.test("iterable", content_1)))) {
      {
        let _items1 = J.iter(content_1);
        const _loop1 = J.loop(_items1);
        for (let _i1 = 0; _i1 < _items1.length; _i1++) {
          _loop1.index0 = _i1; const _it1 = _items1[_i1]; let item_3 = _it1;
          if (J.t(((v) => J.t(v) ? (J.eq(J.attr(item_3, "type"), "text")) : v)(J.test("mapping", item_3)))) {
            out.push(J.str(J.attr(item_3, "text")));
          } else if (J.t(J.test("string", item_3))) {
            out.push(J.str(item_3));
          }
        }
      }
    } else {
      out.push(J.str(content_1));
    }
    return out.join("");
  };
  out.push(J.str("<|beginoftext|>"));
  ns_0 = J.call(J.namespace, J.kw({"has_system": false, "last_user_index": J.neg(1), "last_assistant_index": J.neg(1)}));
  if (J.t(((v) => J.t(v) ? (J.eq(J.attr(J.item(messages_0, 0), "role"), "system")) : v)((J.gt(J.filter("length", messages_0), 0))))) {
    J.setattr(ns_0, "has_system", true);
  }
  {
    let _items2 = J.iter(messages_0);
    const _loop2 = J.loop(_items2);
    for (let _i2 = 0; _i2 < _items2.length; _i2++) {
      _loop2.index0 = _i2; const _it2 = _items2[_i2]; let m_4 = _it2;
      if (J.t((J.eq(J.attr(m_4, "role"), "user")))) {
        J.setattr(ns_0, "last_user_index", J.attr(_loop2, "index0"));
      } else if (J.t((J.eq(J.attr(m_4, "role"), "assistant")))) {
        J.setattr(ns_0, "last_assistant_index", J.attr(_loop2, "index0"));
      }
    }
  }
  if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", tools_0), 0)) : v)(J.test("iterable", tools_0)))) {
    out.push(J.str("<|startofturn|><|system|>"), J.str("# Tools\n\nYou may call one or more functions to assist with the user query.\n\n"), J.str("You are provided with function signatures within <tools></tools> XML tags:\n\n<tools>"));
    {
      let _items3 = J.iter(tools_0);
      const _loop3 = J.loop(_items3);
      for (let _i3 = 0; _i3 < _items3.length; _i3++) {
        _loop3.index0 = _i3; const _it3 = _items3[_i3]; let tool_5 = _it3;
        if (J.t((J.attr(tool_5, "function") !== undefined))) {
          tool_5 = J.attr(tool_5, "function");
        }
        out.push(J.str(J.concat("\n", J.filter("tojson", tool_5))));
      }
    }
    out.push(J.str("\n</tools>"), J.str("\n\nFor each function call, output in JSON within <tool_call> tags:\n"));
    {
      let _items4 = J.iter(tools_0);
      const _loop4 = J.loop(_items4);
      for (let _i4 = 0; _i4 < _items4.length; _i4++) {
        _loop4.index0 = _i4; const _it4 = _items4[_i4]; let tool_6 = _it4;
        let _props_7 = _props_0;
        let _keys_8 = _keys_0;
        if (J.t((J.attr(tool_6, "function") !== undefined))) {
          tool_6 = J.attr(tool_6, "function");
        }
        _props_7 = (J.t(((v) => J.t(v) ? (J.attr(J.attr(tool_6, "parameters"), "properties") !== undefined) : v)((J.attr(tool_6, "parameters") !== undefined))) ? J.attr(J.attr(tool_6, "parameters"), "properties") : ({}));
        out.push(J.str(J.concat("\n<tool_call>{\"name\": \"", J.attr(tool_6, "name"), "\", \"arguments\": {")));
        _keys_8 = J.filter("list", _props_7);
        {
          let _items5 = J.iter(_keys_8);
          const _loop5 = J.loop(_items5);
          for (let _i5 = 0; _i5 < _items5.length; _i5++) {
            _loop5.index0 = _i5; const _it5 = _items5[_i5]; let k_9 = _it5;
            out.push(J.str(J.concat("\"", k_9, "\": <", k_9, ">")));
            if (J.t(!J.t(J.attr(_loop5, "last")))) {
              out.push(J.str(", "));
            }
          }
        }
        out.push(J.str("}}</tool_call>"));
      }
    }
    if (J.t(J.attr(ns_0, "has_system"))) {
      out.push(J.str(J.concat("\n\n", J.call(visible_text_0, J.attr(J.item(messages_0, 0), "content")))));
    }
    out.push(J.str("<|endofturn|>"));
  } else if (J.t(J.attr(ns_0, "has_system"))) {
    out.push(J.str(J.concat("<|startofturn|><|system|>", J.call(visible_text_0, J.attr(J.item(messages_0, 0), "content")), "<|endofturn|>")));
  }
  {
    let _items6 = J.iter(messages_0);
    const _loop6 = J.loop(_items6);
    for (let _i6 = 0; _i6 < _items6.length; _i6++) {
      _loop6.index0 = _i6; const _it6 = _items6[_i6]; let m_10 = _it6;
      let _content_11 = _content_0;
      let _reasoning_12 = _reasoning_0;
      let _has_tools_13 = _has_tools_0;
      let _emit_think_14 = _emit_think_0;
      let _is_last_assistant_15 = _is_last_assistant_0;
      if (J.t(((v) => J.t(v) ? (J.eq(J.attr(m_10, "role"), "system")) : v)((J.eq(J.attr(_loop6, "index0"), 0))))) {
      } else if (J.t((J.eq(J.attr(m_10, "role"), "system")))) {
        out.push(J.str(J.concat("<|startofturn|><|system|>", J.call(visible_text_0, J.attr(m_10, "content")), "<|endofturn|>")));
      } else if (J.t((J.eq(J.attr(m_10, "role"), "user")))) {
        out.push(J.str("<|startofturn|><|user|>"));
        if (J.t(((v) => J.t(v) ? J.attr(m_10, "references") : v)((J.attr(m_10, "references") !== undefined)))) {
          out.push(J.str(J.concat("<|reference|>", J.attr(m_10, "references"), "\n")));
        }
        out.push(J.str(J.call(visible_text_0, J.attr(m_10, "content"))), J.str("<|endofturn|>"));
      } else if (J.t((J.eq(J.attr(m_10, "role"), "assistant")))) {
        out.push(J.str("<|startofturn|><|assistant|>"));
        _content_11 = J.call(visible_text_0, J.attr(m_10, "content"));
        _reasoning_12 = "";
        if (J.t(J.test("string", J.attr(m_10, "reasoning_content")))) {
          _reasoning_12 = J.attr(m_10, "reasoning_content");
        } else if (J.t((J.has("</think>", _content_11)))) {
          _reasoning_12 = J.call(J.attr(J.item(J.call(J.attr(J.item(J.call(J.attr(_content_11, "split"), "</think>"), 0), "split"), "<think>"), J.neg(1)), "strip"));
          _content_11 = J.call(J.attr(J.item(J.call(J.attr(_content_11, "split"), "</think>", 1), J.neg(1)), "lstrip"), "\n");
        }
        _has_tools_13 = ((v) => J.t(v) ? (J.gt(J.filter("length", tools_0), 0)) : v)(((v) => J.t(v) ? J.test("iterable", tools_0) : v)((tools_0 !== undefined)));
        _emit_think_14 = ((v) => J.t(v) ? ((v) => J.t(v) ? v : (J.eq(J.attr(_loop6, "index0"), J.attr(ns_0, "last_assistant_index"))))(_has_tools_13) : v)(_reasoning_12);
        if (J.t(_emit_think_14)) {
          out.push(J.str(J.concat("<think>", J.call(J.attr(_reasoning_12, "strip")), "</think>")));
        }
        if (J.t(J.call(J.attr(_content_11, "strip")))) {
          out.push(J.str(J.call(J.attr(_content_11, "strip"))));
        }
        if (J.t(((v) => J.t(v) ? J.attr(m_10, "tool_calls") : v)((J.attr(m_10, "tool_calls") !== undefined)))) {
          _is_last_assistant_15 = ((v) => J.t(v) ? !J.t(add_generation_prompt_0) : v)((J.eq(J.attr(_loop6, "index0"), J.attr(ns_0, "last_assistant_index"))));
          {
            let _items7 = J.iter(J.attr(m_10, "tool_calls"));
            const _loop7 = J.loop(_items7);
            for (let _i7 = 0; _i7 < _items7.length; _i7++) {
              _loop7.index0 = _i7; const _it7 = _items7[_i7]; let tc_16 = _it7;
              let _tc_id_17 = _tc_id_0;
              let _id_suffix_18 = _id_suffix_0;
              _tc_id_17 = (J.t(((v) => J.t(v) ? !J.t(_is_last_assistant_15) : v)((J.attr(tc_16, "id") !== undefined))) ? J.attr(tc_16, "id") : null);
              if (J.t((J.attr(tc_16, "function") !== undefined))) {
                tc_16 = J.attr(tc_16, "function");
              }
              _id_suffix_18 = (J.t(!J.t((_tc_id_17 === null))) ? J.concat(", \"id\": ", J.filter("tojson", _tc_id_17)) : "");
              if (J.t(((v) => J.t(v) ? v : (J.eq(J.attr(tc_16, "arguments"), "")))(((v) => J.t(v) ? v : !J.t(J.attr(tc_16, "arguments")))(!J.t((J.attr(tc_16, "arguments") !== undefined)))))) {
                out.push(J.str(J.concat("\n<tool_call>", "{\"name\": \"", J.attr(tc_16, "name"), "\", \"arguments\": ", null_0, _id_suffix_18, "}", "</tool_call>")));
              } else if (J.t(J.test("string", J.attr(tc_16, "arguments")))) {
                out.push(J.str(J.concat("\n<tool_call>", "{\"name\": \"", J.attr(tc_16, "name"), "\", \"arguments\": ", J.attr(tc_16, "arguments"), _id_suffix_18, "}", "</tool_call>")));
              } else {
                out.push(J.str(J.concat("\n<tool_call>", "{\"name\": \"", J.attr(tc_16, "name"), "\", \"arguments\": ", J.filter("tojson", J.attr(tc_16, "arguments")), _id_suffix_18, "}", "</tool_call>")));
              }
            }
          }
        }
        out.push(J.str("<|endofturn|>"));
      } else if (J.t((J.eq(J.attr(m_10, "role"), "tool")))) {
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.attr(_loop6, "previtem"), "role"), "tool")))(J.attr(_loop6, "first")))) {
          out.push(J.str("<|startofturn|><|tool|>"));
        }
        out.push(J.str(J.concat("<tool_response>", J.filter("tojson", ({"tool_call_id": J.attr(m_10, "tool_call_id"), "content": J.attr(m_10, "content")})), "</tool_response>")));
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.attr(_loop6, "nextitem"), "role"), "tool")))(J.attr(_loop6, "last")))) {
          out.push(J.str("<|endofturn|>"));
        }
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|startofturn|><|assistant|>"));
    if (J.t(((v) => J.t(v) ? !J.t(enable_thinking_0) : v)((enable_thinking_0 !== undefined)))) {
      out.push(J.str("<think></think>"));
    } else {
      out.push(J.str("<think>"));
    }
  } else {
    out.push(J.str("<|endoftext|>"));
  }
  return out.join("");
}
