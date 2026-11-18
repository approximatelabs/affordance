// What a Jinja chat template means, for views converted from one (tools/jinja2tsx.py): Python's
// truthiness, text, equality and string methods, and Jinja's filters, tests and loop object.
// `undefined` is Jinja's Undefined, `null` is None. A failure Jinja would raise as an
// UndefinedError or TypeError is a TypeError here; `raise_exception` throws an Error.

const fail = msg => { throw new TypeError(msg); };
const isStr = v => typeof v === "string";
const isNum = v => typeof v === "number";
const isBool = v => typeof v === "boolean";
const isList = v => Array.isArray(v);
const isMap = v => isPlainObject(v);
const isFn = v => typeof v === "function";

// Keyword arguments travel as a last argument marked this way.
export const kw = o => ({__kw: true, ...o});
const isKw = v => isMap(v) && v.__kw === true;
const split = args => args.length && isKw(args[args.length - 1])
  ? [args.slice(0, -1), args[args.length - 1]] : [args, {}];

// Python truthiness.
export function t(v) {
  if (v === undefined || v === null || v === false || v === 0 || v === "") return false;
  if (isList(v)) return v.length > 0;
  if (isMap(v)) return Object.keys(v).length > 0;
  if (v instanceof Map) return v.size > 0;
  return true;
}
// What `{{ v }}` prints: Python's str(v); Undefined prints nothing.
export function str(v) {
  if (v === undefined) return "";
  if (isStr(v)) return v;
  if (isFn(v)) return "<bound method>";
  return formatPythonValue(v);
}
// str(v) where Undefined is an error (`~`, string methods).
const text = v => v === undefined ? fail("UndefinedError: value is undefined") : str(v);
const def = v => v === undefined ? fail("UndefinedError: value is undefined") : v;

// Python equality.
export function eq(a, b) {
  if (a === b) return true;
  if (a === undefined || b === undefined || a === null || b === null) return false;
  if (isBool(a)) a = a ? 1 : 0;
  if (isBool(b)) b = b ? 1 : 0;
  if (isNum(a) && isNum(b)) return a === b;
  if (isList(a) && isList(b)) return a.length === b.length && a.every((x, i) => eq(x, b[i]));
  if (isMap(a) && isMap(b)) {
    const ka = Object.keys(a), kb = Object.keys(b);
    return ka.length === kb.length && ka.every(k => k in b && eq(a[k], b[k]));
  }
  return false;
}
function order(a, b) {
  def(a); def(b);
  if (isBool(a)) a = a ? 1 : 0;
  if (isBool(b)) b = b ? 1 : 0;
  if (isNum(a) && isNum(b)) return a < b ? -1 : a > b ? 1 : 0;
  if (isStr(a) && isStr(b)) return a < b ? -1 : a > b ? 1 : 0;
  if (isList(a) && isList(b)) {
    for (let i = 0; i < Math.min(a.length, b.length); i++) { const c = order(a[i], b[i]); if (c) return c; }
    return a.length - b.length;
  }
  return fail("TypeError: '<' not supported between these values");
}
export const lt = (a, b) => order(a, b) < 0;
export const le = (a, b) => order(a, b) <= 0;
export const gt = (a, b) => order(a, b) > 0;
export const ge = (a, b) => order(a, b) >= 0;
// `a in b`.
export function has(a, b) {
  if (isStr(b)) return isStr(a) ? b.includes(a) : fail("TypeError: 'in <string>' requires string as left operand");
  if (isList(b)) return b.some(x => eq(x, a));
  if (isMap(b)) return isStr(a) ? a in b : false;
  if (b instanceof Map) return [...b.keys()].some(k => eq(k, a));
  if (b === undefined) return false;
  return fail("TypeError: argument is not iterable");
}

