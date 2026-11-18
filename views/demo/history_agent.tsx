import { Qwen3Instruct2507 } from "../qwen/qwen3_2507.tsx";
// A bounded-history harness app. The model format is Qwen3-Instruct-2507.
// System tools and the original task stay pinned; assistant/tool exchanges are
// dropped as whole children. The mutable screen comes after history.
"use strict_tokens";
import { Screen, TOOLS } from "./list_agent.tsx";

// Keep each assistant call and all of its consecutive tool results together.
// A user/system message starts its own group. No tool response is left orphaned.
function exchanges(messages) {
  let out = [], group = [];
  for (const m of messages) {
    if (m.role !== "tool" && group.length > 0) {
      out.push(group);
      group = [];
    }
    group.push(m);
  }
  if (group.length > 0) out.push(group);
  return out;
}

const History = ({ messages }) => exchanges(messages).map(group =>
  <Qwen3Instruct2507 messages={group} tools={null} add_generation_prompt={false}/>);

export default function HistoryAgent({ messages, numbers, offset = 0, pending = [], found = [], note = "", history_budget = 256 }) {
  return <>
    <Qwen3Instruct2507 messages={messages.slice(0, 1)} tools={TOOLS} add_generation_prompt={false}/>
    <History label="history" messages={messages.slice(1)} style={{ maxTokens: history_budget, overflow: "drop-children" }}/>
    <Screen numbers={numbers} offset={offset} pending={pending} found={found} note={note} format="2507" thinking={false}/>
  </>;
}
