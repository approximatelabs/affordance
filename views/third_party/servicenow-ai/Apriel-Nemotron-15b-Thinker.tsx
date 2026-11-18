// Converted from the chat template of ServiceNow-AI/Apriel-Nemotron-15b-Thinker by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let available_tool_string_0 = ctx["available_tool_string"];
  let add_tool_id_0 = ctx["add_tool_id"];
  let reasoning_prompt_0 = ctx["reasoning_prompt"];
  let reasoning_asst_turn_start_0 = ctx["reasoning_asst_turn_start"];
  let available_tools_string_0 = ctx["available_tools_string"];
  let tools_0 = ctx["tools"] ?? null;
  let messages_0 = ctx["messages"];
  let training_prompt_0 = ctx["training_prompt"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  [available_tool_string_0, add_tool_id_0] = J.unpack(["", true], 2);
  reasoning_prompt_0 = "You are a thoughtful and systematic AI assistant built by ServiceNow Language Models (SLAM) lab. Before providing an answer, analyze the problem carefully and present your reasoning step by step. After explaining your thought process, provide the final solution in the following format: [BEGIN FINAL RESPONSE] ... [END FINAL RESPONSE].";
  reasoning_asst_turn_start_0 = "Here are my reasoning steps:\n";
  if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", tools_0), 0)) : v)(!J.t((tools_0 === null))))) {
    {
      const captured = (() => {
      const out = [];
      out.push("You are provided with function signatures within <available_tools></available_tools> XML tags. You may call one or more functions to assist with the user query. Don't make assumptions about the arguments. You should infer the argument values from previous user responses and the system message. Here are the available tools: <available_tools>");
      {
        let _items1 = J.iter(tools_0);
        const _loop1 = J.loop(_items1);
        for (let _i1 = 0; _i1 < _items1.length; _i1++) {
          _loop1.index0 = _i1; const _it1 = _items1[_i1]; let tool_1 = _it1;
          out.push(J.str(J.filter("string", tool_1)));
        }
      }
      out.push("</available_tools>.\n\nReturn all function calls as a list of json objects within <tool_calls></tool_calls> XML tags. Each json object should contain a function name and arguments as follows: <tool_calls>[{\"name\": <function-name-1>, \"arguments\": <args-dict-1>}, {\"name\": <function-name-2>, \"arguments\": <args-dict-2>},...]</tool_calls>");
      return out.join("");
      })();
      available_tools_string_0 = captured;
    }
  }
  if (J.t((!J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
    if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", tools_0), 0)) : v)(!J.t((tools_0 === null))))) {
      out.push(J.str(J.add(J.add(J.add(J.add("<|system|>\n", reasoning_prompt_0), "\n\n"), available_tools_string_0), "\n<|end|>\n")));
    } else {
      out.push(J.str(J.add(J.add("<|system|>\n", reasoning_prompt_0), "\n<|end|>\n")));
    }
  }
  {
    let _items2 = J.iter(messages_0);
    const _loop2 = J.loop(_items2);
    for (let _i2 = 0; _i2 < _items2.length; _i2++) {
      _loop2.index0 = _i2; const _it2 = _items2[_i2]; let message_2 = _it2;
      let add_tool_id_3 = add_tool_id_0;
      if (J.t((J.eq(J.item(message_2, "role"), "user")))) {
        out.push(J.str(J.add(J.add("<|user|>\n", J.item(message_2, "content")), "\n<|end|>\n")));
      } else if (J.t((J.eq(J.item(message_2, "role"), "system")))) {
        if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", tools_0), 0)) : v)(!J.t((tools_0 === null))))) {
          out.push(J.str(J.add(J.add(J.add(J.add(J.add(J.add("<|system|>\n", reasoning_prompt_0), "\n\n"), J.item(message_2, "content")), "\n"), available_tools_string_0), "\n<|end|>\n")));
        } else {
          out.push(J.str(J.add(J.add(J.add(J.add("<|system|>\n", reasoning_prompt_0), "\n\n"), J.item(message_2, "content")), "\n<|end|>\n")));
        }
      } else if (J.t((J.eq(J.item(message_2, "role"), "assistant")))) {
        if (J.t(J.attr(_loop2, "last"))) {
          add_tool_id_3 = false;
        }
        out.push(J.str("<|assistant|>\n"));
        if (J.t(!J.t((J.item(message_2, "content") === null)))) {
          out.push(J.str(J.item(message_2, "content")));
        } else if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", J.item(message_2, "chosen")), 0)) : v)(!J.t((J.item(message_2, "chosen") === null))))) {
          out.push(J.str(J.item(J.item(message_2, "chosen"), 0)));
        }
        if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", J.item(message_2, "tool_calls")), 0)) : v)(!J.t((J.item(message_2, "tool_calls") === null))))) {
          out.push(J.str("\n<tool_calls>["));
          {
            let _items3 = J.iter(J.item(message_2, "tool_calls"));
            const _loop3 = J.loop(_items3);
            for (let _i3 = 0; _i3 < _items3.length; _i3++) {
              _loop3.index0 = _i3; const _it3 = _items3[_i3]; let tool_call_4 = _it3;
              out.push(J.str(J.add(J.add(J.add("{\"name\": \"", J.item(J.item(tool_call_4, "function"), "name")), "\", \"arguments\": "), J.filter("string", J.item(J.item(tool_call_4, "function"), "arguments")))));
              if (J.t((J.eq(add_tool_id_3, true)))) {
                out.push(J.str(J.add(", \"id\": \"", J.item(tool_call_4, "id"))));
              }
              out.push(J.str("}"));
              if (J.t(!J.t(J.attr(_loop3, "last")))) {
                out.push(J.str(", "));
              }
            }
          }
          out.push(J.str("]</tool_calls>"));
        }
        if (J.t(((v) => J.t(v) ? v : training_prompt_0)(!J.t(J.attr(_loop2, "last"))))) {
          out.push(J.str("\n<|end|>\n"));
        }
      } else if (J.t((J.eq(J.item(message_2, "role"), "tool")))) {
        out.push(J.str(J.add(J.add("<|tool_result|>\n", J.filter("string", J.item(message_2, "content"))), "\n<|end|>\n")));
      }
      if (J.t(((v) => J.t(v) ? (!J.eq(J.item(message_2, "role"), "assistant")) : v)(((v) => J.t(v) ? add_generation_prompt_0 : v)(J.attr(_loop2, "last"))))) {
        out.push(J.str(J.add("<|assistant|>\n", reasoning_asst_turn_start_0)));
      }
    }
  }
  return out.join("");
}
