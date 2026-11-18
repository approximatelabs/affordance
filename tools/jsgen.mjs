// Seeded random JavaScript programs over the view language's subset, for differential testing
// against Node (tests/js_differential.rs). Usage: node tools/jsgen.mjs <seed> <count>
// Prints a JSON array of {name, body}: each body is a function body that returns a value.
//
// The programs avoid the documented differences (spec/LANGUAGE.md, "Differences from
// JavaScript"): no characters outside the Basic Multilingual Plane (strings index by code
// points), no sparse arrays, no regex lastIndex, no enumeration of error objects, no prototype
// members in the expression grammar, no non-ASCII localeCompare or inexact sin/cos/pow results. Mutation through aliases is generated on purpose: objects
// are references, as in JavaScript.
// Each generated expression runs in its own try/catch, so one throw does not hide the rest.

const seed = Number(process.argv[2] ?? 1);
const count = Number(process.argv[3] ?? 100);

// mulberry32
function rng(a) {
  return () => {
    a |= 0; a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}
let R = rng(seed);
const int = (n) => Math.floor(R() * n);
const pick = (xs) => xs[int(xs.length)];
const chance = (p) => R() < p;
const times = (n, f) => Array.from({ length: n }, (_, i) => f(i));

// ---------------- literals ----------------

const NUMS = ["0", "-0", "1", "-1", "2", "3", "7", "10", "-7", "0.5", "-0.5", "1.5", "2.5", "-2.5", "0.1", "0.2", "0.3", "1e21", "1e-7", "123.456",
  "9007199254740991", "9007199254740993", "4294967296", "2147483647", "-2147483648", "NaN", "Infinity", "-Infinity", "1/3", "100", "255", "1e300", "5e-324",
  "0.000001", "1234.5678", "99.995", "1.005", "8.345", "0x1f", "1_000", ".25", "3e2"];
function numLit() {
  if (chance(0.35)) {
    const v = pick([int(1000) - 500, (R() * 2000 - 1000), R() * 1e6, R(), -R() * 10, int(10)]);
    let s = String(Number(v.toPrecision(1 + int(8))));
    if (s.startsWith("-")) s = "(" + s + ")";
    return s;
  }
  const s = pick(NUMS);
  return s.startsWith("-") || s.includes("/") ? "(" + s + ")" : s;
}

const CHARS = ["a", "b", "c", "x", "y", "z", "A", "B", "Z", "0", "1", "2", "9", " ", "-", "_", ",", ".", "é", "ß", "日", "本", "\\n", "\\t", "'", "\\\"", "\\\\", "$", "{", "}", "İ", "Σ", "ς", "\\u00a0", "\\u2028", "e", "E", "+"];
function strLit() {
  if (chance(0.15)) return pick(['""', '" "', '"abc"', '"a,b,,c"', '"  pad  "', '"12px"', '"0x1A"', '"  -3.5e2xyz"', '"Infinity"', '"-0"', '"1e1000"', '"010"', '"0b101"', '"\\t\\n42\\n"', '"aXbXc"', '"Hello World"', '"ÄÖÜ straße"']);
  return '"' + times(int(7), () => pick(CHARS)).join("") + '"';
}

// ASCII only (localeCompare beyond ASCII is a documented difference).
function asciiStrLit() {
  return '"' + times(int(6), () => pick(["a", "b", "B", "A", "z", "Z", "0", "9", " ", "-", "_", ".", "!", "~", "$", "aa", "Ab"])).join("") + '"';
}

// A mutated receiver may be one of the program's variables (the mutation is seen through every
// reference to it), or a copy.
const fresh = (a) => (/^[a-z]+$/.test(a) && chance(0.5) ? `[...${a}]` : a);
const freshObj = (o) => (/^[a-z]+$/.test(o) && chance(0.5) ? `{...${o}}` : o);

const IDENT_KEYS =["a", "b", "c", "x", "y", "key", "name", "value", "length", "z"];
function keyLit() {
  return pick([pick(IDENT_KEYS), '"' + pick(IDENT_KEYS) + '"', String(int(12)), '"' + int(12) + '"', '"01"', '"-1"', '"1.5"', "[" + strLit() + "]", "[" + int(5) + "]"]);
}

// A property key as an expression (`o[k]`, `k in o`).
function keyExpr() {
  return pick(["\"" + pick(IDENT_KEYS) + "\"", String(int(12)), "\"" + int(12) + "\"", "\"01\"", "\"-1\"", "\"1.5\"", strLit(), "1.5", "-0", "null", "true"]);
}

// ---------------- expressions by kind ----------------

const VARS = { num: ["n", "m"], str: ["s", "t"], arr: ["xs", "ys"], obj: ["o", "p"] };

function num(d) {
  if (d <= 0 || chance(0.25)) return chance(0.2) && VARS.num.length ? pick(VARS.num) : numLit();
  const k = int(20);
  switch (k) {
    case 0: case 1: case 2: return `(${num(d - 1)} ${pick(["+", "-", "*", "/", "%"])} ${num(d - 1)})`;
    case 3: return `(${any(d - 1)} ${pick(["-", "*", "/", "%"])} ${any(d - 1)})`;
    case 4: return `Math.${pick(["floor", "ceil", "round", "trunc", "abs", "sign", "sqrt", "cbrt", "fround", "clz32"])}(${num(d - 1)})`;
    case 5: return `Math.${pick(["max", "min", "hypot", "atan2", "imul"])}(${times(int(4), () => num(d - 1)).join(", ")})`;
    case 6: return `Math.${pick(["exp", "log", "log2", "log10", "tan", "atan", "sinh", "tanh", "expm1", "log1p", "asinh"])}(${num(d - 1)})`;
    case 7: return `parseInt(${pick([strLit(), num(d - 1), str(d - 1)])}${chance(0.4) ? ", " + pick(["2", "8", "10", "16", "36", "0", "1", "37", "undefined", num(d - 1)]) : ""})`;
    case 8: return `parseFloat(${pick([strLit(), num(d - 1), str(d - 1)])})`;
    case 9: return `Number(${any(d - 1)})`;
    case 10: return `${str(d - 1)}.length`;
    case 11: return `${arr(d - 1)}.length`;
    case 12: return `${arr(d - 1)}.indexOf(${any(d - 1)}${chance(0.3) ? ", " + num(d - 1) : ""})`;
    case 13: return `${str(d - 1)}.${pick(["indexOf", "lastIndexOf"])}(${str(d - 1)}${chance(0.4) ? ", " + num(d - 1) : ""})`;
    case 14: return `(${pick(["-", "+", "~"])}${any(d - 1)})`;
    case 15: return `(${num(d - 1)} ${pick(["|", "&", "^", "<<", ">>", ">>>"])} ${num(d - 1)})`;
    case 16: return `${arr(d - 1)}.reduce((acc, v) => acc + (typeof v === "number" ? v : 1), ${num(d - 1)})`;
    case 17: return `${str(d - 1)}.charCodeAt(${num(d - 1)})`;
    case 18: return `${arr(d - 1)}.findLastIndex(v => ${bool(d - 1, "v")})`;
    default: return `(${bool(d - 1)} ? ${num(d - 1)} : ${num(d - 1)})`;
  }
}

function str(d) {
  if (d <= 0 || chance(0.25)) return chance(0.2) && VARS.str.length ? pick(VARS.str) : strLit();
  const k = int(26);
  const n = () => (chance(0.3) ? pick(["0", "1", "2", "-1", "-2", "10", "undefined", "NaN", "1.5", "Infinity", "-Infinity"]) : num(d - 1));
  switch (k) {
    case 0: return `(${str(d - 1)} + ${any(d - 1)})`;
    case 1: return `(${any(d - 1)} + ${str(d - 1)})`;
    case 2: return `${str(d - 1)}.slice(${n()}${chance(0.5) ? ", " + n() : ""})`;
    case 3: return `${str(d - 1)}.substring(${n()}${chance(0.5) ? ", " + n() : ""})`;
    case 4: return `${str(d - 1)}.${pick(["toUpperCase", "toLowerCase", "trim", "trimStart", "trimEnd"])}()`;
    case 5: return `${str(d - 1)}.${pick(["padStart", "padEnd"])}(${pick(["0", "3", "8", "-1", n()])}${chance(0.6) ? ", " + strLit() : ""})`;
    case 6: return `${str(d - 1)}.repeat(${pick(["0", "1", "2", "3"])})`;
    case 7: return `${str(d - 1)}.${pick(["replace", "replaceAll"])}(${str(d - 1)}, ${pick([strLit(), '"[$&]"', '"$$"', "\"$'\"", '"$`"', '"$1"', "(m, i) => m.length + \":\" + i", "m => m.toUpperCase()"])})`;
    case 8: return `${arr(d - 1)}.join(${chance(0.7) ? strLit() : ""})`;
    case 9: return `String(${any(d - 1)})`;
    case 10: return `\`<\${${any(d - 1)}}|\${${any(d - 1)}}>\``;
    case 11: return `JSON.stringify(${any(d - 1)}${chance(0.3) ? ", null, " + pick(["2", "0", '"--"', '"\\t"', "11", "-3", '""']) : ""})`;
    case 12: return `(${num(d - 1)}).toFixed(${pick(["0", "1", "2", "3", "5", "10", "20"])})`;
    case 13: return `(${num(d - 1)}).toString(${pick(["", "2", "8", "16", "36", "10", "3", "7"])})`;
    case 14: return `${str(d - 1)}.${pick(["at", "charAt"])}(${n()})`;
    case 15: return `${str(d - 1)}.split(${pick([strLit(), '""', "undefined", '","', '" "'])}${chance(0.3) ? ", " + pick(["0", "1", "2", "-1"]) : ""}).join("|")`;
    case 16: return `${str(d - 1)}.concat(${times(int(3), () => any(d - 1)).join(", ")})`;
    case 17: return `typeof ${any(d - 1)}`;
    case 18: return `Object.keys(${obj(d - 1)}).join()`;
    case 19: return `(${bool(d - 1)} ? ${str(d - 1)} : ${str(d - 1)})`;
    case 20: return `${str(d - 1)}.substr(${n()}${chance(0.5) ? ", " + n() : ""})`;
    case 21: return `String.fromCharCode(${times(1 + int(3), () => pick(["65", "97", "233", "0x41", "8364", "48"])).join(", ")})`;
    case 22: return `${str(d - 1)}.replace(${pick(["/[aeiou]/g", "/(\\w)(\\w)/", "/\\s+/g", "/\\d+/g", "/x*/g"])}, ${pick(['"-"', '"$2$1"', '"<$&>"', "(m) => m.length"])})`;
    case 23: return `${arr(d - 1)}.toString()`;
    case 24: return `[${times(int(5), () => asciiStrLit()).join(", ")}].sort((a, b) => a.localeCompare(b)).join()`;
    default: return `(${num(d - 1)} + "")`;
  }
}

