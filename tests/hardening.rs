//! Regressions for reproduced host-boundary, lifetime and process-crash defects.
use crate::tokenizer;
use affordance::{
    View,
    emit::{Encoder, RunCache},
    runtime::{Agent, AgentConfig, CallContext, CancelToken, Completions, Exec, Executor, GenRequest, Model, Target},
    value::{Coll, Obj, Value},
};
use serde_json::{Value as J, json};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    net::TcpListener,
    path::PathBuf,
    rc::Rc,
    sync::{
        OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
struct Files(PathBuf);
impl Files {
    fn new() -> Self {
        static N: AtomicUsize = AtomicUsize::new(0);
        let p = std::env::temp_dir().join(format!(
            "aff-audit-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&p).unwrap();
        Self(p)
    }
    fn app(&self, s: &str) -> View {
        let p = self.0.join("app.tsx");
        std::fs::write(&p, s).unwrap();
        View::load(&p, "Main", enc()).unwrap()
    }
}
impl Drop for Files {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn enc() -> &'static Encoder {
    static E: OnceLock<Encoder> = OnceLock::new();
    E.get_or_init(tokenizer::encoder)
}
fn cfg() -> AgentConfig {
    AgentConfig {
        context: None,
        retry: Default::default(),
        template: None,
        id: "a".into(),
        props: json!({}),
        env: BTreeMap::new(),
        max_new: 10,
        max_turns: None,
        max_prompt: None,
        start_ms: 0,
        seed: 1,
        machines: vec![],
        model: None,
    }
}
fn now() -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 10, 23)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
}
#[test]
fn store_autokeys_do_not_overwrite_user_keys() {
    let s = affordance::runtime::store::Store::new(None);
    s.call("a", "store.put", &json!({"key":"b1","text":"first"})).unwrap();
    let next = s.call("a", "store.put", &json!({"text":"second"})).unwrap();
    eprintln!("auto key={next}");
    assert_eq!(s.call("a", "store.get", &json!({"key":"b1"})).unwrap()["text"], "first");
}
#[test]
fn store_rejects_invalid_complete_records() {
    let f = Files::new();
    std::fs::create_dir_all(f.0.join("a")).unwrap();
    std::fs::write(f.0.join("a/store.jsonl"), "{\"unrelated\":true}\n").unwrap();
    let s = affordance::runtime::store::Store::new(Some(f.0.clone()));
    let r = s.load("a");
    eprintln!("load={r:?}");
    assert!(r.is_err());
}
#[test]
fn schema_additional_properties_matches_declared_contract() {
    let closed = affordance::reply::validate(
        &json!({"type":"object","properties":{},"additionalProperties":false}),
        &json!({"extra":1}),
    );
    let open = affordance::reply::validate(
        &json!({"type":"object","properties":{"x":{"type":"string"}}}),
        &json!({"x":"ok","extra":1}),
    );
    eprintln!("closed={closed:?}, default-open={open:?}");
    assert!(closed.is_err());
    assert!(open.is_ok());
    assert!(
        affordance::reply::validate(
            &json!({"type":"object","additionalProperties":{"type":"number"}}),
            &json!({"extra":"bad"})
        )
        .is_err()
    );
    assert!(
        affordance::reply::validate(
            &json!({"type":"object","additionalProperties":{"type":"number"}}),
            &json!({"extra":1})
        )
        .is_ok()
    );
}
fn peer(reply: J) -> (String, std::thread::JoinHandle<J>) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", l.local_addr().unwrap());
    let h = std::thread::spawn(move || {
        let (mut s, _) = l.accept().unwrap();
        s.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        let mut raw = vec![];
        let mut b = [0u8; 4096];
        let head = loop {
            let n = s.read(&mut b).unwrap();
            assert!(n > 0);
            raw.extend_from_slice(&b[..n]);
            if let Some(i) = raw.windows(4).position(|v| v == b"\r\n\r\n") {
                break i + 4;
            }
        };
        let text = String::from_utf8_lossy(&raw[..head]);
        let len: usize = text
            .lines()
            .find_map(|v| {
                v.to_ascii_lowercase()
                    .strip_prefix("content-length: ")
                    .and_then(|v| v.trim().parse().ok())
            })
            .unwrap();
        while raw.len() < head + len {
            let n = s.read(&mut b).unwrap();
            raw.extend_from_slice(&b[..n]);
        }
        let req = serde_json::from_slice(&raw[head..head + len]).unwrap();
        let body = reply.to_string();
        write!(
            s,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )
        .unwrap();
        req
    });
    (url, h)
}
#[test]
fn malformed_native_token_ids_are_rejected() {
    let (url, h) = peer(json!({"choices":[{"token_ids":[7,"bad",4294967296u64],"finish_reason":"stop"}]}));
    let mut m = Completions::new(&url, "m");
    m.stream = false;
    let req = GenRequest::new("a", vec![1, 2, 3], 10, vec![], None, CancelToken::new());
    let result = m.generate(&req);
    eprintln!("result={:?}", result.as_ref().map(|r| &r.ids));
    h.join().unwrap();
    assert!(result.is_err());
}
#[test]
fn cyclic_collections_are_refused_by_state_boundary() {
    let c = Rc::new(Obj::new(Coll::new(false)));
    let v = Value::Coll(c.clone());
    c.write().unwrap().insert(Value::str("self"), v.clone());
    let r = affordance::value::freeze_state(&v);
    eprintln!("cyclic state result={r:?}");
    assert!(r.is_err());
}
#[test]
fn local_instance_home_names_do_not_alias() {
    let f = Files::new();
    let e = Exec {
        target: Target::Local {
            home: Some(format!("{}/{{agent}}", f.0.display())),
        },
        default_timeout_s: 1,
        max_capture: 1024,
    };
    let context = |id: &str| CallContext {
        agent: id.into(),
        task: 0,
        env: BTreeMap::new(),
        cancel: CancelToken::new(),
        blobs: None,
        output: None,
    };
    let a = e.call(&context("root/child"), &json!({"cmd":"pwd"})).unwrap();
    let b = e.call(&context("root_child"), &json!({"cmd":"pwd"})).unwrap();
    eprintln!("first={}, second={}", a["output"], b["output"]);
    assert_ne!(a["output"], b["output"]);
}

