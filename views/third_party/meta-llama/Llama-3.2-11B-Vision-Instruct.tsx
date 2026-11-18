// Converted from the chat template of meta-llama/Llama-3.2-11B-Vision-Instruct by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let tools_0 = ctx["tools"] ?? null;
  let tools_in_user_message_0 = ctx["tools_in_user_message"];
  let date_string_0 = ctx["date_string"];
  let system_message_0 = ctx["system_message"];
  let messages_0 = ctx["messages"];
  let user_supplied_system_message_0 = ctx["user_supplied_system_message"];
  let image_ns_0 = ctx["image_ns"];
  let first_user_message_0 = ctx["first_user_message"];
  let bos_token_0 = ctx["bos_token"];
  let custom_tools_0 = ctx["custom_tools"];
  let tool_call_0 = ctx["tool_call"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  out.push(J.str(bos_token_0));
  if (J.t((custom_tools_0 !== undefined))) {
    tools_0 = custom_tools_0;
  }
  if (J.t(!J.t((tools_in_user_message_0 !== undefined)))) {
    tools_in_user_message_0 = true;
  }
  if (J.t(!J.t((date_string_0 !== undefined)))) {
    if (J.t((J.strftime_now !== undefined))) {
      date_string_0 = J.call(J.strftime_now, "%d %b %Y");
    } else {
      date_string_0 = "26 Jul 2024";
    }
  }
  if (J.t(!J.t((tools_0 !== undefined)))) {
    tools_0 = null;
  }
  if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
    system_message_0 = J.filter("trim", J.item(J.item(messages_0, 0), "content"));
    messages_0 = J.slice(messages_0, 1, undefined, undefined);
    user_supplied_system_message_0 = true;
  } else {
    system_message_0 = "";
    user_supplied_system_message_0 = false;
  }
  image_ns_0 = J.call(J.namespace, J.kw({"has_images": false}));
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      {
        let _items2 = J.iter(J.item(message_1, "content"));
        const _loop2 = J.loop(_items2);
        for (let _i2 = 0; _i2 < _items2.length; _i2++) {
          _loop2.index0 = _i2; const _it2 = _items2[_i2]; let content_2 = _it2;
          if (J.t((J.eq(J.item(content_2, "type"), "image")))) {
            J.setattr(image_ns_0, "has_images", true);
          }
        }
      }
    }
  }
  if (J.t(((v) => J.t(v) ? v : !J.t(J.attr(image_ns_0, "has_images")))(user_supplied_system_message_0))) {
    out.push(J.str("<|start_header_id|>system<|end_header_id|>\n\n"));
    if (J.t(!J.t((tools_0 === null)))) {
      out.push(J.str("Environment: ipython\n"));
    }
    out.push(J.str("Cutting Knowledge Date: December 2023\n"), J.str(J.add(J.add("Today Date: ", date_string_0), "\n\n")));
    if (J.t(((v) => J.t(v) ? !J.t(tools_in_user_message_0) : v)(!J.t((tools_0 === null))))) {
      out.push(J.str("You have access to the following functions. To call a function, please respond with JSON for a function call."), J.str("Respond in the format {\"name\": function name, \"parameters\": dictionary of argument name and its value}."), J.str("Do not use variables.\n\n"));
      {
        let _items3 = J.iter(tools_0);
        const _loop3 = J.loop(_items3);
        for (let _i3 = 0; _i3 < _items3.length; _i3++) {
          _loop3.index0 = _i3; const _it3 = _items3[_i3]; let t_3 = _it3;
          out.push(J.str(J.filter("tojson", t_3, J.kw({"indent": 4}))), J.str("\n\n"));
        }
      }
    }
    out.push(J.str(system_message_0), J.str("<|eot_id|>"));
  }
  if (J.t(((v) => J.t(v) ? !J.t((tools_0 === null)) : v)(tools_in_user_message_0))) {
    if (J.t((!J.eq(J.filter("length", messages_0), 0)))) {
      first_user_message_0 = J.filter("trim", J.item(J.item(messages_0, 0), "content"));
      messages_0 = J.slice(messages_0, 1, undefined, undefined);
    } else {
      out.push(J.str(J.call(J.raise_exception, "Cannot put tools in the first user message when there's no first user message!")));
    }
    out.push(J.str("<|start_header_id|>user<|end_header_id|>\n\n"), J.str("Given the following functions, please respond with a JSON for a function call "), J.str("with its proper arguments that best answers the given prompt.\n\n"), J.str("Respond in the format {\"name\": function name, \"parameters\": dictionary of argument name and its value}."), J.str("Do not use variables.\n\n"));
    {
      let _items4 = J.iter(tools_0);
      const _loop4 = J.loop(_items4);
      for (let _i4 = 0; _i4 < _items4.length; _i4++) {
        _loop4.index0 = _i4; const _it4 = _items4[_i4]; let t_4 = _it4;
        out.push(J.str(J.filter("tojson", t_4, J.kw({"indent": 4}))), J.str("\n\n"));
      }
    }
    out.push(J.str(J.add(first_user_message_0, "<|eot_id|>")));
  }
  {
    let _items5 = J.iter(messages_0);
    const _loop5 = J.loop(_items5);
    for (let _i5 = 0; _i5 < _items5.length; _i5++) {
      _loop5.index0 = _i5; const _it5 = _items5[_i5]; let message_5 = _it5;
      let tool_call_6 = tool_call_0;
      if (J.t(!J.t(((v) => J.t(v) ? v : (J.has("tool_calls", message_5)))(((v) => J.t(v) ? v : (J.eq(J.attr(message_5, "role"), "tool")))((J.eq(J.attr(message_5, "role"), "ipython"))))))) {
        out.push(J.str(J.add(J.add("<|start_header_id|>", J.item(message_5, "role")), "<|end_header_id|>\n\n")));
        if (J.t(J.test("string", J.item(message_5, "content")))) {
          out.push(J.str(J.item(message_5, "content")));
        } else {
          {
            let _items6 = J.iter(J.item(message_5, "content"));
            const _loop6 = J.loop(_items6);
            for (let _i6 = 0; _i6 < _items6.length; _i6++) {
              _loop6.index0 = _i6; const _it6 = _items6[_i6]; let content_7 = _it6;
              if (J.t((J.eq(J.item(content_7, "type"), "image")))) {
                out.push(J.str("<|image|>"));
              } else if (J.t((J.eq(J.item(content_7, "type"), "text")))) {
                out.push(J.str(J.item(content_7, "text")));
              }
            }
          }
        }
        out.push(J.str("<|eot_id|>"));
      } else if (J.t((J.has("tool_calls", message_5)))) {
        if (J.t(!J.t((J.eq(J.filter("length", J.attr(message_5, "tool_calls")), 1))))) {
          out.push(J.str(J.call(J.raise_exception, "This model only supports single tool-calls at once!")));
        }
        tool_call_6 = J.attr(J.item(J.attr(message_5, "tool_calls"), 0), "function");
        out.push(J.str("<|start_header_id|>assistant<|end_header_id|>\n\n"), J.str(J.add(J.add("{\"name\": \"", J.attr(tool_call_6, "name")), "\", ")), J.str("\"parameters\": "), J.str(J.filter("tojson", J.attr(tool_call_6, "arguments"))), J.str("}"), J.str("<|eot_id|>"));
      } else if (J.t(((v) => J.t(v) ? v : (J.eq(J.attr(message_5, "role"), "ipython")))((J.eq(J.attr(message_5, "role"), "tool"))))) {
        out.push(J.str("<|start_header_id|>ipython<|end_header_id|>\n\n"));
        if (J.t(((v) => J.t(v) ? v : J.test("iterable", J.attr(message_5, "content")))(J.test("mapping", J.attr(message_5, "content"))))) {
          out.push(J.str(J.filter("tojson", J.attr(message_5, "content"))));
        } else {
          out.push(J.str(J.attr(message_5, "content")));
        }
        out.push(J.str("<|eot_id|>"));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|start_header_id|>assistant<|end_header_id|>\n\n"));
  }
  return out.join("");
}
