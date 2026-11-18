// Converted from the chat template of IFM/K2-Horizon-7B by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let tool_presentation_fmt_0 = ctx["tool_presentation_fmt"];
  let tool_call_fmt_0 = ctx["tool_call_fmt"];
  let RB_0 = ctx["RB"];
  let value_contains_mapping_0 = ctx["value_contains_mapping"];
  let REFS_0 = ctx["REFS"];
  let render_compact_type_name_0 = ctx["render_compact_type_name"];
  let render_compact_type_0 = ctx["render_compact_type"];
  let render_markdown_type_name_0 = ctx["render_markdown_type_name"];
  let render_markdown_type_0 = ctx["render_markdown_type"];
  let render_xml_text_0 = ctx["render_xml_text"];
  let render_python_string_0 = ctx["render_python_string"];
  let render_python_repr_0 = ctx["render_python_repr"];
  let render_xml_value_0 = ctx["render_xml_value"];
  let render_xml_enum_value_0 = ctx["render_xml_enum_value"];
  let render_xml_enum_0 = ctx["render_xml_enum"];
  let render_xml_default_attr_0 = ctx["render_xml_default_attr"];
  let render_xml_attr_0 = ctx["render_xml_attr"];
  let validate_schema_0 = ctx["validate_schema"];
  let validate_tools_0 = ctx["validate_tools"];
  let render_tools_json_0 = ctx["render_tools_json"];
  let render_xml_schema_attrs_0 = ctx["render_xml_schema_attrs"];
  let xml_schema_has_children_0 = ctx["xml_schema_has_children"];
  let render_xml_schema_node_0 = ctx["render_xml_schema_node"];
  let render_xml_pattern_property_0 = ctx["render_xml_pattern_property"];
  let render_xml_schema_children_0 = ctx["render_xml_schema_children"];
  let render_xml_param_0 = ctx["render_xml_param"];
  let render_tools_xml_0 = ctx["render_tools_xml"];
  let render_markdown_literal_0 = ctx["render_markdown_literal"];
  let render_allowed_values_0 = ctx["render_allowed_values"];
  let render_markdown_value_0 = ctx["render_markdown_value"];
  let render_markdown_detail_0 = ctx["render_markdown_detail"];
  let render_markdown_metadata_detail_0 = ctx["render_markdown_metadata_detail"];
  let render_markdown_schema_annotations_0 = ctx["render_markdown_schema_annotations"];
  let render_markdown_metadata_annotations_0 = ctx["render_markdown_metadata_annotations"];
  let render_markdown_schema_extras_0 = ctx["render_markdown_schema_extras"];
  let render_markdown_metadata_extras_0 = ctx["render_markdown_metadata_extras"];
  let markdown_schema_has_extra_0 = ctx["markdown_schema_has_extra"];
  let markdown_parameter_schema_has_details_0 = ctx["markdown_parameter_schema_has_details"];
  let render_markdown_schema_structure_0 = ctx["render_markdown_schema_structure"];
  let render_markdown_schema_details_0 = ctx["render_markdown_schema_details"];
  let render_markdown_parameter_schema_0 = ctx["render_markdown_parameter_schema"];
  let render_markdown_param_0 = ctx["render_markdown_param"];
  let render_tools_markdown_0 = ctx["render_tools_markdown"];
  let render_tool_presentation_0 = ctx["render_tool_presentation"];
  let render_call_instructions_0 = ctx["render_call_instructions"];
  let render_system_with_tools_0 = ctx["render_system_with_tools"];
  let render_argument_value_0 = ctx["render_argument_value"];
  let render_value_type_0 = ctx["render_value_type"];
  let schema_has_combinator_0 = ctx["schema_has_combinator"];
  let render_arg_type_0 = ctx["render_arg_type"];
  let render_tool_calls_block_0 = ctx["render_tool_calls_block"];
  let render_text_content_0 = ctx["render_text_content"];
  let render_tool_response_messages_0 = ctx["render_tool_response_messages"];
  let available_tools_0 = ctx["available_tools"];
  let system_content_0 = ctx["system_content"];
  let effort_0 = ctx["effort"];
  let bos_token_0 = ctx["bos_token"];
  let tool_presentation_0 = ctx["tool_presentation"];
  let tool_calling_format_0 = ctx["tool_calling_format"];
  let tool_format_0 = ctx["tool_format"];
  let tool_presentation_format_0 = ctx["tool_presentation_format"];
  let tool_call_format_0 = ctx["tool_call_format"];
  let f_0 = ctx["f"];
  let fn_0 = ctx["fn"];
  let structural_keys_0 = ctx["structural_keys"];
  let _r_0 = ctx["_r"];
  let _k_0 = ctx["_k"];
  let _r2_0 = ctx["_r2"];
  let _k2_0 = ctx["_k2"];
  let fnp_0 = ctx["fnp"];
  let fn_ret_0 = ctx["fn_ret"];
  let rendered_keys_0 = ctx["rendered_keys"];
  let found_0 = ctx["found"];
  let spec_0 = ctx["spec"];
  let _d_0 = ctx["_d"];
  let tool_call_0 = ctx["tool_call"];
  let tools_0 = ctx["tools"] ?? null;
  let messages_0 = ctx["messages"];
  let content_0 = ctx["content"];
  let thinking_content_0 = ctx["thinking_content"];
  let think_tag_0 = ctx["think_tag"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let reasoning_effort_0 = ctx["reasoning_effort"];
  out.push(J.str(bos_token_0));
  if (J.t((tool_presentation_0 !== undefined))) {
    out.push(J.str(J.call(J.raise_exception, "Unsupported argument: tool_presentation. Use tool_presentation_format with one of: json, xml, markdown.")));
  }
  if (J.t((tool_calling_format_0 !== undefined))) {
    out.push(J.str(J.call(J.raise_exception, "Unsupported argument: tool_calling_format. Use tool_call_format with one of: json, xml, xml_typed.")));
  }
  if (J.t((tool_format_0 !== undefined))) {
    out.push(J.str(J.call(J.raise_exception, "Unsupported argument: tool_format. Use tool_call_format with one of: json, xml, xml_typed.")));
  }
  tool_presentation_fmt_0 = J.filter("default", tool_presentation_format_0, "markdown");
  tool_call_fmt_0 = J.filter("default", tool_call_format_0, "xml");
  if (J.t(((v) => J.t(v) ? (!J.eq(tool_presentation_fmt_0, "markdown")) : v)(((v) => J.t(v) ? (!J.eq(tool_presentation_fmt_0, "xml")) : v)((!J.eq(tool_presentation_fmt_0, "json")))))) {
    out.push(J.str(J.call(J.raise_exception, J.concat("Unsupported tool_presentation_format: '", tool_presentation_fmt_0, "'. Supported formats: json, xml, markdown."))));
  }
  if (J.t(((v) => J.t(v) ? (!J.eq(tool_call_fmt_0, "xml_typed")) : v)(((v) => J.t(v) ? (!J.eq(tool_call_fmt_0, "xml")) : v)((!J.eq(tool_call_fmt_0, "json")))))) {
    out.push(J.str(J.call(J.raise_exception, J.concat("Unsupported tool_call_format: '", tool_call_fmt_0, "'. Supported formats: json, xml, xml_typed."))));
  }
  RB_0 = J.call(J.namespace, J.kw({"ok": true, "bad": "|"}));
  value_contains_mapping_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_2 = k.caller;
    let v_1 = p.length > 0 ? p[0] : "v" in k ? k["v"] : undefined;
    let f_3 = f_0;
    if (J.t(J.test("mapping", v_1))) {
      out.push("true");
    } else if (J.t(((v) => J.t(v) ? !J.t(J.test("string", v_1)) : v)(J.test("sequence", v_1)))) {
      f_3 = J.call(J.namespace, J.kw({"x": "false"}));
      {
        let _items1 = J.iter(v_1);
        const _loop1 = J.loop(_items1);
        for (let _i1 = 0; _i1 < _items1.length; _i1++) {
          _loop1.index0 = _i1; const _it1 = _items1[_i1]; let c_4 = _it1;
          if (J.t((J.eq(J.call(value_contains_mapping_0, c_4), "true")))) {
            J.setattr(f_3, "x", "true");
          }
        }
      }
      out.push(J.str(J.attr(f_3, "x")));
    } else {
      out.push("false");
    }
    return out.join("");
  };
  REFS_0 = J.call(J.namespace, J.kw({"defs": ({}), "seen": "|"}));
  render_compact_type_name_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_7 = k.caller;
    let type_name_5 = p.length > 0 ? p[0] : "type_name" in k ? k["type_name"] : undefined;
    let spec_6 = p.length > 1 ? p[1] : "spec" in k ? k["spec"] : undefined;
    if (J.t((J.eq(type_name_5, "array")))) {
      out.push("array[");
      if (J.t((J.has("items", spec_6)))) {
        out.push(J.str(J.call(render_compact_type_0, J.item(spec_6, "items"))));
      } else {
        out.push("any");
      }
      out.push("]");
    } else if (J.t(type_name_5)) {
      out.push(J.str(type_name_5));
    } else {
      out.push("any");
    }
    return out.join("");
  };
  render_compact_type_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_9 = k.caller;
    let spec_8 = p.length > 0 ? p[0] : "spec" in k ? k["spec"] : undefined;
    if (J.t(!J.t(J.test("mapping", spec_8)))) {
      out.push("any");
    } else if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", J.attr(spec_8, "type")), 0)) : v)(((v) => J.t(v) ? !J.t(J.test("string", J.attr(spec_8, "type"))) : v)(((v) => J.t(v) ? J.test("sequence", J.attr(spec_8, "type")) : v)((J.attr(spec_8, "type") !== undefined)))))) {
      {
        let _items2 = J.iter(J.attr(spec_8, "type"));
        const _loop2 = J.loop(_items2);
        for (let _i2 = 0; _i2 < _items2.length; _i2++) {
          _loop2.index0 = _i2; const _it2 = _items2[_i2]; let type_name_10 = _it2;
          out.push(J.str(J.call(render_compact_type_name_0, type_name_10, spec_8)));
          if (J.t(!J.t(J.attr(_loop2, "last")))) {
            out.push("|");
          }
        }
      }
    } else if (J.t(((v) => J.t(v) ? !J.t(J.test("string", J.attr(spec_8, "type"))) : v)(((v) => J.t(v) ? J.test("sequence", J.attr(spec_8, "type")) : v)((J.attr(spec_8, "type") !== undefined))))) {
      out.push("any");
    } else if (J.t(J.attr(spec_8, "type"))) {
      out.push(J.str(J.call(render_compact_type_name_0, J.attr(spec_8, "type"), spec_8)));
    } else if (J.t(J.test("string", J.item(spec_8, "$ref")))) {
      out.push(J.str(J.filter("last", J.call(J.attr(J.item(spec_8, "$ref"), "split"), "/"))));
    } else if (J.t(J.attr(spec_8, "oneOf"))) {
      out.push("oneOf[");
      {
        let _items3 = J.iter(J.attr(spec_8, "oneOf"));
        const _loop3 = J.loop(_items3);
        for (let _i3 = 0; _i3 < _items3.length; _i3++) {
          _loop3.index0 = _i3; const _it3 = _items3[_i3]; let variant_11 = _it3;
          out.push(J.str(J.call(render_compact_type_0, variant_11)));
          if (J.t(!J.t(J.attr(_loop3, "last")))) {
            out.push("|");
          }
        }
      }
      out.push("]");
    } else if (J.t(J.attr(spec_8, "anyOf"))) {
      out.push("anyOf[");
      {
        let _items4 = J.iter(J.attr(spec_8, "anyOf"));
        const _loop4 = J.loop(_items4);
        for (let _i4 = 0; _i4 < _items4.length; _i4++) {
          _loop4.index0 = _i4; const _it4 = _items4[_i4]; let variant_12 = _it4;
          out.push(J.str(J.call(render_compact_type_0, variant_12)));
          if (J.t(!J.t(J.attr(_loop4, "last")))) {
            out.push("|");
          }
        }
      }
      out.push("]");
    } else if (J.t(J.attr(spec_8, "properties"))) {
      out.push("object");
    } else if (J.t((J.has("items", spec_8)))) {
      out.push("array[", J.str(J.call(render_compact_type_0, J.item(spec_8, "items"))), "]");
    } else {
      out.push("any");
    }
    return out.join("");
  };
  render_markdown_type_name_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_15 = k.caller;
    let type_name_13 = p.length > 0 ? p[0] : "type_name" in k ? k["type_name"] : undefined;
    let spec_14 = p.length > 1 ? p[1] : "spec" in k ? k["spec"] : undefined;
    if (J.t((J.eq(type_name_13, "array")))) {
      out.push("array of ");
      if (J.t((J.has("items", spec_14)))) {
        out.push(J.str(J.call(render_markdown_type_0, J.item(spec_14, "items"))));
      } else {
        out.push("any");
      }
    } else if (J.t(type_name_13)) {
      out.push(J.str(type_name_13));
    } else {
      out.push("any");
    }
    return out.join("");
  };
  render_markdown_type_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_17 = k.caller;
    let spec_16 = p.length > 0 ? p[0] : "spec" in k ? k["spec"] : undefined;
    if (J.t(J.test("sameas", spec_16, true))) {
      out.push("True");
    } else if (J.t(J.test("sameas", spec_16, false))) {
      out.push("False");
    } else if (J.t(!J.t(J.test("mapping", spec_16)))) {
      out.push("any");
    } else if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", J.attr(spec_16, "type")), 0)) : v)(((v) => J.t(v) ? !J.t(J.test("string", J.attr(spec_16, "type"))) : v)(((v) => J.t(v) ? J.test("sequence", J.attr(spec_16, "type")) : v)((J.attr(spec_16, "type") !== undefined)))))) {
      {
        let _items5 = J.iter(J.attr(spec_16, "type"));
        const _loop5 = J.loop(_items5);
        for (let _i5 = 0; _i5 < _items5.length; _i5++) {
          _loop5.index0 = _i5; const _it5 = _items5[_i5]; let type_name_18 = _it5;
          out.push(J.str(J.call(render_markdown_type_name_0, type_name_18, spec_16)));
          if (J.t(!J.t(J.attr(_loop5, "last")))) {
            out.push(" or ");
          }
        }
      }
    } else if (J.t(((v) => J.t(v) ? !J.t(J.test("string", J.attr(spec_16, "type"))) : v)(((v) => J.t(v) ? J.test("sequence", J.attr(spec_16, "type")) : v)((J.attr(spec_16, "type") !== undefined))))) {
      out.push("any");
    } else if (J.t(J.attr(spec_16, "type"))) {
      out.push(J.str(J.call(render_markdown_type_name_0, J.attr(spec_16, "type"), spec_16)));
    } else if (J.t(J.test("string", J.item(spec_16, "$ref")))) {
      out.push(J.str(J.filter("last", J.call(J.attr(J.item(spec_16, "$ref"), "split"), "/"))));
    } else if (J.t(J.attr(spec_16, "oneOf"))) {
      out.push("oneOf[");
      {
        let _items6 = J.iter(J.attr(spec_16, "oneOf"));
        const _loop6 = J.loop(_items6);
        for (let _i6 = 0; _i6 < _items6.length; _i6++) {
          _loop6.index0 = _i6; const _it6 = _items6[_i6]; let variant_19 = _it6;
          out.push(J.str(J.call(render_markdown_type_0, variant_19)));
          if (J.t(!J.t(J.attr(_loop6, "last")))) {
            out.push(" or ");
          }
        }
      }
      out.push("]");
    } else if (J.t(J.attr(spec_16, "anyOf"))) {
      out.push("anyOf[");
      {
        let _items7 = J.iter(J.attr(spec_16, "anyOf"));
        const _loop7 = J.loop(_items7);
        for (let _i7 = 0; _i7 < _items7.length; _i7++) {
          _loop7.index0 = _i7; const _it7 = _items7[_i7]; let variant_20 = _it7;
          out.push(J.str(J.call(render_markdown_type_0, variant_20)));
          if (J.t(!J.t(J.attr(_loop7, "last")))) {
            out.push(" or ");
          }
        }
      }
      out.push("]");
    } else if (J.t(J.attr(spec_16, "properties"))) {
      out.push("object");
    } else if (J.t((J.has("items", spec_16)))) {
      out.push("array of ", J.str(J.call(render_markdown_type_0, J.item(spec_16, "items"))));
    } else {
      out.push("any");
    }
    return out.join("");
  };
  render_xml_text_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_22 = k.caller;
    let value_21 = p.length > 0 ? p[0] : "value" in k ? k["value"] : undefined;
    out.push(J.str(J.filter("join", J.call(J.attr(value_21, "split")), " ")));
    return out.join("");
  };
  render_python_string_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_24 = k.caller;
    let value_23 = p.length > 0 ? p[0] : "value" in k ? k["value"] : undefined;
    out.push("'", J.str(J.filter("replace", J.filter("replace", J.filter("join", J.call(J.attr(value_23, "split")), " "), "\\", "\\\\"), "'", "\\'")), "'");
    return out.join("");
  };
  render_python_repr_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_26 = k.caller;
    let value_25 = p.length > 0 ? p[0] : "value" in k ? k["value"] : undefined;
    if (J.t(J.test("string", value_25))) {
      out.push(J.str(J.call(render_python_string_0, value_25)));
    } else if (J.t(J.test("sameas", value_25, true))) {
      out.push("True");
    } else if (J.t(J.test("sameas", value_25, false))) {
      out.push("False");
    } else if (J.t((value_25 === null))) {
      out.push("None");
    } else if (J.t(J.test("mapping", value_25))) {
      out.push(J.str("{"));
      {
        let _items8 = J.iter(J.filter("items", value_25));
        const _loop8 = J.loop(_items8);
        for (let _i8 = 0; _i8 < _items8.length; _i8++) {
          _loop8.index0 = _i8; const _it8 = _items8[_i8]; let [key_27, child_28] = J.unpack(_it8, 2);
          out.push(J.str(J.call(render_python_repr_0, key_27)), ": ", J.str(J.call(render_python_repr_0, child_28)));
          if (J.t(!J.t(J.attr(_loop8, "last")))) {
            out.push(", ");
          }
        }
      }
      out.push(J.str("}"));
    } else if (J.t(J.test("sequence", value_25))) {
      out.push(J.str("["));
      {
        let _items9 = J.iter(value_25);
        const _loop9 = J.loop(_items9);
        for (let _i9 = 0; _i9 < _items9.length; _i9++) {
          _loop9.index0 = _i9; const _it9 = _items9[_i9]; let child_29 = _it9;
          out.push(J.str(J.call(render_python_repr_0, child_29)));
          if (J.t(!J.t(J.attr(_loop9, "last")))) {
            out.push(", ");
          }
        }
      }
      out.push(J.str("]"));
    } else {
      out.push(J.str(value_25));
    }
    return out.join("");
  };
  render_xml_value_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_31 = k.caller;
    let value_30 = p.length > 0 ? p[0] : "value" in k ? k["value"] : undefined;
    if (J.t(J.test("string", value_30))) {
      out.push(J.str(J.call(render_xml_text_0, value_30)));
    } else {
      out.push(J.str(J.call(render_python_repr_0, value_30)));
    }
    return out.join("");
  };
  render_xml_enum_value_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_33 = k.caller;
    let value_32 = p.length > 0 ? p[0] : "value" in k ? k["value"] : undefined;
    if (J.t(J.test("string", value_32))) {
      out.push("\"", J.str(J.filter("replace", J.filter("replace", value_32, "\\", "\\\\"), "\"", "\\\"")), "\"");
    } else {
      out.push("\"", J.str(J.filter("replace", J.filter("replace", J.call(render_python_repr_0, value_32), "\\", "\\\\"), "\"", "\\\"")), "\"");
    }
    return out.join("");
  };
  render_xml_enum_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_35 = k.caller;
    let values_34 = p.length > 0 ? p[0] : "values" in k ? k["values"] : undefined;
    {
      let _items10 = J.iter(values_34);
      const _loop10 = J.loop(_items10);
      for (let _i10 = 0; _i10 < _items10.length; _i10++) {
        _loop10.index0 = _i10; const _it10 = _items10[_i10]; let value_36 = _it10;
        out.push(J.str(J.call(render_xml_enum_value_0, value_36)));
        if (J.t(!J.t(J.attr(_loop10, "last")))) {
          out.push("|");
        }
      }
    }
    return out.join("");
  };
  render_xml_default_attr_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_38 = k.caller;
    let value_37 = p.length > 0 ? p[0] : "value" in k ? k["value"] : undefined;
    out.push(J.str(" default="));
    if (J.t(J.test("string", value_37))) {
      out.push("\"", J.str(J.filter("replace", J.filter("replace", value_37, "\\", "\\\\"), "\"", "\\\"")), "\"");
    } else {
      out.push(J.str(J.call(render_xml_value_0, value_37)));
    }
    return out.join("");
  };
  render_xml_attr_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_41 = k.caller;
    let name_39 = p.length > 0 ? p[0] : "name" in k ? k["name"] : undefined;
    let value_40 = p.length > 1 ? p[1] : "value" in k ? k["value"] : undefined;
    out.push(J.str(J.add(J.add(" ", name_39), "=")));
    if (J.t((J.eq(value_40, "")))) {
      out.push("\"\"");
    } else {
      out.push(J.str(J.call(render_xml_value_0, value_40)));
    }
    return out.join("");
  };
  validate_schema_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_47 = k.caller;
    let spec_42 = p.length > 0 ? p[0] : "spec" in k ? k["spec"] : undefined;
    let path_43 = p.length > 1 ? p[1] : "path" in k ? k["path"] : undefined;
    let lenient_44 = p.length > 2 ? p[2] : "lenient" in k ? k["lenient"] : false;
    let classify_45 = p.length > 3 ? p[3] : "classify" in k ? k["classify"] : true;
    let in_variant_46 = p.length > 4 ? p[4] : "in_variant" in k ? k["in_variant"] : false;
    if (J.t(J.test("mapping", spec_42))) {
      if (J.t(!J.t(lenient_44))) {
        if (J.t((J.attr(spec_42, "required") !== undefined))) {
          if (J.t(((v) => J.t(v) ? v : !J.t(J.test("sequence", J.attr(spec_42, "required"))))(J.test("string", J.attr(spec_42, "required"))))) {
            out.push(J.str(J.call(J.raise_exception, J.add(J.add("Schema '", path_43), "' has 'required' but it is not a list."))));
          }
          if (J.t(((v) => J.t(v) ? !J.t(in_variant_46) : v)(((v) => J.t(v) ? !J.t(J.attr(spec_42, "properties")) : v)((J.gt(J.filter("length", J.attr(spec_42, "required")), 0)))))) {
            out.push(J.str(J.call(J.raise_exception, J.add(J.add("Schema '", path_43), "' has required fields but no properties object to define them."))));
          }
          if (J.t(J.attr(spec_42, "properties"))) {
            {
              let _items11 = J.iter(J.attr(spec_42, "required"));
              const _loop11 = J.loop(_items11);
              for (let _i11 = 0; _i11 < _items11.length; _i11++) {
                _loop11.index0 = _i11; const _it11 = _items11[_i11]; let required_name_48 = _it11;
                if (J.t((!J.has(required_name_48, J.attr(spec_42, "properties"))))) {
                  out.push(J.str(J.call(J.raise_exception, J.add(J.add(J.add(J.add("Schema '", path_43), "' marks '"), required_name_48), "' as required, but that property is not defined in properties."))));
                }
              }
            }
          }
        }
      }
      if (J.t(classify_45)) {
        {
          let _items12 = J.iter(J.filter("items", spec_42));
          const _loop12 = J.loop(_items12);
          for (let _i12 = 0; _i12 < _items12.length; _i12++) {
            _loop12.index0 = _i12; const _it12 = _items12[_i12]; let [key_49, value_50] = J.unpack(_it12, 2);
            if (J.t((J.eq(key_49, "$ref")))) {
              if (J.t(!J.t(J.test("string", value_50)))) {
                J.setattr(RB_0, "ok", false);
              } else if (J.t(!J.t(((v) => J.t(v) ? v : J.call(J.attr(value_50, "startswith"), "#/definitions/"))(J.call(J.attr(value_50, "startswith"), "#/$defs/"))))) {
                J.setattr(RB_0, "ok", false);
              }
            } else if (J.t(((v) => J.t(v) ? v : (J.eq(key_49, "definitions")))((J.eq(key_49, "$defs"))))) {
              if (J.t(J.test("mapping", value_50))) {
                {
                  let _items13 = J.iter(J.filter("items", value_50));
                  const _loop13 = J.loop(_items13);
                  for (let _i13 = 0; _i13 < _items13.length; _i13++) {
                    _loop13.index0 = _i13; const _it13 = _items13[_i13]; let [dk_51, dv_52] = J.unpack(_it13, 2);
                    out.push(J.str(J.call(validate_schema_0, dv_52, J.add(J.add(path_43, ".$defs."), dk_51), true)));
                  }
                }
              } else {
                J.setattr(RB_0, "ok", false);
              }
            } else if (J.t((J.eq(key_49, "type")))) {
              if (J.t(J.test("mapping", value_50))) {
                J.setattr(RB_0, "ok", false);
              }
            } else if (J.t((J.eq(key_49, "enum")))) {
              if (J.t(((v) => J.t(v) ? v : !J.t(J.test("sequence", value_50)))(((v) => J.t(v) ? v : J.test("mapping", value_50))(J.test("string", value_50))))) {
                J.setattr(RB_0, "ok", false);
              }
            } else if (J.t((J.eq(key_49, "items")))) {
            } else if (J.t(((v) => J.t(v) ? v : (J.eq(key_49, "anyOf")))((J.eq(key_49, "oneOf"))))) {
              if (J.t(((v) => J.t(v) ? v : !J.t(J.test("sequence", value_50)))(((v) => J.t(v) ? v : J.test("string", value_50))(J.test("mapping", value_50))))) {
                J.setattr(RB_0, "ok", false);
              }
            } else if (J.t((J.eq(key_49, "required")))) {
              if (J.t(((v) => J.t(v) ? !J.t(J.attr(spec_42, "properties")) : v)(value_50))) {
                J.setattr(RB_0, "ok", false);
              }
            } else if (J.t((J.has(J.concat("|", key_49, "|"), "|description|default|title|examples|properties|patternProperties|additionalProperties|returns|")))) {
            } else if (J.t(J.test("mapping", value_50))) {
              {
                let _items14 = J.iter(J.filter("items", value_50));
                const _loop14 = J.loop(_items14);
                for (let _i14 = 0; _i14 < _items14.length; _i14++) {
                  _loop14.index0 = _i14; const _it14 = _items14[_i14]; let [uk_53, uv_54] = J.unpack(_it14, 2);
                  if (J.t((J.eq(J.call(value_contains_mapping_0, uv_54), "true")))) {
                    J.setattr(RB_0, "ok", false);
                  }
                }
              }
            } else if (J.t(((v) => J.t(v) ? !J.t(J.test("string", value_50)) : v)(J.test("sequence", value_50)))) {
              if (J.t((J.eq(J.call(value_contains_mapping_0, value_50), "true")))) {
                J.setattr(RB_0, "ok", false);
              }
            }
          }
        }
      }
      if (J.t(J.attr(spec_42, "properties"))) {
        {
          let _items15 = J.iter(J.filter("items", J.attr(spec_42, "properties")));
          const _loop15 = J.loop(_items15);
          for (let _i15 = 0; _i15 < _items15.length; _i15++) {
            _loop15.index0 = _i15; const _it15 = _items15[_i15]; let [child_name_55, child_spec_56] = J.unpack(_it15, 2);
            out.push(J.str(J.call(validate_schema_0, child_spec_56, J.add(J.add(path_43, "."), child_name_55), lenient_44, classify_45)));
          }
        }
      }
      if (J.t((J.has("items", spec_42)))) {
        out.push(J.str(J.call(validate_schema_0, J.item(spec_42, "items"), J.add(path_43, "[]"), lenient_44, classify_45)));
      }
      if (J.t(J.attr(spec_42, "oneOf"))) {
        {
          let _items16 = J.iter(J.attr(spec_42, "oneOf"));
          const _loop16 = J.loop(_items16);
          for (let _i16 = 0; _i16 < _items16.length; _i16++) {
            _loop16.index0 = _i16; const _it16 = _items16[_i16]; let variant_57 = _it16;
            out.push(J.str(J.call(validate_schema_0, variant_57, J.add(J.add(J.add(path_43, ".oneOf["), J.filter("string", J.attr(_loop16, "index0"))), "]"), lenient_44, classify_45, true)));
          }
        }
      }
      if (J.t(J.attr(spec_42, "anyOf"))) {
        {
          let _items17 = J.iter(J.attr(spec_42, "anyOf"));
          const _loop17 = J.loop(_items17);
          for (let _i17 = 0; _i17 < _items17.length; _i17++) {
            _loop17.index0 = _i17; const _it17 = _items17[_i17]; let variant_58 = _it17;
            out.push(J.str(J.call(validate_schema_0, variant_58, J.add(J.add(J.add(path_43, ".anyOf["), J.filter("string", J.attr(_loop17, "index0"))), "]"), lenient_44, classify_45, true)));
          }
        }
      }
      if (J.t(J.test("mapping", J.attr(spec_42, "additionalProperties")))) {
        out.push(J.str(J.call(validate_schema_0, J.attr(spec_42, "additionalProperties"), J.add(path_43, ".additionalProperties"), lenient_44, classify_45)));
      }
      if (J.t(J.test("mapping", J.attr(spec_42, "patternProperties")))) {
        {
          let _items18 = J.iter(J.filter("items", J.attr(spec_42, "patternProperties")));
          const _loop18 = J.loop(_items18);
          for (let _i18 = 0; _i18 < _items18.length; _i18++) {
            _loop18.index0 = _i18; const _it18 = _items18[_i18]; let [pattern_59, pattern_spec_60] = J.unpack(_it18, 2);
            out.push(J.str(J.call(validate_schema_0, pattern_spec_60, J.add(J.add(J.add(path_43, ".patternProperties["), pattern_59), "]"), lenient_44, classify_45)));
          }
        }
      }
      if (J.t(J.test("mapping", J.attr(spec_42, "returns")))) {
        out.push(J.str(J.call(validate_schema_0, J.attr(spec_42, "returns"), J.add(path_43, ".returns"), lenient_44, classify_45)));
      }
    }
    return out.join("");
  };
  validate_tools_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_63 = k.caller;
    let tools_list_61 = p.length > 0 ? p[0] : "tools_list" in k ? k["tools_list"] : undefined;
    let classify_62 = p.length > 1 ? p[1] : "classify" in k ? k["classify"] : true;
    J.setattr(RB_0, "bad", "|");
    {
      let _items19 = J.iter(tools_list_61);
      const _loop19 = J.loop(_items19);
      for (let _i19 = 0; _i19 < _items19.length; _i19++) {
        _loop19.index0 = _i19; const _it19 = _items19[_i19]; let tool_64 = _it19;
        let fn_65 = fn_0;
        fn_65 = (J.t((J.attr(tool_64, "function") !== undefined)) ? J.attr(tool_64, "function") : tool_64);
        J.setattr(RB_0, "ok", true);
        if (J.t(((v) => J.t(v) ? J.test("string", J.attr(fn_65, "parameters")) : v)((J.attr(fn_65, "parameters") !== undefined)))) {
          out.push(J.str(J.call(J.raise_exception, "tool.function.parameters must be a dict, not a JSON string. Parse it before passing to the template.")));
        }
        if (J.t(((v) => J.t(v) ? v : (J.attr(fn_65, "parameters") === null))(!J.t((J.attr(fn_65, "parameters") !== undefined))))) {
          if (J.t((J.attr(fn_65, "arguments") !== undefined))) {
            out.push(J.str(J.call(J.raise_exception, J.add(J.add("Tool '", J.attr(fn_65, "name")), "' has 'arguments' instead of 'parameters'. Rename 'arguments' to 'parameters'."))));
          } else {
            out.push(J.str(J.call(J.raise_exception, J.add(J.add("Tool '", J.attr(fn_65, "name")), "' is missing required 'parameters' field. Each tool must have a 'parameters' dict with 'type', 'properties', and 'required' keys."))));
          }
        }
        out.push(J.str(J.call(validate_schema_0, J.attr(fn_65, "parameters"), J.add(J.add("tool.", J.attr(fn_65, "name")), ".parameters"), false, classify_62)));
        if (J.t(classify_62)) {
          if (J.t(J.test("mapping", J.attr(fn_65, "parameters")))) {
            {
              let _items20 = J.iter(J.filter("items", J.attr(fn_65, "parameters")));
              const _loop20 = J.loop(_items20);
              for (let _i20 = 0; _i20 < _items20.length; _i20++) {
                _loop20.index0 = _i20; const _it20 = _items20[_i20]; let [rk_66, rv_67] = J.unpack(_it20, 2);
                if (J.t((!J.has(rk_66, ["type", "description", "enum", "default", "properties", "required", "optional", "title", "items", "oneOf", "anyOf", "additionalProperties", "patternProperties", "returns", "examples", "$defs", "definitions", "$ref"])))) {
                  if (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? !J.t(J.test("string", rv_67)) : v)(J.test("sequence", rv_67)))(J.test("mapping", rv_67)))) {
                    J.setattr(RB_0, "ok", false);
                  }
                }
              }
            }
          } else {
            J.setattr(RB_0, "ok", false);
          }
        }
        if (J.t(J.test("mapping", J.attr(fn_65, "returns")))) {
          out.push(J.str(J.call(validate_schema_0, J.attr(fn_65, "returns"), J.add(J.add("tool.", J.attr(fn_65, "name")), ".returns"), false, classify_62)));
        }
        if (J.t(((v) => J.t(v) ? J.test("mapping", J.attr(fn_65, "response")) : v)(((v) => J.t(v) ? !J.t((J.attr(fn_65, "returns") !== undefined)) : v)(classify_62)))) {
          out.push(J.str(J.call(validate_schema_0, J.attr(fn_65, "response"), J.add(J.add("tool.", J.attr(fn_65, "name")), ".response"), true)));
        }
        if (J.t(classify_62)) {
          {
            let _items21 = J.iter(J.filter("items", fn_65));
            const _loop21 = J.loop(_items21);
            for (let _i21 = 0; _i21 < _items21.length; _i21++) {
              _loop21.index0 = _i21; const _it21 = _items21[_i21]; let [fk_68, fv_69] = J.unpack(_it21, 2);
              if (J.t((!J.has(fk_68, ["name", "description", "parameters", "returns", "response", "type", "function"])))) {
                if (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? !J.t(J.test("string", fv_69)) : v)(J.test("sequence", fv_69)))(J.test("mapping", fv_69)))) {
                  J.setattr(RB_0, "ok", false);
                }
              }
            }
          }
        }
        if (J.t(!J.t(J.attr(RB_0, "ok")))) {
          J.setattr(RB_0, "bad", J.concat(J.attr(RB_0, "bad"), J.attr(_loop19, "index0"), "|"));
        }
      }
    }
    return out.join("");
  };
  render_tools_json_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_71 = k.caller;
    let tools_list_70 = p.length > 0 ? p[0] : "tools_list" in k ? k["tools_list"] : undefined;
    out.push(J.str("<ifm|tools>"));
    {
      let _items22 = J.iter(tools_list_70);
      const _loop22 = J.loop(_items22);
      for (let _i22 = 0; _i22 < _items22.length; _i22++) {
        _loop22.index0 = _i22; const _it22 = _items22[_i22]; let tool_72 = _it22;
        out.push(J.str("\n"), J.str(J.filter("tojson", tool_72)));
      }
    }
    out.push(J.str("\n</ifm|tools>"));
    return out.join("");
  };
  render_xml_schema_attrs_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_75 = k.caller;
    let spec_73 = p.length > 0 ? p[0] : "spec" in k ? k["spec"] : undefined;
    let include_value_attrs_74 = p.length > 1 ? p[1] : "include_value_attrs" in k ? k["include_value_attrs"] : undefined;
    let structural_keys_76 = structural_keys_0;
    if (J.t(J.test("mapping", spec_73))) {
      structural_keys_76 = ["type", "description", "enum", "default", "properties", "required", "items", "oneOf", "anyOf", "additionalProperties", "patternProperties", "returns"];
      if (J.t(((v) => J.t(v) ? J.attr(spec_73, "enum") : v)(include_value_attrs_74))) {
        out.push(J.str(" enum="), J.str(J.call(render_xml_enum_0, J.attr(spec_73, "enum"))));
      }
      if (J.t(((v) => J.t(v) ? (J.attr(spec_73, "default") !== undefined) : v)(include_value_attrs_74))) {
        out.push(J.str(J.call(render_xml_default_attr_0, J.attr(spec_73, "default"))));
      }
      if (J.t(((v) => J.t(v) ? !J.t(J.test("mapping", J.attr(spec_73, "additionalProperties"))) : v)((J.attr(spec_73, "additionalProperties") !== undefined)))) {
        out.push(J.str(J.call(render_xml_attr_0, "additionalProperties", J.attr(spec_73, "additionalProperties"))));
      }
      if (J.t(((v) => J.t(v) ? !J.t(J.test("mapping", J.attr(spec_73, "patternProperties"))) : v)((J.attr(spec_73, "patternProperties") !== undefined)))) {
        out.push(J.str(J.call(render_xml_attr_0, "patternProperties", J.attr(spec_73, "patternProperties"))));
      }
      {
        let _items23 = J.iter(J.filter("items", spec_73));
        const _loop23 = J.loop(_items23);
        for (let _i23 = 0; _i23 < _items23.length; _i23++) {
          _loop23.index0 = _i23; const _it23 = _items23[_i23]; let [key_77, value_78] = J.unpack(_it23, 2);
          if (J.t((!J.has(key_77, structural_keys_76)))) {
            out.push(J.str(J.call(render_xml_attr_0, key_77, value_78)));
          }
        }
      }
    }
    return out.join("");
  };
  xml_schema_has_children_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_82 = k.caller;
    let spec_79 = p.length > 0 ? p[0] : "spec" in k ? k["spec"] : undefined;
    let include_properties_80 = p.length > 1 ? p[1] : "include_properties" in k ? k["include_properties"] : undefined;
    let include_description_81 = p.length > 2 ? p[2] : "include_description" in k ? k["include_description"] : undefined;
    if (J.t(!J.t(J.test("mapping", spec_79)))) {
      out.push("false");
    } else if (J.t(((v) => J.t(v) ? v : (J.attr(spec_79, "returns") !== undefined))(((v) => J.t(v) ? v : J.test("mapping", J.attr(spec_79, "patternProperties")))(((v) => J.t(v) ? v : J.test("mapping", J.attr(spec_79, "additionalProperties")))(((v) => J.t(v) ? v : J.attr(spec_79, "anyOf"))(((v) => J.t(v) ? v : J.attr(spec_79, "oneOf"))(((v) => J.t(v) ? v : (J.has("items", spec_79)))(((v) => J.t(v) ? v : ((v) => J.t(v) ? J.attr(spec_79, "properties") : v)(include_properties_80))(((v) => J.t(v) ? (J.attr(spec_79, "description") !== undefined) : v)(include_description_81)))))))))) {
      out.push("true");
    } else {
      out.push("false");
    }
    return out.join("");
  };
  render_xml_schema_node_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_86 = k.caller;
    let tag_83 = p.length > 0 ? p[0] : "tag" in k ? k["tag"] : undefined;
    let spec_84 = p.length > 1 ? p[1] : "spec" in k ? k["spec"] : undefined;
    let include_properties_85 = p.length > 2 ? p[2] : "include_properties" in k ? k["include_properties"] : undefined;
    let _r_87 = _r_0;
    let _k_88 = _k_0;
    let _r2_89 = _r2_0;
    let _k2_90 = _k2_0;
    if (J.t(((v) => J.t(v) ? J.test("string", J.item(spec_84, "$ref")) : v)(J.test("mapping", spec_84)))) {
      _r_87 = J.item(spec_84, "$ref");
      _k_88 = (J.t(J.call(J.attr(_r_87, "startswith"), "#/$defs/")) ? J.slice(_r_87, 8, undefined, undefined) : (J.t(J.call(J.attr(_r_87, "startswith"), "#/definitions/")) ? J.slice(_r_87, 14, undefined, undefined) : null));
      if (J.t(((v) => J.t(v) ? J.test("mapping", J.item(J.attr(REFS_0, "defs"), _k_88)) : v)(((v) => J.t(v) ? (!J.has(J.add(J.add("|", _k_88), "|"), J.attr(REFS_0, "seen"))) : v)(!J.t((_k_88 === null)))))) {
        spec_84 = J.call(J.dict, J.add(J.filter("list", J.filter("items", J.item(J.attr(REFS_0, "defs"), _k_88))), J.filter("list", J.filter("rejectattr", J.filter("items", spec_84), "0", "equalto", "$ref"))));
        J.setattr(REFS_0, "seen", J.add(J.add(J.attr(REFS_0, "seen"), _k_88), "|"));
        if (J.t(J.test("string", J.item(spec_84, "$ref")))) {
          _r2_89 = J.item(spec_84, "$ref");
          _k2_90 = (J.t(J.call(J.attr(_r2_89, "startswith"), "#/$defs/")) ? J.slice(_r2_89, 8, undefined, undefined) : (J.t(J.call(J.attr(_r2_89, "startswith"), "#/definitions/")) ? J.slice(_r2_89, 14, undefined, undefined) : null));
          if (J.t(((v) => J.t(v) ? J.test("mapping", J.item(J.attr(REFS_0, "defs"), _k2_90)) : v)(!J.t((_k2_90 === null))))) {
            spec_84 = J.call(J.dict, J.add(J.filter("list", J.filter("items", J.item(J.attr(REFS_0, "defs"), _k2_90))), J.filter("list", J.filter("rejectattr", J.filter("items", spec_84), "0", "equalto", "$ref"))));
            J.setattr(REFS_0, "seen", J.add(J.add(J.attr(REFS_0, "seen"), _k2_90), "|"));
          }
        }
      }
    }
    if (J.t(J.test("mapping", spec_84))) {
      out.push(J.str(J.add(J.add(J.add("<", tag_83), " type="), J.call(render_compact_type_0, spec_84))), J.str(J.call(render_xml_schema_attrs_0, spec_84, true)));
      if (J.t((J.eq(J.call(xml_schema_has_children_0, spec_84, include_properties_85, true), "true")))) {
        out.push(J.str(">"), J.str(J.call(render_xml_schema_children_0, spec_84, include_properties_85, true)), J.str(J.add(J.add("</", tag_83), ">")));
      } else {
        out.push(J.str("/>"));
      }
    } else {
      out.push(J.str(J.add(J.add("<", tag_83), ">")), J.str(J.call(render_xml_value_0, spec_84)), J.str(J.add(J.add("</", tag_83), ">")));
    }
    return out.join("");
  };
  render_xml_pattern_property_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_93 = k.caller;
    let pattern_91 = p.length > 0 ? p[0] : "pattern" in k ? k["pattern"] : undefined;
    let spec_92 = p.length > 1 ? p[1] : "spec" in k ? k["spec"] : undefined;
    let _r_94 = _r_0;
    let _k_95 = _k_0;
    let _r2_96 = _r2_0;
    let _k2_97 = _k2_0;
    if (J.t(((v) => J.t(v) ? J.test("string", J.item(spec_92, "$ref")) : v)(J.test("mapping", spec_92)))) {
      _r_94 = J.item(spec_92, "$ref");
      _k_95 = (J.t(J.call(J.attr(_r_94, "startswith"), "#/$defs/")) ? J.slice(_r_94, 8, undefined, undefined) : (J.t(J.call(J.attr(_r_94, "startswith"), "#/definitions/")) ? J.slice(_r_94, 14, undefined, undefined) : null));
      if (J.t(((v) => J.t(v) ? J.test("mapping", J.item(J.attr(REFS_0, "defs"), _k_95)) : v)(((v) => J.t(v) ? (!J.has(J.add(J.add("|", _k_95), "|"), J.attr(REFS_0, "seen"))) : v)(!J.t((_k_95 === null)))))) {
        spec_92 = J.call(J.dict, J.add(J.filter("list", J.filter("items", J.item(J.attr(REFS_0, "defs"), _k_95))), J.filter("list", J.filter("rejectattr", J.filter("items", spec_92), "0", "equalto", "$ref"))));
        J.setattr(REFS_0, "seen", J.add(J.add(J.attr(REFS_0, "seen"), _k_95), "|"));
        if (J.t(J.test("string", J.item(spec_92, "$ref")))) {
          _r2_96 = J.item(spec_92, "$ref");
          _k2_97 = (J.t(J.call(J.attr(_r2_96, "startswith"), "#/$defs/")) ? J.slice(_r2_96, 8, undefined, undefined) : (J.t(J.call(J.attr(_r2_96, "startswith"), "#/definitions/")) ? J.slice(_r2_96, 14, undefined, undefined) : null));
          if (J.t(((v) => J.t(v) ? J.test("mapping", J.item(J.attr(REFS_0, "defs"), _k2_97)) : v)(!J.t((_k2_97 === null))))) {
            spec_92 = J.call(J.dict, J.add(J.filter("list", J.filter("items", J.item(J.attr(REFS_0, "defs"), _k2_97))), J.filter("list", J.filter("rejectattr", J.filter("items", spec_92), "0", "equalto", "$ref"))));
            J.setattr(REFS_0, "seen", J.add(J.add(J.attr(REFS_0, "seen"), _k2_97), "|"));
          }
        }
      }
    }
    if (J.t(J.test("mapping", spec_92))) {
      out.push(J.str("<patternProperty"), J.str(J.call(render_xml_attr_0, "pattern", pattern_91)), J.str(J.add(" type=", J.call(render_compact_type_0, spec_92))), J.str(J.call(render_xml_schema_attrs_0, spec_92, true)));
      if (J.t((J.eq(J.call(xml_schema_has_children_0, spec_92, true, true), "true")))) {
        out.push(J.str(">"), J.str(J.call(render_xml_schema_children_0, spec_92, true, true)), J.str("</patternProperty>"));
      } else {
        out.push(J.str("/>"));
      }
    } else {
      out.push(J.str("<patternProperty"), J.str(J.call(render_xml_attr_0, "pattern", pattern_91)), J.str(">"), J.str(J.call(render_xml_value_0, spec_92)), J.str("</patternProperty>"));
    }
    return out.join("");
  };
  render_xml_schema_children_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_101 = k.caller;
    let spec_98 = p.length > 0 ? p[0] : "spec" in k ? k["spec"] : undefined;
    let include_properties_99 = p.length > 1 ? p[1] : "include_properties" in k ? k["include_properties"] : undefined;
    let include_description_100 = p.length > 2 ? p[2] : "include_description" in k ? k["include_description"] : undefined;
    if (J.t(((v) => J.t(v) ? (J.attr(spec_98, "description") !== undefined) : v)(include_description_100))) {
      out.push(J.str("<description>"), J.str(J.attr(spec_98, "description")), J.str("</description>"));
    }
    if (J.t(((v) => J.t(v) ? J.attr(spec_98, "properties") : v)(include_properties_99))) {
      {
        let _items24 = J.iter(J.filter("items", J.attr(spec_98, "properties")));
        const _loop24 = J.loop(_items24);
        for (let _i24 = 0; _i24 < _items24.length; _i24++) {
          _loop24.index0 = _i24; const _it24 = _items24[_i24]; let [child_name_102, child_spec_103] = J.unpack(_it24, 2);
          out.push(J.str(J.call(render_xml_param_0, child_name_102, child_spec_103, ((v) => J.t(v) ? v : [])(J.attr(spec_98, "required")))));
        }
      }
    }
    if (J.t((J.has("items", spec_98)))) {
      out.push(J.str(J.call(render_xml_schema_node_0, "items", J.item(spec_98, "items"), true)));
    }
    if (J.t(J.attr(spec_98, "oneOf"))) {
      out.push(J.str("<oneOf>"));
      {
        let _items25 = J.iter(J.attr(spec_98, "oneOf"));
        const _loop25 = J.loop(_items25);
        for (let _i25 = 0; _i25 < _items25.length; _i25++) {
          _loop25.index0 = _i25; const _it25 = _items25[_i25]; let variant_104 = _it25;
          out.push(J.str(J.call(render_xml_schema_node_0, "variant", variant_104, true)));
        }
      }
      out.push(J.str("</oneOf>"));
    }
    if (J.t(J.attr(spec_98, "anyOf"))) {
      out.push(J.str("<anyOf>"));
      {
        let _items26 = J.iter(J.attr(spec_98, "anyOf"));
        const _loop26 = J.loop(_items26);
        for (let _i26 = 0; _i26 < _items26.length; _i26++) {
          _loop26.index0 = _i26; const _it26 = _items26[_i26]; let variant_105 = _it26;
          out.push(J.str(J.call(render_xml_schema_node_0, "variant", variant_105, true)));
        }
      }
      out.push(J.str("</anyOf>"));
    }
    if (J.t(J.test("mapping", J.attr(spec_98, "additionalProperties")))) {
      out.push(J.str(J.call(render_xml_schema_node_0, "additionalProperties", J.attr(spec_98, "additionalProperties"), true)));
    }
    if (J.t(J.test("mapping", J.attr(spec_98, "patternProperties")))) {
      out.push(J.str("<patternProperties>"));
      {
        let _items27 = J.iter(J.filter("items", J.attr(spec_98, "patternProperties")));
        const _loop27 = J.loop(_items27);
        for (let _i27 = 0; _i27 < _items27.length; _i27++) {
          _loop27.index0 = _i27; const _it27 = _items27[_i27]; let [pattern_106, pattern_spec_107] = J.unpack(_it27, 2);
          out.push(J.str(J.call(render_xml_pattern_property_0, pattern_106, pattern_spec_107)));
        }
      }
      out.push(J.str("</patternProperties>"));
    } else if (J.t((J.attr(spec_98, "patternProperties") !== undefined))) {
      out.push("<patternProperties>", J.str(J.call(render_xml_value_0, J.attr(spec_98, "patternProperties"))), "</patternProperties>");
    }
    if (J.t(J.test("mapping", J.attr(spec_98, "returns")))) {
      out.push(J.str(J.call(render_xml_schema_node_0, "returns", J.attr(spec_98, "returns"), true)));
    } else if (J.t((J.attr(spec_98, "returns") !== undefined))) {
      out.push("<returns>", J.str(J.call(render_xml_value_0, J.attr(spec_98, "returns"))), "</returns>");
    }
    return out.join("");
  };
  render_xml_param_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_111 = k.caller;
    let name_108 = p.length > 0 ? p[0] : "name" in k ? k["name"] : undefined;
    let spec_109 = p.length > 1 ? p[1] : "spec" in k ? k["spec"] : undefined;
    let required_list_110 = p.length > 2 ? p[2] : "required_list" in k ? k["required_list"] : undefined;
    let _r_112 = _r_0;
    let _k_113 = _k_0;
    let _r2_114 = _r2_0;
    let _k2_115 = _k2_0;
    if (J.t(((v) => J.t(v) ? J.test("string", J.item(spec_109, "$ref")) : v)(J.test("mapping", spec_109)))) {
      _r_112 = J.item(spec_109, "$ref");
      _k_113 = (J.t(J.call(J.attr(_r_112, "startswith"), "#/$defs/")) ? J.slice(_r_112, 8, undefined, undefined) : (J.t(J.call(J.attr(_r_112, "startswith"), "#/definitions/")) ? J.slice(_r_112, 14, undefined, undefined) : null));
      if (J.t(((v) => J.t(v) ? J.test("mapping", J.item(J.attr(REFS_0, "defs"), _k_113)) : v)(((v) => J.t(v) ? (!J.has(J.add(J.add("|", _k_113), "|"), J.attr(REFS_0, "seen"))) : v)(!J.t((_k_113 === null)))))) {
        spec_109 = J.call(J.dict, J.add(J.filter("list", J.filter("items", J.item(J.attr(REFS_0, "defs"), _k_113))), J.filter("list", J.filter("rejectattr", J.filter("items", spec_109), "0", "equalto", "$ref"))));
        J.setattr(REFS_0, "seen", J.add(J.add(J.attr(REFS_0, "seen"), _k_113), "|"));
        if (J.t(J.test("string", J.item(spec_109, "$ref")))) {
          _r2_114 = J.item(spec_109, "$ref");
          _k2_115 = (J.t(J.call(J.attr(_r2_114, "startswith"), "#/$defs/")) ? J.slice(_r2_114, 8, undefined, undefined) : (J.t(J.call(J.attr(_r2_114, "startswith"), "#/definitions/")) ? J.slice(_r2_114, 14, undefined, undefined) : null));
          if (J.t(((v) => J.t(v) ? J.test("mapping", J.item(J.attr(REFS_0, "defs"), _k2_115)) : v)(!J.t((_k2_115 === null))))) {
            spec_109 = J.call(J.dict, J.add(J.filter("list", J.filter("items", J.item(J.attr(REFS_0, "defs"), _k2_115))), J.filter("list", J.filter("rejectattr", J.filter("items", spec_109), "0", "equalto", "$ref"))));
            J.setattr(REFS_0, "seen", J.add(J.add(J.attr(REFS_0, "seen"), _k2_115), "|"));
          }
        }
      }
    }
    out.push(J.str(J.add(J.add(J.add("<param name=", name_108), " type="), J.call(render_compact_type_0, spec_109))));
    if (J.t((J.has(name_108, ((v) => J.t(v) ? v : [])(required_list_110))))) {
      out.push(J.str(" required=true"));
    }
    if (J.t(J.attr(spec_109, "enum"))) {
      out.push(J.str(" enum="), J.str(J.call(render_xml_enum_0, J.attr(spec_109, "enum"))));
    }
    if (J.t((J.attr(spec_109, "default") !== undefined))) {
      out.push(J.str(J.call(render_xml_default_attr_0, J.attr(spec_109, "default"))));
    }
    out.push(J.str(J.call(render_xml_schema_attrs_0, spec_109, false)));
    if (J.t(((v) => J.t(v) ? v : (J.eq(J.call(xml_schema_has_children_0, spec_109, true, false), "true")))(J.attr(spec_109, "description")))) {
      out.push(J.str(">"));
      if (J.t(J.attr(spec_109, "description"))) {
        out.push(J.str(J.attr(spec_109, "description")));
      }
      out.push(J.str(J.call(render_xml_schema_children_0, spec_109, true, false)), J.str("</param>"));
    } else {
      out.push(J.str("/>"));
    }
    return out.join("");
  };
  render_tools_xml_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_117 = k.caller;
    let tools_list_116 = p.length > 0 ? p[0] : "tools_list" in k ? k["tools_list"] : undefined;
    out.push(J.str("<ifm|tools>"));
    {
      let _items28 = J.iter(tools_list_116);
      const _loop28 = J.loop(_items28);
      for (let _i28 = 0; _i28 < _items28.length; _i28++) {
        _loop28.index0 = _i28; const _it28 = _items28[_i28]; let tool_118 = _it28;
        let fn_119 = fn_0;
        let fnp_120 = fnp_0;
        let _r_121 = _r_0;
        let _k_122 = _k_0;
        let fn_ret_123 = fn_ret_0;
        fn_119 = (J.t((J.attr(tool_118, "function") !== undefined)) ? J.attr(tool_118, "function") : tool_118);
        J.setattr(REFS_0, "defs", (J.t(((v) => J.t(v) ? J.test("mapping", J.item(J.attr(fn_119, "parameters"), "$defs")) : v)(J.test("mapping", J.attr(fn_119, "parameters")))) ? J.item(J.attr(fn_119, "parameters"), "$defs") : (J.t(((v) => J.t(v) ? J.test("mapping", J.item(J.attr(fn_119, "parameters"), "definitions")) : v)(J.test("mapping", J.attr(fn_119, "parameters")))) ? J.item(J.attr(fn_119, "parameters"), "definitions") : ({}))));
        J.setattr(REFS_0, "seen", "|");
        fnp_120 = J.call(J.namespace, J.kw({"p": J.attr(fn_119, "parameters")}));
        if (J.t(((v) => J.t(v) ? J.test("string", J.item(J.attr(fnp_120, "p"), "$ref")) : v)(J.test("mapping", J.attr(fnp_120, "p"))))) {
          _r_121 = J.item(J.attr(fnp_120, "p"), "$ref");
          _k_122 = (J.t(J.call(J.attr(_r_121, "startswith"), "#/$defs/")) ? J.slice(_r_121, 8, undefined, undefined) : (J.t(J.call(J.attr(_r_121, "startswith"), "#/definitions/")) ? J.slice(_r_121, 14, undefined, undefined) : null));
          if (J.t(((v) => J.t(v) ? J.test("mapping", J.item(J.attr(REFS_0, "defs"), _k_122)) : v)(!J.t((_k_122 === null))))) {
            J.setattr(fnp_120, "p", J.call(J.dict, J.add(J.filter("list", J.filter("items", J.item(J.attr(REFS_0, "defs"), _k_122))), J.filter("list", J.filter("rejectattr", J.filter("items", J.attr(fnp_120, "p")), "0", "equalto", "$ref")))));
            J.setattr(REFS_0, "seen", J.add(J.add(J.attr(REFS_0, "seen"), _k_122), "|"));
          }
        }
        out.push(J.str(J.add(J.add("\n<function name=", J.attr(fn_119, "name")), ">")));
        if (J.t(J.attr(fn_119, "description"))) {
          out.push(J.str("<description>"), J.str(J.attr(fn_119, "description")), J.str("</description>"));
        }
        out.push(J.str("<parameters>"));
        if (J.t(((v) => J.t(v) ? J.attr(J.attr(fnp_120, "p"), "properties") : v)(J.attr(fnp_120, "p")))) {
          {
            let _items29 = J.iter(J.filter("items", J.attr(J.attr(fnp_120, "p"), "properties")));
            const _loop29 = J.loop(_items29);
            for (let _i29 = 0; _i29 < _items29.length; _i29++) {
              _loop29.index0 = _i29; const _it29 = _items29[_i29]; let [pname_124, pspec_125] = J.unpack(_it29, 2);
              out.push(J.str(J.call(render_xml_param_0, pname_124, pspec_125, ((v) => J.t(v) ? v : [])(J.attr(J.attr(fnp_120, "p"), "required")))));
            }
          }
        } else if (J.t(((v) => J.t(v) ? ((v) => J.t(v) ? v : (J.has("items", J.attr(fnp_120, "p"))))(((v) => J.t(v) ? v : J.attr(J.attr(fnp_120, "p"), "anyOf"))(J.attr(J.attr(fnp_120, "p"), "oneOf"))) : v)(J.test("mapping", J.attr(fnp_120, "p"))))) {
          out.push(J.str(J.call(render_xml_schema_children_0, J.attr(fnp_120, "p"), true, false)));
        }
        out.push(J.str("</parameters>"));
        fn_ret_123 = (J.t((J.attr(fn_119, "returns") !== undefined)) ? J.attr(fn_119, "returns") : J.attr(fn_119, "response"));
        if (J.t(J.test("mapping", fn_ret_123))) {
          out.push(J.str(J.call(render_xml_schema_node_0, "returns", fn_ret_123, true)));
        } else if (J.t((fn_ret_123 !== undefined))) {
          out.push("<returns>", J.str(J.call(render_xml_value_0, fn_ret_123)), "</returns>");
        }
        out.push(J.str("</function>"));
      }
    }
    out.push(J.str("\n</ifm|tools>"));
    return out.join("");
  };
  render_markdown_literal_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_127 = k.caller;
    let value_126 = p.length > 0 ? p[0] : "value" in k ? k["value"] : undefined;
    if (J.t(((v) => J.t(v) ? (J.eq(value_126, "")) : v)(J.test("string", value_126)))) {
      out.push("\"\"");
    } else if (J.t(J.test("string", value_126))) {
      out.push("`", J.str(J.filter("replace", value_126, "\n", "\\n")), "`");
    } else {
      out.push("`", J.str(J.call(render_python_repr_0, value_126)), "`");
    }
    return out.join("");
  };
  render_allowed_values_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_129 = k.caller;
    let values_128 = p.length > 0 ? p[0] : "values" in k ? k["values"] : undefined;
    {
      let _items30 = J.iter(values_128);
      const _loop30 = J.loop(_items30);
      for (let _i30 = 0; _i30 < _items30.length; _i30++) {
        _loop30.index0 = _i30; const _it30 = _items30[_i30]; let value_130 = _it30;
        out.push(J.str(J.call(render_markdown_literal_0, value_130)));
        if (J.t(!J.t(J.attr(_loop30, "last")))) {
          out.push(", ");
        }
      }
    }
    return out.join("");
  };
  render_markdown_value_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_132 = k.caller;
    let value_131 = p.length > 0 ? p[0] : "value" in k ? k["value"] : undefined;
    if (J.t(((v) => J.t(v) ? (J.eq(value_131, "")) : v)(J.test("string", value_131)))) {
      out.push("\"\"");
    } else if (J.t(J.test("string", value_131))) {
      out.push(J.str(value_131));
    } else {
      out.push(J.str(J.call(render_python_repr_0, value_131)));
    }
    return out.join("");
  };
  render_markdown_detail_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_136 = k.caller;
    let indent_133 = p.length > 0 ? p[0] : "indent" in k ? k["indent"] : undefined;
    let label_134 = p.length > 1 ? p[1] : "label" in k ? k["label"] : undefined;
    let value_135 = p.length > 2 ? p[2] : "value" in k ? k["value"] : undefined;
    out.push(J.str(J.add(J.add(J.add(J.add("\n", indent_133), "  - "), label_134), ": ")), J.str(J.call(render_markdown_value_0, value_135)));
    return out.join("");
  };
  render_markdown_metadata_detail_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_139 = k.caller;
    let label_137 = p.length > 0 ? p[0] : "label" in k ? k["label"] : undefined;
    let value_138 = p.length > 1 ? p[1] : "value" in k ? k["value"] : undefined;
    out.push(J.str(J.add(J.add("\n- ", label_137), ": ")), J.str(J.call(render_markdown_value_0, value_138)));
    return out.join("");
  };
  render_markdown_schema_annotations_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_143 = k.caller;
    let spec_140 = p.length > 0 ? p[0] : "spec" in k ? k["spec"] : undefined;
    let indent_141 = p.length > 1 ? p[1] : "indent" in k ? k["indent"] : undefined;
    let include_value_details_142 = p.length > 2 ? p[2] : "include_value_details" in k ? k["include_value_details"] : undefined;
    if (J.t(((v) => J.t(v) ? (J.attr(spec_140, "description") !== undefined) : v)(include_value_details_142))) {
      out.push(J.str(J.call(render_markdown_detail_0, indent_141, "Description", J.filter("replace", J.attr(spec_140, "description"), "\n", J.add(J.add("\n", indent_141), "    ")))));
    }
    if (J.t(((v) => J.t(v) ? (J.attr(spec_140, "enum") !== undefined) : v)(include_value_details_142))) {
      out.push(J.str(J.add(J.add("\n", indent_141), "  - Allowed values: ")), J.str(J.call(render_allowed_values_0, J.attr(spec_140, "enum"))));
    }
    if (J.t(((v) => J.t(v) ? (J.attr(spec_140, "default") !== undefined) : v)(include_value_details_142))) {
      out.push(J.str(J.add(J.add("\n", indent_141), "  - Default: ")), J.str(J.call(render_markdown_literal_0, J.attr(spec_140, "default"))));
    }
    if (J.t((J.attr(spec_140, "additionalProperties") !== undefined))) {
      if (J.t(J.test("mapping", J.attr(spec_140, "additionalProperties")))) {
        out.push(J.str(J.add(J.add(J.add(J.add("\n", indent_141), "  - Additional properties *("), J.call(render_markdown_type_0, J.attr(spec_140, "additionalProperties"))), ")*")), J.str(J.call(render_markdown_schema_details_0, J.attr(spec_140, "additionalProperties"), J.add(indent_141, "  "), true)));
      } else {
        out.push(J.str(J.call(render_markdown_detail_0, indent_141, "Additional properties", J.attr(spec_140, "additionalProperties"))));
      }
    }
    return out.join("");
  };
  render_markdown_metadata_annotations_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_145 = k.caller;
    let spec_144 = p.length > 0 ? p[0] : "spec" in k ? k["spec"] : undefined;
    if (J.t((J.attr(spec_144, "description") !== undefined))) {
      out.push(J.str(J.call(render_markdown_metadata_detail_0, "Description", J.filter("replace", J.attr(spec_144, "description"), "\n", "\n    "))));
    }
    if (J.t((J.attr(spec_144, "enum") !== undefined))) {
      out.push(J.str("\n- Allowed values: "), J.str(J.call(render_allowed_values_0, J.attr(spec_144, "enum"))));
    }
    if (J.t((J.attr(spec_144, "default") !== undefined))) {
      out.push(J.str("\n- Default: "), J.str(J.call(render_markdown_literal_0, J.attr(spec_144, "default"))));
    }
    if (J.t((J.attr(spec_144, "additionalProperties") !== undefined))) {
      if (J.t(J.test("mapping", J.attr(spec_144, "additionalProperties")))) {
        out.push(J.str(J.add(J.add("\n- Additional properties *(", J.call(render_markdown_type_0, J.attr(spec_144, "additionalProperties"))), ")*")), J.str(J.call(render_markdown_schema_details_0, J.attr(spec_144, "additionalProperties"), "", true)));
      } else {
        out.push(J.str(J.call(render_markdown_metadata_detail_0, "Additional properties", J.attr(spec_144, "additionalProperties"))));
      }
    }
    return out.join("");
  };
  render_markdown_schema_extras_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_148 = k.caller;
    let spec_146 = p.length > 0 ? p[0] : "spec" in k ? k["spec"] : undefined;
    let indent_147 = p.length > 1 ? p[1] : "indent" in k ? k["indent"] : undefined;
    let rendered_keys_149 = rendered_keys_0;
    rendered_keys_149 = ["type", "description", "enum", "default", "properties", "required", "items", "oneOf", "anyOf", "additionalProperties", "patternProperties", "returns"];
    {
      let _items31 = J.iter(J.filter("items", spec_146));
      const _loop31 = J.loop(_items31);
      for (let _i31 = 0; _i31 < _items31.length; _i31++) {
        _loop31.index0 = _i31; const _it31 = _items31[_i31]; let [key_150, value_151] = J.unpack(_it31, 2);
        if (J.t((!J.has(key_150, rendered_keys_149)))) {
          out.push(J.str(J.add(J.add(J.add(J.add("\n", indent_147), "  - "), key_150), ": ")), J.str(J.call(render_markdown_value_0, value_151)));
        }
      }
    }
    return out.join("");
  };
  render_markdown_metadata_extras_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_153 = k.caller;
    let spec_152 = p.length > 0 ? p[0] : "spec" in k ? k["spec"] : undefined;
    let rendered_keys_154 = rendered_keys_0;
    rendered_keys_154 = ["type", "description", "enum", "default", "properties", "required", "items", "oneOf", "anyOf", "additionalProperties", "patternProperties", "returns"];
    {
      let _items32 = J.iter(J.filter("items", spec_152));
      const _loop32 = J.loop(_items32);
      for (let _i32 = 0; _i32 < _items32.length; _i32++) {
        _loop32.index0 = _i32; const _it32 = _items32[_i32]; let [key_155, value_156] = J.unpack(_it32, 2);
        if (J.t((!J.has(key_155, rendered_keys_154)))) {
          out.push(J.str(J.add(J.add("\n- ", key_155), ": ")), J.str(J.call(render_markdown_value_0, value_156)));
        }
      }
    }
    return out.join("");
  };
  markdown_schema_has_extra_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_158 = k.caller;
    let spec_157 = p.length > 0 ? p[0] : "spec" in k ? k["spec"] : undefined;
    let rendered_keys_159 = rendered_keys_0;
    let found_160 = found_0;
    rendered_keys_159 = ["type", "description", "enum", "default", "properties", "required", "items", "oneOf", "anyOf", "additionalProperties", "patternProperties", "returns"];
    found_160 = J.call(J.namespace, J.kw({"value": "false"}));
    {
      let _items33 = J.iter(J.filter("items", spec_157));
      const _loop33 = J.loop(_items33);
      for (let _i33 = 0; _i33 < _items33.length; _i33++) {
        _loop33.index0 = _i33; const _it33 = _items33[_i33]; let [key_161, value_162] = J.unpack(_it33, 2);
        if (J.t((!J.has(key_161, rendered_keys_159)))) {
          J.setattr(found_160, "value", "true");
        }
      }
    }
    out.push(J.str(J.attr(found_160, "value")));
    return out.join("");
  };
  markdown_parameter_schema_has_details_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_164 = k.caller;
    let spec_163 = p.length > 0 ? p[0] : "spec" in k ? k["spec"] : undefined;
    if (J.t(((v) => J.t(v) ? v : (J.eq(J.call(markdown_schema_has_extra_0, spec_163), "true")))(((v) => J.t(v) ? v : (J.attr(spec_163, "returns") !== undefined))(((v) => J.t(v) ? v : J.attr(spec_163, "anyOf"))(((v) => J.t(v) ? v : J.attr(spec_163, "oneOf"))(((v) => J.t(v) ? v : (J.has("items", spec_163)))(((v) => J.t(v) ? v : (J.attr(spec_163, "patternProperties") !== undefined))(((v) => J.t(v) ? v : (J.attr(spec_163, "additionalProperties") !== undefined))(((v) => J.t(v) ? v : (J.attr(spec_163, "default") !== undefined))(((v) => J.t(v) ? v : (J.attr(spec_163, "enum") !== undefined))((J.attr(spec_163, "description") !== undefined)))))))))))) {
      out.push("true");
    } else {
      out.push("false");
    }
    return out.join("");
  };
  render_markdown_schema_structure_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_168 = k.caller;
    let spec_165 = p.length > 0 ? p[0] : "spec" in k ? k["spec"] : undefined;
    let indent_166 = p.length > 1 ? p[1] : "indent" in k ? k["indent"] : undefined;
    let include_properties_167 = p.length > 2 ? p[2] : "include_properties" in k ? k["include_properties"] : undefined;
    if (J.t(((v) => J.t(v) ? J.attr(spec_165, "properties") : v)(include_properties_167))) {
      {
        let _items34 = J.iter(J.filter("items", J.attr(spec_165, "properties")));
        const _loop34 = J.loop(_items34);
        for (let _i34 = 0; _i34 < _items34.length; _i34++) {
          _loop34.index0 = _i34; const _it34 = _items34[_i34]; let [child_name_169, child_spec_170] = J.unpack(_it34, 2);
          out.push(J.str(J.call(render_markdown_param_0, child_name_169, child_spec_170, ((v) => J.t(v) ? v : [])(J.attr(spec_165, "required")), J.add(indent_166, "  "))));
        }
      }
    }
    if (J.t(((v) => J.t(v) ? J.test("mapping", J.item(spec_165, "items")) : v)((J.has("items", spec_165))))) {
      out.push(J.str(J.add(J.add(J.add(J.add("\n", indent_166), "  - Items *("), J.call(render_markdown_type_0, J.item(spec_165, "items"))), ")*")), J.str(J.call(render_markdown_schema_details_0, J.item(spec_165, "items"), J.add(indent_166, "  "), true)));
    } else if (J.t((J.has("items", spec_165)))) {
      out.push(J.str(J.call(render_markdown_detail_0, indent_166, "Items", J.item(spec_165, "items"))));
    }
    if (J.t(J.attr(spec_165, "oneOf"))) {
      out.push(J.str(J.add(J.add("\n", indent_166), "  - oneOf:")));
      {
        let _items35 = J.iter(J.attr(spec_165, "oneOf"));
        const _loop35 = J.loop(_items35);
        for (let _i35 = 0; _i35 < _items35.length; _i35++) {
          _loop35.index0 = _i35; const _it35 = _items35[_i35]; let variant_171 = _it35;
          out.push(J.str(J.add(J.add("\n", indent_166), "    - Variant ")), J.str(J.attr(_loop35, "index")), J.str(J.add(J.add(" *(", J.call(render_markdown_type_0, variant_171)), ")*")), J.str(J.call(render_markdown_schema_details_0, variant_171, J.add(indent_166, "    "), true)));
        }
      }
    }
    if (J.t(J.attr(spec_165, "anyOf"))) {
      out.push(J.str(J.add(J.add("\n", indent_166), "  - anyOf:")));
      {
        let _items36 = J.iter(J.attr(spec_165, "anyOf"));
        const _loop36 = J.loop(_items36);
        for (let _i36 = 0; _i36 < _items36.length; _i36++) {
          _loop36.index0 = _i36; const _it36 = _items36[_i36]; let variant_172 = _it36;
          out.push(J.str(J.add(J.add("\n", indent_166), "    - Variant ")), J.str(J.attr(_loop36, "index")), J.str(J.add(J.add(" *(", J.call(render_markdown_type_0, variant_172)), ")*")), J.str(J.call(render_markdown_schema_details_0, variant_172, J.add(indent_166, "    "), true)));
        }
      }
    }
    if (J.t(J.test("mapping", J.attr(spec_165, "patternProperties")))) {
      out.push(J.str(J.add(J.add("\n", indent_166), "  - Pattern properties:")));
      {
        let _items37 = J.iter(J.filter("items", J.attr(spec_165, "patternProperties")));
        const _loop37 = J.loop(_items37);
        for (let _i37 = 0; _i37 < _items37.length; _i37++) {
          _loop37.index0 = _i37; const _it37 = _items37[_i37]; let [pattern_173, pattern_spec_174] = J.unpack(_it37, 2);
          if (J.t(J.test("mapping", pattern_spec_174))) {
            out.push(J.str(J.add(J.add(J.add(J.add(J.add(J.add("\n", indent_166), "    - `"), pattern_173), "` *("), J.call(render_markdown_type_0, pattern_spec_174)), ")*")), J.str(J.call(render_markdown_schema_details_0, pattern_spec_174, J.add(indent_166, "    "), true)));
          } else {
            out.push(J.str(J.add(J.add(J.add(J.add("\n", indent_166), "    - `"), pattern_173), "`: ")), J.str(J.call(render_markdown_value_0, pattern_spec_174)));
          }
        }
      }
    } else if (J.t((J.attr(spec_165, "patternProperties") !== undefined))) {
      out.push(J.str(J.call(render_markdown_detail_0, indent_166, "Pattern properties", J.attr(spec_165, "patternProperties"))));
    }
    if (J.t(J.test("mapping", J.attr(spec_165, "returns")))) {
      out.push(J.str(J.add(J.add(J.add(J.add("\n", indent_166), "  - Returns *("), J.call(render_markdown_type_0, J.attr(spec_165, "returns"))), ")*")), J.str(J.call(render_markdown_schema_details_0, J.attr(spec_165, "returns"), J.add(indent_166, "  "), true)));
    } else if (J.t((J.attr(spec_165, "returns") !== undefined))) {
      out.push(J.str(J.call(render_markdown_detail_0, indent_166, "Returns", J.attr(spec_165, "returns"))));
    }
    return out.join("");
  };
  render_markdown_schema_details_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_178 = k.caller;
    let spec_175 = p.length > 0 ? p[0] : "spec" in k ? k["spec"] : undefined;
    let indent_176 = p.length > 1 ? p[1] : "indent" in k ? k["indent"] : undefined;
    let include_value_details_177 = p.length > 2 ? p[2] : "include_value_details" in k ? k["include_value_details"] : undefined;
    let _r_179 = _r_0;
    let _k_180 = _k_0;
    let _r2_181 = _r2_0;
    let _k2_182 = _k2_0;
    if (J.t(((v) => J.t(v) ? J.test("string", J.item(spec_175, "$ref")) : v)(J.test("mapping", spec_175)))) {
      _r_179 = J.item(spec_175, "$ref");
      _k_180 = (J.t(J.call(J.attr(_r_179, "startswith"), "#/$defs/")) ? J.slice(_r_179, 8, undefined, undefined) : (J.t(J.call(J.attr(_r_179, "startswith"), "#/definitions/")) ? J.slice(_r_179, 14, undefined, undefined) : null));
      if (J.t(((v) => J.t(v) ? J.test("mapping", J.item(J.attr(REFS_0, "defs"), _k_180)) : v)(((v) => J.t(v) ? (!J.has(J.add(J.add("|", _k_180), "|"), J.attr(REFS_0, "seen"))) : v)(!J.t((_k_180 === null)))))) {
        spec_175 = J.call(J.dict, J.add(J.filter("list", J.filter("items", J.item(J.attr(REFS_0, "defs"), _k_180))), J.filter("list", J.filter("rejectattr", J.filter("items", spec_175), "0", "equalto", "$ref"))));
        J.setattr(REFS_0, "seen", J.add(J.add(J.attr(REFS_0, "seen"), _k_180), "|"));
        if (J.t(J.test("string", J.item(spec_175, "$ref")))) {
          _r2_181 = J.item(spec_175, "$ref");
          _k2_182 = (J.t(J.call(J.attr(_r2_181, "startswith"), "#/$defs/")) ? J.slice(_r2_181, 8, undefined, undefined) : (J.t(J.call(J.attr(_r2_181, "startswith"), "#/definitions/")) ? J.slice(_r2_181, 14, undefined, undefined) : null));
          if (J.t(((v) => J.t(v) ? J.test("mapping", J.item(J.attr(REFS_0, "defs"), _k2_182)) : v)(!J.t((_k2_182 === null))))) {
            spec_175 = J.call(J.dict, J.add(J.filter("list", J.filter("items", J.item(J.attr(REFS_0, "defs"), _k2_182))), J.filter("list", J.filter("rejectattr", J.filter("items", spec_175), "0", "equalto", "$ref"))));
            J.setattr(REFS_0, "seen", J.add(J.add(J.attr(REFS_0, "seen"), _k2_182), "|"));
          }
        }
      }
    }
    if (J.t(J.test("mapping", spec_175))) {
      out.push(J.str(J.call(render_markdown_schema_annotations_0, spec_175, indent_176, include_value_details_177)), J.str(J.call(render_markdown_schema_structure_0, spec_175, indent_176, true)), J.str(J.call(render_markdown_schema_extras_0, spec_175, indent_176)));
    } else if (J.t(((v) => J.t(v) ? !J.t(J.test("sameas", spec_175, false)) : v)(!J.t(J.test("sameas", spec_175, true))))) {
      out.push(J.str(J.add(J.add("\n", indent_176), "  - Value: ")), J.str(J.call(render_markdown_literal_0, spec_175)));
    }
    return out.join("");
  };
  render_markdown_parameter_schema_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_184 = k.caller;
    let spec_183 = p.length > 0 ? p[0] : "spec" in k ? k["spec"] : undefined;
    let _r_185 = _r_0;
    let _k_186 = _k_0;
    let _r2_187 = _r2_0;
    let _k2_188 = _k2_0;
    if (J.t(((v) => J.t(v) ? J.test("string", J.item(spec_183, "$ref")) : v)(J.test("mapping", spec_183)))) {
      _r_185 = J.item(spec_183, "$ref");
      _k_186 = (J.t(J.call(J.attr(_r_185, "startswith"), "#/$defs/")) ? J.slice(_r_185, 8, undefined, undefined) : (J.t(J.call(J.attr(_r_185, "startswith"), "#/definitions/")) ? J.slice(_r_185, 14, undefined, undefined) : null));
      if (J.t(((v) => J.t(v) ? J.test("mapping", J.item(J.attr(REFS_0, "defs"), _k_186)) : v)(((v) => J.t(v) ? (!J.has(J.add(J.add("|", _k_186), "|"), J.attr(REFS_0, "seen"))) : v)(!J.t((_k_186 === null)))))) {
        spec_183 = J.call(J.dict, J.add(J.filter("list", J.filter("items", J.item(J.attr(REFS_0, "defs"), _k_186))), J.filter("list", J.filter("rejectattr", J.filter("items", spec_183), "0", "equalto", "$ref"))));
        J.setattr(REFS_0, "seen", J.add(J.add(J.attr(REFS_0, "seen"), _k_186), "|"));
        if (J.t(J.test("string", J.item(spec_183, "$ref")))) {
          _r2_187 = J.item(spec_183, "$ref");
          _k2_188 = (J.t(J.call(J.attr(_r2_187, "startswith"), "#/$defs/")) ? J.slice(_r2_187, 8, undefined, undefined) : (J.t(J.call(J.attr(_r2_187, "startswith"), "#/definitions/")) ? J.slice(_r2_187, 14, undefined, undefined) : null));
          if (J.t(((v) => J.t(v) ? J.test("mapping", J.item(J.attr(REFS_0, "defs"), _k2_188)) : v)(!J.t((_k2_188 === null))))) {
            spec_183 = J.call(J.dict, J.add(J.filter("list", J.filter("items", J.item(J.attr(REFS_0, "defs"), _k2_188))), J.filter("list", J.filter("rejectattr", J.filter("items", spec_183), "0", "equalto", "$ref"))));
            J.setattr(REFS_0, "seen", J.add(J.add(J.attr(REFS_0, "seen"), _k2_188), "|"));
          }
        }
      }
    }
    if (J.t(J.test("mapping", spec_183))) {
      out.push(J.str(J.call(render_markdown_metadata_annotations_0, spec_183)));
      if (J.t(((v) => J.t(v) ? J.test("mapping", J.item(spec_183, "items")) : v)((J.has("items", spec_183))))) {
        out.push(J.str(J.add(J.add("\n- Items *(", J.call(render_markdown_type_0, J.item(spec_183, "items"))), ")*")), J.str(J.call(render_markdown_schema_details_0, J.item(spec_183, "items"), "", true)));
      } else if (J.t((J.has("items", spec_183)))) {
        out.push(J.str(J.call(render_markdown_metadata_detail_0, "Items", J.item(spec_183, "items"))));
      }
      if (J.t(J.attr(spec_183, "oneOf"))) {
        out.push(J.str("\n- oneOf:"));
        {
          let _items38 = J.iter(J.attr(spec_183, "oneOf"));
          const _loop38 = J.loop(_items38);
          for (let _i38 = 0; _i38 < _items38.length; _i38++) {
            _loop38.index0 = _i38; const _it38 = _items38[_i38]; let variant_189 = _it38;
            out.push(J.str("\n  - Variant "), J.str(J.attr(_loop38, "index")), J.str(J.add(J.add(" *(", J.call(render_markdown_type_0, variant_189)), ")*")), J.str(J.call(render_markdown_schema_details_0, variant_189, "  ", true)));
          }
        }
      }
      if (J.t(J.attr(spec_183, "anyOf"))) {
        out.push(J.str("\n- anyOf:"));
        {
          let _items39 = J.iter(J.attr(spec_183, "anyOf"));
          const _loop39 = J.loop(_items39);
          for (let _i39 = 0; _i39 < _items39.length; _i39++) {
            _loop39.index0 = _i39; const _it39 = _items39[_i39]; let variant_190 = _it39;
            out.push(J.str("\n  - Variant "), J.str(J.attr(_loop39, "index")), J.str(J.add(J.add(" *(", J.call(render_markdown_type_0, variant_190)), ")*")), J.str(J.call(render_markdown_schema_details_0, variant_190, "  ", true)));
          }
        }
      }
      if (J.t(J.test("mapping", J.attr(spec_183, "patternProperties")))) {
        out.push(J.str("\n- Pattern properties:"));
        {
          let _items40 = J.iter(J.filter("items", J.attr(spec_183, "patternProperties")));
          const _loop40 = J.loop(_items40);
          for (let _i40 = 0; _i40 < _items40.length; _i40++) {
            _loop40.index0 = _i40; const _it40 = _items40[_i40]; let [pattern_191, pattern_spec_192] = J.unpack(_it40, 2);
            if (J.t(J.test("mapping", pattern_spec_192))) {
              out.push(J.str(J.add(J.add(J.add(J.add("\n  - `", pattern_191), "` *("), J.call(render_markdown_type_0, pattern_spec_192)), ")*")), J.str(J.call(render_markdown_schema_details_0, pattern_spec_192, "  ", true)));
            } else {
              out.push(J.str(J.add(J.add("\n  - `", pattern_191), "`: ")), J.str(J.call(render_markdown_value_0, pattern_spec_192)));
            }
          }
        }
      } else if (J.t((J.attr(spec_183, "patternProperties") !== undefined))) {
        out.push(J.str(J.call(render_markdown_metadata_detail_0, "Pattern properties", J.attr(spec_183, "patternProperties"))));
      }
      if (J.t(J.test("mapping", J.attr(spec_183, "returns")))) {
        out.push(J.str(J.add(J.add("\n- Returns *(", J.call(render_markdown_type_0, J.attr(spec_183, "returns"))), ")*")), J.str(J.call(render_markdown_schema_details_0, J.attr(spec_183, "returns"), "", true)));
      } else if (J.t((J.attr(spec_183, "returns") !== undefined))) {
        out.push(J.str(J.call(render_markdown_metadata_detail_0, "Returns", J.attr(spec_183, "returns"))));
      }
      out.push(J.str(J.call(render_markdown_metadata_extras_0, spec_183)));
    }
    return out.join("");
  };
  render_markdown_param_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_197 = k.caller;
    let name_193 = p.length > 0 ? p[0] : "name" in k ? k["name"] : undefined;
    let spec_194 = p.length > 1 ? p[1] : "spec" in k ? k["spec"] : undefined;
    let required_list_195 = p.length > 2 ? p[2] : "required_list" in k ? k["required_list"] : undefined;
    let indent_196 = p.length > 3 ? p[3] : "indent" in k ? k["indent"] : undefined;
    let _r_198 = _r_0;
    let _k_199 = _k_0;
    let _r2_200 = _r2_0;
    let _k2_201 = _k2_0;
    if (J.t(((v) => J.t(v) ? J.test("string", J.item(spec_194, "$ref")) : v)(J.test("mapping", spec_194)))) {
      _r_198 = J.item(spec_194, "$ref");
      _k_199 = (J.t(J.call(J.attr(_r_198, "startswith"), "#/$defs/")) ? J.slice(_r_198, 8, undefined, undefined) : (J.t(J.call(J.attr(_r_198, "startswith"), "#/definitions/")) ? J.slice(_r_198, 14, undefined, undefined) : null));
      if (J.t(((v) => J.t(v) ? J.test("mapping", J.item(J.attr(REFS_0, "defs"), _k_199)) : v)(((v) => J.t(v) ? (!J.has(J.add(J.add("|", _k_199), "|"), J.attr(REFS_0, "seen"))) : v)(!J.t((_k_199 === null)))))) {
        spec_194 = J.call(J.dict, J.add(J.filter("list", J.filter("items", J.item(J.attr(REFS_0, "defs"), _k_199))), J.filter("list", J.filter("rejectattr", J.filter("items", spec_194), "0", "equalto", "$ref"))));
        J.setattr(REFS_0, "seen", J.add(J.add(J.attr(REFS_0, "seen"), _k_199), "|"));
        if (J.t(J.test("string", J.item(spec_194, "$ref")))) {
          _r2_200 = J.item(spec_194, "$ref");
          _k2_201 = (J.t(J.call(J.attr(_r2_200, "startswith"), "#/$defs/")) ? J.slice(_r2_200, 8, undefined, undefined) : (J.t(J.call(J.attr(_r2_200, "startswith"), "#/definitions/")) ? J.slice(_r2_200, 14, undefined, undefined) : null));
          if (J.t(((v) => J.t(v) ? J.test("mapping", J.item(J.attr(REFS_0, "defs"), _k2_201)) : v)(!J.t((_k2_201 === null))))) {
            spec_194 = J.call(J.dict, J.add(J.filter("list", J.filter("items", J.item(J.attr(REFS_0, "defs"), _k2_201))), J.filter("list", J.filter("rejectattr", J.filter("items", spec_194), "0", "equalto", "$ref"))));
            J.setattr(REFS_0, "seen", J.add(J.add(J.attr(REFS_0, "seen"), _k2_201), "|"));
          }
        }
      }
    }
    out.push(J.str(J.add(J.add(J.add(J.add(J.add("\n", indent_196), "- `"), name_193), "` *("), J.call(render_markdown_type_0, spec_194))));
    if (J.t((J.has(name_193, ((v) => J.t(v) ? v : [])(required_list_195))))) {
      out.push(J.str(", required"));
    }
    out.push(J.str(")*"));
    if (J.t(J.attr(spec_194, "description"))) {
      out.push(J.str(J.add(" - ", J.filter("replace", J.attr(spec_194, "description"), "\n", J.add(J.add("\n", indent_196), "  ")))));
    }
    if (J.t(J.attr(spec_194, "enum"))) {
      out.push(J.str(J.add(J.add("\n", indent_196), "  - Allowed values: ")), J.str(J.call(render_allowed_values_0, J.attr(spec_194, "enum"))));
    }
    if (J.t((J.attr(spec_194, "default") !== undefined))) {
      out.push(J.str(J.add(J.add("\n", indent_196), "  - Default: ")), J.str(J.call(render_markdown_literal_0, J.attr(spec_194, "default"))));
    }
    out.push(J.str(J.call(render_markdown_schema_details_0, spec_194, indent_196, false)));
    return out.join("");
  };
  render_tools_markdown_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_203 = k.caller;
    let tools_list_202 = p.length > 0 ? p[0] : "tools_list" in k ? k["tools_list"] : undefined;
    out.push(J.str("<ifm|tools>"));
    {
      let _items41 = J.iter(tools_list_202);
      const _loop41 = J.loop(_items41);
      for (let _i41 = 0; _i41 < _items41.length; _i41++) {
        _loop41.index0 = _i41; const _it41 = _items41[_i41]; let tool_204 = _it41;
        let fn_205 = fn_0;
        let fnp_206 = fnp_0;
        let _r_207 = _r_0;
        let _k_208 = _k_0;
        let fn_ret_209 = fn_ret_0;
        fn_205 = (J.t((J.attr(tool_204, "function") !== undefined)) ? J.attr(tool_204, "function") : tool_204);
        J.setattr(REFS_0, "defs", (J.t(((v) => J.t(v) ? J.test("mapping", J.item(J.attr(fn_205, "parameters"), "$defs")) : v)(J.test("mapping", J.attr(fn_205, "parameters")))) ? J.item(J.attr(fn_205, "parameters"), "$defs") : (J.t(((v) => J.t(v) ? J.test("mapping", J.item(J.attr(fn_205, "parameters"), "definitions")) : v)(J.test("mapping", J.attr(fn_205, "parameters")))) ? J.item(J.attr(fn_205, "parameters"), "definitions") : ({}))));
        J.setattr(REFS_0, "seen", "|");
        fnp_206 = J.call(J.namespace, J.kw({"p": J.attr(fn_205, "parameters")}));
        if (J.t(((v) => J.t(v) ? J.test("string", J.item(J.attr(fnp_206, "p"), "$ref")) : v)(J.test("mapping", J.attr(fnp_206, "p"))))) {
          _r_207 = J.item(J.attr(fnp_206, "p"), "$ref");
          _k_208 = (J.t(J.call(J.attr(_r_207, "startswith"), "#/$defs/")) ? J.slice(_r_207, 8, undefined, undefined) : (J.t(J.call(J.attr(_r_207, "startswith"), "#/definitions/")) ? J.slice(_r_207, 14, undefined, undefined) : null));
          if (J.t(((v) => J.t(v) ? J.test("mapping", J.item(J.attr(REFS_0, "defs"), _k_208)) : v)(!J.t((_k_208 === null))))) {
            J.setattr(fnp_206, "p", J.call(J.dict, J.add(J.filter("list", J.filter("items", J.item(J.attr(REFS_0, "defs"), _k_208))), J.filter("list", J.filter("rejectattr", J.filter("items", J.attr(fnp_206, "p")), "0", "equalto", "$ref")))));
            J.setattr(REFS_0, "seen", J.add(J.add(J.attr(REFS_0, "seen"), _k_208), "|"));
          }
        }
        out.push(J.str(J.add("\n## ", J.attr(fn_205, "name"))));
        if (J.t(J.attr(fn_205, "description"))) {
          out.push(J.str(J.add("\n", J.attr(fn_205, "description"))));
        }
        out.push(J.str("\n\n**Parameters**"));
        if (J.t(((v) => J.t(v) ? J.attr(J.attr(fnp_206, "p"), "properties") : v)(J.attr(fnp_206, "p")))) {
          {
            let _items42 = J.iter(J.filter("items", J.attr(J.attr(fnp_206, "p"), "properties")));
            const _loop42 = J.loop(_items42);
            for (let _i42 = 0; _i42 < _items42.length; _i42++) {
              _loop42.index0 = _i42; const _it42 = _items42[_i42]; let [pname_210, pspec_211] = J.unpack(_it42, 2);
              out.push(J.str(J.call(render_markdown_param_0, pname_210, pspec_211, ((v) => J.t(v) ? v : [])(J.attr(J.attr(fnp_206, "p"), "required")), "")));
            }
          }
        } else if (J.t(((v) => J.t(v) ? ((v) => J.t(v) ? v : (J.has("items", J.attr(fnp_206, "p"))))(((v) => J.t(v) ? v : J.attr(J.attr(fnp_206, "p"), "anyOf"))(J.attr(J.attr(fnp_206, "p"), "oneOf"))) : v)(J.test("mapping", J.attr(fnp_206, "p"))))) {
          out.push(J.str(J.call(render_markdown_parameter_schema_0, J.attr(fnp_206, "p"))));
        } else {
          out.push(J.str("\n- None"));
        }
        fn_ret_209 = (J.t((J.attr(fn_205, "returns") !== undefined)) ? J.attr(fn_205, "returns") : J.attr(fn_205, "response"));
        if (J.t(J.test("mapping", fn_ret_209))) {
          out.push(J.str("\n\n**Returns**"), J.str(J.add(J.add("\n- Return *(", J.call(render_markdown_type_0, fn_ret_209)), ")*")), J.str(J.call(render_markdown_schema_details_0, fn_ret_209, "", true)));
        } else if (J.t((fn_ret_209 !== undefined))) {
          out.push(J.str("\n\n**Returns**\n- "), J.str(J.call(render_markdown_value_0, fn_ret_209)));
        }
        if (J.t(!J.t(J.attr(_loop41, "last")))) {
          out.push(J.str("\n"));
        }
      }
    }
    out.push(J.str("\n</ifm|tools>"));
    return out.join("");
  };
  render_tool_presentation_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_214 = k.caller;
    let tools_list_212 = p.length > 0 ? p[0] : "tools_list" in k ? k["tools_list"] : undefined;
    let fmt_213 = p.length > 1 ? p[1] : "fmt" in k ? k["fmt"] : undefined;
    if (J.t((J.eq(fmt_213, "json")))) {
      out.push(J.str(J.call(render_tools_json_0, tools_list_212)));
    } else if (J.t((!J.eq(J.attr(RB_0, "bad"), "|")))) {
      out.push(J.str(J.call(render_tools_json_0, tools_list_212)));
    } else if (J.t((J.eq(fmt_213, "xml")))) {
      out.push(J.str(J.call(render_tools_xml_0, tools_list_212)));
    } else if (J.t((J.eq(fmt_213, "markdown")))) {
      out.push(J.str(J.call(render_tools_markdown_0, tools_list_212)));
    } else {
      out.push(J.str(J.call(J.raise_exception, J.add(J.add("Unsupported tool_presentation_format: '", fmt_213), "'. Supported formats: json, xml, markdown."))));
    }
    return out.join("");
  };
  render_call_instructions_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_216 = k.caller;
    let fmt_215 = p.length > 0 ? p[0] : "fmt" in k ? k["fmt"] : undefined;
    if (J.t((J.eq(fmt_215, "json")))) {
      out.push(J.str("Wrap all tool calls in a single <ifm|tool_calls></ifm|tool_calls> block. For each call, emit one JSON object with the function name and arguments on the same line inside <ifm|tool_call></ifm|tool_call> tags:\n\n<ifm|tool_calls>\n<ifm|tool_call>{\"name\": <function-name>, \"arguments\": <args-json-object>}</ifm|tool_call>\n</ifm|tool_calls>"));
    } else if (J.t((J.eq(fmt_215, "xml")))) {
      out.push(J.str("Wrap all tool calls in a single <ifm|tool_calls></ifm|tool_calls> block. For each call, write the function name at the start of <ifm|tool_call>, followed by paired <ifm|arg_key> and <ifm|arg_value> tags for each argument:\n\n<ifm|tool_calls>\n<ifm|tool_call>$FUNCTION_NAME\n<ifm|arg_key>$PARAMETER_NAME</ifm|arg_key>\n<ifm|arg_value>$PARAMETER_VALUE</ifm|arg_value>\n...\n</ifm|tool_call>\n</ifm|tool_calls>\n\nString and scalar parameters should be written as plain text. Array and object parameters should be written as JSON literals."));
    } else if (J.t((J.eq(fmt_215, "xml_typed")))) {
      out.push(J.str("Wrap all tool calls in a single <ifm|tool_calls></ifm|tool_calls> block. For each call, write the function name at the start of <ifm|tool_call>, followed by <ifm|arg_key>, <ifm|arg_type>, and <ifm|arg_value> tags for each argument:\n\n<ifm|tool_calls>\n<ifm|tool_call>$FUNCTION_NAME\n<ifm|arg_key>$PARAMETER_NAME</ifm|arg_key>\n<ifm|arg_type>$ARGUMENT_TYPE</ifm|arg_type>\n<ifm|arg_value>$PARAMETER_VALUE</ifm|arg_value>\n...\n</ifm|tool_call>\n</ifm|tool_calls>\n\nUse the parameter type shown in the tool definition. If that type contains anyOf or oneOf, use the actual argument value type instead. String and scalar parameters should be written as plain text. Array and object parameters should be written as JSON literals."));
    } else {
      out.push(J.str(J.call(J.raise_exception, J.add(J.add("Unsupported tool_call_format: '", fmt_215), "'. Supported formats: json, xml, xml_typed."))));
    }
    return out.join("");
  };
  render_system_with_tools_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_221 = k.caller;
    let tools_list_217 = p.length > 0 ? p[0] : "tools_list" in k ? k["tools_list"] : undefined;
    let system_content_218 = p.length > 1 ? p[1] : "system_content" in k ? k["system_content"] : undefined;
    let presentation_fmt_219 = p.length > 2 ? p[2] : "presentation_fmt" in k ? k["presentation_fmt"] : undefined;
    let call_fmt_220 = p.length > 3 ? p[3] : "call_fmt" in k ? k["call_fmt"] : undefined;
    out.push(J.str("<|ifm|im_start|>system\n# Tools\nYou may call one or more tools to assist with the user query.\n\nAvailable tools are:\n\n"), J.str(J.call(render_tool_presentation_0, tools_list_217, presentation_fmt_219)), J.str("\n\nWhen calling tools, you MUST follow the tool-call format below:\n\n"), J.str(J.call(render_call_instructions_0, call_fmt_220)));
    if (J.t(system_content_218)) {
      out.push(J.str(J.add("\n\n", system_content_218)));
    }
    out.push(J.str("<|ifm|im_end|>"));
    return out.join("");
  };
  render_argument_value_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_223 = k.caller;
    let value_222 = p.length > 0 ? p[0] : "value" in k ? k["value"] : undefined;
    if (J.t(J.test("string", value_222))) {
      out.push(J.str(value_222));
    } else {
      out.push(J.str(J.filter("tojson", value_222)));
    }
    return out.join("");
  };
  render_value_type_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_225 = k.caller;
    let value_224 = p.length > 0 ? p[0] : "value" in k ? k["value"] : undefined;
    if (J.t((value_224 === null))) {
      out.push("null");
    } else if (J.t(J.test("boolean", value_224))) {
      out.push("boolean");
    } else if (J.t(J.test("integer", value_224))) {
      out.push("integer");
    } else if (J.t(J.test("number", value_224))) {
      out.push("number");
    } else if (J.t(J.test("string", value_224))) {
      out.push("string");
    } else if (J.t(J.test("mapping", value_224))) {
      out.push("object");
    } else if (J.t(J.test("sequence", value_224))) {
      out.push("array");
    } else {
      out.push("any");
    }
    return out.join("");
  };
  schema_has_combinator_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_227 = k.caller;
    let spec_226 = p.length > 0 ? p[0] : "spec" in k ? k["spec"] : undefined;
    let found_228 = found_0;
    if (J.t(((v) => J.t(v) ? v : J.attr(spec_226, "anyOf"))(J.attr(spec_226, "oneOf")))) {
      out.push("true");
    } else if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", J.attr(spec_226, "type")), 1)) : v)(((v) => J.t(v) ? !J.t(J.test("string", J.attr(spec_226, "type"))) : v)(((v) => J.t(v) ? J.test("sequence", J.attr(spec_226, "type")) : v)((J.attr(spec_226, "type") !== undefined)))))) {
      out.push("true");
    } else if (J.t(((v) => J.t(v) ? (J.has("items", spec_226)) : v)((J.eq(J.attr(spec_226, "type"), "array"))))) {
      out.push(J.str(J.call(schema_has_combinator_0, J.item(spec_226, "items"))));
    } else if (J.t(J.attr(spec_226, "properties"))) {
      found_228 = J.call(J.namespace, J.kw({"value": "false"}));
      {
        let _items43 = J.iter(J.filter("items", J.attr(spec_226, "properties")));
        const _loop43 = J.loop(_items43);
        for (let _i43 = 0; _i43 < _items43.length; _i43++) {
          _loop43.index0 = _i43; const _it43 = _items43[_i43]; let [child_name_229, child_spec_230] = J.unpack(_it43, 2);
          if (J.t((J.eq(J.call(schema_has_combinator_0, child_spec_230), "true")))) {
            J.setattr(found_228, "value", "true");
          }
        }
      }
      out.push(J.str(J.attr(found_228, "value")));
    } else {
      out.push("false");
    }
    return out.join("");
  };
  render_arg_type_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_235 = k.caller;
    let tools_list_231 = p.length > 0 ? p[0] : "tools_list" in k ? k["tools_list"] : undefined;
    let tool_name_232 = p.length > 1 ? p[1] : "tool_name" in k ? k["tool_name"] : undefined;
    let arg_name_233 = p.length > 2 ? p[2] : "arg_name" in k ? k["arg_name"] : undefined;
    let value_234 = p.length > 3 ? p[3] : "value" in k ? k["value"] : undefined;
    let found_236 = found_0;
    found_236 = J.call(J.namespace, J.kw({"type": "any"}));
    {
      let _items44 = J.iter(tools_list_231);
      const _loop44 = J.loop(_items44);
      for (let _i44 = 0; _i44 < _items44.length; _i44++) {
        _loop44.index0 = _i44; const _it44 = _items44[_i44]; let tool_237 = _it44;
        let fn_238 = fn_0;
        let spec_239 = spec_0;
        let _r_240 = _r_0;
        let _k_241 = _k_0;
        let _d_242 = _d_0;
        fn_238 = (J.t((J.attr(tool_237, "function") !== undefined)) ? J.attr(tool_237, "function") : tool_237);
        if (J.t(((v) => J.t(v) ? (J.has(arg_name_233, J.attr(J.attr(fn_238, "parameters"), "properties"))) : v)(((v) => J.t(v) ? J.attr(J.attr(fn_238, "parameters"), "properties") : v)(((v) => J.t(v) ? J.attr(fn_238, "parameters") : v)((J.eq(J.attr(fn_238, "name"), tool_name_232))))))) {
          spec_239 = J.item(J.attr(J.attr(fn_238, "parameters"), "properties"), arg_name_233);
          if (J.t(((v) => J.t(v) ? J.test("string", J.item(spec_239, "$ref")) : v)(J.test("mapping", spec_239)))) {
            _r_240 = J.item(spec_239, "$ref");
            _k_241 = (J.t(J.call(J.attr(_r_240, "startswith"), "#/$defs/")) ? J.slice(_r_240, 8, undefined, undefined) : (J.t(J.call(J.attr(_r_240, "startswith"), "#/definitions/")) ? J.slice(_r_240, 14, undefined, undefined) : null));
            _d_242 = (J.t(J.test("mapping", J.item(J.attr(fn_238, "parameters"), "$defs"))) ? J.item(J.attr(fn_238, "parameters"), "$defs") : J.item(J.attr(fn_238, "parameters"), "definitions"));
            spec_239 = (J.t(((v) => J.t(v) ? J.test("mapping", J.item(_d_242, _k_241)) : v)(((v) => J.t(v) ? J.test("mapping", _d_242) : v)(!J.t((_k_241 === null))))) ? J.call(J.dict, J.add(J.filter("list", J.filter("items", J.item(_d_242, _k_241))), J.filter("list", J.filter("rejectattr", J.filter("items", spec_239), "0", "equalto", "$ref")))) : spec_239);
          }
          if (J.t((J.eq(J.call(schema_has_combinator_0, spec_239), "true")))) {
            J.setattr(found_236, "type", J.call(render_value_type_0, value_234));
          } else {
            J.setattr(found_236, "type", J.call(render_compact_type_0, spec_239));
          }
        }
      }
    }
    out.push(J.str(J.attr(found_236, "type")));
    return out.join("");
  };
  render_tool_calls_block_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_246 = k.caller;
    let tool_calls_243 = p.length > 0 ? p[0] : "tool_calls" in k ? k["tool_calls"] : undefined;
    let fmt_244 = p.length > 1 ? p[1] : "fmt" in k ? k["fmt"] : undefined;
    let tools_list_245 = p.length > 2 ? p[2] : "tools_list" in k ? k["tools_list"] : undefined;
    out.push(J.str("<ifm|tool_calls>"));
    {
      let _items45 = J.iter(tool_calls_243);
      const _loop45 = J.loop(_items45);
      for (let _i45 = 0; _i45 < _items45.length; _i45++) {
        _loop45.index0 = _i45; const _it45 = _items45[_i45]; let raw_tool_call_247 = _it45;
        let tool_call_248 = tool_call_0;
        tool_call_248 = (J.t(J.attr(raw_tool_call_247, "function")) ? J.attr(raw_tool_call_247, "function") : raw_tool_call_247);
        if (J.t(J.test("string", J.attr(tool_call_248, "arguments")))) {
          out.push(J.str(J.call(J.raise_exception, "tool_call.arguments must be a dict, not a JSON string. Parse it before passing to the template.")));
        }
        if (J.t((J.eq(fmt_244, "json")))) {
          out.push(J.str(J.add(J.add("\n<ifm|tool_call>{\"name\": \"", J.attr(tool_call_248, "name")), "\", \"arguments\": ")), J.str(J.filter("tojson", J.attr(tool_call_248, "arguments"))), J.str("}</ifm|tool_call>"));
        } else if (J.t(((v) => J.t(v) ? v : (J.eq(fmt_244, "xml_typed")))((J.eq(fmt_244, "xml"))))) {
          out.push(J.str(J.add(J.add("\n<ifm|tool_call>", J.attr(tool_call_248, "name")), "\n")));
          {
            let _items46 = J.iter(J.filter("items", J.attr(tool_call_248, "arguments")));
            const _loop46 = J.loop(_items46);
            for (let _i46 = 0; _i46 < _items46.length; _i46++) {
              _loop46.index0 = _i46; const _it46 = _items46[_i46]; let [key_249, value_250] = J.unpack(_it46, 2);
              out.push(J.str(J.add(J.add("<ifm|arg_key>", key_249), "</ifm|arg_key>\n")));
              if (J.t((J.eq(fmt_244, "xml_typed")))) {
                out.push(J.str(J.add(J.add("<ifm|arg_type>", J.call(render_arg_type_0, tools_list_245, J.attr(tool_call_248, "name"), key_249, value_250)), "</ifm|arg_type>\n")));
              }
              out.push(J.str("<ifm|arg_value>"), J.str(J.call(render_argument_value_0, value_250)), J.str("</ifm|arg_value>\n"));
            }
          }
          out.push(J.str("</ifm|tool_call>"));
        } else {
          out.push(J.str(J.call(J.raise_exception, J.add(J.add("Unsupported tool_call_format: '", fmt_244), "'. Supported formats: json, xml, xml_typed."))));
        }
      }
    }
    out.push(J.str("\n</ifm|tool_calls>"));
    return out.join("");
  };
  render_text_content_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_253 = k.caller;
    let raw_content_251 = p.length > 0 ? p[0] : "raw_content" in k ? k["raw_content"] : undefined;
    let role_252 = p.length > 1 ? p[1] : "role" in k ? k["role"] : undefined;
    if (J.t(J.test("string", raw_content_251))) {
      out.push(J.str(raw_content_251));
    } else if (J.t(((v) => J.t(v) ? ((v) => J.t(v) ? v : (raw_content_251 === undefined))((raw_content_251 === null)) : v)((J.eq(role_252, "assistant"))))) {
      out.push(J.str(""));
    } else if (J.t(((v) => J.t(v) ? !J.t(J.test("mapping", raw_content_251)) : v)(((v) => J.t(v) ? J.test("sequence", raw_content_251) : v)((raw_content_251 !== undefined))))) {
      {
        let _items47 = J.iter(raw_content_251);
        const _loop47 = J.loop(_items47);
        for (let _i47 = 0; _i47 < _items47.length; _i47++) {
          _loop47.index0 = _i47; const _it47 = _items47[_i47]; let part_254 = _it47;
          if (J.t(((v) => J.t(v) ? J.test("string", J.attr(part_254, "text")) : v)(((v) => J.t(v) ? (J.eq(J.attr(part_254, "type"), "text")) : v)(J.test("mapping", part_254))))) {
            out.push(J.str(J.attr(part_254, "text")));
          } else {
            out.push(J.str(J.call(J.raise_exception, J.concat("Unsupported ", role_252, " content part at index ", J.attr(_loop47, "index0"), ". Expected type='text' with a string text field; extract file contents to text before templating."))));
          }
        }
      }
    } else {
      out.push(J.str(J.call(J.raise_exception, J.concat("Unsupported ", role_252, " message content. Expected a string or a list of text parts."))));
    }
    return out.join("");
  };
  render_tool_response_messages_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_256 = k.caller;
    let raw_content_255 = p.length > 0 ? p[0] : "raw_content" in k ? k["raw_content"] : undefined;
    if (J.t(J.test("string", raw_content_255))) {
      out.push(J.str(J.add(J.add("<|ifm|im_start|>tool\n", raw_content_255), "<|ifm|im_end|>")));
    } else if (J.t(((v) => J.t(v) ? !J.t(J.test("mapping", raw_content_255)) : v)(((v) => J.t(v) ? !J.t(J.test("string", raw_content_255)) : v)(J.test("sequence", raw_content_255))))) {
      if (J.t((J.eq(J.filter("length", raw_content_255), 0)))) {
        out.push(J.str(J.call(J.raise_exception, "tool message content list must not be empty.")));
      }
      out.push(J.str("<|ifm|im_start|>tool\n"));
      {
        let _items48 = J.iter(raw_content_255);
        const _loop48 = J.loop(_items48);
        for (let _i48 = 0; _i48 < _items48.length; _i48++) {
          _loop48.index0 = _i48; const _it48 = _items48[_i48]; let item_257 = _it48;
          if (J.t(!J.t(J.attr(_loop48, "first")))) {
            out.push(J.str("\n"));
          }
          if (J.t(J.test("string", item_257))) {
            out.push(J.str(item_257));
          } else if (J.t(((v) => J.t(v) ? J.test("string", J.attr(item_257, "text")) : v)(J.test("mapping", item_257)))) {
            out.push(J.str(J.attr(item_257, "text")));
          } else {
            out.push(J.str(J.filter("tojson", item_257)));
          }
        }
      }
      out.push(J.str("<|ifm|im_end|>"));
    } else {
      out.push(J.str("<|ifm|im_start|>tool\n"), J.str(J.filter("tojson", raw_content_255)), J.str("<|ifm|im_end|>"));
    }
    return out.join("");
  };
  available_tools_0 = (J.t(tools_0) ? tools_0 : []);
  if (J.t(((v) => J.t(v) ? J.call(J.attr(J.item(messages_0, 0), "get"), "tools") : v)(((v) => J.t(v) ? (J.eq(J.attr(J.item(messages_0, 0), "role"), "system")) : v)(!J.t(available_tools_0))))) {
    available_tools_0 = J.item(J.item(messages_0, 0), "tools");
  }
  system_content_0 = "";
  if (J.t((J.eq(J.attr(J.item(messages_0, 0), "role"), "system")))) {
    system_content_0 = J.call(render_text_content_0, J.attr(J.item(messages_0, 0), "content"), "system");
  }
  if (J.t(available_tools_0)) {
    out.push(J.str(J.call(validate_tools_0, available_tools_0, (!J.eq(tool_presentation_fmt_0, "json")))), J.str(J.call(render_system_with_tools_0, available_tools_0, system_content_0, tool_presentation_fmt_0, tool_call_fmt_0)));
  } else {
    if (J.t((J.eq(J.attr(J.item(messages_0, 0), "role"), "system")))) {
      out.push(J.str(J.add(J.add("<|ifm|im_start|>system\n", system_content_0), "<|ifm|im_end|>")));
    }
  }
  {
    let _items49 = J.iter(messages_0);
    const _loop49 = J.loop(_items49);
    for (let _i49 = 0; _i49 < _items49.length; _i49++) {
      _loop49.index0 = _i49; const _it49 = _items49[_i49]; let message_258 = _it49;
      let content_259 = content_0;
      let thinking_content_260 = thinking_content_0;
      let think_tag_261 = think_tag_0;
      if (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? !J.t(J.attr(_loop49, "first")) : v)((J.eq(J.attr(message_258, "role"), "system"))))(((v) => J.t(v) ? v : (J.eq(J.attr(message_258, "role"), "assistant")))((J.eq(J.attr(message_258, "role"), "user")))))) {
        content_259 = J.call(render_text_content_0, J.attr(message_258, "content"), J.attr(message_258, "role"));
      } else {
        content_259 = "";
      }
      if (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? !J.t(J.attr(_loop49, "first")) : v)((J.eq(J.attr(message_258, "role"), "system"))))((J.eq(J.attr(message_258, "role"), "user"))))) {
        out.push(J.str(J.add(J.add(J.add(J.add("<|ifm|im_start|>", J.attr(message_258, "role")), "\n"), content_259), "<|ifm|im_end|>")));
      } else if (J.t((J.eq(J.attr(message_258, "role"), "assistant")))) {
        thinking_content_260 = "";
        think_tag_261 = "ifm|think";
        if (J.t(((v) => J.t(v) ? J.test("string", J.attr(message_258, "think")) : v)((J.attr(message_258, "think") !== undefined)))) {
          thinking_content_260 = J.attr(message_258, "think");
          think_tag_261 = "ifm|think";
        } else if (J.t(((v) => J.t(v) ? J.test("string", J.attr(message_258, "think_fast")) : v)((J.attr(message_258, "think_fast") !== undefined)))) {
          thinking_content_260 = J.attr(message_258, "think_fast");
          think_tag_261 = "ifm|think_fast";
        } else if (J.t(((v) => J.t(v) ? J.test("string", J.attr(message_258, "think_faster")) : v)((J.attr(message_258, "think_faster") !== undefined)))) {
          thinking_content_260 = J.attr(message_258, "think_faster");
          think_tag_261 = "ifm|think_faster";
        } else if (J.t(((v) => J.t(v) ? J.test("string", J.attr(message_258, "reasoning_content")) : v)((J.attr(message_258, "reasoning_content") !== undefined)))) {
          thinking_content_260 = J.attr(message_258, "reasoning_content");
          think_tag_261 = "ifm|think";
        } else if (J.t(((v) => J.t(v) ? J.test("string", J.attr(message_258, "reasoning")) : v)((J.attr(message_258, "reasoning") !== undefined)))) {
          thinking_content_260 = J.attr(message_258, "reasoning");
          think_tag_261 = "ifm|think";
        } else if (J.t(((v) => J.t(v) ? !J.t((J.attr(message_258, "think_faster") !== undefined)) : v)(((v) => J.t(v) ? !J.t((J.attr(message_258, "think_fast") !== undefined)) : v)(((v) => J.t(v) ? !J.t((J.attr(message_258, "reasoning_content") !== undefined)) : v)(((v) => J.t(v) ? !J.t((J.attr(message_258, "reasoning") !== undefined)) : v)(!J.t((J.attr(message_258, "think") !== undefined)))))))) {
          out.push(J.str(J.call(J.raise_exception, "Assistant message is missing a thinking field. Provide one of: think, reasoning, reasoning_content, think_fast, think_faster.")));
        } else {
          out.push(J.str(J.call(J.raise_exception, "Assistant thinking fields must be strings. Provide one of: think, reasoning, reasoning_content, think_fast, think_faster as a string.")));
        }
        out.push(J.str(J.add("<|ifm|im_start|>", J.attr(message_258, "role"))), "\n");
        {
          if (J.t(think_tag_261)) {
            if (J.t(thinking_content_260)) {
              out.push(J.str(J.add(J.add(J.add(J.add(J.add(J.add(J.add("<", think_tag_261), ">\n"), thinking_content_260), "</"), think_tag_261), ">"), content_259)));
            } else {
              out.push(J.str(J.add(J.add(J.add(J.add(J.add("<", think_tag_261), ">\n</"), think_tag_261), ">"), content_259)));
            }
          } else {
            out.push(J.str(content_259));
          }
          if (J.t(J.attr(message_258, "tool_calls"))) {
            out.push(J.str(J.call(render_tool_calls_block_0, J.attr(message_258, "tool_calls"), tool_call_fmt_0, available_tools_0)));
          }
          out.push(J.str("<|ifm|im_end|>"));
        }
      } else if (J.t((J.eq(J.attr(message_258, "role"), "tool")))) {
        out.push(J.str(J.call(render_tool_response_messages_0, J.attr(message_258, "content"))));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    effort_0 = J.filter("default", reasoning_effort_0, "high");
    if (J.t((J.eq(effort_0, "high")))) {
      out.push(J.str("<|ifm|im_start|>assistant\n<ifm|think>\n"));
    } else if (J.t((J.eq(effort_0, "medium")))) {
      out.push(J.str("<|ifm|im_start|>assistant\n<ifm|think_fast>\n"));
    } else if (J.t((J.eq(effort_0, "low")))) {
      out.push(J.str("<|ifm|im_start|>assistant\n<ifm|think_faster>\n"));
    } else {
      out.push(J.str(J.call(J.raise_exception, J.add(J.add("Unsupported reasoning_effort: '", effort_0), "'. Supported values: high, medium, low."))));
    }
  }
  return out.join("");
}
