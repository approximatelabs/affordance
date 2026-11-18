//! `memoScan(items, init, step, deps)`: a fold whose per-item states and outputs an agent keeps
//! between renders, so a render does the work of what is new, not of the whole history.
//!
//! `step(state, item, index)` returns `[next_state, out]`; `memoScan` returns
//! `{state: final_state, outs: [out, ...]}`. The contract: `step` depends only on its arguments,
//! on `deps` and on invariant module constants. A document-local value whose identity matters
//! belongs in `deps`, including an object-valued module constant. Then, when `items` start like
//! a kept scan's (`Object.is`: shared transcript records), the states and outputs for those are
//! reused and only the rest are computed. Reused outputs are the kept values themselves, so a
//! stage downstream that scans them compares them by pointer.
//!
//! A scan is identified by its call site, the component path and its order among that site's
//! calls in the render. A render resumes from the kept variant with its deps and init that shares
//! the longest prefix with its items; the render then keeps only its own result (a step that fails
//! keeps the prefix computed before it, so the next render resumes there). A scan not used in
//! a render is dropped at the next one. With `verify`, every scan is
//! also computed from scratch and compared, and a difference is an error (a step that broke the
//! contract).
//!
//! The store also keeps the turn each transcript record maps to (`TurnStrategy`), by record.
use crate::emit::Item;
use crate::value::{Text, Value};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

pub type ScanKey = (usize, String, usize);

pub struct Entry {
    /// Keep the call site's code alive while its address identifies a cached scan.
    pub(crate) _module: Rc<crate::modules::Instance>,
    pub deps: Value,
    pub init: Value,
    pub items: Vec<Value>,
    pub states: Vec<Value>,
    pub outs: Vec<Value>,
}

#[derive(Default)]
pub struct MemoStore {
    scans: HashMap<ScanKey, Vec<Entry>>,
    used: HashSet<ScanKey>,
    /// A record's turn, by the record's address: (the record, kept so the address stays its own; the turn).
    turns: HashMap<(usize, u64), (Value, Value)>,
    old_turns: HashMap<(usize, u64), (Value, Value)>,
    /// Where the turns being asked for are rendered (`turn_scope`): part of every key.
    scope: u64,
    /// The same, by a record's fields: a record rebuilt every render with the same fields
    /// (`{kind: "input", content: intro}`) gets the same turn, so the template's scans downstream
    /// keep reusing their history. A turn is the runtime's pure function of the record's content.
    by_fields: HashMap<(u64, u64), (Value, Value)>,
    old_by_fields: HashMap<(u64, u64), (Value, Value)>,
    /// Items reused and computed, over the store's life.
    pub reused: usize,
    pub computed: usize,
    pub verify: bool,
    /// Scans that had a kept variant and could reuse none of it (`memo_miss` trace entries):
    /// an app whose scan inputs change identity every render pays the whole history each time.
    pub misses: Vec<serde_json::Value>,
    reported: HashSet<String>,
}

/// Scans shorter than this are cheap to recompute; a miss is not worth reporting.
const MISS_REPORT: usize = 64;

impl MemoStore {
    /// The record-to-turn cache's records and turns. (A scan's outputs are marked `derived` when
    /// it computes them, so a recovery image refuses them without walking the scans.)
    pub(crate) fn checkpoint_turns(&self) -> (Vec<Value>, Vec<Value>) {
        self.turns.values().chain(self.old_turns.values()).cloned().unzip()
    }
    pub fn new() -> Self {
        MemoStore {
            verify: std::env::var("AFFORDANCE_MEMO_VERIFY").is_ok(),
            ..Default::default()
        }
    }

    /// A new turn: scans and record turns not used since the last one are dropped.
    pub fn next_generation(&mut self) {
        let used = std::mem::take(&mut self.used);
        self.scans.retain(|k, _| used.contains(k));
        self.old_turns = std::mem::take(&mut self.turns);
        self.old_by_fields = std::mem::take(&mut self.by_fields);
    }

