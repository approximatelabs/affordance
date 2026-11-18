// Converted from the chat template of moonshotai/Kimi-K2.7-Code by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";
import { kimiInputs } from "./tool_declaration_ts.tsx";

function Template(ctx) {
  const out = [];
  let render_content_0 = ctx["render_content"];
  let set_roles_0 = ctx["set_roles"];
  let render_toolcalls_0 = ctx["render_toolcalls"];
  let c_0 = ctx["c"];
  let role_name_0 = ctx["role_name"];
  let formatted_id_0 = ctx["formatted_id"];
  let tools_0 = ctx["tools"] ?? null;
  let tools_ts_str_0 = ctx["tools_ts_str"];
  let messages_0 = ctx["messages"];
  let rc_0 = ctx["rc"];
  let tool_call_id_0 = ctx["tool_call_id"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  render_content_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_2 = k.caller;
    let msg_1 = p.length > 0 ? p[0] : "msg" in k ? k["msg"] : undefined;
    let c_3 = c_0;
    c_3 = J.call(J.attr(msg_1, "get"), "content");
    if (J.t(J.test("string", c_3))) {
      out.push(J.str(c_3));
    } else if (J.t(!J.t((c_3 === null)))) {
      {
        let _items1 = J.iter(c_3);
        const _loop1 = J.loop(_items1);
        for (let _i1 = 0; _i1 < _items1.length; _i1++) {
          _loop1.index0 = _i1; const _it1 = _items1[_i1]; let content_4 = _it1;
          if (J.t(((v) => J.t(v) ? v : (J.eq(J.item(content_4, "type"), "image_url")))((J.eq(J.item(content_4, "type"), "image"))))) {
            out.push("<|media_begin|>image<|media_content|><|media_pad|><|media_end|>\n");
          } else if (J.t(((v) => J.t(v) ? v : (J.eq(J.item(content_4, "type"), "video_url")))((J.eq(J.item(content_4, "type"), "video"))))) {
            out.push("<|kimi_k25_video_placeholder|>\n");
          } else {
            out.push(J.str(J.item(content_4, "text")));
          }
        }
      }
    }
    return out.join("");
  };
  set_roles_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_6 = k.caller;
    let message_5 = p.length > 0 ? p[0] : "message" in k ? k["message"] : undefined;
    let role_name_7 = role_name_0;
    role_name_7 = ((v) => J.t(v) ? v : J.item(message_5, "role"))(J.call(J.attr(message_5, "get"), "name"));
    if (J.t((J.eq(J.item(message_5, "role"), "user")))) {
      out.push("<|im_user|>", J.str(role_name_7), "<|im_middle|>");
    } else if (J.t((J.eq(J.item(message_5, "role"), "assistant")))) {
      out.push("<|im_assistant|>", J.str(role_name_7), "<|im_middle|>");
    } else {
      out.push("<|im_system|>", J.str(role_name_7), "<|im_middle|>");
    }
    return out.join("");
  };
  render_toolcalls_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_9 = k.caller;
    let message_8 = p.length > 0 ? p[0] : "message" in k ? k["message"] : undefined;
    out.push("<|tool_calls_section_begin|>");
    {
      let _items2 = J.iter(J.item(message_8, "tool_calls"));
      const _loop2 = J.loop(_items2);
      for (let _i2 = 0; _i2 < _items2.length; _i2++) {
        _loop2.index0 = _i2; const _it2 = _items2[_i2]; let tool_call_10 = _it2;
        let formatted_id_11 = formatted_id_0;
        formatted_id_11 = J.item(tool_call_10, "id");
        out.push("<|tool_call_begin|>", J.str(formatted_id_11), "<|tool_call_argument_begin|>");
        if (J.t(J.test("string", J.item(J.item(tool_call_10, "function"), "arguments")))) {
          out.push(J.str(J.item(J.item(tool_call_10, "function"), "arguments")));
        } else {
          out.push(J.str(J.filter("tojson", J.item(J.item(tool_call_10, "function"), "arguments"))));
        }
        out.push("<|tool_call_end|>");
      }
    }
    out.push("<|tool_calls_section_end|>");
    return out.join("");
  };
  if (J.t(tools_0)) {
    if (J.t(tools_ts_str_0)) {
      out.push("<|im_system|>tool_declare<|im_middle|>", J.str(tools_ts_str_0), "<|im_end|>");
    } else {
      out.push("<|im_system|>tool_declare<|im_middle|>", J.str(J.filter("tojson", tools_0, J.kw({"separators": [",", ":"]}))), "<|im_end|>");
    }
  }
  {
    let _items3 = J.iter(messages_0);
    const _loop3 = J.loop(_items3);
    for (let _i3 = 0; _i3 < _items3.length; _i3++) {
      _loop3.index0 = _i3; const _it3 = _items3[_i3]; let message_12 = _it3;
      let rc_13 = rc_0;
      let tool_call_id_14 = tool_call_id_0;
      out.push(J.str(J.call(set_roles_0, message_12)));
      if (J.t((J.eq(J.item(message_12, "role"), "assistant")))) {
        rc_13 = J.call(J.attr(message_12, "get"), "reasoning", J.call(J.attr(message_12, "get"), "reasoning_content", ""));
        out.push("<think>", J.str(rc_13), "</think>", J.str(J.call(render_content_0, message_12)));
        if (J.t(J.call(J.attr(message_12, "get"), "tool_calls"))) {
          out.push(J.str(J.call(render_toolcalls_0, message_12)));
        }
      } else if (J.t((J.eq(J.item(message_12, "role"), "tool")))) {
        tool_call_id_14 = J.attr(message_12, "tool_call_id");
        out.push("## Return of ", J.str(tool_call_id_14), "\n", J.str(J.call(render_content_0, message_12)));
      } else if (J.t(!J.t((J.item(message_12, "content") === null)))) {
        out.push(J.str(J.call(render_content_0, message_12)));
      }
      out.push("<|im_end|>");
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push("<|im_assistant|>assistant<|im_middle|><think>");
  }
  return out.join("");
}

// What the model's own code does to the inputs before its template runs.
export default function Main(given) {
  const ctx = kimiInputs(given);
  return Template(ctx);
}
