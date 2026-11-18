//! Tree-walking interpreter: state (JSON) + view program -> token stream.

use crate::ast::*;
#[path = "compiler.rs"]
pub(crate) mod compiler;
#[path = "document.rs"]
pub(crate) mod document;
#[path = "vm.rs"]
pub(crate) mod vm;
use crate::emit::{Capture, Item, Out, SpanKind, SpanSpec};
use crate::modules::Binding;
use crate::value::*;
use document::{Element, Siblings};
use std::collections::HashMap;
use std::rc::Rc;

pub type RResult<T> = Result<T, String>;
/// Private unwind used only by resumable action dispatch.
/// Turn maps a template was handed, by address: the log record index and kind each came from.
type TurnOrigins = std::collections::HashMap<usize, (usize, Rc<str>), ahash::RandomState>;

pub const SUSPEND: &str = "\u{0}suspend";

pub struct Program {
    pub funcs: HashMap<Binding, Rc<Func>>,
    pub consts: Vec<(Binding, Expr)>,
    pub entry: Sym,
    pub root: Sym,
    pub modules: HashMap<Sym, crate::modules::Scope>,
    /// Shadowable platform globals, pointing into the isolated standard-library module.
    pub globals: HashMap<Sym, Binding>,
    pub pragmas: Vec<String>,
    /// Token style sheet rules from imported `.tss` files.
    pub styles: Vec<crate::style::Rule>,
    /// Where modules may come from (canonical directories): the app's own and the library's.
    /// Empty: anywhere (a trusted author's tool or test). See `View::load_within`.
    pub roots: Vec<std::path::PathBuf>,
    /// The deployment's model templates: what `affordance:model` is made from.
    pub templates: Vec<String>,
}

#[derive(Debug)]
enum Flow {
    Normal,
    Break(Option<Sym>),
    Continue(Option<Sym>),
    Return(Value),
}

enum Call {
    Normal,
    Body(Option<Rc<crate::effects::Promise>>),
}
impl Call {
    fn entry(self) -> Option<Rc<crate::effects::Promise>> {
        match self {
            Self::Normal => None,
            Self::Body(entry) => entry,
        }
    }
}

/// Steps (loop iterations and function calls) one render or handler run may take: a runaway
/// view is an error, never a hung worker that hosts other instances.
pub const FUEL: u64 = 10_000_000;

/// `[...x, more]` or `[...x.slice(0, -k), more]` assigned to `x`: how many items to drop first.
fn grow_in_place(name: &Sym, items: &[Expr]) -> Option<usize> {
    let Some(Expr::Spread(first)) = items.first() else {
        return None;
    };
    let int = |e: &Expr| match e {
        Expr::Lit(Value::Int(i)) => Some(*i),
        Expr::Neg(x) => match &**x {
            Expr::Lit(Value::Int(i)) => Some(-*i),
            _ => None,
        },
        _ => None,
    };
    match &**first {
        Expr::Var(v) if v == name => Some(0),
        Expr::Method(base, m, args)
            if &**m == "slice"
                && matches!(&**base, Expr::Var(v) if v == name)
                && args.len() == 2
                && int(&args[0]) == Some(0)
                && int(&args[1]).is_some_and(|k| k < 0) =>
        {
            Some((-int(&args[1]).unwrap()) as usize)
        }
        _ => None,
    }
}

thread_local! {
    /// A per-function profile of TSX (`profile_start` / `profile_take`): calls and inclusive time.
    static PROFILE: std::cell::RefCell<Option<HashMap<String, (u64, u64)>>> = const { std::cell::RefCell::new(None) };
}

/// Start profiling named TSX functions on this thread (calls and inclusive time).
pub(crate) fn profile_start() {
    PROFILE.with(|p| *p.borrow_mut() = Some(HashMap::new()));
}

/// Stop profiling: `(function, calls, inclusive ms)`, the slowest first.
pub(crate) fn profile_take() -> Vec<(String, u64, f64)> {
    let m = PROFILE.with(|p| p.borrow_mut().take()).unwrap_or_default();
    let mut v: Vec<_> = m.into_iter().map(|(k, (n, ns))| (k, n, ns as f64 / 1e6)).collect();
    v.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
    v
}

/// See `Interp::snapshot`.
#[derive(Clone)]
pub struct Snapshot {
    globals: Rc<Map>,
    specials: Rc<Map>,
    consts: Map,
    now: chrono::NaiveDateTime,
    path: Vec<String>,
    handlers: Vec<Value>,
    ui_seen: Map,
    consts_ready: bool,
    instances: HashMap<Sym, Rc<crate::modules::Instance>>,
    scope: Rc<crate::modules::Instance>,
    turn_origins: Rc<TurnOrigins>,
}

impl Snapshot {
    pub(crate) fn contains_hook(&self, key: &str) -> bool {
        self.ui_seen.contains_key(key)
    }
    pub(crate) fn with_context(&self, harness: Value, now: chrono::NaiveDateTime) -> Self {
        let mut snapshot = self.clone();
        // A reused render's captures must still alias unchanged context fields. Replacing an
        // equal model/roles object would break `captured === harness.model/machines` in handlers.
        let harness = match (self.consts.get("harness"), &harness) {
            (Some(Value::Map(old)), Value::Map(new)) => {
                if old
                    .read()
                    .iter()
                    .all(|(key, value)| new.read().get(key).is_some_and(|v| value.to_wire() == v.to_wire()))
                    && old.read().len() == new.read().len()
                {
                    Value::Map(old.clone())
                } else {
                    let values = new
                        .read()
                        .iter()
                        .map(|(key, value)| {
                            let old_value = old.read().get(key).filter(|v| v.to_wire() == value.to_wire()).cloned();
                            (key.clone(), old_value.unwrap_or_else(|| value.clone()))
                        })
                        .collect();
                    let value = Value::map(values);
                    crate::value::freeze(&value);
                    value
                }
            }
            _ => harness,
        };
        snapshot.consts.insert("harness".into(), harness);
        snapshot.now = now;
        snapshot
    }
}

#[derive(Clone)]
pub struct Interp<'p> {
    temps: Vec<Value>,
    pub(crate) realm: Option<crate::microtasks::Realm>,
    pub(crate) task: u64,
    pub(crate) services: Option<(i64, u64)>,
    /// This run's view of the instance's abort signals (the instance's own, in a handler step).
    pub(crate) signals: crate::effects::Signals,
    pub(crate) prog: &'p Program,
    instances: HashMap<Sym, Rc<crate::modules::Instance>>,
    scope: Rc<crate::modules::Instance>,
    /// Steps taken by this run (`FUEL` bounds them).
    pub(crate) steps: u64,
    initializing: usize,
    globals: Rc<Map>,
    specials: Rc<Map>,
    consts: Map,
    scopes: Vec<(Sym, Slot)>,
    frames: Vec<usize>,
    pub now: chrono::NaiveDateTime,
    depth: usize,
    // ---- useState / actions ----
    /// The elements being built, outermost first, for descendant selectors; kept only when a
    /// sheet has one (`descendants`).
    pub(crate) ancestry: Vec<(Sym, Rc<Vec<(Sym, Value)>>)>,
    descendants: bool,
    /// Structural element path for local hook/viewport state (keyed or positional).
    path: Vec<String>,
    /// Per element scope: sibling tag counts/keys, with rollback for abandoned construction.
    sibling_counts: Vec<Siblings>,
    /// Per path level: hooks used so far in this component call.
    hook_counts: Vec<usize>,
    /// Handlers of on* props, in render order; affordance spans carry their index.
    pub handlers: Vec<Value>,
    /// true while an action handler runs: setters write here instead of failing.
    pub acting: bool,
    pub ui_updates: Map,
    /// An agent's live hook state, shared by its tasks: a setter writes here once,
    /// and components evaluated in a task read it. `None` outside an agent (the overlay above).
    pub live_ui: Option<Rc<std::cell::RefCell<Map>>>,
    /// `useEffect(setup, deps)` calls of this render: (identity, deps, setup), in tree order.
    pub effect_decls: Vec<(String, Value, Value)>,
    /// `useEvent(name, listener)` calls of this render, in order: laid out as zero-token
    /// listener spans after the root (`<aff event=name onCallback>`'s).
    event_decls: Vec<(String, Value)>,
    callback_decls: Vec<Rc<SpanSpec>>,
    /// Per component call being evaluated (the first entry: the root), the screen actions its
    /// `templateInput` calls consumed. The element lays out those its output does not carry.
    template_actions: Vec<Vec<crate::emit::Span>>,
    pub(crate) mount_decls: Vec<serde_json::Value>,
    /// The value each hook had in the last render (the setter's "current" value).
    ui_seen: Map,
    pub effects: Option<crate::effects::Effects>,
    consts_ready: bool,
    pub(crate) encoder: Option<&'p crate::emit::Encoder>,
    pub(crate) window: Option<crate::emit::Window>,
    projection_cache: Option<(&'p std::cell::RefCell<crate::emit::RunCache>, bool)>,
    /// Line of the statement running now (TSX), for error locations.
    line: u32,
    /// Turn maps handed to a template by `templateInput`, with the log record each came from.
    turn_origins: Rc<TurnOrigins>,
    /// The pending thrown value, restored when an await observes a rejected promise.
    pub(crate) thrown: Option<Value>,
    /// The error text the last `throw` raised (a `catch` of that error binds `thrown`).
    pub(crate) thrown_text: String,
    /// `memoScan`'s store (an agent's renders), and this render's count of calls per scan site.
    memo: Option<Rc<std::cell::RefCell<crate::memo::MemoStore>>>,
    memo_counts: HashMap<(usize, String), usize>,
    /// This run's id (see `VarCell`): closures share the variables they capture within a run.
    pub(crate) run: u64,
    /// The arrays and objects `JSON.stringify` is inside (a value that contains itself throws).
    pub(crate) json_path: Vec<usize>,
    /// The receiver of the call about to be made (`o.f()`, `f.call(t)`): the next function
    /// called takes it as `this`.
    pending_this: Option<Value>,
    /// The class of the method about to be called (its `super`).
    pending_super: Option<Rc<crate::value::Class>>,
}

#[derive(Clone)]
enum ReferenceKey {
    Public(Value),
    Private(Rc<crate::value::Class>, Sym),
}

/// A local variable: a plain value, or a cell shared with the closures that captured it.
#[derive(Clone)]
pub(crate) enum Slot {
    Val(Value),
    Cell(Rc<VarCell>),
    /// A variable a closure kept from an earlier run captured: its value as that run left it.
    /// It cannot be assigned (the assignment would not last past this run).
    Kept(Value),
}

impl Slot {
    fn get(&self) -> Value {
        match self {
            Slot::Val(v) | Slot::Kept(v) => v.clone(),
            Slot::Cell(c) => c.value.borrow().clone(),
        }
    }
    fn set(&mut self, v: Value) {
        match self {
            Slot::Val(s) => *s = v,
            Slot::Cell(c) => *c.value.borrow_mut() = v,
            Slot::Kept(_) => unreachable!("set_var refuses a kept variable"),
        }
    }
    /// Assigning it would be lost: a kept closure's variable (`Slot::Kept`, or a cell whose value
    /// was frozen when its closure was kept).
    fn kept(&self) -> bool {
        matches!(self, Slot::Kept(_)) || matches!(self, Slot::Cell(c) if c.frozen.get())
    }
    /// The value, leaving undefined (so a uniquely held list or object can be updated in place).
    fn take(&mut self) -> Value {
        match self {
            Slot::Val(s) => std::mem::replace(s, Value::Undef),
            Slot::Cell(c) => std::mem::replace(&mut *c.value.borrow_mut(), Value::Undef),
            Slot::Kept(v) => v.clone(),
        }
    }
}

/// A fresh run id.
fn next_run() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// Unwinds an optional chain whose base was null or undefined (`a?.b.c`).
pub(crate) const SHORT_CIRCUIT: &str = "\u{0}optional-chain";

/// A declared `let`/`const` before its declaration runs (the temporal dead zone): reading it is
/// a ReferenceError. Only ever a variable's value, never an expression's.
pub(crate) fn uninitialized() -> Value {
    thread_local! { static TDZ: Value = Value::Setter(Rc::from("\u{0}uninitialized")) }
    TDZ.with(|v| v.clone())
}

fn is_uninitialized(v: &Value) -> bool {
    matches!(v, Value::Setter(k) if &**k == "\u{0}uninitialized")
}

fn tdz_error<T>(name: &str) -> RResult<T> {
    err(format!("ReferenceError: Cannot access '{name}' before initialization"))
}

impl Interp<'_> {
    /// The string a fragment stands for: text as is, tokens as their token text.
    fn frag_text(&self, f: &[Item]) -> RResult<String> {
        let mut out = String::new();
        for it in f {
            match it {
                Item::Text(t) => out.push_str(t),
                Item::Tok(id) => {
                    if let Some((k, _)) = self
                        .specials
                        .iter()
                        .find(|(_, v)| matches!(v, Value::Int(i) if *i == *id as i64))
                    {
                        out.push_str(k);
                    }
                }
                Item::Tokens(..) => {
                    return err("laid-out app trees cannot be converted to strings; emit app.tree as a child");
                }
                Item::Turns(..) => {
                    return err("turn plans cannot be converted to strings; consume them in a template app");
                }
                _ => {}
            }
        }
        Ok(out)
    }
}

fn has_tokens(l: &[Value]) -> bool {
    l.iter().any(|v| matches!(v, Value::Token(..) | Value::Frag(_)))
}

/// join over items that hold tokens: the result is a token fragment, built with `+`.
fn join_tokens(l: &[Value], sep: &Value) -> RResult<Value> {
    let mut acc = Value::str("");
    for (i, v) in l.iter().enumerate() {
        if i > 0 {
            acc = binop(BinOp::Add, &acc, sep)?;
        }
        acc = binop(BinOp::Add, &acc, v)?;
    }
    Ok(acc)
}

fn err<T>(m: impl Into<String>) -> RResult<T> {
    Err(m.into())
}

impl<'p> Interp<'p> {
    fn finish_document_jobs(&self) -> RResult<()> {
        if self.acting || crate::microtasks::current().is_some() {
            return Ok(());
        }
        if let Some(realm) = &self.realm {
            while realm.borrow_mut().next_task().is_some() {
                crate::microtasks::Queue::native(realm, self.task)?;
            }
        }
        Ok(())
    }
    /// Action forks share immutable values from one render, but keep each
    /// independent state's setter writes and interpreter stacks separate.
    pub fn fork_for_action(&self) -> Self {
        let mut fork = self.clone();
        fork.acting = true;
        fork.ui_updates = Map::default();
        fork.run = next_run();
        fork
    }

    pub fn new(prog: &'p Program, globals: Rc<Map>, specials: Rc<Map>, now: chrono::NaiveDateTime) -> RResult<Self> {
        // A new run: free the cycles earlier runs left.
        crate::value::sweep();
        let mut it = Interp {
            prog,
            temps: vec![],
            realm: crate::microtasks::current(),
            task: if crate::microtasks::current().is_some() {
                crate::microtasks::DOCUMENT
            } else {
                0
            },
            services: None,
            signals: Default::default(),
            globals,
            specials,
            consts: Map::default(),
            scopes: vec![],
            frames: vec![0],
            now,
            steps: 0,
            initializing: 0,
            depth: 0,
            ancestry: vec![],
            descendants: prog.styles.iter().any(|r| !r.sel.ancestors.is_empty()),
            path: vec![],
            sibling_counts: vec![Siblings::default()],
            hook_counts: vec![0],
            handlers: vec![],
            acting: false,
            ui_updates: Map::default(),
            live_ui: None,
            effect_decls: vec![],
            event_decls: vec![],
            callback_decls: vec![],
            template_actions: vec![vec![]],
            mount_decls: vec![],
            ui_seen: Map::default(),
            effects: None,
            consts_ready: false,
            instances: HashMap::new(),
            scope: crate::modules::Instance::new(Rc::new(HashMap::new())),
            encoder: None,
            window: None,
            projection_cache: None,
            line: 0,
            turn_origins: Rc::default(),
            thrown: None,
            thrown_text: String::new(),
            json_path: vec![],
            pending_this: None,
            pending_super: None,
            memo: None,
            memo_counts: HashMap::new(),
            run: next_run(),
        };
        let _ = &mut it;
        Ok(it)
    }

    /// What a handler needs from the render that defined it (the state, consts, the hooks' values
    /// and the handlers), without the interpreter's borrows: an agent keeps it, and runs that
    /// render's handlers later without rendering again (`from_snapshot`).
    /// A snapshot is kept past this run (a render kept for dispatch, its handlers, its effects):
    /// what it holds is frozen first.
    pub fn snapshot(&self) -> RResult<Snapshot> {
        self.finish_document_jobs()?;
        for v in self
            .handlers
            .iter()
            .chain(self.consts.values())
            .chain(self.ui_seen.values())
        {
            crate::value::freeze(v)
        }
        for (_, deps, setup) in &self.effect_decls {
            crate::value::freeze(deps);
            crate::value::freeze(setup)
        }
        Ok(Snapshot {
            globals: self.globals.clone(),
            specials: self.specials.clone(),
            consts: self.consts.clone(),
            now: self.now,
            path: self.path.clone(),
            handlers: self.handlers.clone(),
            ui_seen: self.ui_seen.clone(),
            consts_ready: self.consts_ready,
            instances: self.instances.clone(),
            scope: self.scope.clone(),
            turn_origins: self.turn_origins.clone(),
        })
    }

    /// An interpreter as it was at the end of a render (`snapshot`), ready to run its handlers.
    pub fn from_snapshot(prog: &'p Program, s: &Snapshot) -> Self {
        let mut it =
            Interp::new(prog, s.globals.clone(), s.specials.clone(), s.now).expect("Interp::new does not fail");
        it.consts = s.consts.clone();
        it.path = s.path.clone();
        it.handlers = s.handlers.clone();
        it.ui_seen = s.ui_seen.clone();
        it.consts_ready = s.consts_ready;
        it.instances = s.instances.clone();
        it.scope = s.scope.clone();
        it.turn_origins = s.turn_origins.clone();
        it
    }

    /// Window drivers have separate local hook state while sharing explicit app fields.
    pub fn bind_hook_namespace(&mut self, name: &str) {
        self.path.push(name.to_owned());
    }

    /// Define a program-wide constant (e.g. the `tok` namespace of the bound tokenizer).
    pub fn set_const(&mut self, name: &str, v: Value) {
        self.consts.insert(Rc::from(name), v);
    }

    // Direct context fields are observed individually. Any other use of the context variable
    // escapes the whole object and conservatively observes every field.
    fn receiver(&mut self, expression: &Expr) -> RResult<Value> {
        if let Expr::Var(name) = expression
            && &**name == "harness"
            && self.platform_binding("harness")
        {
            let value = self.lookup(name);
            if !matches!(value, Value::Undef) {
                return Ok(value);
            }
        }
        self.eval(expression)
    }

    /// Keep `memoScan`s in `store` between renders (see `memo.rs`).
    pub fn bind_memo(&mut self, store: Rc<std::cell::RefCell<crate::memo::MemoStore>>) {
        self.memo = Some(store);
    }

    pub fn bind_encoder(&mut self, enc: &'p crate::emit::Encoder) {
        self.encoder = Some(enc);
    }
    /// The model prompts are laid out for (ad-hoc `model(<View/>)` renders use it with their
    /// own `max_new`).
    pub fn bind_window(&mut self, window: Option<crate::emit::Window>) {
        self.window = window;
    }

    pub fn bind_projection_cache(&mut self, cache: &'p std::cell::RefCell<crate::emit::RunCache>, enabled: bool) {
        self.projection_cache = Some((cache, enabled));
    }

