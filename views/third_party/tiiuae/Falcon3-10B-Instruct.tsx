// Converted from the chat template of tiiuae/Falcon3-10B-Instruct by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let remaining_messages_0 = ctx["remaining_messages"];
  let tools_0 = ctx["tools"] ?? null;
  let messages_0 = ctx["messages"];
  let eos_token_0 = ctx["eos_token"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  if (J.t(tools_0)) {
    out.push(J.str("<|system|>\n"));
    if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
      out.push(J.str(J.item(J.item(messages_0, 0), "content")));
      remaining_messages_0 = J.slice(messages_0, 1, undefined, undefined);
    } else {
      remaining_messages_0 = messages_0;
    }
    out.push(J.str(J.add(J.add("You are a Falcon assistant skilled in function calling. You are helpful, respectful, and concise.\n\n# Tools\n\nYou have access to the following functions. You MUST use them to answer questions when needed. For each function call, you MUST return a JSON object inside <tool_call></tool_call> tags.\n\n<tools>", J.filter("tojson", tools_0, J.kw({"indent": 2}))), "</tools>\n\n# Output Format\n\nYour response MUST follow this format when making function calls:\n<tool_call>\n[\n  {\"name\": \"function_name\", \"arguments\": {\"arg1\": \"value1\", \"arg2\": \"value2\"}},\n  {\"name\": \"another_function\", \"arguments\": {\"arg\": \"value\"}}\n]\n</tool_call>\nIf no function calls are needed, respond normally without the tool_call tags.\n")));
    {
      let _items1 = J.iter(remaining_messages_0);
      const _loop1 = J.loop(_items1);
      for (let _i1 = 0; _i1 < _items1.length; _i1++) {
        _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
        if (J.t((J.eq(J.item(message_1, "role"), "user")))) {
          out.push(J.str(J.add(J.add("<|user|>\n", J.item(message_1, "content")), "\n")));
        } else if (J.t((J.eq(J.item(message_1, "role"), "assistant")))) {
          if (J.t(J.attr(message_1, "content"))) {
            out.push(J.str(J.add("<|assistant|>\n", J.item(message_1, "content"))));
          }
          if (J.t(J.attr(message_1, "tool_calls"))) {
            out.push(J.str("\n<tool_call>\n"), J.str(J.filter("tojson", J.attr(message_1, "tool_calls"), J.kw({"indent": 2}))), J.str("\n</tool_call>"));
          }
          out.push(J.str(J.add(eos_token_0, "\n")));
        } else if (J.t((J.eq(J.item(message_1, "role"), "tool")))) {
          out.push(J.str(J.add(J.add("<|assistant|>\n<tool_response>\n", J.item(message_1, "content")), "\n</tool_response>\n")));
        }
      }
    }
    out.push(J.str((J.t(add_generation_prompt_0) ? "<|assistant|>\n" : undefined)));
  } else {
    {
      let _items2 = J.iter(messages_0);
      const _loop2 = J.loop(_items2);
      for (let _i2 = 0; _i2 < _items2.length; _i2++) {
        _loop2.index0 = _i2; const _it2 = _items2[_i2]; let message_2 = _it2;
        if (J.t((J.eq(J.item(message_2, "role"), "system")))) {
          out.push(J.str(J.add(J.add("<|system|>\n", J.item(message_2, "content")), "\n")));
        } else if (J.t((J.eq(J.item(message_2, "role"), "user")))) {
          out.push(J.str(J.add(J.add("<|user|>\n", J.item(message_2, "content")), "\n")));
        } else if (J.t((J.eq(J.item(message_2, "role"), "assistant")))) {
          if (J.t(!J.t(J.attr(_loop2, "last")))) {
            out.push(J.str(J.add(J.add(J.add("<|assistant|>\n", J.item(message_2, "content")), eos_token_0), "\n")));
          } else {
            out.push(J.str(J.add(J.add("<|assistant|>\n", J.item(message_2, "content")), eos_token_0)));
          }
        }
        if (J.t(((v) => J.t(v) ? add_generation_prompt_0 : v)(J.attr(_loop2, "last")))) {
          out.push(J.str("<|assistant|>\n"));
        }
      }
    }
  }
  return out.join("");
}
