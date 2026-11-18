//! Runtime values: JSON-shaped, plus fragments, tokens, functions, promises and regexes. The
//! language's semantics are JavaScript (`jsvalue.rs`). The Python text formats here (`str()`,
//! `repr()`, `json.dumps`) back `formatPythonValue`, `formatPythonRepr` and `formatJSON`: the
//! official chat templates print values that way, and views reproduce them byte for byte.

pub(crate) use crate::intrinsic::{Native, Prototype};
use indexmap::IndexMap;
use std::fmt::Write;
use std::rc::Rc;

use crate::ast::Lambda;

/// An object's properties, in insertion order.
pub type Map = IndexMap<Rc<str>, Value, ahash::RandomState>;

/// An array's or an object's storage: shared by every reference to it, as in JavaScript.
/// A write goes through `write`, which refuses a frozen object with
/// JavaScript's `TypeError`. Two flags: `Object.freeze`'s (shallow, as in JavaScript), and the
/// runtime's (`freeze`: deep; what the runtime keeps never changes, so a pointer still names its
/// contents).
pub struct Obj<T: Contents> {
    flags: std::cell::Cell<u8>,
    /// With `CHECKED`: the nesting height of this (deeply frozen) subtree.
    height: std::cell::Cell<u8>,
    data: std::cell::RefCell<T>,
    /// A class instance's class, its private members and its own non-enumerable properties (an
    /// error's `message`); none for a plain object or array.
    ext: std::cell::OnceCell<Box<Ext>>,
}

/// What an instance has beyond its own enumerable properties.
pub struct Ext {
    pub(crate) class: Rc<Class>,
    pub(crate) private: std::cell::RefCell<PrivateMap>,
    pub(crate) brands: std::cell::RefCell<std::collections::HashSet<ClassBrand>>,
    pub(crate) hidden: std::cell::RefCell<Map>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ClassBrand {
    Definition(usize),
    Evaluation(usize),
}
pub(crate) type PrivateMap = IndexMap<(ClassBrand, Rc<str>), Value, ahash::RandomState>;
pub(crate) struct Field {
    pub name: Rc<str>,
    pub init: Option<Value>,
    pub private: bool,
}
#[derive(Default)]
pub(crate) struct PrivateMember {
    pub is_static: bool,
    pub field: bool,
    pub method: Option<Value>,
    pub get: Option<Value>,
    pub set: Option<Value>,
}
#[derive(Debug)]
pub struct Method {
    pub(crate) function: Value,
    pub(crate) home: Rc<Class>,
}

/// A class (`class A extends B {..}`, or a built-in error class): what its instances share.
pub struct Class {
    /// A module declaration's stable type brand. Its value/statics are evaluation-local.
    pub(crate) definition: Option<Rc<crate::ast::ClassDef>>,
    pub(crate) name: Rc<str>,
    pub(crate) parent: Option<Rc<Class>>,
    /// A built-in error class's name (`Error`, `TypeError`, ..): its instances are errors.
    pub(crate) error: Option<Rc<str>>,
    pub(crate) ctor: Option<Value>,
    /// The prototype's methods, getters and setters, by name (private ones start with `#`).
    pub(crate) methods: Map,
    pub(crate) getters: Map,
    pub(crate) setters: Map,
    /// Instance fields, in order: name and initializer (a function called with `this`).
    pub(crate) fields: Vec<Field>,
    pub(crate) private_members: IndexMap<Rc<str>, PrivateMember, ahash::RandomState>,
    pub(crate) private_statics: Rc<Object>,
    pub(crate) static_methods: Map,
    /// Static members (fields and methods), and static accessors.
    pub(crate) statics: Rc<Object>,
    pub(crate) static_getters: Map,
    pub(crate) static_setters: Map,
}

impl Class {
    /// The identity `instanceof` and private members check.
    pub(crate) fn brand(self: &Rc<Self>) -> ClassBrand {
        self.definition.as_ref().map_or_else(
            || ClassBrand::Evaluation(Rc::as_ptr(self) as usize),
            |d| ClassBrand::Definition(Rc::as_ptr(d) as usize),
        )
    }
    /// The class in this chain that has `name` in the table `which` picks, and the member.
    pub(crate) fn find(self: &Rc<Self>, name: &str, which: fn(&Class) -> &Map) -> Option<(Rc<Class>, Value)> {
        let mut c = Some(self.clone());
        while let Some(k) = c {
            if let Some(v) = which(&k).get(name) {
                return Some((k.clone(), v.clone()));
            }
            c = k.parent.clone();
        }
        None
    }
    /// The built-in error class at the root of this chain, if any.
    pub(crate) fn error_kind(&self) -> Option<Rc<str>> {
        if let Some(e) = &self.error {
            return Some(e.clone());
        }
        self.parent.as_ref().and_then(|p| p.error_kind())
    }
    /// `x instanceof C`: `C` is this class or one of its ancestors.
    pub(crate) fn is_a(self: &Rc<Self>, c: &Rc<Class>) -> bool {
        let mut k = Some(self.clone());
        while let Some(x) = k {
            if x.brand() == c.brand() {
                return true;
            }
            k = x.parent.clone();
        }
        false
    }
}

impl std::fmt::Debug for Class {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "class {}", self.name)
    }
}

const FROZEN: u8 = 1;
const DEEP: u8 = 2;
/// Made by a cache (a `memoScan` step), not by the app's state: see `freeze_derived`.
const DERIVED: u8 = 4;
/// A deeply frozen subtree of plain data (no functions) the state check has passed: acyclic,
/// `height` levels deep, and unable to reach a closure, so later state checks and the cycle
/// collector stop here and a write costs what it adds, not the state's size.
const CHECKED: u8 = 8;
/// A deeply frozen subtree whose large strings are already on disk (`Blobs::lower_value`).
const LOWERED: u8 = 16;