export function add(a, b) {
  def(a); def(b);
  if (isStr(a) && isStr(b)) return a + b;
  if (isList(a) && isList(b)) return [...a, ...b];
  if ((isNum(a) || isBool(a)) && (isNum(b) || isBool(b))) return Number(a) + Number(b);
  return fail("TypeError: unsupported operand type(s) for +");
}
const num = v => (isNum(v) || isBool(v)) ? Number(v) : fail("TypeError: unsupported operand type(s)");
export const sub = (a, b) => num(def(a)) - num(def(b));
export function mul(a, b) {
  def(a); def(b);
  if (isStr(a) && isNum(b)) return a.repeat(Math.max(0, b));
  if (isStr(b) && isNum(a)) return b.repeat(Math.max(0, a));
  if (isList(a) && isNum(b)) return Array.from({length: Math.max(0, b)}, () => a).flat();
  return num(a) * num(b);
}
export const div = (a, b) => num(def(a)) / num(def(b));
export const floordiv = (a, b) => Math.floor(num(def(a)) / num(def(b)));
export function mod(a, b) {
  def(a);
  if (isStr(a)) return format(a, isList(b) ? b : [b]);
  const x = num(a), y = num(def(b));
  return ((x % y) + y) % y;
}
export const pow = (a, b) => num(def(a)) ** num(def(b));
export const neg = a => -num(def(a));
// `a ~ b` joins the text of each part; Undefined is empty text there.
export const concat = (...parts) => parts.map(str).join("");

// Python's %-formatting: %s %r %d %i %f %x %%, with the flags `0`, `-`, `+`, a width and a precision.
function format(pattern, args) {
  let i = 0;
  return pattern.replace(/%([-+0 ]*)(\d+)?(?:\.(\d+))?([sdirfxX%])/g, (m, flags, width, precision, c) => {
    if (c === "%") return "%";
    const v = args[i++];
    let body = c === "s" ? str(v) : c === "r" ? formatPythonRepr(v)
      : c === "f" ? Math.abs(num(v)).toFixed(precision === undefined ? 6 : Number(precision))
      : c === "x" ? Math.abs(Math.trunc(num(v))).toString(16) : c === "X" ? Math.abs(Math.trunc(num(v))).toString(16).toUpperCase()
      : String(Math.abs(Math.trunc(num(v))));
    if (c === "s" && precision !== undefined) body = body.slice(0, Number(precision));
    const sign = c === "s" || c === "r" ? "" : num(v) < 0 ? "-" : flags.includes("+") ? "+" : flags.includes(" ") ? " " : "";
    const pad = Math.max(0, Number(width || 0) - body.length - sign.length);
    if (flags.includes("-")) return sign + body + " ".repeat(pad);
    if (flags.includes("0") && c !== "s" && c !== "r") return sign + "0".repeat(pad) + body;
    return " ".repeat(pad) + sign + body;
  });
}

// Iteration: a list's items, a dict's keys, a string's characters; Undefined is empty.
export function iter(v) {
  if (v === undefined) return [];
  if (isList(v)) return v;
  if (isStr(v)) return [...v];
  if (isMap(v)) return Object.keys(v);
  if (v instanceof Map) return [...v.keys()];
  return fail("TypeError: object is not iterable");
}
export function len(v) {
  if (v === undefined) return 0;
  if (isStr(v)) return [...v].length;
  if (isList(v)) return v.length;
  if (isMap(v)) return Object.keys(v).length;
  if (v instanceof Map) return v.size;
  return fail("TypeError: object has no len()");
}
const idx = (i, n) => i < 0 ? i + n : i;
// `v[a:b:c]`.
export function slice(v, a, b, c) {
  def(v);
  const items = isStr(v) ? [...v] : isList(v) ? v : fail("TypeError: object is not subscriptable");
  const n = items.length, step = c === undefined || c === null ? 1 : c;
  let out = [];
  if (step > 0) {
    let lo = a === undefined || a === null ? 0 : Math.min(Math.max(idx(a, n), 0), n);
    const hi = b === undefined || b === null ? n : Math.min(Math.max(idx(b, n), 0), n);
    for (let i = lo; i < hi; i += step) out.push(items[i]);
  } else {
    let lo = a === undefined || a === null ? n - 1 : Math.min(Math.max(idx(a, n), -1), n - 1);
    const hi = b === undefined || b === null ? -1 : Math.min(Math.max(idx(b, n), -1), n - 1);
    for (let i = lo; i > hi; i += step) out.push(items[i]);
  }
  return isStr(v) ? out.join("") : out;
}

