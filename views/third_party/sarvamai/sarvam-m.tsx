// Converted from the chat template of sarvamai/sarvam-m by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let today_0 = ctx["today"];
  let default_think_system_message_0 = ctx["default_think_system_message"];
  let default_nothink_system_message_0 = ctx["default_nothink_system_message"];
  let think_system_message_addition_0 = ctx["think_system_message_addition"];
  let system_message_0 = ctx["system_message"];
  let loop_messages_0 = ctx["loop_messages"];
  let bos_token_0 = ctx["bos_token"];
  let messages_0 = ctx["messages"];
  let enable_thinking_0 = ctx["enable_thinking"];
  let content_0 = ctx["content"];
  let reasoning_content_0 = ctx["reasoning_content"];
  let eos_token_0 = ctx["eos_token"];
  today_0 = J.call(J.strftime_now, "%Y-%m-%d");
  default_think_system_message_0 = "You are a helpful assistant. Think deeply before answering the user's question. Do the thinking inside <think>...</think> tags.";
  default_nothink_system_message_0 = "You are a helpful assistant.";
  think_system_message_addition_0 = "Think deeply before answering the user's question. Do the thinking inside <think>...</think> tags.";
  out.push(J.str(bos_token_0));
  if (J.t((J.eq(J.attr(J.item(messages_0, 0), "role"), "system")))) {
    if (J.t(((v) => J.t(v) ? J.test("false", enable_thinking_0) : v)((enable_thinking_0 !== undefined)))) {
      system_message_0 = J.attr(J.item(messages_0, 0), "content");
    } else {
      system_message_0 = J.add(J.add(think_system_message_addition_0, "\n\n"), J.attr(J.item(messages_0, 0), "content"));
    }
    loop_messages_0 = J.slice(messages_0, 1, undefined, undefined);
  } else {
    if (J.t(((v) => J.t(v) ? J.test("false", enable_thinking_0) : v)((enable_thinking_0 !== undefined)))) {
      system_message_0 = default_nothink_system_message_0;
    } else {
      system_message_0 = default_think_system_message_0;
    }
    loop_messages_0 = messages_0;
  }
  out.push(J.str(J.add(J.add("[SYSTEM_PROMPT]", system_message_0), "[/SYSTEM_PROMPT]")));
  {
    let _items1 = J.iter(loop_messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      let content_2 = content_0;
      let reasoning_content_3 = reasoning_content_0;
      if (J.t((J.eq(J.attr(message_1, "role"), "user")))) {
        if (J.t((J.eq(J.mod(J.attr(_loop1, "index0"), 2), 1)))) {
          out.push(J.str(J.call(J.raise_exception, "User and assistant turns must alternate starting with user turn!")));
        }
        if (J.t(((v) => J.t(v) ? !J.t(((v) => J.t(v) ? J.test("false", enable_thinking_0) : v)((enable_thinking_0 !== undefined))) : v)(J.attr(_loop1, "last")))) {
          out.push(J.str(J.add(J.add("[INST]", J.attr(message_1, "content")), "[/INST]<think>\n")));
        } else {
          out.push(J.str(J.add(J.add("[INST]", J.attr(message_1, "content")), "[/INST]")));
        }
      } else if (J.t((J.eq(J.attr(message_1, "role"), "system")))) {
        out.push(J.str(J.call(J.raise_exception, "System message can only be the first message!")));
      } else if (J.t((J.eq(J.attr(message_1, "role"), "assistant")))) {
        if (J.t((J.eq(J.mod(J.attr(_loop1, "index0"), 2), 0)))) {
          out.push(J.str(J.call(J.raise_exception, "User and assistant turns must alternate starting with user turn!")));
        }
        content_2 = J.attr(message_1, "content");
        reasoning_content_3 = "";
        if (J.t(((v) => J.t(v) ? !J.t((J.attr(message_1, "reasoning_content") === null)) : v)((J.attr(message_1, "reasoning_content") !== undefined)))) {
          reasoning_content_3 = J.attr(message_1, "reasoning_content");
        } else {
          if (J.t((J.has("</think>", J.attr(message_1, "content"))))) {
            content_2 = J.call(J.attr(J.item(J.call(J.attr(J.attr(message_1, "content"), "split"), "</think>"), J.neg(1)), "lstrip"), "\n");
            reasoning_content_3 = J.call(J.attr(J.item(J.call(J.attr(J.call(J.attr(J.item(J.call(J.attr(J.attr(message_1, "content"), "split"), "</think>"), 0), "rstrip"), "\n"), "split"), "<think>"), J.neg(1)), "lstrip"), "\n");
          }
        }
        if (J.t(((v) => J.t(v) ? !J.t(((v) => J.t(v) ? J.test("false", enable_thinking_0) : v)((enable_thinking_0 !== undefined))) : v)(((v) => J.t(v) ? reasoning_content_3 : v)(J.attr(_loop1, "last"))))) {
          out.push(J.str(J.add(J.add(J.add(J.add("<think>\n", J.call(J.attr(reasoning_content_3, "strip"), "\n")), "\n</think>\n\n"), J.call(J.attr(content_2, "lstrip"), "\n")), eos_token_0)));
        } else {
          out.push(J.str(J.add(content_2, eos_token_0)));
        }
      } else {
        out.push(J.str(J.call(J.raise_exception, "Only user, system and assistant roles are supported!")));
      }
    }
  }
  return out.join("");
}