impl<T: Contents> Obj<T> {
    /// A new, unfrozen object holding `data`.
    pub fn new(data: T) -> Obj<T> {
        Obj {
            flags: std::cell::Cell::new(0),
            height: std::cell::Cell::new(0),
            data: std::cell::RefCell::new(data),
            ext: Default::default(),
        }
    }
    /// Already deeply frozen (data the runtime made and keeps).
    pub fn frozen(data: T) -> Obj<T> {
        Obj {
            flags: std::cell::Cell::new(FROZEN | DEEP),
            height: std::cell::Cell::new(0),
            data: std::cell::RefCell::new(data),
            ext: Default::default(),
        }
    }
    /// An instance of `class`.
    pub(crate) fn instance(data: T, class: Rc<Class>) -> Obj<T> {
        let o = Obj::new(data);
        let _ = o.ext.set(Box::new(Ext {
            class,
            private: Default::default(),
            brands: Default::default(),
            hidden: Default::default(),
        }));
        o
    }
    /// The class data of a class instance, if it is one.
    pub(crate) fn ext(&self) -> Option<&Ext> {
        self.ext.get().map(|b| &**b)
    }
    /// The class it is an instance of, if any.
    pub(crate) fn class(&self) -> Option<&Rc<Class>> {
        self.ext().map(|e| &e.class)
    }
    /// Borrow the contents to read.
    pub fn read(&self) -> std::cell::Ref<'_, T> {
        self.data.borrow()
    }
    /// The contents to change, or JavaScript's error for a frozen object.
    pub fn write(&self) -> Result<std::cell::RefMut<'_, T>, String> {
        if self.flags.get() & FROZEN != 0 {
            return Err(if self.flags.get() & DEEP != 0 {
                "TypeError: Cannot change this value: it is kept by the runtime (state, props, a kept render or a cached result) and is frozen; copy it to change it ({...o}, [...xs], structuredClone(v))".into()
            } else {
                "TypeError: Cannot assign to read only property of a frozen object".into()
            });
        }
        self.data.try_borrow_mut().map_err(|_| "TypeError: an object cannot be changed while it is being read (a callback changed the object it is iterating)".to_string())
    }
    pub(crate) fn try_read(&self) -> Result<std::cell::Ref<'_, T>, std::cell::BorrowError> {
        self.data.try_borrow()
    }
    /// Whether writes to it are refused (`Object.freeze`, or kept by the runtime).
    pub fn is_frozen(&self) -> bool {
        self.flags.get() & FROZEN != 0
    }
    /// The contents, for the runtime's own bookkeeping (not a language write: no frozen check).
    pub fn data_mut(&self) -> Result<std::cell::RefMut<'_, T>, std::cell::BorrowMutError> {
        self.data.try_borrow_mut()
    }
    /// Whether everything reachable from it is frozen too.
    pub fn is_deep(&self) -> bool {
        self.flags.get() & DEEP != 0
    }
    /// `Object.freeze`: this object only.
    pub fn freeze_shallow(&self) {
        self.flags.set(self.flags.get() | FROZEN)
    }
    pub(crate) fn mark_deep(&self) {
        self.flags.set(self.flags.get() | FROZEN | DEEP)
    }
    pub(crate) fn mark_derived(&self) {
        self.flags.set(self.flags.get() | DERIVED)
    }
    /// Whether a cache step made it (see `freeze_derived`).
    pub fn is_derived(&self) -> bool {
        self.flags.get() & DERIVED != 0
    }
    /// The height of a checked, deeply frozen subtree.
    /// Plain frozen data the state check has passed: it cannot reach a function or a mutable object.
    pub(crate) fn is_checked(&self) -> bool {
        self.flags.get() & (CHECKED | DEEP) == CHECKED | DEEP
    }
    pub(crate) fn checked_height(&self) -> Option<usize> {
        (self.flags.get() & (CHECKED | DEEP) == CHECKED | DEEP).then(|| self.height.get() as usize)
    }
    fn mark_checked(&self, height: usize) {
        if self.is_deep() && height <= u8::MAX as usize {
            self.height.set(height as u8);
            self.flags.set(self.flags.get() | CHECKED);
        }
    }
    pub(crate) fn is_lowered(&self) -> bool {
        self.flags.get() & (LOWERED | DEEP) == LOWERED | DEEP
    }
    pub(crate) fn mark_lowered(&self) {
        if self.is_deep() {
            self.flags.set(self.flags.get() | LOWERED)
        }
    }
    /// The contents, taken when this is the only reference (an in-place fast path), else cloned.
    pub fn into_data(self: Rc<Self>) -> T
    where
        T: Clone + Default,
    {
        match Rc::try_unwrap(self) {
            Ok(mut o) => std::mem::take(o.data.get_mut()),
            Err(rc) => rc.data.borrow().clone(),
        }
    }
}

/// What an `Obj` holds: the values it owns, which its drop releases without recursing per level.
pub trait Contents {
    /// Drop the values, now.
    fn clear(&mut self);
    /// Move the values to `out`.
    fn drain_into(&mut self, out: &mut Vec<Value>);
}

impl Contents for Vec<Value> {
    fn clear(&mut self) {
        Vec::clear(self)
    }
    fn drain_into(&mut self, out: &mut Vec<Value>) {
        out.append(self)
    }
}

impl Contents for Map {
    fn clear(&mut self) {
        IndexMap::clear(self)
    }
    fn drain_into(&mut self, out: &mut Vec<Value>) {
        out.extend(self.drain(..).map(|(_, v)| v))
    }
}

impl Contents for Coll {
    fn clear(&mut self) {
        self.entries.clear()
    }
    fn drain_into(&mut self, out: &mut Vec<Value>) {
        out.extend(self.entries.drain(..).flatten().flat_map(|(k, v)| [k, v]))
    }
}

thread_local! {
    static DROP_DEPTH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    /// Whether `DEFERRED` holds values (read without touching it).
    static DEFERRING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static DEFERRED: std::cell::RefCell<Vec<Value>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Containers dropping inside each other this deep defer the rest to the outermost drop.
const DROP_NESTING: usize = 64;

impl<T: Contents> Drop for Obj<T> {
    // Drop glue recurses once per level: dropping a 100k-deep list would overflow the stack. The
    // values drop here instead, counted, and past `DROP_NESTING` levels move to a list that the
    // outermost drop empties. A shallow value costs a thread-local counter.
    fn drop(&mut self) {
        let ext = self.ext.get_mut().map(|e| (e.private.get_mut(), e.hidden.get_mut()));
        let data = self.data.get_mut();
        let depth = DROP_DEPTH.get();
        if depth >= DROP_NESTING {
            // (Not while the thread is exiting: then the glue drops them.)
            let _ = DEFERRED.try_with(|d| {
                let mut d = d.borrow_mut();
                data.drain_into(&mut d);
                if let Some((private, hidden)) = ext {
                    d.extend(private.drain(..).map(|(_, v)| v));
                    hidden.drain_into(&mut d);
                }
                DEFERRING.set(true);
            });
            return;
        }
        DROP_DEPTH.set(depth + 1);
        data.clear();
        if let Some((private, hidden)) = ext {
            private.clear();
            hidden.clear();
        }
        if depth == 0 && DEFERRING.get() {
            while let Some(v) = DEFERRED.try_with(|d| d.borrow_mut().pop()).ok().flatten() {
                drop(v);
            }
            DEFERRING.set(false);
        }
        DROP_DEPTH.set(depth);
    }
}

impl<T: Contents + std::fmt::Debug> std::fmt::Debug for Obj<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.data.try_borrow() {
            Ok(d) => d.fmt(f),
            Err(_) => write!(f, "<in use>"),
        }
    }
}

/// An array.
pub type Arr = Obj<Vec<Value>>;
/// A plain object or a class instance.
pub type Object = Obj<Map>;

/// A `Map` or a `Set` (JavaScript's): entries in insertion order, keyed by SameValueZero. A deleted
/// entry leaves a hole until the next compaction, so an iteration in progress sees what JavaScript's
/// does: entries deleted before it reaches them are skipped, entries added during it are visited.
#[derive(Clone, Default)]
pub struct Coll {
    /// A `Set` (else a `Map`).
    pub set: bool,
    /// (key, value); a Set's value is its key.
    pub entries: Vec<Option<(Value, Value)>>,
    index: std::collections::HashMap<Key, usize, ahash::RandomState>,
    /// Iterations in progress (no compaction while one runs).
    pub iterating: u32,
}