function bool(d, v) {
  if (d <= 0 || chance(0.2)) return pick(["true", "false", v ?? "true"]);
  const x = () => (v && chance(0.5) ? v : any(d - 1));
  const k = int(12);
  switch (k) {
    case 0: case 1: return `(${x()} ${pick(["==", "!=", "===", "!=="])} ${any(d - 1)})`;
    case 2: case 3: return `(${x()} ${pick(["<", "<=", ">", ">="])} ${any(d - 1)})`;
    case 4: return `!${x()}`;
    case 5: return `(${bool(d - 1, v)} ${pick(["&&", "||"])} ${bool(d - 1, v)})`;
    case 6: return `${arr(d - 1)}.includes(${any(d - 1)})`;
    case 7: return `Array.isArray(${x()})`;
    case 8: return `Number.${pick(["isInteger", "isFinite", "isNaN", "isSafeInteger"])}(${x()})`;
    case 9: return `${str(d - 1)}.${pick(["includes", "startsWith", "endsWith"])}(${str(d - 1)}${chance(0.3) ? ", " + num(d - 1) : ""})`;
    case 10: return `(${keyExpr()} in ${obj(d - 1)})`;
    default: return `${pick(["isNaN", "isFinite", "Boolean"])}(${x()})`;
  }
}

