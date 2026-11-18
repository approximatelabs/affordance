"use strict_tokens";
import { GLM53 } from "../glm/glm.tsx";
import { prepareTemplateInput, publisherTurns } from "./input.tsx";
// The protocol span (zero tokens) tells a runtime how to read this model's replies: the reply
// syntax, the stop tokens, and whether the prompt ends inside an open <think> block. GLM-5.3
// always thinks (its template has no switch); `reasoning_effort` is low, high or max (default).
// `clear_thinking`: the template's own switch, off unless set (past thinking is kept).
// `keep_replies`: every reply stays as its own tokens.
export function GLM53App({ children, system = "Use the available tools to interact with the app.", reasoning_effort, clear_thinking, keep_replies = false }) {
  const input = prepareTemplateInput(children, system, true);
  const format = (messages, generate) => <GLM53 messages={messages} tools={input.tools} add_generation_prompt={generate} reasoning_effort={reasoning_effort} clear_thinking={clear_thinking}/>;
  const turns = publisherTurns(input.turns, keep_replies, (messages, generate) => textContent(format(messages, generate)));
  return [format(turns, true),
          <span label="protocol" syntax="glm" stop={[tok.user.id, tok.observation.id, tok.endoftext.id]} in_think={true}/>];
}