    /// Evaluate the program's constants (after `set_const` has added `tok` etc.).
    fn init_consts(&mut self) -> RResult<()> {
        if self.consts_ready {
            return Ok(());
        }
        self.consts_ready = true;
        for (id, module) in &self.prog.modules {
            self.instances
                .insert(id.clone(), crate::modules::Instance::new(module.types.clone()));
        }
        for (id, module) in &self.prog.modules {
            let instance = &self.instances[id];
            let mut bindings = instance.bindings.borrow_mut();
            for (local, target) in &module.bindings {
                let cell = if target.module == *id {
                    match self.prog.funcs.get(target) {
                        Some(f) => crate::modules::Cell::Function(f.clone()),
                        None => crate::modules::Cell::Uninitialized,
                    }
                } else {
                    crate::modules::Cell::Alias(self.instances[&target.module].clone(), target.name.clone())
                };
                bindings.insert(local.clone(), cell);
            }
        }
        for (binding, expression) in &self.prog.consts {
            self.initialize_value(
                self.instances[&binding.module].clone(),
                binding.name.clone(),
                expression,
            )?;
        }
        self.scope = self.instances[&self.prog.root].clone();
        Ok(())
    }

    pub fn render(&mut self, out: &mut dyn Out) -> RResult<()> {
        let (root, tail) = self.evaluate()?;
        emit_child(&root, out)?;
        for spec in tail {
            out.open(spec);
            out.close()?;
        }
        Ok(())
    }

    /// The document, evaluated but not laid out: its root value, and the zero-token spans after
    /// it (`useEvent` listeners, `<Mount>` callbacks). Layout replays them into an output.
    pub fn evaluate(&mut self) -> RResult<(Value, Vec<Rc<SpanSpec>>)> {
        self.init_consts()?;
        // The view is a function of state; the state is its props object.
        let f = self
            .module_function(&self.prog.entry)
            .ok_or_else(|| format!("no function `{}`", self.prog.entry))?;
        // The props object the app sees: the runtime's state, frozen (the runtime keeps it).
        let props = Value::Map(Rc::new(crate::value::Obj::frozen((*self.globals).clone())));
        let v = self.invoke(&f, vec![props])?;
        let mut tail = vec![];
        // `useEvent` listeners: zero-token spans, as `<aff event=name onCallback>` lays out.
        for (event, f) in std::mem::take(&mut self.event_decls) {
            let handler = self.handlers.len();
            self.handlers.push(f.clone());
            let names = |ps: &[crate::ast::Param]| {
                ps.iter()
                    .map(|p| serde_json::Value::from(p.name.to_string()))
                    .collect::<Vec<_>>()
            };
            let (params, object_param) = match &f {
                Value::Lambda(l, _) => (
                    l.props.as_deref().map_or_else(|| names(&l.params), names),
                    l.props.is_none() && !l.params.is_empty(),
                ),
                Value::Func(g) => (
                    g.props.as_deref().map_or_else(|| names(&g.params), names),
                    g.props.is_none() && !g.params.is_empty(),
                ),
                _ => (vec![], false),
            };
            let meta = serde_json::json!({"name": "callback", "target": "", "params": params, "required_params": [], "object_param": object_param, "handler": handler, "props": {"event": event}});
            tail.push(Rc::new(SpanSpec {
                label: Rc::from("callback"),
                kind: SpanKind::Affordance,
                meta,
            }));
        }
        tail.extend(std::mem::take(&mut self.callback_decls));
        // A template app's screen actions when the entry function is the template app.
        let mut carried = std::collections::HashSet::new();
        crate::turns::handlers_in(&v, &mut carried);
        for s in std::mem::take(&mut self.template_actions[0]) {
            if carried.insert(s.meta["handler"].as_u64().unwrap_or(u64::MAX)) {
                tail.push(Rc::new(SpanSpec {
                    label: Rc::from(s.label.as_str()),
                    kind: SpanKind::Affordance,
                    meta: s.meta,
                }));
            }
        }
        Ok((v, tail))
    }

    // ---------------- scopes ----------------

    fn lookup(&self, name: &str) -> Value {
        let base = *self.frames.last().unwrap();
        for (n, v) in self.scopes[base..].iter().rev() {
            if &**n == name {
                return v.get();
            }
        }
        if self.scope.contains(name) {
            return self.scope.get(name);
        }
        if let Some(value) = self.consts.get(name) {
            return value.clone();
        }
        // A state key is a global, but data never replaces the language's own names (`Math`,
        // `JSON`): state `{"JSON": 5}` must not change what code means.
        if let Some(v) = self.globals.get(name)
            && matches!(intrinsic(name), Value::Undef)
        {
            return v.clone();
        }
        self.prog
            .globals
            .get(name)
            .map_or_else(|| intrinsic(name), |key| self.binding_value(key))
    }

    /// `name` is only a state key: no code declares it. Calling it calls the platform function of
    /// that name (`model`, `dispatch`, `useState`), so a prop named `model` cannot disable `model()`.
    fn only_state_key(&self, name: &str) -> bool {
        self.globals.contains_key(name)
            && self.local(name).is_none()
            && !self.scope.contains(name)
            && !self.prog.globals.contains_key(name)
            && !self.consts.contains_key(name)
    }

    fn platform_binding(&self, name: &str) -> bool {
        self.local(name).is_none() && !self.scope.contains(name)
    }

    fn binding_value(&self, key: &Binding) -> Value {
        self.instances
            .get(&key.module)
            .map_or(Value::Undef, |module| module.get(&key.name))
    }

    fn module_function(&self, name: &str) -> Option<Rc<Function>> {
        match self.lookup(name) {
            Value::Func(f) => Some(f),
            _ => None,
        }
    }

    fn eval_list_literal(&mut self, items: &[Expr]) -> RResult<Value> {
        let mut v = Vec::with_capacity(items.len());
        for i in items {
            match i {
                // A spread is one step: its items count against the bulk array limit.
                Expr::Spread(x) => match self.eval(x)? {
                    Value::Str(s) => {
                        crate::jsvalue::bulk_array(v.len() + s.char_count())?;
                        v.extend(s.chars().map(|c| Value::string(c.to_string())))
                    }
                    other => {
                        let items = iterate(&other)?;
                        crate::jsvalue::bulk_array(v.len() + items.len())?;
                        v.extend(items)
                    }
                },
                i => v.push(self.eval(i)?),
            }
        }
        Ok(Value::list(v))
    }

    /// A local variable's value, taken out of its slot (left undefined), so its list can be
    /// updated in place when nothing else holds it.
    fn take_var(&mut self, name: &Sym) -> Option<Value> {
        let base = *self.frames.last().unwrap();
        self.scopes[base..]
            .iter_mut()
            .rev()
            .find(|(n, _)| n == name)
            .map(|(_, slot)| slot.take())
    }

    fn set_var(&mut self, name: &Sym, value: Value) -> RResult<()> {
        if is_uninitialized(&self.lookup(name)) {
            return tdz_error(name);
        }
        self.init_var(name, value)
    }

    fn init_var(&mut self, name: &Sym, v: Value) -> RResult<()> {
        let base = *self.frames.last().unwrap();
        for (n, slot) in self.scopes[base..].iter_mut().rev() {
            if n == name {
                if let Slot::Cell(c) = slot {
                    crate::value::note_cell_write(c, &v)
                }
                if slot.kept() {
                    return err(format!(
                        "TypeError: Assignment to `{name}`, a variable of a closure kept from an earlier run (a render's handler or effect): the assignment would not last past this run; keep it in state (useState)"
                    ));
                }
                slot.set(v);
                return Ok(());
            }
        }
        if self.scope.contains(name) {
            return err(format!(
                "TypeError: Assignment to read-only module binding `{name}`; keep mutable app data in useState"
            ));
        }
        err(format!(
            "ReferenceError: {name} is not defined (assignment to an undeclared variable; declare it with let)"
        ))
    }

    /// A local variable of this name is in scope.
    fn is_declared(&self, name: &str) -> bool {
        let base = *self.frames.last().unwrap();
        self.scopes[base..].iter().any(|(n, _)| &**n == name)
            || self.scope.contains(name)
            || self.prog.globals.contains_key(name)
            || self.consts.contains_key(name)
            || self.globals.contains_key(name)
            || name == "undefined"
            || !matches!(intrinsic(name), Value::Undef)
    }

    fn assign(&mut self, t: &Target, value: Value) -> RResult<()> {
        let reference = self.reference(t)?;
        self.write_reference(&reference, value)
    }

    // ---------------- expressions ----------------

    pub fn eval(&mut self, e: &Expr) -> RResult<Value> {
        stacker::maybe_grow(128 * 1024, 2 * 1024 * 1024, || self.eval_inner(e))
    }

    fn eval_inner(&mut self, e: &Expr) -> RResult<Value> {
        Ok(match e {
            // A global regex literal makes a new RegExp each time it is evaluated (its `lastIndex`
            // is its own).
            Expr::Lit(Value::Regex(r)) if r.global => Value::Regex(Rc::new(r.fresh())),
            Expr::Lit(v) => v.clone(),
            Expr::Temp(r) => self.temps[*r].clone(),
            Expr::Var(n) => match self.lookup(n) {
                Value::Undef => match self.scope.unavailable(n) {
                    Some(e) => return err(format!("{e} (needed by const {n})")),
                    None => match crate::migrate::name(n) {
                        Some(js) => return err(format!("ReferenceError: {n} is not defined (Python; use {js})")),
                        // A name declared nowhere (not a local, a module const or function, nor a
                        // key of the state) is a ReferenceError, as in JavaScript.
                        None if !self.is_declared(n) => {
                            return err(match crate::migrate::platform(n) {
                                Some(hint) => format!("ReferenceError: {n} is not defined ({hint})"),
                                None => format!("ReferenceError: {n} is not defined"),
                            });
                        }
                        None => Value::Undef,
                    },
                },
                v if is_uninitialized(&v) => return tdz_error(n),
                v => {
                    crate::observe::escape(&v);
                    v
                }
            },
            Expr::List(items) => self.eval_list_literal(items)?,
            Expr::Map(items) => {
                let mut m = Map::default();
                for (k, v) in items {
                    if let Expr::Spread(x) = k {
                        match self.eval(x)? {
                            Value::Map(o) => {
                                for (k, v) in o.read().iter() {
                                    crate::jsvalue::obj_insert(&mut m, k.clone(), v.clone());
                                }
                            }
                            // An array's items and a string's characters spread under their indexes.
                            v @ (Value::List(_) | Value::Str(_)) => {
                                for (i, x) in iterate(&v)?.into_iter().enumerate() {
                                    crate::jsvalue::obj_insert(&mut m, Rc::from(i.to_string()), x);
                                }
                            }
                            // Primitives have no own properties to spread.
                            Value::None
                            | Value::Undef
                            | Value::Bool(_)
                            | Value::Int(_)
                            | Value::Float(_)
                            | Value::BigInt(_) => {}
                            other => return err(format!("cannot spread a {} into an object", other.type_name())),
                        }
                        continue;
                    }
                    let k = self.eval(k)?;
                    let k: Rc<str> = match k {
                        Value::Str(s) => s.to_rc(),
                        other => Rc::from(crate::jsvalue::to_string(&self.primitive(&other, true)?)),
                    };
                    let v = self.eval(v)?;
                    crate::jsvalue::obj_insert(&mut m, k, v);
                }
                Value::map(m)
            }
            Expr::Private(b, name) => {
                let recv = self.eval(b)?;
                let owner = self.private_owner(name)?;
                self.get_private(&recv, &owner, name)?
            }
            Expr::PrivateMethod(b, name, args) => {
                let recv = self.eval(b)?;
                let owner = self.private_owner(name)?;
                let function = self.get_private(&recv, &owner, name)?;
                let args = self.eval_args(args)?;
                self.pending_this = Some(recv);
                self.call_value(&function, args)?
            }
            Expr::Attr(b, n) => {
                let v = self.receiver(b)?;
                let r = self.get_prop(&v, n)?;
                if matches!(r, Value::Undef)
                    && matches!(&**b, Expr::Var(t) if &**t == "tok")
                    && self.platform_binding("tok")
                {
                    return err(format!("this tokenizer has no special token tok.{}", n));
                }
                r
            }
            Expr::Index(b, i) => {
                let v = self.receiver(b)?;
                let i = self.eval(i)?;
                self.get_dynamic(&v, &i)?
            }
            Expr::Call(name, args) => self.call(name, args)?,
            Expr::Await(x) => {
                if self.initializing > 0 {
                    return err("module initializers cannot await");
                }

                let value = self.eval(x)?;
                self.resolve_promise(&value)?
            }
            Expr::Method(b, name, args)
                if matches!(b.as_ref(), Expr::Var(v) if v.as_ref() == "harness")
                    && self.platform_binding("harness") =>
            {
                let pos = self.eval_args(args)?;
                self.harness_method(name, pos)?
            }
            // `harness.store.put(..)`, `harness.machine.call(..)`: a namespaced harness API.
            // `harness.machine.get(role)` is not an effect: it names one of this instance's machines.
            Expr::Method(b, name, args)
                if matches!(b.as_ref(), Expr::Attr(h, ns) if matches!(&**ns, "store" | "machine" | "signal") && matches!(h.as_ref(), Expr::Var(v) if v.as_ref() == "harness"))
                    && self.platform_binding("harness") =>
            {
                let Expr::Attr(_, ns) = b.as_ref() else { unreachable!() };
                let pos = self.eval_args(args)?;
                if &**ns == "machine" && &**name == "get" {
                    let role = pos
                        .first()
                        .and_then(Value::as_str)
                        .ok_or("harness.machine.get expects a machine role")?;
                    let mut m = Map::default();
                    m.insert(Rc::from("__machine"), Value::str(role));
                    Value::map(m)
                } else if &**ns == "machine" && &**name == "call" {
                    self.machine_call(Value::str(""), pos)?
                } else {
                    self.harness_method(&format!("{ns}.{name}"), pos)?
                }
            }
            Expr::Method(b, name, args) if &**name == "call" => {
                let recv = self.eval(b)?;
                let pos = self.eval_args(args)?;
                match &recv {
                    Value::Map(m) if m.read().contains_key("__machine") => {
                        let role = m.read()["__machine"].clone();
                        self.machine_call(role, pos)?
                    }
                    _ => self.method(&recv, name, pos)?,
                }
            }
            // `super.m(..)`: the parent class's method, on this object.
            Expr::Method(b, name, args) if matches!(b.as_ref(), Expr::Var(v) if &**v == "super") => {
                let pos = self.eval_args(args)?;
                self.super_method(name, pos)?
            }
            // Array(n).fill is the supported dense-array construction. Recognize it only
            // when the receiver's binding is the actual Array intrinsic.
            Expr::Method(b, name, args)
                if &**name == "fill"
                    && matches!(b.as_ref(), Expr::Call(n, a) | Expr::New(n, a) if a.len() == 1 && matches!(self.lookup(n), Value::Native(ref k) if &**k == "Array")) =>
            {
                let (Expr::Call(_, count) | Expr::New(_, count)) = b.as_ref() else {
                    unreachable!()
                };
                let mut pos = self.eval_args(count)?;
                pos.extend(self.eval_args(args)?);
                self.builtin("__arrayFill", pos)?
            }
            Expr::Method(b, name, args) => {
                let recv = self.eval(b)?;
                let function = self.get_prop(&recv, name)?;
                let pos = self.eval_args(args)?;
                if !function.is_callable() {
                    return err(not_a_function(
                        if matches!(recv, Value::List(_)) {
                            "array"
                        } else {
                            crate::jsvalue::type_of(&recv)
                        },
                        name,
                    ));
                }
                self.call_with_receiver(&function, recv, pos)?
            }
            Expr::Class(def) => self.eval_class(def, None)?,
            Expr::New(name, args) => {
                let (constructor, _) = self.eval_callee(&Expr::Var(name.clone()))?;
                let pos = self.eval_args(args)?;
                self.construct_value(&constructor, pos)?
            }
            Expr::Neg(x) => match self.eval(x)? {
                Value::Int(i) if i != 0 => Value::Int(-i),
                v => match self.primitive(&v, false)? {
                    Value::Int(i) if i != 0 => Value::Int(-i),
                    v => crate::jsvalue::number(-crate::jsvalue::to_number(&v)),
                },
            },
            Expr::Not(x) => Value::Bool(!crate::jsvalue::truthy(&self.eval(x)?)),
            Expr::Truthy(x) => Value::Bool(crate::jsvalue::truthy(&self.eval(x)?)),
            Expr::And(a, b) => {
                let a = self.eval(a)?;
                if !crate::jsvalue::truthy(&a) { a } else { self.eval(b)? }
            }
            Expr::Or(a, b) => {
                let a = self.eval(a)?;
                if crate::jsvalue::truthy(&a) { a } else { self.eval(b)? }
            }
            Expr::ToString(x) => match self.eval(x)? {
                // Fragments keep their meaning (a plain template literal with one is an error).
                v @ (Value::Frag(_) | Value::Token(..)) => v,
                v => Value::string(crate::jsvalue::try_to_string(&self.primitive(&v, true)?)?),
            },
            Expr::Keys(x) => Value::list(crate::jsvalue::keys(&self.eval(x)?)),
            Expr::Spread(_) => return err("`...` spreads only inside an array or object literal"),
            // An optional link whose base is null or undefined ends its whole chain (`Expr::Chain`).
            Expr::OptAttr(b, n) => match self.eval(b)? {
                Value::None | Value::Undef => return err(SHORT_CIRCUIT),
                v => self.get_prop(&v, n)?,
            },
            Expr::OptMethod(b, name, args) => match self.eval(b)? {
                Value::None | Value::Undef => return err(SHORT_CIRCUIT),
                recv => {
                    let function = self.get_prop(&recv, name)?;
                    let pos = self.eval_args(args)?;
                    self.call_with_receiver(&function, recv, pos)?
                }
            },
            Expr::OptIndex(b, i) => match self.eval(b)? {
                Value::None | Value::Undef => return err(SHORT_CIRCUIT),
                v => {
                    let i = self.eval(i)?;
                    self.get_dynamic(&v, &i)?
                }
            },
            Expr::OptApply(f, args) => {
                let (f, receiver) = self.eval_callee(f)?;
                if matches!(f, Value::None | Value::Undef) {
                    return err(SHORT_CIRCUIT);
                }
                let pos = self.eval_args(args)?;
                self.pending_this = receiver;
                self.call_value(&f, pos)?
            }
            Expr::Chain(x) => match self.eval(x) {
                Err(e) if e == SHORT_CIRCUIT => Value::Undef,
                r => r?,
            },
            Expr::Apply(f, args) => {
                let (f, receiver) = self.eval_callee(f)?;
                let pos = self.eval_args(args)?;
                self.pending_this = receiver;
                self.call_value(&f, pos)?
            }
            Expr::Unary(op, x) => {
                let v = self.eval(x)?;
                let v = if matches!(op, UnaryOp::Void) {
                    v
                } else {
                    self.primitive(&v, false)?
                };
                match op {
                    UnaryOp::Plus => crate::jsvalue::number(crate::jsvalue::to_number(&v)),
                    UnaryOp::BitNot => Value::Int(!crate::jsvalue::to_int32(&v) as i64),
                    UnaryOp::Void => Value::Undef,
                }
            }
            Expr::Comma(a, b) => {
                self.eval(a)?;
                self.eval(b)?
            }
            Expr::Assign(t, op, v) => self.eval_assign(t, *op, v)?,
            Expr::Update(t, delta, prefix) => {
                let reference = self.reference(t)?;
                let old = self.read_reference(&reference)?;
                let old = crate::jsvalue::number(crate::jsvalue::to_number(&self.primitive(&old, false)?));
                let new = crate::jsvalue::add(&old, &Value::Int(*delta as i64))?;
                self.write_reference(&reference, new.clone())?;
                if *prefix { new } else { old }
            }
            Expr::Mutate(t, name, args) => self.eval_mutate(t, name, args)?,
            Expr::Typeof(x) => {
                // `typeof undeclared` is "undefined", not an error.
                let v = match &**x {
                    Expr::Var(n) => match self.lookup(n) {
                        v if is_uninitialized(&v) => return tdz_error(n),
                        v => v,
                    },
                    e => self.eval(e)?,
                };
                Value::str(crate::jsvalue::type_of(&v))
            }
            Expr::Nullish(a, b) => {
                let a = self.eval(a)?;
                if matches!(a, Value::Undef | Value::None) {
                    self.eval(b)?
                } else {
                    a
                }
            }
            Expr::Cond(c, a, b) => {
                if crate::jsvalue::truthy(&self.eval(c)?) {
                    self.eval(a)?
                } else {
                    self.eval(b)?
                }
            }
            Expr::Bin(op, a, b) => {
                let a = self.eval(a)?;
                let b = self.eval(b)?;
                self.bin(*op, &a, &b)?
            }
            Expr::Lambda(l) => {
                // A lexical closure: it shares the enclosing variables it names (each becomes a
                // cell, shared by this scope and every closure that captures it).
                let base = *self.frames.last().unwrap();
                let mut env = Vec::with_capacity(l.captures.len());
                for name in &l.captures {
                    if let Some(i) = self.scopes[base..].iter().rposition(|(n, _)| n == name) {
                        let slot = &mut self.scopes[base + i].1;
                        let cell = match slot {
                            Slot::Cell(c) if c.run == self.run => c.clone(),
                            _ => {
                                let (v, kept) = match slot {
                                    Slot::Val(v) => (std::mem::replace(v, Value::Undef), false),
                                    Slot::Cell(c) => (c.value.borrow().clone(), false),
                                    Slot::Kept(v) => (v.clone(), true),
                                };
                                let c = Rc::new(VarCell::new(self.run, v));
                                // A kept variable stays unassignable through the new closure too.
                                if kept {
                                    c.frozen.set(true)
                                }
                                *slot = Slot::Cell(c.clone());
                                c
                            }
                        };
                        env.push((name.clone(), cell));
                    }
                }
                Value::Lambda(
                    l.clone(),
                    Rc::new(Env {
                        locals: env,
                        module: self.scope.clone(),
                    }),
                )
            }
            Expr::Markup(nodes) => {
                let mut cap = Capture::default();
                match self.nodes(nodes, &mut cap)? {
                    Flow::Normal => {}
                    _ => return err("break/continue/return inside JSX"),
                }
                Value::Frag(Rc::new(cap.items))
            }
            Expr::Template { tagged, parts } => {
                if *tagged {
                    let mut cap = Capture::default();
                    for p in parts {
                        match p {
                            TplPart::Lit(_, nodes) => {
                                self.nodes(nodes, &mut cap)?;
                            }
                            TplPart::Expr(e) => {
                                let v = self.eval(e)?;
                                emit_child(&v, &mut cap)?;
                            }
                        }
                    }
                    Value::Frag(Rc::new(cap.items))
                } else {
                    let mut s = crate::jsvalue::StrBuf::default();
                    for p in parts {
                        match p {
                            TplPart::Lit(t, _) => s.append(t)?,
                            TplPart::Expr(e) => {
                                let v = self.eval(e)?;
                                s.append(&crate::jsvalue::try_to_string(&self.primitive(&v, true)?)?)?
                            }
                        }
                    }
                    Value::string(s.into_string())
                }
            }
        })
    }

