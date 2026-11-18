"use strict_tokens";
import { Nemotron3 } from "../nemotron/nemotron3.tsx";
import { textTurns, publisherTurns } from "./input.tsx";
export function Nemotron3App({ children, system = "Use the available tools to interact with the app.", enable_thinking = false, keep_replies = false }) {
  const input = templateInput(children, system, true);
  const format = (messages, generate) => <Nemotron3 messages={messages} tools={input.tools} add_generation_prompt={generate} enable_thinking={enable_thinking}/>;
  const turns = publisherTurns(textTurns(input.turns), keep_replies, (messages, generate) => textContent(format(messages, generate)));
  return [format(turns, true),
          <span label="protocol" syntax="xml" stop={[tok.im_end.id]} in_think={enable_thinking}/>];
}
