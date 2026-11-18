// Converted from the chat template of reflection/Beam-1.0-501B-A23B by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let ts_type_name_0 = ctx["ts_type_name"];
  let render_default_0 = ctx["render_default"];
  let render_nullable_type_0 = ctx["render_nullable_type"];
  let json_schema_to_typescript_0 = ctx["json_schema_to_typescript"];
  let render_tool_declaration_0 = ctx["render_tool_declaration"];
  let render_tools_section_0 = ctx["render_tools_section"];
  let render_json_value_0 = ctx["render_json_value"];
  let render_parameter_value_0 = ctx["render_parameter_value"];
  let render_tool_call_body_0 = ctx["render_tool_call_body"];
  let build_system_message_0 = ctx["build_system_message"];
  let builtin_tool_names_0 = ctx["builtin_tool_names"];
  let trained_bash_tool_0 = ctx["trained_bash_tool"];
  let tool_groups_0 = ctx["tool_groups"];
  let leading_instructions_0 = ctx["leading_instructions"];
  let developer_message_0 = ctx["developer_message"];
  let loop_messages_0 = ctx["loop_messages"];
  let has_tools_0 = ctx["has_tools"];
  let tool_call_registry_0 = ctx["tool_call_registry"];
  let is_enum_0 = ctx["is_enum"];
  let rendered_0 = ctx["rendered"];
  let rendered_enum_0 = ctx["rendered_enum"];
  let required_0 = ctx["required"];
  let rendered_types_0 = ctx["rendered_types"];
  let property_description_0 = ctx["property_description"];
  let first_variant_0 = ctx["first_variant"];
  let first_description_0 = ctx["first_description"];
  let render_property_description_0 = ctx["render_property_description"];
  let render_variant_description_0 = ctx["render_variant_description"];
  let fn_0 = ctx["fn"];
  let args_0 = ctx["args"];
  let value_text_0 = ctx["value_text"];
  let model_identity_0 = ctx["model_identity"];
  let reasoning_effort_0 = ctx["reasoning_effort"];
  let messages_0 = ctx["messages"];
  let builtin_tools_0 = ctx["builtin_tools"];
  let tools_0 = ctx["tools"] ?? null;
  let tool_definition_0 = ctx["tool_definition"];
  let tool_name_0 = ctx["tool_name"];
  let tool_parameters_0 = ctx["tool_parameters"];
  let tool_properties_0 = ctx["tool_properties"];
  let bash_shape_0 = ctx["bash_shape"];
  let command_schema_0 = ctx["command_schema"];
  let is_trained_bash_0 = ctx["is_trained_bash"];
  let allow_missing_thinking_0 = ctx["allow_missing_thinking"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let thinking_text_0 = ctx["thinking_text"];
  let resolved_tool_0 = ctx["resolved_tool"];
  let call_id_0 = ctx["call_id"];
  ts_type_name_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_2 = k.caller;
    let type_val_1 = p.length > 0 ? p[0] : "type_val" in k ? k["type_val"] : undefined;
    if (J.t((J.eq(type_val_1, "integer")))) {
      out.push(J.str("number"));
    } else if (J.t((J.has(type_val_1, ["string", "number", "boolean"])))) {
      out.push(J.str(type_val_1));
    } else {
      out.push(J.str("any"));
    }
    return out.join("");
  };
  render_default_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_5 = k.caller;
    let schema_3 = p.length > 0 ? p[0] : "schema" in k ? k["schema"] : undefined;
    let bare_enum_string_4 = p.length > 1 ? p[1] : "bare_enum_string" in k ? k["bare_enum_string"] : true;
    let is_enum_6 = is_enum_0;
    is_enum_6 = ((v) => J.t(v) ? J.call(J.attr(schema_3, "get"), "enum") : v)(((v) => J.t(v) ? !J.t(J.test("string", J.call(J.attr(schema_3, "get"), "enum"))) : v)(J.test("sequence", J.call(J.attr(schema_3, "get"), "enum"))));
    if (J.t(((v) => J.t(v) ? ((v) => J.t(v) ? v : !J.t(bare_enum_string_4))(!J.t(is_enum_6)) : v)(J.test("string", J.attr(schema_3, "default"))))) {
      out.push(J.str(J.add(J.add("\"", J.attr(schema_3, "default")), "\"")));
    } else if (J.t(J.test("string", J.attr(schema_3, "default")))) {
      out.push(J.str(J.attr(schema_3, "default")));
    } else {
      out.push(J.str(J.filter("tojson", J.attr(schema_3, "default"), J.kw({"separators": [",", ":"]}))));
    }
    return out.join("");
  };
  render_nullable_type_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_9 = k.caller;
    let schema_7 = p.length > 0 ? p[0] : "schema" in k ? k["schema"] : undefined;
    let indent_8 = p.length > 1 ? p[1] : "indent" in k ? k["indent"] : undefined;
    let rendered_10 = rendered_0;
    rendered_10 = J.call(json_schema_to_typescript_0, schema_7, indent_8);
    out.push(J.str(rendered_10));
    if (J.t(((v) => J.t(v) ? (!J.has("null", rendered_10)) : v)(((v) => J.t(v) ? J.call(J.attr(schema_7, "get"), "nullable") : v)(J.test("mapping", schema_7))))) {
      out.push(J.str(" | null"));
    }
    return out.join("");
  };
  json_schema_to_typescript_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_13 = k.caller;
    let schema_11 = p.length > 0 ? p[0] : "schema" in k ? k["schema"] : undefined;
    let indent_12 = p.length > 1 ? p[1] : "indent" in k ? k["indent"] : 0;
    let rendered_enum_14 = rendered_enum_0;
    let required_15 = required_0;
    let rendered_types_16 = rendered_types_0;
    if (J.t(((v) => J.t(v) ? !J.t(J.test("string", J.call(J.attr(schema_11, "get"), "oneOf"))) : v)(((v) => J.t(v) ? J.test("sequence", J.call(J.attr(schema_11, "get"), "oneOf")) : v)(J.test("mapping", schema_11))))) {
      {
        let _items1 = J.iter(J.attr(schema_11, "oneOf"));
        const _loop1 = J.loop(_items1);
        for (let _i1 = 0; _i1 < _items1.length; _i1++) {
          _loop1.index0 = _i1; const _it1 = _items1[_i1]; let variant_17 = _it1;
          out.push(J.str(J.add(J.add("\n", J.mul(" ", indent_12)), " | ")), J.str(J.call(render_nullable_type_0, variant_17, J.add(indent_12, 3))));
          if (J.t(((v) => J.t(v) ? J.test("string", J.call(J.attr(variant_17, "get"), "description")) : v)(J.test("mapping", variant_17)))) {
            out.push(J.str(J.add(" // ", J.attr(variant_17, "description"))));
            if (J.t((J.has("default", variant_17)))) {
              out.push(J.str(J.add(" default: ", J.call(render_default_0, variant_17, false))));
            }
          } else if (J.t(((v) => J.t(v) ? (J.has("default", variant_17)) : v)(J.test("mapping", variant_17)))) {
            out.push(J.str(J.add(" // default: ", J.call(render_default_0, variant_17, false))));
          }
        }
      }
    } else if (J.t(((v) => J.t(v) ? (J.eq(J.call(J.attr(schema_11, "get"), "type"), "string")) : v)(((v) => J.t(v) ? J.call(J.attr(schema_11, "get"), "enum") : v)(((v) => J.t(v) ? !J.t(J.test("string", J.call(J.attr(schema_11, "get"), "enum"))) : v)(((v) => J.t(v) ? J.test("sequence", J.call(J.attr(schema_11, "get"), "enum")) : v)(J.test("mapping", schema_11))))))) {
      rendered_enum_14 = J.call(J.namespace, J.kw({"any": false}));
      {
        let _items2 = J.iter(J.attr(schema_11, "enum"));
        const _loop2 = J.loop(_items2);
        for (let _i2 = 0; _i2 < _items2.length; _i2++) {
          _loop2.index0 = _i2; const _it2 = _items2[_i2]; let val_18 = _it2;
          if (J.t(J.test("string", val_18))) {
            if (J.t(J.attr(rendered_enum_14, "any"))) {
              out.push(" | ");
            }
            out.push(J.str(J.filter("tojson", val_18)));
            J.setattr(rendered_enum_14, "any", true);
          }
        }
      }
      if (J.t(!J.t(J.attr(rendered_enum_14, "any")))) {
        out.push("string");
      }
    } else if (J.t(((v) => J.t(v) ? (J.eq(J.call(J.attr(schema_11, "get"), "type"), "object")) : v)(J.test("mapping", schema_11)))) {
      if (J.t(J.test("string", J.call(J.attr(schema_11, "get"), "description")))) {
        out.push(J.str(J.add(J.add(J.add(J.mul(" ", indent_12), "// "), J.attr(schema_11, "description")), "\n")));
      }
      out.push(J.str("{\n"));
      required_15 = (J.t(J.call(J.attr(schema_11, "get"), "required")) ? J.attr(schema_11, "required") : []);
      if (J.t(J.call(J.attr(schema_11, "get"), "properties"))) {
        {
          let _items3 = J.iter(J.call(J.attr(J.attr(schema_11, "properties"), "items")));
          const _loop3 = J.loop(_items3);
          for (let _i3 = 0; _i3 < _items3.length; _i3++) {
            _loop3.index0 = _i3; const _it3 = _items3[_i3]; let [prop_name_19, prop_spec_20] = J.unpack(_it3, 2);
            let property_description_21 = property_description_0;
            let first_variant_22 = first_variant_0;
            let first_description_23 = first_description_0;
            let render_property_description_24 = render_property_description_0;
            if (J.t(((v) => J.t(v) ? J.test("string", J.call(J.attr(prop_spec_20, "get"), "title")) : v)(J.test("mapping", prop_spec_20)))) {
              out.push(J.str(J.add(J.add(J.add(J.mul(" ", indent_12), "// "), J.attr(prop_spec_20, "title")), "\n")), J.str(J.add(J.mul(" ", indent_12), "//\n")));
            }
            if (J.t(((v) => J.t(v) ? J.test("string", J.call(J.attr(prop_spec_20, "get"), "description")) : v)(((v) => J.t(v) ? (!J.has("oneOf", prop_spec_20)) : v)(J.test("mapping", prop_spec_20))))) {
              out.push(J.str(J.add(J.add(J.add(J.mul(" ", indent_12), "// "), J.attr(prop_spec_20, "description")), "\n")));
            }
            if (J.t(((v) => J.t(v) ? J.call(J.attr(prop_spec_20, "get"), "examples") : v)(((v) => J.t(v) ? !J.t(J.test("string", J.call(J.attr(prop_spec_20, "get"), "examples"))) : v)(((v) => J.t(v) ? J.test("sequence", J.call(J.attr(prop_spec_20, "get"), "examples")) : v)(J.test("mapping", prop_spec_20)))))) {
              out.push(J.str(J.add(J.mul(" ", indent_12), "// Examples:\n")));
              {
                let _items4 = J.iter(J.attr(prop_spec_20, "examples"));
                const _loop4 = J.loop(_items4);
                for (let _i4 = 0; _i4 < _items4.length; _i4++) {
                  _loop4.index0 = _i4; const _it4 = _items4[_i4]; let example_25 = _it4;
                  if (J.t(J.test("string", example_25))) {
                    out.push(J.str(J.add(J.add(J.add(J.mul(" ", indent_12), "// - \""), example_25), "\"\n")));
                  }
                }
              }
            }
            if (J.t(((v) => J.t(v) ? !J.t(J.test("string", J.call(J.attr(prop_spec_20, "get"), "oneOf"))) : v)(((v) => J.t(v) ? J.test("sequence", J.call(J.attr(prop_spec_20, "get"), "oneOf")) : v)(J.test("mapping", prop_spec_20))))) {
              property_description_21 = (J.t(J.test("string", J.call(J.attr(prop_spec_20, "get"), "description"))) ? J.call(J.attr(prop_spec_20, "get"), "description") : null);
              first_variant_22 = (J.t(J.attr(prop_spec_20, "oneOf")) ? J.item(J.attr(prop_spec_20, "oneOf"), 0) : ({}));
              first_description_23 = (J.t(((v) => J.t(v) ? J.test("string", J.call(J.attr(first_variant_22, "get"), "description")) : v)(J.test("mapping", first_variant_22))) ? J.call(J.attr(first_variant_22, "get"), "description") : null);
              render_property_description_24 = ((v) => J.t(v) ? (!J.eq(property_description_21, first_description_23)) : v)(property_description_21);
              if (J.t(render_property_description_24)) {
                out.push(J.str(J.add(J.add(J.add(J.mul(" ", indent_12), "// "), property_description_21), "\n")));
              }
              if (J.t((J.has("default", prop_spec_20)))) {
                out.push(J.str(J.add(J.add(J.add(J.mul(" ", indent_12), "// default: "), J.call(render_default_0, prop_spec_20)), "\n")));
              }
              out.push(J.str(J.add(J.mul(" ", indent_12), prop_name_19)));
              if (J.t((!J.has(prop_name_19, required_15)))) {
                out.push("?");
              }
              out.push(J.str(":\n"));
              {
                let _items5 = J.iter(J.attr(prop_spec_20, "oneOf"));
                const _loop5 = J.loop(_items5);
                for (let _i5 = 0; _i5 < _items5.length; _i5++) {
                  _loop5.index0 = _i5; const _it5 = _items5[_i5]; let variant_26 = _it5;
                  let render_variant_description_27 = render_variant_description_0;
                  out.push(J.str(J.add(J.mul(" ", indent_12), " | ")), J.str(J.call(render_nullable_type_0, variant_26, J.add(indent_12, 3))));
                  render_variant_description_27 = ((v) => J.t(v) ? !J.t(((v) => J.t(v) ? render_property_description_24 : v)(J.attr(_loop5, "first"))) : v)(((v) => J.t(v) ? (!J.eq(J.attr(variant_26, "description"), property_description_21)) : v)(((v) => J.t(v) ? J.test("string", J.call(J.attr(variant_26, "get"), "description")) : v)(J.test("mapping", variant_26))));
                  if (J.t(render_variant_description_27)) {
                    out.push(J.str(J.add(" // ", J.attr(variant_26, "description"))));
                    if (J.t((J.has("default", variant_26)))) {
                      out.push(J.str(J.add(" default: ", J.call(render_default_0, variant_26))));
                    }
                  } else if (J.t(((v) => J.t(v) ? (J.has("default", variant_26)) : v)(J.test("mapping", variant_26)))) {
                    out.push(J.str(J.add(" // default: ", J.call(render_default_0, variant_26))));
                  }
                  out.push(J.str("\n"));
                }
              }
              out.push(J.str(J.add(J.mul(" ", indent_12), ",\n")));
            } else {
              out.push(J.str(J.add(J.mul(" ", indent_12), prop_name_19)));
              if (J.t((!J.has(prop_name_19, required_15)))) {
                out.push("?");
              }
              out.push(J.str(": "), J.str(J.call(render_nullable_type_0, prop_spec_20, J.add(indent_12, 4))), J.str(","));
              if (J.t(((v) => J.t(v) ? (J.has("default", prop_spec_20)) : v)(J.test("mapping", prop_spec_20)))) {
                out.push(J.str(J.add(" // default: ", J.call(render_default_0, prop_spec_20))));
              }
              out.push(J.str("\n"));
            }
          }
        }
      }
      out.push(J.str(J.add(J.mul(" ", indent_12), "}")));
    } else if (J.t(((v) => J.t(v) ? (J.eq(J.call(J.attr(schema_11, "get"), "type"), "array")) : v)(J.test("mapping", schema_11)))) {
      if (J.t(J.call(J.attr(schema_11, "get"), "items"))) {
        out.push(J.str(J.add(J.call(json_schema_to_typescript_0, J.item(schema_11, "items"), indent_12), "[]")));
      } else {
        out.push(J.str("Array<any>"));
      }
    } else if (J.t(((v) => J.t(v) ? J.test("string", J.call(J.attr(schema_11, "get"), "type")) : v)(J.test("mapping", schema_11)))) {
      out.push(J.str(J.call(ts_type_name_0, J.attr(schema_11, "type"))));
    } else if (J.t(((v) => J.t(v) ? J.test("sequence", J.call(J.attr(schema_11, "get"), "type")) : v)(J.test("mapping", schema_11)))) {
      rendered_types_16 = J.call(J.namespace, J.kw({"any": false}));
      {
        let _items6 = J.iter(J.attr(schema_11, "type"));
        const _loop6 = J.loop(_items6);
        for (let _i6 = 0; _i6 < _items6.length; _i6++) {
          _loop6.index0 = _i6; const _it6 = _items6[_i6]; let ty_28 = _it6;
          if (J.t(J.test("string", ty_28))) {
            if (J.t(J.attr(rendered_types_16, "any"))) {
              out.push(" | ");
            }
            if (J.t((J.eq(ty_28, "integer")))) {
              out.push("number");
            } else {
              out.push(J.str(ty_28));
            }
            J.setattr(rendered_types_16, "any", true);
          }
        }
      }
      if (J.t(!J.t(J.attr(rendered_types_16, "any")))) {
        out.push("any");
      }
    } else {
      out.push(J.str("any"));
    }
    return out.join("");
  };
  render_tool_declaration_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_30 = k.caller;
    let tool_29 = p.length > 0 ? p[0] : "tool" in k ? k["tool"] : undefined;
    tool_29 = (J.t((J.attr(tool_29, "function") !== undefined)) ? J.attr(tool_29, "function") : tool_29);
    if (J.t(((v) => J.t(v) ? !J.t(J.call(J.attr(J.attr(tool_29, "parameters"), "get"), "properties")) : v)(((v) => J.t(v) ? (J.eq(J.call(J.attr(J.attr(tool_29, "parameters"), "get"), "type"), "string")) : v)(J.test("mapping", J.attr(tool_29, "parameters")))))) {
      out.push(J.str(J.add(J.add("## ", J.attr(tool_29, "name")), "\n\n")), J.str(((v) => J.t(v) ? v : "")(J.attr(tool_29, "description"))));
    } else {
      if (J.t(J.attr(tool_29, "description"))) {
        {
          let _items7 = J.iter(J.call(J.attr(J.attr(tool_29, "description"), "split"), "\n"));
          const _loop7 = J.loop(_items7);
          for (let _i7 = 0; _i7 < _items7.length; _i7++) {
            _loop7.index0 = _i7; const _it7 = _items7[_i7]; let line_31 = _it7;
            out.push(J.str(J.add(J.add("// ", line_31), "\n")));
          }
        }
      }
      out.push(J.str(J.add(J.add(J.add(J.add("type ", J.attr(tool_29, "name")), " = (_: "), J.call(json_schema_to_typescript_0, J.attr(tool_29, "parameters"))), ") => any;")));
    }
    return out.join("");
  };
  render_tools_section_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_33 = k.caller;
    let tools_32 = p.length > 0 ? p[0] : "tools" in k ? k["tools"] : undefined;
    out.push(J.str("# Tools\n\n"));
    {
      let _items8 = J.iter(tools_32);
      const _loop8 = J.loop(_items8);
      for (let _i8 = 0; _i8 < _items8.length; _i8++) {
        _loop8.index0 = _i8; const _it8 = _items8[_i8]; let tool_34 = _it8;
        out.push(J.str(J.call(render_tool_declaration_0, tool_34)));
        if (J.t(!J.t(J.attr(_loop8, "last")))) {
          out.push(J.str("\n\n"));
        }
      }
    }
    return out.join("");
  };
  render_json_value_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_36 = k.caller;
    let value_35 = p.length > 0 ? p[0] : "value" in k ? k["value"] : undefined;
    if (J.t(J.test("mapping", value_35))) {
      out.push(J.str("{"));
      {
        let _items9 = J.iter(J.call(J.attr(value_35, "items")));
        const _loop9 = J.loop(_items9);
        for (let _i9 = 0; _i9 < _items9.length; _i9++) {
          _loop9.index0 = _i9; const _it9 = _items9[_i9]; let [k_37, v_38] = J.unpack(_it9, 2);
          out.push(J.str(J.filter("tojson", k_37)), ": ", J.str(J.call(render_json_value_0, v_38)));
          if (J.t(!J.t(J.attr(_loop9, "last")))) {
            out.push(", ");
          }
        }
      }
      out.push(J.str("}"));
    } else if (J.t(((v) => J.t(v) ? !J.t(J.test("string", value_35)) : v)(J.test("sequence", value_35)))) {
      out.push(J.str("["));
      {
        let _items10 = J.iter(value_35);
        const _loop10 = J.loop(_items10);
        for (let _i10 = 0; _i10 < _items10.length; _i10++) {
          _loop10.index0 = _i10; const _it10 = _items10[_i10]; let item_39 = _it10;
          out.push(J.str(J.call(render_json_value_0, item_39)));
          if (J.t(!J.t(J.attr(_loop10, "last")))) {
            out.push(", ");
          }
        }
      }
      out.push(J.str("]"));
    } else {
      out.push(J.str(J.filter("tojson", value_35)));
    }
    return out.join("");
  };
  render_parameter_value_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_41 = k.caller;
    let value_40 = p.length > 0 ? p[0] : "value" in k ? k["value"] : undefined;
    if (J.t(J.test("string", value_40))) {
      out.push(J.str(value_40));
    } else {
      out.push(J.str(J.call(render_json_value_0, value_40)));
    }
    return out.join("");
  };
  render_tool_call_body_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_43 = k.caller;
    let tool_call_42 = p.length > 0 ? p[0] : "tool_call" in k ? k["tool_call"] : undefined;
    let fn_44 = fn_0;
    let args_45 = args_0;
    fn_44 = (J.t((J.attr(tool_call_42, "function") !== undefined)) ? J.attr(tool_call_42, "function") : tool_call_42);
    args_45 = (J.t((J.attr(fn_44, "arguments") !== undefined)) ? J.attr(fn_44, "arguments") : J.attr(tool_call_42, "arguments"));
    if (J.t(J.test("mapping", args_45))) {
      {
        let _items11 = J.iter(args_45);
        const _loop11 = J.loop(_items11);
        for (let _i11 = 0; _i11 < _items11.length; _i11++) {
          _loop11.index0 = _i11; const _it11 = _items11[_i11]; let param_name_46 = _it11;
          let value_text_47 = value_text_0;
          value_text_47 = J.call(render_parameter_value_0, J.item(args_45, param_name_46));
          if (J.t((J.has("</rfl:parameter>", value_text_47)))) {
            out.push(J.str(J.call(J.raise_exception, J.add(J.add("Tool-call parameter '", J.filter("string", param_name_46)), "' contains the reserved delimiter </rfl:parameter>"))));
          }
          if (J.t(!J.t(J.attr(_loop11, "first")))) {
            out.push(J.str("\n"));
          }
          out.push(J.str(J.add(J.add("<rfl:parameter name=\"", param_name_46), "\">\n")), J.str(value_text_47), J.str("\n</rfl:parameter>"));
        }
      }
    } else if (J.t(((v) => J.t(v) ? !J.t((args_45 === null)) : v)((args_45 !== undefined)))) {
      out.push(J.str(J.call(J.raise_exception, "function.arguments must be a dict; parse JSON strings before apply_chat_template")));
    }
    return out.join("");
  };
  build_system_message_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_49 = k.caller;
    let builtin_tool_definitions_48 = p.length > 0 ? p[0] : "builtin_tool_definitions" in k ? k["builtin_tool_definitions"] : undefined;
    let model_identity_50 = model_identity_0;
    let reasoning_effort_51 = reasoning_effort_0;
    if (J.t(!J.t((model_identity_50 !== undefined)))) {
      model_identity_50 = "You are Beam, an AI assistant trained by Reflection.";
    }
    out.push(J.str(model_identity_50));
    if (J.t(((v) => J.t(v) ? v : (reasoning_effort_51 === null))(!J.t((reasoning_effort_51 !== undefined))))) {
      reasoning_effort_51 = 512;
    }
    if (J.t(J.test("string", reasoning_effort_51))) {
      if (J.t((J.eq(reasoning_effort_51, "minimal")))) {
        reasoning_effort_51 = 8;
      } else if (J.t((J.eq(reasoning_effort_51, "low")))) {
        reasoning_effort_51 = 32;
      } else if (J.t((J.eq(reasoning_effort_51, "medium")))) {
        reasoning_effort_51 = 64;
      } else if (J.t((J.eq(reasoning_effort_51, "high")))) {
        reasoning_effort_51 = 128;
      } else if (J.t((J.eq(reasoning_effort_51, "xhigh")))) {
        reasoning_effort_51 = 256;
      } else if (J.t((J.eq(reasoning_effort_51, "max")))) {
        reasoning_effort_51 = 512;
      } else {
        out.push(J.str(J.call(J.raise_exception, "reasoning_effort must be an int or one of: minimal, low, medium, high, xhigh, max")));
      }
    }
    out.push(J.str(J.add("\n\nReasoning effort: ", J.filter("string", reasoning_effort_51))));
    if (J.t(builtin_tool_definitions_48)) {
      out.push(J.str("\n\n"), J.str(J.call(render_tools_section_0, builtin_tool_definitions_48)));
    }
    out.push(J.str("\n\n# Valid channels: analysis, final. Channel must be included for every message."));
    return out.join("");
  };
  if (J.t(!J.t(messages_0))) {
    out.push(J.str(J.call(J.raise_exception, "messages cannot be empty")));
  }
  builtin_tool_names_0 = J.filter("default", builtin_tools_0, [], true);
  trained_bash_tool_0 = ({"name": "bash", "description": "Execute a bash command. The output contains the command's stdout and stderr.\nCommands time out after 10 minutes. You can set a shorter timeout within the bash command.", "parameters": ({"type": "object", "properties": ({"command": ({"type": "string", "description": "The command to execute"})}), "required": ["command"]})});
  tool_groups_0 = J.call(J.namespace, J.kw({"builtin": [], "regular": []}));
  {
    let _items12 = J.iter(J.filter("default", tools_0, [], true));
    const _loop12 = J.loop(_items12);
    for (let _i12 = 0; _i12 < _items12.length; _i12++) {
      _loop12.index0 = _i12; const _it12 = _items12[_i12]; let tool_52 = _it12;
      let tool_definition_53 = tool_definition_0;
      let tool_name_54 = tool_name_0;
      let tool_parameters_55 = tool_parameters_0;
      let tool_properties_56 = tool_properties_0;
      let bash_shape_57 = bash_shape_0;
      let command_schema_58 = command_schema_0;
      let is_trained_bash_59 = is_trained_bash_0;
      tool_definition_53 = (J.t((J.attr(tool_52, "function") !== undefined)) ? J.attr(tool_52, "function") : tool_52);
      tool_name_54 = J.attr(tool_definition_53, "name");
      tool_parameters_55 = (J.t(J.test("mapping", J.attr(tool_definition_53, "parameters"))) ? J.attr(tool_definition_53, "parameters") : ({}));
      tool_properties_56 = (J.t(J.test("mapping", J.attr(tool_parameters_55, "properties"))) ? J.attr(tool_parameters_55, "properties") : ({}));
      bash_shape_57 = J.call(J.namespace, J.kw({"plain": true}));
      {
        let _items13 = J.iter(tool_parameters_55);
        const _loop13 = J.loop(_items13);
        for (let _i13 = 0; _i13 < _items13.length; _i13++) {
          _loop13.index0 = _i13; const _it13 = _items13[_i13]; let key_60 = _it13;
          if (J.t((!J.has(key_60, ["type", "properties", "required", "additionalProperties", "description"])))) {
            J.setattr(bash_shape_57, "plain", false);
          }
        }
      }
      command_schema_58 = (J.t(J.test("mapping", J.attr(tool_properties_56, "command"))) ? J.attr(tool_properties_56, "command") : ({}));
      {
        let _items14 = J.iter(command_schema_58);
        const _loop14 = J.loop(_items14);
        for (let _i14 = 0; _i14 < _items14.length; _i14++) {
          _loop14.index0 = _i14; const _it14 = _items14[_i14]; let key_61 = _it14;
          if (J.t((!J.has(key_61, ["type", "description"])))) {
            J.setattr(bash_shape_57, "plain", false);
          }
        }
      }
      is_trained_bash_59 = ((v) => J.t(v) ? (J.eq(J.call(J.attr(tool_parameters_55, "get"), "required", ["command"]), ["command"])) : v)(((v) => J.t(v) ? (J.eq(J.call(J.attr(command_schema_58, "get"), "type"), "string")) : v)(((v) => J.t(v) ? (J.eq(J.filter("list", tool_properties_56), ["command"])) : v)(((v) => J.t(v) ? (J.eq(J.call(J.attr(tool_parameters_55, "get"), "type"), "object")) : v)(((v) => J.t(v) ? J.attr(bash_shape_57, "plain") : v)((J.eq(tool_name_54, "bash")))))));
      if (J.t(is_trained_bash_59)) {
        J.setattr(tool_groups_0, "builtin", J.add(J.attr(tool_groups_0, "builtin"), [trained_bash_tool_0]));
      } else if (J.t((J.has(tool_name_54, builtin_tool_names_0)))) {
        J.setattr(tool_groups_0, "builtin", J.add(J.attr(tool_groups_0, "builtin"), [tool_52]));
      } else {
        J.setattr(tool_groups_0, "regular", J.add(J.attr(tool_groups_0, "regular"), [tool_52]));
      }
    }
  }
  out.push(J.str("<|start|>system<|message|>"), J.str(J.call(build_system_message_0, J.attr(tool_groups_0, "builtin"))), J.str("<|end|>"));
  leading_instructions_0 = J.call(J.namespace, J.kw({"count": 0, "parts": []}));
  {
    let _items15 = J.iter(messages_0);
    const _loop15 = J.loop(_items15);
    for (let _i15 = 0; _i15 < _items15.length; _i15++) {
      _loop15.index0 = _i15; const _it15 = _items15[_i15]; let message_62 = _it15;
      if (J.t(((v) => J.t(v) ? (J.has(J.attr(message_62, "role"), ["system", "developer"])) : v)((J.eq(J.attr(_loop15, "index0"), J.attr(leading_instructions_0, "count")))))) {
        J.setattr(leading_instructions_0, "count", J.add(J.attr(leading_instructions_0, "count"), 1));
        if (J.t(J.attr(message_62, "content"))) {
          J.setattr(leading_instructions_0, "parts", J.add(J.attr(leading_instructions_0, "parts"), [J.attr(message_62, "content")]));
        }
      }
    }
  }
  developer_message_0 = J.filter("join", J.attr(leading_instructions_0, "parts"), "\n\n");
  loop_messages_0 = J.slice(messages_0, J.attr(leading_instructions_0, "count"), undefined, undefined);
  if (J.t(((v) => J.t(v) ? (J.eq(allow_missing_thinking_0, false)) : v)(((v) => J.t(v) ? (allow_missing_thinking_0 !== undefined) : v)(add_generation_prompt_0)))) {
    {
      let _items16 = J.iter(loop_messages_0);
      const _loop16 = J.loop(_items16);
      for (let _i16 = 0; _i16 < _items16.length; _i16++) {
        _loop16.index0 = _i16; const _it16 = _items16[_i16]; let message_63 = _it16;
        if (J.t(((v) => J.t(v) ? !J.t(((v) => J.t(v) ? v : J.attr(message_63, "reasoning"))(((v) => J.t(v) ? v : J.attr(message_63, "reasoning_content"))(J.attr(message_63, "thinking")))) : v)((J.eq(J.attr(message_63, "role"), "assistant"))))) {
          out.push(J.str(J.call(J.raise_exception, J.add(J.add("The assistant turn at messages[", J.filter("string", J.add(J.attr(_loop16, "index0"), J.attr(leading_instructions_0, "count")))), "] is missing its reasoning. Dropping reasoning from earlier turns degrades this model's output. Send it back in 'reasoning_content', or set allow_missing_thinking=true to skip this check."))));
        }
      }
    }
  }
  has_tools_0 = J.attr(tool_groups_0, "regular");
  if (J.t(((v) => J.t(v) ? v : has_tools_0)(developer_message_0))) {
    out.push(J.str("<|start|>developer<|message|>"));
    if (J.t(developer_message_0)) {
      out.push(J.str("# Instructions\n\n"), J.str(developer_message_0));
      if (J.t(has_tools_0)) {
        out.push(J.str("\n\n"));
      }
    }
    if (J.t(has_tools_0)) {
      out.push(J.str(J.call(render_tools_section_0, J.attr(tool_groups_0, "regular"))));
    }
    out.push(J.str("<|end|>"));
  }
  tool_call_registry_0 = J.call(J.namespace, J.kw({"entries": []}));
  {
    let _items17 = J.iter(loop_messages_0);
    const _loop17 = J.loop(_items17);
    for (let _i17 = 0; _i17 < _items17.length; _i17++) {
      _loop17.index0 = _i17; const _it17 = _items17[_i17]; let message_64 = _it17;
      let thinking_text_65 = thinking_text_0;
      let resolved_tool_66 = resolved_tool_0;
      if (J.t((J.eq(J.attr(message_64, "role"), "assistant")))) {
        if (J.t(J.attr(message_64, "content"))) {
          if (J.t(((v) => J.t(v) ? v : (J.has("<|channel|>final<|message|>", J.attr(message_64, "content"))))((J.has("<|channel|>analysis<|message|>", J.attr(message_64, "content")))))) {
            out.push(J.str(J.call(J.raise_exception, "Pass plain text in API fields, not Harmony wire tags. Use 'thinking' for analysis and 'content' for final.")));
          }
        }
        thinking_text_65 = ((v) => J.t(v) ? v : J.attr(message_64, "reasoning"))(((v) => J.t(v) ? v : J.attr(message_64, "reasoning_content"))(J.attr(message_64, "thinking")));
        if (J.t(thinking_text_65)) {
          if (J.t(((v) => J.t(v) ? v : (J.has("<|channel|>final<|message|>", thinking_text_65)))((J.has("<|channel|>analysis<|message|>", thinking_text_65))))) {
            out.push(J.str(J.call(J.raise_exception, "Pass plain text in 'thinking'/'reasoning_content'/'reasoning' without Harmony wire tags.")));
          }
        }
        out.push(J.str("<|start|>assistant"));
        {
          out.push(J.str(J.add(J.add("<|channel|>analysis<|message|>", ((v) => J.t(v) ? v : "")(thinking_text_65)), "<|end|>")));
          if (J.t(J.attr(message_64, "tool_calls"))) {
            {
              let _items18 = J.iter(J.attr(message_64, "tool_calls"));
              const _loop18 = J.loop(_items18);
              for (let _i18 = 0; _i18 < _items18.length; _i18++) {
                _loop18.index0 = _i18; const _it18 = _items18[_i18]; let tool_call_67 = _it18;
                let fn_68 = fn_0;
                let tool_name_69 = tool_name_0;
                let call_id_70 = call_id_0;
                fn_68 = (J.t((J.attr(tool_call_67, "function") !== undefined)) ? J.attr(tool_call_67, "function") : tool_call_67);
                tool_name_69 = (J.t((J.attr(fn_68, "name") !== undefined)) ? J.attr(fn_68, "name") : J.attr(tool_call_67, "name"));
                call_id_70 = (J.t((J.attr(tool_call_67, "id") !== undefined)) ? J.attr(tool_call_67, "id") : (J.t((J.attr(fn_68, "id") !== undefined)) ? J.attr(fn_68, "id") : null));
                if (J.t(((v) => J.t(v) ? tool_name_69 : v)(call_id_70))) {
                  J.setattr(tool_call_registry_0, "entries", J.add(J.attr(tool_call_registry_0, "entries"), [({"id": call_id_70, "name": tool_name_69})]));
                }
                out.push(J.str(J.add(J.add("<|start|>assistant to=", tool_name_69), "<|channel|>analysis <|constrain|>xml<|message|>")), J.str(J.call(render_tool_call_body_0, tool_call_67)));
                if (J.t(J.attr(_loop18, "last"))) {
                  out.push(J.str("<|call|>"));
                } else {
                  out.push(J.str("<|end|>"));
                }
              }
            }
          } else {
            out.push(J.str(J.add(J.add("<|start|>assistant<|channel|>final<|message|>", ((v) => J.t(v) ? v : "")(J.attr(message_64, "content"))), "<|return|>")));
          }
        }
      } else if (J.t((J.eq(J.attr(message_64, "role"), "tool")))) {
        resolved_tool_66 = J.call(J.namespace, J.kw({"name": null}));
        if (J.t(((v) => J.t(v) ? J.attr(message_64, "name") : v)((J.attr(message_64, "name") !== undefined)))) {
          J.setattr(resolved_tool_66, "name", J.attr(message_64, "name"));
        } else if (J.t((J.attr(message_64, "tool_call_id") !== undefined))) {
          {
            let _items19 = J.iter(J.attr(tool_call_registry_0, "entries"));
            const _loop19 = J.loop(_items19);
            for (let _i19 = 0; _i19 < _items19.length; _i19++) {
              _loop19.index0 = _i19; const _it19 = _items19[_i19]; let entry_71 = _it19;
              if (J.t((J.eq(J.attr(entry_71, "id"), J.attr(message_64, "tool_call_id"))))) {
                J.setattr(resolved_tool_66, "name", J.attr(entry_71, "name"));
              }
            }
          }
        }
        if (J.t((J.attr(resolved_tool_66, "name") === null))) {
          out.push(J.str(J.call(J.raise_exception, "Tool message needs 'name' or a matching prior tool_call 'id' / 'tool_call_id'.")));
        }
        out.push(J.str(J.add(J.add(J.add(J.add("<|start|>", J.attr(resolved_tool_66, "name")), " to=assistant<|channel|>analysis<|message|>"), ((v) => J.t(v) ? v : "")(J.attr(message_64, "content"))), "<|end|>")));
      } else if (J.t((J.eq(J.attr(message_64, "role"), "user")))) {
        out.push(J.str(J.add(J.add("<|start|>user<|message|>", ((v) => J.t(v) ? v : "")(J.attr(message_64, "content"))), "<|end|>")));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push("<|start|>assistant");
  }
  return out.join("");
}