    /// Where a scan of `items` resumes: the kept prefix length and its states and outputs, from
    /// the variant with these deps and init sharing the longest prefix with `items` (taken out:
    /// the result replaces it).
    pub fn resume(
        &mut self,
        key: &ScanKey,
        deps: &Value,
        init: &Value,
        items: &[Value],
    ) -> (usize, Vec<Value>, Vec<Value>) {
        self.used.insert(key.clone());
        let Some(list) = self.scans.get_mut(key) else {
            return (0, vec![], vec![]);
        };
        let best = list
            .iter()
            .enumerate()
            .filter(|(_, e)| deps_eq(&e.deps, deps) && crate::jsvalue::same_value(&e.init, init))
            .map(|(i, e)| {
                (
                    i,
                    e.items
                        .iter()
                        .zip(items)
                        .take_while(|(x, y)| crate::jsvalue::same_value(x, y))
                        .count(),
                )
            })
            .max_by_key(|&(i, n)| (n, std::cmp::Reverse(i)));
        // A miss: what must be recomputed is large and the kept variant diverges before its end
        // (not a longer list that only gained items). Diagnosed only then, once per scan.
        let reused = best.map_or(0, |(_, n)| n);
        if items.len() - reused >= MISS_REPORT && !self.reported.contains(&key.1) {
            let why = list
                .iter()
                .map(|e| {
                    (
                        deps_eq(&e.deps, deps),
                        crate::jsvalue::same_value(&e.init, init),
                        e.items
                            .iter()
                            .zip(items)
                            .take_while(|(x, y)| crate::jsvalue::same_value(x, y))
                            .count(),
                        e.items.len(),
                    )
                })
                .max_by_key(|(d, i, n, _)| (*d && *i, *n));
            if let Some((d, i, n, kept)) = why.filter(|b| b.2 < b.3.min(items.len()) || !(b.0 && b.1)) {
                self.reported.insert(key.1.clone());
                let reason = if !d {
                    "deps differ (by Object.is: an object or array literal is new every render; pass a list of stable values)"
                } else if !i {
                    "init differs (by Object.is: an object or array literal is new every render; use null and a default inside the step)"
                } else {
                    "an item differs from the kept one at the same index (items are new objects every render; keep the records themselves)"
                };
                self.misses.push(serde_json::json!({"t": "memo_miss", "site": key.1, "items": items.len(), "kept": kept, "matching_prefix": n, "reason": reason}));
            }
        }
        match best {
            None | Some((_, 0)) => (0, vec![], vec![]),
            Some((i, n)) => {
                let mut e = list.remove(i);
                e.states.truncate(n);
                e.outs.truncate(n);
                (n, e.states, e.outs)
            }
        }
    }

    /// Keep `entry`, the scan's one variant.
    pub fn put(&mut self, key: ScanKey, entry: Entry) {
        let list = self.scans.entry(key).or_default();
        list.clear();
        list.push(entry);
    }

    /// The turn kept for this record (`record` is a shared map: its address is its identity).
    /// A record's turn depends on where it is rendered as well as on the record: an action's
    /// result box is styled by rules that may name its ancestors (`Panel[mode="compact"] result`).
    /// `scope` says where (the rules and the ancestors' attributes); a turn kept under one scope
    /// is never returned under another.
    pub fn turn_scope(&mut self, scope: u64) {
        self.scope = scope;
    }

    pub fn turn(&mut self, record: &Value) -> Option<Value> {
        let Value::Map(m) = record else { return None };
        let k = (Rc::as_ptr(m) as usize, self.scope);
        if let Some((_, t)) = self.turns.get(&k) {
            return Some(t.clone());
        }
        if let Some((r, t)) = self.old_turns.remove(&k) {
            self.turns.insert(k, (r, t.clone()));
            return Some(t);
        }
        // A rebuilt record with the same fields: the same turn.
        let key = (shallow_key(record)?, self.scope);
        let found = self
            .by_fields
            .get(&key)
            .or_else(|| self.old_by_fields.get(&key))
            .filter(|(r, _)| shallow_eq(r, record))
            .map(|(_, t)| t.clone())?;
        self.keep_turn(record, found.clone());
        Some(found)
    }

    pub fn keep_turn(&mut self, record: &Value, turn: Value) {
        if let Value::Map(m) = record {
            // Kept by address: neither may change.
            crate::value::freeze(record);
            // What the cache made (not the record's own values, frozen just above) is marked as
            // derived, as a scan's outputs are: a recovery image refuses to write one, whenever
            // it is written. Deciding that from the cache's contents at writing time could miss
            // a turn the cache had since let go of.
            crate::value::freeze_derived(&turn);
            if let Some(key) = shallow_key(record) {
                self.by_fields.insert((key, self.scope), (record.clone(), turn.clone()));
            }
            self.turns
                .insert((Rc::as_ptr(m) as usize, self.scope), (record.clone(), turn));
        }
    }
}

// A single undefined dependency is a value here, not an effect's run-after-every-render request.
fn deps_eq(a: &Value, b: &Value) -> bool {
    matches!((a, b), (Value::Undef, Value::Undef)) || !crate::jsvalue::deps_changed(a, b)
}

