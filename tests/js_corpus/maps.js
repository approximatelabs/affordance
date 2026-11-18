// case: map-basic
const m = new Map(); m.set("a", 1).set("b", 2); return [m.size, m.get("a"), m.get("z"), m.has("b"), m.has("z")];
// case: map-order-and-iteration
const m = new Map([["z", 1], ["a", 2]]); m.set("m", 3); m.set("z", 9); return [[...m], [...m.keys()], [...m.values()], Array.from(m.entries())];
// case: map-delete-clear
const m = new Map([[1, "a"], [2, "b"]]); const d = [m.delete(1), m.delete(5)]; const s1 = m.size; m.clear(); return [d, s1, m.size];
// case: map-object-keys-identity
const k1 = {}; const k2 = {}; const m = new Map([[k1, "one"]]); return [m.get(k1), m.get(k2), m.has({})];
// case: map-samevaluezero
const m = new Map(); m.set(NaN, "nan"); m.set(0, "zero"); return [m.get(NaN), m.get(-0), m.size, m.get("0")];
// case: map-foreach
const out = []; new Map([["a", 1], ["b", 2]]).forEach((v, k, map) => out.push(`${k}=${v}/${map.size}`)); return out;
// case: map-for-of-destructure
const out = []; for (const [k, v] of new Map([["x", 1], ["y", 2]])) out.push(k + v); return out;
// case: map-from-entries
return [Object.fromEntries(new Map([["a", 1], ["b", 2]])), new Map(Object.entries({c: 3})).get("c")];
// case: map-json
return [JSON.stringify(new Map([["a", 1]])), JSON.stringify({m: new Map()}), typeof new Map(), new Map() instanceof Map];
// case: map-mutate-during-iteration
const m = new Map([["a", 1]]); const seen = []; for (const [k] of m) { seen.push(k); if (k === "a") m.set("b", 2); } return seen;
// case: map-groupby
const g = Map.groupBy([1, 2, 3, 4, 5], x => x % 2 ? "odd" : "even"); return [[...g.keys()], g.get("odd")];
// case: map-alias
const m = new Map(); const n = m; n.set("k", 1); return m.get("k");
// case: map-counter
const counts = new Map(); for (const w of "a b a c b a".split(" ")) counts.set(w, (counts.get(w) ?? 0) + 1); return [...counts].sort((x, y) => y[1] - x[1]);
// case: set-basic
const s = new Set([3, 1, 3, 2, 1]); s.add(5).add(1); return [s.size, [...s], s.has(2), s.has(4)];
// case: set-delete-iteration
const s = new Set(["a", "b", "c"]); s.delete("b"); const out = []; s.forEach((v, v2, set) => out.push(v + v2 + set.size)); return [out, [...s.values()], [...s.entries()]];
// case: set-dedupe
return [...new Set("mississippi")].join("");
// case: set-object-identity
const o = {}; const s = new Set([o, o, {}]); return [s.size, s.has(o)];
// case: set-json-and-type
return [JSON.stringify(new Set([1])), typeof new Set(), new Set() instanceof Set, Array.isArray(new Set())];
// case: map-get-missing-method
const m = new Map(); try { m.nope(); return "no"; } catch (e) { return e.name; }
