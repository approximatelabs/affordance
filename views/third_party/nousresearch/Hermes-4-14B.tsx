// Converted from the chat template of NousResearch/Hermes-4-14B by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let thinking_prompt_0 = ctx["thinking_prompt"];
  let standard_prompt_0 = ctx["standard_prompt"];
  let thinking_0 = ctx["thinking"];
  let keep_cots_0 = ctx["keep_cots"];
  let system_prompt_0 = ctx["system_prompt"];
  let tools_0 = ctx["tools"] ?? null;
  let messages_0 = ctx["messages"];
  let content_0 = ctx["content"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  thinking_prompt_0 = "You are a deep thinking AI, you may use extremely long chains of thought to deeply consider the problem and deliberate with yourself via systematic reasoning processes to help come to a correct solution prior to answering. You should enclose your thoughts and internal monologue inside <think> </think> tags, and then provide your solution or response to the problem.";
  standard_prompt_0 = "You are Hermes, created by Nous Research.";
  if (J.t(!J.t((thinking_0 !== undefined)))) {
    thinking_0 = false;
  }
  if (J.t(!J.t((keep_cots_0 !== undefined)))) {
    keep_cots_0 = false;
  }
  if (J.t(thinking_0)) {
    system_prompt_0 = thinking_prompt_0;
  } else {
    system_prompt_0 = standard_prompt_0;
  }
  if (J.t(tools_0)) {
    out.push(J.str("<|im_start|>system\n"));
    if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
      out.push(J.str(J.item(J.item(messages_0, 0), "content")));
    } else {
      out.push(J.str(system_prompt_0));
    }
    out.push(J.str("\n\n# Tools\n\nYou are a function calling AI model. You may call one or more functions to assist with the user query.\n\nYou are provided with function signatures within <tools></tools> XML tags:\n<tools>"));
    {
      let _items1 = J.iter(tools_0);
      const _loop1 = J.loop(_items1);
      for (let _i1 = 0; _i1 < _items1.length; _i1++) {
        _loop1.index0 = _i1; const _it1 = _items1[_i1]; let tool_1 = _it1;
        out.push(J.str("\n"), J.str(J.filter("tojson", tool_1)));
      }
    }
    out.push(J.str("\n</tools>\n\nFor each function call, return a json object with function name and arguments within <tool_call></tool_call> XML tags:\n<tool_call>\n{\"name\": \"<function-name>\", \"arguments\": <args-json-object>}\n</tool_call><|im_end|>\n"));
  } else {
    if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
      out.push(J.str(J.add(J.add("<|im_start|>system\n", J.item(J.item(messages_0, 0), "content")), "<|im_end|>\n")));
    } else {
      out.push(J.str(J.add(J.add("<|im_start|>system\n", system_prompt_0), "<|im_end|>\n")));
    }
  }
  {
    let _items2 = J.iter(messages_0);
    const _loop2 = J.loop(_items2);
    for (let _i2 = 0; _i2 < _items2.length; _i2++) {
      _loop2.index0 = _i2; const _it2 = _items2[_i2]; let message_2 = _it2;
      let content_3 = content_0;
      if (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? !J.t(J.attr(_loop2, "first")) : v)((J.eq(J.attr(message_2, "role"), "system"))))((J.eq(J.attr(message_2, "role"), "user"))))) {
        out.push(J.str(J.add(J.add(J.add(J.add(J.add("<|im_start|>", J.attr(message_2, "role")), "\n"), J.attr(message_2, "content")), "<|im_end|>"), "\n")));
      } else if (J.t(((v) => J.t(v) ? !J.t(J.attr(message_2, "tool_calls")) : v)((J.eq(J.attr(message_2, "role"), "assistant"))))) {
        out.push(J.str(J.add("<|im_start|>", J.attr(message_2, "role"))));
        if (J.t(J.attr(message_2, "content"))) {
          content_3 = J.item(message_2, "content");
          if (J.t(thinking_0)) {
            if (J.t(!J.t(keep_cots_0))) {
              content_3 = J.add("<think> </think>", J.item(J.call(J.attr(content_3, "split"), "</think>", 1), 1));
            }
          }
          out.push(J.str(J.add(J.add(J.add("\n", content_3), "<|im_end|>"), "\n")));
        }
      } else if (J.t((J.eq(J.attr(message_2, "role"), "assistant")))) {
        out.push(J.str(J.add("<|im_start|>", J.attr(message_2, "role"))));
        if (J.t(J.attr(message_2, "content"))) {
          content_3 = J.item(message_2, "content");
          if (J.t(thinking_0)) {
            if (J.t(!J.t(keep_cots_0))) {
              content_3 = J.add("<think> </think>", J.item(J.call(J.attr(content_3, "split"), "</think>", 1), 1));
            }
          }
          out.push(J.str(J.add("\n", content_3)));
        }
        {
          let _items3 = J.iter(J.attr(message_2, "tool_calls"));
          const _loop3 = J.loop(_items3);
          for (let _i3 = 0; _i3 < _items3.length; _i3++) {
            _loop3.index0 = _i3; const _it3 = _items3[_i3]; let tool_call_4 = _it3;
            if (J.t((J.attr(tool_call_4, "function") !== undefined))) {
              tool_call_4 = J.attr(tool_call_4, "function");
            }
            out.push(J.str("\n<tool_call>\n{\"name\": \""), J.str(J.attr(tool_call_4, "name")), J.str("\", \"arguments\": "), J.str(J.filter("tojson", J.attr(tool_call_4, "arguments"))), J.str("}\n</tool_call>"));
          }
        }
        out.push(J.str("<|im_end|>\n"));
      } else if (J.t((J.eq(J.attr(message_2, "role"), "tool")))) {
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.sub(J.attr(_loop2, "index0"), 1)), "role"), "tool")))((J.eq(J.attr(_loop2, "index0"), 0))))) {
          out.push(J.str("<|im_start|>user"));
        }
        out.push(J.str("\n<tool_response>\n"), J.str(J.attr(message_2, "content")), J.str("\n</tool_response>"));
        if (J.t(((v) => J.t(v) ? v : (!J.eq(J.attr(J.item(messages_0, J.add(J.attr(_loop2, "index0"), 1)), "role"), "tool")))(J.attr(_loop2, "last")))) {
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
