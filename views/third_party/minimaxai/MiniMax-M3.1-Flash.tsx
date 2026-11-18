// Converted from the chat template of MiniMaxAI/MiniMax-M3.1-Flash by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let ns_token_0 = ctx["ns_token"];
  let bod_token_0 = ctx["bod_token"];
  let bos_token_0 = ctx["bos_token"];
  let eos_token_0 = ctx["eos_token"];
  let toolcall_begin_token_0 = ctx["toolcall_begin_token"];
  let toolcall_end_token_0 = ctx["toolcall_end_token"];
  let think_begin_token_0 = ctx["think_begin_token"];
  let think_end_token_0 = ctx["think_end_token"];
  let image_token_0 = ctx["image_token"];
  let video_token_0 = ctx["video_token"];
  let to_xml_0 = ctx["to_xml"];
  let render_tool_namespace_0 = ctx["render_tool_namespace"];
  let visible_text_0 = ctx["visible_text"];
  let build_system_message_0 = ctx["build_system_message"];
  let build_developer_message_0 = ctx["build_developer_message"];
  let system_message_0 = ctx["system_message"];
  let developer_message_0 = ctx["developer_message"];
  let conversation_messages_0 = ctx["conversation_messages"];
  let last_tool_call_0 = ctx["last_tool_call"];
  let reasoning_effort_0 = ctx["reasoning_effort"];
  let effort_0 = ctx["effort"];
  let model_identity_0 = ctx["model_identity"];
  let messages_0 = ctx["messages"];
  let tools_0 = ctx["tools"] ?? null;
  let reasoning_content_0 = ctx["reasoning_content"];
  let content_0 = ctx["content"];
  let _args_0 = ctx["_args"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  ns_token_0 = "]<]minimax[>[";
  bod_token_0 = "]~!b[";
  bos_token_0 = "]~b]";
  eos_token_0 = "[e~[";
  toolcall_begin_token_0 = J.concat(ns_token_0, "<tool_call>");
  toolcall_end_token_0 = J.concat(ns_token_0, "</tool_call>");
  think_begin_token_0 = "<mm:think>";
  think_end_token_0 = "</mm:think>";
  image_token_0 = "]<]image[>[";
  video_token_0 = "]<]video[>[";
  to_xml_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_3 = k.caller;
    let val_1 = p.length > 0 ? p[0] : "val" in k ? k["val"] : undefined;
    let ns_2 = p.length > 1 ? p[1] : "ns" in k ? k["ns"] : undefined;
    if (J.t(J.test("mapping", val_1))) {
      {
        let _items1 = J.iter(J.call(J.attr(val_1, "items")));
        _items1 = _items1.filter(_it1 => { let [k_4, v_5] = J.unpack(_it1, 2); return J.t(!J.t((v_5 === null))); });
        const _loop1 = J.loop(_items1);
        for (let _i1 = 0; _i1 < _items1.length; _i1++) {
          _loop1.index0 = _i1; const _it1 = _items1[_i1]; let [k_4, v_5] = J.unpack(_it1, 2);
          out.push(J.str(ns_2), "<", J.str(k_4), ">", J.str(J.call(to_xml_0, v_5, ns_2)), J.str(ns_2), "</", J.str(k_4), ">");
        }
      }
    } else if (J.t(((v) => J.t(v) ? !J.t(J.test("string", val_1)) : v)(J.test("iterable", val_1)))) {
      {
        let _items2 = J.iter(val_1);
        const _loop2 = J.loop(_items2);
        for (let _i2 = 0; _i2 < _items2.length; _i2++) {
          _loop2.index0 = _i2; const _it2 = _items2[_i2]; let item_6 = _it2;
          out.push(J.str(ns_2), "<item>", J.str(J.call(to_xml_0, item_6, ns_2)), J.str(ns_2), "</item>");
        }
      }
    } else if (J.t((val_1 === null))) {
    } else if (J.t(J.test("boolean", val_1))) {
      out.push(J.str(J.filter("tojson", val_1)));
    } else {
      out.push(J.str(val_1));
    }
    return out.join("");
  };
  render_tool_namespace_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_9 = k.caller;
    let namespace_name_7 = p.length > 0 ? p[0] : "namespace_name" in k ? k["namespace_name"] : undefined;
    let tool_list_8 = p.length > 1 ? p[1] : "tool_list" in k ? k["tool_list"] : undefined;
    {
      let _items3 = J.iter(tool_list_8);
      const _loop3 = J.loop(_items3);
      for (let _i3 = 0; _i3 < _items3.length; _i3++) {
        _loop3.index0 = _i3; const _it3 = _items3[_i3]; let tool_10 = _it3;
        out.push("<tool>", J.str(J.filter("tojson", J.attr(tool_10, "function"), J.kw({"ensure_ascii": false}))), "</tool>\n");
      }
    }
    return out.join("");
  };
  visible_text_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_12 = k.caller;
    let content_11 = p.length > 0 ? p[0] : "content" in k ? k["content"] : undefined;
    if (J.t(J.test("string", content_11))) {
      out.push(J.str(content_11));
    } else if (J.t(((v) => J.t(v) ? !J.t(J.test("mapping", content_11)) : v)(J.test("iterable", content_11)))) {
      {
        let _items4 = J.iter(content_11);
        const _loop4 = J.loop(_items4);
        for (let _i4 = 0; _i4 < _items4.length; _i4++) {
          _loop4.index0 = _i4; const _it4 = _items4[_i4]; let item_13 = _it4;
          if (J.t(((v) => J.t(v) ? (J.eq(J.attr(item_13, "type"), "text")) : v)(J.test("mapping", item_13)))) {
            out.push(J.str(J.attr(item_13, "text")));
          } else if (J.t(((v) => J.t(v) ? (J.eq(J.attr(item_13, "type"), "image")) : v)(J.test("mapping", item_13)))) {
            out.push(J.str(image_token_0));
          } else if (J.t(((v) => J.t(v) ? (J.eq(J.attr(item_13, "type"), "video")) : v)(J.test("mapping", item_13)))) {
            out.push(J.str(video_token_0));
          } else if (J.t(J.test("string", item_13))) {
            out.push(J.str(item_13));
          }
        }
      }
    } else if (J.t((content_11 === null))) {
      out.push(J.str(""));
    } else {
      out.push(J.str(content_11));
    }
    return out.join("");
  };
  build_system_message_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_15 = k.caller;
    let system_message_14 = p.length > 0 ? p[0] : "system_message" in k ? k["system_message"] : undefined;
    if (J.t(((v) => J.t(v) ? J.attr(system_message_14, "content") : v)(system_message_14))) {
      out.push(J.str(J.call(visible_text_0, J.attr(system_message_14, "content"))));
      if (J.t(J.attr(system_message_14, "current_date"))) {
        out.push(J.str(J.add(J.concat("\n", "Current date: "), J.attr(system_message_14, "current_date"))));
      }
      if (J.t(J.attr(system_message_14, "current_location"))) {
        out.push(J.str(J.add(J.concat("\n", "Current location: "), J.attr(system_message_14, "current_location"))));
      }
    } else {
      out.push(J.str("Your model version is MiniMax-M3.1, developed by MiniMax. Knowledge cutoff: January 2026. Founded in early 2022, MiniMax is a global AI foundation model company committed to advancing the frontiers of AI towards AGI."));
    }
    out.push(J.str("\n\n<thinking_instructions>\n"), J.str(J.concat("You have a thinking capability that allows you to reason step by step before responding. When thinking is enabled, wrap your reasoning in ", think_begin_token_0, think_end_token_0, " tags before your response. When thinking is disabled, begin your response directly after the ", think_end_token_0, " prefix. When thinking is adaptive, decide on your own whether to think for the current turn.\n")), J.str("Current thinking mode: adaptive. You are encouraged to think for complex decision-making, multi-step reasoning, or when analyzing function/tool results.\n"), J.str("</thinking_instructions>"), J.str("\n\n<model_id>8bb8149fe</model_id>"));
    if (J.t(((v) => J.t(v) ? !J.t((reasoning_effort_0 === null)) : v)((reasoning_effort_0 !== undefined)))) {
      out.push(J.str(J.concat("\n\n<effort>", reasoning_effort_0, "</effort>")));
    } else if (J.t(((v) => J.t(v) ? !J.t((effort_0 === null)) : v)((effort_0 !== undefined)))) {
      out.push(J.str(J.concat("\n\n<effort>", effort_0, "</effort>")));
    }
    return out.join("");
  };
  build_developer_message_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_17 = k.caller;
    let developer_message_16 = p.length > 0 ? p[0] : "developer_message" in k ? k["developer_message"] : undefined;
    let model_identity_18 = model_identity_0;
    if (J.t(((v) => J.t(v) ? J.attr(developer_message_16, "content") : v)(developer_message_16))) {
      out.push(J.str(J.call(visible_text_0, J.attr(developer_message_16, "content"))));
    } else {
      if (J.t(!J.t((model_identity_18 !== undefined)))) {
        model_identity_18 = "You are a helpful assistant.";
      }
      out.push(J.str(model_identity_18));
    }
    return out.join("");
  };
  system_message_0 = null;
  developer_message_0 = null;
  conversation_messages_0 = messages_0;
  if (J.t(((v) => J.t(v) ? (J.eq(J.attr(J.item(messages_0, 0), "role"), "root")) : v)(messages_0))) {
    system_message_0 = J.item(messages_0, 0);
    conversation_messages_0 = J.slice(messages_0, 1, undefined, undefined);
    if (J.t(((v) => J.t(v) ? (J.has(J.attr(J.item(conversation_messages_0, 0), "role"), ["system", "developer"])) : v)(conversation_messages_0))) {
      developer_message_0 = J.item(conversation_messages_0, 0);
      conversation_messages_0 = J.slice(conversation_messages_0, 1, undefined, undefined);
    }
  } else if (J.t(((v) => J.t(v) ? (J.has(J.attr(J.item(messages_0, 0), "role"), ["system", "developer"])) : v)(messages_0))) {
    developer_message_0 = J.item(messages_0, 0);
    conversation_messages_0 = J.slice(messages_0, 1, undefined, undefined);
  }
  out.push(J.str(J.concat(bod_token_0, bos_token_0, "system", "\n")), J.str(J.call(build_system_message_0, system_message_0)), J.str(J.concat(eos_token_0, "\n")), J.str(J.concat(bos_token_0, "developer", "\n")), J.str(J.call(build_developer_message_0, developer_message_0)));
  if (J.t(tools_0)) {
    out.push(J.str(J.concat("\n\n", "# Tools", "\n", "You may call one or more tools to assist with the user query.\nHere are the tools available in JSONSchema format:", "\n")), J.str(J.concat("\n", "<tools>", "\n")), J.str(J.call(render_tool_namespace_0, "functions", tools_0)), J.str(J.concat("</tools>", "\n\n")), J.str(J.concat("To call tools, wrap all invocations in a single ", toolcall_begin_token_0, toolcall_end_token_0, " block. Parameter values containing nested objects or arrays are recursively expanded into XML elements. Example:\n")), J.str(J.concat("\n", toolcall_begin_token_0, "\n")), J.str(J.add(ns_token_0, "<invoke name=\"tool-name-1\">")), J.str(J.add(J.add(J.add(ns_token_0, "<param-1>value-1"), ns_token_0), "</param-1>")), J.str(J.add(ns_token_0, "<param-2>")), J.str(J.add(ns_token_0, "<item>")), J.str(J.add(J.add(J.add(ns_token_0, "<key-a>val-a"), ns_token_0), "</key-a>")), J.str(J.add(J.add(J.add(ns_token_0, "<key-b>val-b"), ns_token_0), "</key-b>")), J.str(J.add(ns_token_0, "</item>")), J.str(J.add(ns_token_0, "</param-2>")), J.str(J.add(ns_token_0, "</invoke>\n")), J.str(J.add(ns_token_0, "<invoke name=\"tool-name-2\">")), J.str(J.add(J.add(J.add(ns_token_0, "<param-1>value-1"), ns_token_0), "</param-1>")), J.str(J.add(ns_token_0, "</invoke>\n")), J.str(toolcall_end_token_0));
  }
  out.push(J.str(J.concat(eos_token_0, "\n")));
  last_tool_call_0 = J.call(J.namespace, J.kw({"name": null}));
  {
    let _items5 = J.iter(conversation_messages_0);
    const _loop5 = J.loop(_items5);
    for (let _i5 = 0; _i5 < _items5.length; _i5++) {
      _loop5.index0 = _i5; const _it5 = _items5[_i5]; let message_19 = _it5;
      let reasoning_content_20 = reasoning_content_0;
      let content_21 = content_0;
      if (J.t((J.eq(J.attr(message_19, "role"), "assistant")))) {
        out.push(J.str(J.concat(bos_token_0, "ai", "\n")));
        reasoning_content_20 = "";
        content_21 = J.call(visible_text_0, J.attr(message_19, "content"));
        if (J.t(J.test("string", J.attr(message_19, "reasoning_content")))) {
          reasoning_content_20 = J.attr(message_19, "reasoning_content");
        } else {
          if (J.t((J.has(think_end_token_0, content_21)))) {
            reasoning_content_20 = J.call(J.attr(J.item(J.call(J.attr(J.call(J.attr(J.item(J.call(J.attr(content_21, "split"), think_end_token_0), 0), "strip"), "\n"), "split"), think_begin_token_0), J.neg(1)), "strip"), "\n");
            content_21 = J.call(J.attr(J.item(J.call(J.attr(content_21, "split"), think_end_token_0), J.neg(1)), "strip"), "\n");
          }
        }
        if (J.t(reasoning_content_20)) {
          out.push(J.str(J.concat(think_begin_token_0, reasoning_content_20, think_end_token_0)));
        } else {
          out.push(J.str(think_end_token_0));
        }
        if (J.t(content_21)) {
          out.push(J.str(content_21));
        }
        if (J.t(J.attr(message_19, "tool_calls"))) {
          out.push(J.str(J.concat(toolcall_begin_token_0, "\n")));
          {
            let _items6 = J.iter(J.attr(message_19, "tool_calls"));
            const _loop6 = J.loop(_items6);
            for (let _i6 = 0; _i6 < _items6.length; _i6++) {
              _loop6.index0 = _i6; const _it6 = _items6[_i6]; let tool_call_22 = _it6;
              let _args_23 = _args_0;
              if (J.t(J.attr(tool_call_22, "function"))) {
                tool_call_22 = J.attr(tool_call_22, "function");
              }
              out.push(J.str(J.add(J.add(J.add(ns_token_0, "<invoke name=\""), J.attr(tool_call_22, "name")), "\">")));
              _args_23 = J.attr(tool_call_22, "arguments");
              {
                let _items7 = J.iter(J.call(J.attr(_args_23, "items")));
                _items7 = _items7.filter(_it7 => { let [k_24, v_25] = J.unpack(_it7, 2); return J.t(!J.t((v_25 === null))); });
                const _loop7 = J.loop(_items7);
                for (let _i7 = 0; _i7 < _items7.length; _i7++) {
                  _loop7.index0 = _i7; const _it7 = _items7[_i7]; let [k_24, v_25] = J.unpack(_it7, 2);
                  out.push(J.str(J.add(J.add(J.add(ns_token_0, "<"), k_24), ">")), J.str(J.call(to_xml_0, v_25, ns_token_0)), J.str(J.add(J.add(J.add(ns_token_0, "</"), k_24), ">")));
                }
              }
              out.push(J.str(J.add(ns_token_0, J.concat("</invoke>", "\n"))));
            }
          }
          out.push(J.str(toolcall_end_token_0));
          if (J.t(J.attr(J.item(J.attr(message_19, "tool_calls"), J.neg(1)), "function"))) {
            J.setattr(last_tool_call_0, "name", J.attr(J.attr(J.item(J.attr(message_19, "tool_calls"), J.neg(1)), "function"), "name"));
          } else {
            J.setattr(last_tool_call_0, "name", J.attr(J.item(J.attr(message_19, "tool_calls"), J.neg(1)), "name"));
          }
        } else {
          J.setattr(last_tool_call_0, "name", null);
        }
        out.push(J.str(J.concat(eos_token_0, "\n")));
      } else if (J.t((J.eq(J.attr(message_19, "role"), "tool")))) {
        if (J.t((J.attr(last_tool_call_0, "name") === null))) {
          out.push(J.str(J.call(J.raise_exception, "Message has tool role, but there was no previous assistant message with a tool call!")));
        }
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(conversation_messages_0, J.sub(J.attr(_loop5, "index0"), 1)), "role"), "tool")))(J.attr(_loop5, "first")))) {
          out.push(J.str(J.concat(bos_token_0, "tool")));
        }
        out.push(J.str("\n<response>"));
        if (J.t(J.test("string", J.attr(message_19, "content")))) {
          out.push(J.str(J.attr(message_19, "content")));
        } else {
          {
            let _items8 = J.iter(J.attr(message_19, "content"));
            const _loop8 = J.loop(_items8);
            for (let _i8 = 0; _i8 < _items8.length; _i8++) {
              _loop8.index0 = _i8; const _it8 = _items8[_i8]; let tr_26 = _it8;
              if (J.t(((v) => J.t(v) ? (J.eq(J.attr(tr_26, "type"), "image")) : v)(((v) => J.t(v) ? (J.attr(tr_26, "type") !== undefined) : v)(J.test("mapping", tr_26))))) {
                out.push(J.str(image_token_0));
              } else if (J.t(((v) => J.t(v) ? (J.eq(J.attr(tr_26, "type"), "video")) : v)(((v) => J.t(v) ? (J.attr(tr_26, "type") !== undefined) : v)(J.test("mapping", tr_26))))) {
                out.push(J.str(video_token_0));
              } else {
                out.push(J.str((J.t((J.attr(tr_26, "output") !== undefined)) ? J.attr(tr_26, "output") : (J.t(((v) => J.t(v) ? (J.attr(tr_26, "text") !== undefined) : v)((J.eq(J.attr(tr_26, "type"), "text")))) ? J.attr(tr_26, "text") : tr_26))));
              }
            }
          }
        }
        out.push(J.str("</response>"));
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(conversation_messages_0, J.add(J.attr(_loop5, "index0"), 1)), "role"), "tool")))(J.attr(_loop5, "last")))) {
          out.push(J.str(J.concat(eos_token_0, "\n")));
        }
      } else if (J.t((J.has(J.attr(message_19, "role"), ["system", "developer"])))) {
        out.push(J.str(J.concat(bos_token_0, "developer", "\n")), J.str(J.call(build_developer_message_0, message_19)), J.str(J.concat(eos_token_0, "\n")));
      } else if (J.t((J.eq(J.attr(message_19, "role"), "user")))) {
        out.push(J.str(J.concat(bos_token_0, "user", "\n")), J.str(J.call(visible_text_0, J.attr(message_19, "content"))), J.str(J.concat(eos_token_0, "\n")));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str(J.concat(bos_token_0, "ai", "\n")));
  }
  return out.join("");
}