    fn eval_args(&mut self, args: &[Expr]) -> RResult<Vec<Value>> {
        let mut v = Vec::with_capacity(args.len());
        for a in args {
            match a {
                Expr::Spread(x) => {
                    let items = iterate(&self.eval(x)?)?;
                    crate::jsvalue::bulk_array(v.len() + items.len())?;
                    v.extend(items)
                }
                a => v.push(self.eval(a)?),
            }
        }
        Ok(v)
    }

    pub fn call_value(&mut self, f: &Value, args: Vec<Value>) -> RResult<Value> {
        self.call_value_with(f, args, Call::Normal)
    }

    /// Run an action handler. The handler body is the root effect scope even when it is
    /// declared `async`: its result is the patch, not a promise.
    pub fn call_handler(&mut self, f: &Value, args: Vec<Value>) -> RResult<Value> {
        if !self.async_active() {
            return self.call_value_with(f, args, Call::Body(None));
        }
        let realm = self.realm();
        let target = realm.borrow().groups[&self.task].root.clone();
        let result = self.call_value_with(f, args, Call::Body(Some(target.clone())));
        self.complete_call(&target, result);
        Ok(Value::Promise(target))
    }

    pub(crate) fn call_with_receiver(&mut self, function: &Value, receiver: Value, args: Vec<Value>) -> RResult<Value> {
        self.pending_this = Some(receiver);
        self.call_value(function, args)
    }

    fn call_value_with(&mut self, f: &Value, args: Vec<Value>, mode: Call) -> RResult<Value> {
        let result = stacker::maybe_grow(128 * 1024, 2 * 1024 * 1024, || self.call_value_inner(f, args, mode));
        self.pending_this = None;
        self.pending_super = None;
        result
    }

    fn call_value_inner(&mut self, f: &Value, args: Vec<Value>, mode: Call) -> RResult<Value> {
        if self.initializing > 0
            && (matches!(f.declaration(),Value::Func(f) if f.is_async)
                || matches!(f.declaration(),Value::Lambda(f,_) if f.is_async))
        {
            return err(
                "module initializers cannot call async functions; call them from a component effect or handler",
            );
        }
        if let Value::Native(name) = f {
            if let Some(owner) = name.owner {
                let receiver = self.pending_this.take().unwrap_or(Value::Undef);
                self.pending_super = None;
                return self.intrinsic_method(owner, &receiver, &name.name, args);
            }
            if name.is_namespace() {
                return err(format!("TypeError: {name} is not a function"));
            }
            if name.starts_with("Promise.")
                && !matches!(&self.pending_this, Some(Value::Native(receiver)) if &**receiver == "Promise")
            {
                return err("TypeError: Promise static methods require the Promise receiver");
            }
            self.pending_this = None;
            self.pending_super = None;
            return self.builtin(name, args);
        }
        // Only a function takes a receiver: anything else leaves none behind for the next call.
        if !f.is_callable() {
            self.pending_this = None;
            self.pending_super = None;
        }
        if let Value::Class(c) = f {
            return err(format!(
                "TypeError: Class constructor {} cannot be invoked without 'new'",
                c.name
            ));
        }
        match f {
            Value::Method(method) => {
                self.pending_super = Some(method.home.clone());
                self.call_value_with(&method.function, args, mode)
            }
            Value::Bound(b) => {
                let (target, this, pre) = &**b;
                let mut all = pre.clone();
                all.extend(args);
                self.pending_this = Some(this.clone());
                self.call_value_with(target, all, mode)
            }
            Value::Lambda(l, _) if matches!(&mode, Call::Normal) && l.is_async && self.async_active() => {
                self.run_async(|it, entry| it.call_value_with(f, args, Call::Body(Some(entry))))
            }
            Value::Lambda(l, env) => {
                let this = self.pending_this.take();
                let home = self.pending_super.take();
                self.spend()?;
                if self.depth >= 400 {
                    return err("RangeError: Maximum call stack size exceeded (recursion deeper than 400 calls)");
                }
                self.depth += 1;
                let caller_scope = std::mem::replace(&mut self.scope, env.module.clone());
                let base = self.scopes.len();
                self.frames.push(base);
                // The captured variables: shared within this run; a closure from an earlier run
                // (a render's handler) starts from a copy of their values.
                let run = self.run;
                self.scopes.extend(env.iter().map(|(n, c)| {
                    (
                        n.clone(),
                        if c.run == run {
                            Slot::Cell(c.clone())
                        } else {
                            Slot::Kept(c.value.borrow().clone())
                        },
                    )
                }));
                if let Some(n) = &l.self_name {
                    self.scopes.push((n.clone(), Slot::Val(f.clone())));
                }
                if let Some(owner) = &home {
                    let mut chain = match self.local("\0private.context") {
                        Some(Value::List(v)) => v.read().clone(),
                        _ => vec![],
                    };
                    chain.insert(0, Value::Class(owner.clone()));
                    self.scopes
                        .push((Rc::from("\0private.context"), Slot::Val(Value::list(chain))));
                }
                if !l.arrow && l.uses_this {
                    self.scopes
                        .push((Rc::from("this"), Slot::Val(this.unwrap_or(Value::Undef))));
                }
                if !l.arrow && l.uses_super {
                    self.scopes
                        .push((Rc::from("super"), Slot::Val(home.map_or(Value::Undef, Value::Class))));
                }
                let bound = match &l.props {
                    Some(pp) => self.bind_props(pp, args.first().cloned().unwrap_or(Value::Undef)),
                    None => self.bind_params(&l.params, args),
                };
                if let Err(e) = bound {
                    self.scope = caller_scope;
                    self.scopes.truncate(base);
                    self.frames.pop();
                    self.depth -= 1;
                    return Err(e);
                }
                let caller_line = self.line;
                let r = match &l.body {
                    LambdaBody::Expr(e) => {
                        let code = l.code.get_or_init(|| crate::bytecode::Code::expression(e)).clone();
                        self.exec_code(code, mode.entry()).map(|f| match f {
                            Flow::Return(v) => v,
                            _ => Value::Undef,
                        })
                    }
                    LambdaBody::Block(b) => self
                        .exec_code(
                            l.code.get_or_init(|| crate::bytecode::Code::block(b)).clone(),
                            mode.entry(),
                        )
                        .map(|f| match f {
                            Flow::Return(v) => v,
                            _ => Value::Undef,
                        }),
                };
                // An error keeps the lambda's line: the enclosing function's frame reports it.
                if r.is_ok() {
                    self.line = caller_line;
                }
                self.scopes.truncate(base);
                self.frames.pop();
                self.depth -= 1;
                self.scope = caller_scope;
                r
            }
            Value::Func(f) => {
                let f = f.clone();
                match mode {
                    Call::Normal => self.invoke(&f, args),
                    Call::Body(entry) => self.invoke_body(&f, args, entry),
                }
            }
            // A callback prop from the parent instance (`<Mount onX={...}>`): calling it is an
            // event into the parent, which runs its handler.
            Value::Map(m) if m.read().contains_key("__parent") => {
                let name = m.read()["__parent"].clone();
                self.harness_method("parent", vec![name, Value::list(args)])
            }
            Value::Setter(key) => {
                self.pending_this = None;
                self.pending_super = None;
                if !self.acting {
                    return err("setState called during render (state changes only inside an action handler)");
                }
                let arg = args.into_iter().next().unwrap_or(Value::Undef);
                if let Some(live) = self.live_ui.clone() {
                    if !live.borrow().contains_key(key) {
                        return Ok(Value::Undef);
                    }
                    let cur = live
                        .borrow()
                        .get(&**key)
                        .cloned()
                        .or_else(|| self.ui_seen.get(&**key).cloned())
                        .or_else(|| self.ui_value(key))
                        .unwrap_or(Value::Undef);
                    let new = if arg.is_callable() {
                        self.call_value(&arg, vec![cur])?
                    } else {
                        arg
                    };
                    crate::value::freeze_state(&new)?;
                    live.borrow_mut().insert(key.clone(), new);
                    return Ok(Value::Undef);
                }
                let cur = self
                    .ui_updates
                    .get(&**key)
                    .cloned()
                    .or_else(|| self.ui_seen.get(&**key).cloned())
                    .or_else(|| self.ui_value(key))
                    .unwrap_or(Value::Undef);
                let new = if arg.is_callable() {
                    self.call_value(&arg, vec![cur])?
                } else {
                    arg
                };
                crate::value::freeze_state(&new)?;
                self.ui_updates.insert(key.clone(), new);
                Ok(Value::Undef)
            }
            other => err(format!("TypeError: '{}' is not callable", other.type_name())),
        }
    }

    /// Local hook/viewport state first used in this render: the instance keeps
    /// them from now on (state is fixed at mount, as in React), so `useState(prop)` does not follow
    /// the prop.
    pub fn mounted_hooks(&self) -> RResult<Vec<(String, Value)>> {
        self.finish_document_jobs()?;
        // The values themselves (frozen when used): a `useState(new Map())` mounts a Map.
        Ok(self
            .ui_seen
            .iter()
            .filter(|(k, _)| self.ui_value(k).is_none())
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect())
    }

    fn ui_value(&self, key: &str) -> Option<Value> {
        match self.globals.get("__ui") {
            Some(Value::Map(m)) => m.read().get(key).cloned(),
            _ => None,
        }
    }

    /// `useState(init)`: component-local state stored in the instance state's `__ui` map, keyed by
    /// the component path and hook order. `useState(init, "name")`: a named field of the instance's
    /// state (`@name`), the same for every component that names it and stable across changes to
    /// the component tree. Returns [value, setter].
    fn use_state(&mut self, init: Value, name: Option<&str>) -> RResult<Value> {
        let n = {
            let c = self.hook_counts.last_mut().unwrap();
            *c += 1;
            *c - 1
        };
        let key: Rc<str> = match name {
            Some(name) => Rc::from(format!("@{name}")),
            None => Rc::from(format!("{}/{}", self.path.join("/"), n)),
        };
        // In a task, components read the live hooks (a view evaluated now is current).
        let live = self.live_ui.as_ref().and_then(|l| l.borrow().get(&key).cloned());
        let v = match live
            .or_else(|| self.ui_updates.get(&key).cloned())
            .or_else(|| self.ui_value(&key))
            .or_else(|| self.ui_seen.get(&key).cloned())
        {
            Some(v) => v,
            // The initial value is the hook's from now on: frozen, as state is.
            None => {
                let init = if init.is_callable() {
                    self.call_value(&init, vec![])?
                } else {
                    init
                };
                crate::value::freeze_state(&init)?;
                init
            }
        };
        self.ui_seen.insert(key.clone(), v.clone());
        Ok(Value::list(vec![v, Value::Setter(key)]))
    }

    /// `useRef`: a hook under a `~` key (see `is_ref_key`), read through a native that refuses
    /// renders, written through an ordinary setter.
    fn use_ref(&mut self, init: Value, name: Option<&str>) -> RResult<Value> {
        let n = {
            let c = self.hook_counts.last_mut().unwrap();
            *c += 1;
            *c - 1
        };
        let key: Rc<str> = match name {
            Some(name) => Rc::from(format!("~{name}")),
            None => Rc::from(format!("~{}/{}", self.path.join("/"), n)),
        };
        let known = self.live_ui.as_ref().is_some_and(|l| l.borrow().contains_key(&key))
            || self.ui_updates.contains_key(&key)
            || self.ui_value(&key).is_some()
            || self.ui_seen.contains_key(&key);
        if !known {
            let init = if init.is_callable() {
                self.call_value(&init, vec![])?
            } else {
                init
            };
            crate::value::freeze_state(&init)?;
            self.ui_seen.insert(key.clone(), init);
        } else if !self.ui_seen.contains_key(&key) {
            let v = self.ui_value(&key).unwrap_or(Value::Undef);
            self.ui_seen.insert(key.clone(), v);
        }
        let read = Value::Bound(Rc::new((
            Value::Native(Rc::<str>::from("\0refget").into()),
            Value::Undef,
            vec![Value::Str(key.clone().into())],
        )));
        Ok(Value::list(vec![read, Value::Setter(key)]))
    }

    /// Destructure a props object (`({a, b = 1})`): missing or undefined keys take the default.
    /// Each name is bound before the next default is evaluated (a default sees the earlier ones).
    fn bind_props(&mut self, pp: &[Param], props: Value) -> RResult<()> {
        if matches!(props, Value::Undef | Value::None) && !pp.is_empty() {
            return err(format!(
                "TypeError: Cannot destructure property '{}' of '{}' as it is {}.",
                pp[0].name,
                crate::jsvalue::to_string(&props),
                crate::jsvalue::to_string(&props)
            ));
        }
        for p in pp {
            let v = match &props {
                Value::Map(m) => m.read().get(&*p.name).cloned().unwrap_or(Value::Undef),
                other => crate::jsvalue::index(other, &Value::Str(p.name.clone().into()))?,
            };
            let v = match (v, &p.default) {
                (Value::Undef, Some(d)) => self.eval(d)?,
                (v, _) => v,
            };
            self.scopes.push((p.name.clone(), Slot::Val(v)));
        }
        Ok(())
    }

    /// Call a user function (positional args, or one props object for a component).
    pub fn invoke(&mut self, f: &Rc<Function>, pos: Vec<Value>) -> RResult<Value> {
        if f.is_async && self.async_active() {
            return self.run_async(|it, entry| it.invoke_body(f, pos, Some(entry)));
        }
        self.invoke_body(f, pos, None)
    }

    /// One step of the run's fuel.
    pub(crate) fn spend(&mut self) -> RResult<()> {
        self.steps += 1;
        if self.steps > FUEL {
            return err(format!(
                "out of fuel: more than {FUEL} steps (loop iterations and calls) in one render or handler"
            ));
        }
        Ok(())
    }

    fn invoke_body(
        &mut self,
        f: &Rc<Function>,
        pos: Vec<Value>,
        entry: Option<Rc<crate::effects::Promise>>,
    ) -> RResult<Value> {
        if PROFILE.with(|p| p.borrow().is_some()) {
            let started = std::time::Instant::now();
            let r = stacker::maybe_grow(128 * 1024, 2 * 1024 * 1024, || self.invoke_body_inner(f, pos, entry));
            let ns = started.elapsed().as_nanos() as u64;
            PROFILE.with(|p| {
                if let Some(m) = p.borrow_mut().as_mut() {
                    let e = m.entry(f.name.to_string()).or_default();
                    e.0 += 1;
                    e.1 += ns;
                }
            });
            return r;
        }
        stacker::maybe_grow(128 * 1024, 2 * 1024 * 1024, || self.invoke_body_inner(f, pos, entry))
    }

    fn invoke_body_inner(
        &mut self,
        f: &Rc<Function>,
        pos: Vec<Value>,
        entry: Option<Rc<crate::effects::Promise>>,
    ) -> RResult<Value> {
        let this = self.pending_this.take();
        self.spend()?;
        if self.depth >= 400 {
            return err("RangeError: Maximum call stack size exceeded (recursion deeper than 400 calls)");
        }
        self.depth += 1;
        let caller_scope = std::mem::replace(&mut self.scope, f.module.clone());
        let base = self.scopes.len();
        self.frames.push(base);
        let caller_line = std::mem::replace(&mut self.line, f.loc.line);
        if !f.arrow && f.uses_this {
            self.scopes
                .push((Rc::from("this"), Slot::Val(this.unwrap_or(Value::Undef))));
        }
        // Parameters are bound in the callee's scope (a default sees the earlier parameters).
        let bound = match &f.props {
            Some(pp) => self.bind_props(pp, pos.into_iter().next().unwrap_or(Value::Undef)),
            None => self.bind_params(&f.params, pos),
        };
        let r = match bound {
            Ok(()) => self.exec_code(
                f.code.get_or_init(|| crate::bytecode::Code::block(&f.body)).clone(),
                entry,
            ),
            Err(e) => Err(e),
        };
        let r = r.map_err(|e| self.frame_error(e, &f.name, &f.loc.file, self.line, base));
        self.line = caller_line;
        self.scopes.truncate(base);
        self.frames.pop();
        self.depth -= 1;
        self.scope = caller_scope;
        match r? {
            Flow::Return(v) => Ok(v),
            _ => Ok(Value::Undef),
        }
    }