/// SameValueZero, as a hash key: numbers by value (one key for `0` and `-0`, one for NaN, the same
/// for an integer and its double), strings by content, everything else by identity (the entry
/// keeps the key alive, so the address stays its own).
#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) enum Key {
    Undef,
    Null,
    Bool(bool),
    Num(u64),
    Str(Rc<str>),
    Token(u32),
    Method(Box<Key>, ClassBrand),
    Native(Native),
    Setter(Rc<str>),
    Ref(usize),
}

impl Key {
    /// The key `v` is filed under.
    pub fn of(v: &Value) -> Key {
        match v {
            Value::Undef => Key::Undef,
            Value::None => Key::Null,
            Value::Bool(b) => Key::Bool(*b),
            Value::Int(_) | Value::Float(_) | Value::BigInt(_) => {
                let f = crate::jsvalue::to_number(v);
                Key::Num(if f.is_nan() {
                    f64::NAN.to_bits()
                } else if f == 0.0 {
                    0f64.to_bits()
                } else {
                    f.to_bits()
                })
            }
            Value::Str(s) => Key::Str(s.to_rc()),
            Value::Token(id, _) => Key::Token(*id),
            Value::List(l) => Key::Ref(Rc::as_ptr(l) as usize),
            Value::Map(m) => Key::Ref(Rc::as_ptr(m) as usize),
            Value::Coll(c) => Key::Ref(Rc::as_ptr(c) as usize),
            Value::Frag(f) => Key::Ref(Rc::as_ptr(f) as usize),
            Value::Lambda(_, e) => Key::Ref(Rc::as_ptr(e) as usize),
            Value::Func(f) => Key::Ref(Rc::as_ptr(&f.definition) as usize),
            Value::Regex(r) => Key::Ref(Rc::as_ptr(r) as usize),
            Value::Promise(p) => Key::Ref(Rc::as_ptr(p) as usize),
            Value::Native(s) => Key::Native(s.clone()),
            Value::Setter(s) => Key::Setter(s.clone()),
            Value::Method(m) => Key::Method(Box::new(Key::of(&m.function)), m.home.brand()),
            Value::Bound(b) => Key::Ref(Rc::as_ptr(b) as usize),
            Value::Class(c) => Key::Ref(Rc::as_ptr(c) as usize),
        }
    }
}

impl Coll {
    /// An empty `Set` (`set`) or `Map`.
    pub fn new(set: bool) -> Coll {
        Coll {
            set,
            ..Default::default()
        }
    }
    /// How many entries it holds.
    pub fn len(&self) -> usize {
        self.index.len()
    }
    /// Whether it holds none.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// The value under `k`.
    pub fn get(&self, k: &Value) -> Option<&Value> {
        let i = *self.index.get(&Key::of(k))?;
        self.entries[i].as_ref().map(|(_, v)| v)
    }
    /// Whether `k` is a key.
    pub fn has(&self, k: &Value) -> bool {
        self.get(k).is_some()
    }
    /// Set `k` to `v`; a key already present keeps its place.
    pub fn insert(&mut self, k: Value, v: Value) {
        let k = match k {
            Value::Float(0.0) => Value::Int(0),
            k => k,
        };
        match self.index.get(&Key::of(&k)).copied() {
            Some(i) => self.entries[i].as_mut().unwrap().1 = v,
            None => {
                self.compact();
                let i = self.entries.len();
                self.index.insert(Key::of(&k), i);
                self.entries.push(Some((k, v)));
            }
        }
    }
    /// Delete `k`; whether it was a key.
    pub fn remove(&mut self, k: &Value) -> bool {
        match self.index.remove(&Key::of(k)) {
            Some(i) => {
                self.entries[i] = None;
                true
            }
            None => false,
        }
    }
    /// Delete every entry.
    pub fn clear(&mut self) {
        for e in &mut self.entries {
            *e = None
        }
        self.index.clear();
    }
    /// Live entries in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (&Value, &Value)> {
        self.entries.iter().enumerate().filter_map(|(i, _)| self.entry_at(i))
    }
    pub(crate) fn entry_at(&self, i: usize) -> Option<(&Value, &Value)> {
        self.entries[i].as_ref().map(|(k, v)| (k, v))
    }
    fn compact(&mut self) {
        let n = self.index.len();
        let holes = self.entries.len() - n;
        if self.iterating > 0 || holes < 32 || holes < n {
            return;
        }
        self.entries.retain(|e| e.is_some());
        self.index.clear();
        for (i, e) in self.entries.iter().enumerate() {
            if let Some((k, _)) = e {
                self.index.insert(Key::of(k), i);
            }
        }
    }
}

impl std::fmt::Debug for Coll {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}({})", if self.set { "Set" } else { "Map" }, self.len())
    }
}

fn graph_id(v: &Value) -> Option<usize> {
    match v {
        Value::List(l) => Some(Rc::as_ptr(l) as usize),
        Value::Map(m) => Some(Rc::as_ptr(m) as usize),
        Value::Coll(c) => Some(Rc::as_ptr(c) as usize),
        _ => None,
    }
}

/// Serializable children, including both keys and values of Map/Set collections.
fn graph_children(v: &Value) -> Vec<Value> {
    match v {
        Value::List(l) => l.read().clone(),
        Value::Map(m) => m.read().values().cloned().collect(),
        Value::Coll(c) => c.read().iter().flat_map(|(k, v)| [k.clone(), v.clone()]).collect(),
        _ => vec![],
    }
}

/// Whether the serializable graph contains a cycle. The iterative postorder traversal visits
/// each container once, including shared subtrees; heights also bound state nesting.
pub fn cyclic(v: &Value) -> bool {
    check_graph(v, usize::MAX).is_err()
}

/// What keeps a serializable graph from crossing a boundary as JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphFault {
    /// The value contains itself.
    Cycle,
    /// It nests deeper than the limit.
    Deep,
}

/// How deeply state and other data crossing a boundary as JSON (`<Mount>` props) nest: the
/// depth the host's JSON reader (serde_json) reads back.
pub const MAX_DATA_NESTING: usize = 128;

/// Whether `v` can cross as JSON: it contains no cycle and nests at most `max_depth` levels.
pub fn check_graph(v: &Value, max_depth: usize) -> Result<(), GraphFault> {
    check_graph_marking(v, max_depth, None)
}

/// The checked height of a deeply frozen container, if a state check has passed it.
fn checked_height(v: &Value) -> Option<usize> {
    match v {
        Value::List(l) => l.checked_height(),
        Value::Map(m) => m.checked_height(),
        Value::Coll(c) => c.checked_height(),
        _ => None,
    }
}

