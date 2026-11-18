// Converted from the chat template of skt/A.X-K2 by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let render_content_0 = ctx["render_content"];
  let render_tool_calls_0 = ctx["render_tool_calls"];
  let ns_0 = ctx["ns"];
  let tools_0 = ctx["tools"] ?? null;
  let messages_0 = ctx["messages"];
  let reasoning_content_0 = ctx["reasoning_content"];
  let content_0 = ctx["content"];
  let render_thinking_0 = ctx["render_thinking"];
  let show_thinking_0 = ctx["show_thinking"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let enable_thinking_0 = ctx["enable_thinking"];
  if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", tools_0), 0)) : v)(J.test("iterable", tools_0)))) {
    out.push(J.str("<|im_start|>system\n\ub2f9\uc2e0\uc740 \ub3c4\uad6c \ud638\ucd9c \uae30\ub2a5\uc744 \uac16\ucd98 \uc720\uc6a9\ud55c \ub3c4\uc6b0\ubbf8\uc785\ub2c8\ub2e4. \uc0ac\uc6a9\uc790\uc758 \uc694\uccad\uc744 \ucc98\ub9ac\ud558\uae30 \uc704\ud574\uc11c \ud544\uc694\ud55c \ub3c4\uad6c\uac00 \uc8fc\uc5b4\uc9c4 \ubaa9\ub85d\uc5d0 \uc788\ub294 \uacbd\uc6b0 \ub3c4\uad6c \ud638\ucd9c\ub85c \uc751\ub2f5\ud558\uc138\uc694.\n\ud544\uc694\ud55c \ub3c4\uad6c\uac00 \ubaa9\ub85d\uc5d0 \uc5c6\ub294 \uacbd\uc6b0\uc5d0\ub294 \ub3c4\uad6c \ud638\ucd9c \uc5c6\uc774 \uc0ac\uc6a9\uc790\uac00 \uc694\uad6c\ud55c \uc815\ubcf4\ub97c \uc81c\uacf5\ud558\uc138\uc694.\n\ud544\uc694\ud55c \ub3c4\uad6c\uac00 \ubaa9\ub85d\uc5d0 \uc788\uc9c0\ub9cc \ud574\ub2f9 \ub3c4\uad6c\ub97c \ud638\ucd9c\ud558\ub294\ub370 \ud544\uc694\ud55c argument \uc815\ubcf4\uac00 \ubd80\uc871\ud55c \uacbd\uc6b0 \ud574\ub2f9 \uc815\ubcf4\ub97c \uc0ac\uc6a9\uc790\uc5d0\uac8c \uc694\uccad\ud558\uc138\uc694.\n\uc0ac\uc6a9\uc790\uc758 \uc694\uccad\uc744 \ucc98\ub9ac\ud558\uae30 \uc704\ud574 \uc5ec\ub7ec\ubc88 \ub3c4\uad6c\ub97c \ud638\ucd9c\ud560 \uc218 \uc788\uc5b4\uc57c \ud569\ub2c8\ub2e4.\n\ub3c4\uad6c \ud638\ucd9c \uc774\ud6c4 \ub3c4\uad6c \uc2e4\ud589 \uacb0\uacfc\ub97c \uc785\ub825\uc73c\ub85c \ubc1b\uc73c\uba74 \ud574\ub2f9 \uacb0\uacfc\ub97c \ud65c\uc6a9\ud558\uc5ec \ub2f5\ubcc0\uc744 \uc0dd\uc131\ud558\uc138\uc694.\n\n\ub2e4\uc74c\uc740 \uc811\uadfc\ud560 \uc218 \uc788\ub294 \ub3c4\uad6c\ub4e4\uc758 \ubaa9\ub85d \uc785\ub2c8\ub2e4:\n<tools>\n"));
    {
      let _items1 = J.iter(tools_0);
      const _loop1 = J.loop(_items1);
      for (let _i1 = 0; _i1 < _items1.length; _i1++) {
        _loop1.index0 = _i1; const _it1 = _items1[_i1]; let t_1 = _it1;
        out.push(J.str(J.filter("tojson", t_1)), J.str("\n"));
      }
    }
    out.push(J.str("</tools>\n\n\ub3c4\uad6c\ub97c \ud638\ucd9c\ud558\ub824\uba74 \uc544\ub798\uc758 JSON\uc73c\ub85c \uc751\ub2f5\ud558\uc138\uc694.\n\ub3c4\uad6c \ud638\ucd9c \ud615\uc2dd: <tool_call>{\"name\": \ub3c4\uad6c \uc774\ub984, \"arguments\": dictionary \ud615\ud0dc\uc758 \ub3c4\uad6c \uc778\uc790\uac12}</tool_call><|im_end|>\n"));
  }
  render_content_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_3 = k.caller;
    let content_2 = p.length > 0 ? p[0] : "content" in k ? k["content"] : undefined;
    if (J.t((content_2 === null))) {
    } else if (J.t(J.test("string", content_2))) {
      out.push(J.str(content_2));
    } else if (J.t(J.test("mapping", content_2))) {
      out.push(J.str((J.t((J.attr(content_2, "text") !== undefined)) ? J.attr(content_2, "text") : content_2)));
    } else if (J.t(J.test("sequence", content_2))) {
      {
        let _items2 = J.iter(content_2);
        const _loop2 = J.loop(_items2);
        for (let _i2 = 0; _i2 < _items2.length; _i2++) {
          _loop2.index0 = _i2; const _it2 = _items2[_i2]; let part_4 = _it2;
          if (J.t(((v) => J.t(v) ? (J.attr(part_4, "text") !== undefined) : v)(J.test("mapping", part_4)))) {
            out.push(J.str(J.attr(part_4, "text")));
          } else if (J.t(J.test("string", part_4))) {
            out.push(J.str(part_4));
          }
        }
      }
    } else {
      out.push(J.str(content_2));
    }
    return out.join("");
  };
  render_tool_calls_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_6 = k.caller;
    let tool_calls_5 = p.length > 0 ? p[0] : "tool_calls" in k ? k["tool_calls"] : undefined;
    {
      let _items3 = J.iter(tool_calls_5);
      const _loop3 = J.loop(_items3);
      for (let _i3 = 0; _i3 < _items3.length; _i3++) {
        _loop3.index0 = _i3; const _it3 = _items3[_i3]; let tool_call_7 = _it3;
        if (J.t((J.attr(tool_call_7, "function") !== undefined))) {
          tool_call_7 = J.attr(tool_call_7, "function");
        }
        out.push(J.str("<tool_call>\n"), J.str(J.add(J.add("{\"name\": \"", J.attr(tool_call_7, "name")), "\"")));
        if (J.t((J.attr(tool_call_7, "arguments") !== undefined))) {
          out.push(J.str(", \"arguments\": "));
          if (J.t(J.test("string", J.attr(tool_call_7, "arguments")))) {
            out.push(J.str(J.attr(tool_call_7, "arguments")));
          } else {
            out.push(J.str(J.filter("tojson", J.attr(tool_call_7, "arguments"))));
          }
        }
        out.push(J.str("}\n</tool_call>"));
      }
    }
    return out.join("");
  };
  ns_0 = J.call(J.namespace, J.kw({"last_user_index": J.neg(1)}));
  {
    let _items4 = J.iter(messages_0);
    const _loop4 = J.loop(_items4);
    for (let _i4 = 0; _i4 < _items4.length; _i4++) {
      _loop4.index0 = _i4; const _it4 = _items4[_i4]; let m_8 = _it4;
      if (J.t((J.eq(J.attr(m_8, "role"), "user")))) {
        J.setattr(ns_0, "last_user_index", J.attr(_loop4, "index0"));
      }
    }
  }
  {
    let _items5 = J.iter(messages_0);
    const _loop5 = J.loop(_items5);
    for (let _i5 = 0; _i5 < _items5.length; _i5++) {
      _loop5.index0 = _i5; const _it5 = _items5[_i5]; let message_9 = _it5;
      let reasoning_content_10 = reasoning_content_0;
      let content_11 = content_0;
      let render_thinking_12 = render_thinking_0;
      if (J.t(((v) => J.t(v) ? v : (J.eq(J.attr(message_9, "role"), "user")))((J.eq(J.attr(message_9, "role"), "system"))))) {
        out.push(J.str(J.add(J.add("<|im_start|>", J.attr(message_9, "role")), "\n")));
        reasoning_content_10 = "";
        if (J.t(((v) => J.t(v) ? !J.t((J.attr(message_9, "reasoning_content") === null)) : v)((J.attr(message_9, "reasoning_content") !== undefined)))) {
          reasoning_content_10 = J.attr(message_9, "reasoning_content");
        } else if (J.t(((v) => J.t(v) ? !J.t((J.attr(message_9, "reasoning") === null)) : v)((J.attr(message_9, "reasoning") !== undefined)))) {
          reasoning_content_10 = J.attr(message_9, "reasoning");
        }
        if (J.t(reasoning_content_10)) {
          out.push(J.str(J.add(J.add("<think>", J.call(J.attr(reasoning_content_10, "strip"))), "</think>")));
        }
        out.push(J.str(J.call(render_content_0, J.attr(message_9, "content"))));
        if (J.t(((v) => J.t(v) ? !J.t((J.attr(message_9, "tool_calls") === null)) : v)((J.attr(message_9, "tool_calls") !== undefined)))) {
          out.push(J.str(J.call(render_tool_calls_0, J.attr(message_9, "tool_calls"))));
        }
        out.push(J.str("<|im_end|>\n"));
      } else if (J.t((J.eq(J.attr(message_9, "role"), "assistant")))) {
        out.push(J.str("<|im_start|>assistant\n"));
        reasoning_content_10 = "";
        content_11 = "";
        if (J.t(((v) => J.t(v) ? !J.t((J.attr(message_9, "content") === null)) : v)((J.attr(message_9, "content") !== undefined)))) {
          content_11 = J.call(render_content_0, J.attr(message_9, "content"));
        }
        if (J.t(((v) => J.t(v) ? !J.t((J.attr(message_9, "reasoning_content") === null)) : v)((J.attr(message_9, "reasoning_content") !== undefined)))) {
          reasoning_content_10 = J.attr(message_9, "reasoning_content");
        } else if (J.t(((v) => J.t(v) ? !J.t((J.attr(message_9, "reasoning") === null)) : v)((J.attr(message_9, "reasoning") !== undefined)))) {
          reasoning_content_10 = J.attr(message_9, "reasoning");
        } else if (J.t((J.has("</think>", content_11)))) {
          reasoning_content_10 = J.item(J.call(J.attr(J.item(J.call(J.attr(content_11, "split"), "</think>"), 0), "split"), "<think>"), J.neg(1));
          content_11 = J.call(J.attr(J.item(J.call(J.attr(content_11, "split"), "</think>"), J.neg(1)), "lstrip"), "\n");
        }
        render_thinking_12 = false;
        if (J.t(reasoning_content_10)) {
          if (J.t(((v) => J.t(v) ? show_thinking_0 : v)((show_thinking_0 !== undefined)))) {
            render_thinking_12 = true;
          } else if (J.t(!J.t(((v) => J.t(v) ? !J.t(show_thinking_0) : v)((show_thinking_0 !== undefined))))) {
            if (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? (J.gt(J.attr(_loop5, "index0"), J.attr(ns_0, "last_user_index"))) : v)((J.attr(message_9, "tool_calls") !== undefined)))(J.attr(_loop5, "last")))) {
              render_thinking_12 = true;
            }
          }
        }
        if (J.t(render_thinking_12)) {
          out.push(J.str(J.add(J.add("<think>", J.call(J.attr(reasoning_content_10, "strip"))), "</think>")));
        } else {
          out.push(J.str("</think>"));
        }
        out.push(J.str(content_11));
        if (J.t(((v) => J.t(v) ? !J.t((J.attr(message_9, "tool_calls") === null)) : v)((J.attr(message_9, "tool_calls") !== undefined)))) {
          out.push(J.str(J.call(render_tool_calls_0, J.attr(message_9, "tool_calls"))));
        }
        out.push(J.str(J.add("<|im_end|>", (J.t(((v) => J.t(v) ? v : add_generation_prompt_0)(!J.t(J.attr(_loop5, "last")))) ? "\n" : ""))));
      } else if (J.t((J.eq(J.attr(message_9, "role"), "tool")))) {
        out.push(J.str("<|im_start|>tool\n"), J.str("<tool_response>\n"), J.str(J.call(render_content_0, J.attr(message_9, "content"))), J.str("\n"), J.str("</tool_response><|im_end|>\n"));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|im_start|>assistant\n"));
    if (J.t(((v) => J.t(v) ? J.test("true", enable_thinking_0) : v)((enable_thinking_0 !== undefined)))) {
      out.push(J.str("<think>"));
    } else {
      out.push(J.str("</think>"));
    }
  }
  return out.join("");
}