// ---- string, dict and list methods, as bound functions ----
// What Python's str.isspace() accepts: strip() and split() with no argument use it.
const WS = " \t\n\r\x0b\x0c\x1c\x1d\x1e\x1f\x85\xa0\u1680\u2000\u2001\u2002\u2003\u2004\u2005\u2006\u2007\u2008\u2009\u200a\u2028\u2029\u202f\u205f\u3000";
const strip = (s, chars, left, right) => {
  const set = chars === undefined || chars === null ? WS : chars;
  let a = 0, b = s.length;
  if (left) while (a < b && set.includes(s[a])) a++;
  if (right) while (b > a && set.includes(s[b - 1])) b--;
  return s.slice(a, b);
};
function pySplit(s, sep, max) {
  if (max === undefined || max === null) max = -1;
  if (sep === undefined || sep === null) {
    const parts = []; let i = 0;
    while (true) {
      while (i < s.length && WS.includes(s[i])) i++;
      if (i >= s.length) break;
      if (max >= 0 && parts.length >= max) { parts.push(strip(s.slice(i), undefined, false, true) === "" ? "" : s.slice(i)); return parts.filter((p, k) => p !== "" || k < parts.length - 1); }
      let j = i;
      while (j < s.length && !WS.includes(s[j])) j++;
      parts.push(s.slice(i, j)); i = j;
    }
    return parts;
  }
  if (sep === "") fail("ValueError: empty separator");
  const parts = []; let i = 0;
  while (max < 0 || parts.length < max) {
    const j = s.indexOf(sep, i);
    if (j < 0) break;
    parts.push(s.slice(i, j)); i = j + sep.length;
  }
  parts.push(s.slice(i));
  return parts;
}
function pyRsplit(s, sep, max) {
  if (max === undefined || max === null || max < 0) return pySplit(s, sep, -1);
  if (sep === undefined || sep === null) fail("TypeError: rsplit without a separator and with a limit is not supported");
  const parts = []; let end = s.length;
  while (parts.length < max) {
    const j = s.lastIndexOf(sep, end - sep.length);
    if (j < 0 || end - sep.length < 0) break;
    parts.unshift(s.slice(j + sep.length, end)); end = j;
  }
  parts.unshift(s.slice(0, end));
  return parts;
}
const starts = (s, p, f) => isList(p) ? p.some(x => f(s, x)) : f(s, p);
const title = s => s.replace(/[A-Za-z]+/g, w => w[0].toUpperCase() + w.slice(1).toLowerCase());
// A dict written with keys that are not strings (`{0: "a", 512: "b"}`).
const MAP = {
  get: m => (k, d) => { for (const [key, v] of m) if (eq(key, k)) return v; return d === undefined ? null : d; },
  items: m => () => [...m.entries()], keys: m => () => [...m.keys()], values: m => () => [...m.values()],
};
export const pairs = list => new Map(list);
// The template sandbox refuses calls that change a list or dict.
const mutation = name => () => fail("SecurityError: the template may not call '" + name + "'");
const STR = {
  strip: s => c => strip(s, c, true, true), lstrip: s => c => strip(s, c, true, false), rstrip: s => c => strip(s, c, false, true),
  split: s => (...a) => { const [p, k] = split(a); return pySplit(s, p[0] ?? k.sep, p[1] ?? k.maxsplit); },
  rsplit: s => (...a) => { const [p, k] = split(a); return pyRsplit(s, p[0] ?? k.sep, p[1] ?? k.maxsplit); },
  splitlines: s => () => s === "" ? [] : s.replace(/\r\n?/g, "\n").replace(/\n$/, "").split("\n"),
  startswith: s => p => starts(s, p, (x, y) => x.startsWith(y)), endswith: s => p => starts(s, p, (x, y) => x.endsWith(y)),
  replace: s => (a, b, n) => { if (n === undefined || n < 0) return s.split(a).join(b); let out = s, from = 0; for (let i = 0; i < n; i++) { const j = out.indexOf(a, from); if (j < 0) break; out = out.slice(0, j) + b + out.slice(j + a.length); from = j + b.length; } return out; },
  upper: s => () => s.toUpperCase(), lower: s => () => s.toLowerCase(), title: s => () => title(s),
  capitalize: s => () => s === "" ? s : s[0].toUpperCase() + s.slice(1).toLowerCase(),
  join: s => xs => iter(xs).map(x => isStr(x) ? x : fail("TypeError: sequence item: expected str")).join(s),
  find: s => x => s.indexOf(x), rfind: s => x => s.lastIndexOf(x),
  index: s => x => s.indexOf(x) < 0 ? fail("ValueError: substring not found") : s.indexOf(x),
  count: s => x => x === "" ? s.length + 1 : s.split(x).length - 1,
  isdigit: s => () => /^\d+$/.test(s), isalpha: s => () => /^[A-Za-z]+$/.test(s), isalnum: s => () => /^[A-Za-z0-9]+$/.test(s),
  isspace: s => () => /^\s+$/.test(s), isupper: s => () => /[A-Z]/.test(s) && s === s.toUpperCase(), islower: s => () => /[a-z]/.test(s) && s === s.toLowerCase(),
  format: s => (...a) => { const [p, k] = split(a); let i = 0; return s.replace(/\{(\w*)\}/g, (m, n) => str(n === "" ? p[i++] : /^\d+$/.test(n) ? p[Number(n)] : k[n])); },
  removeprefix: s => p => s.startsWith(p) ? s.slice(p.length) : s, removesuffix: s => p => p && s.endsWith(p) ? s.slice(0, -p.length) : s,
  partition: s => sep => { const j = s.indexOf(sep); return j < 0 ? [s, "", ""] : [s.slice(0, j), sep, s.slice(j + sep.length)]; },
  zfill: s => n => s.padStart(n, "0"), encode: s => () => s,
};
const DICT = {
  get: d => (k, v) => isStr(k) && k in d ? d[k] : v === undefined ? null : v,
  items: d => () => Object.entries(d), keys: d => () => Object.keys(d), values: d => () => Object.values(d),
  copy: d => () => ({...d}),
  update: () => mutation("update"), pop: () => mutation("pop"), popitem: () => mutation("popitem"), setdefault: () => mutation("setdefault"), clear: () => mutation("clear"),
};
const LIST = {
  index: xs => x => { const i = xs.findIndex(y => eq(y, x)); return i < 0 ? fail("ValueError: value is not in list") : i; },
  count: xs => x => xs.filter(y => eq(y, x)).length, copy: xs => () => [...xs],
  append: () => mutation("append"), extend: () => mutation("extend"), pop: () => mutation("pop"), insert: () => mutation("insert"),
  remove: () => mutation("remove"), reverse: () => mutation("reverse"), sort: () => mutation("sort"), clear: () => mutation("clear"),
};

