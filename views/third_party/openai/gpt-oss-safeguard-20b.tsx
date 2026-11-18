// Converted from the chat template of openai/gpt-oss-safeguard-20b by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let render_typescript_type_0 = ctx["render_typescript_type"];
  let render_tool_namespace_0 = ctx["render_tool_namespace"];
  let render_builtin_tools_0 = ctx["render_builtin_tools"];
  let build_system_message_0 = ctx["build_system_message"];
  let developer_message_0 = ctx["developer_message"];
  let loop_messages_0 = ctx["loop_messages"];
  let last_tool_call_0 = ctx["last_tool_call"];
  let inner_type_0 = ctx["inner_type"];
  let has_object_variants_0 = ctx["has_object_variants"];
  let model_identity_0 = ctx["model_identity"];
  let reasoning_effort_0 = ctx["reasoning_effort"];
  let available_builtin_tools_0 = ctx["available_builtin_tools"];
  let builtin_tools_0 = ctx["builtin_tools"];
  let tools_0 = ctx["tools"] ?? null;
  let messages_0 = ctx["messages"];
  let future_final_message_0 = ctx["future_final_message"];
  let tool_call_0 = ctx["tool_call"];
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
  render_tool_namespace_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_14 = k.caller;
    let namespace_name_12 = p.length > 0 ? p[0] : "namespace_name" in k ? k["namespace_name"] : undefined;
    let tools_13 = p.length > 1 ? p[1] : "tools" in k ? k["tools"] : undefined;
    out.push(J.str(J.add(J.add("## ", namespace_name_12), "\n\n")), J.str(J.add(J.add("namespace ", namespace_name_12), " {\n\n")));
    {
      let _items4 = J.iter(tools_13);
      const _loop4 = J.loop(_items4);
      for (let _i4 = 0; _i4 < _items4.length; _i4++) {
        _loop4.index0 = _i4; const _it4 = _items4[_i4]; let tool_15 = _it4;
        tool_15 = J.attr(tool_15, "function");
        out.push(J.str(J.add(J.add("// ", J.attr(tool_15, "description")), "\n")), J.str(J.add(J.add("type ", J.attr(tool_15, "name")), " = ")));
        if (J.t(((v) => J.t(v) ? J.attr(J.attr(tool_15, "parameters"), "properties") : v)(J.attr(tool_15, "parameters")))) {
          out.push(J.str("(_: {\n"));
          {
            let _items5 = J.iter(J.call(J.attr(J.attr(J.attr(tool_15, "parameters"), "properties"), "items")));
            const _loop5 = J.loop(_items5);
            for (let _i5 = 0; _i5 < _items5.length; _i5++) {
              _loop5.index0 = _i5; const _it5 = _items5[_i5]; let [param_name_16, param_spec_17] = J.unpack(_it5, 2);
              if (J.t(J.attr(param_spec_17, "description"))) {
                out.push(J.str(J.add(J.add("// ", J.attr(param_spec_17, "description")), "\n")));
              }
              out.push(J.str(param_name_16));
              if (J.t((!J.has(param_name_16, ((v) => J.t(v) ? v : [])(J.attr(J.attr(tool_15, "parameters"), "required")))))) {
                out.push(J.str("?"));
              }
              out.push(J.str(": "), J.str(J.call(render_typescript_type_0, param_spec_17, ((v) => J.t(v) ? v : [])(J.attr(J.attr(tool_15, "parameters"), "required")))));
              if (J.t((J.attr(param_spec_17, "default") !== undefined))) {
                if (J.t(J.attr(param_spec_17, "enum"))) {
                  out.push(J.str(J.add(", // default: ", J.attr(param_spec_17, "default"))));
                } else if (J.t(J.attr(param_spec_17, "oneOf"))) {
                  out.push(J.str(J.add("// default: ", J.attr(param_spec_17, "default"))));
                } else {
                  out.push(J.str(J.add(", // default: ", J.filter("tojson", J.attr(param_spec_17, "default")))));
                }
              }
              if (J.t(!J.t(J.attr(_loop5, "last")))) {
                out.push(J.str(",\n"));
              } else {
                out.push(J.str(",\n"));
              }
            }
          }
          out.push(J.str("}) => any;\n\n"));
        } else {
          out.push(J.str("() => any;\n\n"));
        }
      }
    }
    out.push(J.str(J.add("} // namespace ", namespace_name_12)));
    return out.join("");
  };
  render_builtin_tools_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_20 = k.caller;
    let browser_tool_18 = p.length > 0 ? p[0] : "browser_tool" in k ? k["browser_tool"] : undefined;
    let python_tool_19 = p.length > 1 ? p[1] : "python_tool" in k ? k["python_tool"] : undefined;
    if (J.t(browser_tool_18)) {
      out.push(J.str("## browser\n\n"), J.str("// Tool for browsing.\n"), J.str("// The `cursor` appears in brackets before each browsing display: `[{cursor}]`.\n"), J.str("// Cite information from the tool using the following format:\n"), J.str("// `\u3010{cursor}\u2020L{line_start}(-L{line_end})?\u3011`, for example: `\u30106\u2020L9-L11\u3011` or `\u30108\u2020L3\u3011`.\n"), J.str("// Do not quote more than 10 words directly from the tool output.\n"), J.str("// sources=web (default: web)\n"), J.str("namespace browser {\n\n"), J.str("// Searches for information related to `query` and displays `topn` results.\n"), J.str("type search = (_: {\n"), J.str("query: string,\n"), J.str("topn?: number, // default: 10\n"), J.str("source?: string,\n"), J.str("}) => any;\n\n"), J.str("// Opens the link `id` from the page indicated by `cursor` starting at line number `loc`, showing `num_lines` lines.\n"), J.str("// Valid link ids are displayed with the formatting: `\u3010{id}\u2020.*\u3011`.\n"), J.str("// If `cursor` is not provided, the most recent page is implied.\n"), J.str("// If `id` is a string, it is treated as a fully qualified URL associated with `source`.\n"), J.str("// If `loc` is not provided, the viewport will be positioned at the beginning of the document or centered on the most relevant passage, if available.\n"), J.str("// Use this function without `id` to scroll to a new location of an opened page.\n"), J.str("type open = (_: {\n"), J.str("id?: number | string, // default: -1\n"), J.str("cursor?: number, // default: -1\n"), J.str("loc?: number, // default: -1\n"), J.str("num_lines?: number, // default: -1\n"), J.str("view_source?: boolean, // default: false\n"), J.str("source?: string,\n"), J.str("}) => any;\n\n"), J.str("// Finds exact matches of `pattern` in the current page, or the page given by `cursor`.\n"), J.str("type find = (_: {\n"), J.str("pattern: string,\n"), J.str("cursor?: number, // default: -1\n"), J.str("}) => any;\n\n"), J.str("} // namespace browser\n\n"));
    }
    if (J.t(python_tool_19)) {
      out.push(J.str("## python\n\n"), J.str("Use this tool to execute Python code in your chain of thought. The code will not be shown to the user. This tool should be used for internal reasoning, but not for code that is intended to be visible to the user (e.g. when creating plots, tables, or files).\n\n"), J.str("When you send a message containing Python code to python, it will be executed in a stateful Jupyter notebook environment. python will respond with the output of the execution or time out after 120.0 seconds. The drive at '/mnt/data' can be used to save and persist user files. Internet access for this session is UNKNOWN. Depends on the cluster.\n\n"));
    }
    return out.join("");
  };
  build_system_message_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_21 = k.caller;
    let model_identity_22 = model_identity_0;
    let reasoning_effort_23 = reasoning_effort_0;
    let available_builtin_tools_24 = available_builtin_tools_0;
    if (J.t(!J.t((model_identity_22 !== undefined)))) {
      model_identity_22 = "You are ChatGPT, a large language model trained by OpenAI.";
    }
    out.push(J.str(J.add(model_identity_22, "\n")), J.str("Knowledge cutoff: 2024-06\n"), J.str(J.add(J.add("Current date: ", J.call(J.strftime_now, "%Y-%m-%d")), "\n\n")));
    if (J.t(!J.t((reasoning_effort_23 !== undefined)))) {
      reasoning_effort_23 = "medium";
    }
    out.push(J.str(J.add(J.add("Reasoning: ", reasoning_effort_23), "\n\n")));
    if (J.t(builtin_tools_0)) {
      out.push(J.str("# Tools\n\n"));
      available_builtin_tools_24 = J.call(J.namespace, J.kw({"browser": false, "python": false}));
      {
        let _items6 = J.iter(builtin_tools_0);
        const _loop6 = J.loop(_items6);
        for (let _i6 = 0; _i6 < _items6.length; _i6++) {
          _loop6.index0 = _i6; const _it6 = _items6[_i6]; let tool_25 = _it6;
          if (J.t((J.eq(tool_25, "browser")))) {
            J.setattr(available_builtin_tools_24, "browser", true);
          } else if (J.t((J.eq(tool_25, "python")))) {
            J.setattr(available_builtin_tools_24, "python", true);
          }
        }
      }
      out.push(J.str(J.call(render_builtin_tools_0, J.attr(available_builtin_tools_24, "browser"), J.attr(available_builtin_tools_24, "python"))));
    }
    out.push(J.str("# Valid channels: analysis, commentary, final. Channel must be included for every message."));
    if (J.t(tools_0)) {
      out.push(J.str("\nCalls to these tools must go to the commentary channel: 'functions'."));
    }
    return out.join("");
  };
  out.push(J.str("<|start|>system<|message|>"), J.str(J.call(build_system_message_0)), J.str("<|end|>"));
  if (J.t(((v) => J.t(v) ? v : (J.eq(J.attr(J.item(messages_0, 0), "role"), "system")))((J.eq(J.attr(J.item(messages_0, 0), "role"), "developer"))))) {
    developer_message_0 = J.attr(J.item(messages_0, 0), "content");
    loop_messages_0 = J.slice(messages_0, 1, undefined, undefined);
  } else {
    developer_message_0 = "";
    loop_messages_0 = messages_0;
  }
  if (J.t(((v) => J.t(v) ? v : tools_0)(developer_message_0))) {
    out.push(J.str("<|start|>developer<|message|>"));
    if (J.t(developer_message_0)) {
      out.push(J.str("# Instructions\n\n"), J.str(developer_message_0));
    }
    if (J.t(tools_0)) {
      out.push(J.str("\n\n"), J.str("# Tools\n\n"), J.str(J.call(render_tool_namespace_0, "functions", tools_0)));
    }
    out.push(J.str("<|end|>"));
  }
  last_tool_call_0 = J.call(J.namespace, J.kw({"name": null}));
  {
    let _items7 = J.iter(loop_messages_0);
    const _loop7 = J.loop(_items7);
    for (let _i7 = 0; _i7 < _items7.length; _i7++) {
      _loop7.index0 = _i7; const _it7 = _items7[_i7]; let message_26 = _it7;
      let future_final_message_27 = future_final_message_0;
      let tool_call_28 = tool_call_0;
      if (J.t((J.eq(J.attr(message_26, "role"), "assistant")))) {
        if (J.t((J.has("content", message_26)))) {
          if (J.t(((v) => J.t(v) ? v : (J.has("<|channel|>final<|message|>", J.attr(message_26, "content"))))((J.has("<|channel|>analysis<|message|>", J.attr(message_26, "content")))))) {
            out.push(J.str(J.call(J.raise_exception, "You have passed a message containing <|channel|> tags in the content field. Instead of doing this, you should pass analysis messages (the string between '<|message|>' and '<|end|>') in the 'thinking' field, and final messages (the string between '<|message|>' and '<|end|>') in the 'content' field.")));
          }
        }
        if (J.t((J.has("thinking", message_26)))) {
          if (J.t(((v) => J.t(v) ? v : (J.has("<|channel|>final<|message|>", J.attr(message_26, "thinking"))))((J.has("<|channel|>analysis<|message|>", J.attr(message_26, "thinking")))))) {
            out.push(J.str(J.call(J.raise_exception, "You have passed a message containing <|channel|> tags in the thinking field. Instead of doing this, you should pass analysis messages (the string between '<|message|>' and '<|end|>') in the 'thinking' field, and final messages (the string between '<|message|>' and '<|end|>') in the 'content' field.")));
          }
        }
        if (J.t((J.has("tool_calls", message_26)))) {
          future_final_message_27 = J.call(J.namespace, J.kw({"found": false}));
          {
            let _items8 = J.iter(J.slice(loop_messages_0, J.attr(_loop7, "index"), undefined, undefined));
            const _loop8 = J.loop(_items8);
            for (let _i8 = 0; _i8 < _items8.length; _i8++) {
              _loop8.index0 = _i8; const _it8 = _items8[_i8]; let future_message_29 = _it8;
              if (J.t(((v) => J.t(v) ? (!J.has("tool_calls", future_message_29)) : v)((J.eq(J.attr(future_message_29, "role"), "assistant"))))) {
                J.setattr(future_final_message_27, "found", true);
              }
            }
          }
          tool_call_28 = J.item(J.attr(message_26, "tool_calls"), 0);
          if (J.t(J.attr(tool_call_28, "function"))) {
            tool_call_28 = J.attr(tool_call_28, "function");
          }
          if (J.t(((v) => J.t(v) ? J.attr(message_26, "thinking") : v)(J.attr(message_26, "content")))) {
            out.push(J.str(J.call(J.raise_exception, "Cannot pass both content and thinking in an assistant message with tool calls! Put the analysis message in one or the other, but not both.")));
          } else if (J.t(((v) => J.t(v) ? !J.t(J.attr(future_final_message_27, "found")) : v)(J.attr(message_26, "content")))) {
            out.push(J.str(J.add(J.add("<|start|>assistant<|channel|>analysis<|message|>", J.attr(message_26, "content")), "<|end|>")));
          } else if (J.t(((v) => J.t(v) ? !J.t(J.attr(future_final_message_27, "found")) : v)(J.attr(message_26, "thinking")))) {
            out.push(J.str(J.add(J.add("<|start|>assistant<|channel|>analysis<|message|>", J.attr(message_26, "thinking")), "<|end|>")));
          }
          out.push(J.str("<|start|>assistant to="), J.str(J.add(J.add("functions.", J.attr(tool_call_28, "name")), "<|channel|>commentary ")), J.str(J.add((J.t((J.attr(tool_call_28, "content_type") !== undefined)) ? J.attr(tool_call_28, "content_type") : "json"), "<|message|>")), J.str(J.filter("tojson", J.attr(tool_call_28, "arguments"))), J.str("<|call|>"));
          J.setattr(last_tool_call_0, "name", J.attr(tool_call_28, "name"));
        } else if (J.t(((v) => J.t(v) ? !J.t(add_generation_prompt_0) : v)(J.attr(_loop7, "last")))) {
          if (J.t((J.has("thinking", message_26)))) {
            out.push(J.str(J.add(J.add("<|start|>assistant<|channel|>analysis<|message|>", J.attr(message_26, "thinking")), "<|end|>")));
          }
          out.push(J.str(J.add(J.add("<|start|>assistant<|channel|>final<|message|>", J.attr(message_26, "content")), "<|return|>")));
        } else {
          out.push(J.str(J.add(J.add("<|start|>assistant<|channel|>final<|message|>", J.attr(message_26, "content")), "<|end|>")));
          J.setattr(last_tool_call_0, "name", null);
        }
      } else if (J.t((J.eq(J.attr(message_26, "role"), "tool")))) {
        if (J.t((J.attr(last_tool_call_0, "name") === null))) {
          out.push(J.str(J.call(J.raise_exception, "Message has tool role, but there was no previous assistant message with a tool call!")));
        }
        out.push(J.str(J.add("<|start|>functions.", J.attr(last_tool_call_0, "name"))), J.str(J.add(J.add(" to=assistant<|channel|>commentary<|message|>", J.filter("tojson", J.attr(message_26, "content"))), "<|end|>")));
      } else if (J.t((J.eq(J.attr(message_26, "role"), "user")))) {
        out.push(J.str(J.add(J.add("<|start|>user<|message|>", J.attr(message_26, "content")), "<|end|>")));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push("<|start|>assistant");
  }
  return out.join("");
}