    /// Add one stack frame (`at name (file:line)`) to an error leaving a user function or
    /// component. The first frame whose locals hold a turn map from `templateInput` also names
    /// the TurnStrategy log record being rendered. The message itself stays the first line.
    fn frame_error(&self, mut e: String, name: &str, file: &str, line: u32, base: usize) -> String {
        if e == SUSPEND {
            return e;
        }
        e.push_str(&format!("\n    at {name} ({file}:{line})"));
        if !self.turn_origins.is_empty() && !e.contains("\n    while rendering TurnStrategy log[") {
            let found = self.scopes[base.min(self.scopes.len())..]
                .iter()
                .rev()
                .find_map(|(_, v)| match v.get() {
                    Value::Map(m) => self.turn_origins.get(&(Rc::as_ptr(&m) as usize)),
                    _ => None,
                });
            if let Some((i, kind)) = found {
                e.push_str(&format!("\n    while rendering TurnStrategy log[{i}] (kind {kind:?})"));
            }
        }
        e
    }

    /// Positional parameters; a default applies when the argument is missing or undefined.
    /// Each parameter is bound before the next default is evaluated (a default sees the earlier ones).
    fn bind_params(&mut self, params: &[Param], pos: Vec<Value>) -> RResult<()> {
        let mut pos = pos.into_iter();
        for p in params {
            if p.rest {
                self.scopes
                    .push((p.name.clone(), Slot::Val(Value::list(pos.by_ref().collect()))));
                break;
            }
            let v = match (pos.next(), &p.default) {
                (None | Some(Value::Undef), Some(d)) => self.eval(d)?,
                (v, _) => v.unwrap_or(Value::Undef),
            };
            self.scopes.push((p.name.clone(), Slot::Val(v)));
        }
        Ok(())
    }

    /// A local variable of this name, if one is in scope.
    fn local(&self, name: &str) -> Option<Value> {
        let base = *self.frames.last().unwrap();
        self.scopes[base..]
            .iter()
            .rev()
            .find(|(n, _)| &**n == name)
            .map(|(_, v)| v.get())
    }

    /// JavaScript's ToPrimitive for an object that defines `toString` or `valueOf` itself (or
    /// whose class does): the first of the two that returns a primitive gives the value,
    /// `toString` first where text is wanted and `valueOf` first elsewhere. Any other value, and
    /// an object that defines neither, is returned as it is, and converts as built in.
    pub(crate) fn primitive(&mut self, v: &Value, text: bool) -> RResult<Value> {
        if let Value::List(_) = v {
            // An array's text is its items' texts joined: an item that defines its own is asked.
            return Ok(self.items_asked(v)?.unwrap_or_else(|| v.clone()));
        }
        if !matches!(v, Value::Map(_)) {
            return Ok(v.clone());
        }
        let mut defined = false;
        for name in if text {
            ["toString", "valueOf"]
        } else {
            ["valueOf", "toString"]
        } {
            let method = self.get_prop(v, name)?;
            let own = matches!(
                method,
                Value::Lambda(..) | Value::Func(_) | Value::Method(_) | Value::Bound(_)
            );
            // The built-in `toString` answers with text, so where text is wanted nothing after
            // it is asked.
            if text && name == "toString" && !own {
                break;
            }
            if own {
                defined = true;
                let result = self.call_with_receiver(&method, v.clone(), vec![])?;
                if !matches!(
                    result,
                    Value::Map(_) | Value::List(_) | Value::Coll(_) | Value::Regex(_)
                ) {
                    return Ok(result);
                }
            }
        }
        if defined {
            return err("TypeError: Cannot convert object to primitive value");
        }
        Ok(v.clone())
    }

    /// `list` with each item that defines its own `toString` replaced by that text, at any depth;
    /// `None` if no item does. An array inside itself adds no text, as in JavaScript's join. The
    /// walk keeps its own stack, so an array nested any number of levels deep is safe.
    fn items_asked(&mut self, list: &Value) -> RResult<Option<Value>> {
        struct Frame {
            at: *const (),
            items: std::vec::IntoIter<Value>,
            out: Vec<Value>,
            changed: bool,
        }
        fn frame(list: &Value) -> Option<Frame> {
            let Value::List(l) = list else { return None };
            let items: Vec<Value> = l.read().clone();
            Some(Frame {
                at: Rc::as_ptr(l) as *const (),
                out: Vec::with_capacity(items.len()),
                items: items.into_iter(),
                changed: false,
            })
        }
        let Some(first) = frame(list) else { return Ok(None) };
        let mut open = std::collections::HashSet::from([first.at]);
        let mut stack = vec![first];
        loop {
            let top = stack.last_mut().expect("a frame");
            let Some(item) = top.items.next() else {
                // This array is done: it goes to the one that holds it, changed or as it was.
                let done = stack.pop().expect("a frame");
                open.remove(&done.at);
                let asked = done.changed.then(|| Value::list(done.out));
                let Some(parent) = stack.last_mut() else {
                    return Ok(asked);
                };
                parent.changed |= asked.is_some();
                // The original is the item `parent` just handed out; it kept it in `out`.
                if let Some(asked) = asked {
                    *parent.out.last_mut().expect("the item") = asked;
                }
                continue;
            };
            match &item {
                Value::Map(_) => {
                    let asked = self.primitive(&item, true)?;
                    let top = stack.last_mut().expect("a frame");
                    top.changed |= !matches!(asked, Value::Map(_));
                    top.out.push(asked);
                }
                Value::List(l) if open.contains(&(Rc::as_ptr(l) as *const ())) => {
                    top.changed = true;
                    top.out.push(Value::str(""));
                }
                Value::List(_) => {
                    top.out.push(item.clone());
                    let inner = frame(&item).expect("a list");
                    open.insert(inner.at);
                    stack.push(inner);
                }
                _ => top.out.push(item),
            }
        }
    }

    /// The arguments of a built-in method, with an object where text or a number is expected
    /// asked for it (`primitive`). A string method's arguments are text, except the positions
    /// that are counts or indexes; a number method's are numbers; `join` asks the array's items.
    fn asked(
        &mut self,
        owner: crate::value::Prototype,
        recv: &Value,
        name: &str,
        mut a: Vec<Value>,
    ) -> RResult<(Value, Vec<Value>)> {
        use crate::value::Prototype;
        let mut recv = recv.clone();
        if !a
            .iter()
            .chain([&recv])
            .any(|v| matches!(v, Value::Map(_) | Value::List(_)))
        {
            return Ok((recv, a));
        }
        match owner {
            Prototype::String => {
                for (i, v) in a.iter_mut().enumerate() {
                    let number = match name {
                        "at" | "charAt" | "charCodeAt" | "codePointAt" | "slice" | "substring" | "substr"
                        | "repeat" => true,
                        "padStart" | "padEnd" => i == 0,
                        "indexOf" | "lastIndexOf" | "includes" | "startsWith" | "endsWith" | "split" => i == 1,
                        _ => false,
                    };
                    if matches!(v, Value::Map(_)) || (!number && matches!(v, Value::List(_))) {
                        *v = self.primitive(v, !number)?;
                    }
                }
            }
            Prototype::Number => {
                for v in a.iter_mut().filter(|v| matches!(v, Value::Map(_))) {
                    *v = self.primitive(v, false)?;
                }
            }
            Prototype::Array if matches!(name, "join" | "toString") => {
                recv = self.primitive(&recv, true)?;
                if let Some(separator @ Value::Map(_)) = a.first_mut() {
                    *separator = self.primitive(separator, true)?;
                }
            }
            _ => {}
        }
        Ok((recv, a))
    }

    /// A binary operator on two values. An operator that works on primitives first asks an
    /// object operand for its primitive (`primitive`); `===`, `in` and `instanceof` do not, and
    /// `==` does only against a primitive.
    fn bin(&mut self, op: BinOp, a: &Value, b: &Value) -> RResult<Value> {
        let object = |v: &Value| matches!(v, Value::Map(_) | Value::List(_));
        let (object_a, object_b) = (object(a), object(b));
        if op == BinOp::In && object_a {
            return binop(op, &self.primitive(a, true)?, b);
        }
        let converts = match op {
            BinOp::StrictEq | BinOp::StrictNe | BinOp::In | BinOp::InstanceOf => false,
            BinOp::Eq | BinOp::Ne => {
                let primitive = |v: &Value| {
                    matches!(
                        v,
                        Value::Bool(_) | Value::Int(_) | Value::BigInt(_) | Value::Float(_) | Value::Str(_)
                    )
                };
                (object_a && primitive(b)) || (object_b && primitive(a))
            }
            _ => object_a || object_b,
        };
        if !converts {
            return binop(op, a, b);
        }
        let a = self.primitive(a, false)?;
        let b = self.primitive(b, false)?;
        binop(op, &a, &b)
    }

    /// Call a function value (`f(x)` of a computed `f`): a TypeError if it is not a function.
    fn get_dynamic(&mut self, receiver: &Value, key: &Value) -> RResult<Value> {
        if matches!(receiver, Value::Map(_) | Value::Class(_) | Value::Native(_)) {
            let key = self.primitive(key, true)?;
            return self.get_prop(receiver, &crate::jsvalue::to_string(&key));
        }
        let value = crate::jsvalue::index(receiver, key)?;
        if matches!(value, Value::Undef)
            && let Value::Str(name) = key
        {
            return self.get_prop(receiver, name);
        }
        Ok(value)
    }
    fn eval_callee(&mut self, expression: &Expr) -> RResult<(Value, Option<Value>)> {
        // Lexical platform calls retain their meaning when argument evaluation suspends.
        if let Expr::Attr(base, name) = expression {
            if self.platform_binding("harness") {
                let method = match &**base {
                    Expr::Var(v) if &**v == "harness" => Some(name.to_string()),
                    Expr::Attr(h, ns)
                        if matches!(&**h,Expr::Var(v) if &**v=="harness")
                            && matches!(&**ns, "store" | "machine" | "signal") =>
                    {
                        Some(format!("{ns}.{name}"))
                    }
                    _ => None,
                };
                if let Some(method) = method {
                    return Ok((
                        Value::Native(Rc::<str>::from(format!("\0harness.{method}")).into()),
                        None,
                    ));
                }
            }
            if matches!(&**base,Expr::Var(n) if &**n=="super") {
                return self.super_reference(name).map(|(f, r)| (f, Some(r)));
            }
        }
        if let Expr::Var(name) = expression
            && !self.is_declared(name)
            && matches!(self.lookup(name), Value::Undef)
        {
            let f = Value::Native(name.clone().into());
            return Ok((
                if &**name == "memoScan" {
                    Value::Bound(Rc::new((
                        f,
                        Value::Undef,
                        vec![Value::Int(expression as *const Expr as usize as i64)],
                    )))
                } else {
                    f
                },
                None,
            ));
        }
        match expression {
            Expr::Attr(base, name) | Expr::OptAttr(base, name) => {
                let receiver = self.eval(base)?;
                if matches!(expression, Expr::OptAttr(..)) && matches!(receiver, Value::None | Value::Undef) {
                    return err(SHORT_CIRCUIT);
                };
                let function =
                    if &**name == "call" && matches!(&receiver,Value::Map(m) if m.read().contains_key("__machine")) {
                        Value::Bound(Rc::new((
                            Value::Native(Rc::<str>::from("\0machine.call").into()),
                            Value::Undef,
                            vec![get_attr(&receiver, "__machine")?],
                        )))
                    } else {
                        self.get_prop(&receiver, name)?
                    };
                Ok((function, Some(receiver)))
            }
            Expr::Index(base, key) | Expr::OptIndex(base, key) => {
                let receiver = self.eval(base)?;
                if matches!(expression, Expr::OptIndex(..)) && matches!(receiver, Value::None | Value::Undef) {
                    return err(SHORT_CIRCUIT);
                };
                let key = self.eval(key)?;
                let function = self.get_dynamic(&receiver, &key)?;
                Ok((function, Some(receiver)))
            }
            Expr::Private(base, name) => {
                let receiver = self.eval(base)?;
                let owner = self.private_owner(name)?;
                let function = self.get_private(&receiver, &owner, name)?;
                Ok((function, Some(receiver)))
            }
            _ => Ok((self.eval(expression)?, None)),
        }
    }

    fn call(&mut self, name: &Sym, args: &[Expr]) -> RResult<Value> {
        if &**name == "super" {
            let pos = self.eval_args(args)?;
            return self.super_call(pos);
        }
        // Resolve the reference before arguments can assign a different function to it.
        let local = self.local(name);
        let lv = local.clone().unwrap_or_else(|| self.lookup(name));
        if is_uninitialized(&lv) {
            return tdz_error(name);
        }
        let pos = self.eval_args(args)?;
        // A local function (or variable) shadows a top-level function of the same name.
        if let Some(lv) = local {
            return match lv {
                v if is_uninitialized(&v) => tdz_error(name),
                Value::Method(_)
                | Value::Native(_)
                | Value::Lambda(..)
                | Value::Setter(_)
                | Value::Func(_)
                | Value::Bound(_)
                | Value::Class(_) => self.call_value(&lv, pos),
                Value::Map(ref m) if m.read().contains_key("__parent") => self.call_value(&lv, pos),
                other => err(format!(
                    "TypeError: {name} is not a function (it is {})",
                    crate::jsvalue::type_of(&other)
                )),
            };
        }
        if let Value::Method(_)
        | Value::Native(_)
        | Value::Lambda(..)
        | Value::Setter(_)
        | Value::Func(_)
        | Value::Bound(_)
        | Value::Class(_) = lv
        {
            return self.call_value(&lv, pos);
        }
        if matches!(&lv, Value::Map(m) if m.read().contains_key("__parent")) {
            return self.call_value(&lv, pos);
        }
        if self.is_declared(name) && !self.only_state_key(name) {
            return err(format!("TypeError: {name} is not a function"));
        }
        // The call site identifies a scan: its arguments' place in the program.
        if &**name == "memoScan" {
            return self.memo_scan(args.as_ptr() as usize, pos);
        }
        self.builtin(name, pos)
    }

    /// `memoScan(items, init, step, deps)`: see `memo.rs`.
    fn memo_scan(&mut self, site: usize, a: Vec<Value>) -> RResult<Value> {
        let Some(Value::List(items)) = a.first().cloned() else {
            return err("memoScan(items, init, step, deps): items must be a list");
        };
        let init = a.get(1).cloned().unwrap_or(Value::Undef);
        let Some(step) = a.get(2).cloned() else {
            return err("memoScan(items, init, step, deps): no step");
        };
        let deps = a.get(3).cloned().unwrap_or(Value::Undef);
        let key = {
            let path = self.path.join("/");
            let c = self.memo_counts.entry((site, path.clone())).or_insert(0);
            *c += 1;
            (site, path, *c - 1)
        };
        // What the store keeps never changes: its init, deps, items, and each step's state and out
        // are frozen.
        let kept = self.memo.is_some();
        if kept {
            crate::value::freeze(&init);
            crate::value::freeze(&deps);
            for x in items.read().iter() {
                crate::value::freeze(x)
            }
        }
        // The kept prefix: items equal to a kept variant's.
        let (start, mut states, mut outs) = match &self.memo {
            Some(m) => m.borrow_mut().resume(&key, &deps, &init, &items.read()),
            None => (0, vec![], vec![]),
        };
        let mut state = if start == 0 {
            init.clone()
        } else {
            states[start - 1].clone()
        };
        let steps = |it: &mut Self,
                     from: usize,
                     mut state: Value,
                     states: &mut Vec<Value>,
                     outs: &mut Vec<Value>|
         -> RResult<Value> {
            for i in from..items.read().len() {
                let Some(item) = items.read().get(i).cloned() else {
                    break;
                };
                let r = it.call_value(&step, vec![state, item, Value::Int(i as i64)])?;
                let Value::List(pair) = r else {
                    return err("memoScan: step must return [state, out]");
                };
                let (s, o) = match pair.read().as_slice() {
                    [s, o] => (s.clone(), o.clone()),
                    _ => return err("memoScan: step must return [state, out]"),
                };
                // What a step made is the cache's: a recovery image may not hold it, since a cold
                // cache would make a new object where the original reuses this one.
                if kept {
                    crate::value::freeze_derived(&s);
                    crate::value::freeze_derived(&o);
                }
                states.push(s.clone());
                outs.push(o);
                state = s;
            }
            Ok(state)
        };
        state = match steps(self, start, state, &mut states, &mut outs) {
            Ok(state) => state,
            Err(e) => {
                // A failed render keeps the prefix it computed: the next resumes there, not from
                // index 0 (a scan grown one item per render may not fit one render's fuel whole).
                if let Some(m) = &self.memo
                    && !states.is_empty()
                {
                    let prefix = items.read()[..states.len()].to_vec();
                    m.borrow_mut().put(
                        key,
                        crate::memo::Entry {
                            _module: self.scope.clone(),
                            deps,
                            init,
                            items: prefix,
                            states,
                            outs,
                        },
                    );
                }
                return Err(e);
            }
        };
        if let Some(m) = self.memo.clone() {
            {
                let mut m = m.borrow_mut();
                m.reused += start;
                m.computed += items.read().len() - start;
            }
            if m.borrow().verify && start > 0 {
                let (mut s2, mut o2) = (vec![], vec![]);
                let fresh = steps(self, 0, init.clone(), &mut s2, &mut o2)?;
                if !crate::memo::deep_eq(&fresh, &state)
                    || o2.len() != outs.len()
                    || !o2.iter().zip(&outs).all(|(x, y)| crate::memo::deep_eq(x, y))
                {
                    return err(
                        "memoScan: a kept result differs from a fresh one: the step depends on something besides (state, item, index, deps)",
                    );
                }
            }
            m.borrow_mut().put(
                key,
                crate::memo::Entry {
                    _module: self.scope.clone(),
                    deps,
                    init,
                    items: items.read().to_vec(),
                    states,
                    outs: outs.clone(),
                },
            );
        }
        let mut r = Map::default();
        r.insert(Rc::from("state"), state);
        r.insert(Rc::from("outs"), Value::list(outs));
        Ok(Value::map(r))
    }

    // ---------------- statements ----------------

