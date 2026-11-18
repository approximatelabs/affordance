"use strict_tokens";
import { Qwen3Instruct2507 } from "../qwen/qwen3_2507.tsx";
import { textTurns, publisherTurns } from "./input.tsx";
export function Qwen3App({ children, system = "Use the available tools to interact with the app.", keep_replies = false }) {
  const input = templateInput(children, system, true);
  const format = (messages, generate) => <Qwen3Instruct2507 messages={messages} tools={input.tools} add_generation_prompt={generate}/>;
  const turns = publisherTurns(textTurns(input.turns), keep_replies, (messages, generate) => textContent(format(messages, generate)));
  return [format(turns, true),
          <span label="protocol" syntax="hermes" stop={[tok.im_end.id, tok.endoftext.id]} in_think={false}/>];
}
