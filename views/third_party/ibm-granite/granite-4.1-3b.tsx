// Converted from the chat template of ibm-granite/granite-4.1-3b by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let tools_system_message_prefix_0 = ctx["tools_system_message_prefix"];
  let tools_system_message_suffix_0 = ctx["tools_system_message_suffix"];
  let documents_system_message_prefix_0 = ctx["documents_system_message_prefix"];
  let documents_system_message_suffix_0 = ctx["documents_system_message_suffix"];
  let tools_0 = ctx["tools"] ?? null;
  let ns_0 = ctx["ns"];
  let available_tools_0 = ctx["available_tools"];
  let documents_0 = ctx["documents"] ?? null;
  let messages_0 = ctx["messages"];
  let content_0 = ctx["content"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  tools_system_message_prefix_0 = "You are a helpful assistant with access to the following tools. You may call one or more tools to assist with the user query.\n\nYou are provided with function signatures within <tools></tools> XML tags:\n<tools>";
  tools_system_message_suffix_0 = "\n</tools>\n\nFor each tool call, return a json object with function name and arguments within <tool_call></tool_call> XML tags:\n<tool_call>\n{\"name\": <function-name>, \"arguments\": <args-json-object>}\n</tool_call>. If a tool does not exist in the provided list of tools, notify the user that you do not have the ability to fulfill the request.";
  documents_system_message_prefix_0 = "You are a helpful assistant with access to the following documents. You may use one or more documents to assist with the user query.\n\nYou are given a list of documents within <documents></documents> XML tags:\n<documents>";
  documents_system_message_suffix_0 = "\n</documents>\n\nWrite the response to the user's input by strictly aligning with the facts in the provided documents. If the information needed to answer the question is not available in the documents, inform the user that the question cannot be answered based on the available data.";
  if (J.t(((v) => J.t(v) ? available_tools_0 : v)((available_tools_0 !== undefined)))) {
    tools_0 = available_tools_0;
  }
  ns_0 = J.call(J.namespace, J.kw({"tools_system_message": tools_system_message_prefix_0, "documents_system_message": documents_system_message_prefix_0, "system_message": ""}));
  if (J.t(tools_0)) {
    {
      let _items1 = J.iter(tools_0);
      const _loop1 = J.loop(_items1);
      for (let _i1 = 0; _i1 < _items1.length; _i1++) {
        _loop1.index0 = _i1; const _it1 = _items1[_i1]; let tool_1 = _it1;
        J.setattr(ns_0, "tools_system_message", J.add(J.add(J.attr(ns_0, "tools_system_message"), "\n"), J.filter("tojson", tool_1)));
      }
    }
    J.setattr(ns_0, "tools_system_message", J.add(J.attr(ns_0, "tools_system_message"), tools_system_message_suffix_0));
  } else {
    J.setattr(ns_0, "tools_system_message", "");
  }
  if (J.t(documents_0)) {
    {
      let _items2 = J.iter(documents_0);
      const _loop2 = J.loop(_items2);
      for (let _i2 = 0; _i2 < _items2.length; _i2++) {
        _loop2.index0 = _i2; const _it2 = _items2[_i2]; let document_2 = _it2;
        J.setattr(ns_0, "documents_system_message", J.add(J.add(J.attr(ns_0, "documents_system_message"), "\n"), J.filter("tojson", document_2)));
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
        let _items3 = J.iter(J.attr(J.item(messages_0, 0), "content"));
        const _loop3 = J.loop(_items3);
        for (let _i3 = 0; _i3 < _items3.length; _i3++) {
          _loop3.index0 = _i3; const _it3 = _items3[_i3]; let entry_3 = _it3;
          if (J.t((J.eq(J.attr(entry_3, "type"), "text")))) {
            if (J.t((!J.eq(J.attr(ns_0, "system_message"), "")))) {
              J.setattr(ns_0, "system_message", J.add(J.attr(ns_0, "system_message"), "\n"));
            }
            J.setattr(ns_0, "system_message", J.add(J.attr(ns_0, "system_message"), J.attr(entry_3, "text")));
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
  }
  {
    let _items4 = J.iter(messages_0);
    const _loop4 = J.loop(_items4);
    for (let _i4 = 0; _i4 < _items4.length; _i4++) {
      _loop4.index0 = _i4; const _it4 = _items4[_i4]; let message_4 = _it4;
      let content_5 = content_0;
      content_5 = J.call(J.namespace, J.kw({"val": ""}));
      if (J.t(J.test("string", J.attr(message_4, "content")))) {
        J.setattr(content_5, "val", J.attr(message_4, "content"));
      } else {
        if (J.t(J.test("iterable", J.attr(message_4, "content")))) {
          {
            let _items5 = J.iter(J.attr(message_4, "content"));
            const _loop5 = J.loop(_items5);
            for (let _i5 = 0; _i5 < _items5.length; _i5++) {
              _loop5.index0 = _i5; const _it5 = _items5[_i5]; let entry_6 = _it5;
              if (J.t((J.eq(J.attr(entry_6, "type"), "text")))) {
                if (J.t((!J.eq(J.attr(content_5, "val"), "")))) {
                  J.setattr(content_5, "val", J.add(J.attr(content_5, "val"), "\n"));
                }
                J.setattr(content_5, "val", J.add(J.attr(content_5, "val"), J.attr(entry_6, "text")));
              }
            }
          }
        }
      }
      if (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? !J.t(J.attr(_loop4, "first")) : v)((J.eq(J.attr(message_4, "role"), "system"))))((J.eq(J.attr(message_4, "role"), "user"))))) {
        out.push(J.str(J.add(J.add(J.add(J.add("<|start_of_role|>", J.attr(message_4, "role")), "<|end_of_role|>"), J.attr(content_5, "val")), "<|end_of_text|>\n")));
      } else if (J.t((J.eq(J.attr(message_4, "role"), "assistant")))) {
        out.push(J.str(J.add(J.add(J.add("<|start_of_role|>", J.attr(message_4, "role")), "<|end_of_role|>"), J.attr(content_5, "val"))));
        if (J.t(J.attr(message_4, "tool_calls"))) {
          {
            let _items6 = J.iter(J.attr(message_4, "tool_calls"));
            const _loop6 = J.loop(_items6);
            for (let _i6 = 0; _i6 < _items6.length; _i6++) {
              _loop6.index0 = _i6; const _it6 = _items6[_i6]; let tool_call_7 = _it6;
              if (J.t(((v) => J.t(v) ? v : !J.t(J.attr(_loop6, "first")))(((v) => J.t(v) ? J.attr(content_5, "val") : v)(J.attr(_loop6, "first"))))) {
                out.push(J.str("\n"));
              }
              if (J.t(J.attr(tool_call_7, "function"))) {
                tool_call_7 = J.attr(tool_call_7, "function");
              }
              out.push(J.str("<tool_call>\n{\"name\": \""), J.str(J.attr(tool_call_7, "name")), J.str("\", \"arguments\": "));
              if (J.t(J.test("string", J.attr(tool_call_7, "arguments")))) {
                out.push(J.str(J.attr(tool_call_7, "arguments")));
              } else {
                out.push(J.str(J.filter("tojson", J.attr(tool_call_7, "arguments"))));
              }
              out.push(J.str("}\n</tool_call>"));
            }
          }
        }
        out.push(J.str("<|end_of_text|>\n"));
      } else if (J.t((J.eq(J.attr(message_4, "role"), "tool")))) {
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.sub(J.attr(_loop4, "index0"), 1)), "role"), "tool")))(J.attr(_loop4, "first")))) {
          out.push(J.str("<|start_of_role|>user<|end_of_role|>"));
        }
        out.push(J.str("\n<tool_response>\n"), J.str(J.attr(content_5, "val")), J.str("\n</tool_response>"));
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.add(J.attr(_loop4, "index0"), 1)), "role"), "tool")))(J.attr(_loop4, "last")))) {
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
