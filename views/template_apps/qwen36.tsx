"use strict_tokens";
import { publisherTurns } from "./input.tsx";
import { Qwen36 } from "../qwen/qwen35.tsx";
// `preserve_thinking`: the template's own switch, off unless set (a past reply's thinking is
// dropped once a new user turn arrives). `keep_replies`: every reply stays as its own tokens.
export function Qwen36App({ children, system = "Use the available tools to interact with the app.", enable_thinking = false, preserve_thinking, keep_replies = false }) {
  const input = templateInput(children, system, true);
  const format = (messages, generate) => <Qwen36 messages={messages} tools={input.tools} add_generation_prompt={generate}
                  enable_thinking={enable_thinking} preserve_thinking={preserve_thinking}/>;
  const turns = publisherTurns(input.turns, keep_replies, (messages, generate) => textContent(format(messages, generate)));
  return [format(turns, true),
          <span label="protocol" syntax="xml" stop={[tok.im_end.id, tok.endoftext.id]} in_think={enable_thinking !== false}/>];
}
