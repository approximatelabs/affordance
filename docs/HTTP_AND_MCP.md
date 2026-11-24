# Fetch, HTTP bindings and MCP

Contract authority: [public contracts](../spec/CONTRACTS.md),
[language](../spec/LANGUAGE.md) and [harness](../spec/HARNESS.md).

Applications use global `fetch(input, options)` without an import. It returns a promise for
an HTTP `Response`. The host supplies network access through the existing executor boundary;
MCP is ordinary TSX composed over Fetch.

## Global Fetch

```tsx
const response = await fetch("/items", {
  method: "POST",
  headers: {"Content-Type": "application/json"},
  body: JSON.stringify({name: "example"})
});
if (!response.ok) throw new Error(`HTTP ${response.status}`);
const result = await response.json();
```

Fetch runs inside effects and handlers, where external results can be journaled. HTTP errors
such as 404 resolve with a Response; network or permission failures reject with TypeError.
The HTTP executor's deadline produces TimeoutError, and runtime cancellation produces
AbortError. The runtime's outer deadline retains its existing executor-timeout error.

Global fetch explicitly uses the role **`http`**. Configure and grant that role in deployment:

```json
"machines": {
  "http": {
    "kind": "http",
    "url": "https://api.example.com/v1/",
    "timeout_ms": 30000,
    "max_request_bytes": 8388608,
    "max_response_bytes": 8388608
  }
}
```

An instance's `machines: ["http"]` explicitly grants access. Without the `http` role, global
fetch fails; it never guesses another binding. Existing defaults still apply: omitting an
instance's role list grants all deployment bindings. Children can receive only roles their
parent holds.

Paths resolve against the configured URL: `items` uses its base path and `/items` starts at
the origin's root. Absolute URLs must have the same scheme, hostname and port. Fragments
are removed by Fetch. URL credentials are rejected. The permission covers the entire origin,
not just the base path. CONNECT, TRACE, TRACK and app-controlled routing/framing headers are
blocked. The host rechecks these limits, regardless of the TSX facade.

Requests currently support `method`, `headers`, `body`, `redirect` and `signal` (an
`AbortSignal`: aborting it cancels the request, which rejects with `AbortError`). Bodies are text;
string conversion and the default `text/plain;charset=UTF-8` content type follow Fetch.
GET and HEAD cannot carry a body. Headers accept a record, pairs or a Headers instance.
Unsupported options fail explicitly before I/O.

The host's no-redirect policy blocks default redirect following. Redirect statuses reject
unless `redirect: "manual"` is requested. Requests go through Affordance's one HTTP client, the
one model calls use: certificates are checked against the operating system's roots and the
Mozilla roots, proxies come from the environment as curl reads them (never for loopback
addresses), and idle connections are pooled for 4 seconds, shared by every executor in the
process. There is no cookie jar and no automatic retry. Cancellation drops the active request,
which closes its connection. Defaults are 30 seconds and 8 MiB per body, in
addition to the runtime's outer `call_timeout_s` limit. Oversized bodies reject rather than
silently truncate. The current transport accepts UTF-8 response bodies.

## Response and Headers

Supported Response properties are `ok`, `status`, `statusText`, `url`, `redirected`, `type`,
`headers` and `bodyUsed`. `text()` and `json()` consume a non-null body once; `clone()` must
be called before consumption and gives an independently consumable copy. HEAD and null-body
statuses have no body. Fetched headers are immutable. Reading a response or parsing JSON
adds no external effect: the underlying HTTP result is already journaled.

Global `Headers` supports construction, `get`, `has`, `set`, `append`, `delete`, `forEach`
and `getSetCookie`. Names are normalized and validated; repeated headers are preserved and
combined as the Web API specifies. Global `Response` supports text/null construction,
`clone()` and `Response.json(data, init)` for tests and application-defined fetch functions.

These are a documented subset, not a claim of full browser Fetch compatibility. Request,
URL and Headers iterator APIs, ReadableStream, binary bodies, cookies and CORS are not
implemented here. Accessing a non-null `response.body` stream fails explicitly; use
`text()` or `json()`. Responses are buffered until complete, so headers and progress are not
delivered before the body. Keep Response objects local to handlers; persist parsed data.

The standard-library implementation is readable JavaScript in `src/web.tsx`, embedded in the
engine. Applications can shadow the public globals; its internal classes retain their own
identities. Node differential tests exercise the supported semantics.

## Named capabilities and injected Fetch

For another origin or credential scope, use a named function rather than extending RequestInit:

```tsx
import { createFetch } from "@affordance/lib/http.tsx";

const fetchDocs = createFetch("docs");
const response = await fetchDocs("/search", {method: "POST", body: "example"});
```

The deployment must bind `docs` to that instance. `createFetch` shares the global Fetch
implementation and security checks. An app or library can accept this function as a normal
argument. No credential scope is selected implicitly from a URL.

## Credentials

Deployment headers are protected: application requests cannot replace them, even with
different capitalization. Headers can be executor-owned literal values or environment-derived:

```json
"headers": {
  "Authorization": {"env": "MCP_TOKEN", "prefix": "Bearer "}
}
```