/// A record's fields, for recognizing a record rebuilt with the same fields: field names with
/// each value's identity (a scalar or a string by value, a container by address).
fn shallow_key(record: &Value) -> Option<u64> {
    use std::hash::{Hash, Hasher};
    let Value::Map(m) = record else { return None };
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for (k, v) in m.read().iter() {
        k.hash(&mut h);
        match v {
            Value::Str(t) => {
                0u8.hash(&mut h);
                (**t).hash(&mut h);
            }
            Value::Int(i) => {
                1u8.hash(&mut h);
                i.hash(&mut h);
            }
            Value::Float(f) => {
                2u8.hash(&mut h);
                f.to_bits().hash(&mut h);
            }
            Value::Bool(b) => {
                3u8.hash(&mut h);
                b.hash(&mut h);
            }
            Value::None | Value::Undef => 4u8.hash(&mut h),
            Value::List(l) => {
                5u8.hash(&mut h);
                (Rc::as_ptr(l) as usize).hash(&mut h);
            }
            Value::Map(o) => {
                6u8.hash(&mut h);
                (Rc::as_ptr(o) as usize).hash(&mut h);
            }
            Value::Frag(f) => {
                7u8.hash(&mut h);
                (Rc::as_ptr(f) as usize).hash(&mut h);
            }
            _ => return None,
        }
    }
    Some(h.finish())
}

/// The same fields, each identical (React's `shallowEqual` over a record's own fields).
fn shallow_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Map(x), Value::Map(y)) => {
            let (x, y) = (x.read(), y.read());
            x.len() == y.len()
                && x.iter()
                    .zip(y.iter())
                    .all(|((k1, v1), (k2, v2))| k1 == k2 && crate::jsvalue::same_value(v1, v2))
        }
        _ => false,
    }
}

/// The same value: identical shared data (a list, a map, a fragment), or an equal scalar or string.
pub fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Map(x), Value::Map(y)) => Rc::ptr_eq(x, y),
        (Value::List(x), Value::List(y)) => Rc::ptr_eq(x, y),
        (Value::Coll(x), Value::Coll(y)) => Rc::ptr_eq(x, y),
        (Value::Frag(x), Value::Frag(y)) => Rc::ptr_eq(x, y),
        (Value::Str(Text::Heap(x)), Value::Str(Text::Heap(y))) => Rc::ptr_eq(x, y) || x == y,
        (Value::Str(x), Value::Str(y)) => **x == **y,
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => x.to_bits() == y.to_bits(),
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::None, Value::None) | (Value::Undef, Value::Undef) => true,
        (Value::BigInt(x), Value::BigInt(y)) => x == y,
        (Value::Token(x, n), Value::Token(y, m)) => x == y && n == m,
        (Value::Native(x), Value::Native(y)) => x == y,
        (Value::Setter(x), Value::Setter(y)) => x == y,
        (Value::Method(_), Value::Method(_)) => crate::jsvalue::strict_eq(a, b),
        _ => false,
    }
}

/// Structural result comparison. Verification may recreate equivalent output closures.
pub fn deep_eq(a: &Value, b: &Value) -> bool {
    if same(a, b) {
        return true;
    }
    match (a, b) {
        (Value::Map(x), Value::Map(y)) => {
            x.read().len() == y.read().len()
                && x.read()
                    .iter()
                    .zip(y.read().iter())
                    .all(|((k1, v1), (k2, v2))| k1 == k2 && deep_eq(v1, v2))
        }
        (Value::List(x), Value::List(y)) => {
            x.read().len() == y.read().len() && x.read().iter().zip(y.read().iter()).all(|(p, q)| deep_eq(p, q))
        }
        (Value::Frag(x), Value::Frag(y)) => x.len() == y.len() && x.iter().zip(y.iter()).all(|(p, q)| item_eq(p, q)),
        (Value::Lambda(x, _), Value::Lambda(y, _)) => Rc::ptr_eq(x, y),
        (Value::Func(x), Value::Func(y)) => Rc::ptr_eq(&x.definition, &y.definition),
        _ => false,
    }
}

fn item_eq(a: &Item, b: &Item) -> bool {
    match (a, b) {
        (Item::Text(x), Item::Text(y)) => x == y,
        (Item::Tok(x), Item::Tok(y)) => x == y,
        (Item::Open(x), Item::Open(y)) => Rc::ptr_eq(x, y) || x == y,
        (Item::Close, Item::Close) | (Item::Segment, Item::Segment) => true,
        (Item::Tokens(x, s), Item::Tokens(y, t)) => x == y && s == t,
        (Item::Turns(x), Item::Turns(y)) => Rc::ptr_eq(x, y),
        _ => false,
    }
}