function arr(d) {
  if (d <= 0 || chance(0.2)) {
    if (chance(0.2) && VARS.arr.length) return pick(VARS.arr);
    return `[${times(int(6), () => (chance(0.6) ? numLit() : chance(0.5) ? strLit() : pick(["null", "undefined", "true", "false", "[1, [2]]", "{}"]))).join(", ")}]`;
  }
  const k = int(30);
  const cmp = () => pick(["(a, b) => a - b", "(a, b) => b - a", "(a, b) => (a > b ? 1 : a < b ? -1 : 0)", "(a, b) => a > b", "(a, b) => String(a).length - String(b).length",
    "(a, b) => 0", "() => -1", "(a, b) => (typeof a === typeof b ? 0 : typeof a < typeof b ? -1 : 1)", "(a, b) => (Number(a) || 0) - (Number(b) || 0)", "(a, b) => ((a * 7 + b * 13) % 5) - 2"]);
  const n = () => (chance(0.4) ? pick(["0", "1", "2", "-1", "-3", "10", "undefined"]) : num(d - 1));
  switch (k) {
    case 0: return `${arr(d - 1)}.map((v, i) => ${pick(["v", "i", "v + i", "typeof v", "[v]", "v * 2", "String(v)"])})`;
    case 1: return `${arr(d - 1)}.filter((v, i) => ${bool(d - 1, "v")})`;
    case 2: return `${fresh(arr(d - 1))}.sort(${chance(0.75) ? cmp() : ""})`;
    case 3: return `${arr(d - 1)}.slice(${n()}${chance(0.5) ? ", " + n() : ""})`;
    case 4: return `${arr(d - 1)}.concat(${any(d - 1)}, ${any(d - 1)})`;
    case 5: return `${fresh(arr(d - 1))}.reverse()`;
    case 6: return `${arr(d - 1)}.flat(${pick(["", "0", "1", "2", "Infinity"])})`;
    case 7: return `${arr(d - 1)}.flatMap(v => [v, [v]])`;
    case 8: return `[...${pick([arr(d - 1), str(d - 1)])}]`;
    case 9: return `Object.${pick(["keys", "values"])}(${obj(d - 1)})`;
    case 10: return `Object.entries(${obj(d - 1)})`;
    case 11: return `${str(d - 1)}.split(${pick([strLit(), '""', '","'])})`;
    case 12: return `Array.from(${pick([str(d - 1), arr(d - 1), "{length: " + int(4) + "}"])}${chance(0.5) ? ", (v, i) => [v, i]" : ""})`;
    case 13: return `((a) => { const r = a.splice(${n()}${chance(0.6) ? ", " + n() : ""}${chance(0.4) ? ", " + any(d - 1) : ""}); return [a, r]; })(${fresh(arr(d - 1))})`;
    case 14: return `((a) => { const r = [a.${pick(["push", "unshift"])}(${any(d - 1)}), a.${pick(["pop", "shift"])}()]; return [a, r]; })(${fresh(arr(d - 1))})`;
    case 15: return `${fresh(arr(d - 1))}.fill(${any(d - 1)}${chance(0.5) ? ", " + n() : ""}${chance(0.3) ? ", " + n() : ""})`;
    case 16: return `[${arr(d - 1)}.at(${n()}), ${arr(d - 1)}.findLast(v => ${bool(d - 1, "v")}), ${arr(d - 1)}.find(v => ${bool(d - 1, "v")})]`;
    case 17: return `${arr(d - 1)}.toSorted(${chance(0.5) ? cmp() : ""})`;
    case 18: return `[${arr(d - 1)}.some(v => ${bool(d - 1, "v")}), ${arr(d - 1)}.every(v => ${bool(d - 1, "v")})]`;
    case 19: return `${arr(d - 1)}.reduceRight((acc, v) => [v, acc], ${pick(["[]", "0", '"x"'])})`;
    case 20: return `[...${arr(d - 1)}.entries()]`;
    case 21: return `${arr(d - 1)}.lastIndexOf(${any(d - 1)}${chance(0.4) ? ", " + n() : ""})`;
    case 22: return `[${arr(d - 1)}.includes(${any(d - 1)}, ${n()}), ${arr(d - 1)}.indexOf(${any(d - 1)}, ${n()})]`;
    case 23: return `${arr(d - 1)}.with(${pick(["0", "-1", "1"])}, ${any(d - 1)})`;
    case 24: return `JSON.parse(JSON.stringify(${arr(d - 1)}))`;
    case 25: {
      const len = 10 + int(90);
      return `Array.from({length: ${len}}, (_, i) => (i * ${1 + int(97)}) % ${1 + int(40)}).sort(${cmp()})`;
    }
    case 26: return `${arr(d - 1)}.map(${pick(["Number", "String", "Boolean", "parseFloat", "parseInt", "Math.abs", "Array.isArray"])})`;
    case 27: return `${arr(d - 1)}.filter(Boolean)`;
    case 28: return `${arr(d - 1)}.toReversed()`;
    default: return `[${times(int(4), () => any(d - 1)).join(", ")}]`;
  }
}

