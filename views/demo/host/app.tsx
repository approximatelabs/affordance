// A coding chat for a frontend (`affordance host`): it answers the user, reads, writes and edits
// files and runs shell commands after asking permission, and stops what it is doing on Escape
// (the partial answer stays).
import "./app.tss";
import { ModelContext } from "@affordance/lib/model.tsx";
import { useLog } from "@affordance/lib/log.tsx";
import { useFrontend, ask } from "@affordance/lib/frontend.tsx";
import { Tool } from "@affordance/lib/tool.tsx";
import { machineTools } from "@affordance/tools/machine.tsx";
import { fileTools } from "@affordance/tools/files.tsx";

export default function Main() {
  const log = useLog();
  const turn = useFrontend(log);
  const home = harness.machine.get("home");
  // The user approves each call: the frontend shows what is asked and answers "yes" or not.
  const permit = async (tool, command) => (await ask({ kind: "permission", tool, command }, "no")) === "yes";
  const { shell } = machineTools(home, { signal: turn.signal });
  const { read, write, edit } = fileTools(home, { signal: turn.signal, permit });
  /** Run a shell command in the project. The user approves each one. */
  async function run({ command }: { command: string }) {
    if (!(await permit("run", command))) return "error: the user declined to run this command";
    return await shell({ command });
  }
  return <ModelContext strategy="unrolled" system="You are a careful coding assistant working in the user's project. Use read, write and edit for files, and run for shell commands." log={log.records}>
    <Tool fn={run}>{"[run] "}</Tool>
    <Tool fn={read}>{"[read] "}</Tool>
    <Tool fn={write}>{"[write] "}</Tool>
    <Tool fn={edit}>{"[edit] "}</Tool>
  </ModelContext>;
}
