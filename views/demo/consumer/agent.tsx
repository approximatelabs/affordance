// Example agent app for harness consumers (docs/HARNESS_GUIDE.md, tests/harness_consumer.rs).
// - `tool_name` gives each action its exact tool name (otherwise `label__target`).
// - Handlers that need the outside world await harness APIs (`harness.machine.call`,
//   `harness.send`); the host never dispatches on tool names. A handler returns a state patch; this app
//   puts the text for the `action` log record in `result` (a harness convention, see the guide).
// - The three actions share one line; "[shell][send_message]" are adjacent `<aff>` runs whose
//   joined text tokenizes differently ("][" merges), so a screen logged as text cannot round-trip.
export function ConsumerAgent({ handle, unread = 0, notes = "" }) {
  return <>
    <box label="status">{`${handle} | unread: ${unread}\n`}</box>
    {notes ? <box label="notes">{"Notes: " + notes + "\n"}</box> : ""}
    <aff label="shell" tool_name="shell" description="Run a shell command in your VM."
         args_schema={{type: "object", properties: {command: {type: "string"}}, required: ["command"]}}
         onRun={async ({ command }) => {
           const r = await harness.machine.call({cmd: command});
           return {result: r.output + "\n[exit " + String(r.exit) + "]"};
         }}>{"[shell]"}</aff>
    <aff label="message" tool_name="send_message" description="Send a direct message to another agent by id."
         args_schema={{type: "object", properties: {to: {type: "string"}, text: {type: "string"}}, required: ["to", "text"]}}
         onSend={async ({ to, text }) => {
           const r = await harness.send(to, {text: text});
           return {result: "delivered to " + to + " (#" + String(r.seq) + ")"};
         }}>{"[send_message]"}</aff>
    <aff label="notes" tool_name="set_notes" description="Replace your notes."
         args_schema={{type: "object", properties: {text: {type: "string"}}, required: ["text"]}}
         onSet={({ text }) => ({notes: text, result: "notes saved"})}>{" [set_notes]\n"}</aff>
  </>;
}