function obj(d) {
  if (d <= 0 || chance(0.25)) {
    if (chance(0.2) && VARS.obj.length) return pick(VARS.obj);
    return `{${times(int(5), () => `${keyLit()}: ${chance(0.7) ? numLit() : strLit()}`).join(", ")}}`;
  }
  const k = int(10);
  switch (k) {
    case 0: return `{...${obj(d - 1)}, ${keyLit()}: ${any(d - 1)}}`;
    case 1: return `{...${pick([arr(d - 1), str(d - 1), "null", "undefined", "5"])}}`;
    case 2: return `Object.fromEntries(${arr(d - 1)}.map((v, i) => [${pick(["v", "i", "String(v)", "i % 3"])}, i]))`;
    case 3: return `Object.assign({}, ${obj(d - 1)}, ${obj(d - 1)})`;
    case 4: return `((q) => { q[${keyExpr()}] = ${any(d - 1)}; return q; })(${freshObj(obj(d - 1))})`;
    case 5: return `((q) => { delete q.${pick(IDENT_KEYS)}; return q; })(${freshObj(obj(d - 1))})`;
    case 6: return `JSON.parse(JSON.stringify(${obj(d - 1)}))`;
    case 7: return `{${keyLit()}: ${obj(d - 1)}, ${keyLit()}: [${any(d - 1)}]}`;
    case 8: return `((q) => { const {${pick(IDENT_KEYS)}: first, ...rest} = q; return {first, rest}; })(${obj(d - 1)})`;
    default: return `JSON.parse(${pick(["'{\"b\":1,\"2\":2,\"a\":[1,2.5,-0],\"0\":null}'", "'{\"a\":{\"b\":{}}, \"c\":\"\\\\n\"}'", "'{\"x\": 1e2, \"y\": -0.0, \"z\": [0.1, \"x\\\\u0041\"]}'"])})`;
  }
}

