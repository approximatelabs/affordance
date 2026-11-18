// Qwen1.5-Chat and Qwen2-Instruct: ChatML with a default system prompt, no tools.
"use strict_tokens";
import { GeneratedTurn } from "./common.tsx";
export default function Qwen2({ messages, add_generation_prompt }) {
  if (!messages?.length) throw new Error("Cannot apply chat template to an empty conversation. Provide at least one message.");
  return <>
    {messages[0].role !== "system" && p`${tok.im_start}system\nYou are a helpful assistant.${tok.im_end}\n`}
    {messages.map(m => m.role === "assistant" && m.generated ? <GeneratedTurn ids={m.ids} prompt={p`${tok.im_start}assistant\n`}/>
                                                            : tok.im_start + m.role + "\n" + m.content + tok.im_end + "\n")}
    {add_generation_prompt && p`${tok.im_start}assistant\n`}
  </>;
}

export { Qwen2 };
