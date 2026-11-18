// Demo: a chat view whose token budgets live in a style sheet (chat.tss), not in the components.
"use strict_tokens";
import "./chat.tss";

const Turn = ({ role, children }) => p`${tok.im_start}${role}\n${children}${tok.im_end}\n`;
const ToolOutput = ({ children }) => children;
const Reasoning = ({ text }) => p`${tok.think}\n${text}\n${tok.think_end}\n\n`;

export default function StyledChat({ messages, full_tools = false }) {
  const last = messages.length - 1;
  return <>
    {messages.map((m, i) =>
      <Turn role={m.role}>
        {m.reasoning_content && <Reasoning text={m.reasoning_content} old={i < last}/>}
        {m.role === "tool" ? <ToolOutput full={full_tools}>{m.content}</ToolOutput> : m.content}
      </Turn>)}
    {p`${tok.im_start}assistant\n`}
  </>;
}
