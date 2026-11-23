//! The MCP implementation is TSX running on the unmodified machine-effect boundary.
#![cfg(feature = "http")]
use crate::http_fixture as fixture;
use affordance::{
    connectors::http::{Http, HttpConfig},
    emit::Encoder,
    runtime::{AgentSpec, CallContext, Executor, FnModel, GenRequest, Runtime, RuntimeConfig},
};
use fixture::{Response, Server};
use serde_json::{Value as J, json};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "aff-mcp-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn app(&self, program: &str) -> PathBuf {
        let source = format!(
            r#"import {{ mcpClient }} from "{}/views/lib/mcp.tsx";
export default function Main() {{ useEffect(() => {{ (async () => {{
  try {{ {} }} catch (error) {{ harness.stop({{caught: {{name: error.name, message: error.message, code: error.code, data: error.data, status: error.status}}}}); }}
}})(); }}, []); return "waiting"; }}"#,
            env!("CARGO_MANIFEST_DIR"),
            program
        );
        let path = self.0.join("app.tsx");
        std::fs::write(&path, source).unwrap();
        path
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn encoder() -> Arc<Encoder> {
    static ENC: OnceLock<Arc<Encoder>> = OnceLock::new();
    ENC.get_or_init(|| {
        Arc::new(
            Encoder::from_file(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/ref/tokenizers/Qwen__Qwen3-4B-Instruct-2507.json"
            ))
            .unwrap(),
        )
    })
    .clone()
}
fn config(app: &Path) -> RuntimeConfig {
    let mut config = RuntimeConfig::new(app);
    config.workers = 1;
    config.model_threads = 1;
    config.exec_threads = 4;
    config.max_wall = Some(Duration::from_secs(10));
    // The apps are written to a temp directory and import the repository's views and fixtures.
    config.library = Some(env!("CARGO_MANIFEST_DIR").into());
    config.strict_replay = true;
    config.env.insert("TOKEN".into(), "mcp-test-credential".into());
    config.secrets.insert("TOKEN".into());
    config
}
fn transport(url: &str) -> Http {
    Http::new(
        serde_json::from_value::<HttpConfig>(json!({"url": format!("{url}/mcp"), "timeout_ms": 1000,
        "headers": {"Authorization": {"env": "TOKEN", "prefix": "Bearer "}}}))
        .unwrap(),
    )
    .unwrap()
}
fn model() -> Arc<dyn affordance::runtime::Model> {
    Arc::new(FnModel(|_: &GenRequest| {
        panic!("the fixture app does not call a model")
    }))
}
fn run(app: &Path, url: &str) -> J {
    let mut runtime = Runtime::new(config(app), encoder(), model());
    runtime.bind_machine("http", Arc::new(transport(url)));
    let mut spec = AgentSpec::new("a", json!({}));
    spec.model = None;
    runtime.add_agent(spec).unwrap();
    let report = runtime.run().unwrap();
    assert!(report.agents[0].error.is_none(), "{:?}", report.agents[0].error);
    report.agents[0].stopped.clone().expect("app did not stop")
}
fn tool(name: &str) -> J {
    json!({"name": name, "description": "fixture tool", "inputSchema": {"type": "object", "properties": {
        "location": {"type": "object", "properties": {"region": {"type": "string", "x-mcp-header": "Region"}}},
        "count": {"type": "integer", "x-mcp-header": "Count"},
        "enabled": {"type": "boolean", "x-mcp-header": "Enabled"}
    }}})
}
fn fixture() -> Server {
    Server::new(|request| {
        assert_eq!((request.method.as_str(), request.path.as_str()), ("POST", "/mcp"));
        assert_eq!(request.headers["authorization"], "Bearer mcp-test-credential");
        assert_eq!(request.headers["accept"], "application/json, text/event-stream");
        assert_eq!(request.headers["mcp-protocol-version"], "2026-07-28");
        let body: J = serde_json::from_str(&request.body).unwrap();
        assert_eq!(body["jsonrpc"], "2.0");
        assert_eq!(
            body["params"]["_meta"]["io.modelcontextprotocol/protocolVersion"],
            "2026-07-28"
        );
        assert_eq!(
            body["params"]["_meta"]["io.modelcontextprotocol/clientCapabilities"],
            json!({})
        );
        assert_eq!(body["method"].as_str().unwrap(), request.headers["mcp-method"]);
        let result = match body["method"].as_str().unwrap() {
            "server/discover" => {
                json!({"resultType": "complete", "supportedVersions": ["2026-07-28"], "capabilities": {"tools": {}},
                "_meta": {"io.modelcontextprotocol/serverInfo": {"name": "offline fixture", "version": "1"}}})
            }
            "tools/list" if body["params"].get("cursor").is_none() => {
                json!({"tools": [tool("echo"), tool("error"),
                {"name": "bad", "inputSchema": {"type": "object", "items": {"type": "string", "x-mcp-header": "Bad"}}}], "nextCursor": "next"})
            }
            "tools/list" => {
                assert_eq!(body["params"]["cursor"], "next");
                json!({"resultType": "complete", "tools": [tool("世界"), tool("input")]})
            }
            "tools/call" => {
                let name = body["params"]["name"].as_str().unwrap();
                assert_eq!(
                    request.headers["mcp-name"],
                    if name == "世界" { "=?base64?5LiW55WM?=" } else { name }
                );
                match name {
                    "error" => {
                        json!({"content": [{"type": "text", "text": "tool could not complete"}], "isError": true})
                    }
                    "input" if body["params"].get("inputResponses").is_none() => {
                        json!({"resultType": "input_required",
                        "inputRequests": {"confirm": {"method": "elicitation/create", "params": {}}}, "requestState": "opaque-state"})
                    }
                    "input" => {
                        assert_eq!(body["params"]["requestState"], "opaque-state");
                        json!({"content": [], "structuredContent": {"continued": true}})
                    }
                    _ => {
                        json!({"resultType": "complete", "content": [{"type": "text", "text": "héllo"},
                        {"type": "image", "data": "aGVsbG8=", "mimeType": "image/png"},
                        {"type": "resource_link", "uri": "fixture://result", "name": "result"}],
                        "structuredContent": {"arguments": body["params"]["arguments"]}, "isError": false})
                    }
                }
            }
            method => panic!("unexpected method {method}"),
        };
        let response = json!({"jsonrpc": "2.0", "id": body["id"], "result": result});
        if body["method"] == "tools/call" {
            let notification = json!({"jsonrpc": "2.0", "method": "notifications/progress", "params": {"progress": 1}});
            // Exercise comments, CRLF and multi-line SSE JSON, split across HTTP chunks.
            let text = format!(
                ": heartbeat\r\ndata: {notification}\r\n\r\ndata: {{\r\ndata: \"jsonrpc\":\"2.0\",\"id\":{},\"result\":{}\r\ndata: }}\r\n\r\n",
                body["id"], response["result"]
            );
            Response::sse(&text)
        } else {
            Response::json(response)
        }
    })
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn tsx_mcp_discovers_pages_invokes_and_replays_without_network() {
    let dir = Scratch::new();
    let app = dir.app(r#"
const client = mcpClient("/mcp");
const info = await client.discover();
const listed = await client.listTools();
const args = {location: {region: "Hello, 世界"}, count: 0, enabled: false, arbitrary: [1, {ok: true}]};
const echo = listed.tools.find(tool => tool.name === "echo");
const other = listed.tools.find(tool => tool.name === "世界");
const failure = listed.tools.find(tool => tool.name === "error");
const input = listed.tools.find(tool => tool.name === "input");
const results = await Promise.all([client.callTool(echo, args), client.callTool(other, {}), client.callTool(failure, {})]);
const required = await client.callTool(input, {});
const continued = await client.callTool(input, {}, {inputResponses: {confirm: {action: "accept"}}, requestState: required.requestState});
harness.stop({info, listed, results, required, continued});
"#);
    let server = fixture();
    let count = Arc::new(AtomicUsize::new(0));
    let transport = Arc::new(transport(&server.url));
    let executor = {
        let (count, http) = (count.clone(), transport.clone());
        Arc::new(move |ctx: &CallContext, args: &J| {
            count.fetch_add(1, Ordering::SeqCst);
            http.call(ctx, args)
        })
    };
    let mut config = config(&app);
    config.dir = Some(dir.0.join("run"));
    let mut runtime = Runtime::new(config.clone(), encoder(), model());
    runtime.bind_machine("http", executor.clone());
    let mut spec = AgentSpec::new("a", json!({}));
    spec.model = None;
    runtime.add_agent(spec).unwrap();
    let report = runtime.run().unwrap();
    assert!(report.agents[0].error.is_none(), "{:?}", report.agents[0].error);
    let stopped = report.agents[0].stopped.as_ref().expect("did not stop");
    assert!(stopped.get("caught").is_none(), "{stopped}");
    assert_eq!(stopped["listed"]["tools"].as_array().unwrap().len(), 4);
    assert_eq!(stopped["listed"]["rejected"][0]["name"], "bad");
    assert_eq!(stopped["results"][0]["content"].as_array().unwrap().len(), 3);
    assert_eq!(
        stopped["results"][0]["structuredContent"]["arguments"]["arbitrary"],
        json!([1, {"ok": true}])
    );
    assert_eq!(stopped["results"][2]["isError"], true, "tool errors remain results");
    assert_eq!(stopped["continued"]["structuredContent"]["continued"], true);
    let seen = server.requests.lock().unwrap();
    let echo = seen
        .iter()
        .find(|r| r.headers.get("mcp-name").is_some_and(|n| n == "echo"))
        .unwrap();
    assert_eq!(echo.headers["mcp-param-region"], "=?base64?SGVsbG8sIOS4lueVjA==?=");
    assert_eq!(echo.headers["mcp-param-count"], "0");
    assert_eq!(echo.headers["mcp-param-enabled"], "false");
    let mut ids = std::collections::BTreeSet::new();
    for request in seen.iter() {
        assert!(ids.insert(serde_json::from_str::<J>(&request.body).unwrap()["id"].to_string()));
    }
    let calls = seen.len();
    drop(seen);
    assert_eq!(calls, 8);
    let journal = std::fs::read_to_string(crate::shared::inst(&dir.0, "run/a/journal.jsonl")).unwrap();
    assert!(!journal.contains("mcp-test-credential"));
    drop(server);
    let mut restored = Runtime::restore(config, encoder(), model()).unwrap();
    restored.bind_machine("http", executor);
    let replay = restored.run().unwrap();
    assert!(replay.agents[0].error.is_none(), "{:?}", replay.agents[0].error);
    assert_eq!(replay.agents[0].stopped.as_ref(), Some(stopped));
    assert_eq!(
        count.load(Ordering::SeqCst),
        calls,
        "committed effects must not contact the stopped server"
    );
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn protocol_rpc_and_http_errors_are_distinct_from_tool_errors() {
    let dir = Scratch::new();
    let app = dir.app("const listed = await mcpClient('/mcp').listTools(); harness.stop(listed);");
    for (case, expected) in [
        ("rpc", "McpRpcError"),
        ("rpc-http", "McpRpcError"),
        ("http", "McpHttpError"),
        ("http-json", "McpHttpError"),
        ("http-malformed-json", "McpHttpError"),
        ("id", "McpProtocolError"),
        ("json", "McpProtocolError"),
        ("envelope", "McpProtocolError"),
        ("cursor", "McpProtocolError"),
        ("empty", "McpProtocolError"),
        ("duplicate", "McpProtocolError"),
    ] {
        let server = Server::new(move |request| {
            let body: J = serde_json::from_str(&request.body).unwrap();
            match case {
                "rpc" => Response::json(
                    json!({"jsonrpc": "2.0", "id": body["id"], "error": {"code": -32602, "message": "invalid params", "data": {"field": "q"}}}),
                ),
                "http" => Response::text(401, "sign in"),
                "http-json" | "http-malformed-json" => {
                    let mut response = Response::json(json!({"error": "sign in"}));
                    response.status = 401;
                    if case == "http-malformed-json" {
                        response.body = b"{broken".to_vec();
                    }
                    response
                }
                "rpc-http" => {
                    let mut response = Response::json(json!({"jsonrpc": "2.0", "id": null, "error": {
                        "code": -32022, "message": "unsupported version", "data": {"supported": ["different"]}
                    }}));
                    response.status = 400;
                    response
                }
                "id" => Response::json(json!({"jsonrpc": "2.0", "id": "wrong", "result": {"tools": []}})),
                "json" => {
                    let mut r = Response::json(json!({}));
                    r.body = b"{broken".to_vec();
                    r
                }
                "envelope" => Response::json(json!({"result": {"tools": []}})),
                "cursor" => Response::json(
                    json!({"jsonrpc": "2.0", "id": body["id"], "result": {"tools": [], "nextCursor": "again"}}),
                ),
                "empty" => Response::sse(": heartbeat\n\n"),
                "duplicate" => {
                    let reply = json!({"jsonrpc": "2.0", "id": body["id"], "result": {"tools": []}});
                    Response::sse(&format!("data: {reply}\n\ndata: {reply}\n\n"))
                }
                _ => unreachable!(),
            }
        });
        let result = run(&app, &server.url);
        assert_eq!(result["caught"]["name"], expected, "{case}: {result}");
        if case == "rpc" {
            assert_eq!(result["caught"]["code"], -32602);
            assert_eq!(result["caught"]["data"]["field"], "q");
        }
    }
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn network_authority_is_still_an_instance_binding() {
    let dir = Scratch::new();
    let app = dir.app("await mcpClient('/mcp').discover(); harness.stop('unexpected');");
    let server = fixture();
    let mut runtime = Runtime::new(config(&app), encoder(), model());
    runtime.bind_machine("http", Arc::new(transport(&server.url)));
    let mut spec = AgentSpec::new("a", json!({}));
    spec.model = None;
    spec.machines = Some(vec![]);
    runtime.add_agent(spec).unwrap();
    let report = runtime.run().unwrap();
    assert!(
        report.agents[0].stopped.as_ref().unwrap()["caught"]["message"]
            .as_str()
            .unwrap()
            .contains("no machine http")
    );
    assert!(server.requests.lock().unwrap().is_empty());
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn mcp_accepts_an_injected_fetch_function_with_standard_responses() {
    let dir = Scratch::new();
    let app = dir.app(
        r#"
const client = mcpClient("https://stub.example/mcp", {fetch: async (url, init) => {
  const request = JSON.parse(init.body);
  if (url !== "https://stub.example/mcp" || init.method !== "POST") throw new Error("unexpected request");
  return Response.json({jsonrpc: "2.0", id: request.id, result: {tools: []}});
}});
harness.stop(await client.listTools());
"#,
    );
    // No network binding: the injected function is ordinary app code.
    let mut runtime = Runtime::new(config(&app), encoder(), model());
    let mut spec = AgentSpec::new("a", json!({}));
    spec.model = None;
    runtime.add_agent(spec).unwrap();
    let report = runtime.run().unwrap();
    assert!(report.agents[0].error.is_none(), "{:?}", report.agents[0].error);
    assert_eq!(report.agents[0].stopped, Some(json!({"tools": [], "rejected": []})));
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn example_exposes_only_app_selected_tools_and_dispatches_the_alias() {
    use affordance::runtime::GenReply;
    let app = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/views/demo/mcp/app.tsx"));
    let enc = Arc::new(
        Encoder::from_file(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/ref/tokenizers/zai-org__GLM-5.3.json"
        ))
        .unwrap(),
    );
    let reply_enc = enc.clone();
    let calls = Arc::new(AtomicUsize::new(0));
    let model_calls = calls.clone();
    let model = Arc::new(FnModel(move |request: &GenRequest| {
        let prompt = reply_enc.tok.decode(&request.ids, false).unwrap();
        assert!(prompt.contains("remote_echo"), "{prompt}");
        assert!(!prompt.contains("mcp-test-credential"));
        assert!(!prompt.contains("\"name\": \"error\""));
        let first = model_calls.fetch_add(1, Ordering::SeqCst) == 0;
        let text = if first {
            "Call.</think><tool_call>remote_echo<arg_key>enabled</arg_key><arg_value>false</arg_value></tool_call>"
        } else {
            "Done.</think>Finished."
        };
        let mut ids = reply_enc.tok.encode(text, false).unwrap().get_ids().to_vec();
        ids.push(
            reply_enc
                .token_id(if first { "<|observation|>" } else { "<|user|>" })
                .unwrap(),
        );
        Ok(GenReply {
            ids,
            ..Default::default()
        })
    }));
    let server = fixture();
    let mut runtime = Runtime::new(config(&app), enc, model);
    runtime.bind_machine("http", Arc::new(transport(&server.url)));
    let mut spec = AgentSpec::new(
        "a",
        json!({"task": "Try the remote tool.", "expose": {"echo": "remote_echo"}}),
    );
    spec.max_turns = Some(4);
    runtime.add_agent(spec).unwrap();
    let report = runtime.run().unwrap();
    assert!(report.agents[0].error.is_none(), "{:?}", report.agents[0].error);
    assert_eq!(calls.load(Ordering::SeqCst), 2, "{report:?}");
    let state = &report.agents[0].state;
    let actions: Vec<&J> = state["__ui"]["@log"]["records"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["kind"] == "action")
        .collect();
    assert_eq!(actions.len(), 1, "{state}");
    assert_eq!(actions[0]["name"], "remote_echo");
    assert_eq!(actions[0]["mcp"]["structuredContent"]["arguments"]["enabled"], false);
    let seen = server.requests.lock().unwrap();
    let invoked: Vec<J> = seen
        .iter()
        .map(|r| serde_json::from_str::<J>(&r.body).unwrap())
        .filter(|r| r["method"] == "tools/call")
        .collect();
    assert_eq!(invoked.len(), 1);
    assert_eq!(
        invoked[0]["params"]["name"], "echo",
        "the app alias must not change the server name"
    );
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn mirrored_headers_encode_padding_newlines_and_literal_sentinels_and_skip_null() {
    let dir = Scratch::new();
    let app = dir.app(
        r#"
const tool = {name: "echo", inputSchema: {type: "object", properties: {
  value: {type: "string", "x-mcp-header": "Value"}
}}};
const client = mcpClient("/mcp");
for (const value of [" padded ", "line1\nline2", "=?base64?literal?=", "ordinary", null]) {
  await client.callTool(tool, {value});
}
harness.stop("done");
"#,
    );
    let expected = [
        "=?base64?IHBhZGRlZCA=?=",
        "=?base64?bGluZTEKbGluZTI=?=",
        "=?base64?PT9iYXNlNjQ/bGl0ZXJhbD89?=",
        "ordinary",
    ];
    let index = Arc::new(AtomicUsize::new(0));
    let server = Server::new(move |request| {
        let i = index.fetch_add(1, Ordering::SeqCst);
        if i < expected.len() {
            assert_eq!(request.headers["mcp-param-value"], expected[i]);
        } else {
            assert!(!request.headers.contains_key("mcp-param-value"));
        }
        let body: J = serde_json::from_str(&request.body).unwrap();
        Response::json(json!({"jsonrpc": "2.0", "id": body["id"], "result": {"content": []}}))
    });
    assert_eq!(run(&app, &server.url), "done");
    assert_eq!(server.requests.lock().unwrap().len(), 5);
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn schema_annotations_are_checked_without_confusing_instance_data_with_schemas() {
    let dir = Scratch::new();
    let app = dir.app("harness.stop(await mcpClient('/mcp').listTools());");
    let server = Server::new(|request| {
        let body: J = serde_json::from_str(&request.body).unwrap();
        Response::json(json!({"jsonrpc": "2.0", "id": body["id"], "result": {"tools": [
            {"name": "valid", "inputSchema": {"type": "object", "default": {"x-mcp-header": "ordinary data"},
                "properties": {"q": {"type": "string", "x-mcp-header": "Query"}}}},
            {"name": "reference", "inputSchema": {"type": "object", "properties": {"q": {"$ref": "https://example.invalid/schema"}}}},
            {"name": "bad-ref", "inputSchema": {"type": "object", "$defs": {"q": {"type": "string", "x-mcp-header": "Query"}}}},
            {"name": "bad-branch", "inputSchema": {"type": "object", "oneOf": [{"properties": {"q": {"type": "string", "x-mcp-header": "Query"}}}]}},
            {"name": "duplicate", "inputSchema": {"type": "object", "properties": {
                "a": {"type": "string", "x-mcp-header": "Region"}, "b": {"type": "string", "x-mcp-header": "region"}}}},
            {"name": "bad-name", "inputSchema": {"type": "object", "properties": {"q": {"type": "string", "x-mcp-header": "Bad\r\nHeader"}}}},
            {"name": "bad-type", "inputSchema": {"type": "object", "properties": {"q": {"type": "number", "x-mcp-header": "Number"}}}}
        ]}}))
    });
    let result = run(&app, &server.url);
    let names: Vec<&str> = result["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["valid", "reference"]);
    assert_eq!(result["rejected"].as_array().unwrap().len(), 5);
    assert_eq!(
        server.requests.lock().unwrap().len(),
        1,
        "schema refs must never trigger HTTP requests"
    );
}

#[test]
fn official_javascript_sdk_server_interoperates_over_global_fetch() {
    use std::io::{BufRead, BufReader};
    use std::process::{Child, Stdio};
    let sdk_dir = std::env::var_os("AFFORDANCE_MCP_SDK_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/target/mcp-sdk-oracle")));
    if !sdk_dir
        .join("node_modules/@modelcontextprotocol/server/package.json")
        .is_file()
    {
        eprintln!("Optional SDK oracle unavailable; see docs/HTTP_AND_MCP.md for installation.");
        return;
    }
    struct ServerProcess(Child);
    impl Drop for ServerProcess {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let dir = Scratch::new();
    let app = dir.app(
        r#"
const client = mcpClient("/mcp");
const info = await client.discover();
const listed = await client.listTools();
const echo = listed.tools.find(tool => tool.name === "echo");
const fail = listed.tools.find(tool => tool.name === "fail");
const result = await client.callTool(echo, {text: "Real SDK: 世界"});
const failure = await client.callTool(fail, {});
harness.stop({info, listed, result, failure});
"#,
    );
    for mode in ["auto", "sse"] {
        let process = std::process::Command::new("node")
            .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/support/mcp_sdk_server.mjs"))
            .arg(&sdk_dir)
            .arg(mode)
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let mut process = ServerProcess(process);
        let mut line = String::new();
        BufReader::new(process.0.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        assert!(line.starts_with("http://127.0.0.1:"), "SDK did not start: {line}");
        let url = line.trim().strip_suffix("/mcp").unwrap();
        let result = run(&app, url);
        assert!(result.get("caught").is_none(), "{mode}: {result}");
        assert!(
            result["info"]["supportedVersions"]
                .as_array()
                .unwrap()
                .contains(&json!("2026-07-28"))
        );
        assert_eq!(result["listed"]["tools"].as_array().unwrap().len(), 2);
        assert_eq!(result["result"]["structuredContent"]["text"], "Real SDK: 世界");
        assert_eq!(result["failure"]["isError"], true);
    }
}
