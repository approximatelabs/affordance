"use strict_tokens";
import { Llama31 } from "../llama/llama.tsx";
import { textTurns, publisherTurns } from "./input.tsx";
// A log's text-only `model` record carries its calls as `{name, arguments}`; the Llama 3.1
// template reads the chat-API shape (`tool_calls[i].function`) and raises without it.
const flatCalls = m => !m.generated && Array.isArray(m.tool_calls) && m.tool_calls.some(tc => tc.function === undefined);
const chatCalls = m => flatCalls(m) ? {...m, tool_calls: m.tool_calls.map(tc => tc.function === undefined ? {type: "function", function: tc} : tc)} : m;

export function Llama31App({ children, system = "Use the available tools to interact with the app.", date_string = "26 Jul 2024", keep_replies = false }) {
  const input = templateInput(children, system, true);
  const text = textTurns(input.turns);
  const format = (messages, generate) => <Llama31 messages={messages} tools={input.tools} add_generation_prompt={generate}
                  bos_token={tok.begin_of_text} tools_in_user_message={false} date_string={date_string}/>;
  const turns = publisherTurns(text.some(flatCalls) ? text.map(chatCalls) : text, keep_replies,
    (messages, generate) => textContent(format(messages, generate)));
  return [format(turns, true),
          <span label="protocol" syntax="llama31" stop={[tok.eot_id.id, tok.eom_id.id, tok.end_of_text.id]} in_think={false}/>];
}