/// `check_graph`, collecting the containers it visited with their heights (`visited`), so the
/// caller can mark them once they are frozen. A checked subtree is not entered: it is frozen, so
/// it cannot reach the new containers on the path, and its height is known.
fn check_graph_marking(
    v: &Value,
    max_depth: usize,
    mut visited: Option<&mut Vec<(Value, usize)>>,
) -> Result<(), GraphFault> {
    let mut path = std::collections::HashSet::new();
    let mut heights = std::collections::HashMap::new();
    // (value or exit marker, id, unchecked child ids, own purity, height of checked children)
    let mut work = vec![(Some(v.clone()), 0, Vec::new(), true, 0usize)];
    let mut values = std::collections::HashMap::new();
    // Plain data (no functions anywhere below): only those subtrees are marked checked.
    let mut pure = std::collections::HashMap::new();
    while let Some((v, exit, children, own_pure, checked_max)) = work.pop() {
        let Some(v) = v else {
            let height = 1 + children
                .iter()
                .filter_map(|id| heights.get(id))
                .max()
                .copied()
                .unwrap_or(0)
                .max(checked_max);
            heights.insert(exit, height);
            path.remove(&exit);
            let is_pure = own_pure && children.iter().all(|id| pure.get(id).copied().unwrap_or(false));
            pure.insert(exit, is_pure);
            if let (Some(out), Some(value)) = (visited.as_deref_mut(), values.remove(&exit))
                && is_pure
            {
                out.push((value, height));
            }
            continue;
        };
        let Some(id) = graph_id(&v) else { continue };
        if path.contains(&id) {
            return Err(GraphFault::Cycle);
        }
        if let Some(h) = checked_height(&v) {
            if path.len() + h > max_depth {
                return Err(GraphFault::Deep);
            }
            heights.insert(id, h);
            pure.insert(id, true);
            continue;
        }
        if path.len() + heights.get(&id).copied().unwrap_or(1) > max_depth {
            return Err(GraphFault::Deep);
        }
        if heights.contains_key(&id) {
            continue;
        }
        path.insert(id);
        let children = graph_children(&v);
        if visited.is_some() {
            values.insert(id, v.clone());
        }
        let own_pure = matches!(&v, Value::List(_) | Value::Map(_))
            && !matches!(&v, Value::Map(m) if m.ext().is_some())
            && !matches!(&v, Value::List(l) if l.ext().is_some())
            && children.iter().all(|c| {
                graph_id(c).is_some()
                    || matches!(
                        c,
                        Value::Undef
                            | Value::None
                            | Value::Bool(_)
                            | Value::Int(_)
                            | Value::BigInt(_)
                            | Value::Float(_)
                            | Value::Str(_)
                            | Value::Token(..)
                            | Value::Frag(_)
                    )
            });
        // Checked children are already proven: their heights count, nothing is walked or hashed.
        let checked_max = children.iter().filter_map(checked_height).max().unwrap_or(0);
        if path.len() + checked_max > max_depth {
            return Err(GraphFault::Deep);
        }
        let unchecked: Vec<Value> = children
            .into_iter()
            .filter(|c| graph_id(c).is_some() && checked_height(c).is_none())
            .collect();
        work.push((
            None,
            id,
            unchecked.iter().filter_map(graph_id).collect(),
            own_pure,
            checked_max,
        ));
        work.extend(unchecked.into_iter().map(|v| (Some(v), 0, Vec::new(), true, 0)));
    }
    Ok(())
}

/// Values in state must have an acyclic serializable graph, even if a closure already froze
/// them. Freezing and cycle validation are separate guarantees.
pub fn freeze_state(v: &Value) -> Result<(), String> {
    let mut visited = vec![];
    check_graph_marking(v, MAX_DATA_NESTING, Some(&mut visited)).map_err(|f| match f {
        GraphFault::Cycle => "TypeError: Converting circular structure to state: a value that contains itself cannot be kept in the app's state".to_string(),
        GraphFault::Deep => format!("RangeError: state nesting exceeds {MAX_DATA_NESTING} levels"),
    })?;
    freeze(v);
    for (value, height) in visited {
        match &value {
            Value::List(l) => l.mark_checked(height),
            Value::Map(m) => m.mark_checked(height),
            Value::Coll(c) => c.mark_checked(height),
            _ => {}
        }
    }
    Ok(())
}

/// Deeply freeze the reachable graph without native recursion. Mark before visiting children
/// so cycles through closures, collections and private fields also terminate.
pub fn freeze(v: &Value) {
    // Frozen subtrees are skipped before any bookkeeping: freezing a new list of kept records
    // costs the new containers, not a hash entry per record.
    let mut work = crate::heap::Node::of(v)
        .into_iter()
        .filter(|n| !n.already_frozen())
        .collect::<Vec<_>>();
    if work.is_empty() {
        return;
    }
    let mut seen = std::collections::HashSet::new();
    while let Some(node) = work.pop() {
        if !seen.insert(node.id()) || node.already_frozen() {
            continue;
        }
        let children = node.children().expect("freezing a value while it is mutably borrowed");
        node.freeze();
        work.extend(children.into_iter().filter(|n| !n.already_frozen()));
    }
}

/// `freeze`, marking every object it freezes as derived: what a cache step made. Objects already
/// frozen (state the step only passed through) are not marked.
pub fn freeze_derived(v: &Value) {
    let mut work = crate::heap::Node::of(v).into_iter().collect::<Vec<_>>();
    let mut seen = std::collections::HashSet::new();
    while let Some(node) = work.pop() {
        if !seen.insert(node.id()) || node.already_frozen() {
            continue;
        }
        let children = node.children().expect("freezing a value while it is mutably borrowed");
        node.freeze();
        node.mark_derived();
        work.extend(children);
    }
}

/// A variable captured by a closure: shared by the closure and the scope that declared it.
/// `run` is the interpreter run that created it: a closure kept from an earlier run (a render's
/// handler, called in a later action or replay) gets a private copy, so a run never sees another
/// run's assignments and replays stay deterministic.
#[derive(Debug)]
pub struct VarCell {
    /// The interpreter run that created it.
    pub run: u64,
    /// Its current value.
    pub value: std::cell::RefCell<Value>,
    /// Its value is frozen (a closure that holds it was kept past its run).
    pub frozen: std::cell::Cell<bool>,
}

impl VarCell {
    /// A variable of run `run` holding `v`.
    pub fn new(run: u64, v: Value) -> VarCell {
        VarCell {
            run,
            value: std::cell::RefCell::new(v),
            frozen: std::cell::Cell::new(false),
        }
    }
}

/// A closure's captured variables.
#[derive(Debug)]
pub struct Env {
    pub(crate) locals: Vec<(Rc<str>, Rc<VarCell>)>,
    pub(crate) module: Rc<crate::modules::Instance>,
}
impl std::ops::Deref for Env {
    type Target = Vec<(Rc<str>, Rc<VarCell>)>;
    fn deref(&self) -> &Self::Target {
        &self.locals
    }
}

#[derive(Debug)]
pub struct Function {
    pub(crate) definition: Rc<crate::ast::Func>,
    pub(crate) module: Rc<crate::modules::Instance>,
}
impl std::ops::Deref for Function {
    type Target = crate::ast::Func;
    fn deref(&self) -> &Self::Target {
        &self.definition
    }
}

/// A string's text: on the heap, or held by the host (the harness maps large strings from disk).
/// Both read as `&str` and behave identically in the language; only where the bytes live differs.
#[derive(Clone)]
pub enum Text {
    /// Bytes on the heap.
    Heap(Rc<str>),
    /// Bytes the host holds (a large string mapped from disk).
    Host(Rc<dyn HostText>),
}

/// Text the host owns. The language reads it, and writes `json()` when the value crosses back to
/// the host (`to_wire`), so the host gets its own reference instead of a copy of the bytes.
pub trait HostText {
    /// The text.
    fn as_str(&self) -> &str;
    /// The JSON the host reads back for this text: its own reference to it.
    fn json(&self) -> &str;
    /// Its length in code points.
    fn char_count(&self) -> usize;
}

