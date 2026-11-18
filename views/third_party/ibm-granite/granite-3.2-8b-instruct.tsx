// Converted from the chat template of ibm-granite/granite-3.2-8b-instruct by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let system_message_0 = ctx["system_message"];
  let loop_messages_0 = ctx["loop_messages"];
  let messages_0 = ctx["messages"];
  let documents_0 = ctx["documents"] ?? null;
  let tools_0 = ctx["tools"] ?? null;
  let thinking_0 = ctx["thinking"];
  let controls_0 = ctx["controls"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
    system_message_0 = J.item(J.item(messages_0, 0), "content");
    loop_messages_0 = J.slice(messages_0, 1, undefined, undefined);
  } else {
    system_message_0 = J.add(J.add("Knowledge Cutoff Date: April 2024.\nToday's Date: ", J.call(J.strftime_now, "%B %d, %Y")), ".\nYou are Granite, developed by IBM.");
    if (J.t(((v) => J.t(v) ? documents_0 : v)(tools_0))) {
      system_message_0 = J.add(system_message_0, " You are a helpful AI assistant with access to the following tools. When a tool is required to answer the user's query, respond with <|tool_call|> followed by a JSON list of tools used. If a tool does not exist in the provided list of tools, notify the user that you do not have the ability to fulfill the request.\n\nWrite the response to the user's input by strictly aligning with the facts in the provided documents. If the information needed to answer the question is not available in the documents, inform the user that the question cannot be answered based on the available data.");
    } else if (J.t(tools_0)) {
      system_message_0 = J.add(system_message_0, " You are a helpful AI assistant with access to the following tools. When a tool is required to answer the user's query, respond with <|tool_call|> followed by a JSON list of tools used. If a tool does not exist in the provided list of tools, notify the user that you do not have the ability to fulfill the request.");
    } else if (J.t(documents_0)) {
      system_message_0 = J.add(system_message_0, " Write the response to the user's input by strictly aligning with the facts in the provided documents. If the information needed to answer the question is not available in the documents, inform the user that the question cannot be answered based on the available data.");
    } else if (J.t(thinking_0)) {
      system_message_0 = J.add(system_message_0, " You are a helpful AI assistant.\nRespond to every user query in a comprehensive and detailed way. You can write down your thoughts and reasoning process before responding. In the thought process, engage in a comprehensive cycle of analysis, summarization, exploration, reassessment, reflection, backtracing, and iteration to develop well-considered thinking process. In the response section, based on various attempts, explorations, and reflections from the thoughts section, systematically present the final solution that you deem correct. The response should summarize the thought process. Write your thoughts after 'Here is my thought process:' and write your response after 'Here is my response:' for each user query.");
    } else {
      system_message_0 = J.add(system_message_0, " You are a helpful AI assistant.");
    }
    if (J.t(((v) => J.t(v) ? documents_0 : v)((J.has("citations", controls_0))))) {
      system_message_0 = J.add(system_message_0, "\n\nIn your response, use the symbols <co> and </co> to indicate when a fact comes from a document in the search result, e.g <co>0</co> for a fact from document 0. Afterwards, list all the citations with their corresponding documents in an ordered list.");
    }
    if (J.t(((v) => J.t(v) ? documents_0 : v)((J.has("hallucinations", controls_0))))) {
      system_message_0 = J.add(system_message_0, "\n\nFinally, after the response is written, include a numbered list of sentences from the response that are potentially hallucinated and not based in the documents.");
    }
    loop_messages_0 = messages_0;
  }
  out.push(J.str(J.add(J.add("<|start_of_role|>system<|end_of_role|>", system_message_0), "<|end_of_text|>\n")));
  if (J.t(tools_0)) {
    out.push(J.str("<|start_of_role|>tools<|end_of_role|>"), J.str(J.filter("tojson", tools_0, J.kw({"indent": 4}))), J.str("<|end_of_text|>\n"));
  }
  if (J.t(documents_0)) {
    out.push(J.str("<|start_of_role|>documents<|end_of_role|>"));
    {
      let _items1 = J.iter(documents_0);
      const _loop1 = J.loop(_items1);
      for (let _i1 = 0; _i1 < _items1.length; _i1++) {
        _loop1.index0 = _i1; const _it1 = _items1[_i1]; let document_1 = _it1;
        out.push(J.str(J.add(J.add("Document ", J.filter("string", J.attr(_loop1, "index0"))), "\n")), J.str(J.item(document_1, "text")));
        if (J.t(!J.t(J.attr(_loop1, "last")))) {
          out.push(J.str("\n\n"));
        }
      }
    }
    out.push(J.str("<|end_of_text|>\n"));
  }
  {
    let _items2 = J.iter(loop_messages_0);
    const _loop2 = J.loop(_items2);
    for (let _i2 = 0; _i2 < _items2.length; _i2++) {
      _loop2.index0 = _i2; const _it2 = _items2[_i2]; let message_2 = _it2;
      out.push(J.str(J.add(J.add(J.add(J.add("<|start_of_role|>", J.item(message_2, "role")), "<|end_of_role|>"), J.item(message_2, "content")), "<|end_of_text|>\n")));
      if (J.t(((v) => J.t(v) ? add_generation_prompt_0 : v)(J.attr(_loop2, "last")))) {
        out.push(J.str("<|start_of_role|>assistant"));
        if (J.t(controls_0)) {
          out.push(J.str(J.add(" ", J.filter("tojson", controls_0))));
        }
        out.push(J.str("<|end_of_role|>"));
      }
    }
  }
  return out.join("");
}