fn render(view: &View, props: J) -> Result<String, String> {
    let Value::Map(props) = Value::from_json(&props) else {
        panic!("props")
    };
    let result = view.render(enc(), Rc::new(props.read().clone()), &mut RunCache::new(false), now())?;
    Ok(enc().tok.decode(&result.ids, false).unwrap())
}

#[test]
fn sampling_cannot_replace_the_captured_request() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    for key in [
        "prompt",
        "input_ids",
        "messages",
        "model",
        "tools",
        "stream",
        "return_token_ids",
        "stop_token_ids",
        "max_tokens",
        "max_new_tokens",
        "sampling_params",
    ] {
        let m = Completions::new(&url, "bound-model");
        let mut req = GenRequest::new("a", vec![1, 2, 3], 10, vec![], None, CancelToken::new());
        req.sampling = json!({key: "injected"});
        let err = m.generate(&req).unwrap_err();
        assert!(!err.kind.may_pass(), "{key}: {err:?}");
        assert!(err.message.contains("host-owned"), "{key}: {err:?}");
        assert!(listener.accept().is_err(), "{key} reached the network");
    }
}

#[test]
fn module_class_statics_use_the_current_render_inputs() {
    let f = Files::new();
    let view = f.app("class Box { static value = flag; } export default function Main(){return String(Box.value);}");
    assert_eq!(render(&view, json!({"flag": 1})).unwrap(), "1");
    assert_eq!(render(&view, json!({"flag": 2})).unwrap(), "2");
}

