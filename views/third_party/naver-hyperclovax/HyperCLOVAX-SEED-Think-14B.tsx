// Converted from the chat template of naver-hyperclovax/HyperCLOVAX-SEED-Think-14B by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let ns_0 = ctx["ns"];
  let tools_0 = ctx["tools"] ?? null;
  let messages_0 = ctx["messages"];
  let index_0 = ctx["index"];
  let content_0 = ctx["content"];
  let ns_content_0 = ctx["ns_content"];
  let reasoning_content_0 = ctx["reasoning_content"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let force_reasoning_0 = ctx["force_reasoning"];
  let skip_reasoning_0 = ctx["skip_reasoning"];
  if (J.t(((v) => J.t(v) ? v : (tools_0 === null))(!J.t((tools_0 !== undefined))))) {
    out.push(J.str("<|im_start|>tool_list\n<|im_end|>\n"));
  } else {
    out.push(J.str("<|im_start|>tool_list\n["));
    {
      let _items1 = J.iter(tools_0);
      const _loop1 = J.loop(_items1);
      for (let _i1 = 0; _i1 < _items1.length; _i1++) {
        _loop1.index0 = _i1; const _it1 = _items1[_i1]; let tool_1 = _it1;
        out.push(J.str("{\"name\": \""), J.str(J.attr(J.attr(tool_1, "function"), "name")), J.str("\", "), J.str("\"description\": \""), J.str(J.attr(J.attr(tool_1, "function"), "description")), J.str("\""));
        if (J.t((J.attr(J.attr(tool_1, "function"), "parameters") !== undefined))) {
          out.push(J.str(", \"parameters\": "), J.str(J.filter("tojson", J.attr(J.attr(tool_1, "function"), "parameters"))));
        }
        out.push(J.str("}"));
        if (J.t(!J.t(J.attr(_loop1, "last")))) {
          out.push(J.str(", "));
        }
      }
    }
    out.push(J.str("]<|im_end|>\n"));
  }
  ns_0 = J.call(J.namespace, J.kw({"is_searching": true, "last_query_index": J.neg(1)}));
  {
    let _items2 = J.iter(J.slice(messages_0, undefined, undefined, J.neg(1)));
    const _loop2 = J.loop(_items2);
    for (let _i2 = 0; _i2 < _items2.length; _i2++) {
      _loop2.index0 = _i2; const _it2 = _items2[_i2]; let message_2 = _it2;
      let index_3 = index_0;
      index_3 = J.sub(J.sub(J.filter("length", messages_0), 1), J.attr(_loop2, "index0"));
      if (J.t(((v) => J.t(v) ? ((v) => J.t(v) ? v : (J.eq(J.attr(message_2, "role"), "tool")))((J.eq(J.attr(message_2, "role"), "user"))) : v)(J.attr(ns_0, "is_searching")))) {
        J.setattr(ns_0, "last_query_index", index_3);
        J.setattr(ns_0, "is_searching", false);
      }
    }
  }
  {
    let _items3 = J.iter(messages_0);
    const _loop3 = J.loop(_items3);
    for (let _i3 = 0; _i3 < _items3.length; _i3++) {
      _loop3.index0 = _i3; const _it3 = _items3[_i3]; let message_4 = _it3;
      let content_5 = content_0;
      let ns_content_6 = ns_content_0;
      let reasoning_content_7 = reasoning_content_0;
      if (J.t(((v) => J.t(v) ? (!J.eq(J.attr(message_4, "role"), "system")) : v)((J.eq(J.attr(_loop3, "index0"), 0))))) {
        out.push(J.str("<|im_start|>system\n<|im_end|>\n"));
      }
      if (J.t(J.test("string", J.attr(message_4, "content")))) {
        content_5 = J.attr(message_4, "content");
      } else if (J.t(((v) => J.t(v) ? !J.t((J.attr(message_4, "content") === null)) : v)(J.test("iterable", J.attr(message_4, "content"))))) {
        ns_content_6 = J.call(J.namespace, J.kw({"text": ""}));
        {
          let _items4 = J.iter(J.attr(message_4, "content"));
          const _loop4 = J.loop(_items4);
          for (let _i4 = 0; _i4 < _items4.length; _i4++) {
            _loop4.index0 = _i4; const _it4 = _items4[_i4]; let part_8 = _it4;
            if (J.t(((v) => J.t(v) ? (J.attr(part_8, "text") !== undefined) : v)(((v) => J.t(v) ? (J.eq(J.attr(part_8, "type"), "text")) : v)((J.attr(part_8, "type") !== undefined))))) {
              J.setattr(ns_content_6, "text", J.add(J.attr(ns_content_6, "text"), J.attr(part_8, "text")));
            }
          }
        }
        content_5 = J.attr(ns_content_6, "text");
      } else {
        content_5 = "";
      }
      reasoning_content_7 = "";
      if (J.t(((v) => J.t(v) ? !J.t((J.attr(message_4, "reasoning_content") === null)) : v)((J.attr(message_4, "reasoning_content") !== undefined)))) {
        reasoning_content_7 = J.attr(message_4, "reasoning_content");
      }
      if (J.t((J.eq(J.attr(message_4, "role"), "assistant")))) {
        if (J.t((J.gt(J.attr(_loop3, "index0"), J.attr(ns_0, "last_query_index"))))) {
          if (J.t(reasoning_content_7)) {
            out.push(J.str(J.add(J.add("<|im_start|>assistant/think\n", J.call(J.attr(reasoning_content_7, "strip"), "\n")), "<|im_end|>\n")));
          }
        }
        if (J.t(content_5)) {
          out.push(J.str(J.add(J.add("<|im_start|>assistant\n", J.call(J.attr(content_5, "strip"), "\n")), "<|im_end|>")));
          if (J.t(J.attr(message_4, "tool_calls"))) {
            out.push(J.str("\n"));
          } else {
            out.push(J.str("<|endofturn|>\n"));
          }
        }
        if (J.t(J.attr(message_4, "tool_calls"))) {
          out.push(J.str("<|im_start|>assistant -> tool/function_call\n["));
          {
            let _items5 = J.iter(J.attr(message_4, "tool_calls"));
            const _loop5 = J.loop(_items5);
            for (let _i5 = 0; _i5 < _items5.length; _i5++) {
              _loop5.index0 = _i5; const _it5 = _items5[_i5]; let tool_call_9 = _it5;
              if (J.t(!J.t(J.attr(_loop5, "first")))) {
                out.push(J.str(", "));
              }
              if (J.t(J.attr(tool_call_9, "function"))) {
                tool_call_9 = J.attr(tool_call_9, "function");
              }
              out.push(J.str("{\"name\": \""), J.str(J.attr(tool_call_9, "name")), J.str("\", \"arguments\": "));
              if (J.t(J.test("string", J.attr(tool_call_9, "arguments")))) {
                out.push(J.str(J.attr(tool_call_9, "arguments")));
              } else {
                out.push(J.str(J.filter("tojson", J.attr(tool_call_9, "arguments"))));
              }
              out.push(J.str("}"));
            }
          }
          out.push(J.str("]<|im_end|><|stop|>\n"));
        }
      } else if (J.t((J.eq(J.attr(message_4, "role"), "tool")))) {
        out.push(J.str(J.add(J.add("<|im_start|>tool/function_call\n", content_5), "<|im_end|>\n")));
      } else {
        out.push(J.str(J.add(J.add(J.add(J.add("<|im_start|>", J.attr(message_4, "role")), "\n"), content_5), "<|im_end|>\n")));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    if (J.t(((v) => J.t(v) ? J.test("true", force_reasoning_0) : v)((force_reasoning_0 !== undefined)))) {
      out.push(J.str("<|im_start|>assistant/think\n"));
    } else if (J.t(((v) => J.t(v) ? J.test("true", skip_reasoning_0) : v)((skip_reasoning_0 !== undefined)))) {
      out.push(J.str("<|im_start|>assistant/think\n<|im_end|>\n<|im_start|>assistant\n"));
    } else {
      out.push(J.str("<|im_start|>assistant"));
    }
  }
  return out.join("");
}
