// Converted from the chat template of EssentialAI/rnj-1-instruct by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let ns_0 = ctx["ns"];
  let emit_0 = ctx["emit"];
  let base_system_0 = ctx["base_system"];
  let default_system_0 = ctx["default_system"];
  let has_user_system_0 = ctx["has_user_system"];
  let effective_system_0 = ctx["effective_system"];
  let sys_preamble_0 = ctx["sys_preamble"];
  let sys_content_0 = ctx["sys_content"];
  let content_0 = ctx["content"];
  let messages_0 = ctx["messages"];
  let tools_0 = ctx["tools"] ?? null;
  let bos_token_0 = ctx["bos_token"];
  let index_0 = ctx["index"];
  let block_0 = ctx["block"];
  let body_0 = ctx["body"];
  let header_0 = ctx["header"];
  let open_user_0 = ctx["open_user"];
  let close_user_0 = ctx["close_user"];
  let tc_0 = ctx["tc"];
  let args_json_0 = ctx["args_json"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  ns_0 = J.call(J.namespace, J.kw({"multi_step_tool": true, "last_query_index": J.sub(J.filter("length", messages_0), 1)}));
  emit_0 = J.call(J.namespace, J.kw({"started": false}));
  base_system_0 = "You are rnj-1, a foundation model trained by Essential AI.\n";
  default_system_0 = "You are a helpful assistant.";
  has_user_system_0 = ((v) => J.t(v) ? J.test("string", J.attr(J.item(messages_0, 0), "content")) : v)(((v) => J.t(v) ? (J.eq(J.attr(J.item(messages_0, 0), "role"), "system")) : v)(messages_0));
  effective_system_0 = ((v) => J.t(v) ? v : default_system_0)(((v) => J.t(v) ? J.attr(J.item(messages_0, 0), "content") : v)(has_user_system_0));
  if (J.t(tools_0)) {
    {
      const captured = (() => {
      const out = [];
      out.push("# Tools\n\nYou may call one or more functions to assist with the user query.\n\nYou are provided with function signatures within <tools></tools> XML tags:\n<tools>");
      {
        let _items1 = J.iter(tools_0);
        const _loop1 = J.loop(_items1);
        for (let _i1 = 0; _i1 < _items1.length; _i1++) {
          _loop1.index0 = _i1; const _it1 = _items1[_i1]; let tool_1 = _it1;
          out.push(J.str(J.concat("\n", J.filter("tojson", tool_1))), "\n");
        }
      }
      out.push("</tools>\n\nFor each function call, return a json object with function name and arguments within <tool_call></tool_call> XML tags:\n<tool_call>\n{\"name\": <function-name>, \"arguments\": <args-json-object>}\n</tool_call>");
      return out.join("");
      })();
      sys_preamble_0 = captured;
    }
    sys_content_0 = J.concat(effective_system_0, "\n\n", sys_preamble_0);
    content_0 = J.concat("<|start_header_id|>system<|end_header_id|>\n", base_system_0, "\n", sys_content_0, "<|eot_id|>");
    if (J.t(!J.t(J.attr(emit_0, "started")))) {
      content_0 = J.concat(bos_token_0, content_0);
      J.setattr(emit_0, "started", true);
    }
    out.push(J.str(content_0));
  } else {
    content_0 = J.concat("<|start_header_id|>system<|end_header_id|>\n", base_system_0, "\n", effective_system_0, "<|eot_id|>");
    if (J.t(!J.t(J.attr(emit_0, "started")))) {
      content_0 = J.concat(bos_token_0, content_0);
      J.setattr(emit_0, "started", true);
    }
    out.push(J.str(content_0));
  }
  {
    let _items2 = J.iter(J.slice(messages_0, undefined, undefined, J.neg(1)));
    const _loop2 = J.loop(_items2);
    for (let _i2 = 0; _i2 < _items2.length; _i2++) {
      _loop2.index0 = _i2; const _it2 = _items2[_i2]; let message_2 = _it2;
      let index_3 = index_0;
      index_3 = J.sub(J.sub(J.filter("length", messages_0), 1), J.attr(_loop2, "index0"));
      if (J.t(((v) => J.t(v) ? !J.t(((v) => J.t(v) ? J.call(J.attr(J.attr(message_2, "content"), "endswith"), "</tool_response>") : v)(J.call(J.attr(J.attr(message_2, "content"), "startswith"), "<tool_response>"))) : v)(((v) => J.t(v) ? J.test("string", J.attr(message_2, "content")) : v)(((v) => J.t(v) ? (J.eq(J.attr(message_2, "role"), "user")) : v)(J.attr(ns_0, "multi_step_tool")))))) {
        J.setattr(ns_0, "multi_step_tool", false);
        J.setattr(ns_0, "last_query_index", index_3);
      }
    }
  }
  {
    let _items3 = J.iter(messages_0);
    const _loop3 = J.loop(_items3);
    for (let _i3 = 0; _i3 < _items3.length; _i3++) {
      _loop3.index0 = _i3; const _it3 = _items3[_i3]; let message_4 = _it3;
      let content_5 = content_0;
      let block_6 = block_0;
      let body_7 = body_0;
      let header_8 = header_0;
      let open_user_9 = open_user_0;
      let close_user_10 = close_user_0;
      if (J.t(J.test("string", J.attr(message_4, "content")))) {
        content_5 = J.attr(message_4, "content");
      } else {
        content_5 = "";
      }
      if (J.t(((v) => J.t(v) ? (J.eq(J.attr(message_4, "role"), "system")) : v)(J.attr(_loop3, "first")))) {
      } else if (J.t(((v) => J.t(v) ? v : (J.eq(J.attr(message_4, "role"), "system")))((J.eq(J.attr(message_4, "role"), "user"))))) {
        block_6 = J.concat("<|start_header_id|>", J.attr(message_4, "role"), "<|end_header_id|>\n", content_5, "<|eot_id|>");
        if (J.t(!J.t(J.attr(emit_0, "started")))) {
          block_6 = J.concat(bos_token_0, block_6);
          J.setattr(emit_0, "started", true);
        }
        out.push(J.str(block_6));
      } else if (J.t((J.eq(J.attr(message_4, "role"), "assistant")))) {
        body_7 = content_5;
        header_8 = "<|start_header_id|>assistant<|end_header_id|>\n";
        if (J.t(!J.t(J.attr(emit_0, "started")))) {
          out.push(J.str(bos_token_0));
          J.setattr(emit_0, "started", true);
        }
        out.push(J.str(header_8));
        {
          out.push(J.str(body_7));
          if (J.t(J.attr(message_4, "tool_calls"))) {
            {
              let _items4 = J.iter(J.attr(message_4, "tool_calls"));
              const _loop4 = J.loop(_items4);
              for (let _i4 = 0; _i4 < _items4.length; _i4++) {
                _loop4.index0 = _i4; const _it4 = _items4[_i4]; let tool_call_11 = _it4;
                let tc_12 = tc_0;
                let args_json_13 = args_json_0;
                if (J.t(J.attr(tool_call_11, "function"))) {
                  tc_12 = J.attr(tool_call_11, "function");
                } else {
                  tc_12 = tool_call_11;
                }
                args_json_13 = (J.t(J.test("string", J.attr(tc_12, "arguments"))) ? J.attr(tc_12, "arguments") : J.filter("tojson", J.attr(tc_12, "arguments")));
                if (J.t(J.attr(_loop4, "first"))) {
                  out.push(J.str(J.concat("<tool_call>\n{\"name\": \"", J.attr(tc_12, "name"), "\", \"arguments\": ", args_json_13, "}\n</tool_call>")));
                } else {
                  out.push(J.str(J.concat("\n<tool_call>\n{\"name\": \"", J.attr(tc_12, "name"), "\", \"arguments\": ", args_json_13, "}\n</tool_call>")));
                }
              }
            }
          }
          out.push(J.str("<|eot_id|>"));
        }
      } else if (J.t((J.eq(J.attr(message_4, "role"), "tool")))) {
        open_user_9 = ((v) => J.t(v) ? v : ((v) => J.t(v) ? (!J.eq(J.attr(J.item(messages_0, J.sub(J.attr(_loop3, "index0"), 1)), "role"), "tool")) : v)((J.gt(J.attr(_loop3, "index0"), 0))))(J.attr(_loop3, "first"));
        close_user_10 = ((v) => J.t(v) ? v : ((v) => J.t(v) ? (!J.eq(J.attr(J.item(messages_0, J.add(J.attr(_loop3, "index0"), 1)), "role"), "tool")) : v)((J.lt(J.attr(_loop3, "index0"), J.sub(J.filter("length", messages_0), 1)))))(J.attr(_loop3, "last"));
        if (J.t(open_user_9)) {
          header_8 = "<|start_header_id|>user<|end_header_id|>\n";
          if (J.t(!J.t(J.attr(emit_0, "started")))) {
            header_8 = J.concat(bos_token_0, header_8);
            J.setattr(emit_0, "started", true);
          }
          out.push(J.str(header_8));
        }
        if (J.t(open_user_9)) {
          out.push(J.str("<tool_response>\n"));
        } else {
          out.push(J.str("\n<tool_response>\n"));
        }
        out.push(J.str(content_5), J.str("\n</tool_response>"));
        if (J.t(close_user_10)) {
          out.push(J.str("<|eot_id|>"));
        }
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|start_header_id|>assistant<|end_header_id|>\n"));
  }
  return out.join("");
}
