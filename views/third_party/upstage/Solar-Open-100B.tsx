// Converted from the chat template of upstage/Solar-Open-100B by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  let default_system_prompt_0 = ctx["default_system_prompt"];
  let reasoning_effort_0 = ctx["reasoning_effort"];
  let think_render_option_0 = ctx["think_render_option"];
  let sys_ns_0 = ctx["sys_ns"];
  let last_user_idx_0 = ctx["last_user_idx"];
  let render_system_message_0 = ctx["render_system_message"];
  let render_tool_instruction_0 = ctx["render_tool_instruction"];
  let render_json_response_format_instruction_0 = ctx["render_json_response_format_instruction"];
  let get_tool_name_0 = ctx["get_tool_name"];
  let render_tool_arguments_0 = ctx["render_tool_arguments"];
  let ns_0 = ctx["ns"];
  let messages_0 = ctx["messages"];
  let response_format_0 = ctx["response_format"];
  let tools_0 = ctx["tools"] ?? null;
  let prev_is_tool_0 = ctx["prev_is_tool"];
  let next_is_tool_0 = ctx["next_is_tool"];
  add_generation_prompt_0 = (J.t((add_generation_prompt_0 !== undefined)) ? add_generation_prompt_0 : true);
  default_system_prompt_0 = (J.t((default_system_prompt_0 !== undefined)) ? default_system_prompt_0 : true);
  reasoning_effort_0 = (J.t((reasoning_effort_0 !== undefined)) ? reasoning_effort_0 : "high");
  think_render_option_0 = (J.t((think_render_option_0 !== undefined)) ? think_render_option_0 : "lastthink");
  sys_ns_0 = J.call(J.namespace, J.kw({"is_first_block": true}));
  last_user_idx_0 = J.call(J.namespace, J.kw({"value": J.neg(1)}));
  {
    let _items1 = J.iter(messages_0);
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      if (J.t((J.eq(J.attr(message_1, "role"), "user")))) {
        J.setattr(last_user_idx_0, "value", J.attr(_loop1, "index0"));
      }
    }
  }
  render_system_message_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_3 = k.caller;
    let user_system_messages_2 = p.length > 0 ? p[0] : "user_system_messages" in k ? k["user_system_messages"] : undefined;
    if (J.t(default_system_prompt_0)) {
      if (J.t(!J.t(J.attr(sys_ns_0, "is_first_block")))) {
        out.push(J.str("\n\n"));
      }
      J.setattr(sys_ns_0, "is_first_block", false);
      out.push(J.str(J.add(J.add("## Provider System Prompt\n\nYou are Solar Open 100B, a large language model trained by Upstage AI, a Korean startup. Your knowledge cutoff is 2025-07. The current date is ", J.call(J.strftime_now, "%Y-%m-%d")), ".")));
    }
    if (J.t(user_system_messages_2)) {
      if (J.t(!J.t(J.attr(sys_ns_0, "is_first_block")))) {
        out.push(J.str("\n\n"));
      }
      J.setattr(sys_ns_0, "is_first_block", false);
      out.push(J.str("## System Prompt"));
      {
        let _items2 = J.iter(user_system_messages_2);
        const _loop2 = J.loop(_items2);
        for (let _i2 = 0; _i2 < _items2.length; _i2++) {
          _loop2.index0 = _i2; const _it2 = _items2[_i2]; let system_message_4 = _it2;
          out.push(J.str("\n\n"), J.str(system_message_4));
        }
      }
    }
    return out.join("");
  };
  render_tool_instruction_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_6 = k.caller;
    let tools_5 = p.length > 0 ? p[0] : "tools" in k ? k["tools"] : undefined;
    if (J.t(!J.t(J.attr(sys_ns_0, "is_first_block")))) {
      out.push(J.str("\n\n"));
    }
    J.setattr(sys_ns_0, "is_first_block", false);
    out.push(J.str("## Tools\n\n### Tool Call Instruction"), J.str("\nYou may invoke one or more tools to assist with the user's query. Available tools are provided in JSON Schema format: <|tools:begin|><|tool:begin|><tools-json-object><|tool:end|>...<|tools:end|>\n"), J.str("\n### Available Tools\n"), J.str("<|tools:begin|>"));
    {
      let _items3 = J.iter(tools_5);
      const _loop3 = J.loop(_items3);
      for (let _i3 = 0; _i3 < _items3.length; _i3++) {
        _loop3.index0 = _i3; const _it3 = _items3[_i3]; let tool_7 = _it3;
        out.push(J.str("<|tool:begin|>"), J.str(J.filter("tojson", J.attr(tool_7, "function"))), J.str("<|tool:end|>"));
      }
    }
    out.push(J.str("<|tools:end|>\n"), J.str("\n### Tool Call Format\n"), J.str("For each tool call, return a JSON object with the following structure, enclosed within <|tool_call:begin|> and <|tool_call:end|> tags: \n<|tool_call:begin|><tool-call-id><|tool_call:name|><tool-name><|tool_call:args|><args-json-object><|tool_call:end|>\n"), J.str("- The <tool-call-id> must be a randomly generated string consisting of 10 lowercase letters (a-z) and/or digits (0-9) (e.g., a1b2c3d4e5)\n"), J.str("\n### Tool Response Format\n"), J.str("Each tool is responded by `tool` with the following structure:\n<|tool_response:id|><tool-call-id><|tool_response:name|><tool-name><|tool_response:result|><results><|tool_response:end|>\n"), J.str("- Ensure the <tool-call-id> matches the corresponding tool call"));
    return out.join("");
  };
  render_json_response_format_instruction_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_9 = k.caller;
    let response_format_8 = p.length > 0 ? p[0] : "response_format" in k ? k["response_format"] : undefined;
    if (J.t(!J.t(J.attr(sys_ns_0, "is_first_block")))) {
      out.push(J.str("\n\n"));
    }
    J.setattr(sys_ns_0, "is_first_block", false);
    out.push(J.str("## Output Format Constraint"), J.str("\n\nYour final response should follow the JSON schema: \n[Start of schema]"), J.str(response_format_8), J.str("\n[End of schema]\nPlease ensure your answers adhere to this format and do not contain any unnecessary text."));
    return out.join("");
  };
  get_tool_name_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_12 = k.caller;
    let messages_10 = p.length > 0 ? p[0] : "messages" in k ? k["messages"] : undefined;
    let tool_call_id_11 = p.length > 1 ? p[1] : "tool_call_id" in k ? k["tool_call_id"] : undefined;
    {
      let _items4 = J.iter(messages_10);
      const _loop4 = J.loop(_items4);
      for (let _i4 = 0; _i4 < _items4.length; _i4++) {
        _loop4.index0 = _i4; const _it4 = _items4[_i4]; let msg_13 = _it4;
        if (J.t(((v) => J.t(v) ? J.attr(msg_13, "tool_calls") : v)((J.eq(J.attr(msg_13, "role"), "assistant"))))) {
          {
            let _items5 = J.iter(J.attr(msg_13, "tool_calls"));
            const _loop5 = J.loop(_items5);
            for (let _i5 = 0; _i5 < _items5.length; _i5++) {
              _loop5.index0 = _i5; const _it5 = _items5[_i5]; let tool_call_14 = _it5;
              if (J.t((J.eq(J.attr(tool_call_14, "id"), tool_call_id_11)))) {
                out.push(J.str(J.attr(J.attr(tool_call_14, "function"), "name")));
              }
            }
          }
        }
      }
    }
    return out.join("");
  };
  render_tool_arguments_0 = (...args) => {
    const [p, k] = J.args(args);
    const out = [];
    const caller_16 = k.caller;
    let tool_arguments_15 = p.length > 0 ? p[0] : "tool_arguments" in k ? k["tool_arguments"] : undefined;
    if (J.t(J.test("mapping", tool_arguments_15))) {
      out.push(J.str(J.filter("tojson", tool_arguments_15)));
    } else {
      out.push(J.str(tool_arguments_15));
    }
    return out.join("");
  };
  ns_0 = J.call(J.namespace, J.kw({"system_messages": []}));
  {
    let _items6 = J.iter(messages_0);
    const _loop6 = J.loop(_items6);
    for (let _i6 = 0; _i6 < _items6.length; _i6++) {
      _loop6.index0 = _i6; const _it6 = _items6[_i6]; let message_17 = _it6;
      if (J.t((J.eq(J.attr(message_17, "role"), "system")))) {
        J.setattr(ns_0, "system_messages", J.add(J.attr(ns_0, "system_messages"), [J.attr(message_17, "content")]));
      }
    }
  }
  if (J.t(((v) => J.t(v) ? v : response_format_0)(((v) => J.t(v) ? v : tools_0)(((v) => J.t(v) ? v : default_system_prompt_0)(J.attr(ns_0, "system_messages")))))) {
    out.push(J.str("<|begin|>system<|content|>"), J.str(J.call(render_system_message_0, J.attr(ns_0, "system_messages"))));
    if (J.t(tools_0)) {
      out.push(J.str(J.call(render_tool_instruction_0, tools_0)));
    }
    if (J.t(response_format_0)) {
      out.push(J.str(J.call(render_json_response_format_instruction_0, response_format_0)));
    }
    out.push(J.str("<|end|>"));
  }
  {
    let _items7 = J.iter(messages_0);
    const _loop7 = J.loop(_items7);
    for (let _i7 = 0; _i7 < _items7.length; _i7++) {
      _loop7.index0 = _i7; const _it7 = _items7[_i7]; let message_18 = _it7;
      let prev_is_tool_19 = prev_is_tool_0;
      let next_is_tool_20 = next_is_tool_0;
      if (J.t((J.eq(J.attr(message_18, "role"), "user")))) {
        out.push(J.str(J.add(J.add("<|begin|>user<|content|>", J.attr(message_18, "content")), "<|end|>")));
      } else if (J.t((J.eq(J.attr(message_18, "role"), "tool")))) {
        prev_is_tool_19 = ((v) => J.t(v) ? (J.eq(J.attr(J.item(messages_0, J.sub(J.attr(_loop7, "index0"), 1)), "role"), "tool")) : v)((J.gt(J.attr(_loop7, "index0"), 0)));
        next_is_tool_20 = ((v) => J.t(v) ? (J.eq(J.attr(J.item(messages_0, J.add(J.attr(_loop7, "index0"), 1)), "role"), "tool")) : v)((J.lt(J.attr(_loop7, "index0"), J.sub(J.filter("length", messages_0), 1))));
        if (J.t(!J.t(prev_is_tool_19))) {
          out.push(J.str("<|begin|>tool<|tool_response|>"));
        }
        out.push(J.str(J.add(J.add("<|tool_response:begin|>", J.attr(message_18, "tool_call_id")), "<|tool_response:name|>")), J.str(J.call(get_tool_name_0, messages_0, J.attr(message_18, "tool_call_id"))), J.str("<|tool_response:result|>"), J.str(J.attr(message_18, "content")), J.str("<|tool_response:end|>"));
        if (J.t(!J.t(next_is_tool_20))) {
          out.push(J.str("<|end|>"));
        }
      } else if (J.t((J.eq(J.attr(message_18, "role"), "assistant")))) {
        if (J.t((J.eq(think_render_option_0, "all")))) {
          if (J.t(J.attr(message_18, "reasoning"))) {
            out.push(J.str(J.add(J.add("<|begin|>assistant<|think|>", J.attr(message_18, "reasoning")), "<|end|>")));
          }
        } else if (J.t((J.eq(think_render_option_0, "lastthink")))) {
          if (J.t(((v) => J.t(v) ? (J.gt(J.attr(_loop7, "index0"), J.attr(last_user_idx_0, "value"))) : v)(J.attr(message_18, "reasoning")))) {
            out.push(J.str(J.add(J.add("<|begin|>assistant<|think|>", J.attr(message_18, "reasoning")), "<|end|>")));
          }
        }
        if (J.t(J.attr(message_18, "tool_calls"))) {
          out.push(J.str("<|begin|>assistant<|tool_calls|>"));
          {
            let _items8 = J.iter(J.attr(message_18, "tool_calls"));
            const _loop8 = J.loop(_items8);
            for (let _i8 = 0; _i8 < _items8.length; _i8++) {
              _loop8.index0 = _i8; const _it8 = _items8[_i8]; let tool_call_21 = _it8;
              out.push(J.str(J.add(J.add(J.add(J.add("<|tool_call:begin|>", J.attr(tool_call_21, "id")), "<|tool_call:name|>"), J.attr(J.attr(tool_call_21, "function"), "name")), "<|tool_call:args|>")), J.str(J.call(render_tool_arguments_0, J.attr(J.attr(tool_call_21, "function"), "arguments"))), J.str("<|tool_call:end|>"));
            }
          }
          out.push(J.str("<|calls|>"));
        } else {
          out.push(J.str(J.add(J.add("<|begin|>assistant<|content|>", J.attr(message_18, "content")), "<|end|>")));
        }
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    if (J.t((J.has(reasoning_effort_0, ["low", "minimal"])))) {
      out.push(J.str("<|begin|>assistant<|think|><|end|>"));
    }
    out.push(J.str("<|begin|>assistant"));
  }
  return out.join("");
}
