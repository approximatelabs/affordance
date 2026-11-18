//! Closures are reference counted, so a closure that held itself would never be freed: one
//! leaked per render, per agent, for the run's life. A closure captures only the names that are
//! free in it (not its own parameters and declarations), and a `const` function that calls
//! itself is bound to itself when called, not through its variable. Objects are references, so
//! they make cycles too; each run's end sweeps the cycles it left (value::sweep). These tests
//! count live heap bytes across renders, and pin that the values still behave as JavaScript's do.
#[path = "support/tokenizer.rs"]
mod tokenizer;
use affordance::View;
use affordance::emit::{Encoder, RunCache};
use serde_json::json;
use std::sync::atomic::{AtomicIsize, Ordering};

struct Counting;
static LIVE: AtomicIsize = AtomicIsize::new(0);
unsafe impl std::alloc::GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: std::alloc::Layout) -> *mut u8 {
        LIVE.fetch_add(l.size() as isize, Ordering::Relaxed);
        unsafe { std::alloc::System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: std::alloc::Layout) {
        LIVE.fetch_sub(l.size() as isize, Ordering::Relaxed);
        unsafe { std::alloc::System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: std::alloc::Layout, n: usize) -> *mut u8 {
        LIVE.fetch_add(n as isize - l.size() as isize, Ordering::Relaxed);
        unsafe { std::alloc::System.realloc(p, l, n) }
    }
}
#[global_allocator]
static GA: Counting = Counting;

fn now() -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 10, 19)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap()
}

fn view(enc: &Encoder, source: &str) -> View {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "aff-closures-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("test.tsx"), source).unwrap();
    View::load(&dir.join("test.tsx"), "Main", enc).unwrap()
}

fn text(enc: &Encoder, v: &View) -> String {
    let state = affordance::value::state(&json!({}));
    let r = v.render(enc, state, &mut RunCache::new(false), now()).unwrap();
    enc.tok.decode(&r.ids, false).unwrap()
}

