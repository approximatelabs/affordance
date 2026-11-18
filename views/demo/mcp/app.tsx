import { useLog } from "../../lib/log.tsx";
// The app chooses and names its remote affordances. MCP discovery is ordinary data.
import { mcpClient } from "../../lib/mcp.tsx";
import { Tool } from "../../lib/tool.tsx";
import { useConversation } from "../../lib/loop.tsx";
import { TurnStrategy } from "../../strategies/turn_strategy.tsx";
import { GLM53App } from "../../template_apps/glm53.tsx";

function McpConversation({log}) {
  useConversation(log);
  return null;
}

export default function Main({url = "/mcp", expose = {}, task}) {
  const client = mcpClient(url);
  const log = useLog();
  const selection = JSON.stringify(expose);
  const [catalogue, setCatalogue] = useState({generation: 0, url: null, selection: null,
    tools: [], ready: false, status: "Loading remote tools.", taskAdded: false}, "mcp.catalogue");
  const ready = catalogue.ready && catalogue.url === url && catalogue.selection === selection;
  useEvent("message", ({text}) => log.append({kind: "input", content: text}));
  useEffect(() => {
    const generation = catalogue.generation + 1;
    setCatalogue(c => ({...c, generation, url, selection, tools: [], ready: false, status: "Loading remote tools."}));
    // The render snapshot gives this task a stable generation across replay. A stale
    // completion may finish its HTTP request, but cannot publish or stop the new run.
    const update = change => {
      let current;
      setCatalogue(c => {
        current = c.generation === generation && c.url === url && c.selection === selection;
        return current ? change(c) : c;
      });
      return current;
    };
    const load = async () => {
      try {
        await client.discover();
        const listed = await client.listTools();
        const names = Object.keys(expose);
        const aliases = names.map(name => expose[name]);
        for (const name of names) {
          if (!listed.tools.some(tool => tool.name === name)) throw new Error("Configured MCP tool not found: " + name);
        }
        if (aliases.some(alias => typeof alias !== "string" || !alias.length) ||
            aliases.some((alias, i) => aliases.indexOf(alias) !== i)) throw new Error("MCP tool aliases must be nonempty and unique");
        const current = update(c => ({...c, tools: listed.tools, ready: true, taskAdded: c.taskAdded || !!task,
          status: listed.rejected.length ? `${listed.rejected.length} invalid tool definitions excluded.` : ""}));
        if (current && task && !catalogue.taskAdded) log.append({kind: "input", content: task});
      } catch (error) {
        if (update(c => c)) harness.stop({reason: "mcp_error", error: error.message});
      }
    };
    load();
  }, [url, selection]);
  return <GLM53App>
    <TurnStrategy strategy="unrolled" log={log.records}>
      {ready ? <McpConversation log={log}/> : null}
      {ready ? catalogue.status : "Loading remote tools."}
      {(ready ? catalogue.tools : []).filter(tool => Object.keys(expose).includes(tool.name)).map(tool =>
        <Tool name={expose[tool.name]} description={tool.description || ""} schema={tool.inputSchema}
          fn={async args => {
            const reply = await client.callTool(tool, args);
            return {result: JSON.stringify(reply), mcp: reply};
          }}>{`[${expose[tool.name]}]\n`}</Tool>)}
    </TurnStrategy>
  </GLM53App>;
}