function any(d) {
  const k = int(10);
  if (k < 3) return num(d);
  if (k < 5) return str(d);
  if (k < 7) return arr(d);
  if (k < 8) return obj(d);
  if (k < 9) return bool(d);
  return pick(["null", "undefined", "[]", "{}", "[0]", "[1, 2]", '""', '"0"', "true", "false", "NaN", "-0", '"1e3"', "[null]", "[[]]"]);
}

// ---------------- statements ----------------

function stmtProgram(d) {
  const k = int(19);
  switch (k) {
    case 0: return `let acc = ${pick(["0", '""', "[]"])};\nfor (let i = ${int(3)}; i < ${2 + int(6)}; i${pick(["++", " += 2"])}) {\n  if (i % ${2 + int(3)} === 0) continue;\n  acc = acc + ${pick(["i", "String(i)", "[i]", "i * i"])};\n  if (acc.length > 20) break;\n}\nreturn acc;`;
    case 1: return `const fs = [];\nfor (let i = 0; i < ${1 + int(4)}; i++) fs.push(() => i * ${int(5)});\nlet x = 0;\nconst g = () => x;\nx = ${num(1)};\nreturn [fs.map(f => f()), g()];`;
    case 2: return `let total = 0;\nconst xs = ${arr(d)};\nxs.forEach((v, i) => { total += (typeof v === "number" ? v : i); });\nreturn total;`;
    case 3: return `const make = () => { let c = ${int(5)}; return { inc: () => ++c, dec: () => c--, get: () => c }; };\nconst a = make(), b = make();\na.inc(); a.inc(); b.dec();\nreturn [a.get(), b.get(), a.inc(), b.dec()];`;
    case 4: return `const out = [];\nouter: for (let i = 0; i < ${2 + int(3)}; i++) {\n  for (let j = 0; j < ${2 + int(3)}; j++) {\n    if (j === ${int(3)}) continue outer;\n    if (i * j > ${int(6)}) break outer;\n    out.push(i * 10 + j);\n  }\n}\nreturn out;`;
    case 5: return `const log = [];\nconst f = (v) => {\n  try {\n    log.push("t");\n    if (${bool(1, "v")}) throw new ${pick(["Error", "TypeError", "RangeError"])}("bad " + v);\n    return "ok";\n  } catch (e) {\n    log.push(e.name + ":" + e.message);\n    return "caught";\n  } finally {\n    log.push("f");\n  }\n};\nreturn [f(${any(1)}), f(${any(1)}), log];`;
    case 6: return `const f = (x) => {\n  switch (x) {\n    case ${numLit()}: return "a";\n    case ${strLit()}: return "b";\n    case ${pick(["null", "undefined", "true"])}:\n    case ${numLit()}: return "c";\n    default: return "d";\n  }\n};\nreturn [${times(4, () => "f(" + any(1) + ")").join(", ")}];`;
    case 7: return `const {${pick(IDENT_KEYS)}: a = ${numLit()}, ${pick(IDENT_KEYS)}: b, ...rest} = ${obj(d)};\nconst [p, , q = ${strLit()}, ...more] = ${arr(d)};\nreturn [a, b, rest, p, q, more];`;
    case 8: return `const o = ${obj(d)};\nreturn [o?.${pick(IDENT_KEYS)}?.${pick(IDENT_KEYS)}, o.${pick(IDENT_KEYS)} ?? ${numLit()}, o?.[${keyExpr()}], o.nope?.(), o.nope?.x.y];`;
    case 9: return `let xs = ${arr(d)};\nxs.${pick(["push", "unshift"])}(${any(1)}, ${any(1)});\nconst popped = xs.${pick(["pop", "shift"])}();\nxs.sort(${pick(["", "(a, b) => String(a) < String(b) ? -1 : 1"])});\nxs.reverse();\nreturn [xs, popped, xs.length];`;
    case 10: return `const o = ${obj(d)};\no.${pick(IDENT_KEYS)} = ${any(1)};\no[${keyExpr()}] = ${any(1)};\ndelete o.${pick(IDENT_KEYS)};\nconst keys = [];\nfor (const k in o) keys.push(k);\nreturn [keys, Object.entries(o), JSON.stringify(o)];`;
    case 11: return `let n = ${int(20)}, steps = 0;\ndo { n = n % 2 ? 3 * n + 1 : n / 2; steps++; } while (n > 1 && steps < 50);\nlet m = 1;\nwhile (m < ${10 + int(100)}) m *= ${2 + int(3)};\nreturn [n, steps, m];`;
    case 12: return `function fib(k) { return k < 2 ? k : fib(k - 1) + fib(k - 2); }\nconst memo = {};\nconst f = (k) => (k in memo ? memo[k] : (memo[k] = k < 2 ? k : f(k - 1) + f(k - 2)));\nreturn [fib(${int(15)}), f(${int(40)}), Object.keys(memo).length];`;
    case 13: return `let s = ${str(d)};\nlet t = "";\nfor (const ch of s) t = ch + t;\nconst counts = {};\nfor (const ch of s) counts[ch] = (counts[ch] || 0) + 1;\nreturn [t, counts];`;
    case 14: {
      // the comparator's calls, in order (V8's TimSort)
      const len = pick([5, 20, 40, 63, 64, 65, 100, 130, 300]);
      const cmp = pick(["a - b", "b - a", "(a % 7) - (b % 7)", "a > b ? 1 : -1", "(a * 31 + b * 17) % 3 - 1", "Number(a > b)", "String(a).localeCompare(String(b))"]);
      return `const calls = [];\nconst xs = Array.from({length: ${len}}, (_, i) => (i * ${1 + int(97)} + ${int(13)}) % ${1 + int(len)});\nconst sorted = xs.toSorted((a, b) => { calls.push(a * 1000 + b); return ${cmp}; });\nreturn [sorted, calls.length, calls.slice(0, 40), calls.slice(-20)];`;
    }
    case 15: return `const o = {};\nconst keys = ${arr(1)}.map(String);\nlet i = 0;\nfor (const k of keys) { o[k] ??= []; o[k].push(i++); }\nconst grid = [[1, 2], [3, 4]];\ngrid[${int(2)}][${int(2)}] += ${numLit()};\nlet a, b, c;\na = b = c = ${numLit()};\nconst q = {n: {m: [${numLit()}]}};\nq.n.m[0] *= 2; q.n.k ||= "set"; q.n.m.push(q.n.m.length);\nreturn [o, i, grid, a, b, c, q];`;
    case 17: return `const a = ${arr(d)};\nconst b = a;\nb.${pick(["push", "unshift"])}(${any(1)});\nconst o = ${obj(d)};\nconst q = {inner: o, list: a};\nq.inner.${pick(IDENT_KEYS)} = ${any(1)};\nq.list.${pick(["pop", "shift", "reverse", "sort"])}();\nconst grow = (z, v) => { z.push(v); return z.length; };\nconst n1 = grow(a, ${numLit()});\nconst seen = [];\nfor (const x of a) { if (seen.length < 8) { seen.push(x); if (seen.length === 1) a.push("late"); } }\nreturn [a, b, a === b, o, q.inner === o, n1, seen, a.length];`;
    case 16: return `const f = (a, b = a + 1, ...rest) => [a, b, rest.length, rest];\nconst g = ({x = 1, y: {z = 2} = {}} = {}, [p, q = p] = [${numLit()}]) => [x, z, p, q];\nreturn [f(${numLit()}), f(...${arr(1)}), g(), g({x: ${numLit()}, y: {}}), g(undefined, [${numLit()}, ${numLit()}])];`;
    default: {
      const xs = arr(d);
      return `const xs = ${xs};\nconst byType = {};\nfor (const [i, v] of xs.entries()) { const t = typeof v; (byType[t] ??= []).push(i); }\nconst sorted = Object.entries(byType).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0));\nlet out = "";\nfor (const [t, is] of sorted) { if (!is.length) continue; out += \`\${t}=\${is.join("+")};\`; }\nreturn [out, sorted.length, xs.length];`;
    }
  }
}

