// Converted from the chat template of swiss-ai/Apertus-8B-Instruct-2509 by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let render_typescript_type_0 = ctx["render_typescript_type"];
  let render_tools_0 = ctx["render_tools"];
  let system_token_0 = ctx["system_token"];
  let end_system_token_0 = ctx["end_system_token"];
  let developer_token_0 = ctx["developer_token"];
  let end_developer_token_0 = ctx["end_developer_token"];
  let user_token_0 = ctx["user_token"];
  let end_user_token_0 = ctx["end_user_token"];
  let assistant_token_0 = ctx["assistant_token"];
  let end_assistant_token_0 = ctx["end_assistant_token"];
  let inner_token_0 = ctx["inner_token"];
  let outer_token_0 = ctx["outer_token"];
  let tool_calls_token_0 = ctx["tool_calls_token"];
  let end_tool_calls_token_0 = ctx["end_tool_calls_token"];
  let image_token_0 = ctx["image_token"];
  let ns_0 = ctx["ns"];
  let loop_messages_0 = ctx["loop_messages"];
  let inner_type_0 = ctx["inner_type"];
  let has_object_variants_0 = ctx["has_object_variants"];
  let bos_token_0 = ctx["bos_token"];
  let messages_0 = ctx["messages"];
  let enable_thinking_0 = ctx["enable_thinking"];
  let tools_0 = ctx["tools"] ?? null;
  let parts_0 = ctx["parts"];
  let blocks_0 = ctx["blocks"];
  let function_0 = ctx["function"];
  let continue_assistant_message_0 = ctx["continue_assistant_message"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  render_typescript_type_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_4 = k.caller;
    let param_spec_1 = p.length > 0 ? p[0] : "param_spec" in k ? k["param_spec"] : undefined;
    let required_params_2 = p.length > 1 ? p[1] : "required_params" in k ? k["required_params"] : undefined;
    let is_nullable_3 = p.length > 2 ? p[2] : "is_nullable" in k ? k["is_nullable"] : false;
    let inner_type_5 = inner_type_0;
    let has_object_variants_6 = has_object_variants_0;
    if (J.t((J.eq(J.attr(param_spec_1, "type"), "array")))) {
      if (J.t(J.item(param_spec_1, "items"))) {
        if (J.t((J.eq(J.item(J.item(param_spec_1, "items"), "type"), "string")))) {
          out.push(J.str("string[]"));
        } else if (J.t((J.eq(J.item(J.item(param_spec_1, "items"), "type"), "number")))) {
          out.push(J.str("number[]"));
        } else if (J.t((J.eq(J.item(J.item(param_spec_1, "items"), "type"), "integer")))) {
          out.push(J.str("number[]"));
        } else if (J.t((J.eq(J.item(J.item(param_spec_1, "items"), "type"), "boolean")))) {
          out.push(J.str("boolean[]"));
        } else {
          inner_type_5 = J.call(render_typescript_type_0, J.item(param_spec_1, "items"), required_params_2);
          if (J.t(((v) => J.t(v) ? v : (J.gt(J.filter("length", inner_type_5), 50)))((J.eq(inner_type_5, "object | object"))))) {
            out.push(J.str("any[]"));
          } else {
            out.push(J.str(J.add(inner_type_5, "[]")));
          }
        }
        if (J.t(J.attr(param_spec_1, "nullable"))) {
          out.push(J.str(" | null"));
        }
      } else {
        out.push(J.str("any[]"));
        if (J.t(J.attr(param_spec_1, "nullable"))) {
          out.push(J.str(" | null"));
        }
      }
    } else if (J.t(((v) => J.t(v) ? (J.item(J.attr(param_spec_1, "type"), 0) !== undefined) : v)(((v) => J.t(v) ? !J.t(J.test("mapping", J.attr(param_spec_1, "type"))) : v)(((v) => J.t(v) ? !J.t(J.test("string", J.attr(param_spec_1, "type"))) : v)(((v) => J.t(v) ? J.test("iterable", J.attr(param_spec_1, "type")) : v)((J.attr(param_spec_1, "type") !== undefined))))))) {
      if (J.t((J.gt(J.filter("length", J.attr(param_spec_1, "type")), 1)))) {
        out.push(J.str(J.filter("join", J.attr(param_spec_1, "type"), " | ")));
      } else {
        out.push(J.str(J.item(J.attr(param_spec_1, "type"), 0)));
      }
    } else if (J.t(J.attr(param_spec_1, "oneOf"))) {
      has_object_variants_6 = false;
      {
        let _items1 = J.iter(J.attr(param_spec_1, "oneOf"));
        const _loop1 = J.loop(_items1);
        for (let _i1 = 0; _i1 < _items1.length; _i1++) {
          _loop1.index0 = _i1; const _it1 = _items1[_i1]; let variant_7 = _it1;
          let has_object_variants_8 = has_object_variants_6;
          if (J.t((J.eq(J.attr(variant_7, "type"), "object")))) {
            has_object_variants_8 = true;
          }
        }
      }
      if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", J.attr(param_spec_1, "oneOf")), 1)) : v)(has_object_variants_6))) {
        out.push(J.str("any"));
      } else {
        {
          let _items2 = J.iter(J.attr(param_spec_1, "oneOf"));
          const _loop2 = J.loop(_items2);
          for (let _i2 = 0; _i2 < _items2.length; _i2++) {
            _loop2.index0 = _i2; const _it2 = _items2[_i2]; let variant_9 = _it2;
            out.push(J.str(J.call(render_typescript_type_0, variant_9, required_params_2)));
            if (J.t(J.attr(variant_9, "description"))) {
              out.push(J.str(J.add("// ", J.attr(variant_9, "description"))));
            }
            if (J.t((J.attr(variant_9, "default") !== undefined))) {
              out.push("                    ", J.str(J.add("// default: ", J.filter("tojson", J.attr(variant_9, "default")))));
            }
            if (J.t(!J.t(J.attr(_loop2, "last")))) {
              out.push(J.str(" | "), "\n");
            }
          }
        }
      }
    } else if (J.t((J.eq(J.attr(param_spec_1, "type"), "string")))) {
      if (J.t(J.attr(param_spec_1, "enum"))) {
        out.push(J.str(J.add(J.add("\"", J.filter("join", J.attr(param_spec_1, "enum"), "\" | \"")), "\"")));
      } else {
        out.push(J.str("string"));
        if (J.t(J.attr(param_spec_1, "nullable"))) {
          out.push(J.str(" | null"));
        }
      }
    } else if (J.t((J.eq(J.attr(param_spec_1, "type"), "number")))) {
      out.push(J.str("number"));
    } else if (J.t((J.eq(J.attr(param_spec_1, "type"), "integer")))) {
      out.push(J.str("number"));
    } else if (J.t((J.eq(J.attr(param_spec_1, "type"), "boolean")))) {
      out.push(J.str("boolean"));
    } else if (J.t((J.eq(J.attr(param_spec_1, "type"), "object")))) {
      if (J.t(J.attr(param_spec_1, "properties"))) {
        out.push(J.str("{\n"));
        {
          let _items3 = J.iter(J.call(J.attr(J.attr(param_spec_1, "properties"), "items")));
          const _loop3 = J.loop(_items3);
          for (let _i3 = 0; _i3 < _items3.length; _i3++) {
            _loop3.index0 = _i3; const _it3 = _items3[_i3]; let [prop_name_10, prop_spec_11] = J.unpack(_it3, 2);
            out.push(J.str(prop_name_10));
            if (J.t((!J.has(prop_name_10, ((v) => J.t(v) ? v : [])(J.attr(param_spec_1, "required")))))) {
              out.push(J.str("?"));
            }
            out.push(J.str(": "), "\n                ", J.str(J.call(render_typescript_type_0, prop_spec_11, ((v) => J.t(v) ? v : [])(J.attr(param_spec_1, "required")))));
            if (J.t(!J.t(J.attr(_loop3, "last")))) {
              out.push(J.str(", "));
            }
          }
        }
        out.push(J.str("}"));
      } else {
        out.push(J.str("object"));
      }
    } else {
      out.push(J.str("any"));
    }
    return out.join("");
  };
  render_tools_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_13 = k.caller;
    let tools_12 = p.length > 0 ? p[0] : "tools" in k ? k["tools"] : undefined;
    {
      let _items4 = J.iter(tools_12);
      const _loop4 = J.loop(_items4);
      for (let _i4 = 0; _i4 < _items4.length; _i4++) {
        _loop4.index0 = _i4; const _it4 = _items4[_i4]; let tool_14 = _it4;
        out.push(J.str(J.add(J.add("// ", J.attr(tool_14, "description")), "\n")), J.str(J.add(J.add("type ", J.attr(tool_14, "name")), " = ")));
        if (J.t(((v) => J.t(v) ? J.attr(J.attr(tool_14, "parameters"), "properties") : v)(J.attr(tool_14, "parameters")))) {
          out.push(J.str("(_: {\n"));
          {
            let _items5 = J.iter(J.call(J.attr(J.attr(J.attr(tool_14, "parameters"), "properties"), "items")));
            const _loop5 = J.loop(_items5);
            for (let _i5 = 0; _i5 < _items5.length; _i5++) {
              _loop5.index0 = _i5; const _it5 = _items5[_i5]; let [param_name_15, param_spec_16] = J.unpack(_it5, 2);
              if (J.t(J.attr(param_spec_16, "description"))) {
                out.push(J.str(J.add(J.add("// ", J.attr(param_spec_16, "description")), "\n")));
              }
              out.push(J.str(param_name_15));
              if (J.t((!J.has(param_name_15, ((v) => J.t(v) ? v : [])(J.attr(J.attr(tool_14, "parameters"), "required")))))) {
                out.push(J.str("?"));
              }
              out.push(J.str(": "), J.str(J.call(render_typescript_type_0, param_spec_16, ((v) => J.t(v) ? v : [])(J.attr(J.attr(tool_14, "parameters"), "required")))));
              if (J.t((J.attr(param_spec_16, "default") !== undefined))) {
                if (J.t(J.attr(param_spec_16, "enum"))) {
                  out.push(J.str(J.add(", // default: ", J.attr(param_spec_16, "default"))));
                } else if (J.t(J.attr(param_spec_16, "oneOf"))) {
                  out.push(J.str(J.add("// default: ", J.attr(param_spec_16, "default"))));
                } else {
                  out.push(J.str(J.add(", // default: ", J.filter("tojson", J.attr(param_spec_16, "default")))));
                }
              }
              if (J.t(!J.t(J.attr(_loop5, "last")))) {
                out.push(J.str(",\n"));
              } else {
                out.push(J.str("\n"));
              }
            }
          }
          out.push(J.str("}) => any;"));
        } else {
          out.push(J.str("() => any;"));
        }
        if (J.t(!J.t(J.attr(_loop4, "last")))) {
          out.push(J.str("\n"));
        }
      }
    }
    return out.join("");
  };
  out.push(J.str(bos_token_0));
  system_token_0 = "<|system_start|>";
  end_system_token_0 = "<|system_end|>";
  developer_token_0 = "<|developer_start|>";
  end_developer_token_0 = "<|developer_end|>";
  user_token_0 = "<|user_start|>";
  end_user_token_0 = "<|user_end|>";
  assistant_token_0 = "<|assistant_start|>";
  end_assistant_token_0 = "<|assistant_end|>";
  inner_token_0 = "<|inner_prefix|>";
  outer_token_0 = "<|inner_suffix|>";
  tool_calls_token_0 = "<|tools_prefix|>";
  end_tool_calls_token_0 = "<|tools_suffix|>";
  image_token_0 = "<|image|>";
  ns_0 = J.call(J.namespace, J.kw({"in_assistant": false, "in_tool": false, "in_inner": false, "waiting_for_tool_outputs": false, "assistant_format": null}));
  if (J.t(((v) => J.t(v) ? (J.eq(J.attr(J.item(messages_0, 0), "role"), "system")) : v)(messages_0))) {
    if (J.t((J.has("content", J.item(messages_0, 0))))) {
      if (J.t(J.test("string", J.attr(J.item(messages_0, 0), "content")))) {
        out.push(J.str(J.add(J.add(system_token_0, J.attr(J.item(messages_0, 0), "content")), end_system_token_0)));
      } else if (J.t(((v) => J.t(v) ? (J.has("text", J.attr(J.item(messages_0, 0), "content"))) : v)(J.test("mapping", J.attr(J.item(messages_0, 0), "content"))))) {
        out.push(J.str(J.add(J.add(system_token_0, J.attr(J.attr(J.item(messages_0, 0), "content"), "text")), end_system_token_0)));
      } else {
        out.push(J.str(J.call(J.raise_exception, "Invalid system message")));
      }
    } else {
      out.push(J.str(J.call(J.raise_exception, "Invalid system message")));
    }
    loop_messages_0 = J.slice(messages_0, 1, undefined, undefined);
  } else {
    out.push(J.str(J.add(J.add(J.add(system_token_0, "You are Apertus, a helpful assistant created by the SwissAI initiative.\nKnowledge cutoff: 2024-04\nCurrent date: "), J.call(J.strftime_now, "%Y-%m-%d")), end_system_token_0)));
    loop_messages_0 = messages_0;
  }
  out.push(J.str(J.add(developer_token_0, "Deliberation: ")));
  if (J.t(((v) => J.t(v) ? enable_thinking_0 : v)((enable_thinking_0 !== undefined)))) {
    out.push(J.str("enabled\n"));
  } else {
    out.push(J.str("disabled\n"));
  }
  if (J.t(((v) => J.t(v) ? tools_0 : v)((tools_0 !== undefined)))) {
    out.push(J.str(J.add("Tool Capabilities:\n", J.call(render_tools_0, tools_0))));
  } else {
    out.push(J.str("Tool Capabilities: disabled"));
  }
  out.push(J.str(end_developer_token_0));
  {
    let _items6 = J.iter(loop_messages_0);
    const _loop6 = J.loop(_items6);
    for (let _i6 = 0; _i6 < _items6.length; _i6++) {
      _loop6.index0 = _i6; const _it6 = _items6[_i6]; let message_17 = _it6;
      let parts_18 = parts_0;
      let blocks_19 = blocks_0;
      if (J.t((J.eq(J.attr(message_17, "role"), "user")))) {
        J.setattr(ns_0, "in_inner", false);
        if (J.t(J.attr(ns_0, "in_tool"))) {
          out.push(J.str("]"));
          J.setattr(ns_0, "in_tool", false);
        }
        if (J.t(J.attr(ns_0, "in_assistant"))) {
          out.push(J.str(end_assistant_token_0));
          J.setattr(ns_0, "in_assistant", false);
        }
        if (J.t((J.has("content", message_17)))) {
          out.push(J.str(user_token_0));
          if (J.t(J.test("string", J.attr(message_17, "content")))) {
            out.push(J.str(J.attr(message_17, "content")));
          } else if (J.t(((v) => J.t(v) ? (J.has("parts", J.attr(message_17, "content"))) : v)(J.test("mapping", J.attr(message_17, "content"))))) {
            parts_18 = J.attr(J.attr(message_17, "content"), "parts");
            {
              let _items7 = J.iter(parts_18);
              const _loop7 = J.loop(_items7);
              for (let _i7 = 0; _i7 < _items7.length; _i7++) {
                _loop7.index0 = _i7; const _it7 = _items7[_i7]; let part_20 = _it7;
                if (J.t((J.eq(J.attr(part_20, "type"), "text")))) {
                  out.push(J.str(J.attr(part_20, "text")));
                } else if (J.t((J.eq(J.attr(part_20, "type"), "image")))) {
                  out.push(J.str(image_token_0));
                } else {
                  out.push(J.str(J.call(J.raise_exception, J.add("Invalid user part: ", J.attr(part_20, "type")))));
                }
              }
            }
          } else {
            out.push(J.str(J.call(J.raise_exception, J.add("Invalid user message: ", J.attr(message_17, "role")))));
          }
          out.push(J.str(end_user_token_0));
        }
      } else if (J.t((J.eq(J.attr(message_17, "role"), "assistant")))) {
        if (J.t(!J.t(J.attr(ns_0, "in_assistant")))) {
          out.push(J.str(assistant_token_0));
          J.setattr(ns_0, "in_assistant", true);
        }
        if (J.t((J.has("content", message_17)))) {
          if (J.t(((v) => J.t(v) ? ((v) => J.t(v) ? v : (J.eq(J.attr(ns_0, "assistant_format"), "string")))((J.attr(ns_0, "assistant_format") === null)) : v)(J.test("string", J.attr(message_17, "content"))))) {
            if (J.t(J.attr(ns_0, "in_tool"))) {
              out.push(J.str("]"));
              J.setattr(ns_0, "in_tool", false);
            }
            J.setattr(ns_0, "assistant_format", "string");
            out.push(J.str(J.attr(message_17, "content")));
          } else if (J.t(((v) => J.t(v) ? ((v) => J.t(v) ? v : (J.eq(J.attr(ns_0, "assistant_format"), "mapping")))((J.attr(ns_0, "assistant_format") === null)) : v)(((v) => J.t(v) ? (J.has("blocks", J.attr(message_17, "content"))) : v)(J.test("mapping", J.attr(message_17, "content")))))) {
            J.setattr(ns_0, "assistant_format", "mapping");
            blocks_19 = J.attr(J.attr(message_17, "content"), "blocks");
            {
              let _items8 = J.iter(blocks_19);
              const _loop8 = J.loop(_items8);
              for (let _i8 = 0; _i8 < _items8.length; _i8++) {
                _loop8.index0 = _i8; const _it8 = _items8[_i8]; let block_21 = _it8;
                if (J.t((J.eq(J.attr(block_21, "type"), "thoughts")))) {
                  if (J.t(J.attr(ns_0, "in_tool"))) {
                    out.push(J.str("]"));
                    J.setattr(ns_0, "in_tool", false);
                  }
                  if (J.t(!J.t(J.attr(ns_0, "in_inner")))) {
                    J.setattr(ns_0, "in_inner", true);
                    out.push(J.str(inner_token_0));
                  }
                  out.push(J.str(J.attr(block_21, "text")));
                } else if (J.t((J.eq(J.attr(block_21, "type"), "tool_calls")))) {
                  if (J.t(J.attr(ns_0, "in_tool"))) {
                    out.push(J.str("]"));
                    J.setattr(ns_0, "in_tool", false);
                  }
                  if (J.t(((v) => J.t(v) ? (J.eq(J.attr(J.item(J.attr(block_21, "calls"), 0), "name"), "display_answers")) : v)(((v) => J.t(v) ? (J.eq(J.filter("length", J.attr(block_21, "calls")), 1)) : v)(((v) => J.t(v) ? !J.t(J.attr(_loop8, "first")) : v)(J.attr(ns_0, "in_inner")))))) {
                    J.setattr(ns_0, "in_inner", false);
                    out.push(J.str(outer_token_0));
                  }
                  out.push(J.str(J.add(tool_calls_token_0, "[")));
                  {
                    let _items9 = J.iter(J.attr(block_21, "calls"));
                    const _loop9 = J.loop(_items9);
                    for (let _i9 = 0; _i9 < _items9.length; _i9++) {
                      _loop9.index0 = _i9; const _it9 = _items9[_i9]; let tool_call_22 = _it9;
                      out.push(J.str(J.add(J.add(J.add(J.add("{\"", J.attr(tool_call_22, "name")), "\": "), J.attr(tool_call_22, "arguments")), "}")));
                      if (J.t(!J.t(J.attr(_loop9, "last")))) {
                        out.push(J.str(", "));
                      }
                    }
                  }
                  out.push(J.str(J.add("]", end_tool_calls_token_0)));
                  J.setattr(ns_0, "waiting_for_tool_outputs", true);
                } else if (J.t((J.eq(J.attr(block_21, "type"), "tool_outputs")))) {
                  if (J.t(J.attr(ns_0, "in_tool"))) {
                    out.push(J.str(J.call(J.raise_exception, "Cannot have both tool outputs as separate messages and tool outputs as blocks")));
                  }
                  out.push(J.str("["));
                  {
                    let _items10 = J.iter(J.attr(block_21, "outputs"));
                    const _loop10 = J.loop(_items10);
                    for (let _i10 = 0; _i10 < _items10.length; _i10++) {
                      _loop10.index0 = _i10; const _it10 = _items10[_i10]; let tool_output_23 = _it10;
                      out.push(J.str(J.attr(tool_output_23, "output")));
                      if (J.t(!J.t(J.attr(_loop10, "last")))) {
                        out.push(J.str(", "));
                      }
                    }
                  }
                  out.push(J.str("]"));
                  J.setattr(ns_0, "waiting_for_tool_outputs", false);
                } else if (J.t((J.eq(J.attr(block_21, "type"), "response")))) {
                  if (J.t(J.attr(ns_0, "in_tool"))) {
                    out.push(J.str("]"));
                    J.setattr(ns_0, "in_tool", false);
                  }
                  if (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? J.attr(ns_0, "in_inner") : v)(J.attr(ns_0, "in_assistant")))(((v) => J.t(v) ? J.attr(ns_0, "in_inner") : v)(!J.t(J.attr(_loop8, "first")))))) {
                    J.setattr(ns_0, "in_inner", false);
                    out.push(J.str(outer_token_0));
                  }
                  out.push(J.str(J.attr(block_21, "text")));
                } else {
                  out.push(J.str(J.call(J.raise_exception, J.add("Invalid assistant block type: ", J.attr(block_21, "type")))));
                }
              }
            }
          } else {
            out.push(J.str(J.call(J.raise_exception, "Invalid assistant content")));
          }
        } else {
          out.push(J.str(J.call(J.raise_exception, "Invalid assistant message")));
        }
        if (J.t(((v) => J.t(v) ? J.attr(message_17, "tool_calls") : v)((J.has("tool_calls", message_17))))) {
          out.push(J.str(J.add(tool_calls_token_0, "[")));
          {
            let _items11 = J.iter(J.attr(message_17, "tool_calls"));
            const _loop11 = J.loop(_items11);
            for (let _i11 = 0; _i11 < _items11.length; _i11++) {
              _loop11.index0 = _i11; const _it11 = _items11[_i11]; let tool_call_24 = _it11;
              let function_25 = function_0;
              if (J.t((J.eq(J.attr(tool_call_24, "type"), "function")))) {
                function_25 = J.attr(tool_call_24, "function");
                out.push(J.str(J.add(J.add(J.add(J.add("{\"", J.attr(function_25, "name")), "\": "), J.attr(function_25, "arguments")), "}")));
                if (J.t(!J.t(J.attr(_loop11, "last")))) {
                  out.push(J.str(", "));
                }
              } else {
                out.push(J.str(J.call(J.raise_exception, J.add("Invalid tool call type: ", J.attr(tool_call_24, "type")))));
              }
            }
          }
          out.push(J.str(J.add("]", end_tool_calls_token_0)));
          J.setattr(ns_0, "waiting_for_tool_outputs", true);
        }
      } else if (J.t((J.eq(J.attr(message_17, "role"), "tool")))) {
        if (J.t(!J.t(J.attr(ns_0, "in_assistant")))) {
          out.push(J.str(J.call(J.raise_exception, "Tool message outside of assistant")));
        }
        if (J.t(!J.t(J.attr(ns_0, "in_tool")))) {
          out.push(J.str("["));
          J.setattr(ns_0, "in_tool", true);
        } else {
          out.push(J.str(", "));
        }
        out.push(J.str(J.attr(message_17, "content")));
        J.setattr(ns_0, "waiting_for_tool_outputs", false);
      } else {
        out.push(J.str(J.call(J.raise_exception, "Invalid message role")));
      }
    }
  }
  if (J.t(J.attr(ns_0, "in_tool"))) {
    out.push(J.str("]"));
  }
  if (J.t(((v) => J.t(v) ? !J.t(J.attr(ns_0, "waiting_for_tool_outputs")) : v)(((v) => J.t(v) ? !J.t(((v) => J.t(v) ? continue_assistant_message_0 : v)((continue_assistant_message_0 !== undefined))) : v)(J.attr(ns_0, "in_assistant"))))) {
    out.push(J.str(end_assistant_token_0));
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str(assistant_token_0));
  }
  return out.join("");
}
