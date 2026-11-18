// case: split-limit-is-a-uint32
return ["a1b2c".split(/\d/, 2.0), "a1b2c".split(/\d/, 2.7), "a1b2c".split("1", 1.0)];
// case: json-escape-needs-four-hex-digits
try { JSON.parse('"\\u+041"'); return "accepted"; } catch (e) { return e.name; }
// case: fromCodePoint-range
const out = []; for (const n of [-1, 1114112, 1.5]) { try { String.fromCodePoint(n); out.push("ok"); } catch (e) { out.push(e.name); } } return [out, String.fromCodePoint(65, 128512)];
// case: structuredClone-copies-a-regex
const o = {r: /a/g}; const c = structuredClone(o); c.r.test("aaa"); return [o.r.lastIndex, c.r.lastIndex];
// case: match-with-no-pattern
return ["abc".match(), "abc".search()];
// case: break-leaves-the-loop-not-a-labelled-block
const seen = []; for (let i = 0; i < 3; i++) { blk: { seen.push("in" + i); if (i === 1) break; } seen.push("after" + i); } return seen;
// case: ternary-with-an-arrow-branch
const id = v => v; const pick = flag => flag ? id : x => x * 2; return [pick(true)(4), pick(false)(4)];
// case: spread-reads-before-later-items-run
let x = [1]; const g = () => { x.push(2); return 3; }; x = [...x, g()]; return x;
// case: switch-has-one-scope
let got = ""; switch (1) { case 1: let q = 5; case 2: got = String(q); } return got;
// case: regex-replace-takes-any-callable
const f = function (m) { return "[" + m + "]"; }; return ["a-b".replace(/-/g, f.bind(null)), "a1b".replace(/\d/, String)];
// case: regex-replacer-offsets-count-characters
const offs = []; "é-b-c".replace(/-/g, (m, off) => { offs.push(off); return m; }); return offs;
// case: an-objects-own-toString-gives-its-text
const log = []; const key = {toString() { log.push("t"); return "k"; }}; return [Object.keys({[key]: 1}), `${key}`, String(key), "" + key, key + "!", ({k: 7})[key], log.length];
// case: an-objects-own-valueOf-gives-its-number
const v = {valueOf() { return 3; }}; let n = {valueOf() { return 1; }}; n++; let m = {valueOf() { return 4; }}; m += 1; return [v * 2, +v, -v, v > 2, v + 1, Number(v), v == 3, v === 3, ~v, n, m, `${v}`, String(v)];
// case: toString-and-valueOf-together
const o = {toString() { return "text"; }, valueOf() { return 42; }}; return [`${o}`, o + "", o + 1, String(o), Number(o), o < 50, o == 42, o == "text"];
// case: a-class-defines-toString
class Money { constructor(n) { this.n = n; } toString() { return "$" + this.n; } valueOf() { return this.n; } } const a = new Money(5), b = new Money(7); return [`${a}`, a + b, a < b, String(b), [a, b].map(String), {[a]: 1}];
// case: an-object-without-them-prints-as-before
const o = {a: 1}; return [`${o}`, String(o), o + "", +o, o == o, o == "[object Object]", o == null];
// case: a-conversion-that-returns-an-object-is-a-TypeError
const bad = {toString() { return {}; }}; try { return `${bad}`; } catch (e) { return e.name; }
// case: valueOf-that-returns-an-object-falls-to-toString
const o = {valueOf() { return {}; }, toString() { return "7"; }}; return [o * 2, `${o}`];
// case: arrays-ask-their-items-for-text
const o = {toString() { return "T"; }}; const deep = [o, [o, 2]]; return [[o, 1].join("-"), String([o]), `${[o]}`, deep + "", [o].join(o), [1, 2] + "", [[1, [2]], 3].join(";")];
// case: string-methods-ask-an-object-argument
const o = {toString() { return "T"; }}; const n = {valueOf() { return 2; }}; return ["x".concat(o), "aTb".includes(o), "aTb".split(o), "a".padEnd(3, o), "abcd".slice(n), "abcd".charAt(n), "ab".repeat(n), "aTbT".indexOf(o, n), "aTb".replace(o, "-")];
// case: math-and-number-functions-ask-for-a-number
const v = {valueOf() { return 3; }}; return [Math.max(v, 1), Math.abs(v), Math.pow(v, v), parseInt(v), isNaN(v), isFinite(v), (255).toString({valueOf() { return 16; }}), (1.234).toFixed(v)];
// case: in-and-delete-ask-the-key-for-text
const o = {toString() { return "b"; }}; const m = {a: 1, b: 2}; m[o] = 9; const q = [o in m, m[o]]; delete m[o]; return [m, q];
// case: two-objects-with-valueOf
const a = {valueOf() { return 2; }}, b = {valueOf() { return 2; }}; return [a == b, a <= b, a >= b, a - b, a + b, a ** b, a % b, a | b, a != b];
// case: a-list-that-contains-itself-still-prints
const o = {toString() { return "T"; }}; const xs = [o]; xs.push(xs); return String(xs);
