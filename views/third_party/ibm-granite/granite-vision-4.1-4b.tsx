// Converted from the chat template of ibm-granite/granite-vision-4.1-4b by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let chart2code_prompt_0 = ctx["chart2code_prompt"];
  let chart2csv_prompt_0 = ctx["chart2csv_prompt"];
  let chart2summary_prompt_0 = ctx["chart2summary_prompt"];
  let tables_json_prompt_0 = ctx["tables_json_prompt"];
  let tables_html_prompt_0 = ctx["tables_html_prompt"];
  let tables_otsl_prompt_0 = ctx["tables_otsl_prompt"];
  let expand_tags_0 = ctx["expand_tags"];
  let render_content_0 = ctx["render_content"];
  let tools_system_message_prefix_0 = ctx["tools_system_message_prefix"];
  let tools_system_message_suffix_0 = ctx["tools_system_message_suffix"];
  let documents_system_message_prefix_0 = ctx["documents_system_message_prefix"];
  let documents_system_message_suffix_0 = ctx["documents_system_message_suffix"];
  let g4_default_system_message_0 = ctx["g4_default_system_message"];
  let tools_0 = ctx["tools"] ?? null;
  let ns_0 = ctx["ns"];
  let has_image_0 = ctx["has_image"];
  let img_idx_0 = ctx["img_idx"];
  let tag_idx_0 = ctx["tag_idx"];
  let img_prefix_0 = ctx["img_prefix"];
  let img_suffix_0 = ctx["img_suffix"];
  let available_tools_0 = ctx["available_tools"];
  let documents_0 = ctx["documents"] ?? null;
  let messages_0 = ctx["messages"];
  let content_0 = ctx["content"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  chart2code_prompt_0 = "Generate code that recreates the chart as best as possible.";
  chart2csv_prompt_0 = "Please examine this chart image. Consider you are a data visualization expert, and extract the data into a CSV table.\n\nYour CSV should:\n- Include a header row with clear column names\n- Represent all data series/categories shown in the chart\n- Use numeric values that match the chart as closely as possible\n\nOutput only the CSV data, nothing else.";
  chart2summary_prompt_0 = "Can you describe this chart image?";
  tables_json_prompt_0 = "Identify and extract the table schema\n Extract the schema of all the tables in the image sorted according to the reading order.\nThe output must be a valid JSON object containing a list of dictionaries with the following structure:\n\n                {\n                    \"dimensions\": {\n                        \"rows\": <number of data rows (excluding header rows)>,\n                        \"columns\": <number of columns>,\n                        \"header_rows\": <number of header rows>,\n                        \"total_rows\": <total number of rows including headers>\n                    },\n                    \"cells\": [\n                        {\n                        \"row\": <row index starting at 1>,\n                        \"col\": <column index starting at 1>,\n                        \"colspan\": <number of columns spanned>,\n                        \"rowspan\": <number of rows spanned>,\n                        \"type\": \"<'header' or 'data'>\",\n                        \"header_level\": <header nesting level if type=header, else omit or null>,\n                        \"content\": \"<string content of the cell>\"\n                        },\n                        ...\n                    ]\n                }";
  tables_html_prompt_0 = "Identify and extract the table schema\n Extract the schema of all the tables in the image sorted according to the reading order.\nThe output must be a list of valid HTML tables";
  tables_otsl_prompt_0 = "Identify and extract the table schema\n Extract the schema of all the tables in the image sorted according to the reading order.\nThe output must be a list of valid OTSL objects, each consists of the following fields: \n                        <fcel> - a cell with content in it\n                        <ecel> - an empty cell\n                        <lcel> - a cell that is merged with the cell to its left\n                        <ucel> - a cell that is merged with the cell above it\n                        <xcel> - a cell that is merged with both the cell above it and the cell to its left\n                        <nl> - a new line\n                        <ched> - a column header\n                        <otsl> - the beginning of the OTSL table\n                        </otsl> - the end of the OTSL table\n\n                        An example for an output:\n                        [\n                        <otsl><ched>first table header1<ched>first table header2<nl><fcel>data1<fcel>data2<nl><fcel>data with horizontal span<lcel><nl><fcel>data with vertical span<ecel><nl><ucel><fcel>data3<nl></otsl>,\n                        <otsl><ched>second table header1<ched>second table header2<nl><fcel>data1<fcel>data2<nl><fcel>data with horizontal span<lcel><nl><fcel>data with vertical span<ecel><nl><ucel><fcel>data3<nl></otsl>\n                        ]";
  expand_tags_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_2 = k.caller;
    let text_1 = p.length > 0 ? p[0] : "text" in k ? k["text"] : undefined;
    let has_image_3 = has_image_0;
    let img_idx_4 = img_idx_0;
    let tag_idx_5 = tag_idx_0;
    let img_prefix_6 = img_prefix_0;
    let img_suffix_7 = img_suffix_0;
    has_image_3 = (J.has("<image>", text_1));
    if (J.t(has_image_3)) {
      img_idx_4 = J.call(J.attr(text_1, "index"), "<image>");
      if (J.t((J.has("<chart2code>", text_1)))) {
        tag_idx_5 = J.call(J.attr(text_1, "index"), "<chart2code>");
      } else if (J.t((J.has("<chart2csv>", text_1)))) {
        tag_idx_5 = J.call(J.attr(text_1, "index"), "<chart2csv>");
      } else if (J.t((J.has("<chart2summary>", text_1)))) {
        tag_idx_5 = J.call(J.attr(text_1, "index"), "<chart2summary>");
      } else if (J.t((J.has("<tables_json>", text_1)))) {
        tag_idx_5 = J.call(J.attr(text_1, "index"), "<tables_json>");
      } else if (J.t((J.has("<tables_html>", text_1)))) {
        tag_idx_5 = J.call(J.attr(text_1, "index"), "<tables_html>");
      } else if (J.t((J.has("<tables_otsl>", text_1)))) {
        tag_idx_5 = J.call(J.attr(text_1, "index"), "<tables_otsl>");
      } else {
        tag_idx_5 = 999999;
      }
      img_prefix_6 = (J.t((J.lt(img_idx_4, tag_idx_5))) ? "<image>\n" : "");
      img_suffix_7 = (J.t((J.ge(img_idx_4, tag_idx_5))) ? "<image>\n" : "");
    } else {
      img_prefix_6 = "";
      img_suffix_7 = "";
    }
    if (J.t((J.has("<chart2code>", text_1)))) {
      out.push(J.str(J.add(J.add(img_prefix_6, chart2code_prompt_0), img_suffix_7)));
    } else if (J.t((J.has("<chart2csv>", text_1)))) {
      out.push(J.str(J.add(J.add(img_prefix_6, chart2csv_prompt_0), img_suffix_7)));
    } else if (J.t((J.has("<chart2summary>", text_1)))) {
      out.push(J.str(J.add(J.add(img_prefix_6, chart2summary_prompt_0), img_suffix_7)));
    } else if (J.t((J.has("<tables_json>", text_1)))) {
      out.push(J.str(J.add(J.add(img_prefix_6, tables_json_prompt_0), img_suffix_7)));
    } else if (J.t((J.has("<tables_html>", text_1)))) {
      out.push(J.str(J.add(J.add(img_prefix_6, tables_html_prompt_0), img_suffix_7)));
    } else if (J.t((J.has("<tables_otsl>", text_1)))) {
      out.push(J.str(J.add(J.add(img_prefix_6, tables_otsl_prompt_0), img_suffix_7)));
    } else {
      out.push(J.str(text_1));
    }
    return out.join("");
  };
  render_content_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_9 = k.caller;
    let x_8 = p.length > 0 ? p[0] : "x" in k ? k["x"] : undefined;
    if (J.t(J.test("string", x_8))) {
      out.push("        ", J.str(x_8));
    } else {
      {
        let _items1 = J.iter(x_8);
        const _loop1 = J.loop(_items1);
        for (let _i1 = 0; _i1 < _items1.length; _i1++) {
          _loop1.index0 = _i1; const _it1 = _items1[_i1]; let chunk_10 = _it1;
          if (J.t((J.eq(J.item(chunk_10, "type"), "text")))) {
            out.push(J.str(J.item(chunk_10, "text")));
          } else if (J.t((J.eq(J.item(chunk_10, "type"), "image")))) {
            out.push(J.str("<image>\n"));
          }
        }
      }
    }
    return out.join("");
  };
  tools_system_message_prefix_0 = "You are a helpful assistant with access to the following tools. You may call one or more tools to assist with the user query.\n\nYou are provided with function signatures within <tools></tools> XML tags:\n<tools>";
  tools_system_message_suffix_0 = "\n</tools>\n\nFor each tool call, return a json object with function name and arguments within <tool_call></tool_call> XML tags:\n<tool_call>\n{\"name\": <function-name>, \"arguments\": <args-json-object>}\n</tool_call>. If a tool does not exist in the provided list of tools, notify the user that you do not have the ability to fulfill the request.";
  documents_system_message_prefix_0 = "You are a helpful assistant with access to the following documents. You may use one or more documents to assist with the user query.\n\nYou are given a list of documents within <documents></documents> XML tags:\n<documents>";
  documents_system_message_suffix_0 = "\n</documents>\n\nWrite the response to the user's input by strictly aligning with the facts in the provided documents. If the information needed to answer the question is not available in the documents, inform the user that the question cannot be answered based on the available data.";
  g4_default_system_message_0 = "You are a helpful assistant. Please ensure responses are professional, accurate, and safe.";
  if (J.t(((v) => J.t(v) ? available_tools_0 : v)((available_tools_0 !== undefined)))) {
    tools_0 = available_tools_0;
  }
  ns_0 = J.call(J.namespace, J.kw({"tools_system_message": tools_system_message_prefix_0, "documents_system_message": documents_system_message_prefix_0, "default_system_message": g4_default_system_message_0, "system_message": ""}));
  if (J.t(tools_0)) {
    {
      let _items2 = J.iter(tools_0);
      const _loop2 = J.loop(_items2);
      for (let _i2 = 0; _i2 < _items2.length; _i2++) {
        _loop2.index0 = _i2; const _it2 = _items2[_i2]; let tool_11 = _it2;
        J.setattr(ns_0, "tools_system_message", J.add(J.add(J.attr(ns_0, "tools_system_message"), "\n"), J.filter("tojson", tool_11)));
      }
    }
    J.setattr(ns_0, "tools_system_message", J.add(J.attr(ns_0, "tools_system_message"), tools_system_message_suffix_0));
  } else {
    J.setattr(ns_0, "tools_system_message", "");
  }
  if (J.t(documents_0)) {
    {
      let _items3 = J.iter(documents_0);
      const _loop3 = J.loop(_items3);
      for (let _i3 = 0; _i3 < _items3.length; _i3++) {
        _loop3.index0 = _i3; const _it3 = _items3[_i3]; let document_12 = _it3;
        J.setattr(ns_0, "documents_system_message", J.add(J.add(J.attr(ns_0, "documents_system_message"), "\n"), J.filter("tojson", document_12)));
      }
    }
    J.setattr(ns_0, "documents_system_message", J.add(J.attr(ns_0, "documents_system_message"), documents_system_message_suffix_0));
  } else {
    J.setattr(ns_0, "documents_system_message", "");
  }
  if (J.t((J.eq(J.attr(J.item(messages_0, 0), "role"), "system")))) {
    if (J.t(J.test("string", J.attr(J.item(messages_0, 0), "content")))) {
      J.setattr(ns_0, "system_message", J.attr(J.item(messages_0, 0), "content"));
    } else if (J.t(J.test("iterable", J.attr(J.item(messages_0, 0), "content")))) {
      {
        let _items4 = J.iter(J.attr(J.item(messages_0, 0), "content"));
        const _loop4 = J.loop(_items4);
        for (let _i4 = 0; _i4 < _items4.length; _i4++) {
          _loop4.index0 = _i4; const _it4 = _items4[_i4]; let entry_13 = _it4;
          if (J.t((J.eq(J.attr(entry_13, "type"), "text")))) {
            if (J.t((!J.eq(J.attr(ns_0, "system_message"), "")))) {
              J.setattr(ns_0, "system_message", J.add(J.attr(ns_0, "system_message"), "\n"));
            }
            J.setattr(ns_0, "system_message", J.add(J.attr(ns_0, "system_message"), J.attr(entry_13, "text")));
          }
        }
      }
    }
    if (J.t(((v) => J.t(v) ? documents_0 : v)(tools_0))) {
      J.setattr(ns_0, "system_message", J.add(J.add(J.add(J.add(J.attr(ns_0, "system_message"), "\n\n"), J.attr(ns_0, "tools_system_message")), "\n\n"), J.attr(ns_0, "documents_system_message")));
    } else if (J.t(tools_0)) {
      J.setattr(ns_0, "system_message", J.add(J.add(J.attr(ns_0, "system_message"), "\n\n"), J.attr(ns_0, "tools_system_message")));
    } else if (J.t(documents_0)) {
      J.setattr(ns_0, "system_message", J.add(J.add(J.attr(ns_0, "system_message"), "\n\n"), J.attr(ns_0, "documents_system_message")));
    }
  } else {
    if (J.t(((v) => J.t(v) ? documents_0 : v)(tools_0))) {
      J.setattr(ns_0, "system_message", J.add(J.add(J.attr(ns_0, "tools_system_message"), "\n\n"), J.attr(ns_0, "documents_system_message")));
    } else if (J.t(tools_0)) {
      J.setattr(ns_0, "system_message", J.attr(ns_0, "tools_system_message"));
    } else if (J.t(documents_0)) {
      J.setattr(ns_0, "system_message", J.attr(ns_0, "documents_system_message"));
    }
  }
  if (J.t(J.attr(ns_0, "system_message"))) {
    out.push(J.str(J.add(J.add("<|start_of_role|>system<|end_of_role|>", J.attr(ns_0, "system_message")), "<|end_of_text|>\n")));
  } else {
    out.push(J.str(J.add(J.add("<|start_of_role|>system<|end_of_role|>", J.attr(ns_0, "default_system_message")), "<|end_of_text|>\n")));
  }
  {
    let _items5 = J.iter(messages_0);
    const _loop5 = J.loop(_items5);
    for (let _i5 = 0; _i5 < _items5.length; _i5++) {
      _loop5.index0 = _i5; const _it5 = _items5[_i5]; let message_14 = _it5;
      let content_15 = content_0;
      content_15 = J.call(J.namespace, J.kw({"val": ""}));
      if (J.t(J.test("string", J.call(render_content_0, J.item(message_14, "content"))))) {
        J.setattr(content_15, "val", J.call(render_content_0, J.item(message_14, "content")));
      } else {
        if (J.t(J.test("iterable", J.call(render_content_0, J.item(message_14, "content"))))) {
          {
            let _items6 = J.iter(J.call(render_content_0, J.item(message_14, "content")));
            const _loop6 = J.loop(_items6);
            for (let _i6 = 0; _i6 < _items6.length; _i6++) {
              _loop6.index0 = _i6; const _it6 = _items6[_i6]; let entry_16 = _it6;
              if (J.t((J.eq(J.attr(entry_16, "type"), "text")))) {
                if (J.t((!J.eq(J.attr(content_15, "val"), "")))) {
                  J.setattr(content_15, "val", J.add(J.attr(content_15, "val"), "\n"));
                }
                J.setattr(content_15, "val", J.add(J.attr(content_15, "val"), J.attr(entry_16, "text")));
              }
            }
          }
        }
      }
      if (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? !J.t(J.attr(_loop5, "first")) : v)((J.eq(J.attr(message_14, "role"), "system"))))((J.eq(J.attr(message_14, "role"), "user"))))) {
        out.push(J.str(J.add(J.add(J.add(J.add("<|start_of_role|>", J.attr(message_14, "role")), "<|end_of_role|>"), J.call(expand_tags_0, J.attr(content_15, "val"))), "<|end_of_text|>\n")));
      } else if (J.t((J.eq(J.attr(message_14, "role"), "assistant")))) {
        out.push(J.str(J.add(J.add(J.add("<|start_of_role|>", J.attr(message_14, "role")), "<|end_of_role|>"), J.attr(content_15, "val"))));
        if (J.t(J.attr(message_14, "tool_calls"))) {
          {
            let _items7 = J.iter(J.attr(message_14, "tool_calls"));
            const _loop7 = J.loop(_items7);
            for (let _i7 = 0; _i7 < _items7.length; _i7++) {
              _loop7.index0 = _i7; const _it7 = _items7[_i7]; let tool_call_17 = _it7;
              if (J.t(((v) => J.t(v) ? v : !J.t(J.attr(_loop7, "first")))(((v) => J.t(v) ? J.attr(content_15, "val") : v)(J.attr(_loop7, "first"))))) {
                out.push(J.str("\n"));
              }
              if (J.t(J.attr(tool_call_17, "function"))) {
                tool_call_17 = J.attr(tool_call_17, "function");
              }
              out.push(J.str("<tool_call>\n{\"name\": \""), J.str(J.attr(tool_call_17, "name")), J.str("\", \"arguments\": "));
              if (J.t(J.test("string", J.attr(tool_call_17, "arguments")))) {
                out.push(J.str(J.attr(tool_call_17, "arguments")));
              } else {
                out.push(J.str(J.filter("tojson", J.attr(tool_call_17, "arguments"))));
              }
              out.push(J.str("}\n</tool_call>"));
            }
          }
        }
        out.push(J.str("<|end_of_text|>\n"));
      } else if (J.t((J.eq(J.attr(message_14, "role"), "tool")))) {
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.sub(J.attr(_loop5, "index0"), 1)), "role"), "tool")))(J.attr(_loop5, "first")))) {
          out.push(J.str("<|start_of_role|>user<|end_of_role|>"));
        }
        out.push(J.str("\n<tool_response>\n"), J.str(J.attr(content_15, "val")), J.str("\n</tool_response>"));
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.add(J.attr(_loop5, "index0"), 1)), "role"), "tool")))(J.attr(_loop5, "last")))) {
          out.push(J.str("<|end_of_text|>\n"));
        }
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|start_of_role|>assistant<|end_of_role|>"));
  }
  return out.join("");
}
