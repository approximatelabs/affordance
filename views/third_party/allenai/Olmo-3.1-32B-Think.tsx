// Converted from the chat template of allenai/Olmo-3.1-32B-Think by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let has_system_0 = ctx["has_system"];
  let messages_0 = ctx["messages"];
  let eos_token_0 = ctx["eos_token"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  has_system_0 = (J.gt(J.filter("length", J.filter("list", J.filter("selectattr", messages_0, "role", "equalto", "system"))), 0));
  if (J.t(!J.t(has_system_0))) {
    out.push(J.str("<|im_start|>system\nYou are Olmo, a helpful AI assistant built by Ai2. Your date cutoff is December 2024, and your model weights are available at https://huggingface.co/allenai.<|im_end|>\n"));
  }
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      if (J.t((J.eq(J.item(message_1, "role"), "system")))) {
        out.push(J.str(J.add("<|im_start|>system\n", J.item(message_1, "content"))));
        if (J.t(!J.t((J.call(J.attr(message_1, "get"), "functions", null) === null)))) {
          out.push(J.str(J.add(J.add(" <functions>", J.item(message_1, "functions")), "</functions><|im_end|>\n")));
        } else {
          out.push(J.str(" You do not currently have access to any functions. <functions></functions><|im_end|>\n"));
        }
      } else if (J.t((J.eq(J.item(message_1, "role"), "user")))) {
        if (J.t(!J.t((J.call(J.attr(message_1, "get"), "functions", null) === null)))) {
          out.push(J.str(J.add(J.add(J.add(J.add(J.add("<|im_start|>user\n", J.item(message_1, "content")), "\n"), "<functions>"), J.item(message_1, "functions")), "</functions><|im_end|>\n")));
        } else {
          out.push(J.str(J.add(J.add("<|im_start|>user\n", J.item(message_1, "content")), "<|im_end|>\n")));
        }
      } else if (J.t((J.eq(J.item(message_1, "role"), "assistant")))) {
        out.push(J.str("<|im_start|>assistant\n"));
        if (J.t(!J.t((J.call(J.attr(message_1, "get"), "content", null) === null)))) {
          out.push(J.str(J.item(message_1, "content")));
        }
        if (J.t(!J.t((J.call(J.attr(message_1, "get"), "function_calls", null) === null)))) {
          out.push(J.str(J.add(J.add("<function_calls>", J.item(message_1, "function_calls")), "</function_calls>")));
        }
        if (J.t(!J.t(J.attr(_loop1, "last")))) {
          out.push(J.str(J.add("<|im_end|>", "\n")));
        } else {
          out.push(J.str(eos_token_0));
        }
      } else if (J.t((J.eq(J.item(message_1, "role"), "environment")))) {
        out.push(J.str(J.add(J.add("<|im_start|>environment\n", J.item(message_1, "content")), "<|im_end|>\n")));
      }
      if (J.t(((v) => J.t(v) ? add_generation_prompt_0 : v)(J.attr(_loop1, "last")))) {
        out.push(J.str("<|im_start|>assistant\n<think>"));
      }
    }
  }
  return out.join("");
}