#[test]
fn lexical_bindings_and_aliases_resolve_consistently() {
    let f = Files::new();
    for (source, expected) in [
        (
            "const Math = {max: () => 77, PI: 12}; export default function Main(){const f=Math.max; return `${Math.max(1,2)}|${f()}|${Math.PI}`;}",
            "77|77|12",
        ),
        (
            "export default function Main(){const JSON={stringify:()=> 'own'};return JSON.stringify({});}",
            "own",
        ),
        (
            "function run(Math){return Math.max(1,2);} export default function Main(){return String(run({max:()=>91}));}",
            "91",
        ),
        (
            "export default function Main(){const harness={now:()=>17,store:{get:()=>19}};return `${harness.now()}|${harness.store.get()}`;}",
            "17|19",
        ),
        (
            "export default function Main(){const A=Array,N=Number,M=Map,f=Math.max;const m=new M();m.set('x',3);return `${A.of(N('4'))[0]}|${m.get('x')}|${f(2,5)}|${Math['PI']===Math.PI}|${f===Math.max}`;}",
            "4|3|5|true|true",
        ),
        (
            "export default function Main(){const Object={assign:()=>({x:8})};return String(Object.assign({}, {x:1}).x);}",
            "8",
        ),
    ] {
        let view = f.app(source);
        assert_eq!(render(&view, json!({})).unwrap(), expected, "{source}");
    }
    std::fs::write(f.0.join("math.tsx"), "export const Math={max:()=>81, PI:16};").unwrap();
    let view =
        f.app("import {Math} from './math.tsx'; export default function Main(){return `${Math.max()}|${Math.PI}`;}");
    assert_eq!(render(&view, json!({})).unwrap(), "81|16");
}

#[test]
fn json_replacer_observes_mutations_without_holding_array_borrows() {
    let f = Files::new();
    let view = f.app("export default function Main(){const a=[1,2,3];return JSON.stringify(a,(k,v)=>{if(k==='0')a.pop();return v;});}");
    assert_eq!(render(&view, json!({})).unwrap(), "[1,2,null]");
}

#[test]
fn lazy_hook_initializers_run_only_when_the_hook_mounts() {
    let f = Files::new();
    let view = f.app("export default function Main(){const [x,setX]=useState(()=>{if(fail)throw new Error('initializer repeated');return 42;},'x');return String(x);}");
    let mut config = cfg();
    config.props = json!({"fail": false});
    let mut agent = Agent::new(config).unwrap();
    agent.start(&view, enc()).unwrap();
    assert_eq!(agent.kept_state()["__ui"].to_wire()["@x"], json!(42));
    agent
        .handle(
            &view,
            enc(),
            affordance::runtime::Input::Props {
                props: json!({"fail": true}),
                clock_ms: 1,
            },
        )
        .unwrap();
    assert_eq!(agent.kept_state()["__ui"].to_wire()["@x"], json!(42));
}

#[test]
fn unsupported_function_schema_fields_fail_instead_of_disappearing() {
    let f = Files::new();
    let view = f.app("function tool({name,run}:{name:string;run:()=>string}){return name;} export default function Main(){return JSON.stringify(schemaOf(tool));}");
    let error = render(&view, json!({})).unwrap_err();
    assert!(error.contains("function type"), "{error}");
}

#[test]
fn dropout_tokenizers_are_rejected_at_admission() {
    let f = Files::new();
    let source = tokenizer::path();
    let mut tokenizer: J = serde_json::from_slice(&std::fs::read(source).unwrap()).unwrap();
    tokenizer["model"]["dropout"] = json!(0.5);
    let path = f.0.join("tokenizer.json");
    std::fs::write(&path, tokenizer.to_string()).unwrap();
    let error = Encoder::from_file(path.to_str().unwrap())
        .err()
        .expect("accepted dropout");
    assert!(error.contains("dropout"), "{error}");
    tokenizer["model"]["dropout"] = json!(0.0);
    std::fs::write(&path, tokenizer.to_string()).unwrap();
    let deterministic = Encoder::from_file(path.to_str().unwrap()).unwrap();
    assert_eq!(
        deterministic.encode_run("hello 世界 123", true, true),
        enc().encode_run("hello 世界 123", true, true)
    );
}

// Crash regressions run in a child so a regression produces a failed test, not a lost suite.
fn isolated(name: &str) -> bool {
    if std::env::var("AFFORDANCE_HARDENING_CHILD").ok().as_deref() == Some(name) {
        return false;
    }
    let out = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", &format!("hardening::{name}"), "--nocapture"])
        .env("AFFORDANCE_HARDENING_CHILD", name)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{name}: {}\n{}\n{}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    true
}

