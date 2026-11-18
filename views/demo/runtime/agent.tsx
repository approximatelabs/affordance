import { useLog } from "../../lib/log.tsx";
// A minimal agent app for the runtime (docs/RUNTIME.md): one file, no Rust.
// - State lives in hooks; the runtime persists them.
// - The log (useLog, views/lib/log.tsx) is what the model has seen and said, with exact ids: app state.
// - Tools reach the outside world through the harness (harness.machine, harness.send, ...);
//   their return value is the tool result.
// - An incoming message is a callback that appends an `input` record to the log.
// - When the model acts is the app's: `useLoop(log)` asks the model, runs its calls, repeats.
import { TurnStrategy } from "../../strategies/turn_strategy.tsx";
import { GLM53App } from "../../template_apps/glm53.tsx";
import { useLoop } from "../../lib/loop.tsx";

function lastModel(transcript) {
  for (const r of [...transcript].reverse()) { if (r.kind === "model") return r; }
  return null;
}

function Screen({ notes, unread, nudge, setNotes, setUnread, log }) {
  return <>
    <box label="status">{`${harness.agent} | turn ${harness.turn} | unread: ${unread}\nNotes: ${notes}\n`}</box>
    {nudge ? <box label="nudge">{nudge + "\n"}</box> : ""}
    <aff label="shell" tool_name="shell" description="Run a shell command in your VM."
         args_schema={{type: "object", properties: {command: {type: "string"}}, required: ["command"]}}
         onRun={async ({ command }) => {
           const r = await harness.machine.call({cmd: command, timeout_s: 30});
           return r.output + "\n[exit " + String(r.exit) + "]";
         }}>{"[shell]"}</aff>
    <aff label="message" tool_name="send_message" description="Send a message to another agent."
         args_schema={{type: "object", properties: {to: {type: "string"}, text: {type: "string"}}, required: ["to", "text"]}}
         onSend={async ({ to, text }) => {
           await harness.send(to, {text: text});
           return "sent to " + to;
         }}>{"[send_message]"}</aff>
    <aff label="notes" tool_name="set_notes" description="Replace your notes."
         args_schema={{type: "object", properties: {text: {type: "string"}}, required: ["text"]}}
         onSet={({ text }) => { setNotes(text); return "notes saved"; }}>{" [set_notes]"}</aff>
    <aff label="wait" tool_name="wait" description="Pause until a message arrives or the seconds pass."
         args_schema={{type: "object", properties: {seconds: {type: "integer"}}, required: ["seconds"]}}
         onWait={async ({ seconds }) => {
           const r = await harness.wait({ms: seconds * 1000, events: ["message"]});
           return r.woke === "timeout" ? "waited " + String(seconds) + "s" : "woken by a message";
         }}>{" [wait]\n"}</aff>
    <aff event="message" onCallback={async ({ from, text }) => {
      log.append({kind: "input", content: "[message from " + from + "] " + text});
      setUnread(unread + 1);
    }}/>
  </>;
}

export default function Main({ task }) {
  const log = useLog();
  // An autonomous agent: ask, act, repeat. It keeps going after a reply with no tool call (its
  // turn limit ends it); without `idle`, such a reply would end it as its answer.
  useLoop(log, {idle: () => true});
  const [notes, setNotes] = useState("(none)", "notes");
  const [unread, setUnread] = useState(0, "unread");
  const last = lastModel(log.records);
  const nudge = last && !last.calls?.length ? "Your last reply had no tool call. Act with a tool, or call wait." : "";
  const app = <Screen label="agent" notes={notes} unread={unread} nudge={nudge} setNotes={setNotes} setUnread={setUnread} log={log}/>;
  // `keep_replies`: each reply stays as the tokens the model wrote, so every prompt extends the
  // last one. Without it a past reply is written the way GLM's publisher writes one.
  return <GLM53App keep_replies={true}>
    <TurnStrategy strategy="unrolled" log={[{kind: "input", content: task}, ...log.records]}>{app}</TurnStrategy>
  </GLM53App>;
}

export { Main };
