// Converted from the chat template of XingChen-AGI/Xing4.0-29B-Intent by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let visible_text_0 = ctx["visible_text"];
  let system_message_0 = ctx["system_message"];
  let loop_messages_0 = ctx["loop_messages"];
  let tools_0 = ctx["tools"] ?? null;
  let ns_0 = ctx["ns"];
  let messages_0 = ctx["messages"];
  let reasoning_content_0 = ctx["reasoning_content"];
  let content_0 = ctx["content"];
  let prev_element_0 = ctx["prev_element"];
  let enable_thinking_0 = ctx["enable_thinking"];
  let _args_0 = ctx["_args"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
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
  if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
    system_message_0 = J.item(J.item(messages_0, 0), "content");
    loop_messages_0 = J.slice(messages_0, 1, undefined, undefined);
  } else {
    loop_messages_0 = messages_0;
  }
  if (J.t(!J.t((tools_0 !== undefined)))) {
    tools_0 = [];
  }
  if (J.t((system_message_0 !== undefined))) {
    out.push(J.str(J.add("<_system>", J.call(visible_text_0, system_message_0))));
  } else {
    out.push(J.str("<_system>"));
  }
  if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", tools_0), 0)) : v)(J.test("iterable", tools_0)))) {
    out.push("\n# Tools\n\nYou may call one or more functions to assist with the user query.\n\nYou are provided with function signatures within <tools></tools> XML tags:\n<tools>\n");
    {
      let _items2 = J.iter(tools_0);
      const _loop2 = J.loop(_items2);
      for (let _i2 = 0; _i2 < _items2.length; _i2++) {
        _loop2.index0 = _i2; const _it2 = _items2[_i2]; let tool_4 = _it2;
        out.push(J.str(J.filter("tojson", tool_4, J.kw({"ensure_ascii": false}))), "\n");
      }
    }
    out.push("</tools>\n\nFor each function call, output the function name and arguments within the following XML format:\n<tool_call>{function-name}<param_key>{param-key-1}</param_key><param_value>{param-value-1}</param_value><param_key>{param-key-2}</param_key><param_value>{param-value-2}</param_value>...</tool_call>");
  }
  ns_0 = J.call(J.namespace, J.kw({"last_user_index": J.neg(1)}));
  {
    let _items3 = J.iter(loop_messages_0);
    const _loop3 = J.loop(_items3);
    for (let _i3 = 0; _i3 < _items3.length; _i3++) {
      _loop3.index0 = _i3; const _it3 = _items3[_i3]; let m_5 = _it3;
      if (J.t((J.eq(J.attr(m_5, "role"), "user")))) {
        J.setattr(ns_0, "last_user_index", J.attr(_loop3, "index0"));
      }
    }
  }
  {
    let _items4 = J.iter(loop_messages_0);
    const _loop4 = J.loop(_items4);
    for (let _i4 = 0; _i4 < _items4.length; _i4++) {
      _loop4.index0 = _i4; const _it4 = _items4[_i4]; let m_6 = _it4;
      let reasoning_content_7 = reasoning_content_0;
      let content_8 = content_0;
      let prev_element_9 = prev_element_0;
      if (J.t((J.eq(J.attr(m_6, "role"), "user")))) {
        out.push("<_user>", J.str(J.call(visible_text_0, J.attr(m_6, "content"))));
      } else if (J.t(((v) => J.t(v) ? v : (J.eq(J.attr(m_6, "role"), "bot")))((J.eq(J.attr(m_6, "role"), "assistant"))))) {
        out.push("<_bot>");
        reasoning_content_7 = "";
        content_8 = J.call(visible_text_0, J.attr(m_6, "content"));
        if (J.t(J.test("string", J.attr(m_6, "reasoning_content")))) {
          reasoning_content_7 = J.attr(m_6, "reasoning_content");
        } else if (J.t(J.test("string", J.attr(m_6, "reasoning")))) {
          reasoning_content_7 = J.attr(m_6, "reasoning");
        } else {
          if (J.t((J.has("</think>", content_8)))) {
            reasoning_content_7 = J.call(J.attr(J.item(J.call(J.attr(J.call(J.attr(J.item(J.call(J.attr(content_8, "split"), "</think>"), 0), "rstrip"), "\n"), "split"), "<think>"), J.neg(1)), "lstrip"), "\n");
            content_8 = J.call(J.attr(J.item(J.call(J.attr(content_8, "split"), "</think>"), J.neg(1)), "lstrip"), "\n");
          }
        }
        if (J.t((J.lt(J.attr(_loop4, "index0"), J.attr(ns_0, "last_user_index"))))) {
          out.push(J.str("</think>"));
        } else if (J.t(((v) => J.t(v) ? v : enable_thinking_0)(!J.t((enable_thinking_0 !== undefined))))) {
          out.push(J.str(J.add(J.add("<think>\n", J.call(J.attr(reasoning_content_7, "strip"))), "\n</think>")));
        } else {
          out.push(J.str("</think>"));
        }
        if (J.t(J.call(J.attr(content_8, "strip")))) {
          out.push(J.str(J.call(J.attr(content_8, "strip"))));
        }
        if (J.t(J.attr(m_6, "tool_calls"))) {
          {
            let _items5 = J.iter(J.attr(m_6, "tool_calls"));
            const _loop5 = J.loop(_items5);
            for (let _i5 = 0; _i5 < _items5.length; _i5++) {
              _loop5.index0 = _i5; const _it5 = _items5[_i5]; let tc_10 = _it5;
              let _args_11 = _args_0;
              if (J.t(J.attr(tc_10, "function"))) {
                tc_10 = J.attr(tc_10, "function");
              }
              out.push(J.str(J.add("<tool_call>", J.attr(tc_10, "name"))));
              _args_11 = J.attr(tc_10, "arguments");
              {
                let _items6 = J.iter(J.call(J.attr(_args_11, "items")));
                const _loop6 = J.loop(_items6);
                for (let _i6 = 0; _i6 < _items6.length; _i6++) {
                  _loop6.index0 = _i6; const _it6 = _items6[_i6]; let [k_12, v_13] = J.unpack(_it6, 2);
                  out.push("<param_key>", J.str(k_12), "</param_key><param_value>", J.str((J.t(!J.t(J.test("string", v_13))) ? J.filter("tojson", v_13, J.kw({"ensure_ascii": false})) : v_13)), "</param_value>");
                }
              }
              out.push(J.str("</tool_call>"));
            }
          }
        }
        out.push(J.str("<_end>\n"));
      } else if (J.t((J.eq(J.attr(m_6, "role"), "tool")))) {
        if (J.t((J.gt(J.attr(_loop4, "index0"), 0)))) {
          prev_element_9 = J.item(loop_messages_0, J.sub(J.attr(_loop4, "index0"), 1));
          if (J.t((!J.eq(J.attr(prev_element_9, "role"), "tool")))) {
            out.push(J.str("<_observation>"));
          }
        }
        if (J.t(J.test("string", J.attr(m_6, "content")))) {
          out.push(J.str("<tool_response>"), J.str(J.attr(m_6, "content")), J.str("</tool_response>"));
        } else {
          {
            let _items7 = J.iter(J.attr(m_6, "content"));
            const _loop7 = J.loop(_items7);
            for (let _i7 = 0; _i7 < _items7.length; _i7++) {
              _loop7.index0 = _i7; const _it7 = _items7[_i7]; let tr_14 = _it7;
              out.push(J.str("<tool_response>"), J.str((J.t((J.attr(tr_14, "output") !== undefined)) ? J.attr(tr_14, "output") : tr_14)), J.str("</tool_response>"));
            }
          }
        }
      } else if (J.t((J.eq(J.attr(m_6, "role"), "system")))) {
        out.push("<_system>", J.str(J.call(visible_text_0, J.attr(m_6, "content"))));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push("<_bot>", J.str((J.t(((v) => J.t(v) ? !J.t(enable_thinking_0) : v)((enable_thinking_0 !== undefined))) ? "</think>" : "<think>\n")));
  }
  return out.join("");
}