#[test]
fn cyclic_collections_and_deep_source_return_errors_without_aborting() {
    if isolated("cyclic_collections_and_deep_source_return_errors_without_aborting") {
        return;
    }
    let f = Files::new();
    let sources = [
        format!(
            "export default function Main(){{return {}1{};}}",
            "(".repeat(10_000),
            ")".repeat(10_000)
        ),
        format!("export default function Main(){{return {}1;}}", "!".repeat(10_000)),
        format!("export default function Main(){{return 1{};}}", "+1".repeat(10_000)),
        format!("export default function Main(){{return x{};}}", ".x".repeat(10_000)),
        format!(
            "type Deep=number{};export default function Main(){{return 'ok';}}",
            "[]".repeat(10_000)
        ),
    ];
    for source in sources {
        std::fs::write(f.0.join("deep.tsx"), source).unwrap();
        assert!(View::load(&f.0.join("deep.tsx"), "Main", enc()).is_err());
    }
    let c = Rc::new(Obj::new(Coll::new(false)));
    let v = Value::Coll(c.clone());
    c.write().unwrap().insert(Value::str("self"), v.clone());
    affordance::value::freeze(&v);
    assert!(affordance::value::freeze_state(&v).is_err());
    assert!(affordance::runtime::inspect::encode(&v, false).is_err());
    let view =
        f.app("export default function Main(){const m=new Map();m.set('self',m);useState(m,'cycle');return 'ok';}");
    let mut agent = Agent::new(cfg()).unwrap();
    assert!(agent.start(&view, enc()).is_err());
    assert!(c.write().is_err());
}

#[test]
fn recursive_render_returns_range_error_on_an_ordinary_worker_stack() {
    if isolated("recursive_render_returns_range_error_on_an_ordinary_worker_stack") {
        return;
    }
    let f = Files::new();
    std::fs::write(f.0.join("app.tsx"), "function walk(o){return Object.keys(o).map(k=>walk(o[k])).join('');} export default function Main(){const o={};o.self=o;try{return walk(o);}catch(e){return e.name;}}").unwrap();
    let path = f.0.join("app.tsx");
    let result = std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || {
            let view = View::load(&path, "Main", enc()).unwrap();
            render(&view, json!({})).unwrap()
        })
        .unwrap()
        .join()
        .unwrap();
    assert_eq!(result, "RangeError");
}

#[test]
fn native_conversions_finish_on_an_ordinary_worker_stack() {
    if isolated("native_conversions_finish_on_an_ordinary_worker_stack") {
        return;
    }
    let f = Files::new();
    std::fs::write(f.0.join("app.tsx"), "export async function Main(){const a=[];a.push(a);const o={};o.self=o;let deep=0;for(let i=0;i<5000;i++)deep=[deep];let promise=Promise.resolve(7);for(let i=0;i<12000;i++)promise=Promise.race([promise]);const value=await promise;return JSON.stringify([String(a),a.join(','),formatPythonRepr(a),formatPythonRepr(o),String(deep),String(structuredClone(deep)),value]);}").unwrap();
    let path = f.0.join("app.tsx");
    let result = std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || {
            let view = View::load(&path, "Main", enc()).unwrap();
            render(&view, json!({})).unwrap()
        })
        .unwrap()
        .join()
        .unwrap();
    assert_eq!(
        serde_json::from_str::<J>(&result).unwrap(),
        json!(["", "", "[[...]]", "{'self': {...}}", "0", "0", 7])
    );
}

#[test]
fn native_flattening_handles_deep_values_and_rejects_infinite_cycles() {
    if isolated("native_flattening_handles_deep_values_and_rejects_infinite_cycles") {
        return;
    }
    let f = Files::new();
    std::fs::write(f.0.join("app.tsx"),"export default function Main(){let deep=7;for(let i=0;i<5000;i++)deep=[deep];const a=[];a.push(a);let error;try{a.flat(Infinity);}catch(e){error=e.name;}return JSON.stringify([deep.flat(Infinity),error]);}").unwrap();
    let path = f.0.join("app.tsx");
    let result = std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || {
            let view = View::load(&path, "Main", enc()).unwrap();
            render(&view, json!({})).unwrap()
        })
        .unwrap()
        .join()
        .unwrap();
    assert_eq!(serde_json::from_str::<J>(&result).unwrap(), json!([[7], "RangeError"]));
}

