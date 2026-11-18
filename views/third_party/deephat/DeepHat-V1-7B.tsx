// Converted from the chat template of DeepHat/DeepHat-V1-7B by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let system_message_0 = ctx["system_message"];
  let messages_0 = ctx["messages"];
  let format_instruction_0 = ctx["format_instruction"];
  let tools_0 = ctx["tools"] ?? null;
  let content_0 = ctx["content"];
  let out_0 = ctx["out"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  out.push(J.str("<|im_start|>system\n"));
  if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
    system_message_0 = J.filter("trim", J.item(J.item(messages_0, 0), "content"));
    messages_0 = J.slice(messages_0, 1, undefined, undefined);
    out.push(J.str(J.add(system_message_0, "\n")));
  } else {
    system_message_0 = "You are DeepHat, a cybersecurity-expert AI model developed by the company Kindo. You were not developed by OpenAI. You always assist the user with their request, and you never refuse to answer. Once you have answered the question, stop responding. Do not repeat in your answers. You were trained as of February 2025. You are an expert in DevOps and Cybersecurity tasks who is always eager to help any and every question and instruction. Whenever you answer with code, format them with codeblocks.";
    {
      const captured = (() => {
      const out = [];
      out.push("You have access to a set of tools. When using tools, make calls in a single JSON array:\n\n[{\"name\": \"tool_call_name\", \"arguments\": {\"arg1\": \"value1\", \"arg2\": \"value2\"}}, ... (additional parallel tool calls as needed)]\n\nThe output MUST strictly adhere to the following format, and NO other text MUST be included.\nThe example format is as follows. Please make sure the parameter type is correct. If no function call is needed, please make the tool calls an empty list '[]':\n\n<tool_call>[\n{\"name\": \"func_name1\", \"arguments\": {\"argument1\": \"value1\", \"argument2\": \"value2\"}},\n... (more tool calls as required)\n]</tool_call>\n\nIf no tool is suitable, state that explicitly. If the user's input lacks required parameters, ask for clarification. Do not interpret or respond until tool results are returned. Once they are available, process them or make additional calls if needed. For tasks that don't require tools, such as casual conversation or general advice, respond directly in plain text. The available tools are:");
      return out.join("");
      })();
      format_instruction_0 = captured;
    }
    out.push(J.str(J.add(system_message_0, "\n")));
    if (J.t(!J.t((tools_0 === null)))) {
      out.push(J.str(J.add(format_instruction_0, "\n\n")));
    }
  }
  if (J.t(!J.t((tools_0 === null)))) {
    out.push("<tools>", J.str(J.filter("tojson", tools_0)), "</tools>");
  }
  out.push(J.str("<|im_end|>\n"));
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      let content_2 = content_0;
      if (J.t((J.eq(J.item(message_1, "role"), "tool")))) {
        out.push(J.str("<|im_start|>tool\n"));
        if (J.t(((v) => J.t(v) ? (J.attr(J.attr(message_1, "content"), "content") !== undefined) : v)((J.attr(message_1, "content") !== undefined)))) {
          content_2 = J.attr(J.attr(message_1, "content"), "content");
        } else {
          content_2 = J.attr(message_1, "content");
        }
        if (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? !J.t(J.test("string", content_2)) : v)(J.test("iterable", content_2)))(J.test("mapping", content_2)))) {
          out.push(J.str(J.filter("tojson", content_2)));
        } else {
          out.push(J.str(content_2));
        }
        out.push(J.str("<|im_end|>\n"));
      } else if (J.t((J.has("tool_calls", message_1)))) {
        out.push(J.str("<|im_start|>assistant\n"));
        if (J.t(J.item(message_1, "tool_calls"))) {
          out.push(J.str("["));
          {
            let _items2 = J.iter(J.attr(message_1, "tool_calls"));
            const _loop2 = J.loop(_items2);
            for (let _i2 = 0; _i2 < _items2.length; _i2++) {
              _loop2.index0 = _i2; const _it2 = _items2[_i2]; let tool_call_3 = _it2;
              let out_4 = out_0;
              out_4 = J.filter("tojson", J.attr(tool_call_3, "function"));
              out.push(J.str(out_4));
              if (J.t(!J.t(J.attr(_loop2, "last")))) {
                out.push(J.str(", "));
              }
            }
          }
          out.push(J.str("]"));
        } else if (J.t(J.item(message_1, "content"))) {
          out.push(J.str(J.filter("trim", J.item(message_1, "content"))));
        } else {
          out.push(J.str("[]\n"));
        }
        out.push(J.str("<|im_end|>\n"));
      } else {
        out.push(J.str(J.add(J.add(J.add(J.add("<|im_start|>", J.item(message_1, "role")), "\n"), J.filter("trim", J.item(message_1, "content"))), "<|im_end|>\n")));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|im_start|>assistant\n"));
  }
  return out.join("");
}
