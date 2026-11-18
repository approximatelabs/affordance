// Converted from the chat template of paradigma-inc/limite-1b-violetto by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let system_prompt_0 = ctx["system_prompt"];
  let tools_0 = ctx["tools"] ?? null;
  let messages_0 = ctx["messages"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  system_prompt_0 = "You are a helpful assistant.\nPlease reason step by step, and put your final answer within \\boxed{}.";
  if (J.t(tools_0)) {
    out.push(J.str(J.call(J.raise_exception, "Limite Violetto supports text-only chat; tools are not supported.")));
  }
  out.push(J.str(J.add(J.add("<|endoftext|><|im_start|>system\n", system_prompt_0), "<|im_end|>\n")));
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      if (J.t(((v) => J.t(v) ? v : ((v) => J.t(v) ? !J.t((J.attr(message_1, "function_call") === null)) : v)((J.attr(message_1, "function_call") !== undefined)))(((v) => J.t(v) ? !J.t((J.attr(message_1, "tool_calls") === null)) : v)((J.attr(message_1, "tool_calls") !== undefined))))) {
        out.push(J.str(J.call(J.raise_exception, "Limite Violetto supports text-only chat; tool calls are not supported.")));
      }
      if (J.t(!J.t(J.test("string", J.attr(message_1, "content"))))) {
        out.push(J.str(J.call(J.raise_exception, "Limite Violetto supports text-only chat; message content must be a string.")));
      }
      if (J.t((J.eq(J.attr(message_1, "role"), "system")))) {
        if (J.t((!J.eq(J.attr(message_1, "content"), system_prompt_0)))) {
          out.push(J.str(J.call(J.raise_exception, "Limite Violetto requires its fixed mathematical system prompt; custom system messages are not supported.")));
        }
      } else if (J.t((J.eq(J.attr(message_1, "role"), "user")))) {
        out.push(J.str(J.add(J.add("<|im_start|>user\n", J.attr(message_1, "content")), "<|im_end|>\n")));
      } else if (J.t((J.eq(J.attr(message_1, "role"), "assistant")))) {
        out.push(J.str("<|im_start|>assistant\n"));
        if (J.t(((v) => J.t(v) ? J.attr(message_1, "reasoning_content") : v)(J.test("string", J.attr(message_1, "reasoning_content"))))) {
          out.push(J.str(J.add(J.add("<think>\n", J.attr(message_1, "reasoning_content")), "\n</think>\n\n")));
        }
        out.push(J.str(J.add(J.attr(message_1, "content"), "<|im_end|>\n")));
      } else {
        out.push(J.str(J.call(J.raise_exception, "Limite Violetto supports only user, assistant, and the exact canonical system message; other roles are not supported.")));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|im_start|>assistant\n"));
  }
  return out.join("");
}
