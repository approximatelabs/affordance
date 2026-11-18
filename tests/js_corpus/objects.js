// case: key-order-integer-keys
const o = {b: 1, 2: "two", a: 3, 1: "one"}; return Object.keys(o);
// case: key-order-integer-assign
const o = {}; o.z = 1; o["10"] = 2; o["9"] = 3; o.a = 4; return Object.keys(o);
// case: key-order-non-canonical
const o = {"01": 1, "1": 2, "-1": 3, "1.5": 4, "4294967295": 5, "4294967294": 6}; return Object.keys(o);
// case: key-order-from-entries
return Object.keys(Object.fromEntries([["b", 1], ["3", 2], ["1", 3]]));
// case: key-order-json-parse
return Object.keys(JSON.parse('{"b":1,"2":2,"a":3,"0":4}'));
// case: key-order-stringify
return JSON.stringify({b: 1, 1: 2});
// case: numeric-key-literal
const o = {1: "a", 2.5: "b"}; return [o[1], o["2.5"], Object.keys(o)];
// case: spread-object
const a = {x: 1, y: 2}; const b = {...a, y: 3, z: 4}; return [a, b];
// case: spread-object-order
const a = {x: 1, y: 2}; return {y: 0, ...a};
// case: spread-null
return {...null, ...undefined, a: 1};
// case: spread-array-into-object
return {...[1, 2]};
// case: spread-string-into-object
return {..."ab"};
// case: object-entries
return Object.entries({a: 1, b: [2]});
// case: object-values
return Object.values({a: 1, b: "x"});
// case: object-assign
const t = {a: 1}; const r = Object.assign(t, {b: 2}, null, {a: 3}); return [t, r, t === r];
// case: object-fromEntries
return Object.fromEntries([["a", 1], ["b", 2], ["a", 3]]);
// case: object-keys-array
return [Object.keys([4, 5]), Object.keys("ab"), Object.values("ab")];
// case: object-assign-overwrite-order
const o = {a: 1, b: 2}; o.a = 3; return Object.keys(o);
// case: object-delete
const o = {a: 1, b: 2}; delete o.a; return o;
// case: object-in
const o = {a: undefined}; return ["a" in o, "b" in o, 0 in [1], "length" in [], "toString" in o];
// case: hasOwnProperty
return [{a: 1}.hasOwnProperty("a"), {a: 1}.hasOwnProperty("b")];
// case: object-nested-update
const o = {a: {b: 1}}; o.a.b = 2; o.a.c = 3; return o;
// case: object-computed-key
const k = "x"; return {[k]: 1, [k + "y"]: 2, [1 + 1]: 3};
// case: object-shorthand
const a = 1, b = 2; return {a, b};
// case: object-method-call
const o = {f: x => x + 1}; return o.f(1);
// case: object-to-string
return [String({}), {} + "", `${{a: 1}}`];
// case: object-equality
const o = {}; return [o === o, {} === {}, o == "[object Object]"];
// case: destructure-object
const {a, b = 2, c} = {a: 1, c: undefined}; return [a, b, c];
// case: destructure-object-rest
const {a, ...rest} = {a: 1, b: 2, c: 3}; return [a, rest];
// case: destructure-object-rename
const {a: x, b: {c}} = {a: 1, b: {c: 2}}; return [x, c];
// case: destructure-null
const {a} = null; return a;
// case: destructure-params
const f = ({a, b = 5}) => a + b; return [f({a: 1}), f({a: 1, b: 2})];
// case: destructure-params-array
const f = ([a, b]) => a * b; return f([3, 4]);
// case: optional-chaining
const o = {a: {b: null}, f: null}; return [o?.a?.b, o.x?.y, o.a.b?.c, o.f?.(), o.g?.h.i];
// case: optional-call
const o = {f: () => 1}; return [o.f?.(), o.missing?.()];
// case: optional-index
const o = {xs: [1]}; return [o.xs?.[0], o.ys?.[0]];
// case: nullish
return [null ?? 1, undefined ?? 2, 0 ?? 3, "" ?? 4, false ?? 5, NaN ?? 6];
// case: nullish-assign
let a = null; a ??= 5; let b = 0; b ??= 6; return [a, b];
// case: logical-assign
let a = 0; a ||= 5; let b = 1; b &&= 7; return [a, b];
// case: property-of-undefined
const o = {}; return o.a.b;
// case: property-of-null
const o = {a: null}; return o.a.b;
// case: property-of-number
return [(5).x, "s".x, true.x];
// case: string-length-property
return ["abc".length, [1, 2].length, {length: 7}.length];
// case: typeof-all
return [typeof 1, typeof "s", typeof true, typeof undefined, typeof null, typeof {}, typeof [], typeof (() => 1), typeof NaN, typeof missingName, typeof /x/];
// case: object-keys-with-undefined-values
return Object.keys({a: undefined, b: 1});
// case: object-assign-to-frozen-like
const o = {a: 1}; const p = o; p.a = 2; return o.a;
// case: map-of-objects
const people = [{name: "a", age: 3}, {name: "b", age: 1}]; return people.map(({name, age}) => `${name}:${age}`);
// case: nested-spread-update
const s = {list: [1, 2], meta: {n: 1}}; const t = {...s, meta: {...s.meta, n: 2}, list: [...s.list, 3]}; return [s, t];
// case: object-number-keys-access
const o = {"1": "a"}; return [o[1], o["1"], o[1.0]];
// case: object-key-coercion
const o = {}; o[true] = 1; o[null] = 2; o[undefined] = 3; return Object.keys(o);
// case: array-as-key
const o = {}; o[[1, 2]] = "x"; return Object.keys(o);
// case: object-for-in-order
const o = {b: 1, a: 2, 1: 3}; const out = []; for (const k in o) out.push(k); return out;
// case: error-object-keys
return Object.keys(new Error("x"));
// case: error-props
const e = new Error("boom"); return [e.message, e.name, String(e), e instanceof Error];
// case: error-stringify
return JSON.stringify(new Error("x"));
// case: typeerror-props
const e = new TypeError("bad"); return [e.name, e.message, `${e}`];
// case: getter-like-length-on-object
return {a: 1}.length;
// case: key-order-integer-insertion-paths
const o = {}; o[5] = 1; o.s = 2; o[7] = 3; o[1] = 4; o[6] = 5; o[0] = 6; o[100] = 7; o.t = 8; o[50] = 9; o[5] = 10; o["08"] = 11; o[4294967294] = 12; o[4294967295] = 13; return [Object.keys(o), Object.values(o)];
// case: key-order-integer-many
const o = {}; for (let i = 0; i < 60; i++) o[(i * 37) % 101] = i; o.x = 1; for (let i = 200; i > 150; i -= 7) o[i] = i; o[3] = "again"; return [Object.keys(o).join(","), Object.values(o).join(",")];
// case: key-order-json-parse-descending
const o = JSON.parse('{"3":1,"x":0,"1":2,"2":3,"0":4,"10":5,"y":6,"9":7}'); return [Object.keys(o), JSON.stringify(o)];
