// case: alias-push
const a = [1]; const b = a; b.push(2); return [a, b, a === b];
// case: alias-property
const o = {n: 1}; const p = o; p.n = 2; p.m = 3; return o;
// case: alias-param-mutation
function add(xs, x) { xs.push(x); } const xs = []; add(xs, 1); add(xs, 2); return xs;
// case: alias-nested
const state = {items: []}; const items = state.items; items.push("a"); return state;
// case: alias-in-array
const row = {v: 0}; const rows = [row, row]; rows[0].v = 5; return [rows[1].v, row.v];
// case: alias-closure-counter-object
function counter() { const c = {n: 0}; return {inc: () => { c.n += 1; return c.n; }, read: () => c}; } const k = counter(); k.inc(); k.inc(); const r = k.read(); r.n = 10; return k.inc();
// case: alias-sort-in-place
const a = [3, 1, 2]; const b = a; b.sort(); return a;
// case: alias-object-assign
const t = {a: 1}; const u = t; Object.assign(u, {b: 2}); return t;
// case: alias-delete
const t = {a: 1, b: 2}; const u = t; delete u.a; return t;
// case: alias-mutate-while-iterating
const xs = [1, 2, 3]; const seen = []; for (const x of xs) { seen.push(x); if (x === 1) xs.push(4); } return seen;
// case: alias-foreach-accumulate
const out = {}; ["a", "b", "a"].forEach(k => { out[k] = (out[k] || 0) + 1; }); return out;
// case: alias-reduce-mutating-acc
return ["x", "y", "x"].reduce((acc, k) => { acc[k] = (acc[k] || 0) + 1; return acc; }, {});
// case: alias-identity-after-mutation
const a = [1]; const b = a; a.push(2); return [a === b, b.length];
// case: freeze-shallow
"use strict"; const o = Object.freeze({a: 1, inner: {b: 1}}); o.inner.b = 2; return [o.inner.b, Object.isFrozen(o), Object.isFrozen(o.inner)];
// case: freeze-assign-throws
const o = Object.freeze({a: 1}); try { o.a = 2; return "no throw"; } catch (e) { return [e.name, o.a]; }
// case: freeze-push-throws
const xs = Object.freeze([1]); try { xs.push(2); return "no throw"; } catch (e) { return [e.name, xs.length]; }
// case: structured-clone
const a = {list: [1, {b: 2}]}; const c = structuredClone(a); c.list[1].b = 3; c.list.push(4); return [a, c];

// case: assign_through_call_result
const units = [{to: 0}, {to: 1}];
units.at(-1).to = 5;
const pick = () => units[0];
pick().to += 2;
pick()["k"] = "v";
(units.at(0)).n = 1;
return units;

// case: assign_through_optional_chain_is_rejected
const a = {b: {}};
a?.b.c = 1;
return a;
