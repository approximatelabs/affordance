// Converted from the chat template of RecursiveSI/flint by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let flint_text_0 = ctx["flint_text"];
  let tool_preamble_0 = ctx["tool_preamble"];
  let ns_0 = ctx["ns"];
  let tools_0 = ctx["tools"] ?? null;
  let fn_0 = ctx["fn"];
  let messages_0 = ctx["messages"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  flint_text_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_2 = k.caller;
    let t_1 = p.length > 0 ? p[0] : "t" in k ? k["t"] : undefined;
    if (J.t(J.test("string", t_1))) {
      out.push(J.str(J.filter("replace", J.filter("replace", J.filter("replace", J.filter("replace", J.filter("replace", J.filter("replace", J.filter("replace", J.filter("replace", J.filter("replace", J.filter("replace", t_1, "\u0000", ""), "<think>", ""), "</think>", ""), "<|begin_of_text|>", "&lt;|begin_of_text|&gt;"), "<|end_of_text|>", "&lt;|end_of_text|&gt;"), "<|system|>", "&lt;|system|&gt;"), "<|developer|>", "&lt;|developer|&gt;"), "<|user|>", "&lt;|user|&gt;"), "<|assistant|>", "&lt;|assistant|&gt;"), "<|observation|>", "&lt;|observation|&gt;")));
    } else {
      out.push(J.str(t_1));
    }
    return out.join("");
  };
  {
    const captured = (() => {
    const out = [];
    if (J.t(tools_0)) {
      out.push("<|system|>\n# Tools\n\nYou may call one or more functions to assist with the user query.\n\nYou are provided with function signatures within <tools></tools> XML tags:\n<tools>");
      {
        let _items1 = J.iter(tools_0);
        const _loop1 = J.loop(_items1);
        for (let _i1 = 0; _i1 < _items1.length; _i1++) {
          _loop1.index0 = _i1; const _it1 = _items1[_i1]; let tool_3 = _it1;
          let fn_4 = fn_0;
          fn_4 = (J.t((J.attr(tool_3, "function") !== undefined)) ? J.attr(tool_3, "function") : tool_3);
          out.push(J.str(J.filter("tojson", fn_4, J.kw({"ensure_ascii": false}))));
        }
      }
      out.push("</tools>\n\nFor each function call, output the function name and arguments within the following XML format:\n<tool_call>{function-name}<arg_key>{arg-key-1}</arg_key><arg_value>{arg-value-1}</arg_value>...</tool_call>\n");
    }
    return out.join("");
    })();
    tool_preamble_0 = captured;
  }
  ns_0 = J.call(J.namespace, J.kw({"tools_pending": true}));
  {
    let _items2 = J.iter(messages_0);
    const _loop2 = J.loop(_items2);
    for (let _i2 = 0; _i2 < _items2.length; _i2++) {
      _loop2.index0 = _i2; const _it2 = _items2[_i2]; let m_5 = _it2;
      if (J.t(((v) => J.t(v) ? (!J.eq(J.attr(m_5, "role"), "system")) : v)(J.attr(ns_0, "tools_pending")))) {
        out.push(J.str(tool_preamble_0));
        J.setattr(ns_0, "tools_pending", false);
      }
      if (J.t((J.eq(J.attr(m_5, "role"), "system")))) {
        out.push("<|system|>", J.str(J.call(flint_text_0, J.attr(m_5, "content"))));
      } else if (J.t((J.eq(J.attr(m_5, "role"), "developer")))) {
        out.push("<|developer|>", J.str(J.call(flint_text_0, J.attr(m_5, "content"))));
      } else if (J.t((J.eq(J.attr(m_5, "role"), "user")))) {
        out.push("<|user|>", J.str(J.call(flint_text_0, J.attr(m_5, "content"))));
      } else if (J.t((J.eq(J.attr(m_5, "role"), "assistant")))) {
        out.push("<|assistant|>");
        if (J.t(J.attr(m_5, "reasoning_content"))) {
          out.push("<think>", J.str(J.attr(m_5, "reasoning_content")), "</think>");
        }
        if (J.t(J.attr(m_5, "content"))) {
          out.push(J.str(J.attr(m_5, "content")));
        }
        if (J.t(J.attr(m_5, "tool_calls"))) {
          {
            let _items3 = J.iter(J.attr(m_5, "tool_calls"));
            const _loop3 = J.loop(_items3);
            for (let _i3 = 0; _i3 < _items3.length; _i3++) {
              _loop3.index0 = _i3; const _it3 = _items3[_i3]; let tc_6 = _it3;
              let fn_7 = fn_0;
              fn_7 = (J.t((J.attr(tc_6, "function") !== undefined)) ? J.attr(tc_6, "function") : tc_6);
              out.push("<tool_call>", J.str(J.attr(fn_7, "name")));
              {
                let _items4 = J.iter(J.call(J.attr(J.attr(fn_7, "arguments"), "items")));
                const _loop4 = J.loop(_items4);
                for (let _i4 = 0; _i4 < _items4.length; _i4++) {
                  _loop4.index0 = _i4; const _it4 = _items4[_i4]; let [k_8, v_9] = J.unpack(_it4, 2);
                  out.push("<arg_key>", J.str(k_8), "</arg_key><arg_value>", J.str((J.t(!J.t(J.test("string", v_9))) ? J.filter("tojson", v_9, J.kw({"ensure_ascii": false})) : v_9)), "</arg_value>");
                }
              }
              out.push("</tool_call>");
            }
          }
        }
      } else if (J.t((J.eq(J.attr(m_5, "role"), "tool")))) {
        out.push("<|observation|><tool_response>", J.str(J.call(flint_text_0, J.attr(m_5, "content"))), "</tool_response>");
      }
    }
  }
  if (J.t(J.attr(ns_0, "tools_pending"))) {
    out.push(J.str(tool_preamble_0));
  }
  if (J.t(add_generation_prompt_0)) {
    out.push("<|assistant|>");
  }
  return out.join("");
}