    fn exec_simple(&mut self, b: &[Stmt]) -> RResult<Flow> {
        for s in b {
            match s {
                Stmt::Let(n, e) => {
                    let v = self.eval(e)?;
                    self.scopes.push((n.clone(), Slot::Val(v)));
                }
                Stmt::Init(n, e) => {
                    let v = self.eval(e)?;
                    self.init_var(n, v)?;
                }
                Stmt::Declare(names) => {
                    self.scopes
                        .extend(names.iter().map(|n| (n.clone(), Slot::Val(uninitialized()))));
                }
                // `x = [...x, more]` / `x = [...x.slice(0, -k), more]`: the idiomatic way to grow a
                // list in a template. When nothing else holds x's array it is updated in place
                // (the same result, without copying the list each step: O(n), not O(n^2)).
                Stmt::Assign(Target::Var(n), Expr::List(items)) if grow_in_place(n, items).is_some() => {
                    let drop = grow_in_place(n, items).unwrap();
                    // `...x` is read before the later items run: one of them may change x
                    // (`x = [...x, f()]` where f pushes to x), and that change is not spread.
                    let spread_len = match self.local(n) {
                        Some(Value::List(l)) => Some(l.read().len()),
                        _ => None,
                    };
                    let mut more = Vec::with_capacity(items.len() - 1);
                    for i in &items[1..] {
                        match i {
                            Expr::Spread(x) => match self.eval(x)? {
                                Value::Str(s) => more.extend(s.chars().map(|c| Value::string(c.to_string()))),
                                other => more.extend(iterate(&other)?),
                            },
                            i => more.push(self.eval(i)?),
                        }
                    }
                    match self.take_var(n) {
                        // Only x held the array, so nothing can see it change.
                        Some(Value::List(l)) if Rc::strong_count(&l) == 1 && !l.is_frozen() => {
                            {
                                let mut v = l.write()?;
                                let keep = spread_len.unwrap_or(v.len()).min(v.len()).saturating_sub(drop);
                                v.truncate(keep);
                                v.extend(more);
                            }
                            self.set_var(n, Value::List(l))?;
                        }
                        // Not a list (or not a declared variable): as written, with the items
                        // already evaluated (each runs once).
                        old => {
                            if let Some(old) = old {
                                self.set_var(n, old)?;
                            }
                            let Expr::Spread(first) = &items[0] else { unreachable!() };
                            let mut v = match self.eval(first)? {
                                Value::Str(s) => s.chars().map(|c| Value::string(c.to_string())).collect(),
                                other => iterate(&other)?,
                            };
                            v.extend(more);
                            self.set_var(n, Value::list(v))?;
                        }
                    }
                }
                Stmt::Assign(t, e) => {
                    self.eval_assign(t, AssignOp::Set, e)?;
                }
                Stmt::Expr(e) => {
                    self.eval(e)?;
                }
                Stmt::Return(e) => return Ok(Flow::Return(self.eval(e)?)),
                Stmt::LetPattern(p, e, init) => {
                    let v = self.eval(e)?;
                    self.bind_pattern(p, v, *init)?;
                }
                Stmt::AssignPattern(targets, e) => {
                    let v = self.eval(e)?;
                    let items = iterate(&v)?;
                    for (i, t) in targets.iter().enumerate() {
                        self.assign(t, items.get(i).cloned().unwrap_or(Value::Undef))?;
                    }
                }
                Stmt::Line(n) => self.line = *n,
                Stmt::Throw(e) => {
                    let v = self.eval(e)?;
                    // Thrown values travel as error text; `catch (e)` gets the value itself.
                    let msg = match member_read(&v, &Value::str("message")) {
                        Ok(m @ Value::Str(_)) if matches!(&v, Value::Map(_)) => crate::jsvalue::to_string(&m),
                        _ => crate::jsvalue::to_string(&v),
                    };
                    // An uncaught `throw new TypeError(..)` is a type error (like an invalid
                    // operation); any other thrown value is an explicit raise.
                    let type_error =
                        matches!(&v, Value::Map(m) if m.class().is_some_and(|c| c.is_a(&error_class("TypeError"))));
                    let text = if type_error {
                        format!("TypeError: {msg}")
                    } else {
                        format!("raise: {msg}")
                    };
                    self.thrown = Some(v);
                    self.thrown_text = text.clone();
                    return err(text);
                }
                _ => unreachable!("control flow is compiled to function instructions"),
            }
        }
        Ok(Flow::Normal)
    }

    /// Bind a declaration pattern to a value: push new variables, or (`init`) initialize the
    /// declared ones.
    fn bind_pattern(&mut self, p: &Pattern, v: Value, init: bool) -> RResult<()> {
        match p {
            Pattern::Name(n) => {
                if init {
                    self.init_var(n, v)?;
                } else {
                    self.scopes.push((n.clone(), Slot::Val(v)));
                }
            }
            Pattern::Array(items, rest) => {
                let xs = iterate(&v)?;
                for (i, item) in items.iter().enumerate() {
                    if let Some((p, d)) = item {
                        let x = match (xs.get(i).cloned().unwrap_or(Value::Undef), d) {
                            (Value::Undef, Some(d)) => self.eval(d)?,
                            (x, _) => x,
                        };
                        self.bind_pattern(p, x, init)?;
                    }
                }
                if let Some(r) = rest {
                    self.bind_pattern(r, Value::list(xs.get(items.len()..).unwrap_or(&[]).to_vec()), init)?;
                }
            }
            Pattern::Object(items, rest) => {
                if matches!(v, Value::Undef | Value::None) {
                    let s = crate::jsvalue::to_string(&v);
                    return err(format!("TypeError: Cannot destructure '{s}' as it is {s}."));
                }
                let mut taken = Vec::with_capacity(items.len());
                for (k, p, d) in items {
                    let key = self.eval(k)?;
                    let key = Value::string(crate::jsvalue::to_string(&self.primitive(&key, true)?));
                    let x = match (crate::jsvalue::index(&v, &key)?, d) {
                        (Value::Undef, Some(d)) => self.eval(d)?,
                        (x, _) => x,
                    };
                    taken.push(key);
                    self.bind_pattern(p, x, init)?;
                }
                if let Some(r) = rest {
                    let mut m = Map::default();
                    let own: Vec<(Rc<str>, Value)> = match &v {
                        Value::Map(o) => o.read().iter().map(|(k, x)| (k.clone(), x.clone())).collect(),
                        Value::List(_) | Value::Str(_) => iterate(&v)?
                            .into_iter()
                            .enumerate()
                            .map(|(i, x)| (Rc::from(i.to_string()), x))
                            .collect(),
                        _ => vec![],
                    };
                    for (k, x) in own {
                        if !taken.iter().any(|t| t.as_str() == Some(&*k)) {
                            m.insert(k, x);
                        }
                    }
                    if init {
                        self.init_var(r, Value::map(m))?;
                    } else {
                        self.scopes.push((r.clone(), Slot::Val(Value::map(m))));
                    }
                }
            }
        }
        Ok(())
    }

    /// What `catch (e)` binds: the thrown value itself, or for a failed operation an error
    /// object (`{name, message, stack}`).
    pub(crate) fn caught(&mut self, e: &str) -> Value {
        if self.thrown.is_some() && crate::error_message(e) == self.thrown_text {
            return self.thrown.take().unwrap();
        }
        self.thrown = None;
        self.error_object(e)
    }

    /// `t = v`, `t += v`, `t ??= v` ... as an expression: the value assigned (or, for a logical
    /// assignment that does not assign, the old value).
    fn eval_assign(&mut self, t: &Target, op: AssignOp, v: &Expr) -> RResult<Value> {
        if op == AssignOp::Set
            && let Target::Var(n) = t
        {
            let v = self.eval(v)?;
            self.set_var(n, v.clone())?;
            return Ok(v);
        }
        let reference = self.reference(t)?;
        let new = match op {
            AssignOp::Set => self.eval(v)?,
            AssignOp::Bin(b) => {
                let old = self.read_reference(&reference)?;
                let rhs = self.eval(v)?;
                self.bin(b, &old, &rhs)?
            }
            AssignOp::And | AssignOp::Or | AssignOp::Nullish => {
                let old = self.read_reference(&reference)?;
                let keep = match op {
                    AssignOp::And => !crate::jsvalue::truthy(&old),
                    AssignOp::Or => crate::jsvalue::truthy(&old),
                    _ => !matches!(old, Value::Undef | Value::None),
                };
                if keep {
                    return Ok(old);
                }
                self.eval(v)?
            }
        };
        self.write_reference(&reference, new.clone())?;
        Ok(new)
    }

    /// `xs.push(v)`, `xs.sort()`, `delete o.k`, `Object.assign(o, ..)`: the value at `t` is
    /// updated (in place when nothing else holds it) and the method's result returned.
    /// A changing method (`xs.push(v)`, `delete o.k`, `Object.assign(o, ..)`) on the object a
    /// target names: the object changes in place, and every reference to it sees the change.
    fn eval_mutate(&mut self, t: &Target, name: &str, args: &[Expr]) -> RResult<Value> {
        let recv = self.target_value(t)?;
        let pos = self.eval_args(args)?;
        match (name, &recv) {
            ("__delete", Value::Map(m)) => {
                let key = self.primitive(&pos[0], true)?;
                m.write()?.shift_remove(crate::jsvalue::to_string(&key).as_str());
                Ok(Value::Bool(true))
            }
            ("__delete", Value::List(_)) => {
                err("deleting an array element leaves a hole (sparse arrays are not supported): use splice")
            }
            ("__delete", Value::Undef | Value::None) => err(format!(
                "TypeError: Cannot convert {} to object",
                crate::jsvalue::to_string(&recv)
            )),
            ("__delete", _) => Ok(Value::Bool(true)),
            ("__assign", Value::Map(m)) => {
                for src in &pos {
                    match crate::jsbuiltins::object_global("Object.entries", std::slice::from_ref(src)) {
                        _ if matches!(src, Value::Undef | Value::None) => {}
                        Some(Ok(Value::List(entries))) => {
                            let entries: Vec<Value> = entries.read().clone();
                            for e in &entries {
                                if let Value::List(kv) = e
                                    && let Some(v) = kv.read().get(1)
                                {
                                    crate::value::note_write(&recv, v)
                                }
                            }
                            let mut w = m.write()?;
                            for e in &entries {
                                if let Value::List(kv) = e {
                                    let kv = kv.read();
                                    crate::jsvalue::obj_insert(
                                        &mut w,
                                        Rc::from(crate::jsvalue::to_string(&kv[0])),
                                        kv[1].clone(),
                                    );
                                }
                            }
                        }
                        Some(Err(e)) => return Err(e),
                        _ => {}
                    }
                }
                Ok(recv.clone())
            }
            ("__assign", Value::Undef | Value::None) => err("TypeError: Cannot convert undefined or null to object"),
            ("__assign", _) => err("Object.assign: the target must be an object"),
            // Not an array (an object's own `push` function, a token run, ...): a plain call.
            _ => self.method(&recv, name, pos),
        }
    }

    // ---------------- builtins ----------------

