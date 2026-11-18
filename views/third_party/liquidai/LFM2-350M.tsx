// Converted from the chat template of LiquidAI/LFM2-350M by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let format_arg_value_0 = ctx["format_arg_value"];
  let render_tool_calls_0 = ctx["render_tool_calls"];
  let system_prompt_0 = ctx["system_prompt"];
  let ns_0 = ctx["ns"];
  let messages_0 = ctx["messages"];
  let bos_token_0 = ctx["bos_token"];
  let tool_calls_ns_0 = ctx["tool_calls_ns"];
  let func_0 = ctx["func"];
  let func_name_0 = ctx["func_name"];
  let func_args_0 = ctx["func_args"];
  let args_ns_0 = ctx["args_ns"];
  let tools_0 = ctx["tools"] ?? null;
  let content_0 = ctx["content"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  out.push(J.str(bos_token_0));
  format_arg_value_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_2 = k.caller;
    let arg_value_1 = p.length > 0 ? p[0] : "arg_value" in k ? k["arg_value"] : undefined;
    if (J.t(J.test("string", arg_value_1))) {
      out.push(J.str(J.add(J.add("'", J.filter("replace", J.filter("replace", J.filter("replace", J.filter("replace", arg_value_1, "\\", "\\\\"), "'", "\\'"), "\n", "\\n"), "\r", "\\r")), "'")));
    } else if (J.t(((v) => J.t(v) ? v : J.test("iterable", arg_value_1))(J.test("mapping", arg_value_1)))) {
      out.push(J.str(J.filter("tojson", arg_value_1)));
    } else {
      out.push(J.str(J.filter("string", arg_value_1)));
    }
    return out.join("");
  };
  render_tool_calls_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_4 = k.caller;
    let tool_calls_3 = p.length > 0 ? p[0] : "tool_calls" in k ? k["tool_calls"] : undefined;
    let tool_calls_ns_5 = tool_calls_ns_0;
    tool_calls_ns_5 = J.call(J.namespace, J.kw({"tool_calls": []}));
    {
      let _items1 = J.iter(tool_calls_3);
      const _loop1 = J.loop(_items1);
      for (let _i1 = 0; _i1 < _items1.length; _i1++) {
        _loop1.index0 = _i1; const _it1 = _items1[_i1]; let tool_call_6 = _it1;
        let func_7 = func_0;
        let func_name_8 = func_name_0;
        let func_args_9 = func_args_0;
        let args_ns_10 = args_ns_0;
        func_7 = (J.t((J.has("function", tool_call_6))) ? J.item(tool_call_6, "function") : tool_call_6);
        func_name_8 = J.item(func_7, "name");
        func_args_9 = J.call(J.attr(func_7, "get"), "arguments");
        args_ns_10 = J.call(J.namespace, J.kw({"arg_strings": []}));
        if (J.t(J.test("mapping", func_args_9))) {
          {
            let _items2 = J.iter(J.call(J.attr(func_args_9, "items")));
            const _loop2 = J.loop(_items2);
            for (let _i2 = 0; _i2 < _items2.length; _i2++) {
              _loop2.index0 = _i2; const _it2 = _items2[_i2]; let [arg_name_11, arg_value_12] = J.unpack(_it2, 2);
              J.setattr(args_ns_10, "arg_strings", J.add(J.attr(args_ns_10, "arg_strings"), [J.add(J.add(arg_name_11, "="), J.call(format_arg_value_0, arg_value_12))]));
            }
          }
        } else if (J.t(((v) => J.t(v) ? (!J.has(J.filter("trim", func_args_9), ["", "{}", "null"])) : v)(J.test("string", func_args_9)))) {
          out.push(J.str(J.call(J.raise_exception, "Tool call arguments must be a mapping, got a JSON-encoded string: parse arguments with json.loads() before applying the chat template")));
        }
        J.setattr(tool_calls_ns_5, "tool_calls", J.add(J.attr(tool_calls_ns_5, "tool_calls"), [J.add(J.add(J.add(func_name_8, "("), J.filter("join", J.attr(args_ns_10, "arg_strings"), ", ")), ")")]));
      }
    }
    out.push(J.str(J.add(J.add("<|tool_call_start|>[", J.filter("join", J.attr(tool_calls_ns_5, "tool_calls"), ", ")), "]<|tool_call_end|>")));
    return out.join("");
  };
  system_prompt_0 = "";
  ns_0 = J.call(J.namespace, J.kw({"system_prompt": ""}));
  if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
    J.setattr(ns_0, "system_prompt", J.item(J.item(messages_0, 0), "content"));
    messages_0 = J.slice(messages_0, 1, undefined, undefined);
  }
  if (J.t(tools_0)) {
    J.setattr(ns_0, "system_prompt", J.add(J.add(J.attr(ns_0, "system_prompt"), (J.t(J.attr(ns_0, "system_prompt")) ? "\n" : "")), "List of tools: <|tool_list_start|>["));
    {
      let _items3 = J.iter(tools_0);
      const _loop3 = J.loop(_items3);
      for (let _i3 = 0; _i3 < _items3.length; _i3++) {
        _loop3.index0 = _i3; const _it3 = _items3[_i3]; let tool_13 = _it3;
        if (J.t(!J.t(J.test("string", tool_13)))) {
          tool_13 = J.filter("tojson", tool_13);
        }
        J.setattr(ns_0, "system_prompt", J.add(J.attr(ns_0, "system_prompt"), tool_13));
        if (J.t(!J.t(J.attr(_loop3, "last")))) {
          J.setattr(ns_0, "system_prompt", J.add(J.attr(ns_0, "system_prompt"), ", "));
        }
      }
    }
    J.setattr(ns_0, "system_prompt", J.add(J.attr(ns_0, "system_prompt"), "]<|tool_list_end|>"));
  }
  if (J.t(J.attr(ns_0, "system_prompt"))) {
    out.push(J.str(J.add(J.add("<|im_start|>system\n", J.attr(ns_0, "system_prompt")), "<|im_end|>\n")));
  }
  {
    let _items4 = J.iter(messages_0);
    const _loop4 = J.loop(_items4);
    for (let _i4 = 0; _i4 < _items4.length; _i4++) {
      _loop4.index0 = _i4; const _it4 = _items4[_i4]; let message_14 = _it4;
      let content_15 = content_0;
      out.push(J.str(J.add(J.add("<|im_start|>", J.item(message_14, "role")), "\n")));
      content_15 = J.call(J.attr(message_14, "get"), "content");
      if (J.t(!J.t(J.test("string", content_15)))) {
        content_15 = J.filter("tojson", content_15);
      }
      if (J.t((J.eq(J.item(message_14, "role"), "tool")))) {
        content_15 = J.add(J.add("<|tool_response_start|>", content_15), "<|tool_response_end|>");
      }
      if (J.t(((v) => J.t(v) ? J.call(J.attr(message_14, "get"), "tool_calls") : v)((J.eq(J.item(message_14, "role"), "assistant"))))) {
        if (J.t(((v) => J.t(v) ? (!J.eq(content_15, "null")) : v)(content_15))) {
          out.push(J.str(content_15));
        }
        out.push(J.str(J.call(render_tool_calls_0, J.item(message_14, "tool_calls"))), J.str("<|im_end|>\n"));
      } else {
        out.push(J.str(J.add(content_15, "<|im_end|>\n")));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|im_start|>assistant\n"));
  }
  return out.join("");
}
