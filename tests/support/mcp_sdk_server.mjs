// Optional interoperability oracle: the actual published SDK, never a shipped dependency.
import http from "node:http";
import {createRequire} from "node:module";
import {resolve} from "node:path";

const require = createRequire(resolve(process.argv[2], "package.json"));
const {McpServer, createMcpHandler} = require("@modelcontextprotocol/server");
const {z} = require("zod");
const handler = createMcpHandler(() => {
  const server = new McpServer({name: "affordance-sdk-oracle", version: "1.0.0"});
  server.registerTool("echo", {description: "Echo structured input", inputSchema: z.object({text: z.string()})},
    async ({text}) => ({content: [{type: "text", text}], structuredContent: {text}}));
  server.registerTool("fail", {inputSchema: z.object({})},
    async () => ({content: [{type: "text", text: "tool failure"}], isError: true}));
  return server;
}, {responseMode: process.argv[3]});

const server = http.createServer(async (request, response) => {
  try {
    const chunks = [];
    for await (const chunk of request) chunks.push(chunk);
    const body = Buffer.concat(chunks);
    const input = new Request(`http://${request.headers.host}${request.url}`, {
      method: request.method, headers: request.headers, body: body.length ? body : undefined
    });
    const output = await handler.fetch(input);
    response.writeHead(output.status, Object.fromEntries(output.headers));
    response.end(Buffer.from(await output.arrayBuffer()));
  } catch (error) {
    console.error(error);
    response.writeHead(500).end();
  }
});
server.listen(0, "127.0.0.1", () => {
  console.log(`http://127.0.0.1:${server.address().port}/mcp`);
});
process.once("SIGTERM", async () => {
  await handler.close();
  server.closeAllConnections();
  server.close(() => process.exit(0));
});