    fn builtin(&mut self, name: &str, a: Vec<Value>) -> RResult<Value> {
        if name == "\0compiler.loadModule" {
            return self.load_module(a);
        }
        if self.initializing > 0 && matches!(name, "useState" | "useRef" | "useEffect" | "useEvent") {
            return err("hooks belong in component functions, not module initializers");
        }
        if let Some(method) = name.strip_prefix("\0harness.") {
            if method == "machine.get" {
                let role = a
                    .first()
                    .and_then(Value::as_str)
                    .ok_or("harness.machine.get expects a machine role")?;
                let mut m = Map::default();
                m.insert(Rc::from("__machine"), Value::str(role));
                return Ok(Value::map(m));
            }
            if method == "machine.call" {
                return self.machine_call(Value::str(""), a);
            }
            return self.harness_method(method, a);
        }
        if name == "\0machine.call" {
            let mut a = a;
            let role = a.remove(0);
            return self.machine_call(role, a);
        }
        if name == "\0scroll" {
            return self.scroll(a);
        }
        // A ref's read (`useRef`): in a handler only. A ref does not render, so a render may not
        // read it (as React's rule for `ref.current`).
        if name == "\0refget" {
            if !self.acting {
                return err(
                    "a ref is read in handlers, not during render (it does not render): read it in an event, effect or tool",
                );
            }
            let key: Rc<str> = match a.first() {
                Some(Value::Str(k)) => k.to_rc(),
                _ => unreachable!(),
            };
            let live = self.live_ui.as_ref().and_then(|l| l.borrow().get(&key).cloned());
            return Ok(live
                .or_else(|| self.ui_updates.get(&key).cloned())
                .or_else(|| self.ui_value(&key))
                .or_else(|| self.ui_seen.get(&key).cloned())
                .unwrap_or(Value::Undef));
        }
        if name == "memoScan" {
            let site = match a.first() {
                Some(Value::Int(site)) => *site as usize,
                _ => unreachable!(),
            };
            return self.memo_scan(site, a[1..].to_vec());
        }
        let a0 = a.first().cloned().unwrap_or(Value::Undef);
        Ok(match name {
            // A fault for the runtime's tests: app code that panics (src/runtime/host.rs).
            #[cfg(test)]
            "__testPanic" => panic!("{}", crate::jsvalue::to_string(&a0)),
            "useState" => self.use_state(a0.clone(), a.get(1).and_then(Value::as_str))?,
            // `useRef(initial, name?)`: `[read, write]`, storage that does not render. Kept with
            // the instance's hooks (journal, images), but a write to it alone does not change
            // what renders, so the last evaluated document stays current (React's useRef).
            "useRef" => self.use_ref(a0.clone(), a.get(1).and_then(Value::as_str))?,
            // `useEvent(name, listener)`: the listener runs when an event `name` arrives (the
            // browser's addEventListener, as a hook: registered by each committed render).
            "useEvent" => {
                if !self.acting {
                    let Some(name) = a0.as_str() else {
                        return err("useEvent(name, listener): name is a string");
                    };
                    let f = a
                        .get(1)
                        .cloned()
                        .filter(Value::is_callable)
                        .ok_or("useEvent(name, listener): listener is a function")?;
                    self.event_decls.push((name.to_string(), f));
                }
                Value::Undef
            }
            // What a function's declaration says (a tool reads it): its name (`function f`, or the
            // `const` it is bound to), its JSDoc summary, its props parameter's type as a schema.
            "nameOf" => match a0.declaration() {
                Value::Func(f) => Value::str(&f.name),
                Value::Lambda(l, _) => l.name.as_ref().map_or(Value::Undef, |n| Value::str(n)),
                _ => Value::Undef,
            },
            "docOf" => match a0.declaration() {
                Value::Func(f) => f.doc.as_ref().map_or(Value::Undef, |d| Value::str(d)),
                Value::Lambda(l, _) => l.doc.as_ref().map_or(Value::Undef, |d| Value::str(d)),
                _ => Value::Undef,
            },
            "schemaOf" => {
                let (name, ty, types) = match a0.declaration() {
                    Value::Func(f) => (f.name.to_string(), f.props_type.clone(), &f.module.types),
                    Value::Lambda(l, env) => (
                        l.name.as_deref().unwrap_or("this function").to_string(),
                        l.props_type.clone(),
                        &env.module.types,
                    ),
                    _ => return err("schemaOf(fn): fn is a function"),
                };
                let Some(ty) = ty else {
                    return err(format!(
                        "schemaOf({name}): its parameter has no type (write ({{ q }}: {{ q: string }}) or give the tool a schema)"
                    ));
                };
                match crate::schema::schema_of(&ty, types) {
                    Ok(s) => Value::from_json(&s),
                    Err(e) => return err(format!("schemaOf({name}): {e}")),
                }
            }
            // `model(view?, opts)` and `dispatch(output)`: the model call (the browser's `fetch`,
            // with a view as its request) and running a reply's calls.
            "model" => return self.harness_method("model", a),
            "dispatch" => return self.harness_method("dispatch", a),
            // `useEffect(setup, deps)`: declared by the render; the agent runs `setup` (a task)
            // after a render whose `deps` differ by value from the last committed one's, and its
            // returned cleanup before the next run or when the effect goes away. Its identity is
            // its component path and hook index (the rules of hooks: call it unconditionally).
            "useEffect" => {
                if !self.acting {
                    let n = {
                        let c = self.hook_counts.last_mut().unwrap();
                        *c += 1;
                        *c - 1
                    };
                    let key = format!("{}/effect{}", self.path.join("/"), n);
                    if matches!(&a0, Value::Lambda(l, _) if l.is_async) || matches!(&a0, Value::Func(f) if f.is_async) {
                        return err(
                            "useEffect(setup): setup must be synchronous (start async work inside it and return a cleanup)",
                        );
                    }
                    self.effect_decls
                        .push((key, a.get(1).cloned().unwrap_or(Value::Undef), a0.clone()));
                }
                Value::Undef
            }
            "RegExp" => {
                let flags = match a.get(1) {
                    Some(Value::Str(f)) => Some(f.to_string()),
                    None | Some(Value::Undef) => None,
                    Some(o) => Some(crate::jsvalue::to_string(o)),
                };
                match (&a0, flags) {
                    (Value::Regex(r), None) => Value::Regex(r.clone()),
                    (Value::Regex(r), Some(f)) => Value::Regex(Rc::new(crate::jsregex::compile(&r.source, &f)?)),
                    (Value::Str(s), f) => {
                        Value::Regex(Rc::new(crate::jsregex::compile(s, f.as_deref().unwrap_or(""))?))
                    }
                    (Value::Undef, f) => {
                        Value::Regex(Rc::new(crate::jsregex::compile("", f.as_deref().unwrap_or(""))?))
                    }
                    (o, f) => Value::Regex(Rc::new(crate::jsregex::compile(
                        &crate::jsvalue::to_string(o),
                        f.as_deref().unwrap_or(""),
                    )?)),
                }
            }
            "asChatTools" => {
                let enc = self.encoder.ok_or("asChatTools requires a bound encoder")?;
                let safe = self.prog.pragmas.iter().any(|p| p == "safe_text");
                let callable = match a.get(1) {
                    None | Some(Value::Undef) => true,
                    Some(Value::Map(options)) => {
                        let options = options.read();
                        if options.keys().any(|k| &**k != "callable") {
                            return err("asChatTools: unknown option (use callable)");
                        }
                        match options.get("callable") {
                            None | Some(Value::Undef) => true,
                            Some(Value::Bool(value)) => *value,
                            _ => return err("asChatTools: callable must be a boolean"),
                        }
                    }
                    _ => return err("asChatTools: options must be an object"),
                };
                if let Some((cache, enabled)) = self.projection_cache {
                    let mut cache = cache.borrow_mut();
                    cache.enabled = enabled;
                    crate::adapter::project(
                        &a0,
                        enc,
                        safe,
                        &mut cache,
                        callable,
                        self.window.map(crate::emit::Window::tokens),
                    )?
                } else {
                    crate::adapter::project(
                        &a0,
                        enc,
                        safe,
                        &mut crate::emit::RunCache::new(false),
                        callable,
                        self.window.map(crate::emit::Window::tokens),
                    )?
                }
            }
            // logTurns(log, {skip}): each record (but the kinds in `skip`) as a turn, natively and
            // memoized per record (a record is never changed in place). A turn strategy's building
            // block (views/strategies/turn_strategy.tsx).
            "logTurns" => {
                let skip: Vec<String> = match a.get(1).map(Value::to_wire) {
                    Some(serde_json::Value::Object(o)) => o
                        .get("skip")
                        .and_then(|s| s.as_array())
                        .map(|l| l.iter().filter_map(|k| k.as_str().map(str::to_owned)).collect())
                        .unwrap_or_default(),
                    _ => vec![],
                };
                let memo = self.memo.clone();
                let mut memo = memo.as_ref().map(|m| m.borrow_mut());
                let (turns, origins) =
                    crate::turns::log_turns(&a0, &skip, memo.as_deref_mut(), &self.prog.styles, &self.ancestry)?;
                // Each turn remembers its record, for error messages and the template's checks.
                let table = Rc::make_mut(&mut self.turn_origins);
                for (t, (i, kind)) in turns.iter().zip(origins) {
                    if let Value::Map(m) = t {
                        table.insert(Rc::as_ptr(m) as usize, (i, kind));
                    }
                }
                Value::list(turns)
            }
            // turnPlan(screen, turns): a turn plan for a template app: the turns, then the laid-out
            // screen (`asChatTools(children)`) as the final user turn.
            "turnPlan" => {
                let Value::Map(app) = a0 else {
                    return err("turnPlan(screen, turns): screen is asChatTools(children)");
                };
                let turns: Vec<Value> = match a.get(1) {
                    Some(Value::List(l)) => l.read().to_vec(),
                    None | Some(Value::Undef) => vec![],
                    _ => return err("turnPlan(screen, turns): turns is a list"),
                };
                let origins = turns
                    .iter()
                    .map(|t| match t {
                        Value::Map(m) => self
                            .turn_origins
                            .get(&(Rc::as_ptr(m) as usize))
                            .map(|(i, k)| (*i, k.clone())),
                        _ => None,
                    })
                    .collect();
                crate::turns::fragment(crate::turns::plan(&app.read(), turns, origins)?)
            }
            // templateInput(children, system, generated_ok): the third argument opts the template
            // into model records with exact `ids` (it renders `generated` assistant turns).
            "templateInput" => {
                // templateInput(children, system, generated_ok, alternate)
                let generated_ok = a.get(2).is_some_and(crate::jsvalue::truthy);
                let alternate = a.get(3).is_some_and(crate::jsvalue::truthy);
                let (input, plan) = crate::turns::template_input(
                    &a0,
                    a.get(1)
                        .cloned()
                        .unwrap_or(Value::str("Use the available tools to interact with the app.")),
                    generated_ok,
                    alternate,
                )?;
                let origins = Rc::make_mut(&mut self.turn_origins);
                for (t, o) in plan.turns.iter().zip(&plan.origins) {
                    if let (Value::Map(m), Some((i, kind))) = (t, o) {
                        origins.insert(Rc::as_ptr(m) as usize, (*i, kind.clone()));
                    }
                }
                let actions = crate::turns::screen_actions(&plan);
                self.template_actions.last_mut().unwrap().extend(actions);
                input
            }
            // tokens(ids): exact token ids as a laid-out fragment (spliced, never re-encoded).
            "tokens" if token_run(&a0).is_some() => a0,
            "tokens" => {
                let ids = iterate(&a0)?
                    .iter()
                    .map(|v| match v {
                        Value::Int(n) if (0..=u32::MAX as i64).contains(n) => Ok(*n as u32),
                        o => err(format!(
                            "tokens() takes a list of token ids; got an element of type {}",
                            o.type_name()
                        )),
                    })
                    .collect::<RResult<Vec<u32>>>()?;
                Value::Frag(Rc::new(vec![Item::Tokens(ids.into(), Rc::from(Vec::new()))]))
            }
            // Token length of plain text under the bound tokenizer (special-token strings count as text).
            "countTokens" => {
                let enc = self.encoder.ok_or("countTokens requires a bound encoder")?;
                Value::Int(match &a0 {
                    Value::Str(s) => enc.count_run(s),
                    Value::Frag(f) => enc.count_run(&self.frag_text(f)?),
                    v => enc.count_run(&crate::jsvalue::to_string(v)),
                } as i64)
            }
            // textContent(content): the text laid-out content stands for, as the browser's
            // Node.textContent reads a node: for a template whose model tokenizes one whole string.
            "textContent" => crate::turns::flatten(
                &a0,
                self.encoder.ok_or("textContent requires a bound encoder")?,
                self.window.map(crate::emit::Window::tokens),
                &mut None,
            )?,
            "messagesJson" => crate::turns::messages_json(
                &a0,
                self.encoder.ok_or("messagesJson requires a bound encoder")?,
                self.window.map(crate::emit::Window::tokens),
                &self.prog.styles,
                &self.ancestry,
            )?,
            // A protocol's textual content coercion, with a native-fragment arm.
            "asContent" => {
                if matches!(a0, Value::Frag(_)) {
                    a0
                } else {
                    Value::string(a0.to_py_str())
                }
            }
            // The special token with this text if the tokenizer has one, else the plain text.
            "textOrToken" => match (&a0, a0.as_str().and_then(|t| self.specials.get(t))) {
                (Value::Str(t), Some(Value::Int(id))) => Value::Token(*id as u32, t.to_rc()),
                _ => a0.clone(),
            },
            "isFragment" => Value::Bool(matches!(a0, Value::Frag(_))),
            // `formatNow("%d %b %Y")`: the render clock in strftime format (templates print dates).
            "strftime_now" | "formatNow" => {
                if self.acting && self.effects.is_some() {
                    return err("window handlers read time through harness.now() only");
                }
                crate::observe::clock();
                // An unknown specifier is an error here; chrono panics when asked to print one.
                let format = a0.as_str().unwrap_or("");
                let items: Vec<_> = chrono::format::StrftimeItems::new(format).collect();
                if items.iter().any(|item| matches!(item, chrono::format::Item::Error)) {
                    return err(format!("formatNow: `{format}` is not a strftime format"));
                }
                Value::string(self.now.format_with_items(items.iter()).to_string())
            }
            // Promise combinators (JavaScript semantics: `race` settles and does not cancel).
            "Promise.all" | "Promise.race" | "Promise.any" | "Promise.allSettled" => {
                use crate::effects::Combinator;
                let kind = match name {
                    "Promise.all" => Combinator::All,
                    "Promise.race" => Combinator::Race,
                    "Promise.any" => Combinator::Any,
                    _ => Combinator::AllSettled,
                };
                self.promise_combinator(kind, &a0)
            }
            "Promise.resolve" => self.promise_resolve(a0),
            "Promise.reject" => {
                let msg = crate::jsvalue::to_string(&a0);
                Value::Promise(crate::effects::Promise::settled(crate::effects::Settle::Err(
                    crate::effects::Rejection {
                        text: format!("raise: {msg}"),
                        reason: a0,
                    },
                )))
            }
            "__constAssign" => return err("TypeError: Assignment to constant variable."),
            "JSON.stringify" => self.json_stringify(&a)?,
            "JSON.parse" => self.json_parse(&a)?,
            "structuredClone" => crate::jsbuiltins::structured_clone(&a0)?,
            "Map" | "Set" => crate::jsbuiltins::new_coll(name == "Set", &a0)?,
            "Map.groupBy" => {
                let mut c = crate::value::Coll::new(false);
                let f = a.get(1).cloned().unwrap_or(Value::Undef);
                for (i, x) in iterate(&a0)?.into_iter().enumerate() {
                    let k = self.call_value(&f, vec![x.clone(), Value::Int(i as i64)])?;
                    match c.get(&k).cloned() {
                        Some(Value::List(l)) => l.write()?.push(x),
                        _ => c.insert(k, Value::list(vec![x])),
                    }
                }
                Value::Coll(Rc::new(crate::value::Obj::new(c)))
            }
            "Array.from" => self.array_from(&a)?,
            // `console.log(..)`: a line for whoever watches the app (its trace); it changes nothing.
            n if n
                .strip_prefix("console.")
                .is_some_and(|l| crate::console::level(l).is_some()) =>
            {
                let text = a
                    .iter()
                    .map(|v| match v {
                        Value::Str(s) => s.to_string(),
                        v => match crate::value::check_graph(v, crate::value::MAX_DATA_NESTING) {
                            Ok(()) => v.to_wire().to_string(),
                            Err(crate::value::GraphFault::Cycle) => "[Circular]".to_string(),
                            Err(_) => "[nested too deeply to print]".to_string(),
                        },
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                crate::console::push(crate::console::level(&n["console.".len()..]).unwrap(), text);
                Value::Undef
            }
            // `Date.now()`: the instance's clock, in milliseconds. In a handler it is the journal's
            // (`harness.now()`), so a replay reads the same time; in a render, the render's.
            "Date.now" => {
                if self.acting && self.effects.is_some() {
                    return self.harness_method("now", vec![]);
                }
                crate::observe::clock();
                Value::Int(self.now.and_utc().timestamp_millis())
            }
            n if n.contains('.')
                || matches!(
                    n,
                    "String"
                        | "Number"
                        | "Boolean"
                        | "parseInt"
                        | "parseFloat"
                        | "isNaN"
                        | "isFinite"
                        | "Error"
                        | "TypeError"
                        | "RangeError"
                        | "SyntaxError"
                        | "ReferenceError"
                        | "URIError"
                        | "EvalError"
                        | "Array"
                        | "__arrayFill"
                        | "isPlainObject"
                        | "formatJSON"
                        | "formatPythonValue"
                        | "formatPythonRepr"
                        | "trimChars"
                        | "trimStartChars"
                        | "trimEndChars"
                        | "capitalize"
                        | "titleCase"
                ) =>
            {
                // A function of text or of numbers asks an object argument for its text or its
                // number (`primitive`).
                let text = matches!(n, "String" | "parseInt" | "parseFloat");
                let number = n.starts_with("Math.") || matches!(n, "Number" | "isNaN" | "isFinite");
                if (text || number) && a.iter().any(|v| matches!(v, Value::Map(_) | Value::List(_))) {
                    let a = a.iter().map(|v| self.primitive(v, text)).collect::<RResult<Vec<_>>>()?;
                    js_global(n, &a)?
                } else {
                    js_global(n, &a)?
                }
            }
            _ => {
                return err(match crate::migrate::function(name) {
                    Some(hint) => {
                        format!("ReferenceError: {name} is not defined (Python/Jinja `{name}(..)`: use {hint})")
                    }
                    None => match crate::migrate::platform(name) {
                        Some(hint) => format!("ReferenceError: {name} is not defined ({hint})"),
                        None => format!("ReferenceError: {name} is not defined"),
                    },
                });
            }
        })
    }

    fn method(&mut self, recv: &Value, name: &str, args: Vec<Value>) -> RResult<Value> {
        let function = self.get_prop(recv, name)?;
        self.call_with_receiver(&function, recv.clone(), args)
    }

    fn intrinsic_method(
        &mut self,
        owner: crate::value::Prototype,
        recv: &Value,
        name: &str,
        a: Vec<Value>,
    ) -> RResult<Value> {
        use crate::value::Prototype;
        let (recv, a) = self.asked(owner, recv, name, a)?;
        let recv = &recv;
        match owner {
            Prototype::Object => self.object_method(recv, name, &a),
            Prototype::Error => self.error_string(recv),
            Prototype::Array => {
                if let Value::List(l) = recv
                    && name == "join"
                    && has_tokens(&l.read())
                {
                    return join_tokens(&l.read().clone(), &a.first().cloned().unwrap_or(Value::str(",")));
                }
                self.array_method(recv, name, a)
            }
            Prototype::Map | Prototype::Set => match recv {
                Value::Coll(c) if c.read().set == (owner == Prototype::Set) => self.coll_method(recv, c, name, a),
                _ => err(format!(
                    "TypeError: {owner:?}.prototype.{name} called on incompatible receiver"
                )),
            },
            Prototype::RegExp => {
                if name == "toString" {
                    if !recv.is_object() {
                        return err("TypeError: RegExp.toString requires an object");
                    }
                    let source = self.get_prop(recv, "source")?;
                    let flags = self.get_prop(recv, "flags")?;
                    return Ok(Value::string(format!(
                        "/{}/{}",
                        crate::jsvalue::try_to_string(&source)?,
                        crate::jsvalue::try_to_string(&flags)?
                    )));
                }
                match recv {
                    Value::Regex(re) => self.regex_method(re, name, &a),
                    _ => err("TypeError: RegExp method called on incompatible receiver"),
                }
            }
            Prototype::Number | Prototype::Boolean => {
                let valid = match owner {
                    Prototype::Number => {
                        matches!(recv, Value::Int(_) | Value::Float(_) | Value::BigInt(_))
                    }
                    _ => matches!(recv, Value::Bool(_)),
                };
                if !valid {
                    return err(format!("TypeError: {owner:?} method called on incompatible receiver"));
                }
                self.primitive_method(recv, name, &a)?
                    .ok_or_else(|| not_a_function(crate::jsvalue::type_of(recv), name))
            }
            Prototype::String => {
                if matches!(recv, Value::None | Value::Undef) {
                    return err("TypeError: String method called on null or undefined");
                }
                if matches!(name, "toString" | "valueOf") && !matches!(recv, Value::Str(_)) {
                    return err("TypeError: String method called on incompatible receiver");
                }
                let s = crate::value::Text::from(crate::jsvalue::try_to_string(recv)?);
                let first = a.first().unwrap_or(&Value::Undef);
                if matches!(
                    name,
                    "match" | "matchAll" | "search" | "replace" | "replaceAll" | "split"
                ) && (matches!(first, Value::Regex(_))
                    || matches!(name, "match" | "matchAll" | "search") && !matches!(first, Value::Undef))
                {
                    self.str_regex_method(&s, name, &a)
                } else if matches!(name, "match" | "matchAll" | "search") {
                    // With no pattern JavaScript matches the empty one (`"abc".match()` is `[""]`).
                    self.str_regex_method(&s, name, &[Value::str("")])
                } else {
                    self.string_method(&s, name, &a)
                }
            }
            Prototype::Promise => {
                if self.initializing > 0 {
                    return err("Promise reactions belong in component effects or handlers, not module initializers");
                }
                if matches!(recv, Value::Promise(_)) {
                    self.promise_method(recv, name, a)
                } else {
                    err("TypeError: Promise method called on incompatible receiver")
                }
            }
            Prototype::Function => {
                if name == "toString" {
                    return err("Function source formatting is not supported");
                }
                if !recv.is_callable() {
                    return err("TypeError: Function method called on incompatible receiver");
                }
                let this = a.first().cloned().unwrap_or(Value::Undef);
                let rest = if name == "apply" {
                    match a.get(1) {
                        None | Some(Value::None | Value::Undef) => vec![],
                        Some(value) => {
                            if !value.is_object() {
                                return err("TypeError: CreateListFromArrayLike called on non-object");
                            }
                            self.array_like_values(value)?
                        }
                    }
                } else {
                    a.into_iter().skip(1).collect()
                };
                if name == "bind" {
                    Ok(Value::Bound(Rc::new((recv.clone(), this, rest))))
                } else {
                    self.call_with_receiver(recv, this, rest)
                }
            }
            Prototype::Fragment => self.fragment_method(recv, name, a),
        }
    }

    fn fragment_method(&mut self, recv: &Value, name: &str, a: Vec<Value>) -> RResult<Value> {
        let a0 = a.first().cloned().unwrap_or(Value::Undef);
        if let Some(ids) = token_run(recv) {
            let index = |v: &Value| match v {
                Value::Int(i) => Some(*i),
                Value::Float(f) => Some(*f as i64),
                _ => None,
            };
            let n = ids.len() as i64;
            let clamp = |i: i64| (if i < 0 { i + n } else { i }).clamp(0, n) as usize;
            match name {
                "at" => {
                    let i = index(&a0).unwrap_or(0);
                    let j = if i < 0 { i + n } else { i };
                    return Ok(if (0..n).contains(&j) {
                        Value::Int(ids[j as usize] as i64)
                    } else {
                        Value::Undef
                    });
                }
                "slice" => {
                    let lo = a.first().and_then(index).map_or(0, clamp);
                    let hi = a.get(1).and_then(index).map_or(ids.len(), clamp);
                    let part: Rc<[u32]> = if lo < hi {
                        ids[lo..hi].into()
                    } else {
                        Rc::from(Vec::new())
                    };
                    return Ok(Value::Frag(Rc::new(vec![Item::Tokens(part, Rc::from(Vec::new()))])));
                }
                "includes" => {
                    return Ok(Value::Bool(
                        index(&a0).is_some_and(|t| ids.iter().any(|&x| x as i64 == t)),
                    ));
                }
                _ => {}
            }
        }
        match recv {
            // A token fragment trims like the string it stands for: tokens are never whitespace.
            Value::Frag(f) if matches!(name, "trim" | "trimStart" | "trimEnd") => {
                let (l, r) = (name != "trimEnd", name != "trimStart");
                let mut v: Vec<Item> = f.iter().cloned().collect();
                if l {
                    while let Some(Item::Text(t)) = v.first() {
                        let t2 = t.trim_start();
                        if t2.is_empty() {
                            v.remove(0);
                        } else {
                            v[0] = Item::Text(Rc::from(t2));
                            break;
                        }
                    }
                }
                if r {
                    while let Some(Item::Text(t)) = v.last() {
                        let t2 = t.trim_end();
                        let n = v.len() - 1;
                        if t2.is_empty() {
                            v.pop();
                        } else {
                            v[n] = Item::Text(Rc::from(t2));
                            break;
                        }
                    }
                }
                Ok(Value::Frag(Rc::new(v)))
            }
            // Read-only questions about a fragment look at the text it stands for (tokens as their text).
            Value::Frag(f) if matches!(name, "startsWith" | "endsWith" | "includes" | "indexOf" | "lastIndexOf") => {
                let flat = Value::string(self.frag_text(f)?);
                self.method(&flat, name, a)
            }
            Value::Undef | Value::None => err(format!(
                "TypeError: Cannot read properties of {} (reading '{name}')",
                crate::jsvalue::to_string(recv)
            )),
            other => err(not_a_function(crate::jsvalue::type_of(other), name)),
        }
    }
}

/// Intrinsic names are deliberately resolved only after every app binding.
fn intrinsic(name: &str) -> Value {
    match name {
        "NaN" => Value::Float(f64::NAN),
        "Infinity" => Value::Float(f64::INFINITY),
        "Math" | "JSON" | "Object" | "Array" | "Number" | "String" | "Boolean" | "Promise" | "Map" | "Set"
        | "RegExp" | "Function" | "Date" | "WeakMap" | "WeakSet" | "Error" | "TypeError" | "RangeError"
        | "SyntaxError" | "ReferenceError" | "URIError" | "EvalError" | "parseInt" | "parseFloat" | "isNaN"
        | "isFinite" | "structuredClone" | "console" => Value::Native(Rc::<str>::from(name).into()),
        _ => Value::Undef,
    }
}

// ---------------- value helpers ----------------

/// `TypeError: string.strip is not a function`, with the JavaScript to write for a Python method.
pub(crate) fn not_a_function(kind: &str, name: &str) -> String {
    match crate::migrate::method(name) {
        Some(hint) => format!("TypeError: {kind}.{name} is not a function (Python `.{name}()`: use {hint})"),
        None => format!("TypeError: {kind}.{name} is not a function"),
    }
}

/// How deeply child lists nest (`{[[x]]}`), as `JSON.stringify` nests.
pub const MAX_CHILD_NESTING: usize = 400;

/// React-style child rendering: strings as text, numbers as JavaScript prints them, arrays spliced,
/// fragments replayed; true/false/null/undefined render nothing; objects are an error. A list
/// inside itself is a TypeError, and lists nested past `MAX_CHILD_NESTING` a RangeError (where
/// React's recursion would exhaust the stack).
pub fn emit_child(v: &Value, out: &mut dyn Out) -> RResult<()> {
    emit_nested(v, out, None)
}

/// The lists being spliced, innermost first.
struct Open<'a> {
    list: &'a Rc<crate::value::Arr>,
    depth: usize,
    up: Option<&'a Open<'a>>,
}

fn emit_nested(v: &Value, out: &mut dyn Out, open: Option<&Open>) -> RResult<()> {
    match v {
        Value::Str(s) => out.text_rc(&s.to_rc()),
        // Numbers print as JavaScript does (`2.0` is `2`).
        Value::Int(_) | Value::BigInt(_) | Value::Float(_) => out.text(&crate::jsvalue::to_string(v)),
        Value::Bool(_) | Value::None | Value::Undef => {}
        Value::List(l) => {
            let mut up = open;
            while let Some(o) = up {
                if Rc::ptr_eq(o.list, l) {
                    return err("TypeError: a child list contains itself (a circular structure cannot render)");
                }
                up = o.up;
            }
            let depth = open.map_or(1, |o| o.depth + 1);
            if depth > MAX_CHILD_NESTING {
                return err(format!(
                    "RangeError: child lists nest more than {MAX_CHILD_NESTING} levels"
                ));
            }
            let here = Open {
                list: l,
                depth,
                up: open,
            };
            stacker::maybe_grow(128 * 1024, 2 * 1024 * 1024, || {
                l.read().iter().try_for_each(|x| emit_nested(x, out, Some(&here)))
            })?;
        }
        Value::Frag(f) => out.replay(f)?,
        Value::Token(id, _) => out.tok(*id),
        other => {
            return err(format!(
                "an object is not a valid child (use JSON.stringify(x) or String(x) to print a {})",
                other.type_name()
            ));
        }
    }
    Ok(())
}

/// A run of exact token ids (a model turn's `ids`: one laid-out `Tokens` item, no spans). It reads
/// like a list of ids (`length`, `[i]`, `at`, `slice`, `includes`, iteration) without one value per id.
pub fn token_run(v: &Value) -> Option<&Rc<[u32]>> {
    match v {
        Value::Frag(f) => match f.as_slice() {
            [Item::Tokens(ids, spans)] if spans.is_empty() => Some(ids),
            _ => None,
        },
        _ => None,
    }
}

pub fn get_attr(v: &Value, k: &str) -> RResult<Value> {
    crate::observe::field(v, k);
    if let Value::Native(ns) = v {
        if let Some(v) = crate::jsbuiltins::global_constant(ns, k) {
            return Ok(v);
        }
        if crate::jsbuiltins::is_global_function(ns, k) {
            return Ok(Value::Native(Rc::<str>::from(format!("{ns}.{k}")).into()));
        }
        if k == "name" {
            return Ok(Value::string(ns.rsplit('.').next().unwrap_or(ns).to_string()));
        }
    }

    if k == "length"
        && let Some(ids) = token_run(v)
    {
        return Ok(Value::Int(ids.len() as i64));
    }
    match v {
        Value::Map(m) => Ok(match m.read().get(k) {
            Some(x) => x.clone(),
            None => crate::intrinsic::member(v, k).unwrap_or(Value::Undef),
        }),
        Value::List(_) | Value::Coll(_) if crate::intrinsic::member(v, k).is_some() => {
            Ok(crate::intrinsic::member(v, k).unwrap())
        }
        Value::Str(s) if k == "length" => Ok(Value::Int(s.char_count() as i64)),
        // tok.x.text: the token's string, for inspecting data (never emitted as text by itself)
        Value::Token(id, s) => Ok(match k {
            "text" => Value::Str(s.into()),
            "id" => Value::Int(*id as i64),
            _ => Value::Undef,
        }),
        Value::List(l) if k == "length" => Ok(Value::Int(l.read().len() as i64)),
        Value::Coll(c) if k == "size" => Ok(Value::Int(c.read().len() as i64)),
        Value::Regex(r) => Ok(match k {
            "source" => Value::str(&r.source),
            "flags" => Value::str(&r.flags),
            "global" => Value::Bool(r.global),
            "ignoreCase" => Value::Bool(r.flags.contains('i')),
            "multiline" => Value::Bool(r.flags.contains('m')),
            "dotAll" => Value::Bool(r.flags.contains('s')),
            "unicode" => Value::Bool(r.flags.contains('u')),
            "sticky" | "hasIndices" | "unicodeSets" => Value::Bool(false),
            "lastIndex" => Value::Int(r.last_index.get() as i64),
            _ => crate::intrinsic::member(v, k).unwrap_or(Value::Undef),
        }),
        Value::Undef | Value::None => err(format!(
            "TypeError: Cannot read properties of {} (reading '{}')",
            crate::jsvalue::to_string(v),
            k
        )),
        _ => Ok(crate::intrinsic::member(v, k).unwrap_or(Value::Undef)),
    }
}

/// `base[k] = v`.
fn set_prop(base: &Value, k: &Value, v: Value) -> RResult<()> {
    crate::value::note_write(base, &v);
    match base {
        Value::Map(m) => {
            let mut w = m.write()?;
            crate::jsvalue::obj_insert(&mut w, Rc::from(crate::jsvalue::to_string(k)), v);
            Ok(())
        }
        Value::List(l) => {
            if matches!(k, Value::Str(s) if &**s == "length") {
                let n = crate::jsvalue::to_number(&v);
                if n.fract() != 0.0 || n < 0.0 || n > u32::MAX as f64 {
                    return err("RangeError: Invalid array length");
                }
                if n as usize > l.read().len() {
                    return err("arrays cannot be lengthened by setting length (sparse arrays are not supported)");
                }
                l.write()?.truncate(n as usize);
                return Ok(());
            }
            let Some(i) = crate::jsvalue::array_index_of(k) else {
                return err(format!(
                    "cannot set property '{}' on an array (arrays hold only their items)",
                    crate::jsvalue::to_string(k)
                ));
            };
            let mut l = l.write()?;
            if i < l.len() {
                l[i] = v;
            } else if i == l.len() {
                l.push(v);
            } else {
                return err(format!(
                    "cannot set index {i} of an array of length {} (sparse arrays are not supported)",
                    l.len()
                ));
            }
            Ok(())
        }
        Value::Undef | Value::None => err(format!(
            "TypeError: Cannot set properties of {} (setting '{}')",
            crate::jsvalue::to_string(base),
            crate::jsvalue::to_string(k)
        )),
        Value::Str(_) | Value::Int(_) | Value::Float(_) | Value::BigInt(_) | Value::Bool(_) => err(format!(
            "TypeError: Cannot create property '{}' on {} '{}'",
            crate::jsvalue::to_string(k),
            crate::jsvalue::type_of(base),
            crate::jsvalue::to_string(base)
        )),
        other => err(format!(
            "TypeError: cannot set property '{}' on a {}",
            crate::jsvalue::to_string(k),
            other.type_name()
        )),
    }
}

/// The values `for..of` and `...` visit: array items, string characters.
pub fn iterate(v: &Value) -> RResult<Vec<Value>> {
    match v {
        Value::List(l) => Ok(l.read().clone()),
        // A Map yields its [key, value] entries; a Set its values.
        Value::Coll(c) => {
            let c = c.read();
            Ok(if c.set {
                c.iter().map(|(k, _)| k.clone()).collect()
            } else {
                c.iter().map(|(k, v)| Value::list(vec![k.clone(), v.clone()])).collect()
            })
        }
        Value::Str(s) => {
            crate::jsvalue::bulk_array(s.char_count())?;
            Ok(s.chars().map(|c| Value::string(c.to_string())).collect())
        }
        Value::Frag(_) if token_run(v).is_some() => {
            Ok(token_run(v).unwrap().iter().map(|&t| Value::Int(t as i64)).collect())
        }
        Value::Undef | Value::None => err(format!("TypeError: {} is not iterable", crate::jsvalue::to_string(v))),
        _ => err(format!("TypeError: {} is not iterable", crate::jsvalue::type_of(v))),
    }
}

pub fn binop(op: BinOp, a: &Value, b: &Value) -> RResult<Value> {
    use crate::jsvalue as js;
    use Value::*;
    Ok(match op {
        // Tokens and text concatenate into a fragment (a token is never text). Other values are
        // not silently printed into a prompt: convert them first (`String(x)`, `JSON.stringify(x)`).
        BinOp::Add if matches!(a, Token(..) | Frag(_)) || matches!(b, Token(..) | Frag(_)) => {
            let mut items = vec![];
            for v in [a, b] {
                match v {
                    Token(id, _) => items.push(crate::emit::Item::Tok(*id)),
                    Frag(f) => items.extend(f.iter().cloned()),
                    Str(s) => items.push(crate::emit::Item::Text(s.to_rc())),
                    o => {
                        return err(format!(
                            "TypeError: cannot add {} to a token fragment (convert it with String(x) first)",
                            js::to_string(o).chars().take(40).collect::<String>()
                        ));
                    }
                }
            }
            Frag(Rc::new(items))
        }
        BinOp::Add => js::add(a, b)?,
        BinOp::Sub => js::arith('-', a, b),
        BinOp::Mul => js::arith('*', a, b),
        BinOp::Div => js::arith('/', a, b),
        BinOp::Mod => js::arith('%', a, b),
        BinOp::Eq => Bool(js::loose_eq(a, b)),
        BinOp::Ne => Bool(!js::loose_eq(a, b)),
        BinOp::StrictEq => Bool(js::strict_eq(a, b)),
        BinOp::StrictNe => Bool(!js::strict_eq(a, b)),
        BinOp::Lt => Bool(js::compare("<", a, b)),
        BinOp::Le => Bool(js::compare("<=", a, b)),
        BinOp::Gt => Bool(js::compare(">", a, b)),
        BinOp::Ge => Bool(js::compare(">=", a, b)),
        BinOp::In => Bool(js::has(b, a)?),
        BinOp::Pow => js::arith('^', a, b),
        BinOp::BitAnd => Int((js::to_int32(a) & js::to_int32(b)) as i64),
        BinOp::BitOr => Int((js::to_int32(a) | js::to_int32(b)) as i64),
        BinOp::BitXor => Int((js::to_int32(a) ^ js::to_int32(b)) as i64),
        BinOp::Shl => Int(js::to_int32(a).wrapping_shl(js::to_uint32(b) & 31) as i64),
        BinOp::Shr => Int((js::to_int32(a) >> (js::to_uint32(b) & 31)) as i64),
        BinOp::UShr => Int((js::to_uint32(a) >> (js::to_uint32(b) & 31)) as i64),
        // Both lexical classes and intrinsic constructor aliases are runtime values.
        BinOp::InstanceOf if matches!(b, Class(_)) => {
            let Class(c) = b else { unreachable!() };
            Bool(matches!(a, Map(m) if m.class().is_some_and(|k| k.is_a(c))))
        }
        BinOp::InstanceOf => {
            let Native(name) = b else {
                return err("TypeError: Right-hand side of instanceof is not callable");
            };
            Bool(match &**name {
                "Array" => matches!(a, List(_)),
                "Object" => matches!(
                    a,
                    List(_) | Map(_) | Regex(_) | Native(_) | Lambda(..) | Func(_) | Setter(_) | Promise(_)
                ),
                "Function" => matches!(a, Native(_) | Lambda(..) | Func(_) | Setter(_)),
                "RegExp" => matches!(a, Regex(_)),
                "Promise" => matches!(a, Promise(_)),
                "Error" => matches!(a, Map(m) if m.class().is_some_and(|c| c.error_kind().is_some())),
                "TypeError" | "RangeError" | "SyntaxError" | "ReferenceError" | "URIError" | "EvalError"
                | "AggregateError" | "AbortError" | "TimeoutError" => {
                    matches!(a, Map(m) if m.class().is_some_and(|c| c.is_a(&error_class(name))))
                }
                "Map" => matches!(a, Coll(c) if !c.read().set),
                "Set" => matches!(a, Coll(c) if c.read().set),
                "String" | "Number" | "Boolean" | "Date" | "WeakMap" | "WeakSet" => false,
                other => {
                    return err(format!(
                        "TypeError: Right-hand side of 'instanceof' is not callable (`{other}` is not a built-in constructor)"
                    ));
                }
            })
        }
    })
}

fn strip_set(s: &str, chars: Option<&str>, left: bool, right: bool) -> String {
    let pred = |c: char| match chars {
        Some(cs) => cs.contains(c),
        // ECMAScript white space and line terminators.
        None => crate::jsvalue::is_js_space(c),
    };
    let mut t = s;
    if left {
        t = t.trim_start_matches(pred);
    }
    if right {
        t = t.trim_end_matches(pred);
    }
    t.to_string()
}

/// JavaScript globals: `Math.*`, `Object.*`, `JSON.*`, `Array.*`, `Number.*`, `String(..)`, `parseInt`, ...
fn js_global(name: &str, a: &[Value]) -> RResult<Value> {
    use crate::jsvalue as js;
    let a0 = a.first().cloned().unwrap_or(Value::Undef);
    if let Some(v) = crate::jsbuiltins::math(name, a) {
        return Ok(v);
    }
    if let Some(r) = crate::jsbuiltins::object_global(name, a) {
        return r;
    }
    Ok(match name {
        "Math.random" => {
            return err("Math.random() would make renders non-deterministic; use await harness.random() in a handler");
        }
        "Array.isArray" => Value::Bool(matches!(a0, Value::List(_))),
        // A plain object (not an array, string, fragment, token or null).
        "isPlainObject" => Value::Bool(matches!(a0, Value::Map(_))),
        "Array.of" => Value::list(a.to_vec()),
        "Number.isInteger" => Value::Bool(
            matches!(a0, Value::Int(_)) || matches!(a0, Value::Float(f) if f.fract() == 0.0 && f.is_finite()),
        ),
        "Number.isFinite" => {
            Value::Bool(matches!(a0, Value::Int(_) | Value::Float(_)) && js::to_number(&a0).is_finite())
        }
        "isFinite" => Value::Bool(js::to_number(&a0).is_finite()),
        "Number.isNaN" => Value::Bool(matches!(a0, Value::Float(f) if f.is_nan())),
        "isNaN" => Value::Bool(js::to_number(&a0).is_nan()),
        "Number.parseFloat" | "parseFloat" => crate::jsbuiltins::parse_float(&a0),
        "Number.parseInt" | "parseInt" => crate::jsbuiltins::parse_int(&a0, a.get(1)),
        "Error" | "TypeError" | "RangeError" | "SyntaxError" | "ReferenceError" | "URIError" | "EvalError" => {
            let message = if matches!(a0, Value::Undef) {
                String::new()
            } else {
                js::to_string(&a0)
            };
            let e = make_error(name, &message, None, None);
            if let (Value::Map(m), Some(Value::Map(o))) = (&e, a.get(1))
                && let Some(c) = o.read().get("cause")
            {
                m.ext()
                    .unwrap()
                    .hidden
                    .borrow_mut()
                    .insert(Rc::from("cause"), c.clone());
            }
            e
        }
        // Text helpers for exact output (the official templates print Python-formatted values).
        // `formatJSON(value, {indent, separators: [item, key], ascii, sortKeys})`: JSON text with
        // explicit formatting; defaults ", " / ": " (item / key separators), "," with an indent.
        "formatJSON" => {
            let opt = |k: &str| match &a.get(1) {
                Some(Value::Map(m)) => m.read().get(k).cloned(),
                _ => None,
            };
            let indent = match opt("indent") {
                Some(Value::Int(n)) if n > 64 => return err("RangeError: formatJSON indent is at most 64"),
                Some(Value::Int(n)) => Some(" ".repeat(n.max(0) as usize)),
                Some(Value::Str(s)) => Some(s.to_string()),
                _ => None,
            };
            let (item_sep, key_sep) = match opt("separators") {
                Some(Value::List(l)) if l.read().len() == 2 => {
                    (js::to_string(&l.read()[0]), js::to_string(&l.read()[1]))
                }
                _ => (if indent.is_some() { ",".into() } else { ", ".into() }, ": ".into()),
            };
            let o = crate::value::JsonOpts {
                indent,
                item_sep: &item_sep,
                key_sep: &key_sep,
                ensure_ascii: opt("ascii").is_some_and(|v| js::truthy(&v)),
                sort_keys: opt("sortKeys").is_some_and(|v| js::truthy(&v)),
            };
            // The same bounds as JSON.stringify: a value that contains itself, or nests past 400
            // levels, is an error and never a native recursion without end.
            match crate::value::check_graph(&a0, 400) {
                Ok(()) => {}
                Err(crate::value::GraphFault::Cycle) => {
                    return err("TypeError: Converting circular structure to JSON");
                }
                Err(_) => return err("RangeError: JSON nesting exceeds 400 levels"),
            }
            let mut s = String::new();
            crate::value::json_dumps(&a0, &o, 0, &mut s)?;
            Value::string(s)
        }
        // The text the official (Python) templates print for a value: `True`, `None`, `1.0`, `{'a': 1}`.
        // `str(x)`. Laid-out text (a fragment, e.g. a template app's screen) is its own text: it
        // passes through, keeping its tokens and spans.
        "formatPythonValue" => match a0 {
            Value::Frag(_) => a0,
            _ => Value::string(a0.to_py_str()),
        },
        "formatPythonRepr" => {
            let mut s = String::new();
            a0.py_repr(&mut s);
            Value::string(s)
        }
        // `trimChars(s, chars)` etc.: strip any of `chars` (whitespace if omitted) from the ends.
        "trimChars" | "trimStartChars" | "trimEndChars" => {
            let chars = a
                .get(1)
                .filter(|c| !matches!(c, Value::Undef | Value::None))
                .map(js::to_string);
            Value::string(strip_set(
                &js::to_string(&a0),
                chars.as_deref(),
                name != "trimEndChars",
                name != "trimStartChars",
            ))
        }
        // First character upper case, the rest lower case; and every word so.
        "capitalize" => {
            let s = js::to_string(&a0);
            let mut c = s.chars();
            Value::string(match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + &c.as_str().to_lowercase(),
                None => String::new(),
            })
        }
        "titleCase" => {
            let mut out = String::new();
            let mut prev_cased = false;
            for ch in js::to_string(&a0).chars() {
                if ch.is_alphabetic() {
                    if prev_cased {
                        out.extend(ch.to_lowercase())
                    } else {
                        out.extend(ch.to_uppercase())
                    }
                    prev_cased = true;
                } else {
                    out.push(ch);
                    prev_cased = false;
                }
            }
            Value::string(out)
        }
        "Number" => js::number(js::to_number(&a0)),
        "String" => Value::string(js::try_to_string(&a0)?),
        "Boolean" => Value::Bool(js::truthy(&a0)),
        other => return err(format!("unknown function `{other}`")),
    })
}

