//! The runtime reference graph. Keep this match exhaustive when Value gains a new kind.
//! Safe owning handles protect every discovered node while unreachable mutable edges are cleared.
use crate::value::{Arr, Class, Coll, Env, Obj, Object, Value, VarCell};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::{Rc, Weak},
};

#[derive(Clone)]
pub(crate) enum Node {
    Arr(Rc<Arr>),
    Object(Rc<Object>),
    Collection(Rc<Obj<Coll>>),
    Cell(Rc<VarCell>),
    Env(Rc<Env>),
    Class(Rc<Class>),
    Method(Rc<crate::value::Method>),
    Bound(Rc<(Value, Value, Vec<Value>)>),
    Regex(Rc<crate::jsregex::JsRegex>),
    Promise(Rc<crate::effects::Promise>),
    Func(Rc<crate::value::Function>),
    Fragment(Rc<Vec<crate::emit::Item>>),
    Module(Rc<crate::modules::Instance>),
}
#[derive(Clone)]
enum Candidate {
    Arr(Weak<Arr>),
    Object(Weak<Object>),
    Collection(Weak<Obj<Coll>>),
    Cell(Weak<VarCell>),
    Module(Weak<crate::modules::Instance>),
}
type Candidates = Rc<RefCell<Vec<Candidate>>>;
thread_local! { static WRITTEN: RefCell<Candidates> = RefCell::new(Rc::default()); }
/// The candidate list in use on this thread: the running instance's, or the thread's own.
fn written<R>(f: impl FnOnce(&mut Vec<Candidate>) -> R) -> R {
    let list = WRITTEN.with(|w| w.borrow().clone());
    f(&mut list.borrow_mut())
}

/// One instance's collection candidates. Instances share no values, so a cycle lies within one
/// instance, and a sweep while it runs walks only what it wrote: its cost does not grow with the
/// other instances on the thread. Dropped after the instance's own values (keep it the last
/// field), it frees the cycles they leave.
#[derive(Default)]
pub(crate) struct Written(Candidates);
/// Restores the thread's previous candidate list.
pub(crate) struct WrittenScope(Option<Candidates>);
impl Written {
    /// Use this list on this thread until the scope is dropped.
    pub(crate) fn scope(&self) -> WrittenScope {
        WrittenScope(Some(WRITTEN.with(|w| w.replace(self.0.clone()))))
    }
}
impl Drop for WrittenScope {
    fn drop(&mut self) {
        if let Some(prev) = self.0.take() {
            WRITTEN.with(|w| *w.borrow_mut() = prev);
        }
    }
}
impl Drop for Written {
    fn drop(&mut self) {
        if !self.0.borrow().is_empty() {
            let _scope = self.scope();
            sweep();
        }
    }
}

