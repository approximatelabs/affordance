// Converted from the chat template of stepfun-ai/Step3-VL-10B by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let render_content_0 = ctx["render_content"];
  let ns_0 = ctx["ns"];
  let tools_0 = ctx["tools"] ?? null;
  let messages_0 = ctx["messages"];
  let index_0 = ctx["index"];
  let content_0 = ctx["content"];
  let role_name_0 = ctx["role_name"];
  let reasoning_content_0 = ctx["reasoning_content"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  render_content_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_2 = k.caller;
    let content_1 = p.length > 0 ? p[0] : "content" in k ? k["content"] : undefined;
    if (J.t((content_1 === null))) {
      out.push(J.str(""));
    } else if (J.t(J.test("string", content_1))) {
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
    return out.join("");
  };
  if (J.t(tools_0)) {
    out.push(J.str("<|im_start|>system\n"));
    if (J.t((J.eq(J.attr(J.item(messages_0, 0), "role"), "system")))) {
      out.push(J.str(J.add(J.call(render_content_0, J.attr(J.item(messages_0, 0), "content")), "\n\n")));
    }
    out.push(J.str("# Tools\n\nYou may call one or more functions to assist with the user query.\n\nYou are provided with function signatures within <tools></tools> XML tags:\n<tools>"));
    {
      let _items2 = J.iter(tools_0);
      const _loop2 = J.loop(_items2);
      for (let _i2 = 0; _i2 < _items2.length; _i2++) {
        _loop2.index0 = _i2; const _it2 = _items2[_i2]; let tool_4 = _it2;
        out.push(J.str("\n"), J.str(J.filter("tojson", tool_4)));
      }
    }
    out.push(J.str("\n</tools>\n\nAlways adhere to this exact format for tool use:\n<tool_calls>\n<tool_call>\n{\"name\": <function-name>, \"arguments\": <args-json-object>}\n</tool_call>\n{additional_tool_calls}</tool_calls>\n\nNote:\n- For each function call, return a json object with function name and arguments within <tool_call></tool_call> XML tags.\n- `<function-name>` must be an exact match to one of the available tools.\n- `<args-json-object>` must be valid JSON that strictly follows the tool's parameters schema.<|im_end|>\n"));
  } else {
    if (J.t((J.eq(J.attr(J.item(messages_0, 0), "role"), "system")))) {
      out.push(J.str(J.add(J.add("<|im_start|>system\n", J.call(render_content_0, J.attr(J.item(messages_0, 0), "content"))), "<|im_end|>\n")));
    }
  }
  ns_0 = J.call(J.namespace, J.kw({"multi_step_tool": true, "last_query_index": J.sub(J.filter("length", messages_0), 1)}));
  {
    let _items3 = J.iter(J.slice(messages_0, undefined, undefined, J.neg(1)));
    const _loop3 = J.loop(_items3);
    for (let _i3 = 0; _i3 < _items3.length; _i3++) {
      _loop3.index0 = _i3; const _it3 = _items3[_i3]; let message_5 = _it3;
      let index_6 = index_0;
      index_6 = J.sub(J.sub(J.filter("length", messages_0), 1), J.attr(_loop3, "index0"));
      if (J.t(((v) => J.t(v) ? !J.t(((v) => J.t(v) ? J.call(J.attr(J.call(render_content_0, J.attr(message_5, "content")), "endswith"), "</tool_response>") : v)(J.call(J.attr(J.call(render_content_0, J.attr(message_5, "content")), "startswith"), "<tool_response>"))) : v)(((v) => J.t(v) ? J.test("string", J.call(render_content_0, J.attr(message_5, "content"))) : v)(((v) => J.t(v) ? (J.eq(J.attr(message_5, "role"), "user")) : v)(J.attr(ns_0, "multi_step_tool")))))) {
        J.setattr(ns_0, "multi_step_tool", false);
        J.setattr(ns_0, "last_query_index", index_6);
      }
    }
  }
  {
    let _items4 = J.iter(messages_0);
    const _loop4 = J.loop(_items4);
    for (let _i4 = 0; _i4 < _items4.length; _i4++) {
      _loop4.index0 = _i4; const _it4 = _items4[_i4]; let message_7 = _it4;
      let content_8 = content_0;
      let role_name_9 = role_name_0;
      let reasoning_content_10 = reasoning_content_0;
      content_8 = J.call(render_content_0, J.attr(message_7, "content"));
      if (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? !J.t(J.attr(_loop4, "first")) : v)((J.eq(J.attr(message_7, "role"), "system"))))((J.eq(J.attr(message_7, "role"), "user"))))) {
        role_name_9 = (J.t(((v) => J.t(v) ? (J.eq(J.attr(message_7, "name"), "observation")) : v)(((v) => J.t(v) ? !J.t(J.attr(_loop4, "first")) : v)((J.eq(J.attr(message_7, "role"), "system"))))) ? "observation" : J.attr(message_7, "role"));
        out.push(J.str(J.add(J.add(J.add(J.add(J.add("<|im_start|>", role_name_9), "\n"), content_8), "<|im_end|>"), "\n")));
      } else if (J.t((J.eq(J.attr(message_7, "role"), "assistant")))) {
        if (J.t(J.test("string", J.attr(message_7, "reasoning_content")))) {
          reasoning_content_10 = J.call(render_content_0, J.attr(message_7, "reasoning_content"));
        } else {
          if (J.t((J.has("</think>", content_8)))) {
            reasoning_content_10 = J.call(J.attr(J.item(J.call(J.attr(J.call(J.attr(J.item(J.call(J.attr(content_8, "split"), "</think>"), 0), "rstrip"), "\n"), "split"), "<think>"), J.neg(1)), "lstrip"), "\n");
            content_8 = J.call(J.attr(J.item(J.call(J.attr(content_8, "split"), "</think>"), J.neg(1)), "lstrip"), "\n");
          } else {
            reasoning_content_10 = "";
          }
        }
        if (J.t((J.gt(J.attr(_loop4, "index0"), J.attr(ns_0, "last_query_index"))))) {
          out.push(J.str(J.add(J.add(J.add(J.add(J.add("<|im_start|>", J.attr(message_7, "role")), "\n<think>\n"), reasoning_content_10), "\n</think>\n"), content_8)));
        } else {
          out.push(J.str(J.add(J.add(J.add("<|im_start|>", J.attr(message_7, "role")), "\n"), content_8)));
        }
        if (J.t(J.attr(message_7, "tool_calls"))) {
          out.push(J.str("\n<tool_calls>"));
          {
            let _items5 = J.iter(J.attr(message_7, "tool_calls"));
            const _loop5 = J.loop(_items5);
            for (let _i5 = 0; _i5 < _items5.length; _i5++) {
              _loop5.index0 = _i5; const _it5 = _items5[_i5]; let tool_call_11 = _it5;
              out.push(J.str("\n"));
              if (J.t(J.attr(tool_call_11, "function"))) {
                tool_call_11 = J.attr(tool_call_11, "function");
              }
              out.push(J.str("<tool_call>\n{\"name\": \""), J.str(J.attr(tool_call_11, "name")), J.str("\", \"arguments\": "));
              if (J.t(J.test("string", J.attr(tool_call_11, "arguments")))) {
                out.push(J.str(J.attr(tool_call_11, "arguments")));
              } else {
                out.push(J.str(J.filter("tojson", J.attr(tool_call_11, "arguments"))));
              }
              out.push(J.str("}\n</tool_call>"));
            }
          }
          out.push(J.str("\n</tool_calls>"));
        }
        out.push(J.str("<|im_end|>\n"));
      } else if (J.t((J.eq(J.attr(message_7, "role"), "tool")))) {
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.sub(J.attr(_loop4, "index0"), 1)), "role"), "tool")))(J.attr(_loop4, "first")))) {
          out.push(J.str("<|im_start|>tool_response"));
        }
        out.push(J.str("\n<tool_response>\n"), J.str(content_8), J.str("\n</tool_response>"));
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.add(J.attr(_loop4, "index0"), 1)), "role"), "tool")))(J.attr(_loop4, "last")))) {
          out.push(J.str("<|im_end|>\n"));
        }
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|im_start|>assistant\n<think>\n"));
  }
  return out.join("");
}
