"use strict_tokens";
import { Qwen3 } from "../qwen/qwen3.tsx";
import { textTurns, publisherTurns } from "./input.tsx";
export function Qwen3BaseApp({ children, system = "Use the available tools to interact with the app.", enable_thinking = false, keep_replies = false }) {
  const input = templateInput(children, system, true);
  const format = (messages, generate) => <Qwen3 messages={messages} tools={input.tools} add_generation_prompt={generate} enable_thinking={enable_thinking}/>;
  const turns = publisherTurns(textTurns(input.turns), keep_replies, (messages, generate) => textContent(format(messages, generate)));
  return [format(turns, true),
          <span label="protocol" syntax="hermes" stop={[tok.im_end.id, tok.endoftext.id]} in_think={false}/>];
}
