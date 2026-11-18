// Converted from the chat template of HuggingFaceTB/SmolLM3-3B by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let enable_thinking_0 = ctx["enable_thinking"];
  let reasoning_mode_0 = ctx["reasoning_mode"];
  let system_message_0 = ctx["system_message"];
  let custom_instructions_0 = ctx["custom_instructions"];
  let today_0 = ctx["today"];
  let xml_tools_0 = ctx["xml_tools"];
  let ns_0 = ctx["ns"];
  let xml_tool_string_0 = ctx["xml_tool_string"];
  let python_tool_string_0 = ctx["python_tool_string"];
  let messages_0 = ctx["messages"];
  let tools_0 = ctx["tools"] ?? null;
  let python_tools_0 = ctx["python_tools"];
  let content_0 = ctx["content"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  if (J.t(!J.t((enable_thinking_0 !== undefined)))) {
    enable_thinking_0 = true;
  }
  if (J.t(enable_thinking_0)) {
    reasoning_mode_0 = "/think";
  } else {
    reasoning_mode_0 = "/no_think";
  }
  out.push(J.str("<|im_start|>system\n"));
  if (J.t((J.eq(J.attr(J.item(messages_0, 0), "role"), "system")))) {
    system_message_0 = J.attr(J.item(messages_0, 0), "content");
    if (J.t((J.has("/no_think", system_message_0)))) {
      reasoning_mode_0 = "/no_think";
    } else if (J.t((J.has("/think", system_message_0)))) {
      reasoning_mode_0 = "/think";
    }
    custom_instructions_0 = J.call(J.attr(J.call(J.attr(J.call(J.attr(system_message_0, "replace"), "/no_think", ""), "replace"), "/think", ""), "rstrip"));
  }
  if (J.t((J.has("/system_override", system_message_0)))) {
    out.push(J.str(J.call(J.attr(J.call(J.attr(custom_instructions_0, "replace"), "/system_override", ""), "rstrip"))), J.str("<|im_end|>\n"));
  } else {
    out.push(J.str("## Metadata\n\n"), J.str("Knowledge Cutoff Date: June 2025\n"));
    today_0 = J.call(J.strftime_now, "%d %B %Y");
    out.push(J.str(J.concat("Today Date: ", today_0, "\n")), J.str(J.add(J.add("Reasoning Mode: ", reasoning_mode_0), "\n\n")), J.str("## Custom Instructions\n\n"));
    if (J.t(custom_instructions_0)) {
      out.push(J.str(J.add(custom_instructions_0, "\n\n")));
    } else if (J.t((J.eq(reasoning_mode_0, "/think")))) {
      out.push(J.str("You are a helpful AI assistant named SmolLM, trained by Hugging Face. Your role as an assistant involves thoroughly exploring questions through a systematic thinking process before providing the final precise and accurate solutions. This requires engaging in a comprehensive cycle of analysis, summarizing, exploration, reassessment, reflection, backtracking, and iteration to develop well-considered thinking process. Please structure your response into two main sections: Thought and Solution using the specified format: <think> Thought section </think> Solution section. In the Thought section, detail your reasoning process in steps. Each step should include detailed considerations such as analysing questions, summarizing relevant findings, brainstorming new ideas, verifying the accuracy of the current steps, refining any errors, and revisiting previous steps. In the Solution section, based on various attempts, explorations, and reflections from the Thought section, systematically present the final solution that you deem correct. The Solution section should be logical, accurate, and concise and detail necessary steps needed to reach the conclusion.\n\n"));
    } else {
      out.push(J.str("You are a helpful AI assistant named SmolLM, trained by Hugging Face.\n\n"));
    }
    if (J.t(((v) => J.t(v) ? v : tools_0)(((v) => J.t(v) ? v : python_tools_0)(xml_tools_0)))) {
      out.push(J.str("### Tools\n\n"));
      if (J.t(((v) => J.t(v) ? v : tools_0)(xml_tools_0))) {
        if (J.t(tools_0)) {
          xml_tools_0 = tools_0;
        }
        ns_0 = J.call(J.namespace, J.kw({"xml_tool_string": "You may call one or more functions to assist with the user query.\nYou are provided with function signatures within <tools></tools> XML tags:\n\n<tools>\n"}));
        {
          let _items1 = J.iter(J.slice(xml_tools_0, undefined, undefined, undefined));
          const _loop1 = J.loop(_items1);
          for (let _i1 = 0; _i1 < _items1.length; _i1++) {
            _loop1.index0 = _i1; const _it1 = _items1[_i1]; let tool_1 = _it1;
            J.setattr(ns_0, "xml_tool_string", J.concat(J.attr(ns_0, "xml_tool_string"), J.filter("string", tool_1), "\n"));
          }
        }
        xml_tool_string_0 = J.add(J.attr(ns_0, "xml_tool_string"), "</tools>\n\nFor each function call, return a json object with function name and arguments within <tool_call></tool_call> XML tags:\n<tool_call>\n{\"name\": <function-name>, \"arguments\": <args-json-object>}\n</tool_call>");
        out.push(J.str(xml_tool_string_0));
      }
      if (J.t(python_tools_0)) {
        ns_0 = J.call(J.namespace, J.kw({"python_tool_string": "When you send a message containing Python code between '<code>' and '</code>' tags, it will be executed in a stateful Jupyter notebook environment, and you will then be given the output to continued reasoning in an agentic loop.\n\nYou can use the following tools in your python code like regular functions:\n<tools>\n"}));
        {
          let _items2 = J.iter(J.slice(python_tools_0, undefined, undefined, undefined));
          const _loop2 = J.loop(_items2);
          for (let _i2 = 0; _i2 < _items2.length; _i2++) {
            _loop2.index0 = _i2; const _it2 = _items2[_i2]; let tool_2 = _it2;
            J.setattr(ns_0, "python_tool_string", J.concat(J.attr(ns_0, "python_tool_string"), J.filter("string", tool_2), "\n"));
          }
        }
        python_tool_string_0 = J.add(J.attr(ns_0, "python_tool_string"), "</tools>\n\nThe state persists between code executions: so variables that you define in one step are still available thereafter.");
        out.push(J.str(python_tool_string_0));
      }
      out.push(J.str("\n\n"), J.str("<|im_end|>\n"));
    }
  }
  {
    let _items3 = J.iter(messages_0);
    const _loop3 = J.loop(_items3);
    for (let _i3 = 0; _i3 < _items3.length; _i3++) {
      _loop3.index0 = _i3; const _it3 = _items3[_i3]; let message_3 = _it3;
      let content_4 = content_0;
      content_4 = (J.t(J.test("string", J.attr(message_3, "content"))) ? J.attr(message_3, "content") : "");
      if (J.t((J.eq(J.attr(message_3, "role"), "user")))) {
        out.push(J.str(J.add(J.add(J.add(J.add("<|im_start|>", J.attr(message_3, "role")), "\n"), content_4), "<|im_end|>\n")));
      } else if (J.t((J.eq(J.attr(message_3, "role"), "assistant")))) {
        {
          if (J.t((J.eq(reasoning_mode_0, "/think")))) {
            out.push(J.str(J.add(J.add("<|im_start|>assistant\n", J.call(J.attr(content_4, "lstrip"), "\n")), "<|im_end|>\n")));
          } else {
            out.push(J.str(J.add(J.add(J.add("<|im_start|>assistant\n", "<think>\n\n</think>\n"), J.call(J.attr(content_4, "lstrip"), "\n")), "<|im_end|>\n")));
          }
        }
      } else if (J.t((J.eq(J.attr(message_3, "role"), "tool")))) {
        out.push(J.str(J.add(J.add(J.add("<|im_start|>", "user\n"), content_4), "<|im_end|>\n")));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    if (J.t((J.eq(reasoning_mode_0, "/think")))) {
      out.push(J.str("<|im_start|>assistant\n"));
    } else {
      out.push(J.str(J.add("<|im_start|>assistant\n", "<think>\n\n</think>\n")));
    }
  }
  return out.join("");
}
