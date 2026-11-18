// Converted from the chat template of MiniMaxAI/MiniMax-M2.7 by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let toolcall_begin_token_0 = ctx["toolcall_begin_token"];
  let toolcall_end_token_0 = ctx["toolcall_end_token"];
  let render_tool_namespace_0 = ctx["render_tool_namespace"];
  let visible_text_0 = ctx["visible_text"];
  let build_system_message_0 = ctx["build_system_message"];
  let system_message_0 = ctx["system_message"];
  let conversation_messages_0 = ctx["conversation_messages"];
  let ns_0 = ctx["ns"];
  let last_tool_call_0 = ctx["last_tool_call"];
  let model_identity_0 = ctx["model_identity"];
  let messages_0 = ctx["messages"];
  let tools_0 = ctx["tools"] ?? null;
  let reasoning_content_0 = ctx["reasoning_content"];
  let content_0 = ctx["content"];
  let _args_0 = ctx["_args"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  toolcall_begin_token_0 = "<minimax:tool_call>";
  toolcall_end_token_0 = "</minimax:tool_call>";
  render_tool_namespace_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_3 = k.caller;
    let namespace_name_1 = p.length > 0 ? p[0] : "namespace_name" in k ? k["namespace_name"] : undefined;
    let tool_list_2 = p.length > 1 ? p[1] : "tool_list" in k ? k["tool_list"] : undefined;
    {
      let _items1 = J.iter(tool_list_2);
      const _loop1 = J.loop(_items1);
      for (let _i1 = 0; _i1 < _items1.length; _i1++) {
        _loop1.index0 = _i1; const _it1 = _items1[_i1]; let tool_4 = _it1;
        out.push("<tool>", J.str(J.filter("tojson", J.attr(tool_4, "function"), J.kw({"ensure_ascii": false}))), "</tool>\n");
      }
    }
    return out.join("");
  };
  visible_text_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_6 = k.caller;
    let content_5 = p.length > 0 ? p[0] : "content" in k ? k["content"] : undefined;
    if (J.t(J.test("string", content_5))) {
      out.push(J.str(content_5));
    } else if (J.t(((v) => J.t(v) ? !J.t(J.test("mapping", content_5)) : v)(J.test("iterable", content_5)))) {
      {
        let _items2 = J.iter(content_5);
        const _loop2 = J.loop(_items2);
        for (let _i2 = 0; _i2 < _items2.length; _i2++) {
          _loop2.index0 = _i2; const _it2 = _items2[_i2]; let item_7 = _it2;
          if (J.t(((v) => J.t(v) ? (J.eq(J.attr(item_7, "type"), "text")) : v)(J.test("mapping", item_7)))) {
            out.push(J.str(J.attr(item_7, "text")));
          } else if (J.t(J.test("string", item_7))) {
            out.push(J.str(item_7));
          }
        }
      }
    } else {
      out.push(J.str(content_5));
    }
    return out.join("");
  };
  build_system_message_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_9 = k.caller;
    let system_message_8 = p.length > 0 ? p[0] : "system_message" in k ? k["system_message"] : undefined;
    let model_identity_10 = model_identity_0;
    if (J.t(((v) => J.t(v) ? J.attr(system_message_8, "content") : v)(system_message_8))) {
      out.push(J.str(J.call(visible_text_0, J.attr(system_message_8, "content"))));
    } else {
      if (J.t(!J.t((model_identity_10 !== undefined)))) {
        model_identity_10 = "You are a helpful assistant. Your name is MiniMax-M2.7 and is built by MiniMax.";
      }
      out.push(J.str(model_identity_10));
    }
    if (J.t(((v) => J.t(v) ? J.attr(system_message_8, "current_date") : v)(system_message_8))) {
      out.push(J.str(J.add(J.concat("\n", "Current date: "), J.attr(system_message_8, "current_date"))));
    }
    if (J.t(((v) => J.t(v) ? J.attr(system_message_8, "current_location") : v)(system_message_8))) {
      out.push(J.str(J.add(J.concat("\n", "Current location: "), J.attr(system_message_8, "current_location"))));
    }
    return out.join("");
  };
  system_message_0 = null;
  conversation_messages_0 = messages_0;
  if (J.t(((v) => J.t(v) ? (J.eq(J.attr(J.item(messages_0, 0), "role"), "system")) : v)(messages_0))) {
    system_message_0 = J.item(messages_0, 0);
    conversation_messages_0 = J.slice(messages_0, 1, undefined, undefined);
  }
  ns_0 = J.call(J.namespace, J.kw({"last_user_index": J.neg(1)}));
  {
    let _items3 = J.iter(conversation_messages_0);
    const _loop3 = J.loop(_items3);
    for (let _i3 = 0; _i3 < _items3.length; _i3++) {
      _loop3.index0 = _i3; const _it3 = _items3[_i3]; let m_11 = _it3;
      if (J.t((J.eq(J.attr(m_11, "role"), "user")))) {
        J.setattr(ns_0, "last_user_index", J.attr(_loop3, "index0"));
      }
    }
  }
  out.push(J.str(J.concat("]~!b[", "]~b]system", "\n")), J.str(J.call(build_system_message_0, system_message_0)));
  if (J.t(tools_0)) {
    out.push(J.str(J.concat("\n\n", "# Tools", "\n", "You may call one or more tools to assist with the user query.\nHere are the tools available in JSONSchema format:", "\n")), J.str(J.concat("\n", "<tools>", "\n")), J.str(J.call(render_tool_namespace_0, "functions", tools_0)), J.str(J.concat("</tools>", "\n\n")), J.str(J.concat("When making tool calls, use XML format to invoke tools and pass parameters:", "\n")), J.str(J.concat("\n", toolcall_begin_token_0)), "\n<invoke name=\"tool-name-1\">\n<parameter name=\"param-key-1\">param-value-1</parameter>\n<parameter name=\"param-key-2\">param-value-2</parameter>\n...\n</invoke>", J.str(J.concat("\n", toolcall_end_token_0)));
  }
  out.push(J.str("[e~[\n"));
  last_tool_call_0 = J.call(J.namespace, J.kw({"name": null}));
  {
    let _items4 = J.iter(conversation_messages_0);
    const _loop4 = J.loop(_items4);
    for (let _i4 = 0; _i4 < _items4.length; _i4++) {
      _loop4.index0 = _i4; const _it4 = _items4[_i4]; let message_12 = _it4;
      let reasoning_content_13 = reasoning_content_0;
      let content_14 = content_0;
      if (J.t((J.eq(J.attr(message_12, "role"), "assistant")))) {
        out.push(J.str(J.concat("]~b]ai", "\n")));
        reasoning_content_13 = "";
        content_14 = J.call(visible_text_0, J.attr(message_12, "content"));
        if (J.t(J.test("string", J.attr(message_12, "reasoning_content")))) {
          reasoning_content_13 = J.attr(message_12, "reasoning_content");
        } else {
          if (J.t((J.has("</think>", content_14)))) {
            reasoning_content_13 = J.call(J.attr(J.item(J.call(J.attr(J.call(J.attr(J.item(J.call(J.attr(content_14, "split"), "</think>"), 0), "strip"), "\n"), "split"), "<think>"), J.neg(1)), "strip"), "\n");
            content_14 = J.call(J.attr(J.item(J.call(J.attr(content_14, "split"), "</think>"), J.neg(1)), "strip"), "\n");
          }
        }
        if (J.t(((v) => J.t(v) ? (J.gt(J.attr(_loop4, "index0"), J.attr(ns_0, "last_user_index"))) : v)(reasoning_content_13))) {
          out.push(J.str(J.concat("<think>", "\n", reasoning_content_13, "\n", "</think>", "\n\n")));
        }
        if (J.t(content_14)) {
          out.push(J.str(content_14));
        }
        if (J.t(J.attr(message_12, "tool_calls"))) {
          out.push(J.str(J.concat("\n", toolcall_begin_token_0, "\n")));
          {
            let _items5 = J.iter(J.attr(message_12, "tool_calls"));
            const _loop5 = J.loop(_items5);
            for (let _i5 = 0; _i5 < _items5.length; _i5++) {
              _loop5.index0 = _i5; const _it5 = _items5[_i5]; let tool_call_15 = _it5;
              let _args_16 = _args_0;
              if (J.t(J.attr(tool_call_15, "function"))) {
                tool_call_15 = J.attr(tool_call_15, "function");
              }
              out.push(J.str(J.add(J.add("<invoke name=\"", J.attr(tool_call_15, "name")), "\">")), "\n");
              _args_16 = J.attr(tool_call_15, "arguments");
              {
                let _items6 = J.iter(J.call(J.attr(_args_16, "items")));
                const _loop6 = J.loop(_items6);
                for (let _i6 = 0; _i6 < _items6.length; _i6++) {
                  _loop6.index0 = _i6; const _it6 = _items6[_i6]; let [k_17, v_18] = J.unpack(_it6, 2);
                  out.push(J.str(J.add(J.add("<parameter name=\"", k_17), "\">")), J.str((J.t(!J.t(J.test("string", v_18))) ? J.filter("tojson", v_18, J.kw({"ensure_ascii": false})) : v_18)), J.str("</parameter>"), "\n");
                }
              }
              out.push(J.str(J.concat("</invoke>", "\n")));
            }
          }
          out.push(J.str(toolcall_end_token_0));
          J.setattr(last_tool_call_0, "name", J.attr(J.item(J.attr(message_12, "tool_calls"), J.neg(1)), "name"));
        } else {
          J.setattr(last_tool_call_0, "name", null);
        }
        out.push(J.str(J.concat("[e~[", "\n")));
      } else if (J.t((J.eq(J.attr(message_12, "role"), "tool")))) {
        if (J.t((J.attr(last_tool_call_0, "name") === null))) {
          out.push(J.str(J.call(J.raise_exception, "Message has tool role, but there was no previous assistant message with a tool call!")));
        }
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(conversation_messages_0, J.sub(J.attr(_loop4, "index0"), 1)), "role"), "tool")))(J.attr(_loop4, "first")))) {
          out.push(J.str("]~b]tool"));
        }
        if (J.t(J.test("string", J.attr(message_12, "content")))) {
          out.push(J.str("\n<response>"), J.str(J.attr(message_12, "content")), J.str("</response>"));
        } else {
          {
            let _items7 = J.iter(J.attr(message_12, "content"));
            const _loop7 = J.loop(_items7);
            for (let _i7 = 0; _i7 < _items7.length; _i7++) {
              _loop7.index0 = _i7; const _it7 = _items7[_i7]; let tr_19 = _it7;
              out.push(J.str("\n<response>"), J.str((J.t((J.attr(tr_19, "output") !== undefined)) ? J.attr(tr_19, "output") : (J.t(((v) => J.t(v) ? (J.attr(tr_19, "text") !== undefined) : v)((J.eq(J.attr(tr_19, "type"), "text")))) ? J.attr(tr_19, "text") : tr_19))), J.str("\n</response>"));
            }
          }
        }
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(conversation_messages_0, J.add(J.attr(_loop4, "index0"), 1)), "role"), "tool")))(J.attr(_loop4, "last")))) {
          out.push(J.str("[e~[\n"));
        }
      } else if (J.t((J.eq(J.attr(message_12, "role"), "user")))) {
        out.push(J.str(J.concat("]~b]user", "\n")), J.str(J.call(visible_text_0, J.attr(message_12, "content"))), J.str(J.concat("[e~[", "\n")));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str(J.concat("]~b]ai", "\n", "<think>", "\n")));
  }
  return out.join("");
}