/// Turns a JSON string into host text, or `None` for an ordinary string.
pub type HostResolver = Rc<dyn Fn(&str) -> Option<Rc<dyn HostText>>>;

thread_local! {
    static RESOLVER: std::cell::RefCell<Option<HostResolver>> = const { std::cell::RefCell::new(None) };
}

/// Run `f` with `resolve` deciding which JSON strings `Value::from_json` reads as host text.
pub fn with_host_text<R>(resolve: Option<HostResolver>, f: impl FnOnce() -> R) -> R {
    let prev = RESOLVER.with(|r| r.replace(resolve));
    struct Restore(Option<HostResolver>);
    impl Drop for Restore {
        fn drop(&mut self) {
            let prev = self.0.take();
            RESOLVER.with(|r| *r.borrow_mut() = prev);
        }
    }
    let _restore = Restore(prev);
    f()
}

impl Text {
    /// Its length in code points.
    pub fn char_count(&self) -> usize {
        match self {
            Text::Heap(s) => s.chars().count(),
            Text::Host(h) => h.char_count(),
        }
    }
    /// The text as a shared heap string (copies host text).
    pub fn to_rc(&self) -> Rc<str> {
        match self {
            Text::Heap(s) => s.clone(),
            Text::Host(h) => Rc::from(h.as_str()),
        }
    }
    /// Characters `a..b` (char positions, clamped); no intermediate copy of the whole text.
    pub fn char_slice(&self, a: usize, b: usize) -> &str {
        let s: &str = self;
        if b <= a {
            return "";
        }
        if self.is_ascii_text() {
            return &s[a.min(s.len())..b.min(s.len())];
        }
        let mut bounds = s.char_indices().map(|(i, _)| i).chain(std::iter::once(s.len()));
        let start = bounds.nth(a).unwrap_or(s.len());
        let end = bounds.nth(b - a - 1).unwrap_or(s.len());
        &s[start..end]
    }
    /// The byte offset of char position `n` (clamped to the end).
    pub fn char_to_byte(&self, n: usize) -> usize {
        let s: &str = self;
        if self.is_ascii_text() {
            return n.min(s.len());
        }
        s.char_indices().nth(n).map_or(s.len(), |(i, _)| i)
    }
    /// The char position of byte offset `b` (a char boundary).
    pub fn byte_to_char(&self, b: usize) -> usize {
        if self.is_ascii_text() {
            b
        } else {
            self[..b].chars().count()
        }
    }
    fn is_ascii_text(&self) -> bool {
        match self {
            Text::Heap(s) => s.is_ascii(),
            Text::Host(h) => h.char_count() == h.as_str().len(),
        }
    }
}

impl std::ops::Deref for Text {
    type Target = str;
    fn deref(&self) -> &str {
        match self {
            Text::Heap(s) => s,
            Text::Host(h) => h.as_str(),
        }
    }
}

impl AsRef<str> for Text {
    fn as_ref(&self) -> &str {
        self
    }
}

impl std::fmt::Debug for Text {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Text::Heap(s) => s.fmt(f),
            Text::Host(h) => write!(f, "<host text, {} bytes>", h.as_str().len()),
        }
    }
}

impl From<Rc<str>> for Text {
    fn from(s: Rc<str>) -> Text {
        Text::Heap(s)
    }
}
impl From<&Rc<str>> for Text {
    fn from(s: &Rc<str>) -> Text {
        Text::Heap(s.clone())
    }
}
impl From<&str> for Text {
    fn from(s: &str) -> Text {
        Text::Heap(Rc::from(s))
    }
}
impl From<String> for Text {
    fn from(s: String) -> Text {
        Text::Heap(Rc::from(s))
    }
}

#[derive(Clone, Debug)]
/// A value of the language.
pub enum Value {
    /// `undefined`: a missing key or unset variable.
    Undef,
    /// `null`.
    None,
    /// A boolean.
    Bool(bool),
    /// An integer.
    Int(i64),
    /// A JSON integer that does not fit in `i64`, kept as its exact decimal string.
    BigInt(Rc<str>),
    /// A number with a fraction, or one beyond `i64`.
    Float(f64),
    /// A string.
    Str(Text),
    /// An array.
    List(Rc<Arr>),
    /// A plain object or a class instance.
    Map(Rc<Object>),
    /// A `Map` or a `Set`.
    Coll(Rc<Obj<Coll>>),
    /// `f.bind(this, ...args)`: the function, its `this`, the arguments it was bound with.
    Bound(Rc<(Value, Value, Vec<Value>)>),
    /// A method read off an object, with the class that defines it.
    Method(Rc<Method>),
    /// A class (`typeof` is "function"; `new` makes an instance).
    Class(Rc<Class>),
    /// A rendered fragment (e.g. a component's `children`).
    Frag(Rc<Vec<crate::emit::Item>>),
    /// A closure: the lambda and the enclosing variables it uses, shared with the scope that
    /// created it (JavaScript closures see and make later assignments).
    Lambda(Rc<Lambda>, Rc<Env>),
    /// A named top-level function used as a value (`xs.map(render)`).
    Func(Rc<Function>),
    /// The setter returned by `useState` (key into the `__ui` state map).
    Setter(Rc<str>),
    /// A native builtin or method, retaining its defining prototype when borrowed.
    Native(Native),
    /// A special token as a first-class value (`tok.im_start`): never text.
    Token(u32, Rc<str>),
    /// Host effects are interpreter values, never forgeable JSON request objects.
    Promise(Rc<crate::effects::Promise>),
    /// A compiled JavaScript regular expression (`/re/g`, `new RegExp(..)`).
    Regex(Rc<crate::jsregex::JsRegex>),
}

impl Value {
    pub(crate) fn is_object(&self) -> bool {
        matches!(
            self,
            Value::Map(_)
                | Value::List(_)
                | Value::Coll(_)
                | Value::Regex(_)
                | Value::Promise(_)
                | Value::Native(_)
                | Value::Class(_)
                | Value::Method(_)
                | Value::Lambda(..)
                | Value::Func(_)
                | Value::Bound(_)
                | Value::Setter(_)
        )
    }
    /// The source declaration behind a method or bound function, for tool metadata.
    pub(crate) fn declaration(&self) -> &Value {
        let mut value = self;
        loop {
            value = match value {
                Value::Method(m) => &m.function,
                Value::Bound(b) => &b.0,
                _ => return value,
            };
        }
    }
    /// JavaScript callable values, plus the runtime's parent callback proxy. Keep one
    /// classification for callbacks, hooks, invocation and cloning.
    pub fn is_callable(&self) -> bool {
        match self {
            Value::Native(n) => !n.is_namespace(),
            Value::Method(_)
            | Value::Lambda(..)
            | Value::Func(_)
            | Value::Bound(_)
            | Value::Setter(_)
            | Value::Class(_) => true,
            Value::Map(m) => m.read().contains_key("__parent"),
            _ => false,
        }
    }
    /// A string value.
    pub fn str(s: &str) -> Value {
        Value::Str(Text::from(s))
    }
    /// A string value, taking `s`.
    pub fn string(s: String) -> Value {
        Value::Str(Text::from(s))
    }
    /// An array of `v`.
    pub fn list(v: Vec<Value>) -> Value {
        Value::List(Rc::new(Obj::new(v)))
    }
    /// A plain object with the properties `m`.
    pub fn map(m: Map) -> Value {
        Value::Map(Rc::new(Obj::new(m)))
    }
    /// Content that is already token ids, as a host passes a render it made itself: `ids`, with
    /// `spans` over them (ranges relative to `ids`). A view places it in its prompt as it is,
    /// without decoding or tokenizing it again.
    pub fn tokens(ids: Rc<[u32]>, spans: Rc<[crate::emit::Span]>) -> Value {
        Value::Frag(Rc::new(vec![crate::emit::Item::Tokens(ids, spans)]))
    }