// `o.name` (Jinja tries an attribute, then an item).
export function attr(o, name) {
  if (o === undefined) return fail("UndefinedError: value is undefined and has no attribute '" + name + "'");
  if (o === null) return undefined;
  if (isStr(o)) return name in STR ? STR[name](o) : undefined;
  if (isList(o)) return name in LIST ? LIST[name](o) : undefined;
  if (o instanceof Loop || o instanceof Namespace || o instanceof Cycler || o instanceof Joiner) return o[name];
  if (isMap(o)) return name in DICT ? DICT[name](o) : o[name];
  if (o instanceof Map) return name in MAP ? MAP[name](o) : undefined;
  return undefined;
}
// `o[key]` (Jinja tries an item, then an attribute).
export function item(o, key) {
  if (o === undefined) return fail("UndefinedError: value is undefined and has no element");
  if (o === null) return undefined;
  if (isList(o) || isStr(o)) {
    if (isNum(key)) { const items = isStr(o) ? [...o] : o; return items[idx(key, items.length)]; }
    return isStr(key) ? attr(o, key) : undefined;
  }
  if (o instanceof Loop || o instanceof Namespace) return o[key];
  if (isMap(o)) return isStr(key) && key in o ? o[key] : isStr(key) && key in DICT ? DICT[key](o) : undefined;
  if (o instanceof Map) { for (const [k, v] of o) if (eq(k, key)) return v; return undefined; }
  return undefined;
}
// A call: `f(args, kw({..}))`.
export function call(f, ...args) {
  if (!isFn(f)) return fail(f === undefined ? "UndefinedError: value is undefined and cannot be called" : "TypeError: object is not callable");
  return f(...args);
}

