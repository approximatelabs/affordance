"use strict_tokens";
import { publisherTurns } from "./input.tsx";
import { MiMoV25 } from "../mimo/mimo.tsx";
export function MiMo25App({ children, system = "Use the available tools to interact with the app.", enable_thinking = false, keep_replies = false }) {
  const input = templateInput(children, system, true);
  const format = (messages, generate) => <MiMoV25 messages={messages} tools={input.tools} add_generation_prompt={generate} enable_thinking={enable_thinking}/>;
  const turns = publisherTurns(input.turns, keep_replies, (messages, generate) => textContent(format(messages, generate)));
  return [format(turns, true),
          <span label="protocol" syntax="xml" stop={[tok.im_end.id, tok.endoftext.id]} in_think={false}/>];
}
