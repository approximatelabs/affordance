// Converted from the chat template of stepfun-ai/step3 by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let render_content_0 = ctx["render_content"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let ns_0 = ctx["ns"];
  let tool_schemas_var_0 = ctx["tool_schemas_var"];
  let bos_token_0 = ctx["bos_token"];
  let messages_0 = ctx["messages"];
  let tools_0 = ctx["tools"] ?? null;
  render_content_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_2 = k.caller;
    let content_1 = p.length > 0 ? p[0] : "content" in k ? k["content"] : undefined;
    out.push(" ");
    if (J.t(J.test("string", content_1))) {
      out.push(J.str(content_1));
    } else if (J.t(J.test("mapping", content_1))) {
      out.push(J.str((J.t((J.has("value", content_1))) ? J.item(content_1, "value") : J.item(content_1, "text"))));
    } else if (J.t(J.test("iterable", content_1))) {
      {
        let _items1 = J.iter(content_1);
        const _loop1 = J.loop(_items1);
        for (let _i1 = 0; _i1 < _items1.length; _i1++) {
          _loop1.index0 = _i1; const _it1 = _items1[_i1]; let item_3 = _it1;
          if (J.t((J.eq(J.attr(item_3, "type"), "text")))) {
            out.push(J.str((J.t((J.has("value", item_3))) ? J.item(item_3, "value") : J.item(item_3, "text"))));
          } else if (J.t((J.eq(J.attr(item_3, "type"), "image")))) {
            out.push("<im_patch>");
          }
        }
      }
    }
    out.push(" ");
    return out.join("");
  };
  if (J.t(!J.t((add_generation_prompt_0 !== undefined)))) {
    add_generation_prompt_0 = false;
  }
  out.push(J.str(bos_token_0));
  {
    let _items2 = J.iter(messages_0);
    const _loop2 = J.loop(_items2);
    for (let _i2 = 0; _i2 < _items2.length; _i2++) {
      _loop2.index0 = _i2; const _it2 = _items2[_i2]; let message_4 = _it2;
      if (J.t((J.eq(J.attr(message_4, "role"), "system")))) {
        out.push(J.str(J.call(render_content_0, J.item(message_4, "content"))));
      }
    }
  }
  if (J.t(((v) => J.t(v) ? tools_0 : v)((tools_0 !== undefined)))) {
    ns_0 = J.call(J.namespace, J.kw({"data": ""}));
    {
      let _items3 = J.iter(tools_0);
      const _loop3 = J.loop(_items3);
      for (let _i3 = 0; _i3 < _items3.length; _i3++) {
        _loop3.index0 = _i3; const _it3 = _items3[_i3]; let tool_5 = _it3;
        J.setattr(ns_0, "data", J.add(J.add(J.attr(ns_0, "data"), J.filter("tojson", tool_5, J.kw({"ensure_ascii": false}))), "\n"));
      }
    }
    tool_schemas_var_0 = J.attr(ns_0, "data");
    out.push("# Tools \nYou may call one or more tools to assist with the user query. You are provided with tool schemas within <tools></tools> XML tags: <tools>", J.str(tool_schemas_var_0), "</tools> When making tool calls, use XML format to invoke tools and pass parameters: <\uff5ctool_calls_begin\uff5c>\n<\uff5ctool_call_begin\uff5c>\nfunction<\uff5ctool_sep\uff5c><steptml:invoke name=\"tool_name0\"><steptml:parameter name=\"parameter_name0\">[parameter value]</steptml:parameter>...</steptml:invoke><\uff5ctool_call_end\uff5c>\n<\uff5ctool_call_begin\uff5c>\nfunction<\uff5ctool_sep\uff5c><steptml:invoke name=\"tool_name1\"><steptml:parameter name=\"parameter_name1\">[parameter value]</steptml:parameter>...</steptml:invoke><\uff5ctool_call_end\uff5c>\n<\uff5ctool_calls_end\uff5c>\nNote: * You can invoke one or more tools in parallel. * Each tool call must be complete and self-contained within a single <steptml:toolcall></steptml:toolcall> block. ");
  }
  {
    let _items4 = J.iter(messages_0);
    const _loop4 = J.loop(_items4);
    for (let _i4 = 0; _i4 < _items4.length; _i4++) {
      _loop4.index0 = _i4; const _it4 = _items4[_i4]; let message_6 = _it4;
      if (J.t((J.eq(J.attr(message_6, "role"), "tool_description")))) {
        out.push(J.str(J.call(render_content_0, J.item(message_6, "content"))));
      } else if (J.t((J.eq(J.attr(message_6, "role"), "user")))) {
        out.push(J.str(J.add(J.add(J.add("<|BOT|>", J.attr(message_6, "role")), "\n"), J.call(render_content_0, J.item(message_6, "content")))), J.str("<|EOT|>"));
      } else if (J.t(((v) => J.t(v) ? v : (J.eq(J.attr(message_6, "role"), "tools")))((J.eq(J.attr(message_6, "role"), "tool_response"))))) {
        out.push("<\uff5ctool_outputs_begin\uff5c>\n");
        if (J.t(J.test("string", J.item(message_6, "content")))) {
          out.push(J.str(J.add(J.add("<\uff5ctool_output_begin\uff5c>\n", J.item(message_6, "content")), "<\uff5ctool_output_end\uff5c>")));
        } else {
          {
            let _items5 = J.iter(J.item(message_6, "content"));
            const _loop5 = J.loop(_items5);
            for (let _i5 = 0; _i5 < _items5.length; _i5++) {
              _loop5.index0 = _i5; const _it5 = _items5[_i5]; let tool_output_7 = _it5;
              out.push("<\uff5ctool_output_begin\uff5c>\n", J.str(J.call(render_content_0, tool_output_7)), "<\uff5ctool_output_end\uff5c>");
            }
          }
        }
        out.push("<\uff5ctool_outputs_end\uff5c>\n");
      } else if (J.t((!J.eq(J.attr(message_6, "role"), "system")))) {
        out.push(J.str(J.add(J.add("<|BOT|>", J.attr(message_6, "role")), "\n")));
        if (J.t((J.item(message_6, "content") !== undefined))) {
          out.push(J.str(J.call(render_content_0, J.item(message_6, "content"))));
        }
        if (J.t((J.attr(message_6, "tool_calls") !== undefined))) {
          out.push("<\uff5ctool_calls_begin\uff5c>\n");
          {
            let _items6 = J.iter(J.attr(message_6, "tool_calls"));
            const _loop6 = J.loop(_items6);
            for (let _i6 = 0; _i6 < _items6.length; _i6++) {
              _loop6.index0 = _i6; const _it6 = _items6[_i6]; let tool_8 = _it6;
              out.push("<\uff5ctool_call_begin>\uff5c>\n", J.str(J.item(tool_8, "type")), "<\uff5ctool_sep\uff5c>", J.str(J.add(J.add("<steptml:invoke name=\"", J.item(J.item(tool_8, "function"), "name")), "\">")));
              {
                let _items7 = J.iter(J.call(J.attr(J.item(J.item(tool_8, "function"), "arguments"), "items")));
                const _loop7 = J.loop(_items7);
                for (let _i7 = 0; _i7 < _items7.length; _i7++) {
                  _loop7.index0 = _i7; const _it7 = _items7[_i7]; let [name_9, param_10] = J.unpack(_it7, 2);
                  out.push(J.str(J.add(J.add(J.add(J.add("<steptml:parameter name=\"", name_9), "\">"), J.filter("string", param_10)), "</steptml:parameter>")));
                }
              }
              out.push("</steptml:invoke><\uff5ctool_call_end\uff5c>\n");
            }
          }
          out.push("<\uff5ctool_calls_end\uff5c>\n");
        }
        out.push("<|EOT|>");
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|BOT|>assistant\n<think>\n"));
  }
  return out.join("");
}
