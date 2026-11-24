use affordance::{
    emit::Encoder,
    runtime::{CallContext, FnModel, GenRequest, InstanceSpec, Runtime, RuntimeConfig},
};
use serde_json::{Value as J, json};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

#[cfg(feature = "http")]
use crate::http_fixture as fixture;

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "aff-fetch-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn app(&self, imports: &str, program: &str) -> PathBuf {
        let source = format!(
            r#"{}
export default function Main() {{ useEffect(() => {{ (async () => {{
try {{ {} }} catch (error) {{ harness.stop({{error: error.name, message: error.message}}); }}
}})(); }}, []); return "waiting"; }}"#,
            imports, program
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
    static ENCODER: OnceLock<Arc<Encoder>> = OnceLock::new();
    ENCODER
        .get_or_init(|| {
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
fn runtime(app: &Path) -> Runtime {
    let mut config = RuntimeConfig::new(app);
    config.workers = 1;
    config.model_threads = 1;
    config.exec_threads = 4;
    config.max_wall = Some(Duration::from_secs(8));
    // The apps are written to a temp directory and import the repository's library and fixtures.
    config.library = Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    Runtime::new(
        config,
        encoder(),
        Arc::new(FnModel(|_: &GenRequest| panic!("unexpected model call"))),
    )
}
fn run(mut runtime: Runtime) -> J {
    let mut spec = InstanceSpec::new("a", json!({}));
    spec.model = None;
    runtime.add_instance(spec).unwrap();
    let report = runtime.run().unwrap();
    assert!(report.instances[0].error.is_none(), "{:?}", report.instances[0].error);
    report.instances[0].stopped.clone().expect("app did not stop")
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn headers_and_response_match_nodes_native_web_apis() {
    let dir = Scratch::new();
    let cases = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/fetch_cases.tsx");
    let node = std::process::Command::new("node")
        .args([
            "--input-type=module",
            "-e",
            r#"
import fs from 'node:fs';
const source = fs.readFileSync(process.argv[1], 'utf8');
const {webCases} = await import('data:text/javascript;base64,' + Buffer.from(source).toString('base64'));
const result = await webCases();
process.stdout.write(JSON.stringify(result));
"#,
            cases,
        ])
        .output()
        .expect("Node is required for the Fetch differential test");
    assert!(node.status.success(), "{}", String::from_utf8_lossy(&node.stderr));
    let expected: J = serde_json::from_slice(&node.stdout).unwrap();
    let app = dir.app(
        &format!("import {{ webCases }} from {};", json!(cases)),
        "harness.stop(await webCases());",
    );
    assert_eq!(run(runtime(&app)), expected);
}

#[cfg(feature = "http")]
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn global_fetch_returns_standard_responses_and_reuses_existing_authority_checks() {
    use affordance::connectors::http::{Http, HttpConfig};
    let server = fixture::Server::new(|request| {
        assert_eq!(request.headers["authorization"], "Bearer host-only-credential");
        if request.path == "/redirect" {
            let mut response = fixture::Response::text(302, "redirect");
            response
                .headers
                .push(("Location".into(), "https://example.invalid/".into()));
            return response;
        }
        if request.path == "/empty" {
            return fixture::Response::text(204, "");
        }
        let mut response =
            fixture::Response::json(json!({"method": request.method, "path": request.path, "body": request.body}));
        if request.path == "/missing" {
            response.status = 404;
        }
        response.headers.push(("X-Fixture".into(), "yes".into()));
        response
    });
    let dir = Scratch::new();
    let app = dir.app("", r#"
const response = await fetch("/items#fragment", {method: "post", headers: new Headers({"Content-Type": "application/json"}), body: JSON.stringify({q: "tea"})});
const clone = response.clone();
const info = [response instanceof Response, response.headers instanceof Headers, response.ok, response.status, response.statusText, response.url, response.type, response.redirected];
let immutable = "";
try { response.headers.set("Authorization", "replacement"); } catch (error) { immutable = error.name; }
const value = await response.json();
let consumed = "";
try { await response.text(); } catch (error) { consumed = error.name; }
const empty = await fetch("/empty");
const missing = await fetch("/missing");
const manual = await fetch("/redirect", {redirect: "manual"});
let redirected = "";
try { await fetch("/redirect"); } catch (error) { redirected = error.name; }
let escaped = "";
try { await fetch("https://example.invalid/"); } catch (error) { escaped = error.name; }
let protectedHeader = "";
try { await fetch("/items", {headers: {authorization: "replacement"}}); } catch (error) { protectedHeader = error.name; }
harness.stop({info, value, clone: await clone.json(), immutable, consumed, used: response.bodyUsed,
 empty: [await empty.text(), empty.bodyUsed, empty.body], missing: [missing.ok, missing.status], manual: manual.status, redirected, escaped, protectedHeader});
"#);
    let transport = Http::new(
        serde_json::from_value::<HttpConfig>(json!({"url": server.url,
        "headers": {"Authorization": "Bearer host-only-credential"}}))
        .unwrap(),
    )
    .unwrap();
    let mut runtime = runtime(&app);
    runtime.bind_machine("http", Arc::new(transport));
    let result = run(runtime);
    assert!(result.get("error").is_none(), "{result}");
    assert_eq!(
        result["info"],
        json!([
            true,
            true,
            true,
            200,
            "Fixture",
            format!("{}/items", server.url),
            "basic",
            false
        ])
    );
    assert_eq!(result["value"], result["clone"]);
    assert_eq!(result["value"]["method"], "POST");
    assert_eq!(result["value"]["body"], "{\"q\":\"tea\"}");
    for key in ["immutable", "consumed", "redirected", "escaped", "protectedHeader"] {
        assert_eq!(result[key], "TypeError", "{key}: {result}");
    }
    assert_eq!(result["used"], true);
    assert_eq!(result["empty"], json!(["", false, null]));
    assert_eq!(result["missing"], json!([false, 404]));
    assert_eq!(result["manual"], 302);
    assert_eq!(server.requests.lock().unwrap().len(), 5);
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn fetch_is_a_first_class_function_and_named_bindings_are_explicit() {
    let dir = Scratch::new();
    let library = concat!(env!("CARGO_MANIFEST_DIR"), "/views/lib/http.tsx");
    let app = dir.app(
        &format!("import {{ createFetch }} from {};", json!(library)),
        r#"
let missing = "";
try { await fetch("/items"); } catch (error) { missing = error.name; }
const custom = createFetch("docs");
async function request(fetchImpl) { return await (await fetchImpl("/items")).json(); }
harness.stop({missing, result: await request(custom)});
"#,
    );
    let count = Arc::new(AtomicUsize::new(0));
    let calls = count.clone();
    let mut runtime = runtime(&app);
    runtime.bind_machine(
        "docs",
        Arc::new(move |ctx: &CallContext, args: &J| {
            calls.fetch_add(1, Ordering::SeqCst);
            assert_eq!(ctx.client.name, "a");
            assert_eq!(args["path"], "/items");
            Ok(json!({"status": 200, "headers": {"content-type": ["application/json"]}, "body": "{\"ok\":true}"}))
        }),
    );
    assert_eq!(run(runtime), json!({"missing": "TypeError", "result": {"ok": true}}));
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn unsupported_fetch_features_fail_before_io_and_globals_can_be_shadowed() {
    let dir = Scratch::new();
    let app = dir.app(
        "",
        r#"
const errors = [];
for (const init of [{method: "GET", body: "bad"}, {method: "HEAD", body: "bad"}, {method: "TrAcK"},
 {signal: {}}, {redirect: "invalid"}, {headers: {"bad name": "x"}}, {cache: "no-store"}]) {
 try { await fetch("/items", init); errors.push("accepted"); } catch (error) { errors.push(error.name); }
}
harness.stop(errors);
"#,
    );
    let mut host = runtime(&app);
    host.bind_machine(
        "http",
        Arc::new(|_: &CallContext, _: &J| -> Result<J, String> { panic!("invalid fetch reached I/O") }),
    );
    assert_eq!(
        run(host),
        json!([
            "TypeError",
            "TypeError",
            "TypeError",
            "TypeError",
            "TypeError",
            "TypeError",
            "TypeError"
        ])
    );
    let app = dir.app("async function fetch(url) { return {url}; } function Response() {return 'app response';} function Headers() {return 'app headers';}", "harness.stop([Headers(), Response(), await fetch('local')]); ");
    assert_eq!(
        run(runtime(&app)),
        json!(["app headers", "app response", {"url": "local"}])
    );
}
