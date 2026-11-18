// MCP tools over a generic HTTP binding. No protocol behavior lives in the engine.
// Revision 2026-07-28: per-request metadata, without connection-scoped sessions.
import { httpEvents } from "./http.tsx";

function __mcpObject(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function __mcpHas(value, key) {
  return Object.keys(value).includes(key);
}

function __mcpProtocol(message) {
  return {name: "McpProtocolError", message: "MCP: " + message};
}

function __mcpJson(text) {
  try { return JSON.parse(text); }
  catch (e) { throw __mcpProtocol("response is not JSON"); }
}

// MCP's header sentinel encodes UTF-8, including non-ASCII names and values that could
// otherwise be confused with an already encoded value. Plain TSX, not a host builtin.
function __mcpHeader(value) {
  const text = String(value);
  if (/^[\x20-\x7e\t]*$/.test(text) && text.trim() === text &&
      !(text.startsWith("=?base64?") && text.endsWith("?="))) return text;
  const bytes = [];
  for (const character of text) {
    const c = character.codePointAt(0);
    if (c < 128) bytes.push(c);
    else if (c < 2048) bytes.push(192 | (c >> 6), 128 | (c & 63));
    else if (c < 65536) bytes.push(224 | (c >> 12), 128 | ((c >> 6) & 63), 128 | (c & 63));
    else bytes.push(240 | (c >> 18), 128 | ((c >> 12) & 63), 128 | ((c >> 6) & 63), 128 | (c & 63));
  }
  const alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
  let out = "";
  for (let i = 0; i < bytes.length; i += 3) {
    const a = bytes[i], b = bytes[i + 1] || 0, c = bytes[i + 2] || 0;
    out += alphabet[a >> 2] + alphabet[((a & 3) << 4) | (b >> 4)] +
      (i + 1 < bytes.length ? alphabet[((b & 15) << 2) | (c >> 6)] : "=") +
      (i + 2 < bytes.length ? alphabet[c & 63] : "=");
  }
  return "=?base64?" + out + "?=";
}

// Collect parameter-header paths and reject invalid annotations, including annotations in
// arrays, references or composition branches. Never fetch a schema's $ref over the network.
function __mcpHeaderFields(node, path, reachable, fields, names) {
  if (Array.isArray(node)) {
    for (const child of node) __mcpHeaderFields(child, path, false, fields, names);
    return;
  }
  if (!__mcpObject(node)) return;
  if (__mcpHas(node, "x-mcp-header")) {
    const name = node["x-mcp-header"];
    if (!reachable || !path.length || typeof name !== "string" ||
        !/^[!#$%&'*+.^_`|~0-9A-Za-z-]+$/.test(name) ||
        !["string", "integer", "boolean"].includes(node.type) || names.includes(name.toLowerCase())) {
      throw __mcpProtocol("invalid x-mcp-header annotation");
    }
    names.push(name.toLowerCase());
    fields.push({name, path, type: node.type});
  }
  for (const key of Object.keys(node)) {
    if (key === "properties" && __mcpObject(node[key])) {
      for (const property of Object.keys(node[key])) {
        __mcpHeaderFields(node[key][property], [...path, property], reachable, fields, names);
      }
    } else if (["$defs", "definitions", "patternProperties", "dependentSchemas", "dependencies"].includes(key) && __mcpObject(node[key])) {
      for (const name of Object.keys(node[key])) __mcpHeaderFields(node[key][name], path, false, fields, names);
    } else if (["items", "prefixItems", "additionalItems", "contains", "unevaluatedItems",
                "additionalProperties", "unevaluatedProperties", "propertyNames", "contentSchema",
                "allOf", "anyOf", "oneOf", "not", "if", "then", "else"].includes(key)) {
      __mcpHeaderFields(node[key], path, false, fields, names);
    }
    // Values of default/examples/enum/const are instance data, not subschemas.
  }
}

function __mcpToolFields(tool) {
  if (!__mcpObject(tool) || typeof tool.name !== "string" || !tool.name.length ||
      (tool.description != null && typeof tool.description !== "string") ||
      !__mcpObject(tool.inputSchema) || (tool.inputSchema.type != null && tool.inputSchema.type !== "object")) {
    throw __mcpProtocol("invalid tool definition");
  }
  const fields = [];
  __mcpHeaderFields(tool.inputSchema, [], true, fields, []);
  return fields;
}

async function __mcpRequest(fetchImpl, endpoint, method, params, extraHeaders) {
  // A journaled ID has no hidden mutable counter to restore or share between tasks.
  const id = "mcp-" + String(harness.random());
  const headers = {
    "Content-Type": "application/json",
    "Accept": "application/json, text/event-stream",
    "MCP-Protocol-Version": "2026-07-28",
    "Mcp-Method": method,
    ...extraHeaders
  };
  const body = {jsonrpc: "2.0", id, method, params: {...params, _meta: {
    "io.modelcontextprotocol/protocolVersion": "2026-07-28",
    "io.modelcontextprotocol/clientInfo": {name: "affordance", version: "0.1.0"},
    "io.modelcontextprotocol/clientCapabilities": {}
  }}};
  const response = await fetchImpl(endpoint, {method: "POST", headers, body: JSON.stringify(body)});
  const ok = response.ok;
  const type = (response.headers.get("content-type") || "").split(";")[0].trim().toLowerCase();
  const text = await response.text();
  let messages = [];
  if (!ok) {
    let failure = null;
    try { failure = JSON.parse(text); } catch (error) {}
    if (type !== "application/json" || !__mcpObject(failure) || failure.jsonrpc !== "2.0" || !__mcpHas(failure, "error")) {
      throw {name: "McpHttpError", message: "MCP HTTP " + response.status, status: response.status};
    }
    messages = [failure];
  } else if (type === "application/json") messages = [__mcpJson(text)];
  else if (type === "text/event-stream") messages = httpEvents(text).map(event => __mcpJson(event.data));
  else if (ok) throw __mcpProtocol("expected application/json or text/event-stream");
  let result = null;
  let answered = false;
  for (const message of messages) {
    if (!__mcpObject(message) || message.jsonrpc !== "2.0") throw __mcpProtocol("invalid JSON-RPC envelope");
    if (typeof message.method === "string" && !__mcpHas(message, "id") &&
        !__mcpHas(message, "result") && !__mcpHas(message, "error")) continue;
    if (answered || (message.id !== id && !(message.id === null && !ok && message.error))) {
      throw __mcpProtocol("unexpected or duplicate response ID");
    }
    if (__mcpHas(message, "error")) {
      if (__mcpHas(message, "result") || __mcpHas(message, "method") || !__mcpObject(message.error) ||
          !Number.isInteger(message.error.code) || typeof message.error.message !== "string") {
        throw __mcpProtocol("invalid JSON-RPC error");
      }
      throw {name: "McpRpcError", message: message.error.message,
             code: message.error.code, data: message.error.data, status: response.status};
    }
    if (!__mcpObject(message.result) || __mcpHas(message, "method")) throw __mcpProtocol("invalid JSON-RPC result");
    result = message.result;
    answered = true;
  }
  if (!ok) throw {name: "McpHttpError", message: "MCP HTTP " + response.status, status: response.status};
  if (!answered) throw __mcpProtocol("response ended without a result");
  if (result.resultType != null && !["complete", "input_required"].includes(result.resultType)) {
    throw __mcpProtocol("unknown resultType");
  }
  return result;
}

export function mcpClient(endpoint, options = {}) {
  if (typeof endpoint !== "string") throw new TypeError("mcpClient requires an endpoint URL");
  const fetchImpl = options.fetch || fetch;
  if (typeof fetchImpl !== "function") throw new TypeError("mcpClient fetch must be a function");
  return {
    discover: async () => {
      const result = await __mcpRequest(fetchImpl, endpoint, "server/discover", {}, {});
      if (!Array.isArray(result.supportedVersions) || !result.supportedVersions.includes("2026-07-28") ||
          !__mcpObject(result.capabilities)) throw __mcpProtocol("server does not support MCP 2026-07-28 discovery");
      return result;
    },
    listTools: async () => {
      const tools = [], rejected = [], cursors = [], names = [];
      let cursor = null;
      do {
        if (cursors.length >= 100) throw __mcpProtocol("tool pagination exceeded 100 pages");
        const result = await __mcpRequest(fetchImpl, endpoint, "tools/list", cursor === null ? {} : {cursor}, {});
        if (!Array.isArray(result.tools)) throw __mcpProtocol("tools/list has no tools array");
        for (const tool of result.tools) {
          try { __mcpToolFields(tool); }
          catch (error) { rejected.push({name: tool && tool.name, reason: error.message}); continue; }
          if (names.includes(tool.name)) throw __mcpProtocol("duplicate tool name");
          names.push(tool.name);
          tools.push(tool);
        }
        cursor = result.nextCursor == null ? null : result.nextCursor;
        if (cursor !== null) {
          if (typeof cursor !== "string" || !cursor.length || cursors.includes(cursor)) throw __mcpProtocol("invalid or repeated tool cursor");
          cursors.push(cursor);
        }
      } while (cursor !== null);
      return {tools, rejected};
    },
    // Pass a discovered definition so its parameter-header annotations can be honored. The
    // app owns caching and refresh. An input_required result is data for an explicit retry.
    callTool: async (tool, args, continuation) => {
      if (!__mcpObject(args)) throw __mcpProtocol("tool arguments must be an object");
      const fields = __mcpToolFields(tool);
      const headers = {"Mcp-Name": __mcpHeader(tool.name)};
      for (const field of fields) {
        let value = args;
        for (const part of field.path) value = __mcpObject(value) && __mcpHas(value, part) ? value[part] : undefined;
        if (value == null) continue;
        if ((field.type === "integer" && !Number.isSafeInteger(value)) ||
            (field.type !== "integer" && typeof value !== field.type)) throw __mcpProtocol("invalid mirrored parameter type");
        headers["Mcp-Param-" + field.name] = __mcpHeader(value);
      }
      const params = {name: tool.name, arguments: args};
      if (continuation != null) {
        if (!__mcpObject(continuation) || !__mcpObject(continuation.inputResponses)) throw __mcpProtocol("continuation requires inputResponses");
        params.inputResponses = continuation.inputResponses;
        if (__mcpHas(continuation, "requestState")) params.requestState = continuation.requestState;
      }
      const result = await __mcpRequest(fetchImpl, endpoint, "tools/call", params, headers);
      if (result.resultType === "input_required") {
        if (!__mcpObject(result.inputRequests)) throw __mcpProtocol("input_required has no inputRequests");
      } else if (!Array.isArray(result.content) ||
                 !result.content.every(part => __mcpObject(part) && typeof part.type === "string") ||
                 (result.structuredContent != null && !__mcpObject(result.structuredContent)) ||
                 (result.isError != null && typeof result.isError !== "boolean")) {
        throw __mcpProtocol("invalid tool result");
      }
      return result;
    }
  };
}
