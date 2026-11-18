// Converted from the chat template of baidu/Qianfan-OCR by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let ns_0 = ctx["ns"];
  let messages_0 = ctx["messages"];
  let index_0 = ctx["index"];
  let append_think_0 = ctx["append_think"];
  let raw_content_0 = ctx["raw_content"];
  let content_ns_0 = ctx["content_ns"];
  let content_0 = ctx["content"];
  let reasoning_content_0 = ctx["reasoning_content"];
  let enable_thinking_0 = ctx["enable_thinking"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
    out.push(J.str("<|im_start|>system\n"));
    if (J.t(J.test("string", J.item(J.item(messages_0, 0), "content")))) {
      out.push(J.str(J.item(J.item(messages_0, 0), "content")));
    } else {
      {
        let _items1 = J.iter(J.item(J.item(messages_0, 0), "content"));
        const _loop1 = J.loop(_items1);
        for (let _i1 = 0; _i1 < _items1.length; _i1++) {
          _loop1.index0 = _i1; const _it1 = _items1[_i1]; let item_1 = _it1;
          if (J.t((J.eq(J.item(item_1, "type"), "text")))) {
            out.push(J.str(J.item(item_1, "text")));
          }
        }
      }
    }
    out.push(J.str("<|im_end|>\n"));
  } else {
    out.push(J.str("<|im_start|>system\n\u4f60\u662fQianfan-VL\uff0c\u7531\u767e\u5ea6\u667a\u80fd\u4e91\u7814\u53d1\u7684\u591a\u6a21\u6001\u5927\u8bed\u8a00\u6a21\u578b\u3002<|im_end|>\n"));
  }
  ns_0 = J.call(J.namespace, J.kw({"found_last_user": false, "last_query_index": J.sub(J.filter("length", messages_0), 1)}));
  {
    let _items2 = J.iter(J.slice(messages_0, undefined, undefined, J.neg(1)));
    const _loop2 = J.loop(_items2);
    for (let _i2 = 0; _i2 < _items2.length; _i2++) {
      _loop2.index0 = _i2; const _it2 = _items2[_i2]; let message_2 = _it2;
      let index_3 = index_0;
      index_3 = J.sub(J.sub(J.filter("length", messages_0), 1), J.attr(_loop2, "index0"));
      if (J.t(((v) => J.t(v) ? (J.eq(J.item(message_2, "role"), "user")) : v)(!J.t(J.attr(ns_0, "found_last_user"))))) {
        J.setattr(ns_0, "found_last_user", true);
        J.setattr(ns_0, "last_query_index", index_3);
      }
    }
  }
  {
    let _items3 = J.iter(messages_0);
    const _loop3 = J.loop(_items3);
    for (let _i3 = 0; _i3 < _items3.length; _i3++) {
      _loop3.index0 = _i3; const _it3 = _items3[_i3]; let message_4 = _it3;
      let append_think_5 = append_think_0;
      let raw_content_6 = raw_content_0;
      let content_ns_7 = content_ns_0;
      let content_8 = content_0;
      let reasoning_content_9 = reasoning_content_0;
      if (J.t(((v) => J.t(v) ? v : !J.t(J.attr(_loop3, "first")))((!J.eq(J.item(J.item(messages_0, 0), "role"), "system"))))) {
        if (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? !J.t(J.attr(_loop3, "first")) : v)((J.eq(J.item(message_4, "role"), "system"))))((J.eq(J.item(message_4, "role"), "user"))))) {
          append_think_5 = ((v) => J.t(v) ? (J.eq(J.attr(_loop3, "index0"), J.attr(ns_0, "last_query_index"))) : v)(((v) => J.t(v) ? (J.eq(J.item(message_4, "role"), "user")) : v)(((v) => J.t(v) ? enable_thinking_0 : v)((enable_thinking_0 !== undefined))));
          out.push(J.str(J.add(J.add("<|im_start|>", J.item(message_4, "role")), "\n")));
          if (J.t(J.test("string", J.item(message_4, "content")))) {
            out.push(J.str(J.item(message_4, "content")));
          } else {
            {
              let _items4 = J.iter(J.item(message_4, "content"));
              const _loop4 = J.loop(_items4);
              for (let _i4 = 0; _i4 < _items4.length; _i4++) {
                _loop4.index0 = _i4; const _it4 = _items4[_i4]; let item_10 = _it4;
                if (J.t((J.eq(J.item(item_10, "type"), "image")))) {
                  out.push(J.str("<image>\n"));
                } else if (J.t((J.eq(J.item(item_10, "type"), "video")))) {
                  out.push(J.str("<video>\n"));
                } else if (J.t((J.eq(J.item(item_10, "type"), "text")))) {
                  out.push(J.str(J.item(item_10, "text")));
                }
              }
            }
          }
          if (J.t(append_think_5)) {
            out.push(J.str("<think>"));
          }
          out.push(J.str("<|im_end|>\n"));
        } else if (J.t((J.eq(J.item(message_4, "role"), "assistant")))) {
          if (J.t(J.test("string", J.item(message_4, "content")))) {
            raw_content_6 = J.item(message_4, "content");
          } else {
            content_ns_7 = J.call(J.namespace, J.kw({"raw": ""}));
            {
              let _items5 = J.iter(J.item(message_4, "content"));
              const _loop5 = J.loop(_items5);
              for (let _i5 = 0; _i5 < _items5.length; _i5++) {
                _loop5.index0 = _i5; const _it5 = _items5[_i5]; let item_11 = _it5;
                if (J.t((J.eq(J.item(item_11, "type"), "text")))) {
                  J.setattr(content_ns_7, "raw", J.add(J.attr(content_ns_7, "raw"), J.item(item_11, "text")));
                }
              }
            }
            raw_content_6 = J.attr(content_ns_7, "raw");
          }
          content_8 = raw_content_6;
          reasoning_content_9 = "";
          if (J.t(((v) => J.t(v) ? !J.t((J.item(message_4, "reasoning_content") === null)) : v)((J.has("reasoning_content", message_4))))) {
            reasoning_content_9 = J.item(message_4, "reasoning_content");
          } else if (J.t((J.has("</think>", raw_content_6)))) {
            content_8 = J.call(J.attr(J.item(J.call(J.attr(raw_content_6, "split"), "</think>"), J.neg(1)), "lstrip"), "\n");
            reasoning_content_9 = J.call(J.attr(J.item(J.call(J.attr(J.call(J.attr(J.item(J.call(J.attr(raw_content_6, "split"), "</think>"), 0), "rstrip"), "\n"), "split"), "<think>"), J.neg(1)), "lstrip"), "\n");
          }
          if (J.t(((v) => J.t(v) ? reasoning_content_9 : v)((J.gt(J.attr(_loop3, "index0"), J.attr(ns_0, "last_query_index")))))) {
            out.push(J.str(J.add(J.add(J.add(J.add(J.add("<|im_start|>", J.item(message_4, "role")), "\n<think>\n"), J.call(J.attr(reasoning_content_9, "strip"), "\n")), "\n</think>\n\n"), J.call(J.attr(content_8, "lstrip"), "\n"))));
          } else {
            out.push(J.str(J.add(J.add(J.add("<|im_start|>", J.item(message_4, "role")), "\n"), content_8)));
          }
          out.push(J.str("<|im_end|>\n"));
        }
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|im_start|>assistant\n"));
  }
  return out.join("");
}
