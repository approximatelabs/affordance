// Converted from the chat template of internlm/Intern-S1 by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let default_thinking_sys_0 = ctx["default_thinking_sys"];
  let tool_instruction_0 = ctx["tool_instruction"];
  let ns_0 = ctx["ns"];
  let tools_0 = ctx["tools"] ?? null;
  let messages_0 = ctx["messages"];
  let index_0 = ctx["index"];
  let role_0 = ctx["role"];
  let reasoning_content_0 = ctx["reasoning_content"];
  let content_0 = ctx["content"];
  let system_prompt_0 = ctx["system_prompt"];
  let enable_thinking_0 = ctx["enable_thinking"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  {
    const captured = (() => {
    const out = [];
    out.push("You are an expert reasoner with extensive experience in all areas. You approach problems through systematic thinking and rigorous reasoning. Your response should reflect deep understanding and precise logical thinking, making your solution path and reasoning clear to others. Please put your thinking process within <think>...</think> tags.");
    return out.join("");
    })();
    default_thinking_sys_0 = captured;
  }
  {
    const captured = (() => {
    const out = [];
    out.push("Your response should consist of a reasoning step (**thought**) followed immediately by a function call in valid JSON format. Wrap each function call using the `<|action_start|><|plugin|>` and `<|action_end|>` tags.\n\n**Format example:**\n\n```\n(Your thought goes here...)\n\n<|action_start|><|plugin|>\n{\n    \"name\": \"tool_name\",\n    \"parameters\": {\n        \"parameter1\": \"value1\",\n        \"parameter2\": \"value2\"\n    }\n}\n<|action_end|>\n```\n\n# External Tools\nYou have access to these tools:\n");
    if (J.t(tools_0)) {
      out.push(J.str(J.filter("tojson", tools_0, J.kw({"indent": 2}))));
    } else {
      out.push("[]");
    }
    return out.join("");
    })();
    tool_instruction_0 = captured;
  }
  ns_0 = J.call(J.namespace, J.kw({"multi_step_tool": true, "last_query_index": J.sub(J.filter("length", messages_0), 1)}));
  {
    let _items1 = J.iter(J.slice(messages_0, undefined, undefined, J.neg(1)));
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      let index_2 = index_0;
      index_2 = J.sub(J.sub(J.filter("length", messages_0), 1), J.attr(_loop1, "index0"));
      if (J.t(((v) => J.t(v) ? (J.eq(J.attr(message_1, "role"), "user")) : v)(J.attr(ns_0, "multi_step_tool")))) {
        J.setattr(ns_0, "multi_step_tool", false);
        J.setattr(ns_0, "last_query_index", index_2);
      }
    }
  }
  {
    let _items2 = J.iter(messages_0);
    const _loop2 = J.loop(_items2);
    for (let _i2 = 0; _i2 < _items2.length; _i2++) {
      _loop2.index0 = _i2; const _it2 = _items2[_i2]; let message_3 = _it2;
      let role_4 = role_0;
      let reasoning_content_5 = reasoning_content_0;
      let content_6 = content_0;
      let system_prompt_7 = system_prompt_0;
      role_4 = (J.t((!J.eq(J.attr(message_3, "role"), "tool"))) ? J.attr(message_3, "role") : "environment");
      reasoning_content_5 = "";
      content_6 = J.attr(message_3, "content");
      J.setattr(ns_0, "tool_calls", "");
      if (J.t((J.eq(role_4, "assistant")))) {
        if (J.t(J.test("string", J.attr(message_3, "reasoning_content")))) {
          reasoning_content_5 = J.attr(message_3, "reasoning_content");
        } else if (J.t((J.has("</think>", content_6)))) {
          reasoning_content_5 = J.call(J.attr(J.item(J.call(J.attr(J.call(J.attr(J.item(J.call(J.attr(content_6, "split"), "</think>"), 0), "strip")), "split"), "<think>"), J.neg(1)), "strip"));
          content_6 = J.call(J.attr(J.item(J.call(J.attr(content_6, "split"), "</think>"), J.neg(1)), "lstrip"), "\n");
        }
        if (J.t(J.attr(message_3, "tool_calls"))) {
          {
            let _items3 = J.iter(J.attr(message_3, "tool_calls"));
            const _loop3 = J.loop(_items3);
            for (let _i3 = 0; _i3 < _items3.length; _i3++) {
              _loop3.index0 = _i3; const _it3 = _items3[_i3]; let tool_call_8 = _it3;
              if (J.t(!J.t(J.attr(_loop3, "first")))) {
                J.setattr(ns_0, "tool_calls", J.add(J.attr(ns_0, "tool_calls"), "\n"));
              }
              if (J.t(J.attr(tool_call_8, "function"))) {
                tool_call_8 = J.attr(tool_call_8, "function");
              }
              J.setattr(ns_0, "tool_calls", J.add(J.add(J.add(J.attr(ns_0, "tool_calls"), "<|action_start|><|plugin|>\n{\"name\": \""), J.attr(tool_call_8, "name")), "\", \"parameters\": "));
              if (J.t(J.test("string", J.attr(tool_call_8, "arguments")))) {
                J.setattr(ns_0, "tool_calls", J.add(J.attr(ns_0, "tool_calls"), J.attr(tool_call_8, "arguments")));
              } else {
                J.setattr(ns_0, "tool_calls", J.add(J.attr(ns_0, "tool_calls"), J.filter("tojson", J.attr(tool_call_8, "arguments"))));
              }
              J.setattr(ns_0, "tool_calls", J.add(J.attr(ns_0, "tool_calls"), "}\n<|action_end|>"));
            }
          }
        }
        reasoning_content_5 = J.add(J.add("<think>\n", J.call(J.attr(reasoning_content_5, "strip"), "\n")), "\n</think>\n");
      }
      if (J.t(!J.t(J.test("string", content_6)))) {
        J.setattr(ns_0, "content", "");
        {
          let _items4 = J.iter(J.attr(message_3, "content"));
          const _loop4 = J.loop(_items4);
          for (let _i4 = 0; _i4 < _items4.length; _i4++) {
            _loop4.index0 = _i4; const _it4 = _items4[_i4]; let _content_9 = _it4;
            if (J.t((J.eq(J.attr(_content_9, "type"), "image")))) {
              J.setattr(ns_0, "content", J.concat(J.attr(ns_0, "content"), "\n<IMG_CONTEXT>"));
            } else if (J.t((J.eq(J.attr(_content_9, "type"), "video")))) {
              J.setattr(ns_0, "content", J.concat(J.attr(ns_0, "content"), "\n<video>"));
            } else if (J.t((J.eq(J.attr(_content_9, "type"), "text")))) {
              J.setattr(ns_0, "content", J.concat(J.attr(ns_0, "content"), "\n", J.attr(_content_9, "text")));
            }
          }
        }
        content_6 = J.attr(ns_0, "content");
      }
      content_6 = J.call(J.attr(content_6, "lstrip"), "\n");
      if (J.t(J.attr(ns_0, "tool_calls"))) {
        content_6 = J.add(content_6, J.attr(ns_0, "tool_calls"));
      }
      if (J.t((J.eq(J.attr(_loop2, "index0"), 0)))) {
        system_prompt_7 = "";
        if (J.t((J.eq(role_4, "system")))) {
          system_prompt_7 = J.add(system_prompt_7, content_6);
        } else if (J.t(((v) => J.t(v) ? v : enable_thinking_0)(!J.t((enable_thinking_0 !== undefined))))) {
          system_prompt_7 = J.add(system_prompt_7, default_thinking_sys_0);
        }
        if (J.t(tools_0)) {
          system_prompt_7 = J.add(J.add(J.call(J.attr(system_prompt_7, "rstrip"), "\n"), "\n\n"), tool_instruction_0);
        }
        system_prompt_7 = J.call(J.attr(system_prompt_7, "strip"), "\n");
      }
      if (J.t(((v) => J.t(v) ? system_prompt_7 : v)((J.eq(J.attr(_loop2, "index0"), 0))))) {
        out.push("<|im_start|>system");
        if (J.t(tools_0)) {
          out.push(" name=<|plugin|>");
        }
        out.push("\n", J.str(system_prompt_7), "<|im_end|>\n");
      }
      if (J.t((!J.eq(role_4, "system")))) {
        out.push("<|im_start|>", J.str(role_4));
        if (J.t(((v) => J.t(v) ? v : (J.eq(role_4, "tool")))((J.eq(role_4, "environment"))))) {
          out.push(" name=<|plugin|>");
        }
        out.push("\n");
        if (J.t(((v) => J.t(v) ? ((v) => J.t(v) ? v : ((v) => J.t(v) ? reasoning_content_5 : v)(!J.t(J.attr(_loop2, "last"))))(J.attr(_loop2, "last")) : v)((J.gt(J.attr(_loop2, "index0"), J.attr(ns_0, "last_query_index")))))) {
          out.push(J.str(reasoning_content_5));
        }
        out.push(J.str(content_6), "<|im_end|>\n");
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push("<|im_start|>assistant\n");
    if (J.t(((v) => J.t(v) ? v : enable_thinking_0)(!J.t((enable_thinking_0 !== undefined))))) {
      out.push("<think>");
    }
  }
  return out.join("");
}