/// What a value is if it cannot cross an instance boundary as JSON (`<Mount>` props), else `None`.
/// The walk is iterative and visits each container once; a cycle, or nesting past
/// `MAX_DATA_NESTING`, is refused as state is.
fn not_json(v: &Value) -> Option<String> {
    let mut seen = std::collections::HashSet::new();
    let mut work = vec![v.clone()];
    while let Some(v) = work.pop() {
        match &v {
            Value::Method(_)
            | Value::Native(_)
            | Value::Lambda(..)
            | Value::Func(_)
            | Value::Bound(_)
            | Value::Setter(_) => return Some("a function".into()),
            Value::Frag(_) => return Some("markup".into()),
            Value::Token(..) => return Some("a special token".into()),
            Value::Promise(_) => return Some("a promise".into()),
            Value::Regex(_) => return Some("a regular expression".into()),
            // Reversed: the first offending item (in order) is the one named.
            Value::List(l) if seen.insert(Rc::as_ptr(l) as usize) => work.extend(l.read().iter().rev().cloned()),
            Value::Map(m) if seen.insert(Rc::as_ptr(m) as usize) => work.extend(m.read().values().rev().cloned()),
            _ => {}
        }
    }
    if seen.is_empty() {
        return None;
    }
    match crate::value::check_graph(v, crate::value::MAX_DATA_NESTING) {
        Ok(()) => None,
        Err(crate::value::GraphFault::Cycle) => Some("a value that contains itself".into()),
        Err(crate::value::GraphFault::Deep) => Some(format!(
            "nested more than {} levels deep",
            crate::value::MAX_DATA_NESTING
        )),
    }
}

// ---------------- classes ----------------

/// A member read without calling anything (`read_path`, compound assignment): own data, a private
/// member, an own non-enumerable property, a class's static; else `jsvalue::index`.
fn member_read(v: &Value, k: &Value) -> RResult<Value> {
    if let Value::Map(m) = v
        && let (Some(e), Value::Str(name)) = (m.ext(), k)
        && !m.read().contains_key(&**name)
        && let Some(x) = e.hidden.borrow().get(&**name)
    {
        return Ok(x.clone());
    }
    if let Value::Class(c) = v {
        let name = crate::jsvalue::to_string(k);
        return Ok(static_member(c, &name).unwrap_or(Value::Undef));
    }
    crate::jsvalue::index(v, k)
}

/// A class's static member, own or inherited (static members are inherited along `extends`).
fn static_member(c: &Rc<crate::value::Class>, name: &str) -> Option<Value> {
    let mut k = Some(c.clone());
    while let Some(x) = k {
        if let Some(v) = x.statics.read().get(name) {
            return Some(
                if x.static_methods
                    .get(name)
                    .is_some_and(|f| crate::jsvalue::strict_eq(f, v))
                {
                    method_value(v.clone(), x.clone())
                } else {
                    v.clone()
                },
            );
        }
        k = x.parent.clone();
    }
    if name == "name" {
        return Some(Value::str(&c.name));
    }
    None
}

/// The built-in error class `kind` (`Error`, `TypeError`, ..): what `extends Error` extends.
pub(crate) fn error_class(kind: &str) -> Rc<crate::value::Class> {
    thread_local! { static CLASSES: std::cell::RefCell<HashMap<String, Rc<crate::value::Class>>> = Default::default(); }
    CLASSES.with(|c| {
        if let Some(x) = c.borrow().get(kind) {
            return x.clone();
        }
        let parent = (kind != "Error").then(|| error_class("Error"));
        let x = Rc::new(crate::value::Class {
            definition: None,
            name: Rc::from(kind),
            parent,
            error: Some(Rc::from(kind)),
            ctor: None,
            methods: Map::default(),
            getters: Map::default(),
            setters: Map::default(),
            fields: vec![],
            private_members: Default::default(),
            private_statics: Rc::new(crate::value::Obj::frozen(Map::default())),
            static_methods: Map::default(),
            statics: Rc::new(crate::value::Obj::frozen(Map::default())),
            static_getters: Map::default(),
            static_setters: Map::default(),
        });
        c.borrow_mut().insert(kind.to_string(), x.clone());
        x
    })
}

