//! Independently authored file modules must keep their private state and names isolated.
use crate::tokenizer;
use affordance::{
    View,
    effects::Outcome,
    emit::{Encoder, RunCache},
    runtime::{Agent, AgentConfig, Command, Input},
};
use serde_json::json;
use std::{
    collections::BTreeMap,
    path::PathBuf,
    rc::Rc,
    sync::{
        OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
};

struct Files(PathBuf);
impl Files {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let p = std::env::temp_dir().join(format!(
            "aff-modules-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&p).unwrap();
        Self(p)
    }
    fn write(&self, name: &str, source: &str) {
        std::fs::write(self.0.join(name), source).unwrap();
    }
    fn load(&self) -> Result<View, String> {
        View::load(&self.0.join("app.tsx"), "Main", encoder())
    }
    fn render(&self) -> Result<String, String> {
        let v = self.load()?;
        let now = chrono::NaiveDate::from_ymd_opt(2026, 10, 23)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let r = v.render(encoder(), Rc::new(Default::default()), &mut RunCache::new(false), now)?;
        Ok(encoder().tok.decode(&r.ids, false).unwrap())
    }
}
impl Drop for Files {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn encoder() -> &'static Encoder {
    static ENC: OnceLock<Encoder> = OnceLock::new();
    ENC.get_or_init(|| {
        Encoder::from_file(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/ref/tokenizers/Qwen__Qwen3-8B.json"
        ))
        .unwrap()
    })
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn function_values_compose_through_props_members_and_returned_closures() {
    let f = Files::new();
    f.write(
        "app.tsx",
        r#"
function Wrapper({Panel}) { return <Panel value="prop"/>; }
function make(prefix) {
  const ui = {Panel: ({value, children}) => <span>{prefix}{value}{children}</span>};
  return () => <ui.Panel value="member">child</ui.Panel>;
}
export default function Main() {
  const Panel = ({value}) => value;
  const Returned = make("closed:");
  const Bound = (({value}) => value).bind(null);
  return <><Wrapper Panel={Panel}/>{"|"}<Returned/>{"|"}<Bound value="bound"/></>;
}
"#,
    );
    assert_eq!(f.render().unwrap(), "prop|closed:memberchild|bound");
    f.write(
        "app.tsx",
        "export default function Main(){const Panel={};return <Panel/>;}",
    );
    assert!(f.render().unwrap_err().contains("expected a function"));
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn imports_keep_functions_closures_classes_and_defaults_in_their_defining_file() {
    let f = Files::new();
    f.write(
        "left.tsx",
        r#"
const label = "left";
function helper() { return label; }
export function read() { return helper(); }
export function closure() { return () => helper(); }
export class Box { get value() { return helper(); } }
export function def(value = helper()) { return value; }
"#,
    );
    f.write(
        "right.tsx",
        r#"
const label = "right";
function helper() { return label; }
export default function read() { return helper(); }
export const suffix = "!";
"#,
    );
    f.write(
        "app.tsx",
        r#"
import { read as left, closure, Box, def } from "./left.tsx";
import right, {suffix as ending} from "./right.tsx";
const label = "app";
function helper() { return label; }
export default function Main() {
  const callback = closure();
  return [left(), right()+ending, callback(), new Box().value, def(), helper()].join(",");
}
"#,
    );
    assert_eq!(f.render().unwrap(), "left,right!,left,left,left,app");
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn side_effect_imports_and_transitive_dependencies_do_not_leak_names() {
    let f = Files::new();
    f.write(
        "private.tsx",
        "const secret = 'hidden'; export function read(){ return secret; }",
    );
    f.write(
        "library.tsx",
        "import { read } from './private.tsx'; export function value(){return read();}",
    );
    f.write("app.tsx", "import './private.tsx'; import {value} from './library.tsx'; export default function Main(){return value()+':'+typeof secret+':'+typeof read;}");
    assert_eq!(f.render().unwrap(), "hidden:undefined:undefined");
    f.write(
        "app.tsx",
        "import {secret} from './private.tsx'; export default function Main(){return secret;}",
    );
    assert!(f.load().err().unwrap().contains("has no export `secret`"));
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn diamond_imports_share_binding_identity_and_export_aliases() {
    let f = Files::new();
    f.write(
        "base.tsx",
        "class Box {} const value = {}; export {Box, value as shared};",
    );
    f.write(
        "left.tsx",
        "import {Box, shared} from './base.tsx'; export {Box, shared};",
    );
    f.write(
        "right.tsx",
        "import {Box, shared} from '././base.tsx'; export {Box as Other, shared};",
    );
    f.write("app.tsx", "import {Box, shared as a} from './left.tsx'; import {Other, shared as b} from './right.tsx'; export default function Main(){return JSON.stringify([Box===Other, a===b, new Box() instanceof Other]);}");
    assert_eq!(f.render().unwrap(), "[true,true,true]");
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn jsx_aliases_and_mounts_keep_the_declared_child_entry() {
    let f = Files::new();
    f.write(
        "child.tsx",
        "function helper(){return 'child';} export function Child(){return helper();}",
    );
    f.write("app.tsx", "import {Child as Renamed} from './child.tsx'; export default function Main(){return <><Renamed/><Mount key='child'><Renamed/></Mount></>;}");
    let v = f.load().unwrap();
    let now = chrono::NaiveDate::from_ymd_opt(2026, 10, 23)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let rendered = v
        .render(encoder(), Rc::new(Default::default()), &mut RunCache::new(false), now)
        .unwrap();
    assert_eq!(encoder().tok.decode(&rendered.ids, false).unwrap(), "child");
    let mount = rendered.spans.iter().find(|s| s.label == "mount").unwrap();
    assert_eq!(mount.meta["entry"], "Child");
    let child = View::load(
        std::path::Path::new(mount.meta["app"].as_str().unwrap()),
        "Child",
        encoder(),
    )
    .unwrap();
    assert_eq!(child.entry(), "Child");
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn schema_aliases_are_resolved_in_the_functions_own_module() {
    let f = Files::new();
    f.write(
        "left.tsx",
        "type Arg = {q:string}; export function run({q}:Arg){return q;}",
    );
    f.write(
        "right.tsx",
        "type Arg = {n:number}; export function run({n}:Arg){return n;}",
    );
    f.write("app.tsx", "import {run as left} from './left.tsx'; import {run as right} from './right.tsx'; type Arg = {wrong:boolean}; export default function Main(){return JSON.stringify([schemaOf(left),schemaOf(right)]);}");
    let schemas: serde_json::Value = serde_json::from_str(&f.render().unwrap()).unwrap();
    assert_eq!(schemas[0]["properties"]["q"]["type"], "string");
    assert_eq!(schemas[1]["properties"]["n"]["type"], "number");
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn imports_fail_explicitly_for_ambiguous_or_unsupported_graphs() {
    let f = Files::new();
    f.write("lib.tsx", "export function value(){return 1;}");
    for (source, message) in [
        ("import {missing} from './lib.tsx';", "has no export `missing`"),
        ("import value from './lib.tsx';", "has no export `default`"),
        (
            "import {value} from './lib.tsx'; const value=2;",
            "duplicate module binding",
        ),
        (
            "import {value} from './lib.tsx'; import {value} from './lib.tsx';",
            "duplicate module binding",
        ),
        ("export {missing};", "undefined binding"),
        ("export function value(){} export {value};", "duplicate export"),
        (
            "import * as lib from './lib.tsx';",
            "namespace imports are not supported",
        ),
        (
            "export {value} from './lib.tsx';",
            "re-exports with `from` are not supported",
        ),
        (
            "import {value} from './style.tss';",
            "style sheets support only side-effect imports",
        ),
    ] {
        f.write(
            "app.tsx",
            &format!("{source} export default function Main(){{return '';}}"),
        );
        let e = f.load().err().expect(source);
        assert!(e.contains(message), "{source}: {e}");
    }
    f.write("lib.tsx", "import './app.tsx'; export function value(){}");
    f.write("app.tsx", "import './lib.tsx'; export default function Main(){}");
    assert!(
        f.load()
            .err()
            .unwrap()
            .contains("cyclic module imports are not supported")
    );
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn module_constants_have_a_temporal_dead_zone_and_imports_are_read_only() {
    let f = Files::new();
    f.write(
        "app.tsx",
        "import type {Missing} from './missing.tsx'; export default function Main(){return 'erased';}",
    );
    assert_eq!(f.render().unwrap(), "erased");
    f.write(
        "app.tsx",
        "const a=b; const b=2; export default function Main(){return a;}",
    );
    assert!(f.render().err().unwrap().contains("ReferenceError"));
    f.write("lib.tsx", "export const item = {n:1};");
    f.write(
        "app.tsx",
        "import {item} from './lib.tsx'; export default function Main(){item=3;return item;}",
    );
    assert!(f.render().err().unwrap().contains("read-only module binding"));
    f.write(
        "app.tsx",
        "import {item} from './lib.tsx'; export default function Main(){item.n=2;return item.n;}",
    );
    assert!(f.render().err().unwrap().contains("TypeError"));
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn standard_library_internals_are_isolated_and_public_globals_can_be_shadowed() {
    let f = Files::new();
    f.write("app.tsx", "function headerName(){return 'broken';} function fetch(){return 'local';} export default function Main(){return new Headers({HELLO:'world'}).get('hello')+':'+fetch()+':'+typeof fetchBound;}");
    assert_eq!(f.render().unwrap(), "world:local:undefined");
    f.write("app.tsx", "import {fetch as network, createFetch} from 'affordance:web'; function fetch(){return 'local';} export default function Main(){return [typeof network,typeof createFetch,fetch()].join(':');}");
    assert_eq!(f.render().unwrap(), "function:function:local");
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn imported_async_closures_keep_their_scope_across_effect_replay() {
    let f = Files::new();
    f.write("lib.tsx", "const label='library'; function helper(){return label;} export function make(){return async()=>{const first=await harness.machine.call({cmd:helper()}); return helper()+':'+first.output;};}");
    f.write("app.tsx", "import {make} from './lib.tsx'; function helper(){return 'app';} export default function Main(){useEffect(()=>{(async()=>{harness.stop(await make()());})();},[]);return 'waiting';}");
    let view = f.load().unwrap();
    let cfg = AgentConfig {
        context: None,
        retry: Default::default(),
        template: None,
        id: "a".into(),
        props: json!({}),
        env: BTreeMap::new(),
        max_new: 20,
        max_turns: None,
        max_prompt: None,
        start_ms: 0,
        seed: 7,
        machines: vec!["local".into()],
        model: None,
    };
    let mut agent = Agent::new(cfg.clone()).unwrap();
    let commands = agent.start(&view, encoder()).unwrap();
    let task = match commands.iter().find(|c| matches!(c, Command::Call { .. })).unwrap() {
        Command::Call { task, args, .. } => {
            assert_eq!(args["cmd"], "library");
            *task
        }
        _ => unreachable!(),
    };
    let input = Input::Result {
        task,
        outcome: Outcome::Ok(json!({"output":"result"})),
        clock_ms: 1,
    };
    agent.handle(&view, encoder(), input.clone()).unwrap();
    assert_eq!(agent.stopped().unwrap(), "library:result");
    let restored_view = f.load().unwrap();
    let mut replay = Agent::new(cfg).unwrap();
    replay.start(&restored_view, encoder()).unwrap();
    replay.handle(&restored_view, encoder(), input).unwrap();
    assert_eq!(replay.stopped(), agent.stopped());
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn source_modules_keep_private_bindings_imports_types_and_distinct_identity() {
    let f = Files::new();
    f.write(
        "app.tsx",
        r#"
import {loadModule} from 'affordance:compiler';
const label = 'caller';
export default function Main() {
  const source = `import {prefix} from './dep.tsx';
    const label = 'loaded';
    type Args = {q:string};
    function helper(){return prefix+label;}
    export function run({q}:Args){return helper()+q;}
    export function make(){return ()=>helper();}
    export function def(value=helper()){return value;}
    export class Box {get value(){return helper();}}
    export default function Panel({children}){return <span>{helper()}{children}</span>;}`;
  const dep = {prefix:'dep:'};
  const a = loadModule(source, {imports:{'./dep.tsx':dep}});
  const b = loadModule(source, {imports:{'./dep.tsx':dep}});
  const Box=a.Box;
  return <>{JSON.stringify([a.run({q:'!'}), a.make()(), a.def(), new Box().value,
    schemaOf(a.run), a.run===a.run, a.run===b.run, Object.isFrozen(a)])}<a.default>child</a.default></>;
}
"#,
    );
    let text = f.render().unwrap();
    let (values, component) = text.rsplit_once(']').unwrap();
    let values: serde_json::Value = serde_json::from_str(&format!("{values}]")).unwrap();
    assert_eq!(values[0], "dep:loaded!");
    for i in 1..4 {
        assert_eq!(values[i], "dep:loaded");
    }
    assert_eq!(values[4]["properties"]["q"]["type"], "string");
    assert_eq!(values[5], true);
    assert_eq!(values[6], false);
    assert_eq!(values[7], true);
    assert_eq!(component, "dep:loadedchild");
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn source_module_errors_are_catchable_and_initializers_cannot_use_the_caller() {
    let f = Files::new();
    let cases = [
        ("export default function (", "expected identifier"),
        ("import {read} from './absent.tsx';", "missing export namespace"),
        ("import './style.tss';", "style-file imports"),
        ("import 'affordance:absent';", "unknown built-in module"),
        ("export const a=b; const b=1;", "before initialization"),
        ("export const v=secret;", "secret is not defined"),
        ("export const v=useState(1);", "hooks belong"),
        ("export const v=<aff {...{onRun:()=>1}}>bad</aff>;", "interactive JSX"),
        (
            "export const v=<box style={{overflow:'scroll',maxTokens:2}}>text</box>;",
            "scroll viewports",
        ),
        ("async function f(){return 1;} export const v=f();", "cannot call async"),
        ("export const v=await 1;", "`await` is only valid in an async function"),
        (
            "export const v=Promise.resolve(1).then(()=>harness.stop('no'));",
            "Promise reactions belong",
        ),
        (
            "export const v=harness.machine.call({cmd:'no'});",
            "only run in a handler",
        ),
    ];
    for (source, message) in cases {
        f.write("app.tsx", &format!("import {{loadModule}} from 'affordance:compiler'; export default function Main(){{const secret=42;try{{loadModule({});return 'missing error';}}catch(e){{return e.message;}}}}",serde_json::to_string(source).unwrap()));
        let error = f.render().unwrap();
        assert!(error.contains(message), "{source}: {error}");
    }
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn replaced_source_keeps_its_imports_and_helpers_for_pending_work_and_replay() {
    let f = Files::new();
    f.write(
        "app.tsx",
        r#"
import {loadModule} from 'affordance:compiler';
function decorate(value){return 'dependency:'+value;}
export default function Main(){
  const [current,setCurrent]=useState(null);
  const [result,setResult]=useState('waiting');
  useEvent('install',({source})=>setCurrent(loadModule(source,{imports:{'./dep.tsx':{decorate}}})));
  useEvent('start',async()=>setResult(await current.run()));
  return <>{current && <current.default/>}{'|'}{result}</>;
}"#,
    );
    let module = |label: &str| {
        format!(
            r#"
import {{decorate}} from './dep.tsx';
import {{Headers}} from 'affordance:web';
const label={};
function helper(){{return new Headers({{LABEL:label}}).get('label');}}
export async function run(){{
  const first=await harness.machine.call({{cmd:helper()}});
  const second=await harness.machine.call({{cmd:helper()}});
  return decorate(helper()+':'+first.output+':'+second.output);
}}
export default function Panel(){{return helper();}}
"#,
            json!(label)
        )
    };
    let view = f.load().unwrap();
    let cfg = AgentConfig {
        context: None,
        retry: Default::default(),
        template: None,
        id: "a".into(),
        props: json!({}),
        env: BTreeMap::new(),
        max_new: 20,
        max_turns: None,
        max_prompt: None,
        start_ms: 0,
        seed: 7,
        machines: vec!["local".into()],
        model: None,
    };
    let mut agent = Agent::new(cfg.clone()).unwrap();
    agent.start(&view, encoder()).unwrap();
    let mut journal = vec![];
    let mut input = |agent: &mut Agent, value: Input| {
        journal.push(value.clone());
        agent.handle(&view, encoder(), value).unwrap()
    };
    input(
        &mut agent,
        Input::Event {
            name: "install".into(),
            payload: json!({"source":module("old")}),
            id: None,
            clock_ms: 1,
        },
    );
    let first = input(
        &mut agent,
        Input::Event {
            name: "start".into(),
            payload: json!({}),
            id: None,
            clock_ms: 2,
        },
    );
    let task = |commands: &[Command]| {
        commands
            .iter()
            .find_map(|c| match c {
                Command::Call { task, args, .. } => {
                    assert_eq!(args["cmd"], "old");
                    Some(*task)
                }
                _ => None,
            })
            .expect("pending old-source call")
    };
    let first_task = task(&first);
    input(
        &mut agent,
        Input::Event {
            name: "install".into(),
            payload: json!({"source":module("new")}),
            id: None,
            clock_ms: 3,
        },
    );
    let second = input(
        &mut agent,
        Input::Result {
            task: first_task,
            outcome: Outcome::Ok(json!({"output":"first"})),
            clock_ms: 4,
        },
    );
    input(
        &mut agent,
        Input::Result {
            task: task(&second),
            outcome: Outcome::Ok(json!({"output":"second"})),
            clock_ms: 5,
        },
    );
    let render = |view: &View, agent: &Agent| {
        let r = view
            .render_with_harness(
                encoder(),
                agent.kept_state().clone(),
                &mut RunCache::new(false),
                chrono::DateTime::from_timestamp_millis(5).unwrap().naive_utc(),
                affordance::value::Value::from_json(&json!({"owner":"agent"})),
            )
            .unwrap();
        encoder().tok.decode(&r.ids, false).unwrap()
    };
    assert_eq!(render(&view, &agent), "new|dependency:old:first:second");
    let trace = agent.take_trace();
    assert_eq!(agent.metrics.handler_errors, 0, "{trace:?}");
    let fresh = f.load().unwrap();
    let (restored, _) = Agent::restore(cfg, journal, &fresh, encoder()).unwrap();
    assert_eq!(render(&fresh, &restored), render(&view, &agent));
    assert_eq!(restored.metrics.handler_errors, 0);
}

/// An app the runtime serves imports only from its own directory and the library: not `..` out
/// of it, not an absolute path elsewhere, not through a symlink pointing out (the ticket's attack:
/// an inline app reaching a fleet's shared directory).
#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn a_served_app_imports_only_from_its_directory_and_the_library() {
    let enc = encoder();
    let base = std::env::temp_dir().join(format!("aff-contain-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let (app, lib, outside) = (base.join("app"), base.join("lib"), base.join("shared"));
    for d in [&app, &lib, &outside] {
        std::fs::create_dir_all(d).unwrap();
    }
    std::fs::write(outside.join("x.tsx"), "export const x = \"planted\";\n").unwrap();
    std::fs::write(lib.join("l.tsx"), "export const l = \"library\";\n").unwrap();
    std::fs::write(app.join("part.tsx"), "export const p = \"part\";\n").unwrap();
    std::os::unix::fs::symlink(outside.join("x.tsx"), app.join("link.tsx")).unwrap();
    let main = |import: &str| {
        std::fs::write(
            app.join("main.tsx"),
            format!("import {{ {import};\nexport default function Main() {{ return \"ok\"; }}\n"),
        )
        .unwrap();
        affordance::View::load_within(&app.join("main.tsx"), "Main", enc, std::slice::from_ref(&lib))
    };
    let r = main("p } from \"./part.tsx\"");
    assert!(r.is_ok(), "a sibling: {:?}", r.err());
    assert!(
        main(&format!("l }} from \"{}\"", lib.join("l.tsx").display())).is_ok(),
        "the library, by absolute path"
    );
    for bad in [
        "x } from \"../shared/x.tsx\"".to_string(),
        format!("x }} from \"{}\"", outside.join("x.tsx").display()),
        "x } from \"./link.tsx\"".to_string(),
    ] {
        let e = main(&bad).err().unwrap_or_else(|| panic!("loaded: {bad}"));
        assert!(e.contains("is outside the app"), "{bad}: {e}");
    }
    // A tool or test loading a view it trusts is not confined.
    assert!(affordance::View::load(&app.join("main.tsx"), "Main", enc).is_ok());
    std::fs::remove_dir_all(&base).unwrap();
}

/// An app outside the library that imports from a views/ checkout is refused, and the error
/// names the setting that allows it: the deployment's `library`, with the path filled in.
#[test]
fn an_import_outside_the_library_names_the_setting_to_allow_it() {
    let enc = affordance::emit::Encoder::from_file(tokenizer::path().to_str().unwrap()).unwrap();
    let dir = std::env::temp_dir().join(format!("aff-library-hint-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let views = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("views");
    std::fs::write(
        dir.join("app.tsx"),
        format!(
            "import {{ useLog }} from \"{}/lib/log.tsx\";\nexport default function Main() {{ return \"x\"; }}",
            views.display()
        ),
    )
    .unwrap();
    let error = affordance::View::load_within(&dir.join("app.tsx"), "Main", &enc, &[])
        .err()
        .unwrap();
    let expected = format!(
        "set \"library\": \"{}\" in the deployment",
        views.canonicalize().unwrap().display()
    );
    assert!(
        error.contains("is outside the app") && error.contains(&expected),
        "{error}"
    );
}