/// Render `source`'s `Main` on a thread with the render worker's 2 MiB stack.
fn render_on_worker_stack(source: &str) -> Result<String, String> {
    let f = Files::new();
    std::fs::write(f.0.join("app.tsx"), source).unwrap();
    let path = f.0.join("app.tsx");
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || render(&View::load(&path, "Main", enc()).unwrap(), json!({})))
        .unwrap()
        .join()
        .unwrap()
}

/// Fuel counts steps, and one step can ask for gigabytes: each bulk allocator throws V8's
/// catchable RangeError at V8's string length or the bulk array limit instead of aborting.
#[test]
fn bulk_allocations_throw_range_errors_instead_of_aborting() {
    if isolated("bulk_allocations_throw_range_errors_instead_of_aborting") {
        return;
    }
    let (string, array) = ("RangeError: Invalid string length", "RangeError: Invalid array length");
    let cases = [
        ("Array(1e9).fill(0)", array),
        ("{let s='x';for(let i=0;i<34;i++)s=s+s;}", string),
        ("{let s='x';for(let i=0;i<34;i++)s=`${s}${s}`;}", string),
        ("{const s=big();return [s,s].join('');}", string),
        ("{const s=big();return [s,s].join.call({length:2,0:s,1:s},'');}", string),
        ("{const s=big();return s.concat(s);}", string),
        ("'xx'.replaceAll('x',big())", string),
        ("'xx'.replace(/x/g,big())", string),
        ("{const s=big();return JSON.stringify([s,s]);}", string),
        ("[...'ab'.repeat(6e6)]", array),
        ("'ab'.repeat(6e6).split('')", array),
        ("'ab'.repeat(6e6).match(/./g)", array),
        ("{const a=many();return a.concat(a);}", array),
        ("{const a=many();return [...a,...a];}", array),
        ("{const a=many();a.push(...a);}", array),
        ("{const s=big();return s+s.slice(0,-24);}", "ok"),
    ];
    // Each case copies hundreds of megabytes, so each renders on its own thread.
    std::thread::scope(|scope| {
        let runs: Vec<_> = cases
            .iter()
            .map(|(case, _)| {
                scope.spawn(move || {
                    render_on_worker_stack(&format!(
                        "export default function Main(){{const big=()=>'x'.repeat(2**28);const many=()=>Array(1e7).fill(0);
                        try{{(()=>{case})();return 'ok';}}catch(e){{return `${{e.name}}: ${{e.message}}`;}}}}"
                    ))
                    .unwrap()
                })
            })
            .collect();
        for (run, (case, expected)) in runs.into_iter().zip(cases) {
            assert_eq!(run.join().unwrap(), expected, "{case}");
        }
    });
}

/// Values nest deeper than a native walk can recurse: dropping, parsing and splicing them as
/// children finish on the worker stack, and a child list inside itself is a TypeError.
#[test]
fn deep_and_cyclic_values_are_errors_not_stack_overflows() {
    if isolated("deep_and_cyclic_values_are_errors_not_stack_overflows") {
        return;
    }
    let result = render_on_worker_stack("export default function Main(){const out=[];const t=f=>{try{out.push(f());}catch(e){out.push(`${e.name}: ${e.message}`);}};
        t(()=>{let a=[];for(let i=0;i<100000;i++)a=[a];return 'dropped';});
        t(()=>{const o={};let p=o;for(let i=0;i<100000;i++){p.next={};p=p.next;}return 'dropped';});
        function at(n){return n===0?JSON.parse('['.repeat(4999)+']'.repeat(4999)).length:at(n-1);}
        t(()=>{let d=0;try{for(;;d++)at(d);}catch(e){return `${d} ${e.name}`;}});
        t(()=>{const a=[];a.push(a);return <>{a}</>;});
        t(()=>{let a='leaf';for(let i=0;i<20000;i++)a=[a];return <>{a}</>;});
        t(()=>{let a='leaf';for(let i=0;i<300;i++)a=[a];const x=<>{a}</>;return 'rendered';});
        return JSON.stringify(out);}").unwrap();
    let out: Vec<String> = serde_json::from_str(&result).unwrap();
    assert_eq!(out[..2], ["dropped", "dropped"]);
    // Every call depth up to the recursion limit parses; the limit itself is the RangeError.
    let (depth, name) = out[2].split_once(' ').unwrap();
    assert!(
        depth.parse::<usize>().unwrap() > 390 && name == "RangeError",
        "{}",
        out[2]
    );
    assert_eq!(
        out[3],
        "TypeError: a child list contains itself (a circular structure cannot render)"
    );
    assert_eq!(out[4], "RangeError: child lists nest more than 400 levels");
    assert_eq!(out[5], "rendered");
    for (prop, why) in [
        ("const a=[];a.push(a);", "a value that contains itself"),
        (
            "let a=1;for(let i=0;i<20000;i++)a=[a];",
            "nested more than 128 levels deep",
        ),
    ] {
        let error = render_on_worker_stack(&format!("function Kid(){{return 'kid';}} export default function Main(){{{prop}return <Mount key='k'><Kid data={{a}}/></Mount>;}}")).unwrap_err();
        assert!(error.contains(&format!("prop data of <Kid> is {why}")), "{error}");
    }
}

