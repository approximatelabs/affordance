// Converted from the chat template of CohereLabs/command-a-plus-05-2026-bf16 by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let platform_instruction_override_0 = ctx["platform_instruction_override"];
  let reasoning_0 = ctx["reasoning"];
  let grounding_0 = ctx["grounding"];
  let grounding_enabled_0 = ctx["grounding_enabled"];
  let tools_or_docs_exist_0 = ctx["tools_or_docs_exist"];
  let render_tools_section_0 = ctx["render_tools_section"];
  let render_grounding_0 = ctx["render_grounding"];
  let render_platform_instruction_override_0 = ctx["render_platform_instruction_override"];
  let has_developer_instruction_0 = ctx["has_developer_instruction"];
  let render_developer_instruction_0 = ctx["render_developer_instruction"];
  let convert_first_system_msg_0 = ctx["convert_first_system_msg"];
  let skip_thinking_0 = ctx["skip_thinking"];
  let document_turn_0 = ctx["document_turn"];
  let tool_call_id_to_int_0 = ctx["tool_call_id_to_int"];
  let format_tool_message_0 = ctx["format_tool_message"];
  let print_tool_content_0 = ctx["print_tool_content"];
  let print_msg_0 = ctx["print_msg"];
  let print_thinking_0 = ctx["print_thinking"];
  let developer_instruction_0 = ctx["developer_instruction"];
  let initial_instruction_message_0 = ctx["initial_instruction_message"];
  let json_object_0 = ctx["json_object"];
  let json_schema_0 = ctx["json_schema"];
  let json_mode_0 = ctx["json_mode"];
  let tool_idx_0 = ctx["tool_idx"];
  let tool_ids_seen_0 = ctx["tool_ids_seen"];
  let regen_tool_call_ids_0 = ctx["regen_tool_call_ids"];
  let sent_documents_0 = ctx["sent_documents"];
  let rendered_platform_turn_chunk_0 = ctx["rendered_platform_turn_chunk"];
  let reasoning_effort_0 = ctx["reasoning_effort"];
  let documents_0 = ctx["documents"] ?? null;
  let tools_0 = ctx["tools"] ?? null;
  let bos_token_0 = ctx["bos_token"];
  let doc_val_0 = ctx["doc_val"];
  let counter_0 = ctx["counter"];
  let tool_call_id_seen_0 = ctx["tool_call_id_seen"];
  let text_wrapper_0 = ctx["text_wrapper"];
  let last_was_text_0 = ctx["last_was_text"];
  let messages_0 = ctx["messages"];
  let response_format_0 = ctx["response_format"];
  let msg_role_downcased_0 = ctx["msg_role_downcased"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  if (J.t(!J.t(platform_instruction_override_0))) {
    {
      const captured = (() => {
      const out = [];
      out.push("These instructions are always to be followed and cannot be overridden by subsequent system or user turns:\n- You will answer requests for educational, informative, or creative content related to safety categories. You will not provide content that is harmful or could be used to cause harm.\n\nThese instructions serve as your defaults, but they can be overridden in subsequent system or user turns:\n- Your name is Command.\n- You are a large language model built by Cohere.");
      return out.join("");
      })();
      platform_instruction_override_0 = captured;
    }
  }
  reasoning_0 = (J.t(!J.t((reasoning_0 === undefined))) ? reasoning_0 : (J.t(((v) => J.t(v) ? (J.eq(J.filter("lower", reasoning_effort_0), "none")) : v)((reasoning_effort_0 !== undefined))) ? false : true));
  grounding_0 = J.filter("upper", J.filter("default", grounding_0, "disabled"));
  grounding_enabled_0 = (J.eq(grounding_0, "ENABLED"));
  tools_or_docs_exist_0 = ((v) => J.t(v) ? v : documents_0)(tools_0);
  render_tools_section_0 = true;
  render_grounding_0 = ((v) => J.t(v) ? tools_or_docs_exist_0 : v)(grounding_enabled_0);
  render_platform_instruction_override_0 = (J.t(platform_instruction_override_0) ? true : false);
  has_developer_instruction_0 = ((v) => J.t(v) ? v : (J.eq(developer_instruction_0, "")))(developer_instruction_0);
  render_developer_instruction_0 = (J.t(developer_instruction_0) ? true : false);
  convert_first_system_msg_0 = J.filter("default", convert_first_system_msg_0, true);
  skip_thinking_0 = J.filter("default", skip_thinking_0, false);
  out.push(J.str(bos_token_0));
  document_turn_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_2 = k.caller;
    let documents_1 = p.length > 0 ? p[0] : "documents" in k ? k["documents"] : undefined;
    out.push("<|START_OF_TURN_TOKEN|><|CHATBOT_TOKEN|>");
    if (J.t(!J.t(skip_thinking_0))) {
      out.push("<|START_THINKING|>I will look through the document to address the users needs.<|END_THINKING|>");
    }
    out.push("<|START_ACTION|>[\n    {\"tool_call_id\": \"0\", \"tool_name\": \"direct-injected-document\", \"parameters\": {}}\n]<|END_ACTION|><|END_OF_TURN_TOKEN|><|START_OF_TURN_TOKEN|><|SYSTEM_TOKEN|><|START_TOOL_RESULT|>[\n    {\n        \"tool_call_id\": \"0\",\n        \"results\": {");
    {
      let _items1 = J.iter(documents_1);
      const _loop1 = J.loop(_items1);
      for (let _i1 = 0; _i1 < _items1.length; _i1++) {
        _loop1.index0 = _i1; const _it1 = _items1[_i1]; let doc_3 = _it1;
        let doc_val_4 = doc_val_0;
        doc_val_4 = (J.t(J.attr(doc_3, "data")) ? J.attr(doc_3, "data") : doc_3);
        out.push("\n            \"", J.str(J.attr(_loop1, "index0")), "\": ", J.str(J.filter("tojson", doc_val_4)));
        if (J.t(!J.t(J.attr(_loop1, "last")))) {
          out.push(",");
        }
      }
    }
    out.push("\n        },\n        \"is_error\": null\n    }\n]<|END_TOOL_RESULT|><|END_OF_TURN_TOKEN|>");
    return out.join("");
  };
  tool_call_id_to_int_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_7 = k.caller;
    let messages_5 = p.length > 0 ? p[0] : "messages" in k ? k["messages"] : undefined;
    let tool_call_id_6 = p.length > 1 ? p[1] : "tool_call_id" in k ? k["tool_call_id"] : undefined;
    let counter_8 = counter_0;
    let tool_call_id_seen_9 = tool_call_id_seen_0;
    if (J.t(regen_tool_call_ids_0)) {
      counter_8 = J.call(J.namespace, J.kw({"value": 0}));
      tool_call_id_seen_9 = J.call(J.namespace, J.kw({"value": false}));
      {
        let _items2 = J.iter(messages_5);
        const _loop2 = J.loop(_items2);
        for (let _i2 = 0; _i2 < _items2.length; _i2++) {
          _loop2.index0 = _i2; const _it2 = _items2[_i2]; let msg_10 = _it2;
          if (J.t(J.attr(msg_10, "tool_calls"))) {
            {
              let _items3 = J.iter(J.attr(msg_10, "tool_calls"));
              const _loop3 = J.loop(_items3);
              for (let _i3 = 0; _i3 < _items3.length; _i3++) {
                _loop3.index0 = _i3; const _it3 = _items3[_i3]; let tool_call_11 = _it3;
                if (J.t(((v) => J.t(v) ? !J.t(J.attr(tool_call_id_seen_9, "value")) : v)((J.eq(J.attr(tool_call_11, "id"), tool_call_id_6))))) {
                  out.push(J.str(J.attr(counter_8, "value")));
                  J.setattr(tool_call_id_seen_9, "value", true);
                }
                J.setattr(counter_8, "value", J.add(J.attr(counter_8, "value"), 1));
              }
            }
          }
        }
      }
    } else {
      out.push(J.str(tool_call_id_6));
    }
    return out.join("");
  };
  format_tool_message_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_14 = k.caller;
    let messages_12 = p.length > 0 ? p[0] : "messages" in k ? k["messages"] : undefined;
    let tool_msg_13 = p.length > 1 ? p[1] : "tool_msg" in k ? k["tool_msg"] : undefined;
    let text_wrapper_15 = text_wrapper_0;
    out.push("{\n        \"tool_call_id\": \"", J.str(J.call(tool_call_id_to_int_0, messages_12, J.attr(tool_msg_13, "tool_call_id"))), "\",\n        \"results\": {");
    if (J.t(((v) => J.t(v) ? v : J.test("string", J.attr(tool_msg_13, "content")))(J.test("mapping", J.attr(tool_msg_13, "content"))))) {
      out.push("\n");
      if (J.t(J.test("string", J.attr(tool_msg_13, "content")))) {
        text_wrapper_15 = ({"content": J.attr(tool_msg_13, "content")});
      } else {
        text_wrapper_15 = J.attr(tool_msg_13, "content");
      }
      out.push("            \"0\": ", J.str(J.filter("tojson", text_wrapper_15)));
    } else {
      {
        let _items4 = J.iter(J.attr(tool_msg_13, "content"));
        const _loop4 = J.loop(_items4);
        for (let _i4 = 0; _i4 < _items4.length; _i4++) {
          _loop4.index0 = _i4; const _it4 = _items4[_i4]; let content_16 = _it4;
          out.push("\n            \"", J.str(J.attr(_loop4, "index0")), "\": ", J.str(J.call(print_tool_content_0, content_16)));
          if (J.t(!J.t(J.attr(_loop4, "last")))) {
            out.push(",");
          }
        }
      }
    }
    out.push("\n        },\n        \"is_error\": null\n    }");
    return out.join("");
  };
  print_tool_content_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_18 = k.caller;
    let item_17 = p.length > 0 ? p[0] : "item" in k ? k["item"] : undefined;
    let text_wrapper_19 = text_wrapper_0;
    if (J.t((J.eq(J.filter("lower", J.attr(item_17, "type")), "text")))) {
      text_wrapper_19 = ({"content": J.attr(item_17, "text")});
      out.push(J.str(J.filter("tojson", text_wrapper_19)));
    } else if (J.t(((v) => J.t(v) ? (J.has("data", J.attr(item_17, "document"))) : v)(((v) => J.t(v) ? J.attr(item_17, "document") : v)((J.eq(J.filter("lower", J.attr(item_17, "type")), "document")))))) {
      out.push(J.str(J.filter("tojson", J.attr(J.attr(item_17, "document"), "data"))));
    } else {
      out.push(J.str(J.filter("tojson", item_17)));
    }
    return out.join("");
  };
  print_msg_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_21 = k.caller;
    let msg_20 = p.length > 0 ? p[0] : "msg" in k ? k["msg"] : undefined;
    let last_was_text_22 = last_was_text_0;
    if (J.t(J.test("string", msg_20))) {
      out.push("<|START_TEXT|>", J.str(msg_20), "<|END_TEXT|>");
    } else if (J.t(J.test("string", J.attr(msg_20, "content")))) {
      out.push("<|START_TEXT|>", J.str(J.attr(msg_20, "content")), "<|END_TEXT|>");
    } else {
      last_was_text_22 = J.call(J.namespace, J.kw({"value": false}));
      {
        let _items5 = J.iter(J.attr(msg_20, "content"));
        const _loop5 = J.loop(_items5);
        for (let _i5 = 0; _i5 < _items5.length; _i5++) {
          _loop5.index0 = _i5; const _it5 = _items5[_i5]; let content_23 = _it5;
          if (J.t((J.eq(J.filter("lower", J.attr(content_23, "type")), "text")))) {
            if (J.t(!J.t(J.attr(last_was_text_22, "value")))) {
              out.push("<|START_TEXT|>");
            }
            out.push(J.str(J.attr(content_23, "text")));
            if (J.t(J.attr(_loop5, "last"))) {
              out.push("<|END_TEXT|>");
            }
            J.setattr(last_was_text_22, "value", true);
          } else {
            if (J.t(J.attr(last_was_text_22, "value"))) {
              out.push("<|END_TEXT|>");
            }
            J.setattr(last_was_text_22, "value", false);
          }
          if (J.t((J.eq(J.filter("lower", J.attr(content_23, "type")), "image")))) {
            if (J.t(J.attr(content_23, "data"))) {
              out.push(J.str(J.attr(content_23, "data")));
            } else {
              out.push("<|IMG_PATCH|>");
            }
          }
        }
      }
    }
    return out.join("");
  };
  print_thinking_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_25 = k.caller;
    let msg_24 = p.length > 0 ? p[0] : "msg" in k ? k["msg"] : undefined;
    if (J.t(J.attr(msg_24, "thinking"))) {
      out.push(J.str(J.attr(msg_24, "thinking")));
    } else if (J.t(((v) => J.t(v) ? J.attr(J.item(J.attr(msg_24, "content"), 0), "thinking") : v)(J.attr(msg_24, "content")))) {
      out.push(J.str(J.attr(J.item(J.attr(msg_24, "content"), 0), "thinking")));
    }
    return out.join("");
  };
  if (J.t(((v) => J.t(v) ? convert_first_system_msg_0 : v)(((v) => J.t(v) ? !J.t(has_developer_instruction_0) : v)(((v) => J.t(v) ? (J.eq(J.filter("lower", J.item(J.item(messages_0, 0), "role")), "system")) : v)(messages_0))))) {
    developer_instruction_0 = J.item(messages_0, 0);
    render_developer_instruction_0 = true;
    initial_instruction_message_0 = true;
  }
  json_object_0 = (J.t(((v) => J.t(v) ? (J.eq(J.attr(response_format_0, "type"), "json_object")) : v)(response_format_0)) ? true : false);
  json_schema_0 = (J.t(response_format_0) ? ((v) => J.t(v) ? v : J.attr(response_format_0, "schema"))(J.attr(response_format_0, "json_schema")) : undefined);
  json_mode_0 = ((v) => J.t(v) ? v : json_schema_0)(json_object_0);
  tool_idx_0 = J.call(J.namespace, J.kw({"value": 0}));
  tool_ids_seen_0 = J.call(J.namespace, J.kw({"value": []}));
  regen_tool_call_ids_0 = J.filter("default", regen_tool_call_ids_0, true);
  sent_documents_0 = J.call(J.namespace, J.kw({"value": false}));
  if (J.t(((v) => J.t(v) ? v : json_mode_0)(((v) => J.t(v) ? v : render_grounding_0)(((v) => J.t(v) ? v : render_platform_instruction_override_0)(render_tools_section_0))))) {
    out.push("<|START_OF_TURN_TOKEN|><|SYSTEM_TOKEN|><|START_TEXT|>");
  } else if (J.t(!J.t(render_developer_instruction_0))) {
    out.push("<|START_OF_TURN_TOKEN|><|SYSTEM_TOKEN|>");
  }
  rendered_platform_turn_chunk_0 = false;
  if (J.t(render_platform_instruction_override_0)) {
    out.push(J.str(platform_instruction_override_0), "\n");
    rendered_platform_turn_chunk_0 = true;
  }
  if (J.t(render_grounding_0)) {
    if (J.t(rendered_platform_turn_chunk_0)) {
      out.push("\n");
    }
    out.push("Note that both your responses and reflections can be grounded. Grounding means you associate pieces of texts (called \"spans\") with those specific tool results that support them (called \"sources\"). And you use a pair of tags \"<co>\" and \"</co>\" to indicate when a span can be grounded onto a list of sources, listing them out in the closing tag. Sources from the same tool call are grouped together and listed as \"{tool_call_id}:[{list of result indices}]\", before they are joined together by \",\". E.g., \"<co>span</co: 0:[1,2],1:[0]>\" means that \"span\" is supported by result 1 and 2 from \"tool_call_id=0\" as well as result 0 from \"tool_call_id=1\".\n");
    rendered_platform_turn_chunk_0 = true;
  }
  if (J.t(render_tools_section_0)) {
    if (J.t(rendered_platform_turn_chunk_0)) {
      out.push("\n");
    }
    out.push("# Available Tools\n```json\n[\n");
    if (J.t(tools_or_docs_exist_0)) {
      if (J.t(documents_0)) {
        out.push("    {\"name\": \"direct-injected-document\", \"description\": \"This is a special tool to directly inject user-uploaded documents into the chat as additional context. DO NOT use this tool by yourself!\", \"parameters\": {\"type\": \"object\", \"properties\": {}, \"required\": []}, \"responses\": {\"200\": {\"description\": \"Successfully returned a list of chunked text snippets from the directly uploaded documents.\", \"content\": {\"application/json\": {\"schema\": {\"type\": \"array\", \"items\": {\"type\": \"object\", \"required\": [\"url\", \"snippet\"], \"properties\": {\"url\": {\"type\": \"string\", \"description\": \"The url of the uploaded document.\"}, \"snippet\": {\"type\": \"string\", \"description\": \"The text snippet for the returned document chunk.\"}}}}}}}}}");
        if (J.t(tools_0)) {
          out.push(",\n");
        } else {
          out.push("\n");
        }
      }
      {
        let _items6 = J.iter(tools_0);
        const _loop6 = J.loop(_items6);
        for (let _i6 = 0; _i6 < _items6.length; _i6++) {
          _loop6.index0 = _i6; const _it6 = _items6[_i6]; let tool_26 = _it6;
          out.push("    {\"name\": \"", J.str(J.item(J.item(tool_26, "function"), "name")), "\", \"description\": \"", J.str(J.item(J.item(tool_26, "function"), "description")), "\", \"parameters\": ", J.str(J.filter("tojson", J.item(J.item(tool_26, "function"), "parameters"))), ", \"responses\": null}");
          if (J.t(!J.t(J.attr(_loop6, "last")))) {
            out.push(",");
          }
          out.push("\n");
        }
      }
    } else {
      out.push("\n");
    }
    out.push("]\n```");
    rendered_platform_turn_chunk_0 = true;
  }
  if (J.t(json_mode_0)) {
    if (J.t(rendered_platform_turn_chunk_0)) {
      out.push("\n\n");
    }
    out.push("When generating JSON objects, do not generate block markers. Generate an object directly without prefixing with ```json. Return only the JSON and nothing else.");
    if (J.t(json_schema_0)) {
      out.push("\nYour output should adhere to the following json schema:\n", J.str(json_schema_0));
    }
    rendered_platform_turn_chunk_0 = true;
  }
  if (J.t(rendered_platform_turn_chunk_0)) {
    out.push("<|END_TEXT|><|END_OF_TURN_TOKEN|>");
  } else if (J.t(!J.t(render_developer_instruction_0))) {
    out.push("<|END_OF_TURN_TOKEN|>");
  }
  if (J.t(render_developer_instruction_0)) {
    out.push("<|START_OF_TURN_TOKEN|><|SYSTEM_TOKEN|>", J.str(J.call(print_msg_0, developer_instruction_0)), "<|END_OF_TURN_TOKEN|>");
  }
  {
    let _items7 = J.iter(messages_0);
    const _loop7 = J.loop(_items7);
    for (let _i7 = 0; _i7 < _items7.length; _i7++) {
      _loop7.index0 = _i7; const _it7 = _items7[_i7]; let message_27 = _it7;
      let msg_role_downcased_28 = msg_role_downcased_0;
      msg_role_downcased_28 = J.filter("lower", J.attr(message_27, "role"));
      if (J.t(((v) => J.t(v) ? !J.t(((v) => J.t(v) ? initial_instruction_message_0 : v)(J.attr(_loop7, "first"))) : v)((J.eq(msg_role_downcased_28, "system"))))) {
        out.push("<|START_OF_TURN_TOKEN|><|SYSTEM_TOKEN|>", J.str(J.call(print_msg_0, message_27)), "<|END_OF_TURN_TOKEN|>");
      } else if (J.t((J.eq(msg_role_downcased_28, "user")))) {
        out.push("<|START_OF_TURN_TOKEN|><|USER_TOKEN|>", J.str(J.call(print_msg_0, message_27)), "<|END_OF_TURN_TOKEN|>");
        if (J.t(((v) => J.t(v) ? !J.t(J.attr(sent_documents_0, "value")) : v)(documents_0))) {
          J.setattr(sent_documents_0, "value", true);
          J.setattr(tool_idx_0, "value", J.add(J.attr(tool_idx_0, "value"), 1));
          out.push(J.str(J.call(document_turn_0, documents_0)));
        }
      } else if (J.t(((v) => J.t(v) ? v : (J.eq(msg_role_downcased_28, "chatbot")))((J.eq(msg_role_downcased_28, "assistant"))))) {
        out.push("<|START_OF_TURN_TOKEN|><|CHATBOT_TOKEN|>");
        if (J.t(J.attr(message_27, "tool_calls"))) {
          if (J.t(!J.t(skip_thinking_0))) {
            if (J.t(J.attr(message_27, "tool_plan"))) {
              out.push("<|START_THINKING|>", J.str(J.attr(message_27, "tool_plan")), "<|END_THINKING|>");
            } else if (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? (J.eq(J.attr(J.item(J.attr(message_27, "content"), 0), "type"), "thinking")) : v)(J.attr(message_27, "content")))(J.attr(message_27, "thinking")))) {
              out.push("<|START_THINKING|>", J.str(J.call(print_thinking_0, message_27)), "<|END_THINKING|>");
            }
          }
          out.push("<|START_ACTION|>[");
          {
            let _items8 = J.iter(J.attr(message_27, "tool_calls"));
            const _loop8 = J.loop(_items8);
            for (let _i8 = 0; _i8 < _items8.length; _i8++) {
              _loop8.index0 = _i8; const _it8 = _items8[_i8]; let tc_29 = _it8;
              out.push("\n    {\"tool_call_id\": \"");
              if (J.t(regen_tool_call_ids_0)) {
                out.push(J.str(J.attr(tool_idx_0, "value")));
              } else {
                out.push(J.str(J.attr(tc_29, "id")));
              }
              out.push("\", \"tool_name\": \"", J.str(J.item(J.item(tc_29, "function"), "name")), "\", \"parameters\": ", J.str(J.filter("tojson", J.item(J.item(tc_29, "function"), "arguments"))), "}");
              if (J.t(!J.t(J.attr(_loop8, "last")))) {
                out.push(",");
              }
              J.setattr(tool_idx_0, "value", J.add(J.attr(tool_idx_0, "value"), 1));
            }
          }
          out.push("\n]<|END_ACTION|><|END_OF_TURN_TOKEN|>");
        } else {
          if (J.t(((v) => J.t(v) ? !J.t(skip_thinking_0) : v)(((v) => J.t(v) ? v : ((v) => J.t(v) ? (J.eq(J.attr(J.item(J.attr(message_27, "content"), 0), "type"), "thinking")) : v)(J.attr(message_27, "content")))(J.attr(message_27, "thinking"))))) {
            out.push("<|START_THINKING|>", J.str(J.call(print_thinking_0, message_27)), "<|END_THINKING|>");
          }
          out.push(J.str(J.call(print_msg_0, message_27)), "<|END_OF_TURN_TOKEN|>");
        }
      } else if (J.t(((v) => J.t(v) ? (!J.has(J.attr(message_27, "tool_call_id"), J.attr(tool_ids_seen_0, "value"))) : v)((J.eq(msg_role_downcased_28, "tool"))))) {
        out.push("<|START_OF_TURN_TOKEN|><|SYSTEM_TOKEN|><|START_TOOL_RESULT|>[\n    ", J.str(J.call(format_tool_message_0, messages_0, message_27)));
        {
          let _items9 = J.iter(J.slice(messages_0, J.add(J.attr(_loop7, "index0"), 1), undefined, undefined));
          const _loop9 = J.loop(_items9);
          for (let _i9 = 0; _i9 < _items9.length; _i9++) {
            _loop9.index0 = _i9; const _it9 = _items9[_i9]; let msg_30 = _it9;
            if (J.t((J.eq(J.filter("lower", J.attr(msg_30, "role")), "tool")))) {
              out.push(",\n    ", J.str(J.call(format_tool_message_0, messages_0, msg_30)));
              J.setattr(tool_ids_seen_0, "value", J.add(J.attr(tool_ids_seen_0, "value"), [J.attr(msg_30, "tool_call_id")]));
            } else {
              break;
            }
          }
        }
        out.push("\n]<|END_TOOL_RESULT|><|END_OF_TURN_TOKEN|>");
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push("<|START_OF_TURN_TOKEN|><|CHATBOT_TOKEN|>");
    if (J.t(reasoning_0)) {
      out.push("<|START_THINKING|>");
    } else {
      out.push("<|START_THINKING|><|END_THINKING|>");
    }
  }
  return out.join("");
}