// One test, so no other test allocates while it counts.
#[test]
fn renders_free_their_closures_and_closures_behave_as_in_javascript() {
    let enc = tokenizer::encoder();
    let cases = [
        // A const function that calls itself.
        (
            "recursive",
            r#"export function Main() { const fact = (n) => n <= 1 ? 1 : n * fact(n - 1); return `${fact(5)}`; }"#,
            "120",
        ),
        // A parameter named like an outer variable whose value holds a closure that calls this one
        // (the shape of session.tsx's `screen` and `act`).
        (
            "shadowed",
            r#"export function Main() {
            const show = (act) => `${act.name}`;
            const act = {name: "a", again: () => show({name: "b"})};
            return `${show(act)}${act.again()}`;
        }"#,
            "ab",
        ),
        // A local declared at the top of a closure's body, named like an outer variable.
        (
            "local",
            r#"export function Main() {
            const x = 1;
            const f = () => { const x = 2; const g = () => x; return g(); };
            return `${x}${f()}`;
        }"#,
            "12",
        ),
        // A recursive closure that escapes the render that made it, and is called later.
        (
            "escapes",
            r#"export function Main() {
            const make = () => { const count = (n) => n == 0 ? "done" : count(n - 1); return count; };
            const c = make();
            return c(3);
        }"#,
            "done",
        ),
        // Cycles that objects by reference make: an object written into itself, an object whose
        // method refers to it, an array that holds itself, two closures that call each other.
        (
            "self-object",
            r#"export function Main() { const o = {n: 1}; o.self = o; return `${o.self.self.n}`; }"#,
            "1",
        ),
        (
            "method-refers-to-object",
            r#"export function Main() { const api = {n: 2, get: () => api.n}; api.twice = () => api.get() * 2; return `${api.twice()}`; }"#,
            "4",
        ),
        (
            "self-array",
            r#"export function Main() { const xs = [1]; xs.push(xs); return `${xs[1][1][0]}`; }"#,
            "1",
        ),
        (
            "mutual-closures",
            r#"export function Main() { let even, odd; even = (n) => n == 0 ? true : odd(n - 1); odd = (n) => n == 0 ? false : even(n - 1); return `${even(10)}`; }"#,
            "true",
        ),
        (
            "self-map",
            "export function Main(){const m=new Map();m.set('self',m);return 'ok';}",
            "ok",
        ),
        (
            "frozen-cycle",
            "export function Main(){const a=[];a.push(a);Object.freeze(a);return 'ok';}",
            "ok",
        ),
        (
            "cloned-cycle",
            "export function Main(){const a=[];a.push(a);structuredClone(a);return 'ok';}",
            "ok",
        ),
        (
            "self-set",
            "export function Main(){const s=new Set();s.add(s);return 'ok';}",
            "ok",
        ),
        (
            "bound-receiver",
            "export function Main(){const o={m(){return 1;}};o.bound=o.m.bind(o);return 'ok';}",
            "ok",
        ),
        (
            "private-cycle",
            "export function Main(){class A{#x;constructor(){this.#x=this;}}new A();return 'ok';}",
            "ok",
        ),
        (
            "class-cycle",
            "export function Main(){class A{static self(){return A;}}A.self();return 'ok';}",
            "ok",
        ),
        (
            "collection-closure-cycle",
            "export function Main(){const m=new Map();m.set('f',()=>m);return 'ok';}",
            "ok",
        ),
        (
            "promise-cycle",
            "export function Main(){const a=[];a.push(Promise.resolve(a));return 'ok';}",
            "ok",
        ),
        (
            "promise-completion-cycle",
            "export async function Main(){const a=[];const p=Promise.all([a]);a.push(p);await p;return 'ok';}",
            "ok",
        ),
        (
            "promise-rejection-cycle",
            "export async function Main(){const a=[];const p=Promise.reject(a);a.push(p);try{await p;}catch(e){}return 'ok';}",
            "ok",
        ),
        // Loaded code owns its lexical environment, including genuine constant/class cycles.
        (
            "source-functions",
            "import {loadModule} from 'affordance:compiler'; export function Main(){const m=loadModule(\"const x='ok';export default function Read(){return x;}\");return <m.default/>;}",
            "ok",
        ),
        (
            "source-closure",
            "import {loadModule} from 'affordance:compiler'; export function Main(){const m=loadModule(\"const read=()=>read;export function value(){return read()===read?'ok':'bad';}\");return m.value();}",
            "ok",
        ),
        (
            "source-class",
            "import {loadModule} from 'affordance:compiler'; export function Main(){const m=loadModule(\"export class Box{static value(){return Box;}}\");return m.Box.value()===m.Box?'ok':'bad';}",
            "ok",
        ),
        // Closures still share the variables they capture (a later assignment is seen).
        (
            "shared",
            r#"export function Main() {
            let n = 1;
            const read = () => n;
            n = 2;
            return `${read()}`;
        }"#,
            "2",
        ),
    ];
    for (name, source, expect) in cases {
        let v = view(&enc, source);
        assert_eq!(text(&enc, &v), expect, "{name}");
        for _ in 0..20 {
            text(&enc, &v);
        }
        let before = LIVE.load(Ordering::Relaxed);
        for _ in 0..500 {
            text(&enc, &v);
        }
        let grown = LIVE.load(Ordering::Relaxed) - before;
        assert!(
            grown < 4096,
            "{name}: 500 renders kept {grown} bytes (a closure that holds itself is never freed)"
        );
    }
    // A source editor need not call a model. Its recent scan caches must still release revisions.
    use affordance::runtime::{Agent, AgentConfig, Input};
    let v = view(
        &enc,
        r#"import {loadModule} from 'affordance:compiler'; export function Main(){
      const [current,setCurrent]=useState(null);
      useEvent('install',({source})=>setCurrent(loadModule(source)));
      return current && <current.default/>;}"#,
    );
    let cfg = AgentConfig {
        context: None,
        retry: Default::default(),
        template: None,
        id: "a".into(),
        props: json!({}),
        env: Default::default(),
        max_new: 20,
        max_turns: None,
        max_prompt: None,
        start_ms: 0,
        seed: 7,
        machines: vec![],
        model: None,
    };
    let mut agent = Agent::new(cfg).unwrap();
    agent.start(&v, &enc).unwrap();
    let mut install = |n: i64| {
        let source = format!(
            "const label='{:04}';export default function Panel(){{return memoScan([1],0,(s,x)=>[s+x,label],[]).outs.join('');}}",
            n
        );
        agent
            .handle(
                &v,
                &enc,
                Input::Event {
                    name: "install".into(),
                    payload: json!({"source":source}),
                    op: None,
                    clock_ms: n,
                },
            )
            .unwrap();
        agent.take_trace();
        assert_eq!(agent.metrics.handler_errors, 0);
    };
    // Warm the bounded recent-event window before measuring code retention.
    for n in 1..1101 {
        install(n);
    }
    let before = LIVE.load(Ordering::Relaxed);
    for n in 1101..1601 {
        install(n);
    }
    // A new render frees the cycles earlier runs left.
    text(&enc, &view(&enc, "export default function Main(){return 'x';}"));
    let grown = LIVE.load(Ordering::Relaxed) - before;
    assert!(
        grown < 262144,
        "500 replaced source modules retained {grown} bytes without model calls"
    );
}