impl Node {
    pub(crate) fn of(value: &Value) -> Option<Self> {
        match value {
            Value::List(v) => Some(Self::Arr(v.clone())),
            Value::Map(v) => Some(Self::Object(v.clone())),
            Value::Coll(v) => Some(Self::Collection(v.clone())),
            Value::Lambda(_, v) => Some(Self::Env(v.clone())),
            Value::Class(v) => Some(Self::Class(v.clone())),
            Value::Method(v) => Some(Self::Method(v.clone())),
            Value::Bound(v) => Some(Self::Bound(v.clone())),
            Value::Promise(v) => Some(Self::Promise(v.clone())),
            Value::Regex(v) => Some(Self::Regex(v.clone())),
            Value::Func(v) => Some(Self::Func(v.clone())),
            Value::Frag(v) => Some(Self::Fragment(v.clone())),
            Value::Undef
            | Value::None
            | Value::Bool(_)
            | Value::Int(_)
            | Value::Float(_)
            | Value::BigInt(_)
            | Value::Str(_)
            | Value::Native(_)
            | Value::Setter(_)
            | Value::Token(..) => None,
        }
    }
    pub(crate) fn id(&self) -> usize {
        match self {
            Self::Arr(v) => Rc::as_ptr(v) as usize,
            Self::Object(v) => Rc::as_ptr(v) as usize,
            Self::Collection(v) => Rc::as_ptr(v) as usize,
            Self::Cell(v) => Rc::as_ptr(v) as usize,
            Self::Env(v) => Rc::as_ptr(v) as usize,
            Self::Class(v) => Rc::as_ptr(v) as usize,
            Self::Method(v) => Rc::as_ptr(v) as usize,
            Self::Bound(v) => Rc::as_ptr(v) as usize,
            Self::Regex(v) => Rc::as_ptr(v) as usize,
            Self::Promise(v) => Rc::as_ptr(v) as usize,
            Self::Func(v) => Rc::as_ptr(v) as usize,
            Self::Fragment(v) => Rc::as_ptr(v) as usize,
            Self::Module(v) => Rc::as_ptr(v) as usize,
        }
    }
    fn strong(&self) -> usize {
        match self {
            Self::Arr(v) => Rc::strong_count(v),
            Self::Object(v) => Rc::strong_count(v),
            Self::Collection(v) => Rc::strong_count(v),
            Self::Cell(v) => Rc::strong_count(v),
            Self::Env(v) => Rc::strong_count(v),
            Self::Class(v) => Rc::strong_count(v),
            Self::Method(v) => Rc::strong_count(v),
            Self::Bound(v) => Rc::strong_count(v),
            Self::Regex(v) => Rc::strong_count(v),
            Self::Promise(v) => Rc::strong_count(v),
            Self::Func(v) => Rc::strong_count(v),
            Self::Fragment(v) => Rc::strong_count(v),
            Self::Module(v) => Rc::strong_count(v),
        }
    }
    /// Every strong runtime reference, with multiplicity. ASTs and token/text storage have no
    /// mutable runtime references. A failed borrow defers collection rather than guessing roots.
    pub(crate) fn children(&self) -> Result<Vec<Node>, ()> {
        let values = |values: Vec<Value>| values.iter().filter_map(Self::of).collect::<Vec<_>>();
        Ok(match self {
            Self::Arr(v) => values(v.try_read().map_err(|_| ())?.clone()),
            Self::Object(v) => {
                let mut out = values(v.try_read().map_err(|_| ())?.values().cloned().collect());
                if let Some(ext) = v.ext() {
                    out.push(Self::Class(ext.class.clone()));
                    out.extend(ext.private.try_borrow().map_err(|_| ())?.values().filter_map(Self::of));
                    out.extend(ext.hidden.try_borrow().map_err(|_| ())?.values().filter_map(Self::of));
                }
                out
            }
            Self::Collection(v) => values(
                v.try_read()
                    .map_err(|_| ())?
                    .iter()
                    .flat_map(|(k, v)| [k.clone(), v.clone()])
                    .collect(),
            ),
            Self::Cell(v) => Self::of(&*v.value.try_borrow().map_err(|_| ())?).into_iter().collect(),
            Self::Env(v) => std::iter::once(Self::Module(v.module.clone()))
                .chain(v.iter().map(|(_, c)| Self::Cell(c.clone())))
                .collect(),
            Self::Class(v) => {
                let mut out = vec![Self::Object(v.statics.clone()), Self::Object(v.private_statics.clone())];
                if let Some(p) = &v.parent {
                    out.push(Self::Class(p.clone()));
                }
                out.extend(
                    v.ctor
                        .iter()
                        .chain(v.methods.values())
                        .chain(v.getters.values())
                        .chain(v.setters.values())
                        .chain(v.static_getters.values())
                        .chain(v.static_setters.values())
                        .chain(v.static_methods.values())
                        .chain(v.fields.iter().filter_map(|field| field.init.as_ref()))
                        .filter_map(Self::of),
                );
                for member in v.private_members.values() {
                    out.extend(
                        member
                            .method
                            .iter()
                            .chain(member.get.iter())
                            .chain(member.set.iter())
                            .filter_map(Self::of),
                    );
                }
                out
            }
            Self::Method(v) => {
                let mut out = vec![Self::Class(v.home.clone())];
                out.extend(Self::of(&v.function));
                out
            }
            Self::Bound(v) => {
                v.0.iter_references()
                    .chain(v.1.iter_references())
                    .chain(v.2.iter().filter_map(Self::of))
                    .collect()
            }
            Self::Func(v) => vec![Self::Module(v.module.clone())],
            Self::Module(v) => v
                .bindings
                .try_borrow()
                .map_err(|_| ())?
                .values()
                .filter_map(|cell| match cell {
                    crate::modules::Cell::Ready(value) => Self::of(value),
                    crate::modules::Cell::Alias(module, _) => Some(Self::Module(module.clone())),
                    _ => None,
                })
                .collect(),
            Self::Regex(_) | Self::Fragment(_) => vec![],
            Self::Promise(v) => v.state.borrow().value().and_then(Self::of).into_iter().collect(),
        })
    }
    pub(crate) fn already_frozen(&self) -> bool {
        match self {
            Self::Arr(v) => v.is_deep(),
            Self::Object(v) => v.is_deep(),
            Self::Collection(v) => v.is_deep(),
            Self::Cell(v) => v.frozen.get(),
            Self::Regex(v) => v.frozen.get(),
            Self::Promise(v) => v.frozen.get(),
            Self::Module(v) => v.frozen.get(),
            _ => false,
        }
    }
    pub(crate) fn freeze(&self) {
        match self {
            Self::Arr(v) => v.mark_deep(),
            Self::Object(v) => v.mark_deep(),
            Self::Collection(v) => v.mark_deep(),
            Self::Cell(v) => v.frozen.set(true),
            Self::Regex(v) => v.frozen.set(true),
            Self::Promise(v) => v.frozen.set(true),
            Self::Module(v) => v.frozen.set(true),
            _ => {}
        }
    }
    pub(crate) fn mark_derived(&self) {
        match self {
            Self::Arr(v) => v.mark_derived(),
            Self::Object(v) => v.mark_derived(),
            Self::Collection(v) => v.mark_derived(),
            _ => {}
        }
    }
    fn clear(&self) {
        match self {
            Self::Arr(v) => {
                if let Ok(mut data) = v.data_mut() {
                    data.clear()
                }
            }
            Self::Object(v) => {
                if let Ok(mut data) = v.data_mut() {
                    data.clear()
                }
                if let Some(ext) = v.ext() {
                    if let Ok(mut data) = ext.private.try_borrow_mut() {
                        data.clear()
                    }
                    if let Ok(mut data) = ext.hidden.try_borrow_mut() {
                        data.clear()
                    }
                }
            }
            Self::Collection(v) => {
                if let Ok(mut data) = v.data_mut() {
                    data.clear()
                }
            }
            Self::Cell(v) => {
                if let Ok(mut data) = v.value.try_borrow_mut() {
                    *data = Value::Undef
                }
            }
            Self::Env(_) | Self::Class(_) | Self::Method(_) | Self::Bound(_) | Self::Regex(_) | Self::Promise(_) => {}
            Self::Module(v) => {
                if let Ok(mut bindings) = v.bindings.try_borrow_mut() {
                    bindings.clear();
                }
            }
            Self::Func(_) | Self::Fragment(_) => {}
        }
    }
}
impl Value {
    fn iter_references(&self) -> std::option::IntoIter<Node> {
        Node::of(self).into_iter()
    }
}
impl Candidate {
    fn node(&self) -> Option<Node> {
        match self {
            Self::Arr(v) => v.upgrade().map(Node::Arr),
            Self::Object(v) => v.upgrade().map(Node::Object),
            Self::Collection(v) => v.upgrade().map(Node::Collection),
            Self::Cell(v) => v.upgrade().map(Node::Cell),
            Self::Module(v) => v.upgrade().map(Node::Module),
        }
    }
    fn id(&self) -> usize {
        match self {
            Self::Arr(v) => v.as_ptr() as usize,
            Self::Object(v) => v.as_ptr() as usize,
            Self::Collection(v) => v.as_ptr() as usize,
            Self::Cell(v) => v.as_ptr() as usize,
            Self::Module(v) => v.as_ptr() as usize,
        }
    }
}
pub(crate) fn note_write(target: &Value, value: &Value) {
    if Node::of(value).is_none() {
        return;
    }
    let candidate = match target {
        Value::List(v) => Candidate::Arr(Rc::downgrade(v)),
        Value::Map(v) => Candidate::Object(Rc::downgrade(v)),
        Value::Coll(v) => Candidate::Collection(Rc::downgrade(v)),
        _ => return,
    };
    written(|w| w.push(candidate));
}
pub(crate) fn note_module_write(module: &Rc<crate::modules::Instance>, value: &Value) {
    if Node::of(value).is_some() {
        written(|w| w.push(Candidate::Module(Rc::downgrade(module))));
    }
}
pub(crate) fn note_cell_write(cell: &Rc<VarCell>, value: &Value) {
    if Node::of(value).is_some() {
        written(|w| w.push(Candidate::Cell(Rc::downgrade(cell))));
    }
}