#[test]
fn native_functions_are_state_updaters_and_private_methods_survive_snapshots() {
    let f = Files::new();
    let view=f.app("class Box{#x=9;read(){return this.#x;}write(v){this.#x=v;}}export default function Main(){const [count,setCount]=useState('7','count');const [box]=useState(new Box(),'box');const [out,setOut]=useState('','out');useEvent('update',()=>{setCount(Number);try{box.write(11);}catch(e){setOut(`${box.read()}:${e.name}`);}});return String(count);}");
    let mut a = Agent::new(cfg()).unwrap();
    a.start(&view, enc()).unwrap();
    a.handle(
        &view,
        enc(),
        affordance::runtime::Input::Event {
            name: "update".into(),
            payload: json!({}),
            op: None,
            clock_ms: 1,
        },
    )
    .unwrap();
    assert_eq!(a.state()["__ui"]["@count"], 7);
    assert_eq!(a.state()["__ui"]["@out"], "9:TypeError");
}

#[test]
fn docker_names_and_images_are_arguments_never_host_shell_source() {
    const NAME: &str = "docker_names_and_images_are_arguments_never_host_shell_source";
    if std::env::var("AFFORDANCE_HARDENING_CHILD").ok().as_deref() != Some(NAME) {
        use std::os::unix::fs::PermissionsExt;
        let f = Files::new();
        let stub = f.0.join("docker");
        std::fs::write(
            &stub,
            "#!/bin/sh\nprintf '%s\\n' \"$@\" >> \"$AFFORDANCE_DOCKER_ARGS\"\nexit 125\n",
        )
        .unwrap();
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
        let out = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", &format!("hardening::{NAME}"), "--nocapture"])
            .env("AFFORDANCE_HARDENING_CHILD", NAME)
            .env(
                "PATH",
                format!("{}:{}", f.0.display(), std::env::var("PATH").unwrap_or_default()),
            )
            .env("AFFORDANCE_DOCKER_ARGS", f.0.join("args"))
            .env("AFFORDANCE_DOCKER_MARKER", f.0.join("marker"))
            .output()
            .unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        assert!(!f.0.join("marker").exists());
        let args = std::fs::read_to_string(f.0.join("args")).unwrap();
        assert!(args.contains("aff-i-root_2fchild"), "{args}");
        assert!(
            args.contains("unused$(touch${IFS}"),
            "image was not passed literally: {args}"
        );
        return;
    }
    let marker = std::env::var("AFFORDANCE_DOCKER_MARKER").unwrap();
    let ctx = CallContext {
        agent: format!("root/child$(touch${{IFS}}{marker})"),
        task: 0,
        env: BTreeMap::new(),
        cancel: CancelToken::new(),
        blobs: None,
        output: None,
    };
    let executor = Exec {
        target: Target::Docker {
            image: format!("unused$(touch${{IFS}}{marker})"),
            name: "aff-{agent}".into(),
        },
        default_timeout_s: 1,
        max_capture: 1000,
    };
    let _ = executor.call(&ctx, &json!({"cmd":"true"}));
    assert!(!std::path::Path::new(&marker).exists());
}