/// An error, as JavaScript's are: an instance of the built-in class `kind`, its `message` and
/// `stack` own but not enumerable (`Object.keys(e)` is `[]`, `JSON.stringify(e)` is `"{}"`).
pub(crate) fn make_error(kind: &str, message: &str, stack: Option<String>, value: Option<Value>) -> Value {
    let class = error_class(kind);
    let obj = crate::value::Obj::instance(Map::default(), class);
    {
        let e = obj.ext().unwrap();
        let mut h = e.hidden.borrow_mut();
        h.insert(Rc::from("message"), Value::str(message));
        if let Some(s) = stack {
            h.insert(Rc::from("stack"), Value::string(s));
        }
        if let Some(v) = value {
            h.insert(Rc::from("value"), v);
        }
    }
    Value::Map(Rc::new(obj))
}

fn is_error_kind(name: &str) -> bool {
    matches!(
        name,
        "Error" | "TypeError" | "RangeError" | "SyntaxError" | "ReferenceError" | "URIError" | "EvalError"
    )
}

impl Interp<'_> {
    /// Class declarations keep private slots separate from public property names.
    fn eval_class(
        &mut self,
        def: &crate::ast::ClassDef,
        definition: Option<Rc<crate::ast::ClassDef>>,
    ) -> RResult<Value> {
        use crate::ast::MemberKind;
        use crate::value::{Class, Field, PrivateMember};
        let parent = match &def.parent {
            None => None,
            Some(e) => match self.eval(e)? {
                Value::Class(c) => Some(c),
                Value::Native(n) if is_error_kind(&n) => Some(error_class(&n)),
                other => {
                    return err(format!(
                        "TypeError: Class extends value {} is not a constructor",
                        crate::jsvalue::to_string(&other)
                    ));
                }
            },
        };
        let ctor = def
            .ctor
            .as_ref()
            .map(|l| self.eval(&Expr::Lambda(l.clone())))
            .transpose()?;
        let (
            mut methods,
            mut getters,
            mut setters,
            mut statics,
            mut static_methods,
            mut static_getters,
            mut static_setters,
        ) = (
            Map::default(),
            Map::default(),
            Map::default(),
            Map::default(),
            Map::default(),
            Map::default(),
            Map::default(),
        );
        let mut private_members: indexmap::IndexMap<Rc<str>, PrivateMember, ahash::RandomState> = Default::default();
        let mut fields = vec![];
        let mut static_fields = vec![];
        for member in &def.members {
            let value = member
                .value
                .as_ref()
                .map(|l| self.eval(&Expr::Lambda(l.clone())))
                .transpose()?;
            if member.private {
                let slot = private_members.entry(member.key.clone()).or_default();
                if (slot.field || slot.method.is_some())
                    || ((slot.get.is_some() || slot.set.is_some())
                        && (slot.is_static != member.is_static
                            || !matches!(member.kind, MemberKind::Get | MemberKind::Set)))
                {
                    return err(format!("SyntaxError: duplicate private member {}", member.key));
                }
                slot.is_static = member.is_static;
                match member.kind {
                    MemberKind::Field => slot.field = true,
                    MemberKind::Method => slot.method = value.clone(),
                    MemberKind::Get => {
                        if slot.get.is_some() {
                            return err("SyntaxError: duplicate private getter");
                        }
                        slot.get = value.clone();
                    }
                    MemberKind::Set => {
                        if slot.set.is_some() {
                            return err("SyntaxError: duplicate private setter");
                        }
                        slot.set = value.clone();
                    }
                }
            }
            match (member.kind, member.is_static, member.private) {
                (MemberKind::Field, false, private) => fields.push(Field {
                    name: member.key.clone(),
                    init: value,
                    private,
                }),
                (MemberKind::Field, true, private) => static_fields.push((member.key.clone(), value, private)),
                (_, _, true) => {}
                (MemberKind::Method, false, false) => {
                    methods.insert(member.key.clone(), value.unwrap());
                }
                (MemberKind::Get, false, false) => {
                    getters.insert(member.key.clone(), value.unwrap());
                }
                (MemberKind::Set, false, false) => {
                    setters.insert(member.key.clone(), value.unwrap());
                }
                (MemberKind::Method, true, false) => {
                    let value = value.unwrap();
                    static_methods.insert(member.key.clone(), value.clone());
                    statics.insert(member.key.clone(), value);
                }
                (MemberKind::Get, true, false) => {
                    static_getters.insert(member.key.clone(), value.unwrap());
                }
                (MemberKind::Set, true, false) => {
                    static_setters.insert(member.key.clone(), value.unwrap());
                }
            }
        }
        let class = Rc::new(Class {
            definition,
            name: def.name.clone().unwrap_or_else(|| Rc::from("")),
            parent,
            error: None,
            ctor,
            methods,
            getters,
            setters,
            fields,
            private_members,
            private_statics: Rc::new(crate::value::Obj::new(Map::default())),
            statics: Rc::new(crate::value::Obj::new(statics)),
            static_methods,
            static_getters,
            static_setters,
        });
        for (key, initializer, private) in static_fields {
            let value = if let Some(f) = initializer {
                self.pending_this = Some(Value::Class(class.clone()));
                self.pending_super = Some(class.clone());
                self.call_value(&f, vec![])?
            } else {
                Value::Undef
            };
            let store = if private {
                &class.private_statics
            } else {
                &class.statics
            };
            crate::value::note_write(&Value::Map(store.clone()), &value);
            store.write()?.insert(key, value);
        }
        Ok(Value::Class(class))
    }

    /// `new Name(args)`: a class in scope, else a built-in constructor.
    fn construct_value(&mut self, constructor: &Value, args: Vec<Value>) -> RResult<Value> {
        match constructor {
            Value::Class(c) => self.construct(c, args),
            Value::Native(n) if n.owner.is_none() && !n.is_namespace() && !n.contains('.') => self.builtin(n, args),
            _ => err("TypeError: value is not a constructor (only classes and built-in constructors are supported)"),
        }
    }

    /// A new instance of `class`.
    pub(crate) fn construct(&mut self, class: &Rc<crate::value::Class>, args: Vec<Value>) -> RResult<Value> {
        let obj = Value::Map(Rc::new(crate::value::Obj::instance(Map::default(), class.clone())));
        let r = self.construct_into(class, args, &obj)?;
        // A constructor that returns an object returns that object instead.
        Ok(match r {
            Some(v @ (Value::Map(_) | Value::List(_) | Value::Coll(_))) => v,
            _ => obj,
        })
    }

    /// Run `class`'s part of making `obj`: a base class's fields then its constructor; a derived
    /// class's constructor, whose `super(..)` makes the parent's part and then this class's fields.
    fn construct_into(
        &mut self,
        class: &Rc<crate::value::Class>,
        args: Vec<Value>,
        obj: &Value,
    ) -> RResult<Option<Value>> {
        let derived = class.parent.is_some();
        if !derived {
            if let Some(kind) = &class.error {
                self.init_error(kind, &args, obj);
                return Ok(None);
            }
            self.init_fields(class, obj)?;
        }
        match &class.ctor {
            Some(f) => {
                self.pending_this = Some(obj.clone());
                self.pending_super = Some(class.clone());
                Ok(Some(self.call_value(f, args)?))
            }
            None => {
                if derived {
                    self.construct_into(class.parent.as_ref().unwrap(), args, obj)?;
                    self.init_fields(class, obj)?;
                }
                Ok(None)
            }
        }
    }

    /// An error's own properties: `message` (when given) and `stack`, not enumerable.
    fn init_error(&mut self, _kind: &str, args: &[Value], obj: &Value) {
        let Value::Map(m) = obj else { return };
        let Some(e) = m.ext() else { return };
        if let Some(msg) = args.first().filter(|a| !matches!(a, Value::Undef)) {
            e.hidden
                .borrow_mut()
                .insert(Rc::from("message"), Value::string(crate::jsvalue::to_string(msg)));
        }
        if let Some(Value::Map(o)) = args.get(1)
            && let Some(c) = o.read().get("cause")
        {
            e.hidden.borrow_mut().insert(Rc::from("cause"), c.clone());
        }
    }

    fn init_fields(&mut self, class: &Rc<crate::value::Class>, obj: &Value) -> RResult<()> {
        let Value::Map(object) = obj else { return Ok(()) };
        if let Some(ext) = object.ext() {
            ext.brands.borrow_mut().insert(class.brand());
        }
        for field in &class.fields {
            let value = if let Some(f) = &field.init {
                self.pending_this = Some(obj.clone());
                self.pending_super = Some(class.clone());
                self.call_value(f, vec![])?
            } else {
                Value::Undef
            };
            crate::value::note_write(obj, &value);
            if field.private {
                object
                    .ext()
                    .unwrap()
                    .private
                    .borrow_mut()
                    .insert((class.brand(), field.name.clone()), value);
            } else {
                crate::jsvalue::obj_insert(&mut *object.write()?, field.name.clone(), value);
            }
        }
        Ok(())
    }

    /// `super(args)` in a derived constructor.
    fn super_call(&mut self, args: Vec<Value>) -> RResult<Value> {
        let (Value::Class(home), this) = (self.lookup("super"), self.lookup("this")) else {
            return err("SyntaxError: 'super' keyword unexpected here");
        };
        let Some(parent) = home.parent.clone() else {
            return err("SyntaxError: 'super' keyword unexpected here");
        };
        if let Some(kind) = &parent.error
            && (parent.parent.is_none() || parent.ctor.is_none())
        {
            let k = kind.clone();
            self.init_error(&k, &args, &this);
            self.init_fields(&home, &this)?;
            return Ok(Value::Undef);
        }
        self.construct_into(&parent, args, &this)?;
        self.init_fields(&home, &this)?;
        Ok(Value::Undef)
    }

    /// `super.m(args)` in a method: the method of the class above the method's own, on `this`.
    fn super_method(&mut self, name: &str, args: Vec<Value>) -> RResult<Value> {
        let (f, this) = self.super_reference(name)?;
        self.call_with_receiver(&f, this, args)
    }
    fn super_reference(&mut self, name: &str) -> RResult<(Value, Value)> {
        let (Value::Class(home), this) = (self.lookup("super"), self.lookup("this")) else {
            return err("SyntaxError: 'super' keyword unexpected here");
        };
        let parent = home
            .parent
            .as_ref()
            .ok_or_else(|| format!("TypeError: (intermediate value).{name} is not a function"))?;
        if let Some((owner, f)) = parent.find(name, |c| &c.methods) {
            return Ok((method_value(f, owner), this));
        }
        if name == "toString" && parent.error_kind().is_some() {
            return Ok((
                crate::intrinsic::Native::method(crate::intrinsic::Prototype::Error, "toString"),
                this,
            ));
        }
        err(format!("TypeError: (intermediate value).{name} is not a function"))
    }

    fn private_owner(&self, name: &str) -> RResult<Rc<crate::value::Class>> {
        if let Some(Value::List(context)) = self.local("\0private.context") {
            for value in context.read().iter() {
                if let Value::Class(owner) = value
                    && owner.private_members.contains_key(name)
                {
                    return Ok(owner.clone());
                }
            }
        }
        err(format!(
            "SyntaxError: private member {name} is not declared in this lexical class"
        ))
    }
    fn private_receiver(&self, receiver: &Value, owner: &Rc<crate::value::Class>, name: &str) -> RResult<()> {
        let member = owner
            .private_members
            .get(name)
            .ok_or("SyntaxError: unknown private member")?;
        let valid = if member.is_static {
            matches!(receiver,Value::Class(class) if Rc::ptr_eq(class,owner))
        } else {
            matches!(receiver,Value::Map(object) if object.ext().is_some_and(|ext|ext.brands.borrow().iter().any(|brand|*brand==owner.brand())))
        };
        if valid {
            Ok(())
        } else {
            err(format!(
                "TypeError: Cannot access private member {name} on an object of another class"
            ))
        }
    }
    fn get_private(&mut self, receiver: &Value, owner: &Rc<crate::value::Class>, name: &str) -> RResult<Value> {
        self.private_receiver(receiver, owner, name)?;
        let member = &owner.private_members[name];
        if member.field {
            let value = if member.is_static {
                owner.private_statics.read().get(name).cloned()
            } else {
                let Value::Map(object) = receiver else { unreachable!() };
                object
                    .ext()
                    .unwrap()
                    .private
                    .borrow()
                    .iter()
                    .find(|((brand, key), _)| *brand == owner.brand() && &**key == name)
                    .map(|(_, v)| v.clone())
            };
            return value.ok_or_else(|| format!("TypeError: private field {name} is not initialized"));
        }
        if let Some(function) = &member.method {
            return Ok(method_value(function.clone(), owner.clone()));
        }
        if let Some(function) = &member.get {
            let function = function.clone();
            self.pending_this = Some(receiver.clone());
            self.pending_super = Some(owner.clone());
            return self.call_value(&function, vec![]);
        }
        err(format!("TypeError: private member {name} has no getter"))
    }
    fn set_private(
        &mut self,
        receiver: &Value,
        owner: &Rc<crate::value::Class>,
        name: &str,
        value: Value,
    ) -> RResult<()> {
        self.private_receiver(receiver, owner, name)?;
        let member = &owner.private_members[name];
        if let Some(function) = &member.set {
            let function = function.clone();
            self.pending_this = Some(receiver.clone());
            self.pending_super = Some(owner.clone());
            self.call_value(&function, vec![value])?;
            return Ok(());
        }
        if !member.field {
            return err(format!("TypeError: private member {name} is read only"));
        }
        if member.is_static {
            if !owner.private_statics.read().contains_key(name) {
                return err("TypeError: private field is not initialized");
            }
            crate::value::note_write(&Value::Map(owner.private_statics.clone()), &value);
            owner.private_statics.write()?.insert(Rc::from(name), value);
        } else {
            let Value::Map(object) = receiver else { unreachable!() };
            let ext = object.ext().unwrap();
            let key = ext
                .private
                .borrow()
                .keys()
                .find(|(brand, key)| *brand == owner.brand() && &**key == name)
                .cloned()
                .ok_or("TypeError: private field is not initialized")?;
            if object.is_deep() {
                return err(
                    "TypeError: Cannot change this value: it is kept by the runtime and is frozen; copy it to change it",
                );
            }
            crate::value::note_write(receiver, &value);
            ext.private.borrow_mut().insert(key, value);
        }
        Ok(())
    }

    /// Error.prototype.toString is distinct from Object.prototype.toString, and can be
    /// borrowed with call/apply. Its fields use normal property lookup and conversion.
    fn error_string(&mut self, receiver: &Value) -> RResult<Value> {
        if !receiver.is_object() {
            return err("TypeError: Error.prototype.toString requires an object");
        }
        let name = self.get_prop(receiver, "name")?;
        let name = if matches!(name, Value::Undef) {
            "Error".to_string()
        } else {
            crate::jsvalue::try_to_string(&name)?
        };
        let message = self.get_prop(receiver, "message")?;
        let message = if matches!(message, Value::Undef) {
            String::new()
        } else {
            crate::jsvalue::try_to_string(&message)?
        };
        Ok(Value::string(if name.is_empty() {
            message
        } else if message.is_empty() {
            name
        } else {
            format!("{name}: {message}")
        }))
    }

    /// `o.k`: own data, a getter or a method of its class, a static member.
    pub(crate) fn get_prop(&mut self, v: &Value, k: &str) -> RResult<Value> {
        match v {
            Value::Map(m) if m.ext().is_some() => {
                let e = m.ext().unwrap();
                if let Some(x) = m.read().get(k) {
                    return Ok(x.clone());
                }
                if let Some(x) = e.hidden.borrow().get(k) {
                    return Ok(x.clone());
                }
                let class = e.class.clone();
                if let Some((owner, g)) = class.find(k, |c| &c.getters) {
                    self.pending_this = Some(v.clone());
                    self.pending_super = Some(owner);
                    return self.call_value(&g, vec![]);
                }
                if let Some((owner, f)) = class.find(k, |c| &c.methods) {
                    return Ok(method_value(f, owner));
                }
                if k == "name"
                    && let Some(kind) = class.error_kind()
                {
                    return Ok(Value::Str((&kind).into()));
                }
                if k == "message" && class.error_kind().is_some() {
                    return Ok(Value::str(""));
                }
                if k == "constructor" {
                    return Ok(Value::Class(class));
                }
                Ok(crate::intrinsic::member(v, k).unwrap_or(Value::Undef))
            }
            Value::Class(c) => {
                if let Some(x) = static_member(c, k) {
                    return Ok(x);
                }
                let mut cl = Some(c.clone());
                while let Some(x) = cl {
                    if let Some(g) = x.static_getters.get(k) {
                        let g = g.clone();
                        self.pending_this = Some(v.clone());
                        self.pending_super = Some(x.clone());
                        return self.call_value(&g, vec![]);
                    }
                    cl = x.parent.clone();
                }
                Ok(crate::intrinsic::member(v, k).unwrap_or(Value::Undef))
            }
            _ => get_attr(v, k),
        }
    }

    /// `o[k] = v` on the object a path named: through a setter or into a private member when its
    /// class has one; a class's static; else an ordinary write.
    pub(crate) fn set_member(&mut self, base: &Value, k: &Value, v: Value) -> RResult<()> {
        match (base, k) {
            (Value::Map(m), Value::Str(name)) if m.ext().is_some() => {
                let e = m.ext().unwrap();
                if !m.read().contains_key(&**name) {
                    let mut class = Some(e.class.clone());
                    while let Some(owner) = class {
                        if owner.methods.contains_key(&**name) {
                            break;
                        }
                        if let Some(setter) = owner.setters.get(&**name).cloned() {
                            self.pending_this = Some(base.clone());
                            self.pending_super = Some(owner);
                            self.call_value(&setter, vec![v])?;
                            return Ok(());
                        }
                        if owner.getters.contains_key(&**name) {
                            return err(format!(
                                "TypeError: Cannot set property {} which has only a getter",
                                &**name
                            ));
                        }
                        class = owner.parent.clone();
                    }
                }
                set_prop(base, k, v)
            }
            (Value::Class(c), _) => {
                let name = crate::jsvalue::to_string(k);
                let mut cl = Some(c.clone());
                while let Some(x) = cl {
                    if x.statics.read().contains_key(name.as_str()) {
                        break;
                    }
                    if let Some(s) = x.static_setters.get(name.as_str()) {
                        let s = s.clone();
                        self.pending_this = Some(base.clone());
                        self.pending_super = Some(x.clone());
                        self.call_value(&s, vec![v])?;
                        return Ok(());
                    }
                    if x.static_getters.contains_key(name.as_str()) {
                        return err(format!("TypeError: Cannot set property {name} which has only a getter"));
                    }
                    cl = x.parent.clone();
                }
                crate::value::note_write(&Value::Map(c.statics.clone()), &v);
                let mut w = c.statics.write()?;
                crate::jsvalue::obj_insert(&mut w, Rc::from(name), v);
                Ok(())
            }
            (Value::Regex(r), Value::Str(name)) if &**name == "lastIndex" => {
                if r.frozen.get() {
                    return err("TypeError: Cannot assign to read only property 'lastIndex' of object");
                }
                r.last_index.set(crate::jsvalue::to_number(&v).max(0.0) as usize);
                Ok(())
            }
            _ => set_prop(base, k, v),
        }
    }
}

fn method_value(function: Value, home: Rc<crate::value::Class>) -> Value {
    Value::Method(Rc::new(crate::value::Method { function, home }))
}
