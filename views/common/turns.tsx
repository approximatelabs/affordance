// The protocol turns a chat format writes: runs of
// messages from `start` (by default past a leading system message, which belongs to the header),
// where `joins(i)` says message i continues the turn before it. By default consecutive tool
// results are one turn (they share one opener and closer).
const toolGroups = messages => i => messages[i].role === "tool" && messages[i - 1].role === "tool";

// The header, the history and the current screen (the last turn), each turn a <turn>:
// transparent spans around exactly what the template writes, so a sheet can select or drop whole
// turns and the ids are unchanged.
//
// `items[i]` is what message i writes, `{lead, out}`: `lead` comes before it and ends the turn
// before (an assistant's opener, an observation block), `out` is its own. A template whose
// outputs come from a memoized scan passes them, so kept turns are kept elements and a render
// costs the new messages; `message(m, i)` instead writes each message (no lead) every render.
// `tail` ends the current turn.
function Turns({ messages, header, items, message, joins = toolGroups(messages), start = messages[0]?.role === "system" ? 1 : 0, tail }) {
  items = items ?? messages.map((m, i) => ({lead: null, out: message(m, i)}));
  const head = [header];
  for (let i = 0; i < start; i++) head.push(items[i].lead, items[i].out);
  if (start < items.length) head.push(items[start].lead);
  // A turn closes when the next one starts; its element is built once.
  const r = memoScan(items, null, (open, it, i) => {
    if (i < start) return [null, null];
    if (open !== null && joins(i)) return [{role: open.role, items: [...open.items, it.lead, it.out]}, null];
    return [{role: messages[i].role, items: [it.out]}, open && <turn role={open.role}>{[...open.items, it.lead]}</turn>];
  }, [start]);
  return <>
    <turn role="system">{head}</turn>
    <history>{r.outs}</history>
    {r.state !== null && <turn role={r.state.role} current>{[...r.state.items, tail]}</turn>}
  </>;
}

export { Turns };
