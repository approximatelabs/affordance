//! Faithful recovery images: of an idle agent, or of one at a step boundary.
//! A step boundary is the point inside an input where the step that ended has drained and the
//! effects pass it asks for has not run: nothing executes, so the image is data, and restore runs
//! that pass. Unsupported execution stays on the journal replay path.
use super::*;
use crate::value::{Obj, Text};
use std::collections::HashSet;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
enum Node {
    Array { values: Vec<J> },
    Object { fields: Vec<(String, J)> },
    Tokens { chunks: Vec<Vec<u32>> },
}

#[derive(Serialize, Deserialize)]
struct Data {
    nodes: Vec<Node>,
    state: J,
    /// The state the input's change notes compare against (`Instance::noted`).
    noted: J,
    env: J,
    deps: Vec<(String, J)>,
}

/// Scalars are ordinary JSON where exact; tags distinguish references and non-JSON values.
/// Only containers occupy the arena, keeping token-id arrays compact.
struct Writer<'a> {
    nodes: Vec<Node>,
    ids: HashMap<usize, usize>,
    path: HashSet<usize>,
    forbidden: HashSet<usize>,
    tok: &'a Value,
}
fn id(value: &Value) -> Option<usize> {
    match value {
        Value::List(a) => Some(Rc::as_ptr(a) as usize),
        Value::Map(a) => Some(Rc::as_ptr(a) as usize),
        Value::Frag(a) => Some(Rc::as_ptr(a) as usize),
        _ => None,
    }
}
fn data_ids(values: Vec<Value>) -> HashSet<usize> {
    let mut work = values;
    let mut seen = HashSet::new();
    while let Some(value) = work.pop() {
        let Some(pointer) = id(&value) else { continue };
        if !seen.insert(pointer) {
            continue;
        }
        match value {
            Value::List(a) => work.extend(a.read().iter().filter(|v| id(v).is_some()).cloned()),
            Value::Map(o) => work.extend(o.read().values().filter(|v| id(v).is_some()).cloned()),
            _ => {}
        }
    }
    seen
}
impl Writer<'_> {
    fn known(&self, value: &Value) -> bool {
        id(value).is_none_or(|p| self.ids.contains_key(&p) || Some(p) == id(self.tok))
    }
    fn value(&mut self, value: &Value, depth: usize) -> Option<J> {
        if depth > 128 {
            return None;
        }
        if let Some(p) = id(value) {
            if Some(p) == id(self.tok) {
                return Some(json!({"platform":"tok"}));
            }
            if self.forbidden.contains(&p) {
                return None;
            }
            if let Some(index) = self.ids.get(&p) {
                return Some(json!({"ref":index}));
            }
            if !self.path.insert(p) {
                return None;
            }
            let node = match value {
                Value::List(a) if a.ext().is_none() && a.is_deep() && !a.is_derived() => Node::Array {
                    values: a
                        .read()
                        .iter()
                        .map(|v| self.value(v, depth + 1))
                        .collect::<Option<_>>()?,
                },
                Value::Map(o) if o.ext().is_none() && o.is_deep() && !o.is_derived() => Node::Object {
                    fields: o
                        .read()
                        .iter()
                        .map(|(k, v)| Some((k.to_string(), self.value(v, depth + 1)?)))
                        .collect::<Option<_>>()?,
                },
                Value::Frag(items) => Node::Tokens {
                    chunks: items
                        .iter()
                        .map(|item| match item {
                            crate::emit::Item::Tokens(ids, spans) if spans.is_empty() => Some(ids.to_vec()),
                            _ => None,
                        })
                        .collect::<Option<_>>()?,
                },
                _ => return None,
            };
            self.path.remove(&p);
            let index = self.nodes.len();
            self.nodes.push(node);
            self.ids.insert(p, index);
            return Some(json!({"ref":index}));
        }
        Some(match value {
            Value::None => J::Null,
            Value::Undef => json!({"undefined":true}),
            Value::Bool(v) => json!(v),
            Value::Int(v) => json!(v),
            Value::Float(v) => json!({"float_bits":v.to_bits()}),
            Value::BigInt(v) => json!({"integer":&**v}),
            Value::Str(Text::Heap(v)) => json!(&**v),
            Value::Str(Text::Host(v)) => json!({"blob":v.json()}),
            Value::Token(n, s) => json!({"token":[n,&**s]}),
            _ => return None,
        })
    }
    fn dependency(&mut self, value: &Value) -> Option<J> {
        match value {
            Value::List(values) if values.read().iter().all(|v| self.known(v)) => {}
            Value::List(_) => return None,
            _ if !self.known(value) => return None,
            _ => {}
        }
        self.value(value, 0)
    }
}
fn invalid() -> String {
    "invalid checkpoint data graph".into()
}
fn read_atom(atom: &J, values: &[Value], tok: &Value) -> Result<Value, String> {
    Ok(match atom {
        J::Null => Value::None,
        J::Bool(v) => Value::Bool(*v),
        J::String(v) => Value::str(v),
        J::Number(n) => Value::Int(n.as_i64().ok_or_else(invalid)?),
        J::Object(o) if o.len() == 1 => {
            let (kind, v) = o.iter().next().unwrap();
            match kind.as_str() {
                "ref" => values
                    .get(v.as_u64().and_then(|n| usize::try_from(n).ok()).ok_or_else(invalid)?)
                    .cloned()
                    .ok_or_else(invalid)?,
                "undefined" if v == true => Value::Undef,
                "float_bits" => Value::Float(f64::from_bits(v.as_u64().ok_or_else(invalid)?)),
                "integer" => {
                    let s = v.as_str().ok_or_else(invalid)?;
                    if !crate::value::is_int_lexeme(s)
                        || serde_json::from_str::<J>(s).ok().is_none_or(|v| !v.is_number())
                    {
                        return Err(invalid());
                    }
                    Value::BigInt(Rc::from(s))
                }
                "blob" => {
                    let text = Value::from_json(v);
                    if !matches!(text, Value::Str(Text::Host(_))) {
                        return Err("checkpoint blob cannot be resolved".into());
                    }
                    text
                }
                "token" => {
                    let pair = v.as_array().filter(|a| a.len() == 2).ok_or_else(invalid)?;
                    Value::Token(
                        pair[0]
                            .as_u64()
                            .and_then(|n| u32::try_from(n).ok())
                            .ok_or_else(invalid)?,
                        Rc::from(pair[1].as_str().ok_or_else(invalid)?),
                    )
                }
                "platform" if v == "tok" => tok.clone(),
                _ => return Err(invalid()),
            }
        }
        _ => return Err(invalid()),
    })
}
impl Data {
    fn restore(self, tok: &Value) -> Result<(Rc<Map>, Rc<Map>, Value, EffectState), String> {
        // The same root twice: the noted state is the state (an idle image).
        let same = self.state == self.noted;
        let mut values = Vec::with_capacity(self.nodes.len());
        for node in self.nodes {
            let value = match node {
                Node::Array { values: items } => {
                    let a = Rc::new(Obj::frozen(
                        items
                            .iter()
                            .map(|v| read_atom(v, &values, tok))
                            .collect::<Result<Vec<_>, _>>()?,
                    ));
                    Value::List(a)
                }
                Node::Object { fields } => {
                    let mut map = Map::default();
                    for (k, v) in fields {
                        if map.insert(Rc::from(k), read_atom(&v, &values, tok)?).is_some() {
                            return Err(invalid());
                        }
                    }
                    let o = Rc::new(Obj::frozen(map));
                    Value::Map(o)
                }
                Node::Tokens { chunks } => Value::Frag(Rc::new(
                    chunks
                        .into_iter()
                        .map(|ids| crate::emit::Item::Tokens(Rc::from(ids), Rc::from([])))
                        .collect(),
                )),
            };
            values.push(value);
        }
        let Value::Map(state) = read_atom(&self.state, &values, tok)? else {
            return Err(invalid());
        };
        let Value::Map(noted) = read_atom(&self.noted, &values, tok)? else {
            return Err(invalid());
        };
        let env = read_atom(&self.env, &values, tok)?;
        let mut deps = BTreeMap::new();
        for (key, v) in self.deps {
            if deps.insert(key, (read_atom(&v, &values, tok)?, None)).is_some() {
                return Err(invalid());
            }
        }
        let state = Rc::new(state.read().clone());
        let noted = if same {
            state.clone()
        } else {
            Rc::new(noted.read().clone())
        };
        Ok((state, noted, env, deps))
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Image {
    data: Data,
    clock_ms: i64,
    rng: u64,
    #[serde(default)]
    signals: crate::effects::Signals,
    kept: BTreeMap<u64, Kept>,
    next_dispatch: u64,
    next_task: u64,
    next_handler: u64,
    stopped: bool,
    stop_value: J,
    event_log: VecDeque<(u64, String, J)>,
    version: u64,
    effects_version: u64,
    journaled: usize,
    changed_at: HashMap<String, usize>,
    props_at: usize,
    metrics: Metrics,
    last_full: Option<(Vec<u32>, Vec<u32>)>,
    last_prompt: usize,
    hash_ready: bool,
    children: BTreeMap<String, J>,
    stop_reported: bool,
    seen_msgs: BTreeSet<String>,
    /// Microtask fuel spent in the image's input before the boundary.
    fuel: u64,
}
impl Image {
    pub(crate) fn inputs(&self) -> usize {
        self.journaled
    }
    pub(crate) fn restore(
        self,
        cfg: InstanceConfig,
        view: &View,
        setup: impl FnOnce(&mut Instance),
    ) -> Result<Instance, String> {
        date(self.clock_ms)?;
        let mut a = Instance::new(cfg)?;
        setup(&mut a);
        let (tok, _) = view.checkpoint_context();
        let heap = a.heap.scope();
        let (state, noted, env, deps) = with_host_text(a.resolver.clone(), || self.data.restore(tok))?;
        drop(heap);
        a.state = state;
        a.noted = noted;
        a.realm.borrow_mut().fuel = self.fuel;
        a.env_v = env;
        a.effect_state = deps;
        a.clock_ms = self.clock_ms;
        a.rng = self.rng;
        a.signals = self.signals;
        a.kept = self.kept;
        a.next_dispatch = self.next_dispatch;
        a.next_task = self.next_task;
        a.next_handler = self.next_handler;
        a.stopped = self.stopped.then_some(self.stop_value);
        a.event_log = self.event_log;
        a.version = self.version;
        a.effects_version = self.effects_version;
        a.journaled = self.journaled;
        a.changed_at = self.changed_at.into_iter().map(|(k, v)| (Rc::from(k), v)).collect();
        a.props_at = self.props_at;
        a.metrics = self.metrics;
        a.last_full = self.last_full.map(|(p, r)| (Arc::from(p), r));
        a.last_prompt = self.last_prompt;
        if self.hash_ready {
            let (prompt, reply) = a.last_full.as_ref().ok_or_else(invalid)?;
            let mut hash = sha2::Sha256::default();
            sha_ids(&mut hash, prompt);
            sha_ids(&mut hash, reply);
            a.last_hasher = Some(hash);
        }
        a.children = self.children;
        a.stop_reported = self.stop_reported;
        a.seen_msgs = self.seen_msgs;
        Ok(a)
    }
}
impl Instance {
    pub(crate) fn trace_checkpoint_restore(&mut self, from: usize, replayed: usize) {
        self.note(json!({"t":"checkpoint_restore","from_input":from,"replayed_inputs":replayed}));
    }
    /// The agent as an image can describe it: idle (nothing pending), or at a step boundary
    /// (`boundary`: only the effects pass of the current input is pending). Else why not.
    fn image_source(&self, boundary: bool) -> Result<Source, &'static str> {
        let terminal = self.stopped.is_some();
        if self.diverged_at.is_some() {
            return Err("its replay diverged");
        }
        if !self.handlers.is_empty()
            || !self.tasks.is_empty()
            || (!boundary && (!self.cancelled.is_empty() || !Rc::ptr_eq(&self.noted, &self.state)))
        {
            return Err("work is in progress");
        }
        if boundary && (terminal || self.version == self.effects_version) {
            return Err("not a step boundary");
        }
        if !terminal {
            if !self.open.is_empty()
                || !self.events.is_empty()
                || !self.dispatches.is_empty()
                || (!boundary && self.version != self.effects_version)
            {
                return Err("work is in progress");
            }
            if !self.realm.borrow().is_idle() {
                return Err("Promise jobs are pending");
            }
            if self.effect_state.values().any(|(_, cleanup)| cleanup.is_some()) {
                return Err("an effect has a cleanup");
            }
            if self.kept.values().any(|k| k.view.is_some()) {
                return Err("a model output's view is kept for its dispatch");
            }
        }
        Ok(Source {
            state: self.state.clone(),
            noted: self.noted.clone(),
            env: self.env_v.clone(),
            deps: if terminal {
                vec![]
            } else {
                self.effect_state
                    .iter()
                    .map(|(k, (v, _))| (k.clone(), v.clone()))
                    .collect()
            },
            clock_ms: self.clock_ms,
            rng: self.rng,
            signals: self.signals.clone(),
            kept: if terminal { BTreeMap::new() } else { self.kept.clone() },
            next_dispatch: self.next_dispatch,
            next_task: self.next_task,
            next_handler: self.next_handler,
            stopped: self.stopped.clone(),
            event_log: self.event_log.clone(),
            version: self.version,
            effects_version: self.effects_version,
            journaled: self.journaled,
            changed_at: self.changed_at.clone(),
            props_at: self.props_at,
            metrics: self.metrics.clone(),
            last_full: self.last_full.clone(),
            last_prompt: self.last_prompt,
            hash_ready: self.last_hasher.is_some(),
            children: self.children.clone(),
            stop_reported: self.stop_reported,
            seen_msgs: self.seen_msgs.clone(),
            fuel: if boundary { self.realm.borrow().fuel } else { 0 },
        })
    }
    /// An image of the agent as it is now, when it is idle; else why there is none.
    pub(crate) fn checkpoint_image(&self, view: &View) -> Result<Image, &'static str> {
        self.write_image(self.image_source(false)?, view)
            .ok_or("its state holds a value an image cannot write")
    }
    /// Keep the agent at this step boundary for a later image (cheap: shared values and counters).
    pub(super) fn note_boundary(&mut self) {
        self.forget_render_promises();
        if let Ok(source) = self.image_source(true) {
            self.boundary = Some(Rc::new(source));
        }
    }
    /// The journal position of the latest step boundary kept.
    pub(crate) fn boundary_inputs(&self) -> Option<usize> {
        self.boundary.as_ref().map(|b| b.journaled)
    }
    /// An image of the latest step boundary.
    pub(crate) fn boundary_image(&self, view: &View) -> Option<Image> {
        self.write_image((**self.boundary.as_ref()?).clone(), view)
    }
    fn write_image(&self, source: Source, view: &View) -> Option<Image> {
        let terminal = source.stopped.is_some();
        let (tok, projection) = view.checkpoint_context();
        // A cache's outputs that are not its inputs: a restored agent's cold cache would make new
        // objects where the original reused these. Scan outputs carry their own mark (`derived`).
        let mut forbidden = HashSet::new();
        for cache in [&self.cache.memo, &projection].into_iter().filter(|_| !terminal) {
            let (inputs, outputs) = cache.borrow().checkpoint_turns();
            let inputs = data_ids(inputs);
            forbidden.extend(data_ids(outputs).difference(&inputs).copied());
        }
        let mut writer = Writer {
            nodes: vec![],
            ids: HashMap::new(),
            path: HashSet::new(),
            forbidden,
            tok,
        };
        let env = writer.value(&source.env, 0)?;
        // The roots stay alive while the writer runs: it names nodes by address.
        let roots = [
            Value::Map(Rc::new(Obj::frozen((*source.state).clone()))),
            Value::Map(Rc::new(Obj::frozen((*source.noted).clone()))),
        ];
        let state = writer.value(&roots[0], 0)?;
        let noted = if Rc::ptr_eq(&source.noted, &source.state) {
            state.clone()
        } else {
            writer.value(&roots[1], 0)?
        };
        let deps = source
            .deps
            .iter()
            .map(|(k, v)| Some((k.clone(), writer.dependency(v)?)))
            .collect::<Option<Vec<_>>>()?;
        Some(Image {
            data: Data {
                nodes: writer.nodes,
                state,
                noted,
                env,
                deps,
            },
            clock_ms: source.clock_ms,
            rng: source.rng,
            signals: source.signals,
            kept: source.kept,
            next_dispatch: source.next_dispatch,
            next_task: source.next_task,
            next_handler: source.next_handler,
            stopped: source.stopped.is_some(),
            stop_value: source.stopped.unwrap_or(J::Null),
            event_log: source.event_log,
            version: source.version,
            effects_version: source.effects_version,
            journaled: source.journaled,
            changed_at: source.changed_at.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
            props_at: source.props_at,
            metrics: source.metrics,
            last_full: source.last_full.as_ref().map(|(p, r)| (p.to_vec(), r.clone())),
            last_prompt: source.last_prompt,
            hash_ready: source.hash_ready,
            children: source.children,
            stop_reported: source.stop_reported,
            seen_msgs: source.seen_msgs,
            fuel: source.fuel,
        })
    }
    /// After restoring an image taken at a step boundary: run the input's remaining effects
    /// pass, as the original did there. Its commands are outstanding work (the host resubmits
    /// `outstanding()`); its trace was written by the original and is dropped.
    pub(crate) fn finish_boundary(&mut self, view: &View, enc: &Encoder) -> Result<(), String> {
        if self.stopped.is_some() || self.version == self.effects_version {
            return Ok(());
        }
        let _heap = self.heap.scope();
        with_host_text(self.resolver.clone(), || {
            crate::microtasks::within(self.realm.clone(), || {
                let mut out = self.checkpoint(view, enc)?;
                self.finish_commands(&mut out);
                Ok::<_, String>(())
            })
        })?;
        self.cancelled.clear();
        self.take_trace();
        Ok(())
    }
}

