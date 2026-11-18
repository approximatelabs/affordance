// Converted from the chat template of XHToken/Spark-X2.5-4B by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let enable_thinking_0 = ctx["enable_thinking"];
  let render_content_0 = ctx["render_content"];
  let default_system_0 = ctx["default_system"];
  let ns_0 = ctx["ns"];
  let messages_0 = ctx["messages"];
  let tools_0 = ctx["tools"] ?? null;
  let assistant_content_0 = ctx["assistant_content"];
  let reasoning_content_0 = ctx["reasoning_content"];
  let args_0 = ctx["args"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  if (J.t(!J.t(messages_0))) {
    out.push(J.str(J.call(J.raise_exception, "No messages provided.")));
  }
  enable_thinking_0 = J.filter("default", enable_thinking_0, true);
  render_content_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_3 = k.caller;
    let content_1 = p.length > 0 ? p[0] : "content" in k ? k["content"] : undefined;
    let context_name_2 = p.length > 1 ? p[1] : "context_name" in k ? k["context_name"] : undefined;
    if (J.t(J.test("string", content_1))) {
      out.push(J.str(content_1));
    } else if (J.t(((v) => J.t(v) ? v : (content_1 === undefined))((content_1 === null)))) {
      out.push(J.str(""));
    } else if (J.t(((v) => J.t(v) ? !J.t(J.test("mapping", content_1)) : v)(J.test("iterable", content_1)))) {
      {
        let _items1 = J.iter(content_1);
        const _loop1 = J.loop(_items1);
        for (let _i1 = 0; _i1 < _items1.length; _i1++) {
          _loop1.index0 = _i1; const _it1 = _items1[_i1]; let block_4 = _it1;
          if (J.t((J.eq(J.attr(block_4, "type"), "text")))) {
            out.push(J.str(J.attr(block_4, "text")));
          } else {
            out.push(J.str(J.call(J.raise_exception, J.concat("Unsupported ", context_name_2, " content block type: ", J.filter("string", J.attr(block_4, "type"))))));
          }
        }
      }
    } else {
      out.push(J.str(J.call(J.raise_exception, J.concat(context_name_2, " content must be a string or a list of text blocks"))));
    }
    return out.join("");
  };
  default_system_0 = "you are a helpful assistant.";
  ns_0 = J.call(J.namespace, J.kw({"initial_system": ""}));
  if (J.t((J.eq(J.attr(J.item(messages_0, 0), "role"), "system")))) {
    J.setattr(ns_0, "initial_system", J.call(render_content_0, J.attr(J.item(messages_0, 0), "content"), "system"));
  }
  out.push(J.str(J.add(J.add("<\uff5cstart\u2581of\u2581sentence\uff5c><|System|>", "\n"), default_system_0)));
  if (J.t(tools_0)) {
    out.push(J.str(J.add(J.add(J.add(J.add("## Tools", "\n"), "You have access to the following functions:"), "\n"), "<tools>")));
    {
      let _items2 = J.iter(tools_0);
      const _loop2 = J.loop(_items2);
      for (let _i2 = 0; _i2 < _items2.length; _i2++) {
        _loop2.index0 = _i2; const _it2 = _items2[_i2]; let tool_5 = _it2;
        out.push(J.str(J.add("\n", J.filter("tojson", J.attr(tool_5, "function")))));
      }
    }
    out.push(J.str(J.add("\n", "</tools>")));
  }
  if (J.t(J.attr(ns_0, "initial_system"))) {
    out.push(J.str(J.add("\n\n", J.attr(ns_0, "initial_system"))));
  }
  out.push(J.str("<\uff5cend\u2581of\u2581sentence\uff5c>"));
  {
    let _items3 = J.iter(messages_0);
    const _loop3 = J.loop(_items3);
    for (let _i3 = 0; _i3 < _items3.length; _i3++) {
      _loop3.index0 = _i3; const _it3 = _items3[_i3]; let message_6 = _it3;
      let assistant_content_7 = assistant_content_0;
      let reasoning_content_8 = reasoning_content_0;
      if (J.t((J.eq(J.attr(message_6, "role"), "system")))) {
        if (J.t(!J.t(J.attr(_loop3, "first")))) {
          out.push(J.str(J.add(J.add("<\uff5cstart\u2581of\u2581sentence\uff5c><|System|>\n", J.call(render_content_0, J.attr(message_6, "content"), "system")), "<\uff5cend\u2581of\u2581sentence\uff5c>")));
        }
      } else if (J.t((J.eq(J.attr(message_6, "role"), "user")))) {
        out.push(J.str(J.add(J.add("<\uff5cstart\u2581of\u2581sentence\uff5c><|User|>", J.call(render_content_0, J.attr(message_6, "content"), "user")), "<\uff5cend\u2581of\u2581sentence\uff5c>")));
      } else if (J.t((J.eq(J.attr(message_6, "role"), "assistant")))) {
        assistant_content_7 = J.call(render_content_0, J.attr(message_6, "content"), "assistant");
        if (J.t(((v) => J.t(v) ? J.attr(message_6, "reasoning_content") : v)((J.attr(message_6, "reasoning_content") !== undefined)))) {
          reasoning_content_8 = J.attr(message_6, "reasoning_content");
        } else {
          reasoning_content_8 = "";
        }
        out.push(J.str("<\uff5cstart\u2581of\u2581sentence\uff5c><|Bot|>"));
        if (J.t(reasoning_content_8)) {
          out.push(J.str(J.add(J.add("<think>", reasoning_content_8), "</think>")));
        } else {
          out.push(J.str("</think>"));
        }
        if (J.t(assistant_content_7)) {
          out.push(J.str(assistant_content_7));
        }
        if (J.t(((v) => J.t(v) ? !J.t((J.attr(message_6, "tool_calls") === null)) : v)((J.attr(message_6, "tool_calls") !== undefined)))) {
          {
            let _items4 = J.iter(J.attr(message_6, "tool_calls"));
            const _loop4 = J.loop(_items4);
            for (let _i4 = 0; _i4 < _items4.length; _i4++) {
              _loop4.index0 = _i4; const _it4 = _items4[_i4]; let tool_call_9 = _it4;
              let args_10 = args_0;
              if (J.t(!J.t(J.test("mapping", J.attr(J.attr(tool_call_9, "function"), "arguments"))))) {
                out.push(J.str(J.call(J.raise_exception, "tool_call.function.arguments must be a dictionary; normalize JSON strings before apply_chat_template")));
              }
              args_10 = J.attr(J.attr(tool_call_9, "function"), "arguments");
              out.push(J.str(J.add("<tool_call>", J.attr(J.attr(tool_call_9, "function"), "name"))));
              {
                let _items5 = J.iter(J.call(J.attr(args_10, "items")));
                const _loop5 = J.loop(_items5);
                for (let _i5 = 0; _i5 < _items5.length; _i5++) {
                  _loop5.index0 = _i5; const _it5 = _items5[_i5]; let [k_11, v_12] = J.unpack(_it5, 2);
                  out.push(J.str(J.concat("<arg_key>", k_11, "</arg_key><arg_value>", (J.t(J.test("string", v_12)) ? v_12 : J.filter("tojson", v_12)), "</arg_value>")));
                }
              }
              out.push(J.str("</tool_call>"));
            }
          }
        }
        out.push(J.str("<\uff5cend\u2581of\u2581sentence\uff5c>"));
      } else if (J.t((J.eq(J.attr(message_6, "role"), "tool")))) {
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.attr(_loop3, "previtem"), "role"), "tool")))((J.attr(_loop3, "previtem") === undefined)))) {
          out.push(J.str("<\uff5cstart\u2581of\u2581sentence\uff5c><|Tool|>"));
        }
        out.push(J.str(J.concat("<tool_response>", J.attr(message_6, "content"), "</tool_response>")));
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.attr(_loop3, "nextitem"), "role"), "tool")))((J.attr(_loop3, "nextitem") === undefined)))) {
          out.push(J.str("<\uff5cend\u2581of\u2581sentence\uff5c>"));
        }
      } else {
        out.push(J.str(J.call(J.raise_exception, J.concat("Unsupported message role: ", J.attr(message_6, "role")))));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<\uff5cstart\u2581of\u2581sentence\uff5c><|Bot|>"));
    if (J.t(((v) => J.t(v) ? enable_thinking_0 : v)((enable_thinking_0 !== undefined)))) {
      out.push(J.str("<think>"));
    }
    if (J.t(((v) => J.t(v) ? !J.t(enable_thinking_0) : v)((enable_thinking_0 !== undefined)))) {
      out.push(J.str("</think>"));
    }
  }
  return out.join("");
}