    /// Its type as the Python-dialect templates name it (`NoneType`, `bool`, `int`, ...).
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Undef => "undefined",
            Value::None => "NoneType",
            Value::Bool(_) => "bool",
            Value::Int(_) | Value::BigInt(_) => "int",
            Value::Float(_) => "float",
            Value::Str(_) => "str",
            Value::List(_) => "list",
            Value::Map(_) => "dict",
            Value::Frag(_) => "fragment",
            Value::Native(n) if n.is_namespace() => "object",
            Value::Method(_)
            | Value::Native(_)
            | Value::Lambda(..)
            | Value::Func(_)
            | Value::Bound(_)
            | Value::Setter(_) => "function",
            Value::Token(..) => "token",
            Value::Promise(_) => "promise",
            Value::Regex(_) => "RegExp",
            Value::Coll(c) => {
                if c.read().set {
                    "Set"
                } else {
                    "Map"
                }
            }
            Value::Class(_) => "function",
        }
    }

    /// The text, if it is a string.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    /// The value of JSON read with [`parse_json_exact`]: as [`Value::from_json`], and an integer
    /// past 64 bits is the number with all its digits.
    pub fn from_json_exact(j: &serde_json::Value) -> Value {
        match j {
            serde_json::Value::String(s) => match s.strip_prefix(EXACT_INTEGER) {
                Some(digits) => Value::BigInt(Rc::from(digits)),
                None => Value::from_json(j),
            },
            serde_json::Value::Array(items) => Value::list(items.iter().map(Value::from_json_exact).collect()),
            serde_json::Value::Object(fields) => {
                let mut m = Map::default();
                for (k, v) in fields {
                    m.insert(Rc::from(k.as_str()), Value::from_json_exact(v));
                }
                Value::map(m)
            }
            _ => Value::from_json(j),
        }
    }

    /// The value of a JSON value.
    pub fn from_json(j: &serde_json::Value) -> Value {
        match j {
            serde_json::Value::Null => Value::None,
            serde_json::Value::Bool(b) => Value::Bool(*b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    return Value::Int(i);
                }
                // A serde_json number holds 64 bits; an integer past that was already read as a
                // float. JSON text read here keeps every digit: `from_json_exact`.
                if let Some(u) = n.as_u64() {
                    return Value::BigInt(Rc::from(u.to_string().as_str()));
                }
                Value::Float(n.as_f64().unwrap_or(f64::NAN))
            }
            serde_json::Value::String(s) => match RESOLVER.with(|r| r.borrow().as_ref().and_then(|f| f(s))) {
                Some(h) => Value::Str(Text::Host(h)),
                None => Value::str(s),
            },
            serde_json::Value::Array(a) => Value::list(a.iter().map(Value::from_json).collect()),
            serde_json::Value::Object(o) => {
                let mut m = Map::default();
                for (k, v) in o {
                    m.insert(Rc::from(k.as_str()), Value::from_json(v));
                }
                Value::map(m)
            }
        }
    }

    /// The value as JSON text, as the language sees it (`JSON.stringify`, `messagesJson`): host
    /// text is written out in full.
    pub fn to_json(&self) -> serde_json::Value {
        self.json(false)
    }

    /// The value as it crosses to the host (handler writes and results, effect arguments): host
    /// text stays the host's own reference instead of a copy of its bytes.
    pub fn to_wire(&self) -> serde_json::Value {
        self.json(true)
    }

    fn json(&self, wire: bool) -> serde_json::Value {
        self.json_in(wire, &mut vec![])
    }

    /// `json`, with the objects on the current path: a value that contains itself writes `null`
    /// where it repeats (callers that send values to the host check `cyclic` first, and refuse).
    fn json_in(&self, wire: bool, path: &mut Vec<*const ()>) -> serde_json::Value {
        let p = match self {
            Value::List(l) => Rc::as_ptr(l) as *const (),
            Value::Map(m) => Rc::as_ptr(m) as *const (),
            _ => std::ptr::null(),
        };
        if !p.is_null() {
            if path.contains(&p) {
                return serde_json::Value::Null;
            }
            path.push(p);
        }
        let r = stacker::maybe_grow(128 * 1024, 2 * 1024 * 1024, || self.json_one(wire, path));
        if !p.is_null() {
            path.pop();
        }
        r
    }

    fn json_one(&self, wire: bool, path: &mut Vec<*const ()>) -> serde_json::Value {
        match self {
            Value::Undef | Value::None => serde_json::Value::Null,
            Value::Bool(b) => (*b).into(),
            Value::Int(i) => (*i).into(),
            // A serde_json number holds 64 bits: the integer if it fits, else the nearest float.
            // (Text the engine writes itself, a prompt's `formatJSON`, keeps every digit.)
            Value::BigInt(s) => s
                .parse::<u64>()
                .map(serde_json::Value::from)
                .or_else(|_| s.parse::<i64>().map(serde_json::Value::from))
                .unwrap_or_else(|_| {
                    serde_json::Number::from_f64(s.parse::<f64>().unwrap_or(f64::NAN))
                        .map(serde_json::Value::Number)
                        .unwrap_or(serde_json::Value::Null)
                }),
            Value::Float(f) => serde_json::Number::from_f64(*f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            Value::Str(Text::Host(h)) if wire => h.json().into(),
            Value::Str(s) => s.to_string().into(),
            Value::List(l) => serde_json::Value::Array(l.read().iter().map(|v| v.json_in(wire, path)).collect()),
            Value::Map(m) => serde_json::Value::Object(
                m.read()
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.json_in(wire, path)))
                    .collect(),
            ),
            Value::Regex(_) | Value::Coll(_) => serde_json::Value::Object(Default::default()),
            // Exact token ids (a transcript record's `ids`) write out as the id list.
            Value::Frag(f) if f.iter().all(|i| matches!(i, crate::emit::Item::Tokens(..))) && !f.is_empty() => {
                serde_json::Value::Array(
                    f.iter()
                        .flat_map(|i| match i {
                            crate::emit::Item::Tokens(ids, _) => ids.iter().map(|&t| t.into()).collect::<Vec<_>>(),
                            _ => vec![],
                        })
                        .collect(),
                )
            }
            _ => serde_json::Value::Null,
        }
    }

    /// Python `str(x)` text (`formatPythonValue`). Undefined prints as "".
    pub fn py_str(&self, out: &mut String) {
        match self {
            Value::Undef => {}
            Value::Str(s) => out.push_str(s),
            _ => self.py_repr(out),
        }
    }

    /// As Python's `str()` prints it (official chat templates are Jinja, which prints values so).
    pub fn to_py_str(&self) -> String {
        let mut s = String::new();
        self.py_str(&mut s);
        s
    }

    /// Python `repr(x)`.
    pub fn py_repr(&self, out: &mut String) {
        enum Work {
            Value(Value),
            Text(String),
            Leave(usize),
        }
        let mut work = vec![Work::Value(self.clone())];
        let mut path = std::collections::HashSet::new();
        while let Some(task) = work.pop() {
            let value = match task {
                Work::Text(text) => {
                    out.push_str(&text);
                    continue;
                }
                Work::Leave(id) => {
                    path.remove(&id);
                    continue;
                }
                Work::Value(value) => value,
            };
            match &value {
                Value::List(l) => {
                    let id = Rc::as_ptr(l) as usize;
                    if !path.insert(id) {
                        out.push_str("[...]");
                        continue;
                    }
                    out.push('[');
                    work.push(Work::Leave(id));
                    work.push(Work::Text("]".into()));
                    for (i, v) in l.read().iter().enumerate().rev() {
                        work.push(Work::Value(v.clone()));
                        if i > 0 {
                            work.push(Work::Text(", ".into()));
                        }
                    }
                }
                Value::Map(m) => {
                    let id = Rc::as_ptr(m) as usize;
                    if !path.insert(id) {
                        out.push_str("{...}");
                        continue;
                    }
                    out.push('{');
                    work.push(Work::Leave(id));
                    work.push(Work::Text("}".into()));
                    for (i, (k, v)) in m.read().iter().enumerate().rev() {
                        work.push(Work::Value(v.clone()));
                        let mut key = String::new();
                        py_str_repr(k, &mut key);
                        key.push_str(": ");
                        work.push(Work::Text(key));
                        if i > 0 {
                            work.push(Work::Text(", ".into()));
                        }
                    }
                }

                Value::Undef => {}
                Value::None => out.push_str("None"),
                Value::Bool(true) => out.push_str("True"),
                Value::Bool(false) => out.push_str("False"),
                Value::Int(i) => {
                    let _ = write!(out, "{}", i);
                }
                Value::BigInt(s) => out.push_str(s),
                Value::Float(f) => py_float_repr(*f, out),
                Value::Str(s) => py_str_repr(s, out),
                Value::Frag(_) => out.push_str("<fragment>"),
                Value::Method(_)
                | Value::Native(_)
                | Value::Lambda(..)
                | Value::Func(_)
                | Value::Bound(_)
                | Value::Setter(_) => out.push_str("<function>"),
                Value::Promise(_) => out.push_str("<promise>"),
                Value::Regex(r) => {
                    let _ = write!(out, "/{}/{}", r.source, r.flags);
                }
                Value::Token(id, _) => {
                    let _ = write!(out, "<token {}>", id);
                }
                Value::Coll(c) => out.push_str(if c.read().set { "<Set>" } else { "<Map>" }),
                Value::Class(c) => {
                    let _ = write!(out, "<class {}>", c.name);
                }
            }
        }
    }
}