#[test]
fn encoded_instance_components_are_injective_even_at_the_prefix() {
    let inputs = [
        "",
        "_",
        "i_",
        "-a",
        "i-a",
        "root/child",
        "root_child",
        "root_2fchild",
        "é",
        "_c3_a9",
    ];
    let encoded: std::collections::HashSet<_> = inputs
        .iter()
        .map(|id| affordance::runtime::exec::instance_component(id))
        .collect();
    assert_eq!(encoded.len(), inputs.len());
    for component in encoded {
        assert!(component.as_bytes()[0].is_ascii_alphanumeric());
        assert!(
            component
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
        );
    }
}

#[test]
fn collection_keys_distinguish_intrinsics_and_setters_from_strings() {
    let mut map = Coll::new(false);
    for value in [
        Value::Native(Rc::<str>::from("Math.max").into()),
        Value::str("\0native:Math.max"),
        Value::Setter(Rc::from("@x")),
        Value::str("\0setter:@x"),
    ] {
        map.insert(value, Value::Int(1));
    }
    assert_eq!(map.len(), 4);
}

#[test]
fn shared_subtrees_are_checked_at_their_deepest_state_path() {
    let leaf = Value::list(vec![]);
    let mut deep = leaf.clone();
    for _ in 0..127 {
        deep = Value::list(vec![deep]);
    }
    // The shallow leaf is visited first; the cached subtree height still has to count on
    // the deeper path. Its graph is acyclic, but exceeds the admitted state depth.
    let root = Value::list(vec![deep, leaf]);
    assert!(!affordance::value::cyclic(&root));
    let error = affordance::value::freeze_state(&root).unwrap_err();
    assert!(error.contains("nesting"), "{error}");
}

#[test]
fn complete_invalid_store_records_are_not_treated_as_torn_tails() {
    for record in [
        json!({"key":1,"text":"x","next":0}),
        json!({"key":"x","text":null,"next":0}),
        json!({"key":"x","text":"y"}),
        json!({"key":"x","text":"y","next":-1}),
    ] {
        let f = Files::new();
        std::fs::create_dir_all(f.0.join("a")).unwrap();
        let path = f.0.join("a/store.jsonl");
        let bytes = record.to_string();
        std::fs::write(&path, &bytes).unwrap();
        let store = affordance::runtime::store::Store::new(Some(f.0.clone()));
        assert!(store.load("a").is_err(), "{record}");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), bytes);
        assert_eq!(store.call("a", "store.list", &json!({})).unwrap()["keys"], json!([]));
    }
}

#[test]
fn persisted_auto_keys_survive_reload_and_explicit_collisions() {
    let f = Files::new();
    let put = |store: &affordance::runtime::store::Store, args: J| store.call("a", "store.put", &args).unwrap();
    let first = affordance::runtime::store::Store::new(Some(f.0.clone()));
    put(&first, json!({"key":"b1","text":"first"}));
    assert_eq!(put(&first, json!({"text":"second"}))["key"], "b2");
    let resumed = affordance::runtime::store::Store::new(Some(f.0.clone()));
    resumed.load("a").unwrap();
    resumed.load("a").unwrap();
    assert_eq!(
        resumed.call("a", "store.get", &json!({"key":"b1"})).unwrap()["text"],
        "first"
    );
    assert_eq!(put(&resumed, json!({"text":"third"}))["key"], "b3");
}

#[test]
fn a_backwards_store_counter_is_rejected_before_completing_its_line() {
    let f = Files::new();
    std::fs::create_dir_all(f.0.join("a")).unwrap();
    let path = f.0.join("a/store.jsonl");
    let bytes = b"{\"key\":\"saved\",\"text\":\"first\",\"next\":2}\n{\"key\":\"bad\",\"text\":\"second\",\"next\":1}";
    std::fs::write(&path, bytes).unwrap();
    let store = affordance::runtime::store::Store::new(Some(f.0.clone()));
    assert!(store.load("a").unwrap_err().contains("backwards"));
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert_eq!(store.call("a", "store.list", &json!({})).unwrap()["keys"], json!([]));
}
