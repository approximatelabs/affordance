// Converted from the chat template of moonshotai/Kimi-Linear-48B-A3B-Instruct by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

function Template(ctx) {
  const out = [];
  let render_content_0 = ctx["render_content"];
  let c_0 = ctx["c"];
  let tools_0 = ctx["tools"] ?? null;
  let messages_0 = ctx["messages"];
  let role_name_0 = ctx["role_name"];
  let tool_call_id_0 = ctx["tool_call_id"];
  let formatted_id_0 = ctx["formatted_id"];
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
  if (J.t(tools_0)) {
    out.push("<|im_system|>tool_declare<|im_middle|>", J.str(J.filter("tojson", tools_0, J.kw({"separators": [",", ":"]}))), "<|im_end|>");
  }
  {
    let _items2 = J.iter(messages_0);
    const _loop2 = J.loop(_items2);
    for (let _i2 = 0; _i2 < _items2.length; _i2++) {
      _loop2.index0 = _i2; const _it2 = _items2[_i2]; let message_5 = _it2;
      let role_name_6 = role_name_0;
      let tool_call_id_7 = tool_call_id_0;
      role_name_6 = ((v) => J.t(v) ? v : J.item(message_5, "role"))(J.call(J.attr(message_5, "get"), "name"));
      if (J.t((J.eq(J.item(message_5, "role"), "user")))) {
        out.push("<|im_user|>", J.str(role_name_6), "<|im_middle|>");
      } else if (J.t((J.eq(J.item(message_5, "role"), "assistant")))) {
        out.push("<|im_assistant|>", J.str(role_name_6), "<|im_middle|>");
      } else {
        out.push("<|im_system|>", J.str(role_name_6), "<|im_middle|>");
      }
      if (J.t(((v) => J.t(v) ? J.call(J.attr(message_5, "get"), "tool_calls") : v)((J.eq(J.item(message_5, "role"), "assistant"))))) {
        out.push(J.str(J.call(render_content_0, message_5)), "<|tool_calls_section_begin|>");
        {
          let _items3 = J.iter(J.item(message_5, "tool_calls"));
          const _loop3 = J.loop(_items3);
          for (let _i3 = 0; _i3 < _items3.length; _i3++) {
            _loop3.index0 = _i3; const _it3 = _items3[_i3]; let tool_call_8 = _it3;
            let formatted_id_9 = formatted_id_0;
            formatted_id_9 = J.item(tool_call_8, "id");
            out.push("<|tool_call_begin|>", J.str(formatted_id_9), "<|tool_call_argument_begin|>");
            if (J.t(J.test("string", J.item(J.item(tool_call_8, "function"), "arguments")))) {
              out.push(J.str(J.item(J.item(tool_call_8, "function"), "arguments")));
            } else {
              out.push(J.str(J.filter("tojson", J.item(J.item(tool_call_8, "function"), "arguments"))));
            }
            out.push("<|tool_call_end|>");
          }
        }
        out.push("<|tool_calls_section_end|>");
      } else if (J.t((J.eq(J.item(message_5, "role"), "tool")))) {
        tool_call_id_7 = J.attr(message_5, "tool_call_id");
        out.push("## Return of ", J.str(tool_call_id_7), "\n", J.str(J.call(render_content_0, message_5)));
      } else if (J.t(!J.t((J.item(message_5, "content") === null)))) {
        out.push(J.str(J.call(render_content_0, message_5)));
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