/// True if a JSON number literal is an integer (no fraction/exponent), so Python's `json`
/// would parse it as `int` rather than `float`.
pub(crate) fn is_int_lexeme(s: &str) -> bool {
    let d = s.strip_prefix('-').unwrap_or(s);
    !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit())
}

/// Python float repr: shortest round-trip digits, scientific when exp < -4 or >= 16.
pub fn py_float_repr(f: f64, out: &mut String) {
    if f.is_nan() {
        out.push_str("nan");
        return;
    }
    if f.is_infinite() {
        out.push_str(if f > 0.0 { "inf" } else { "-inf" });
        return;
    }
    if f == 0.0 {
        out.push_str(if f.is_sign_negative() { "-0.0" } else { "0.0" });
        return;
    }
    // Rust's `{:e}` gives the shortest round-trip digits, e.g. "1.2345e-5".
    let e = format!("{:e}", f);
    let (mant, exp) = e.split_once('e').unwrap();
    let exp: i32 = exp.parse().unwrap();
    let neg = mant.starts_with('-');
    let digits: String = mant.chars().filter(|c| c.is_ascii_digit()).collect();
    if neg {
        out.push('-');
    }
    if (-4..16).contains(&exp) {
        if exp >= 0 {
            let ip = (exp + 1) as usize;
            if digits.len() <= ip {
                out.push_str(&digits);
                for _ in digits.len()..ip {
                    out.push('0');
                }
                out.push_str(".0");
            } else {
                out.push_str(&digits[..ip]);
                out.push('.');
                out.push_str(&digits[ip..]);
            }
        } else {
            out.push_str("0.");
            for _ in 0..(-exp - 1) {
                out.push('0');
            }
            out.push_str(&digits);
        }
    } else {
        out.push_str(&digits[..1]);
        if digits.len() > 1 {
            out.push('.');
            out.push_str(&digits[1..]);
        }
        let _ = write!(out, "e{}{:02}", if exp < 0 { '-' } else { '+' }, exp.abs());
    }
}

/// Python's str.isprintable(), approximated: control, separators (except ASCII space),
/// unassigned/private-use and format characters are non-printable.
fn py_printable(c: char) -> bool {
    let u = c as u32;
    if c == ' ' {
        return true;
    }
    if u < 0x20 || (0x7f..0xa0).contains(&u) {
        return false;
    }
    // Zs/Zl/Zp other than ASCII space, and common Cf (format) characters.
    !matches!(u,
        0xa0 | 0xad | 0x1680 | 0x2000..=0x200f | 0x2028..=0x202f | 0x205f..=0x206f | 0x3000 | 0xfeff
        | 0xfff9..=0xfffb | 0xe000..=0xf8ff | 0xd800..=0xdfff | 0x600..=0x605 | 0x61c | 0x6dd | 0x70f
        | 0x180e | 0x110bd | 0x1d173..=0x1d17a | 0xe0001 | 0xe0020..=0xe007f
    )
}

/// Python `repr(str)`.
pub fn py_str_repr(s: &str, out: &mut String) {
    let q = if s.contains('\'') && !s.contains('"') {
        '"'
    } else {
        '\''
    };
    out.push(q);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == q => {
                out.push('\\');
                out.push(c);
            }
            c if !py_printable(c) => {
                let u = c as u32;
                if u < 0x100 {
                    let _ = write!(out, "\\x{:02x}", u);
                } else if u < 0x10000 {
                    let _ = write!(out, "\\u{:04x}", u);
                } else {
                    let _ = write!(out, "\\U{:08x}", u);
                }
            }
            c => out.push(c),
        }
    }
    out.push(q);
}

/// How `json_dumps` writes: the options of Python's `json.dumps`.
pub struct JsonOpts<'a> {
    /// Indentation per level; none writes one line.
    pub indent: Option<String>,
    /// What separates items.
    pub item_sep: &'a str,
    /// What separates a key from its value.
    pub key_sep: &'a str,
    /// Escape every non-ASCII character.
    pub ensure_ascii: bool,
    /// Write an object's keys in sorted order.
    pub sort_keys: bool,
}

impl<'a> JsonOpts<'a> {
    /// HF `tojson` defaults: json.dumps(x, ensure_ascii=False).
    pub fn hf_default() -> JsonOpts<'static> {
        JsonOpts {
            indent: None,
            item_sep: ", ",
            key_sep: ": ",
            ensure_ascii: false,
            sort_keys: false,
        }
    }
}

