// Converted from the chat template of sarvamai/sarvam-105b by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let visible_text_0 = ctx["visible_text"];
  let ns_0 = ctx["ns"];
  let tools_0 = ctx["tools"] ?? null;
  let messages_0 = ctx["messages"];
  let reasoning_content_0 = ctx["reasoning_content"];
  let content_0 = ctx["content"];
  let enable_thinking_0 = ctx["enable_thinking"];
  let _args_0 = ctx["_args"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  out.push(J.str("[@BOS@]\n"));
  if (J.t(tools_0)) {
    out.push("<|start_of_turn|><|tool_declare|>\n<tools>\n");
    {
      let _items1 = J.iter(tools_0);
      const _loop1 = J.loop(_items1);
      for (let _i1 = 0; _i1 < _items1.length; _i1++) {
        _loop1.index0 = _i1; const _it1 = _items1[_i1]; let tool_1 = _it1;
        out.push(J.str(J.filter("tojson", tool_1, J.kw({"ensure_ascii": false}))), "\n");
      }
    }
    out.push("</tools>", J.str("<|end_of_turn|>\n"));
  }
  visible_text_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_3 = k.caller;
    let content_2 = p.length > 0 ? p[0] : "content" in k ? k["content"] : undefined;
    if (J.t(J.test("string", content_2))) {
      out.push(J.str(content_2));
    } else if (J.t(((v) => J.t(v) ? !J.t(J.test("mapping", content_2)) : v)(J.test("iterable", content_2)))) {
      {
        let _items2 = J.iter(content_2);
        const _loop2 = J.loop(_items2);
        for (let _i2 = 0; _i2 < _items2.length; _i2++) {
          _loop2.index0 = _i2; const _it2 = _items2[_i2]; let item_4 = _it2;
          if (J.t(((v) => J.t(v) ? (J.eq(J.attr(item_4, "type"), "text")) : v)(J.test("mapping", item_4)))) {
            out.push(J.str(J.attr(item_4, "text")));
          } else if (J.t(J.test("string", item_4))) {
            out.push(J.str(item_4));
          }
        }
      }
    } else if (J.t((content_2 === null))) {
      out.push(J.str(""));
    } else {
      out.push(J.str(content_2));
    }
    return out.join("");
  };
  ns_0 = J.call(J.namespace, J.kw({"last_user_index": J.neg(1)}));
  {
    let _items3 = J.iter(messages_0);
    const _loop3 = J.loop(_items3);
    for (let _i3 = 0; _i3 < _items3.length; _i3++) {
      _loop3.index0 = _i3; const _it3 = _items3[_i3]; let m_5 = _it3;
      if (J.t((J.eq(J.attr(m_5, "role"), "user")))) {
        J.setattr(ns_0, "last_user_index", J.attr(_loop3, "index0"));
      }
    }
  }
  {
    let _items4 = J.iter(messages_0);
    const _loop4 = J.loop(_items4);
    for (let _i4 = 0; _i4 < _items4.length; _i4++) {
      _loop4.index0 = _i4; const _it4 = _items4[_i4]; let m_6 = _it4;
      let reasoning_content_7 = reasoning_content_0;
      let content_8 = content_0;
      if (J.t((J.eq(J.attr(m_6, "role"), "user")))) {
        out.push("<|start_of_turn|><|user|>\n", J.str(J.call(visible_text_0, J.attr(m_6, "content"))), J.str((J.t(((v) => J.t(v) ? !J.t(J.call(J.attr(J.call(visible_text_0, J.attr(m_6, "content")), "endswith"), "<|nothink|>")) : v)(((v) => J.t(v) ? !J.t(enable_thinking_0) : v)((enable_thinking_0 !== undefined)))) ? "<|nothink|>" : "")), J.str("<|end_of_turn|>\n"));
      } else if (J.t((J.eq(J.attr(m_6, "role"), "assistant")))) {
        out.push(J.str("<|start_of_turn|><|assistant|>\n"));
        reasoning_content_7 = "";
        content_8 = J.call(visible_text_0, J.attr(m_6, "content"));
        if (J.t(J.test("string", J.attr(m_6, "reasoning_content")))) {
          reasoning_content_7 = J.attr(m_6, "reasoning_content");
        } else {
          if (J.t((J.has("</think>", content_8)))) {
            reasoning_content_7 = J.call(J.attr(J.item(J.call(J.attr(J.call(J.attr(J.item(J.call(J.attr(content_8, "split"), "</think>"), 0), "rstrip"), "\n"), "split"), "<think>"), J.neg(1)), "lstrip"), "\n");
            content_8 = J.call(J.attr(J.item(J.call(J.attr(content_8, "split"), "</think>"), J.neg(1)), "lstrip"), "\n");
          }
        }
        if (J.t(((v) => J.t(v) ? reasoning_content_7 : v)((J.gt(J.attr(_loop4, "index0"), J.attr(ns_0, "last_user_index")))))) {
          out.push(J.str(J.add(J.add("<think>", J.call(J.attr(reasoning_content_7, "strip"))), "</think>")));
        } else {
          out.push(J.str("<think></think>"));
        }
        if (J.t(J.call(J.attr(content_8, "strip")))) {
          out.push(J.str(J.add("\n", J.call(J.attr(content_8, "strip")))));
        }
        if (J.t(J.attr(m_6, "tool_calls"))) {
          {
            let _items5 = J.iter(J.attr(m_6, "tool_calls"));
            const _loop5 = J.loop(_items5);
            for (let _i5 = 0; _i5 < _items5.length; _i5++) {
              _loop5.index0 = _i5; const _it5 = _items5[_i5]; let tc_9 = _it5;
              let _args_10 = _args_0;
              if (J.t(J.attr(tc_9, "function"))) {
                tc_9 = J.attr(tc_9, "function");
              }
              out.push(J.str(J.add("\n<tool_call>", J.attr(tc_9, "name"))), "\n");
              _args_10 = J.attr(tc_9, "arguments");
              {
                let _items6 = J.iter(J.call(J.attr(_args_10, "items")));
                const _loop6 = J.loop(_items6);
                for (let _i6 = 0; _i6 < _items6.length; _i6++) {
                  _loop6.index0 = _i6; const _it6 = _items6[_i6]; let [k_11, v_12] = J.unpack(_it6, 2);
                  out.push("<arg_key>", J.str(k_11), "</arg_key>\n<arg_value>", J.str((J.t(!J.t(J.test("string", v_12))) ? J.filter("tojson", v_12, J.kw({"ensure_ascii": false})) : v_12)), "</arg_value>\n");
                }
              }
              out.push("</tool_call>");
            }
          }
        }
        out.push(J.str("<|end_of_turn|>\n"));
      } else if (J.t((J.eq(J.attr(m_6, "role"), "tool")))) {
        if (J.t(J.test("string", J.attr(m_6, "content")))) {
          if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.sub(J.attr(_loop4, "index0"), 1)), "role"), "tool")))(J.attr(_loop4, "first")))) {
            out.push(J.str("<|start_of_turn|><|observation|>"));
          }
          out.push(J.str("\n<tool_response>\n"), J.str(J.attr(m_6, "content")), J.str("\n</tool_response>"));
        } else {
          out.push("<|start_of_turn|><|observation|>");
          {
            let _items7 = J.iter(J.attr(m_6, "content"));
            const _loop7 = J.loop(_items7);
            for (let _i7 = 0; _i7 < _items7.length; _i7++) {
              _loop7.index0 = _i7; const _it7 = _items7[_i7]; let tr_13 = _it7;
              out.push("\n<tool_response>\n", J.str((J.t((J.attr(tr_13, "output") !== undefined)) ? J.attr(tr_13, "output") : tr_13)), "\n</tool_response>");
            }
          }
        }
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.add(J.attr(_loop4, "index0"), 1)), "role"), "tool")))(J.attr(_loop4, "last")))) {
          out.push(J.str("<|end_of_turn|>\n"));
        }
      } else if (J.t((J.eq(J.attr(m_6, "role"), "system")))) {
        out.push("<|start_of_turn|><|system|>\n", J.str(J.call(visible_text_0, J.attr(m_6, "content"))), J.str("<|end_of_turn|>\n"));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|start_of_turn|><|assistant|>\n"));
  }
  return out.join("");
}