// ---- namespace, loop, cycler, joiner ----
export class Namespace { constructor(o) { for (const k of Object.keys(o)) if (k !== "__kw") this[k] = o[k]; } }
export const namespace = (...a) => { const [p, k] = split(a); return new Namespace({...(p[0] || {}), ...k}); };
export function setattr(ns, name, value) {
  if (!(ns instanceof Namespace)) fail("TemplateRuntimeError: cannot assign attribute on a non-namespace object");
  ns[name] = value;
}
export class Loop {
  constructor(items, depth0) { this.items = items; this.length = items.length; this.index0 = 0; this.depth0 = depth0 || 0; this.last_changed = undefined; this.has_changed = false; }
  get index() { return this.index0 + 1; }
  get first() { return this.index0 === 0; }
  get last() { return this.index0 === this.length - 1; }
  get revindex() { return this.length - this.index0; }
  get revindex0() { return this.length - this.index0 - 1; }
  get depth() { return this.depth0 + 1; }
  get previtem() { return this.index0 > 0 ? this.items[this.index0 - 1] : undefined; }
  get nextitem() { return this.index0 < this.length - 1 ? this.items[this.index0 + 1] : undefined; }
  get cycle() { return (...xs) => xs[this.index0 % xs.length]; }
  get changed() { return (...xs) => { const same = this.has_changed && eq(this.last_changed, xs); this.has_changed = true; this.last_changed = xs; return !same; }; }
}
export class Cycler {
  constructor(items) { this.items = items; this.pos = 0; }
  get current() { return this.items[this.pos]; }
  get next() { return () => { const v = this.items[this.pos]; this.pos = (this.pos + 1) % this.items.length; return v; }; }
  get reset() { return () => { this.pos = 0; }; }
}
export const cycler = (...xs) => new Cycler(xs);
export class Joiner { constructor(sep) { this.sep = sep; this.used = false; } }
export const joiner = (sep = ", ") => { const j = new Joiner(sep); return () => { if (!j.used) { j.used = true; return ""; } return j.sep; }; };
export function range(a, b, c) {
  if (b === undefined) { b = a; a = 0; }
  const step = c === undefined ? 1 : c, out = [];
  if (step === 0) fail("ValueError: range() arg 3 must not be zero");
  for (let i = a; step > 0 ? i < b : i > b; i += step) out.push(i);
  return out;
}
export const dict = (...a) => { const [p, k] = split(a); const o = {...(p[0] || {}), ...k}; delete o.__kw; return o; };
export function raise_exception(message) { throw new Error(message); }
export const strftime_now = fmt => formatNow(fmt);