/// What an image is written from: the agent's state and counters at one point, shared, not copied.
#[derive(Clone)]
pub(crate) struct Source {
    state: Rc<Map>,
    noted: Rc<Map>,
    env: Value,
    deps: Vec<(String, Value)>,
    clock_ms: i64,
    rng: u64,
    signals: crate::effects::Signals,
    kept: BTreeMap<u64, Kept>,
    next_dispatch: u64,
    next_task: u64,
    next_handler: u64,
    stopped: Option<J>,
    event_log: VecDeque<(u64, String, J)>,
    version: u64,
    effects_version: u64,
    journaled: usize,
    changed_at: HashMap<Rc<str>, usize>,
    props_at: usize,
    metrics: Metrics,
    last_full: Option<(Arc<[u32]>, Vec<u32>)>,
    last_prompt: usize,
    hash_ready: bool,
    children: BTreeMap<String, J>,
    stop_reported: bool,
    seen_msgs: BTreeSet<String>,
    fuel: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
    };
    struct App(PathBuf);
    impl App {
        fn new(source: &str) -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "aff-checkpoint-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&path).unwrap();
            std::fs::write(
                path.join("app.tsx"),
                source.replace("@ROOT@", env!("CARGO_MANIFEST_DIR")),
            )
            .unwrap();
            Self(path)
        }
        fn view(&self, enc: &Encoder) -> View {
            View::load(&self.0.join("app.tsx"), "Main", enc).unwrap()
        }
    }
    impl Drop for App {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn encoder() -> Encoder {
        Encoder::from_file(crate::test_support::path().to_str().unwrap()).unwrap()
    }
    fn config() -> InstanceConfig {
        InstanceConfig {
            context: None,
            retry: Default::default(),
            template: None,
            id: "a".into(),
            props: json!({}),
            env: BTreeMap::from([("DEMO".into(), "value".into())]),
            max_new: 20,
            max_turns: None,
            max_prompt: None,
            start_ms: 100,
            seed: 7,
            machines: vec!["local".into()],
            model: Some("default".into()),
        }
    }
    fn event(name: &str, msg: &str, clock_ms: i64) -> Input {
        Input::Event {
            name: name.into(),
            payload: json!({}),
            id: Some(msg.into()),
            clock_ms,
        }
    }
    fn answer(commands: &[Command], enc: &Encoder, clock_ms: i64) -> Input {
        let task = commands
            .iter()
            .find_map(|c| match c {
                Command::Generate { task, .. } => Some(*task),
                _ => None,
            })
            .unwrap();
        Input::Reply {
            server: None,
            task,
            ids: enc.tok.encode("answer", false).unwrap().get_ids().to_vec(),
            cut_off: false,
            clock_ms,
            prompt: None,
            aborted: false,
        }
    }
    #[test]
    fn idle_images_preserve_identity_effects_dispatch_and_future_execution() {
        let app = App::new(
            r#"
export default function Main(){
  const [data]=useState(()=>{const shared={n:1};return {a:shared,b:shared,missing:undefined,nan:NaN,neg:-0,token:tok.im_end,ns:tok,env:harness.env};},'data');
  const [runs,setRuns]=useState(0,'runs');
  const [seen,setSeen]=useState(null,'seen');
  const [last,setLast]=useState(null,'last');
  const [blob,setBlob]=useState(null,'blob');
  useEffect(()=>setRuns(n=>n+1),[data.a,harness.env,tok]);
  useEvent('advance',async()=>{const r=await harness.random();const t=await harness.now();setSeen({r,t,
    same:data.a===data.b,missing:'missing' in data && data.missing===undefined,nan:Number.isNaN(data.nan),neg:Object.is(data.neg,-0),
    env:data.env===harness.env,ns:data.ns===tok,token:data.token===tok.im_end,frozen:Object.isFrozen(data.a)});});
  useEvent('ask',async()=>{const out=await model();await dispatch(out);setLast(out);});
  useEvent('again',async()=>{await dispatch(last);});
  useEvent('blob',()=>{const text='x'.repeat(1000);setBlob({a:text,b:text});});
  useEvent('choose',()=>setBlob(d=>({...d,a:d.b})));
  useEvent('stop',()=>{setRuns(n=>n+1);harness.stop(null);});
  useEvent('mutate',()=>{payload.n=99;});
  return <><span label='protocol' syntax='hermes'/>{runs}</>;
}"#,
        );
        let enc = encoder();
        let view = app.view(&enc);
        let mut cfg = config();
        cfg.props = json!({"payload":{"n":0}});
        let blobs = Arc::new(Blobs::new(app.0.join("blobs"), 64).unwrap());
        let mut original = Instance::new(cfg.clone()).unwrap();
        original.set_blobs(blobs.clone());
        original.start(&view, &enc).unwrap();
        let mut journal = vec![];
        for input in [
            event("mutate", "m0", 100),
            event("blob", "b1", 100),
            event("advance", "m1", 101),
            event("ask", "m2", 102),
        ] {
            journal.push(input.clone());
            let commands = original.handle(&view, &enc, input).unwrap();
            if !commands.is_empty() {
                let reply = answer(&commands, &enc, 103);
                journal.push(reply.clone());
                original.handle(&view, &enc, reply).unwrap();
            }
        }
        let image = original
            .checkpoint_image(&view)
            .expect("completed chatbot is checkpointable");
        let bytes = serde_json::to_vec(&image).unwrap();
        let fresh = app.view(&enc);
        let mut restored: Instance = serde_json::from_slice::<Image>(&bytes)
            .unwrap()
            .restore(cfg.clone(), &fresh, |a| a.set_blobs(blobs.clone()))
            .unwrap();
        let (mut replay, _) =
            Instance::restore_with(cfg, journal, &fresh, &enc, |a| a.set_blobs(blobs.clone())).unwrap();
        assert_eq!(restored.state(), replay.state());
        for input in [
            event("choose", "b2", 104),
            event("advance", "m1", 104),
            event("advance", "m3", 105),
            event("again", "m4", 106),
            event("ask", "m5", 107),
        ] {
            let a = restored.handle(&fresh, &enc, input.clone()).unwrap();
            let b = replay.handle(&fresh, &enc, input).unwrap();
            assert_eq!(a, b);
            if !a.is_empty() {
                let reply = answer(&a, &enc, 108);
                assert_eq!(
                    restored.handle(&fresh, &enc, reply.clone()).unwrap(),
                    replay.handle(&fresh, &enc, reply).unwrap()
                );
            }
            assert_eq!(restored.state(), replay.state());
            assert_eq!(restored.metrics, replay.metrics);
            assert_eq!(restored.journal_len(), replay.journal_len());
            assert_eq!(restored.changed_at(Some("@blob")), replay.changed_at(Some("@blob")));
        }
        assert_eq!(restored.state()["__ui"]["@runs"], 1);
        for key in ["same", "missing", "nan", "neg", "env", "ns", "token", "frozen"] {
            assert_eq!(restored.state()["__ui"]["@seen"][key], true, "{key}");
        }
        assert_eq!(restored.metrics.handler_errors, 1);
        assert_eq!(restored.state()["payload"]["n"], 0);
        restored.handle(&fresh, &enc, event("stop", "m6", 109)).unwrap();
        let image = restored
            .checkpoint_image(&fresh)
            .expect("terminal data stays checkpointable after stopping work");
        let mut stopped = image
            .restore(restored.cfg.clone(), &fresh, |a| a.set_blobs(blobs.clone()))
            .unwrap();
        for input in [
            event("advance", "late", 110),
            Input::Props {
                props: json!({"after":"stop"}),
                clock_ms: 111,
            },
        ] {
            assert_eq!(
                restored.handle(&fresh, &enc, input.clone()).unwrap(),
                stopped.handle(&fresh, &enc, input).unwrap()
            );
            assert_eq!(restored.state(), stopped.state());
            assert_eq!(restored.stopped(), stopped.stopped());
        }
    }
    #[test]
    fn conversation_library_checkpoints_after_a_reply_and_preserves_the_next_prompt() {
        let app = App::new(
            r#"
import {useLog} from '@ROOT@/views/lib/log.tsx';
import {useConversation} from '@ROOT@/views/lib/loop.tsx';
import {Qwen3App} from '@ROOT@/views/template_apps/qwen3.tsx';
import {TurnStrategy} from '@ROOT@/views/strategies/turn_strategy.tsx';
export default function Main(){const log=useLog();useConversation(log);
  useEvent('message',({text})=>log.append({kind:'input',text}));
  return <Qwen3App><TurnStrategy strategy='unrolled' log={log.records}>ready</TurnStrategy></Qwen3App>;
}"#,
        );
        let enc = encoder();
        let view = app.view(&enc);
        let cfg = config();
        let mut a = Instance::new(cfg.clone()).unwrap();
        a.start(&view, &enc).unwrap();
        let input = Input::Event {
            name: "message".into(),
            payload: json!({"text":"hello"}),
            id: None,
            clock_ms: 101,
        };
        let commands = a.handle(&view, &enc, input.clone()).unwrap();
        let reply = answer(&commands, &enc, 102);
        a.handle(&view, &enc, reply.clone()).unwrap();
        let image = a
            .checkpoint_image(&view)
            .expect("ordinary conversation library can checkpoint");
        let fresh = app.view(&enc);
        let mut cached = image.restore(cfg.clone(), &fresh, |_| {}).unwrap();
        let (mut full, _) = Instance::restore(cfg, [input, reply], &fresh, &enc).unwrap();
        let next = Input::Event {
            name: "message".into(),
            payload: json!({"text":"again"}),
            id: None,
            clock_ms: 103,
        };
        assert_eq!(
            cached.handle(&fresh, &enc, next.clone()).unwrap(),
            full.handle(&fresh, &enc, next).unwrap()
        );
        assert_eq!(cached.metrics, full.metrics);
        assert_eq!(cached.state(), full.state());
    }
    #[test]
    fn consumed_tool_arguments_do_not_bloat_recovery_images() {
        let app = App::new(
            r#"import {MessagesApp} from '@ROOT@/views/template_apps/messages.tsx';
        import {TurnStrategy} from '@ROOT@/views/strategies/turn_strategy.tsx';
        export default function Main(){
          const [seen,setSeen]=useState(null,'seen'); const [runs,setRuns]=useState(0,'runs');
          useEffect(()=>{(async()=>{
            const first=await model(); const a=await dispatch(first);
            await dispatch(await model()); const b=await dispatch(first); setSeen([a,b]);
          })();},[]);
          return <MessagesApp><TurnStrategy log={[]}><aff label="consume" tool_name="consume" args_schema={{type:'object',properties:{text:{type:'string'}},required:['text']}}
            onRun={({text})=>{setRuns(n=>n+1);return text.length;}}>consume</aff>
            </TurnStrategy></MessagesApp>;
        }"#,
        );
        let enc = encoder();
        let view = app.view(&enc);
        let cfg = config();
        let mut a = Instance::new(cfg.clone()).unwrap();
        let mut commands = a.start(&view, &enc).unwrap();
        let payload = "word ".repeat(52_429);
        let mut journal = vec![];
        for (i, response) in [
            json!({"content":"","tool_calls":[{"name":"consume","arguments":{"text":payload}}]}),
            json!({"content":"done","tool_calls":[]}),
        ]
        .into_iter()
        .enumerate()
        {
            let task = commands
                .iter()
                .find_map(|c| match c {
                    Command::Generate { task, .. } => Some(*task),
                    _ => None,
                })
                .unwrap();
            let ids = enc
                .tok
                .encode(response.to_string().as_str(), false)
                .unwrap()
                .get_ids()
                .to_vec();
            let input = Input::Reply {
                server: None,
                task,
                ids,
                cut_off: false,
                prompt: None,
                clock_ms: 101 + i as i64,
                aborted: false,
            };
            journal.push(input.clone());
            commands = a.handle(&view, &enc, input).unwrap();
        }
        assert_eq!(a.metrics.handler_errors, 0);
        let seen = a.state()["__ui"]["@seen"].clone();
        assert_eq!(seen[0], seen[1], "repeated dispatch returns the original result");
        assert_eq!(seen[0][0]["result"], payload.len().to_string());
        assert_eq!(a.state()["__ui"]["@runs"], 1, "the tool runs once");
        let image = a.checkpoint_image(&view).expect("settled app can checkpoint");
        let image_bytes = serde_json::to_vec(&image).unwrap().len();
        eprintln!(
            "consumed argument: {} bytes; recovery image: {image_bytes} bytes",
            payload.len()
        );
        assert!(
            image_bytes < payload.len() / 2,
            "a recovery image retained {image_bytes} bytes after consuming a {}-byte tool argument",
            payload.len()
        );
        let mut cached = image.restore(cfg.clone(), &view, |_| {}).unwrap();
        let (mut replayed, _) = Instance::restore(cfg, journal, &view, &enc).unwrap();
        let first = *a.kept.keys().next().unwrap();
        for restored in [&mut cached, &mut replayed] {
            assert_eq!(restored.state(), a.state());
            let (answer, commands) = restored.dispatch(&view, &enc, (999, 0), Some(first), None).unwrap();
            assert_eq!(answer, Some(Outcome::Ok(seen[0].clone())));
            assert!(commands.is_empty());
        }
    }

    #[test]
    fn automatic_checkpoints_wait_for_the_interval_and_force_save_the_latest_state() {
        use std::io::Write;
        let app = App::new("export default function Main(){return 'idle';}");
        let enc = encoder();
        let view = app.view(&enc);
        let mut a = Instance::new(config()).unwrap();
        a.start(&view, &enc).unwrap();
        let dir = app.0.join("a");
        std::fs::create_dir(&dir).unwrap();
        let mut journal = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("journal.jsonl"))
            .unwrap();
        let mut cache = crate::runtime::checkpoint::Checkpoint::new(&dir, "same".into(), None).unwrap();
        let path = dir.join("checkpoint.json");
        let mut saved = vec![];
        for n in 1..=257 {
            let input = Input::Tick { clock_ms: 100 + n };
            let line = format!("{}\n", serde_json::to_string(&input).unwrap());
            journal.write_all(line.as_bytes()).unwrap();
            cache.appended(line.as_bytes());
            a.handle(&view, &enc, input).unwrap();
            cache.save(&a, &view, false, &|| Ok(())).unwrap();
            if n < 256 {
                assert!(!path.exists());
            } else if n == 256 {
                saved = std::fs::read(&path).unwrap();
            }
        }
        assert_eq!(std::fs::read(&path).unwrap(), saved);
        let loaded = crate::runtime::checkpoint::load(&dir, "same", None).unwrap().unwrap();
        assert_eq!(loaded.image.inputs(), 256);
        cache.save(&a, &view, true, &|| Ok(())).unwrap();
        assert_eq!(
            crate::runtime::checkpoint::load(&dir, "same", None)
                .unwrap()
                .unwrap()
                .image
                .inputs(),
            257
        );
        assert!(crate::runtime::checkpoint::load(&dir, "different", None).is_err());
    }
    /// An agent that cannot be imaged is told once, with the reason, until it is imaged again.
    #[test]
    fn a_refused_image_reports_its_reason_once() {
        use std::io::Write;
        let app = App::new(
            "export default function Main(){const [n,setN]=useState(0,'n');useEvent('bump',()=>setN(x=>x+1));useEffect(()=>n<3?()=>{}:undefined,[n]);return String(n);}",
        );
        let enc = encoder();
        let view = app.view(&enc);
        let mut a = Instance::new(config()).unwrap();
        a.start(&view, &enc).unwrap();
        let dir = app.0.join("a");
        std::fs::create_dir(&dir).unwrap();
        let mut journal = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("journal.jsonl"))
            .unwrap();
        let mut cache = crate::runtime::checkpoint::Checkpoint::new(&dir, "same".into(), None).unwrap();
        let mut reports = vec![];
        for n in 1..=4 {
            let input = Input::Event {
                name: "bump".into(),
                payload: json!({}),
                id: None,
                clock_ms: 100 + n,
            };
            let line = format!("{}\n", serde_json::to_string(&input).unwrap());
            journal.write_all(line.as_bytes()).unwrap();
            cache.appended(line.as_bytes());
            a.handle(&view, &enc, input).unwrap();
            reports.push((
                cache.save(&a, &view, true, &|| Ok(())).unwrap(),
                dir.join("checkpoint.json").exists(),
            ));
        }
        // Inputs 1 and 2 leave a cleanup: refused, and told once. 3 and 4 are imaged.
        assert_eq!(
            reports,
            [
                (Some("an effect has a cleanup"), false),
                (None, false),
                (None, true),
                (None, true)
            ]
        );
        assert_eq!(
            crate::runtime::checkpoint::load(&dir, "same", None)
                .unwrap()
                .unwrap()
                .image
                .inputs(),
            4
        );
    }
    #[test]
    fn active_code_cleanup_and_untracked_dependency_objects_use_full_replay() {
        let enc = encoder();
        for body in [
            "useEffect(()=>{harness.wait({events:['go']});},[]);",
            "useEffect(()=>()=>{},[]);",
            "useState(()=>()=>1);",
            "useEffect(()=>{},[{}]);",
            "useState(()=>new Map());",
            // A scan's output kept in state: a cold cache would make a new one where it reuses this.
            "const r=memoScan([1],0,(s,x)=>[s+x,{x}],[]);const [v,setV]=useState(null,'v');useEffect(()=>{setV(r.outs[0]);},[]);",
        ] {
            let app = App::new(&format!("export default function Main(){{{body}return 'waiting';}}"));
            let view = app.view(&enc);
            let mut a = Instance::new(config()).unwrap();
            a.start(&view, &enc).unwrap();
            assert!(a.checkpoint_image(&view).is_err(), "{body}");
        }
    }
    /// A loop agent is imaged at every step boundary (inside the input that ends a step), and an
    /// image restored on a fresh view, its pending effects pass run, then behaves exactly as the
    /// original: the same commands for every later input, the same state, metrics and change
    /// notes. A full replay of the whole journal agrees too.
    #[test]
    fn loop_agents_image_at_step_boundaries_and_continue_exactly() {
        let app = App::new(
            r#"
import {useLog} from '@ROOT@/views/lib/log.tsx';
import {useLoop} from '@ROOT@/views/lib/loop.tsx';
import {MessagesApp} from '@ROOT@/views/template_apps/messages.tsx';
import {TurnStrategy} from '@ROOT@/views/strategies/turn_strategy.tsx';
export default function Main(){
  const log=useLog(); useLoop(log);
  const [n,setN]=useState(0,'n'); const [flip,setFlip]=useState('a','flip');
  const sizes=memoScan(log.records,0,(s,r)=>[s+1,{kind:r.kind,at:s}],[]);
  return <MessagesApp><TurnStrategy log={log.records}>
    <aff label="bump" tool_name="bump" args_schema={{type:'object',properties:{}}} onRun={()=>{setN(x=>x+1);setFlip('b');setFlip('a');return 'bumped';}}>bump</aff>
    <aff label="shell" tool_name="shell" args_schema={{type:'object',properties:{cmd:{type:'string'}},required:['cmd']}} onRun={async({cmd})=>(await harness.machine.call({cmd})).output}>shell</aff>
    {`n=${n} records=${sizes.state} last=${sizes.outs.length?sizes.outs[sizes.outs.length-1].kind:''}`}
  </TurnStrategy></MessagesApp>;
}"#,
        );
        let enc = encoder();
        let view = app.view(&enc);
        let cfg = config();
        let mut original = Instance::new(cfg.clone()).unwrap();
        let mut commands = original.start(&view, &enc).unwrap();
        let mut journal: Vec<Input> = vec![];
        let mut restored: Vec<(usize, Instance)> = vec![];
        let fresh = app.view(&enc);
        let mut boundaries = 0;
        for i in 0..14 {
            let clock_ms = 200 + i as i64;
            let input = commands.iter().find_map(|c| match c {
                Command::Generate { task, .. } => {
                    let reply = match i % 3 { 0 => json!({"content":"","tool_calls":[{"name":"bump","arguments":{}}]}),
                        1 => json!({"content":"","tool_calls":[{"name":"shell","arguments":{"cmd":format!("echo {i}")}}]}),
                        _ => json!({"content":format!("thought {i}"),"tool_calls":[]}) };
                    let ids = enc.tok.encode(reply.to_string().as_str(), false).unwrap().get_ids().to_vec();
                    Some(Input::Reply { server: None, task: *task, ids, cut_off: false, prompt: None, clock_ms, aborted: false })
                }
                Command::Call { task, name, .. } if name.starts_with("machine:") => Some(Input::Result { task: *task, outcome: Outcome::Ok(json!({"output": format!("out {i}"), "exit": 0})), clock_ms }),
                _ => None,
            }).expect("the loop always waits on a model or machine call");
            journal.push(input.clone());
            commands = original.handle(&view, &enc, input.clone()).unwrap();
            // The status summary reads the outstanding commands in place; it agrees with them.
            let (all, summary) = (original.outstanding(), original.outstanding_summary());
            let mut timers: Vec<i64> = all
                .iter()
                .filter_map(|c| match c {
                    Command::Timer { at_ms } => Some(*at_ms),
                    _ => None,
                })
                .collect();
            let mut got = summary.timers.clone();
            timers.sort_unstable();
            got.sort_unstable();
            assert_eq!((summary.count, got), (all.len(), timers), "input {i}");
            assert_eq!(
                summary.phase,
                if all.iter().any(|c| matches!(c, Command::Generate { .. })) {
                    "model"
                } else if all.iter().any(|c| matches!(c, Command::Call { .. })) {
                    "tool"
                } else if all.is_empty() {
                    "idle"
                } else {
                    "waiting"
                }
            );
            for (_, r) in restored.iter_mut() {
                assert_eq!(r.handle(&fresh, &enc, input.clone()).unwrap(), commands, "input {i}");
                assert_eq!(r.state(), original.state(), "input {i}");
                assert_eq!(r.metrics, original.metrics);
                for key in ["@n", "@flip", "@log", "@loop"] {
                    assert_eq!(
                        r.changed_at(Some(key)),
                        original.changed_at(Some(key)),
                        "{key} after input {i}"
                    );
                }
            }
            if original.boundary_inputs() == Some(original.journal_len()) {
                boundaries += 1;
                let image = original
                    .boundary_image(&view)
                    .expect("a loop step boundary is imageable");
                let bytes = serde_json::to_vec(&image).unwrap();
                let mut agent = serde_json::from_slice::<Image>(&bytes)
                    .unwrap()
                    .restore(cfg.clone(), &fresh, |_| {})
                    .unwrap();
                agent.finish_boundary(&fresh, &enc).unwrap();
                assert_eq!(agent.outstanding(), original.outstanding(), "boundary at input {i}");
                assert_eq!(agent.state(), original.state(), "boundary at input {i}");
                for key in ["@n", "@flip", "@log", "@loop"] {
                    assert_eq!(
                        agent.changed_at(Some(key)),
                        original.changed_at(Some(key)),
                        "{key} at boundary {i}"
                    );
                }
                restored.push((i, agent));
            }
        }
        assert!(boundaries >= 8, "every step ends at a boundary: {boundaries}");
        assert!(original.state()["__ui"]["@n"].as_i64().unwrap() >= 4);
        let (full, _) = Instance::restore(cfg, journal, &fresh, &enc).unwrap();
        assert_eq!(full.state(), original.state());
        assert_eq!(full.outstanding(), original.outstanding());
    }
    /// A Promise used during render does not stop an image, idle or at a step boundary; the
    /// boundary image, its effects pass run, continues as the original does.
    #[test]
    fn a_promise_used_during_render_is_imageable() {
        let enc = encoder();
        for body in [
            "",
            "Promise.resolve(1);",
            "useState(()=>{Promise.all([1,2]);return 0;},'p');",
        ] {
            let app = App::new(&format!(
                "export default function Main(){{{body}const [n,setN]=useState(0,'n');useEvent('bump',()=>setN(x=>x+1));return String(n);}}"
            ));
            let (view, fresh) = (app.view(&enc), app.view(&enc));
            let mut a = Instance::new(config()).unwrap();
            a.start(&view, &enc).unwrap();
            if let Err(why) = a.checkpoint_image(&view) {
                panic!("idle: {body}: {why}")
            }
            let bump = |clock_ms| Input::Event {
                name: "bump".into(),
                payload: json!({}),
                id: None,
                clock_ms,
            };
            a.handle(&view, &enc, bump(101)).unwrap();
            assert_eq!(a.boundary_inputs(), Some(a.journal_len()), "boundary: {body}");
            let mut restored = a
                .boundary_image(&view)
                .expect("boundary image")
                .restore(config(), &fresh, |_| {})
                .unwrap();
            restored.finish_boundary(&fresh, &enc).unwrap();
            if let Err(why) = a.checkpoint_image(&view) {
                panic!("idle after an input: {body}: {why}")
            }
            assert_eq!(
                restored.handle(&fresh, &enc, bump(102)).unwrap(),
                a.handle(&view, &enc, bump(102)).unwrap(),
                "{body}"
            );
            assert_eq!(restored.state(), a.state(), "{body}");
            assert_eq!(a.state()["__ui"]["@n"], json!(2), "{body}");
        }
    }
    /// A scan whose outputs stay in the render (the cache's own) does not stop an image.
    #[test]
    fn a_scan_used_only_by_the_render_is_imageable() {
        let enc = encoder();
        let app = App::new(
            "export default function Main(){const r=memoScan([1,2],0,(s,x)=>[s+x,{x}],[]);const [n]=useState(r.state,'n');return `${n} ${r.outs.length}`;}",
        );
        let view = app.view(&enc);
        let mut a = Instance::new(config()).unwrap();
        a.start(&view, &enc).unwrap();
        assert!(a.checkpoint_image(&view).is_ok());
    }
    #[test]
    fn data_graph_requires_immutable_values_and_checks_its_references() {
        let tok = Value::map(Map::default());
        let object = Value::map(Map::from_iter([(Rc::from("x"), Value::Int(1))]));
        for shallow in [false, true] {
            if shallow {
                let Value::Map(o) = &object else { unreachable!() };
                o.freeze_shallow();
            }
            let mut writer = Writer {
                nodes: vec![],
                ids: HashMap::new(),
                path: HashSet::new(),
                forbidden: HashSet::new(),
                tok: &tok,
            };
            assert!(writer.value(&object, 0).is_none());
        }
        crate::value::freeze_state(&object).unwrap();
        let mut writer = Writer {
            nodes: vec![],
            ids: HashMap::new(),
            path: HashSet::new(),
            forbidden: HashSet::new(),
            tok: &tok,
        };
        let state = writer.value(&object, 0).unwrap();
        let data = Data {
            nodes: writer.nodes,
            noted: state.clone(),
            state,
            env: J::Null,
            deps: vec![],
        };
        assert_eq!(data.restore(&tok).unwrap().0["x"].to_json(), json!(1));
        assert!(read_atom(&json!({"ref":10}), &[], &tok).is_err());
        assert!(read_atom(&json!({"blob":"missing"}), &[], &tok).is_err());
    }
}
