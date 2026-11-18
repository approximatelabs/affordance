// Converted from the chat template of mistralai/Mistral-Large-4-1T-A52B by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let default_system_message_0 = ctx["default_system_message"];
  let loop_messages_0 = ctx["loop_messages"];
  let available_tools_0 = ctx["available_tools"];
  let has_tools_0 = ctx["has_tools"];
  let json_schema_0 = ctx["json_schema"];
  let response_format_type_0 = ctx["response_format_type"];
  let response_json_schema_0 = ctx["response_json_schema"];
  let reasoning_effort_0 = ctx["reasoning_effort"];
  let model_settings_parts_0 = ctx["model_settings_parts"];
  let model_settings_0 = ctx["model_settings"];
  let render_content_0 = ctx["render_content"];
  let ns_agg_0 = ctx["ns_agg"];
  let ns_0 = ctx["ns"];
  let ns_order_0 = ctx["ns_order"];
  let bos_token_0 = ctx["bos_token"];
  let messages_0 = ctx["messages"];
  let tools_0 = ctx["tools"] ?? null;
  let response_format_0 = ctx["response_format"];
  let ns_c_0 = ctx["ns_c"];
  let merged_content_0 = ctx["merged_content"];
  let reasoning_0 = ctx["reasoning"];
  let think_chunk_0 = ctx["think_chunk"];
  let new_content_0 = ctx["new_content"];
  let ns_msg_0 = ctx["ns_msg"];
  let current_role_0 = ctx["current_role"];
  let blocks_0 = ctx["blocks"];
  let user_content_0 = ctx["user_content"];
  let name_0 = ctx["name"];
  let arguments_0 = ctx["arguments"];
  let eos_token_0 = ctx["eos_token"];
  default_system_message_0 = "";
  out.push(J.str(bos_token_0));
  loop_messages_0 = messages_0;
  if (J.t(((v) => J.t(v) ? (!J.eq(default_system_message_0, "")) : v)((!J.eq(J.item(J.item(messages_0, 0), "role"), "system"))))) {
    out.push(J.str(J.concat("[SYSTEM_PROMPT]", default_system_message_0, "[/SYSTEM_PROMPT]")));
  }
  available_tools_0 = "";
  has_tools_0 = false;
  if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", tools_0), 0)) : v)(((v) => J.t(v) ? !J.t((tools_0 === null)) : v)((tools_0 !== undefined))))) {
    has_tools_0 = true;
    available_tools_0 = J.concat("[AVAILABLE_TOOLS]", J.filter("tojson", tools_0), "[/AVAILABLE_TOOLS]");
  }
  json_schema_0 = null;
  if (J.t(((v) => J.t(v) ? !J.t((response_format_0 === null)) : v)((response_format_0 !== undefined)))) {
    response_format_type_0 = J.call(J.attr(response_format_0, "get"), "type", "text");
    response_json_schema_0 = J.call(J.attr(response_format_0, "get"), "json_schema");
    if (J.t((!J.has(response_format_type_0, ["text", "json_object", "json_schema"])))) {
      out.push(J.str(J.call(J.raise_exception, J.concat("Response format type must be one of text, json_object or json_schema, got ", response_format_type_0, "."))));
    } else if (J.t((J.eq(response_format_type_0, "json_schema")))) {
      if (J.t(((v) => J.t(v) ? v : (J.call(J.attr(response_json_schema_0, "get"), "schema") === null))((response_json_schema_0 === null)))) {
        out.push(J.str(J.call(J.raise_exception, "Response format `json_schema` must define the schema")));
      } else {
        json_schema_0 = J.item(response_json_schema_0, "schema");
      }
    } else if (J.t((J.eq(response_format_type_0, "json_object")))) {
      json_schema_0 = ({"anyOf": [({"type": "object"}), ({"type": "array"})]});
    }
  }
  if (J.t(((v) => J.t(v) ? v : (reasoning_effort_0 === null))(!J.t((reasoning_effort_0 !== undefined))))) {
    reasoning_effort_0 = "high";
  }
  if (J.t((!J.has(reasoning_effort_0, ["low", "medium", "high"])))) {
    out.push(J.str(J.call(J.raise_exception, "reasoning_effort must be one of \"low\", \"medium\", \"high\"")));
  }
  model_settings_parts_0 = [];
  if (J.t(!J.t((json_schema_0 === null)))) {
    model_settings_parts_0 = J.add(model_settings_parts_0, [J.concat("\"json_schema\": ", J.filter("tojson", json_schema_0))]);
  }
  model_settings_parts_0 = J.add(model_settings_parts_0, [J.concat("\"reasoning_effort\": \"", reasoning_effort_0, "\"")]);
  model_settings_0 = "";
  if (J.t(model_settings_parts_0)) {
    model_settings_0 = J.concat("[MODEL_SETTINGS]{", J.filter("join", model_settings_parts_0, ", "), "}[/MODEL_SETTINGS]");
  }
  render_content_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_6 = k.caller;
    let content_1 = p.length > 0 ? p[0] : "content" in k ? k["content"] : undefined;
    let context_name_2 = p.length > 1 ? p[1] : "context_name" in k ? k["context_name"] : undefined;
    let supported_types_desc_3 = p.length > 2 ? p[2] : "supported_types_desc" in k ? k["supported_types_desc"] : undefined;
    let support_thinking_4 = p.length > 3 ? p[3] : "support_thinking" in k ? k["support_thinking"] : undefined;
    let support_images_5 = p.length > 4 ? p[4] : "support_images" in k ? k["support_images"] : undefined;
    if (J.t(J.test("string", content_1))) {
      out.push(J.str(content_1));
    } else if (J.t(content_1)) {
      {
        let _items1 = J.iter(content_1);
        const _loop1 = J.loop(_items1);
        for (let _i1 = 0; _i1 < _items1.length; _i1++) {
          _loop1.index0 = _i1; const _it1 = _items1[_i1]; let block_7 = _it1;
          if (J.t((J.eq(J.item(block_7, "type"), "text")))) {
            out.push(J.str(J.item(block_7, "text")));
          } else if (J.t(((v) => J.t(v) ? (J.eq(J.item(block_7, "type"), "thinking")) : v)(support_thinking_4))) {
            out.push(J.str(J.concat("[THINK]", J.item(block_7, "thinking"))));
            if (J.t(J.call(J.attr(block_7, "get"), "closed", true))) {
              out.push(J.str("[/THINK]"));
            }
          } else if (J.t(((v) => J.t(v) ? (J.has(J.item(block_7, "type"), ["image", "image_url"])) : v)(support_images_5))) {
            out.push(J.str("[IMG]"));
          } else {
            out.push(J.str(J.call(J.raise_exception, J.concat("Only ", supported_types_desc_3, " chunks are supported in ", context_name_2, "."))));
          }
        }
      }
    } else {
      out.push(J.str(J.call(J.raise_exception, J.concat(context_name_2, " must have non-empty content."))));
    }
    return out.join("");
  };
  ns_agg_0 = J.call(J.namespace, J.kw({"messages": [], "current_group": [], "current_role": null}));
  {
    let _items2 = J.iter(J.add(loop_messages_0, [({"role": "__sentinel__"})]));
    const _loop2 = J.loop(_items2);
    for (let _i2 = 0; _i2 < _items2.length; _i2++) {
      _loop2.index0 = _i2; const _it2 = _items2[_i2]; let message_8 = _it2;
      let ns_c_9 = ns_c_0;
      let merged_content_10 = merged_content_0;
      if (J.t(((v) => J.t(v) ? v : (J.eq(J.item(message_8, "role"), "tool")))(((v) => J.t(v) ? v : (J.eq(J.item(message_8, "role"), "system")))((!J.eq(J.item(message_8, "role"), J.attr(ns_agg_0, "current_role"))))))) {
        if (J.t((J.eq(J.attr(ns_agg_0, "current_role"), "tool")))) {
          J.setattr(ns_agg_0, "messages", J.add(J.attr(ns_agg_0, "messages"), J.attr(ns_agg_0, "current_group")));
        } else if (J.t(!J.t((J.attr(ns_agg_0, "current_role") === null)))) {
          ns_c_9 = J.call(J.namespace, J.kw({"text_parts": [], "chunks": [], "has_non_text": false, "tool_calls": []}));
          {
            let _items3 = J.iter(J.attr(ns_agg_0, "current_group"));
            const _loop3 = J.loop(_items3);
            for (let _i3 = 0; _i3 < _items3.length; _i3++) {
              _loop3.index0 = _i3; const _it3 = _items3[_i3]; let msg_11 = _it3;
              let reasoning_12 = reasoning_0;
              let think_chunk_13 = think_chunk_0;
              let new_content_14 = new_content_0;
              let ns_msg_15 = ns_msg_0;
              reasoning_12 = J.call(J.attr(msg_11, "get"), "reasoning_content", J.call(J.attr(msg_11, "get"), "reasoning", null));
              if (J.t(((v) => J.t(v) ? (!J.eq(reasoning_12, "")) : v)(!J.t((reasoning_12 === null))))) {
                if (J.t(((v) => J.t(v) ? !J.t(J.test("string", J.item(msg_11, "content"))) : v)(!J.t((J.item(msg_11, "content") === null))))) {
                  {
                    let _items4 = J.iter(J.item(msg_11, "content"));
                    const _loop4 = J.loop(_items4);
                    for (let _i4 = 0; _i4 < _items4.length; _i4++) {
                      _loop4.index0 = _i4; const _it4 = _items4[_i4]; let block_16 = _it4;
                      if (J.t((J.eq(J.item(block_16, "type"), "thinking")))) {
                        out.push(J.str(J.call(J.raise_exception, "Message cannot have both thinking chunks in content and a top-level `reasoning` or `reasoning_content` field.")));
                      }
                    }
                  }
                }
                think_chunk_13 = ({"type": "thinking", "thinking": reasoning_12});
                if (J.t(((v) => J.t(v) ? (!J.eq(J.item(msg_11, "content"), "")) : v)(J.test("string", J.item(msg_11, "content"))))) {
                  new_content_14 = [think_chunk_13, ({"type": "text", "text": J.item(msg_11, "content")})];
                } else if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", J.item(msg_11, "content")), 0)) : v)(((v) => J.t(v) ? !J.t(J.test("string", J.item(msg_11, "content"))) : v)(!J.t((J.item(msg_11, "content") === null)))))) {
                  new_content_14 = J.add([think_chunk_13], J.filter("list", J.item(msg_11, "content")));
                } else {
                  new_content_14 = [think_chunk_13];
                }
                if (J.t(((v) => J.t(v) ? !J.t((J.item(msg_11, "tool_calls") === null)) : v)((J.item(msg_11, "tool_calls") !== undefined)))) {
                  msg_11 = ({"role": J.item(msg_11, "role"), "content": new_content_14, "tool_calls": J.item(msg_11, "tool_calls")});
                } else {
                  msg_11 = ({"role": J.item(msg_11, "role"), "content": new_content_14});
                }
              }
              if (J.t(J.test("string", J.item(msg_11, "content")))) {
                J.setattr(ns_c_9, "text_parts", J.add(J.attr(ns_c_9, "text_parts"), [J.item(msg_11, "content")]));
              } else if (J.t(!J.t((J.item(msg_11, "content") === null)))) {
                ns_msg_15 = J.call(J.namespace, J.kw({"msg_text_parts": []}));
                {
                  let _items5 = J.iter(J.item(msg_11, "content"));
                  const _loop5 = J.loop(_items5);
                  for (let _i5 = 0; _i5 < _items5.length; _i5++) {
                    _loop5.index0 = _i5; const _it5 = _items5[_i5]; let block_17 = _it5;
                    if (J.t((J.eq(J.item(block_17, "type"), "text")))) {
                      J.setattr(ns_msg_15, "msg_text_parts", J.add(J.attr(ns_msg_15, "msg_text_parts"), [J.item(block_17, "text")]));
                    } else {
                      if (J.t((J.gt(J.filter("length", J.attr(ns_msg_15, "msg_text_parts")), 0)))) {
                        J.setattr(ns_c_9, "text_parts", J.add(J.attr(ns_c_9, "text_parts"), [J.filter("join", J.attr(ns_msg_15, "msg_text_parts"), "")]));
                        J.setattr(ns_msg_15, "msg_text_parts", []);
                      }
                      if (J.t((J.gt(J.filter("length", J.attr(ns_c_9, "text_parts")), 0)))) {
                        J.setattr(ns_c_9, "chunks", J.add(J.attr(ns_c_9, "chunks"), [({"type": "text", "text": J.filter("join", J.attr(ns_c_9, "text_parts"), "\n\n")})]));
                        J.setattr(ns_c_9, "text_parts", []);
                      }
                      J.setattr(ns_c_9, "chunks", J.add(J.attr(ns_c_9, "chunks"), [block_17]));
                      J.setattr(ns_c_9, "has_non_text", true);
                    }
                  }
                }
                if (J.t((J.gt(J.filter("length", J.attr(ns_msg_15, "msg_text_parts")), 0)))) {
                  J.setattr(ns_c_9, "text_parts", J.add(J.attr(ns_c_9, "text_parts"), [J.filter("join", J.attr(ns_msg_15, "msg_text_parts"), "")]));
                }
              }
              if (J.t(((v) => J.t(v) ? !J.t((J.item(msg_11, "tool_calls") === null)) : v)((J.item(msg_11, "tool_calls") !== undefined)))) {
                J.setattr(ns_c_9, "tool_calls", J.add(J.attr(ns_c_9, "tool_calls"), J.filter("list", J.item(msg_11, "tool_calls"))));
              }
            }
          }
          if (J.t(J.attr(ns_c_9, "has_non_text"))) {
            if (J.t((J.gt(J.filter("length", J.attr(ns_c_9, "text_parts")), 0)))) {
              J.setattr(ns_c_9, "chunks", J.add(J.attr(ns_c_9, "chunks"), [({"type": "text", "text": J.filter("join", J.attr(ns_c_9, "text_parts"), "\n\n")})]));
            }
            merged_content_10 = J.attr(ns_c_9, "chunks");
          } else {
            merged_content_10 = J.filter("join", J.attr(ns_c_9, "text_parts"), "\n\n");
          }
          if (J.t((J.gt(J.filter("length", J.attr(ns_c_9, "tool_calls")), 0)))) {
            J.setattr(ns_agg_0, "messages", J.add(J.attr(ns_agg_0, "messages"), [({"role": J.attr(ns_agg_0, "current_role"), "content": merged_content_10, "tool_calls": J.attr(ns_c_9, "tool_calls")})]));
          } else {
            J.setattr(ns_agg_0, "messages", J.add(J.attr(ns_agg_0, "messages"), [({"role": J.attr(ns_agg_0, "current_role"), "content": merged_content_10})]));
          }
        }
        if (J.t((!J.eq(J.item(message_8, "role"), "__sentinel__")))) {
          J.setattr(ns_agg_0, "current_group", [message_8]);
          J.setattr(ns_agg_0, "current_role", J.item(message_8, "role"));
        }
      } else {
        J.setattr(ns_agg_0, "current_group", J.add(J.attr(ns_agg_0, "current_group"), [message_8]));
      }
    }
  }
  loop_messages_0 = J.attr(ns_agg_0, "messages");
  ns_0 = J.call(J.namespace, J.kw({"available_tools_and_settings_emitted": false}));
  if (J.t(((v) => J.t(v) ? (!J.has(J.item(J.item(loop_messages_0, 0), "role"), ["user", "system"])) : v)((J.gt(J.filter("length", loop_messages_0), 0))))) {
    out.push(J.str(J.call(J.raise_exception, J.concat("Conversation must start with a user or system message, got ", J.item(J.item(loop_messages_0, 0), "role"), "."))));
  }
  ns_order_0 = J.call(J.namespace, J.kw({"previous_role": null}));
  {
    let _items6 = J.iter(loop_messages_0);
    const _loop6 = J.loop(_items6);
    for (let _i6 = 0; _i6 < _items6.length; _i6++) {
      _loop6.index0 = _i6; const _it6 = _items6[_i6]; let message_18 = _it6;
      let current_role_19 = current_role_0;
      current_role_19 = J.item(message_18, "role");
      if (J.t(!J.t((J.attr(ns_order_0, "previous_role") === null)))) {
        if (J.t((J.eq(J.attr(ns_order_0, "previous_role"), "system")))) {
          if (J.t((!J.has(current_role_19, ["user", "assistant", "system"])))) {
            out.push(J.str(J.call(J.raise_exception, J.concat("Unexpected role '", current_role_19, "' after role '", J.attr(ns_order_0, "previous_role"), "'"))));
          }
        } else if (J.t((J.eq(J.attr(ns_order_0, "previous_role"), "user")))) {
          if (J.t((!J.has(current_role_19, ["assistant", "system", "user"])))) {
            out.push(J.str(J.call(J.raise_exception, J.concat("Unexpected role '", current_role_19, "' after role '", J.attr(ns_order_0, "previous_role"), "'"))));
          }
        } else if (J.t((J.eq(J.attr(ns_order_0, "previous_role"), "assistant")))) {
          if (J.t((!J.has(current_role_19, ["assistant", "user", "tool"])))) {
            out.push(J.str(J.call(J.raise_exception, J.concat("Unexpected role '", current_role_19, "' after role '", J.attr(ns_order_0, "previous_role"), "'"))));
          }
        } else if (J.t((J.eq(J.attr(ns_order_0, "previous_role"), "tool")))) {
          if (J.t((!J.has(current_role_19, ["assistant", "tool", "user"])))) {
            out.push(J.str(J.call(J.raise_exception, J.concat("Unexpected role '", current_role_19, "' after role '", J.attr(ns_order_0, "previous_role"), "'"))));
          }
        }
      }
      J.setattr(ns_order_0, "previous_role", current_role_19);
    }
  }
  {
    let _items7 = J.iter(loop_messages_0);
    const _loop7 = J.loop(_items7);
    for (let _i7 = 0; _i7 < _items7.length; _i7++) {
      _loop7.index0 = _i7; const _it7 = _items7[_i7]; let message_20 = _it7;
      let blocks_21 = blocks_0;
      let user_content_22 = user_content_0;
      if (J.t((J.eq(J.item(message_20, "role"), "user")))) {
        if (J.t(!J.t(J.attr(ns_0, "available_tools_and_settings_emitted")))) {
          out.push(J.str(available_tools_0), J.str(model_settings_0));
          J.setattr(ns_0, "available_tools_and_settings_emitted", true);
        }
        if (J.t(((v) => J.t(v) ? J.item(message_20, "content") : v)(!J.t(J.test("string", J.item(message_20, "content")))))) {
          if (J.t(((v) => J.t(v) ? (J.has(J.item(J.item(J.item(message_20, "content"), 1), "type"), ["image", "image_url"])) : v)(((v) => J.t(v) ? (J.eq(J.item(J.item(J.item(message_20, "content"), 0), "type"), "text")) : v)((J.eq(J.filter("length", J.item(message_20, "content")), 2)))))) {
            blocks_21 = [J.item(J.item(message_20, "content"), 1), J.item(J.item(message_20, "content"), 0)];
          } else {
            blocks_21 = J.item(message_20, "content");
          }
          user_content_22 = blocks_21;
        } else {
          user_content_22 = J.item(message_20, "content");
        }
        out.push(J.str("[INST]"), J.str(J.call(render_content_0, J.kw({"content": user_content_22, "context_name": "user message content", "supported_types_desc": "text, image and image_url", "support_thinking": false, "support_images": true}))), J.str("[/INST]"));
      } else if (J.t((J.eq(J.item(message_20, "role"), "assistant")))) {
        if (J.t(((v) => J.t(v) ? ((v) => J.t(v) ? v : (J.eq(J.filter("length", J.item(message_20, "tool_calls")), 0)))(((v) => J.t(v) ? v : (J.item(message_20, "tool_calls") === null))(!J.t((J.item(message_20, "tool_calls") !== undefined)))) : v)(((v) => J.t(v) ? v : (J.eq(J.filter("length", J.item(message_20, "content")), 0)))(((v) => J.t(v) ? v : (J.eq(J.item(message_20, "content"), "")))((J.item(message_20, "content") === null)))))) {
          out.push(J.str(J.call(J.raise_exception, "Assistant message must have a string or a list of chunks in content or a list of tool calls.")));
        }
        if (J.t(J.item(message_20, "content"))) {
          out.push(J.str(J.call(render_content_0, J.kw({"content": J.item(message_20, "content"), "context_name": "assistant message contents", "supported_types_desc": "text and thinking", "support_thinking": true, "support_images": false}))));
        }
        if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", J.item(message_20, "tool_calls")), 0)) : v)(((v) => J.t(v) ? !J.t((J.item(message_20, "tool_calls") === null)) : v)((J.item(message_20, "tool_calls") !== undefined))))) {
          {
            let _items8 = J.iter(J.item(message_20, "tool_calls"));
            const _loop8 = J.loop(_items8);
            for (let _i8 = 0; _i8 < _items8.length; _i8++) {
              _loop8.index0 = _i8; const _it8 = _items8[_i8]; let tool_23 = _it8;
              let name_24 = name_0;
              let arguments_25 = arguments_0;
              out.push(J.str("[TOOL_CALLS]"));
              name_24 = J.item(J.item(tool_23, "function"), "name");
              arguments_25 = J.item(J.item(tool_23, "function"), "arguments");
              if (J.t(!J.t(J.test("string", arguments_25)))) {
                arguments_25 = J.filter("safe", J.filter("tojson", arguments_25));
              } else if (J.t((J.eq(arguments_25, "")))) {
                arguments_25 = "{}";
              }
              out.push(J.str(J.concat(name_24, "[ARGS]", arguments_25)));
            }
          }
        }
        out.push(J.str(eos_token_0));
      } else if (J.t((J.eq(J.item(message_20, "role"), "tool")))) {
        out.push(J.str("[TOOL_RESULTS]"), J.str(J.call(render_content_0, J.kw({"content": J.item(message_20, "content"), "context_name": "tool message contents", "supported_types_desc": "text and image", "support_thinking": false, "support_images": true}))), J.str("[/TOOL_RESULTS]"));
      } else if (J.t((J.eq(J.item(message_20, "role"), "system")))) {
        out.push(J.str("[SYSTEM_PROMPT]"), J.str(J.call(render_content_0, J.kw({"content": J.item(message_20, "content"), "context_name": "system message contents", "supported_types_desc": "text", "support_thinking": false, "support_images": false}))), J.str("[/SYSTEM_PROMPT]"));
      } else {
        out.push(J.str(J.call(J.raise_exception, J.concat("Only user, assistant, system and tool roles are supported, got ", J.item(message_20, "role"), "."))));
      }
    }
  }
  return out.join("");
}