// ---- filters ----
const getter = name => { const path = String(name).split("."); return v => path.reduce((o, k) => o === undefined || o === null ? undefined : (/^\d+$/.test(k) ? item(o, Number(k)) : item(o, k)), v); };
const TESTS = {
  defined: v => v !== undefined, undefined: v => v === undefined, none: v => v === null,
  string: v => isStr(v), mapping: v => isMap(v), number: v => isNum(v) || isBool(v),
  integer: v => isNum(v) && Number.isInteger(v) && formatPythonValue(v).indexOf(".") < 0, float: v => isNum(v) && formatPythonValue(v).indexOf(".") >= 0,
  boolean: v => isBool(v), true: v => v === true, false: v => v === false,
  iterable: v => isStr(v) || isList(v) || isMap(v) || v === undefined, sequence: v => isStr(v) || isList(v) || isMap(v),
  callable: v => isFn(v), sameas: (v, o) => v === o,
  equalto: (v, o) => eq(v, o), eq: (v, o) => eq(v, o), "==": (v, o) => eq(v, o), ne: (v, o) => !eq(v, o), "!=": (v, o) => !eq(v, o),
  lt: (v, o) => lt(v, o), lessthan: (v, o) => lt(v, o), "<": (v, o) => lt(v, o), le: (v, o) => le(v, o), "<=": (v, o) => le(v, o),
  gt: (v, o) => gt(v, o), greaterthan: (v, o) => gt(v, o), ">": (v, o) => gt(v, o), ge: (v, o) => ge(v, o), ">=": (v, o) => ge(v, o),
  in: (v, o) => has(v, o), odd: v => v % 2 === 1, even: v => v % 2 === 0, divisibleby: (v, n) => v % n === 0,
  lower: v => str(v) === str(v).toLowerCase(), upper: v => str(v) === str(v).toUpperCase(),
  truthy: v => t(v), falsy: v => !t(v),
};
export function test(name, v, ...args) {
  if (!(name in TESTS)) fail("TemplateAssertionError: No test named '" + name + "'");
  return TESTS[name](v, ...args);
}
// The test or the truthiness a `select`-like filter applies.
const pred = args => args.length ? v => test(args[0], v, ...args.slice(1)) : v => t(v);
const escape = s => str(s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/'/g, "&#39;").replace(/"/g, "&#34;");
const FILTERS = {
  trim: (v, c) => strip(str(v), c, true, true),
  tojson: (v, ...a) => { const [p, k] = split(a); const indent = p[0] ?? k.indent; def(v); return formatJSON(v, {ascii: k.ensure_ascii === true, ...(indent === undefined || indent === null ? {} : {indent}), ...(k.separators ? {separators: k.separators} : {}), ...(k.sort_keys ? {sortKeys: true} : {})}); },
  length: v => len(v), count: v => len(v),
  string: v => str(v), safe: v => v, e: v => escape(v), escape: v => escape(v), forceescape: v => escape(v),
  upper: v => str(v).toUpperCase(), lower: v => str(v).toLowerCase(), title: v => title(str(v)),
  capitalize: v => STR.capitalize(str(v))(),
  replace: (v, a, b, n) => STR.replace(str(v))(str(a), str(b), n),
  default: (v, ...a) => { const [p, k] = split(a); const d = p.length ? p[0] : "", loose = p[1] ?? k.boolean; return v === undefined || (loose && !t(v)) ? d : v; },
  d: (v, ...a) => FILTERS.default(v, ...a),
  first: v => iter(def(v))[0], last: v => { const xs = iter(def(v)); return xs[xs.length - 1]; },
  list: v => [...iter(v)], reverse: v => isStr(v) ? [...v].reverse().join("") : [...iter(v)].reverse(),
  join: (v, ...a) => { const [p, k] = split(a); const sep = p[0] ?? k.d ?? "", at = p[1] ?? k.attribute; return iter(v).map(x => str(at === undefined ? x : getter(at)(x))).join(sep); },
  int: (v, d = 0) => { if (isBool(v)) return v ? 1 : 0; if (isNum(v)) return Math.trunc(v); const s = str(v).trim(); return /^[+-]?\d+$/.test(s) ? parseInt(s) : /^[+-]?(\d+\.?\d*|\.\d+)([eE][+-]?\d+)?$/.test(s) ? Math.trunc(Number(s)) : d; },
  float: (v, d = 0.0) => { const n = isNum(v) ? v : Number(str(v).trim()); return Number.isNaN(n) ? d : n; },
  abs: v => Math.abs(num(v)), round: (v, n = 0) => { const f = 10 ** n; return Math.round(num(v) * f) / f; },
  sum: (v, ...a) => { const [p, k] = split(a); const at = p[0] ?? k.attribute, start = p[1] ?? k.start ?? 0; return iter(v).reduce((s, x) => add(s, at === undefined ? x : getter(at)(x)), start); },
  min: v => iter(v).reduce((m, x) => m === undefined || lt(x, m) ? x : m, undefined),
  max: v => iter(v).reduce((m, x) => m === undefined || gt(x, m) ? x : m, undefined),
  items: v => v === undefined ? [] : v instanceof Map ? [...v.entries()] : isMap(v) ? Object.entries(v) : fail("TypeError: Can only get item pairs from a mapping."),
  dictsort: v => Object.entries(v).sort((a, b) => order(a[0].toLowerCase(), b[0].toLowerCase())),
  sort: (v, ...a) => { const [p, k] = split(a); const reverse = p[0] ?? k.reverse, at = k.attribute; const key = at === undefined ? x => x : getter(at); const low = x => isStr(x) && !(p[1] ?? k.case_sensitive) ? x.toLowerCase() : x; const xs = [...iter(v)].map((x, i) => [x, i]).sort((x, y) => order(low(key(x[0])), low(key(y[0]))) || x[1] - y[1]).map(x => x[0]); return reverse ? xs.reverse() : xs; },
  unique: v => { const out = []; for (const x of iter(v)) if (!out.some(y => eq(isStr(x) ? x.toLowerCase() : x, isStr(y) ? y.toLowerCase() : y))) out.push(x); return out; },
  map: (v, ...a) => { if (!t(v)) return []; const [p, k] = split(a); if (k.attribute !== undefined) { const g = getter(k.attribute); return iter(v).map(x => { const r = g(x); return r === undefined && "default" in k ? k.default : r; }); } return iter(v).map(x => filter(p[0], x, ...p.slice(1))); },
  // A value that is false (None, an empty string) selects nothing, as in Jinja.
  select: (v, ...a) => t(v) ? iter(v).filter(pred(a)) : [], reject: (v, ...a) => t(v) ? iter(v).filter(x => !pred(a)(x)) : [],
  selectattr: (v, name, ...a) => { if (!t(v)) return []; const g = getter(name), f = pred(a); return iter(v).filter(x => f(g(x))); },
  rejectattr: (v, name, ...a) => { if (!t(v)) return []; const g = getter(name), f = pred(a); return iter(v).filter(x => !f(g(x))); },
  indent: (v, ...a) => { const [p, k] = split(a); const width = p[0] ?? k.width ?? 4, first = p[1] ?? k.first ?? false, blank = p[2] ?? k.blank ?? false; const pad = isStr(width) ? width : " ".repeat(width); const lines = (str(v) + "\n").split("\n"); const body = lines.slice(0, -1); const out = body.map((l, i) => (i === 0 ? first : true) && (blank || l !== "") ? pad + l : l).join("\n"); return out; },
  wordcount: v => (str(v).match(/\w+/g) || []).length,
  truncate: (v, n = 255, kill = false, end = "...") => { const s = str(v); if (s.length <= n + 5) return s; if (kill) return s.slice(0, n - end.length) + end; const cut = s.slice(0, n - end.length); const j = cut.lastIndexOf(" "); return (j < 0 ? cut : cut.slice(0, j)) + end; },
  batch: (v, n, fill) => { const xs = iter(v), out = []; for (let i = 0; i < xs.length; i += n) { const row = xs.slice(i, i + n); while (fill !== undefined && row.length < n) row.push(fill); out.push(row); } return out; },
  format: (v, ...a) => format(str(v), a), striptags: v => str(v).replace(/<[^>]*>/g, "").replace(/\s+/g, " ").trim(),
  attr: (v, name) => attr(v, name), pprint: v => str(v), center: (v, n = 80) => { const s = str(v), pad = Math.max(0, n - s.length); return " ".repeat(Math.floor(pad / 2)) + s + " ".repeat(pad - Math.floor(pad / 2)); },
  urlencode: v => encodeURIComponent(str(v)), xmlattr: v => Object.entries(v).filter(e => e[1] !== null && e[1] !== undefined).map(e => " " + e[0] + '="' + escape(e[1]) + '"').join(""),
};
export function filter(name, v, ...args) {
  if (!(name in FILTERS)) fail("TemplateAssertionError: No filter named '" + name + "'");
  return FILTERS[name](v, ...args);
}

// A call's positional and keyword arguments.
export const args = list => split(list);
// `a, b = value`.
export function unpack(v, n) {
  const items = iter(def(v));
  if (items.length !== n) fail("ValueError: " + (items.length < n ? "not enough" : "too many") + " values to unpack (expected " + n + ")");
  return items;
}

export const loop = items => new Loop(items);
// A value with every dict's keys in Python's sorted order, at any depth (Kimi's code sorts the
// tools this way before its template runs).
export function deep_sort(v) {
  if (isList(v)) return v.map(deep_sort);
  if (!isMap(v)) return v;
  const out = {};
  for (const k of Object.keys(v).sort((a, b) => order(a, b))) out[k] = deep_sort(v[k]);
  return out;
}
// Everything above, as the one name converted views use.
export const J = {
  kw, t, str, eq, lt, le, gt, ge, has, add, sub, mul, div, floordiv, mod, pow, neg, concat, iter, len, slice, attr, item, call,
  namespace, setattr, loop, cycler, joiner, range, dict, raise_exception, strftime_now, test, filter, args, unpack, pairs, deep_sort,
};