function exprProgram(d) {
  const n = 2 + int(5);
  // The variables' initializers do not read the variables (that is a TDZ ReferenceError).
  const saved = { ...VARS };
  for (const k of Object.keys(VARS)) VARS[k] = [];
  const lines = [
    `const n = ${numLit()}, m = ${numLit()};`,
    `const s = ${strLit()}, t = ${strLit()};`,
    `const xs = ${arr(1)}, ys = ${arr(1)};`,
    `const o = ${obj(1)}, p = ${obj(1)};`,
    `const r = [];`,
  ];
  Object.assign(VARS, saved);
  for (let i = 0; i < n; i++) lines.push(`try { r.push(${any(d)}); } catch (e) { r.push("E:" + e.name); }`);
  lines.push("return r;");
  return lines.join("\n");
}

// A separate grammar crosses native-function identity, receiver kind and invocation form.
function borrowedProgram() {
  const values=times(int(6),()=>pick([numLit(),strLit(),"null","undefined"]));
  const object=chance(0.6);
  const receiver=object?`{${values.map((v,i)=>`${i}:${v}`).join(",")}${values.length?",":""}length:${values.length}}`:`[${values.join(",")}]`;
  const method=pick(["join","slice","map","filter","reduce","find","includes","indexOf","push","pop","shift","unshift","splice","reverse","fill","sort","at","toSorted","toReversed","with"]);
  const args=({join:[strLit()],slice:[String(int(5)-2)],map:["(v,i)=>String(v)+i"],filter:["Boolean"],reduce:["(a,v)=>a+String(v)", '""'],find:["v=>v!==null"],includes:[pick([numLit(),strLit(),"null","undefined","true"])],indexOf:[pick([numLit(),strLit(),"null","undefined","true"])],push:[pick([numLit(),strLit(),"null","undefined","true"])],unshift:[pick([numLit(),strLit(),"null","undefined","true"])],splice:[String(int(4)),String(int(3)),pick([numLit(),strLit(),"null","undefined","true"])],fill:[numLit(),"1"],sort:["(a,b)=>String(a)<String(b)?-1:String(a)>String(b)?1:0"],at:[String(int(7)-3)],with:[String(int(7)-3),pick([numLit(),strLit(),"null","undefined","true"])]})[method]||[];
  const call=pick([`f.call(r${args.length?","+args.join(","):""})`,`f.apply(r,[${args.join(",")}])`,`f.bind(r${args.length?","+args.join(","):""})()`]);
  return `const r=${receiver};const f=[][${JSON.stringify(method)}];const result=${call};return [result,r];`;
}

const out = [];
for (let i = 0; i < count; i++) {
  R = rng(seed * 1000003 + i);
  let body;
  if (chance(0.2)) {
    body=borrowedProgram();
  } else if (chance(0.3)) {
    // The statement programs declare their own names (an undeclared name reads as undefined in
    // views: a documented difference).
    const saved = { ...VARS };
    for (const k of Object.keys(VARS)) VARS[k] = [];
    body = stmtProgram(2);
    Object.assign(VARS, saved);
  } else {
    body = exprProgram(1 + int(3));
  }
  out.push({ name: `gen-${seed}-${i}`, body });
}
process.stdout.write(JSON.stringify(out));
