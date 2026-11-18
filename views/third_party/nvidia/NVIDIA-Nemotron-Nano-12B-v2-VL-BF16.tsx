// Converted from the chat template of nvidia/NVIDIA-Nemotron-Nano-12B-v2-VL-BF16 by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let ns_0 = ctx["ns"];
  let msg_0 = ctx["msg"];
  let messages_0 = ctx["messages"];
  let bos_token_0 = ctx["bos_token"];
  let tools_0 = ctx["tools"] ?? null;
  let mm_content_0 = ctx["mm_content"];
  let counters_0 = ctx["counters"];
  let image_tags_0 = ctx["image_tags"];
  let fn_0 = ctx["fn"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  ns_0 = J.call(J.namespace, J.kw({"enable_thinking": false, "has_sys_prompt": false, "non_tool_system_content": "", "has_video": false, "explicit_think_requested": false}));
  msg_0 = J.call(J.namespace, J.kw({"content": ""}));
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      if (J.t((J.eq(J.item(message_1, "role"), "system")))) {
        J.setattr(ns_0, "has_sys_prompt", true);
        if (J.t(J.test("string", J.item(message_1, "content")))) {
          J.setattr(ns_0, "non_tool_system_content", J.call(J.attr(J.call(J.attr(J.call(J.attr(J.call(J.attr(J.call(J.attr(J.item(message_1, "content"), "replace"), "</think>", "<_end_think>"), "replace"), "/think", ""), "replace"), "/no_think", ""), "replace"), "<_end_think>", "</think>"), "strip")));
        } else {
          J.setattr(ns_0, "non_tool_system_content", "");
          {
            let _items2 = J.iter(J.item(message_1, "content"));
            const _loop2 = J.loop(_items2);
            for (let _i2 = 0; _i2 < _items2.length; _i2++) {
              _loop2.index0 = _i2; const _it2 = _items2[_i2]; let content_2 = _it2;
              if (J.t((J.eq(J.item(content_2, "type"), "text")))) {
                J.setattr(ns_0, "non_tool_system_content", J.add(J.attr(ns_0, "non_tool_system_content"), J.call(J.attr(J.call(J.attr(J.call(J.attr(J.call(J.attr(J.item(content_2, "text"), "replace"), "</think>", "<_end_think>"), "replace"), "/think", ""), "replace"), "/no_think", ""), "replace"), "<_end_think>", "</think>")));
              }
            }
          }
          J.setattr(ns_0, "non_tool_system_content", J.call(J.attr(J.attr(ns_0, "non_tool_system_content"), "strip")));
        }
      }
      if (J.t(!J.t(J.test("string", J.item(message_1, "content"))))) {
        {
          let _items3 = J.iter(J.item(message_1, "content"));
          const _loop3 = J.loop(_items3);
          for (let _i3 = 0; _i3 < _items3.length; _i3++) {
            _loop3.index0 = _i3; const _it3 = _items3[_i3]; let content_3 = _it3;
            if (J.t(((v) => J.t(v) ? v : (J.eq(J.item(content_3, "type"), "video_url")))((J.eq(J.item(content_3, "type"), "video"))))) {
              J.setattr(ns_0, "has_video", true);
            }
          }
        }
      }
      if (J.t(J.test("string", J.item(message_1, "content")))) {
        if (J.t(((v) => J.t(v) ? v : (J.eq(J.item(message_1, "role"), "system")))((J.eq(J.item(message_1, "role"), "user"))))) {
          if (J.t((J.has("/think", J.call(J.attr(J.item(message_1, "content"), "replace"), "</think>", ""))))) {
            J.setattr(ns_0, "enable_thinking", true);
            J.setattr(ns_0, "explicit_think_requested", true);
          } else if (J.t((J.has("/no_think", J.item(message_1, "content"))))) {
            J.setattr(ns_0, "enable_thinking", false);
          }
        }
      } else {
        {
          let _items4 = J.iter(J.item(message_1, "content"));
          const _loop4 = J.loop(_items4);
          for (let _i4 = 0; _i4 < _items4.length; _i4++) {
            _loop4.index0 = _i4; const _it4 = _items4[_i4]; let content_4 = _it4;
            if (J.t((J.eq(J.item(content_4, "type"), "text")))) {
              if (J.t(((v) => J.t(v) ? v : (J.eq(J.item(message_1, "role"), "system")))((J.eq(J.item(message_1, "role"), "user"))))) {
                if (J.t((J.has("/think", J.call(J.attr(J.item(content_4, "text"), "replace"), "</think>", ""))))) {
                  J.setattr(ns_0, "enable_thinking", true);
                  J.setattr(ns_0, "explicit_think_requested", true);
                } else if (J.t((J.has("/no_think", J.item(content_4, "text"))))) {
                  J.setattr(ns_0, "enable_thinking", false);
                }
              }
            }
          }
        }
      }
    }
  }
  if (J.t(((v) => J.t(v) ? J.attr(ns_0, "explicit_think_requested") : v)(J.attr(ns_0, "has_video")))) {
    out.push(J.str(J.call(J.raise_exception, "Video inputs are not supported with reasoning mode. Please remove /think flag or remove video content.")));
  }
  if (J.t(((v) => J.t(v) ? !J.t(J.attr(ns_0, "explicit_think_requested")) : v)(J.attr(ns_0, "has_video")))) {
    J.setattr(ns_0, "enable_thinking", false);
  }
  out.push(J.str(bos_token_0));
  if (J.t((!J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
    out.push(J.str("<SPECIAL_10>System\n"));
  } else {
    out.push(J.str(J.add("<SPECIAL_10>System\n", J.attr(ns_0, "non_tool_system_content"))));
  }
  if (J.t(tools_0)) {
    if (J.t((!J.eq(J.attr(ns_0, "non_tool_system_content"), "")))) {
      out.push(J.str("\n\n"));
    }
    out.push(J.str("You can use the following tools to assist the user if required:\n"), J.str("<AVAILABLE_TOOLS>["));
    {
      let _items5 = J.iter(tools_0);
      const _loop5 = J.loop(_items5);
      for (let _i5 = 0; _i5 < _items5.length; _i5++) {
        _loop5.index0 = _i5; const _it5 = _items5[_i5]; let tool_5 = _it5;
        out.push(J.str(J.filter("tojson", (J.t((J.attr(tool_5, "function") !== undefined)) ? J.attr(tool_5, "function") : tool_5))), J.str((J.t(!J.t(J.attr(_loop5, "last"))) ? ", " : "")));
      }
    }
    out.push(J.str("]</AVAILABLE_TOOLS>\n\n"), J.str("If you decide to call any tool(s), use the following format:\n"), J.str("<TOOLCALL>[{\"name\": \"tool_name1\", \"arguments\": \"tool_args1\"}, "), J.str("{\"name\": \"tool_name2\", \"arguments\": \"tool_args2\"}]</TOOLCALL>\n\n"), J.str("The user will execute tool-calls and return responses from tool(s) in this format:\n"), J.str("<TOOL_RESPONSE>[{\"response\": \"tool_response1\"}, "), J.str("{\"response\": \"tool_response2\"}]</TOOL_RESPONSE>\n\n"), J.str("Based on the tool responses, you can call additional tools if needed, "), J.str("correct tool calls if any errors are found, or just respond to the user."));
  }
  out.push(J.str("\n"));
  messages_0 = (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system"))) ? J.slice(messages_0, 1, undefined, undefined) : messages_0);
  if (J.t((J.eq(J.filter("length", messages_0), 0)))) {
    messages_0 = [({"role": "user", "content": ""})];
  }
  {
    let _items6 = J.iter(messages_0);
    const _loop6 = J.loop(_items6);
    for (let _i6 = 0; _i6 < _items6.length; _i6++) {
      _loop6.index0 = _i6; const _it6 = _items6[_i6]; let message_6 = _it6;
      let mm_content_7 = mm_content_0;
      let counters_8 = counters_0;
      let image_tags_9 = image_tags_0;
      if (J.t(J.test("string", J.item(message_6, "content")))) {
        J.setattr(msg_0, "content", J.call(J.attr(J.call(J.attr(J.call(J.attr(J.call(J.attr(J.call(J.attr(J.item(message_6, "content"), "replace"), "</think>", "<_end_think>"), "replace"), "/think", ""), "replace"), "/no_think", ""), "replace"), "<_end_think>", "</think>"), "strip")));
      } else {
        J.setattr(msg_0, "content", "");
        mm_content_7 = "";
        counters_8 = J.call(J.namespace, J.kw({"images": 0, "videos": 0}));
        {
          let _items7 = J.iter(J.item(message_6, "content"));
          const _loop7 = J.loop(_items7);
          for (let _i7 = 0; _i7 < _items7.length; _i7++) {
            _loop7.index0 = _i7; const _it7 = _items7[_i7]; let content_10 = _it7;
            if (J.t((J.eq(J.item(content_10, "type"), "image")))) {
              J.setattr(counters_8, "images", J.add(J.attr(counters_8, "images"), 1));
            } else if (J.t((J.eq(J.item(content_10, "type"), "video")))) {
              J.setattr(counters_8, "videos", J.add(J.attr(counters_8, "videos"), 1));
            } else if (J.t((J.eq(J.item(content_10, "type"), "text")))) {
              J.setattr(msg_0, "content", J.add(J.attr(msg_0, "content"), J.item(content_10, "text")));
            }
          }
        }
        if (J.t((J.has("<image>", J.attr(msg_0, "content"))))) {
          J.setattr(counters_8, "images", 0);
        }
        if (J.t((J.has("<video>", J.attr(msg_0, "content"))))) {
          J.setattr(counters_8, "videos", 0);
        }
        if (J.t((J.gt(J.attr(counters_8, "images"), 1)))) {
          image_tags_9 = J.call(J.namespace, J.kw({"tags": []}));
          {
            let _items8 = J.iter(J.call(J.range, J.attr(counters_8, "images")));
            const _loop8 = J.loop(_items8);
            for (let _i8 = 0; _i8 < _items8.length; _i8++) {
              _loop8.index0 = _i8; const _it8 = _items8[_i8]; let i_11 = _it8;
              J.setattr(image_tags_9, "tags", J.add(J.attr(image_tags_9, "tags"), [J.add(J.add("<image ", J.filter("string", J.add(i_11, 1))), "><image>")]));
            }
          }
          mm_content_7 = J.add(J.call(J.attr(" ", "join"), J.attr(image_tags_9, "tags")), "\n");
        } else if (J.t((J.eq(J.attr(counters_8, "images"), 1)))) {
          mm_content_7 = "<image>\n";
        }
        mm_content_7 = J.add(mm_content_7, J.mul("<video>\n", J.attr(counters_8, "videos")));
        J.setattr(msg_0, "content", J.add(mm_content_7, J.call(J.attr(J.attr(msg_0, "content"), "lstrip"), "\n")));
      }
      if (J.t((J.eq(J.item(message_6, "role"), "user")))) {
        out.push(J.str(J.add(J.add("<SPECIAL_11>User\n", J.call(J.attr(J.call(J.attr(J.call(J.attr(J.call(J.attr(J.call(J.attr(J.attr(msg_0, "content"), "replace"), "</think>", "<_end_think>"), "replace"), "/think", ""), "replace"), "/no_think", ""), "replace"), "<_end_think>", "</think>"), "strip"))), "\n")));
      } else if (J.t((J.eq(J.item(message_6, "role"), "tool")))) {
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.sub(J.attr(_loop6, "index0"), 1)), "role"), "tool")))(J.attr(_loop6, "first")))) {
          out.push(J.str(J.add("<SPECIAL_11>User\n", "<TOOL_RESPONSE>[")));
        }
        out.push(J.str(J.attr(msg_0, "content")), J.str((J.t(((v) => J.t(v) ? (J.eq(J.attr(J.item(messages_0, J.add(J.attr(_loop6, "index0"), 1)), "role"), "tool")) : v)(!J.t(J.attr(_loop6, "last")))) ? ", " : "")));
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.add(J.attr(_loop6, "index0"), 1)), "role"), "tool")))(J.attr(_loop6, "last")))) {
          out.push(J.str("]</TOOL_RESPONSE>\n"));
        }
      } else if (J.t((J.eq(J.item(message_6, "role"), "assistant")))) {
        if (J.t((J.has("</think>", J.attr(msg_0, "content"))))) {
          J.setattr(msg_0, "content", J.call(J.attr(J.item(J.call(J.attr(J.attr(msg_0, "content"), "split"), "</think>"), 1), "strip")));
        }
        out.push(J.str(J.add("<SPECIAL_11>Assistant\n", J.call(J.attr(J.attr(msg_0, "content"), "strip")))));
        if (J.t(J.attr(message_6, "tool_calls"))) {
          if (J.t((!J.eq(J.call(J.attr(J.attr(msg_0, "content"), "strip")), "")))) {
            out.push(J.str("\n\n"));
          }
          out.push(J.str("<TOOLCALL>["));
          {
            let _items9 = J.iter(J.attr(message_6, "tool_calls"));
            const _loop9 = J.loop(_items9);
            for (let _i9 = 0; _i9 < _items9.length; _i9++) {
              _loop9.index0 = _i9; const _it9 = _items9[_i9]; let call_12 = _it9;
              let fn_13 = fn_0;
              fn_13 = (J.t((J.attr(call_12, "function") !== undefined)) ? J.attr(call_12, "function") : call_12);
              out.push(J.str(J.add(J.add("{\"name\": \"", J.attr(fn_13, "name")), "\", \"arguments\": ")));
              if (J.t(J.test("string", J.attr(fn_13, "arguments")))) {
                out.push(J.str(J.attr(fn_13, "arguments")));
              } else {
                out.push(J.str(J.filter("tojson", J.attr(fn_13, "arguments"))));
              }
              out.push(J.str(J.add("}", (J.t(!J.t(J.attr(_loop9, "last"))) ? ", " : ""))));
            }
          }
          out.push(J.str("]</TOOLCALL>"));
        }
        out.push(J.str("\n<SPECIAL_12>\n"));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<SPECIAL_11>Assistant\n"));
    if (J.t(((v) => J.t(v) ? J.test("false", J.attr(ns_0, "enable_thinking")) : v)((J.attr(ns_0, "enable_thinking") !== undefined)))) {
      out.push(J.str("<think></think>"));
    } else {
      out.push(J.str("<think>\n"));
    }
  }
  return out.join("");
}