pub(crate) fn sweep() {
    let candidates = written(std::mem::take);
    if candidates.is_empty() {
        return;
    }
    let mut nodes: Vec<Node> = vec![];
    let mut index = HashMap::new();
    let mut edges: Vec<Vec<usize>> = vec![];
    let insert = |node: Node, nodes: &mut Vec<Node>, index: &mut HashMap<usize, usize>, edges: &mut Vec<Vec<usize>>| {
        let id = node.id();
        if let Some(&i) = index.get(&id) {
            return i;
        }
        let i = nodes.len();
        index.insert(id, i);
        nodes.push(node);
        edges.push(vec![]);
        i
    };
    for candidate in &candidates {
        if let Some(node) = candidate.node() {
            insert(node, &mut nodes, &mut index, &mut edges);
        }
    }
    // A checked container is frozen plain data (the state check proved it acyclic and free of
    // functions), so it reaches nothing that could close a cycle: a leaf, never walked. A write
    // next to the kept state then does not walk the whole state.
    let leaf = |n: &Node| match n {
        Node::Arr(v) => v.is_checked(),
        Node::Object(v) => v.is_checked(),
        Node::Collection(v) => v.is_checked(),
        _ => false,
    };
    let mut expanded = vec![false; nodes.len()];
    let mut work: Vec<usize> = (0..nodes.len()).collect();
    while let Some(i) = work.pop() {
        if expanded[i] {
            continue;
        }
        expanded[i] = true;
        let Ok(children) = nodes[i].children() else {
            written(|w| w.extend(candidates));
            return;
        };
        for child in children {
            // Checked plain data cannot be in a cycle: not part of the graph at all.
            if leaf(&child) {
                continue;
            }
            let j = insert(child, &mut nodes, &mut index, &mut edges);
            if expanded.len() < nodes.len() {
                expanded.resize(nodes.len(), false);
            }
            edges[i].push(j);
            if !expanded[j] {
                work.push(j);
            }
        }
    }
    let mut incoming = vec![0; nodes.len()];
    for children in &edges {
        for &j in children {
            incoming[j] += 1;
        }
    }
    // Each node has exactly one collector-owned handle. Everything beyond it and the
    // graph's incoming edges is an external owner, including snapshots and frozen state.
    let mut live = vec![false; nodes.len()];
    let mut work = vec![];
    for (i, node) in nodes.iter().enumerate() {
        if node.strong() > incoming[i] + 1 {
            live[i] = true;
            work.push(i);
        }
    }
    while let Some(i) = work.pop() {
        for &j in &edges[i] {
            if !live[j] {
                live[j] = true;
                work.push(j);
            }
        }
    }
    for (i, node) in nodes.iter().enumerate() {
        if !live[i] {
            node.clear();
        }
    }
    let mut seen = HashSet::new();
    let again = candidates
        .into_iter()
        .filter(|c| index.get(&c.id()).is_some_and(|&i| live[i]) && seen.insert(c.id()))
        .collect::<Vec<_>>();
    written(|w| w.extend(again));
    drop(nodes);
}
