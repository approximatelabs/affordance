// case: for-c-loop
let s = ""; for (let i = 0; i < 5; i++) { if (i === 1) continue; if (i === 4) break; s += i; } return s;
// case: for-c-no-init
let i = 0; let s = 0; for (; i < 3; i++) s += i; return [i, s];
// case: for-c-two-vars
let s = ""; for (let i = 0, j = 10; i < 3; i++, j--) s += i + ":" + j + " "; return s;
// case: for-c-closure-capture
const fs = []; for (let i = 0; i < 3; i++) fs.push(() => i); return fs.map(f => f());
// case: while-loop
let n = 10, steps = 0; while (n !== 1) { n = n % 2 ? 3 * n + 1 : n / 2; steps++; } return steps;
// case: do-while
let i = 0; do { i++; } while (i < 3); return i;
// case: labeled-break
let out = []; outer: for (let i = 0; i < 3; i++) { for (let j = 0; j < 3; j++) { if (j === 2) continue outer; if (i === 2) break outer; out.push(i * 10 + j); } } return out;
// case: for-of-break
let s = 0; for (const x of [1, 2, 3, 4]) { if (x > 2) break; s += x; } return s;
// case: for-of-destructure
let s = ""; for (const [k, v] of Object.entries({a: 1, b: 2})) s += k + v; return s;
// case: for-of-object-destructure
let s = ""; for (const {a, b} of [{a: 1, b: 2}, {a: 3, b: 4}]) s += a + b; return s;
// case: for-in-object
let s = ""; for (const k in {x: 1, y: 2}) s += k; return s;
// case: switch-fallthrough
const f = x => { let s = ""; switch (x) { case 1: s += "one"; case 2: s += "two"; break; case 3: s += "three"; break; default: s += "other"; } return s; }; return [f(1), f(2), f(3), f(4)];
// case: switch-strict
const f = x => { switch (x) { case 1: return "num"; case "1": return "str"; default: return "none"; } }; return [f(1), f("1"), f(true)];
// case: switch-default-middle
const f = x => { let s = ""; switch (x) { case 1: s += "a"; default: s += "d"; case 2: s += "b"; } return s; }; return [f(1), f(2), f(9)];
// case: switch-in-loop-continue
let s = ""; for (const x of [1, 2, 3]) { switch (x) { case 2: continue; default: s += x; } s += "."; } return s;
// case: try-catch-finally-order
const log = []; try { log.push("try"); throw new Error("x"); } catch (e) { log.push("catch " + e.message); } finally { log.push("finally"); } return log;
// case: finally-overrides-return
const f = () => { try { return "try"; } finally { return "finally"; } }; return f();
// case: finally-runs-on-return
const log = []; const f = () => { try { return "try"; } finally { log.push("fin"); } }; return [f(), log];
// case: catch-rethrow
const f = () => { try { throw new TypeError("a"); } catch (e) { throw new RangeError("b: " + e.message); } }; try { f(); } catch (e) { return [e.name, e.message]; }
// case: catch-thrown-string
try { throw "plain"; } catch (e) { return [typeof e, e]; }
// case: catch-thrown-object
try { throw {code: 7}; } catch (e) { return e.code; }
// case: catch-thrown-number
try { throw 42; } catch (e) { return e; }
// case: catch-type-error
try { const o = null; o.x; } catch (e) { return [e.name, e.message, e instanceof TypeError]; }
// case: catch-reference-error
try { return notDefined; } catch (e) { return e.name; }
// case: catch-reference-error-call
try { return notDefinedFn(); } catch (e) { return e.name; }
// case: catch-no-binding
try { throw 1; } catch { return "caught"; }
// case: nested-try
const log = []; try { try { throw new Error("inner"); } finally { log.push("f1"); } } catch (e) { log.push(e.message); } return log;
// case: error-message-format
try { null.x; } catch (e) { return e.message; }
// case: closure-counter
const make = () => { let n = 0; return () => { n += 1; return n; }; }; const c = make(); c(); c(); return c();
// case: closure-sees-later-assignment
let x = 1; const f = () => x; x = 2; return f();
// case: closure-writes-outer
let count = 0; [1, 2, 3].forEach(x => { count += x; }); return count;
// case: closure-writes-outer-map
let total = 0; const ys = [1, 2].map(x => { total += x; return x * 2; }); return [total, ys];
// case: closure-pair-shared-state
const make = () => { let n = 0; return {inc: () => { n++; }, get: () => n}; }; const o = make(); o.inc(); o.inc(); return o.get();
// case: closure-capture-loop-var
const fs = []; for (const x of [1, 2, 3]) fs.push(() => x * 10); return fs.map(f => f());
// case: closure-recursive-const
const fact = n => n <= 1 ? 1 : n * fact(n - 1); return fact(5);
// case: closure-recursive-local
const go = () => { const fib = n => n < 2 ? n : fib(n - 1) + fib(n - 2); return fib(10); }; return go();
// case: closure-default-params
const f = (a, b = a * 2, c = a + b) => [a, b, c]; return [f(1), f(1, 1), f(1, undefined, 0)];
// case: rest-params
const f = (a, ...r) => [a, r, r.length]; return [f(1), f(1, 2, 3)];
// case: spread-call
const f = (a, b, c) => a + b + c; return f(...[1, 2], 3);
// case: arguments-missing
const f = (a, b) => [a, b]; return f(1);
// case: function-declaration-inner
function add(a, b) { return a + b; } return add(2, 3);
// case: iife
return (() => 42)();
// case: arrow-returning-object
const f = () => ({a: 1}); return f();
// case: recursion-depth
const f = n => n === 0 ? 0 : 1 + f(n - 1); return f(300);
// case: block-scope-shadow
let x = 1; { let x = 2; } return x;
// case: block-scope-shadow-if
let x = 1; if (true) { let x = 2; x = 3; } return x;
// case: var-hoist-assign
let r = 0; if (true) { r = 5; } return r;
// case: const-reassign
const x = 1; x = 2; return x;
// case: assign-undeclared
y = 5; return y;
// case: return-nothing
const f = () => { return; }; return f();
// case: no-return
const f = () => { 1 + 1; }; return f();
// case: if-else-chain
const f = n => { if (n < 0) return "a"; else if (n === 0) return "b"; else return "c"; }; return [f(-1), f(0), f(1)];
// case: while-break-continue
let i = 0, s = ""; while (true) { i++; if (i % 2) continue; if (i > 6) break; s += i; } return s;
// case: nested-loops-counter
let n = 0; for (let i = 0; i < 4; i++) for (let j = i; j < 4; j++) n++; return n;
// case: increment-in-expression
let i = 0; const xs = [10, 20]; const v = xs[i++]; return [v, i];
// case: postfix-value
let i = 5; const a = i++; const b = ++i; return [a, b, i];
// case: assignment-expression-value
let a; const b = (a = 3) + 1; return [a, b];
// case: chained-assignment
let a, b; a = b = 4; return [a, b];
// case: string-concat-loop
let s = ""; for (let i = 0; i < 3; i++) s = s + i + ","; return s;
// case: early-return-loop
const f = xs => { for (const x of xs) if (x > 1) return x; return -1; }; return [f([1, 2, 3]), f([])];
// case: for-of-over-object-throws
for (const x of {a: 1}) {} return 1;
// case: map-with-index-callback-fn
const double = x => x * 2; return [1, 2].map(double);
// case: higher-order
const compose = (f, g) => x => f(g(x)); return compose(x => x + 1, x => x * 2)(5);
// case: currying
const add = a => b => c => a + b + c; return add(1)(2)(3);
// case: closure-in-object-method-this
const o = {n: 2, f() { return this.n; }}; return o.f();
// case: getter
// expect: unsupported
const o = {get x() { return 1; }}; return o.x;
// case: class
class A { constructor(x) { this.x = x; } } return new A(1).x;
// case: generator
// expect: unsupported
function* g() { yield 1; } return [...g()];
// case: new-map
const m = new Map(); m.set("a", 1); return m.get("a");
// case: new-set
return [...new Set([1, 1, 2])];
// case: date-now
return typeof Date.now();
// case: new-date
// expect: unsupported
return typeof new Date();