The environment source is the actual caller's `CallContext.env`. Supply the value through
deployment or instance `env` and list its key in `secrets` for persistence/redaction controls.
These values remain readable by TSX through `harness.env`. For credentials that must stay
outside app code, keep the protected header value exclusively in the host executor's
configuration, outside the app environment. Bindings do not hide credentials from another
broad filesystem/shell permission granted to the same app.
Keep credentials out of URL query strings: `response.url` exposes the requested URL.

Headers are inserted in the host, outside model-visible definitions and Fetch arguments.
Transport errors omit URLs, headers and bodies. Server responses may contain sensitive data
and are journaled as returned. Tool visibility in a view does not replace server authorization.

## MCP as a Fetch library

```tsx
import { mcpClient } from "@affordance/lib/mcp.tsx";

const client = mcpClient("/mcp");
const server = await client.discover();
const {tools, rejected} = await client.listTools();
const search = tools.find(tool => tool.name === "search");
const reply = await client.callTool(search, {query: "installation"});
```

For a named binding: `mcpClient("/mcp", {fetch: createFetch("docs")})`. An injected function
must follow Fetch's input/options/Response shape; it can also be an ordinary TSX test stub
returning `Response.json(...)`. The MCP library uses no machine or executor APIs itself.
The earlier role-based `mcpClient(role)` now takes an endpoint URL. Deployments using the
global function grant `http`; other role names use an injected `createFetch(role)`.

Discovery returns data and installs no prompt instructions or affordances. The app owns
filtering, naming, caching and refresh. Tool pagination follows up to 100 pages and rejects
repeated cursors and duplicate names. Invalid tool definitions are excluded and reported in
`rejected` as `{name, reason}`. Passing a discovered definition to `callTool` lets the library
honor `x-mcp-header` annotations. Schema references are preserved, never fetched. The server
remains responsible for full JSON Schema validation.

`callTool` preserves content blocks, structuredContent, isError and metadata. Tool-reported
errors remain ordinary results. Protocol, RPC and HTTP failures throw objects named
McpProtocolError, McpRpcError and McpHttpError respectively, with message and relevant
code/data/status fields. An `input_required` result is returned unchanged; the app can call
again with `{inputResponses, requestState}` as the third argument. No sampling, roots or
elicitation capabilities are advertised automatically.

The library targets **MCP 2026-07-28 Streamable HTTP**, using JSON or finite SSE responses.
It handles request metadata, header mirroring, SSE comments and multiline data. Live
subscriptions, stdio, older initialize/session protocols and automatic OAuth flows are outside
this version. Servers must finish response streams within the configured time and body limits.

Edit `views/demo/mcp/affordance.yaml` to use your endpoint, model server and tokenizer.
Its `expose` prop maps selected server tools to app-selected affordance names. The example
prints raw MCP results as JSON and retains structured results in action metadata; media
blocks are not automatically rendered as model media.
Changing `url` or `expose` hides the old catalogue and pauses new conversation calls until
discovery finishes for that configuration. Late results and failures from an older discovery
cannot replace the current catalogue or stop the new configuration. The example enqueues its
initial `task` once. Already issued model/tool calls retain the view and endpoint they captured.

```sh
affordance run views/demo/mcp
```

## Official JavaScript SDK alignment

The official [TypeScript SDK v2](https://github.com/modelcontextprotocol/typescript-sdk)
accepts a `fetch` function in its client transport options. This is the interface used here.
The published `@modelcontextprotocol/server` 2.3.1 package is also an optional interoperability
oracle: a real SDK server serves discovery, tool definitions, structured results and tool
errors over JSON and SSE to the Affordance app's global Fetch.

**The full SDK does not run unchanged inside Affordance yet.** Its package imports and
transitive dependencies need npm/ES-module loading. Its client also needs additional Web APIs,
including URL, AbortController, streaming body readers and TextDecoderStream. The current
work aligns the supported Fetch contract and protocol wire behavior; it does not substitute
our smaller client for the complete SDK's API.

Install the optional development oracle outside the shipped runtime:

```sh
npm install --prefix target/mcp-sdk-oracle --ignore-scripts --no-audit --no-fund \
  --save-exact @modelcontextprotocol/server@2.3.1 zod@4.6.7
cargo test --release --test mcp official_javascript_sdk_server
```

Without those packages the SDK-specific test reports that its optional oracle is unavailable.
Node's native Headers/Response differential and all Rust fixture tests remain independent
of those packages. `AFFORDANCE_MCP_SDK_DIR` selects another installation directory.

## Replay and the host interface

Fetch uses the same journaled machine effects as the underlying HTTP executor. Committed
results replay without network access; an operation without a committed result can repeat
on recovery. Neither HTTP nor MCP provides exactly-once execution. The library does not
retry tool calls automatically. Use server/application idempotency for changing operations.

Rust hosts bind `affordance::connectors::http::Http::new(HttpConfig)` to `http` or another role
using `Runtime::bind_machine`. The `http` Cargo feature is enabled by default; renderer-only
embedders can omit its HTTP/TLS dependencies with `--no-default-features`. The global TSX
standard library remains available without that host adapter.

The lower-level executor contract remains available for host integrations: a machine call
accepts `{path?, method?, headers?, body?, timeout_ms?}` and returns UTF-8
`{status, statusText, url, headers, body}`, with lowercase header names and arrays of values.
A raw call's timeout may shorten the deployment limit, never increase it.

```sh
cargo test --release --test fetch --test http_connector --test mcp
```
