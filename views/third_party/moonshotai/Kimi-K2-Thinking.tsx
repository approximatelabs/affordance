// Converted from the chat template of moonshotai/Kimi-K2-Thinking by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

function Template(ctx) {
  const out = [];
  let render_content_0 = ctx["render_content"];
  let set_roles_0 = ctx["set_roles"];
  let render_toolcalls_0 = ctx["render_toolcalls"];
  let ns_0 = ctx["ns"];
  let hist_msgs_0 = ctx["hist_msgs"];
  let suffix_msgs_0 = ctx["suffix_msgs"];
  let c_0 = ctx["c"];
  let role_name_0 = ctx["role_name"];
  let formatted_id_0 = ctx["formatted_id"];
  let messages_0 = ctx["messages"];
  let tools_0 = ctx["tools"] ?? null;
  let tool_call_id_0 = ctx["tool_call_id"];
  let rc_0 = ctx["rc"];
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
          if (J.t(((v) => J.t(v) ? v : (J.has("image_url", content_4)))(((v) => J.t(v) ? v : (J.has("image", content_4)))((J.eq(J.item(content_4, "type"), "image")))))) {
            out.push("<|media_start|>image<|media_content|><|media_pad|><|media_end|>\n");
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
  ns_0 = J.call(J.namespace, J.kw({"last_non_tool_call_assistant_msg": J.neg(1)}));
  {
    let _items3 = J.iter(J.call(J.range, J.sub(J.filter("length", messages_0), 1), J.neg(1), J.neg(1)));
    const _loop3 = J.loop(_items3);
    for (let _i3 = 0; _i3 < _items3.length; _i3++) {
      _loop3.index0 = _i3; const _it3 = _items3[_i3]; let idx_12 = _it3;
      if (J.t(((v) => J.t(v) ? !J.t(J.call(J.attr(J.item(messages_0, idx_12), "get"), "tool_calls")) : v)((J.eq(J.item(J.item(messages_0, idx_12), "role"), "assistant"))))) {
        J.setattr(ns_0, "last_non_tool_call_assistant_msg", idx_12);
        break;
      }
    }
  }
  hist_msgs_0 = J.slice(messages_0, undefined, J.add(J.attr(ns_0, "last_non_tool_call_assistant_msg"), 1), undefined);
  suffix_msgs_0 = J.slice(messages_0, J.add(J.attr(ns_0, "last_non_tool_call_assistant_msg"), 1), undefined, undefined);
  if (J.t(tools_0)) {
    out.push("<|im_system|>tool_declare<|im_middle|>", J.str(J.filter("tojson", tools_0, J.kw({"separators": [",", ":"]}))), "<|im_end|>");
  }
  if (J.t(((v) => J.t(v) ? v : (!J.eq(J.item(J.item(messages_0, 0), "role"), "system")))((J.eq(J.filter("length", messages_0), 0))))) {
    out.push("<|im_system|>system<|im_middle|>You are Kimi, an AI assistant created by Moonshot AI.<|im_end|>");
  }
  {
    let _items4 = J.iter(hist_msgs_0);
    const _loop4 = J.loop(_items4);
    for (let _i4 = 0; _i4 < _items4.length; _i4++) {
      _loop4.index0 = _i4; const _it4 = _items4[_i4]; let message_13 = _it4;
      let tool_call_id_14 = tool_call_id_0;
      out.push(J.str(J.call(set_roles_0, message_13)));
      if (J.t((J.eq(J.item(message_13, "role"), "assistant")))) {
        out.push("<think></think>", J.str(J.call(render_content_0, message_13)));
        if (J.t(J.call(J.attr(message_13, "get"), "tool_calls"))) {
          out.push(J.str(J.call(render_toolcalls_0, message_13)));
        }
      } else if (J.t((J.eq(J.item(message_13, "role"), "tool")))) {
        tool_call_id_14 = J.attr(message_13, "tool_call_id");
        out.push("## Return of ", J.str(tool_call_id_14), "\n", J.str(J.call(render_content_0, message_13)));
      } else if (J.t(!J.t((J.item(message_13, "content") === null)))) {
        out.push(J.str(J.call(render_content_0, message_13)));
      }
      out.push("<|im_end|>");
    }
  }
  {
    let _items5 = J.iter(suffix_msgs_0);
    const _loop5 = J.loop(_items5);
    for (let _i5 = 0; _i5 < _items5.length; _i5++) {
      _loop5.index0 = _i5; const _it5 = _items5[_i5]; let message_15 = _it5;
      let rc_16 = rc_0;
      let tool_call_id_17 = tool_call_id_0;
      out.push(J.str(J.call(set_roles_0, message_15)));
      if (J.t((J.eq(J.item(message_15, "role"), "assistant")))) {
        rc_16 = J.call(J.attr(message_15, "get"), "reasoning_content", "");
        out.push("<think>", J.str(rc_16), "</think>", J.str(J.call(render_content_0, message_15)));
        if (J.t(J.call(J.attr(message_15, "get"), "tool_calls"))) {
          out.push(J.str(J.call(render_toolcalls_0, message_15)));
        }
      } else if (J.t((J.eq(J.item(message_15, "role"), "tool")))) {
        tool_call_id_17 = J.attr(message_15, "tool_call_id");
        out.push("## Return of ", J.str(tool_call_id_17), "\n", J.str(J.call(render_content_0, message_15)));
      } else if (J.t(!J.t((J.item(message_15, "content") === null)))) {
        out.push(J.str(J.call(render_content_0, message_15)));
      }
      out.push("<|im_end|>");
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push("<|im_assistant|>assistant<|im_middle|>");
  }
  return out.join("");
}

// What the model's own code does to the inputs before its template runs.
export default function Main(given) {
  const ctx = ({...given, tools: J.deep_sort(given.tools ?? null)});
  return Template(ctx);
}