fn json_str(s: &str, ascii: bool, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c if ascii && (c as u32) > 0x7e => {
                let mut buf = [0u16; 2];
                for u in c.encode_utf16(&mut buf) {
                    let _ = write!(out, "\\u{:04x}", u);
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Python `json.dumps` with the given options.
pub fn json_dumps(v: &Value, o: &JsonOpts, depth: usize, out: &mut String) -> Result<(), String> {
    let nl = |out: &mut String, d: usize| {
        if let Some(ind) = &o.indent {
            out.push('\n');
            for _ in 0..d {
                out.push_str(ind);
            }
        }
    };
    match v {
        Value::None => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Int(i) => {
            let _ = write!(out, "{}", i);
        }
        Value::BigInt(s) => out.push_str(s),
        Value::Float(f) => {
            if f.is_nan() {
                out.push_str("NaN")
            } else if f.is_infinite() {
                out.push_str(if *f > 0.0 { "Infinity" } else { "-Infinity" })
            } else {
                py_float_repr(*f, out)
            }
        }
        Value::Str(s) => json_str(s, o.ensure_ascii, out),
        Value::List(l) => {
            if l.read().is_empty() {
                out.push_str("[]");
                return Ok(());
            }
            out.push('[');
            for (i, x) in l.read().iter().enumerate() {
                if i > 0 {
                    out.push_str(o.item_sep);
                }
                nl(out, depth + 1);
                json_dumps(x, o, depth + 1, out)?;
            }
            nl(out, depth);
            out.push(']');
        }
        Value::Map(m) => {
            if m.read().is_empty() {
                out.push_str("{}");
                return Ok(());
            }
            out.push('{');
            let m = m.read();
            let mut keys: Vec<&Rc<str>> = m.keys().collect();
            if o.sort_keys {
                keys.sort();
            }
            for (i, k) in keys.into_iter().enumerate() {
                if i > 0 {
                    out.push_str(o.item_sep);
                }
                nl(out, depth + 1);
                json_str(k, o.ensure_ascii, out);
                out.push_str(o.key_sep);
                json_dumps(&m[k], o, depth + 1, out)?;
            }
            nl(out, depth);
            out.push('}');
        }
        Value::Undef => return Err("Object of type Undefined is not JSON serializable".into()),
        other => return Err(format!("Object of type {} is not JSON serializable", other.type_name())),
    }
    Ok(())
}

impl PartialEq for Text {
    fn eq(&self, other: &Text) -> bool {
        **self == **other
    }
}

/// A JSON object as the runtime's state map (props and hooks), e.g. for a render or a test.
pub fn state(j: &serde_json::Value) -> Rc<Map> {
    match Value::from_json(j) {
        Value::Map(m) => Rc::new(m.into_data()),
        _ => Rc::new(Map::default()),
    }
}

// Mutation sites register possible cycle-closing edges with the exhaustive heap graph.
pub(crate) fn note_write(target: &Value, value: &Value) {
    crate::heap::note_write(target, value)
}
pub(crate) fn note_cell_write(cell: &Rc<VarCell>, value: &Value) {
    crate::heap::note_cell_write(cell, value)
}
pub(crate) fn sweep() {
    crate::heap::sweep()
}

/// How [`parse_json_exact`] writes an integer past 64 bits: a string that starts with this,
/// then the digits. (A private-use character: text does not start with it by accident.)
const EXACT_INTEGER: &str = "\u{E000}integer:";

/// Parse JSON text, keeping integers past 64 bits. serde_json's numbers hold 64 bits and read a
/// longer integer as a float; Python (and so an official chat template) keeps every digit. Here
/// such an integer becomes a marked string, which [`Value::from_json_exact`] reads as the number.
/// The result is ordinary JSON for every other purpose.
///
/// The engine does not ask serde_json for arbitrary precision instead: Cargo builds one
/// serde_json for a whole program, and that feature changes how every number in it serializes,
/// the host's included.
pub fn parse_json_exact(text: &str) -> Result<serde_json::Value, String> {
    let bytes = text.as_bytes();
    let mut out: Option<String> = None;
    let (mut i, mut copied) = (0, 0);
    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                i += 1;
                while i < bytes.len() && bytes[i] != b'"' {
                    i += if bytes[i] == b'\\' { 2 } else { 1 };
                }
                i += 1;
            }
            b'-' | b'0'..=b'9' => {
                let start = i;
                i += usize::from(bytes[i] == b'-');
                let digits = i;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                let integer = !matches!(bytes.get(i), Some(b'.' | b'e' | b'E'));
                let lexeme = &text[start..i];
                if integer && i - digits > 18 && lexeme.parse::<i64>().is_err() && lexeme.parse::<u64>().is_err() {
                    let out = out.get_or_insert_with(String::new);
                    out.push_str(&text[copied..start]);
                    out.push('"');
                    out.push_str(EXACT_INTEGER);
                    out.push_str(lexeme);
                    out.push('"');
                    copied = i;
                }
                // The rest of a number that is not an integer.
                while i < bytes.len() && matches!(bytes[i], b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-') {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }
    let parsed = match out {
        Some(mut out) => {
            out.push_str(&text[copied..]);
            serde_json::from_str(&out)
        }
        None => serde_json::from_str(text),
    };
    parsed.map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fr(f: f64) -> String {
        let mut s = String::new();
        py_float_repr(f, &mut s);
        s
    }
    #[test]
    fn float_repr_matches_python() {
        assert_eq!(fr(0.93), "0.93");
        assert_eq!(fr(1.0), "1.0");
        assert_eq!(fr(1e-5), "1e-05");
        assert_eq!(fr(0.0001), "0.0001");
        assert_eq!(fr(1e16), "1e+16");
        assert_eq!(fr(123456789012345.6), "123456789012345.6");
        assert_eq!(fr(-2.5e-7), "-2.5e-07");
        assert_eq!(fr(100.0), "100.0");
    }
    #[test]
    fn str_repr_matches_python() {
        let mut s = String::new();
        py_str_repr("it's", &mut s);
        assert_eq!(s, "\"it's\"");
        s.clear();
        py_str_repr("a\nb'\"", &mut s);
        assert_eq!(s, "'a\\nb\\'\"'");
    }
}

#[cfg(test)]
mod collector_tests {
    use super::{Value, freeze, note_write, sweep};

    #[test]
    fn the_collector_keeps_live_graphs_and_frees_dropped_cycles() {
        // A collector must preserve live graphs too, including duplicate edges and frozen owners.
        // Once the external owner disappears the very same cycle must become collectable.
        for frozen in [false, true] {
            let child = Value::list(vec![]);
            let Value::List(array) = &child else { unreachable!() };
            let weak = std::rc::Rc::downgrade(array);
            note_write(&child, &child);
            array.write().unwrap().push(child.clone());
            let owner = Value::list(vec![child.clone(), child.clone()]);
            if frozen {
                freeze(&owner)
            }
            drop(child);
            sweep();
            let held = weak.upgrade().expect("a live owner lost its child");
            assert_eq!(held.read().len(), 1);
            assert!(matches!(&held.read()[0],Value::List(v) if std::rc::Rc::ptr_eq(v,&held)));
            drop(held);
            drop(owner);
            sweep();
            assert!(weak.upgrade().is_none(), "dropped frozen={frozen} graph leaked");
        }
    }
}
