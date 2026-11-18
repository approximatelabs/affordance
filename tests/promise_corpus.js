// Observable programs, shared with Node. host(label) is the deterministic fake host.
// Add a case for a behavior, rather than asserting interpreter structure.

// case: independent rejection values
const fail = async code => { throw {code}; };
const p = fail(7), q = fail(9), seen = [];
for (const x of [p, q, p]) { try { await x; } catch (e) { seen.push(e.code); } }
return seen;

// case: an async throw retains the Error instance
const reason = new TypeError('bad');
const fail = async () => { throw reason; };
const p = fail();
try { await p; } catch (e) {
  return [e === reason, e instanceof TypeError, e.message, await p.catch(e => e === reason)];
}

// case: rejected primitives retain their values
const values = [undefined, null, false, 0, '', 'oops'];
const promises = values.map(x => Promise.reject(x));
const seen = [];
for (let i = 0; i < values.length; i++) {
  try { await promises[i]; } catch (e) { seen.push(e === values[i]); }
}
return seen;

// case: one host completion has one identity
const p = host('object');
const a = await p, b = await p, both = await Promise.all([p, p]);
return [a === b, both[0] === both[1], new Map([[a, 'found']]).get(b)];

// case: resolve preserves an existing promise
const p = host('object');
return [Promise.resolve(p) === p, await Promise.resolve(3)];

// case: combinator completions retain containers and nested identities
const value = {n: 1}, p = Promise.resolve(value);
const promises = [Promise.all([p, p]), Promise.allSettled([p, Promise.reject(value)])];
const out = [];
for (const promise of promises) {
  const a = await promise, b = await promise;
  out.push(a === b, a[0] === b[0]);
}
const rows = await promises[1];
return [out, rows[0].value === value, rows[1].reason === value];

// case: reactions forward values and ignore noncallables
const value = {n: 3}, p = Promise.resolve(value);
const fulfilled = p.then(17).catch(false).finally('ignored');
const rejected = Promise.reject(value).then(null, false).catch(e => e);
return [await fulfilled === value, await rejected === value,
        await host('object').then(r => r.n + 1).then(n => n * 10)];

// case: catch receives original reason and a throw in a callback replaces it
const reason = {code: 7}, replacement = {code: 9};
const p = Promise.reject(reason);
const seen = [];
try { await p.catch(e => { seen.push(e === reason); throw replacement; }); }
catch (e) { seen.push(e === replacement); }
try { await p; } catch (e) { seen.push(e === reason); }
return seen;

// case: finally forwards original completion despite another caught exception
const reason = {code: 7}, other = {code: 9}, value = {ok: true};
const cleanup = () => { try { throw other; } catch (e) {} };
const seen = [await Promise.resolve(value).finally(cleanup) === value];
try { await Promise.reject(reason).finally(cleanup); } catch (e) { seen.push(e === reason); }
return seen;

// case: finally rejection replaces fulfillment or rejection
const first = {first: true}, last = {last: true}, seen = [];
for (const p of [Promise.resolve(first), Promise.reject(first)]) {
  try { await p.finally(() => Promise.reject(last)); } catch (e) { seen.push(e === last); }
}
return seen;

// case: try finally preserves a pending thrown value
const original = {code: 7}, other = {code: 9};
try {
  try { throw original; }
  finally { try { throw other; } catch (e) {} }
} catch (e) { return e === original; }

// case: all and race forward original rejection
const reason = {code: 7}, p = Promise.reject(reason), seen = [];
for (const x of [Promise.all([p]), Promise.race([p])]) {
  try { await x; } catch (e) { seen.push(e === reason); }
}
return seen;

// case: any owns one AggregateError with ordered reasons
const first = {code: 7}, second = {code: 9};
const p = Promise.any([Promise.reject(first), Promise.reject(second)]);
let one;
try { await p; } catch (e) { one = e; }
try { await p; } catch (e) {
  return [e === one, e.name, e.errors[0] === first, e.errors[1] === second];
}

// case: empty combinators
const a = Promise.all([]), b = Promise.allSettled([]);
let error;
try { await Promise.any([]); } catch (e) { error = [e.name, e.errors.length]; }
return [(await a).length, (await b).length, error];

// case: race keeps the earliest host completion when replay sees both
const slow = host('slow'), fast = host('fast');
return await Promise.race([slow, fast]);

// case: allSettled describes host success and failure
const a = host('object'), b = host('failure');
const settled = await Promise.allSettled([a, b]);
let caught;
try { await Promise.all([a, b]); } catch (e) { caught = e.message; }
return [settled.map(s => s.status), caught];

// case: host errors retain type and identity across awaits and reactions
const p = host('abort');
let first;
try { await p; } catch (e) { first = e; }
return [first.name, first.message, await p.catch(e => e === first),
        (await Promise.allSettled([p]))[0].reason === first];

// case: finally waits for cleanup after work completes
try { await host('work'); } finally { await host('cleanup'); }
return 'done';

// case: async rejection survives suspension and concurrent completion
const fail = async code => { await host('object'); throw {code}; };
const p = fail(7), q = fail(9), out = [];
for (const x of [p, q]) { try { await x; } catch (e) { out.push(e.code); } }
return out;

// case: fulfilled await yields to its caller
const order = [];
const f = async () => { order.push('before'); await 0; order.push('after'); };
const p = f(); order.push('caller'); await p;
return order;

// case: a late race observes ready inputs in input order
const slow = host('slow'), fast = host('fast');
await Promise.all([slow, fast]);
return [await Promise.race([slow, fast]), await Promise.any([slow, fast])];

// case: an async child inherits its caller's observed completion point
const slow = host('slow'), fast = host('fast');
await Promise.all([slow, fast]);
const race = async () => await Promise.race([slow, fast]);
return await race();

// case: a late all forwards the first ready rejection by input order
const first = host('slow').then(() => { throw 'first'; });
const second = host('fast').then(() => { throw 'second'; });
await Promise.allSettled([first, second]);
try { await Promise.all([first, second]); } catch (e) { return e; }

// case: combinators accept supported iterables
return [await Promise.all(new Set([1,2])), await Promise.all('ab'),
        await Promise.all(new Map([['x',1],['y',2]])), await Promise.race(new Set([4,5]))];

// case: invalid iterables reject without throwing at the call
const out = [];
for (const input of [null, undefined, 7, {}]) {
  for (const make of [x => Promise.all(x), x => Promise.race(x), x => Promise.any(x), x => Promise.allSettled(x)]) {
    let promise;
    try { promise = make(input); out.push('returned'); } catch (e) { out.push('threw'); }
    try { await promise; } catch (e) { out.push(e.name); }
  }
}
return out;

// case: Promise static methods retain their constructor receiver
const out = [];
for (const make of [Promise.all, Promise.race, Promise.any, Promise.allSettled]) {
  try { make([7]); } catch (e) { out.push(e.name); }
  try { make.call({}, [7]); } catch (e) { out.push(e.name); }
  out.push(await make.bind(Promise)([7]), await make.call(Promise, [7]));
}
for (const make of [Promise.resolve, Promise.reject]) {
  try { make(7); } catch (e) { out.push(e.name); }
}
return out;

// case: deep adoption chains preserve their value
const value = {n: 7};
let promise = Promise.resolve(value);
for (let i = 0; i < 12000; i++) { promise = (async p => p)(promise); }
return await promise === value;

// case: deep combinator graphs preserve their value
let promise = Promise.resolve(7);
for (let i = 0; i < 12000; i++) { promise = Promise.race([promise]); }
return await promise;

// case: race observes combinator reaction jobs after immediately resolved inputs
return await Promise.race([Promise.all([0]), Promise.resolve(1)]);

// case: expression operands and references are captured before awaiting
let a = {n: 1}, old = a, f = x => 'first' + x;
const order = [];
const wait = async () => { order.push('wait'); a = {n: 100}; f = x => 'second' + x; return 2; };
old.n += await wait();
const called = f(await (async () => { f = x => 'third' + x; return 4; })());
let xs = [1], first = xs;
xs.push(await (async () => { xs = []; return 5; })());
return [old.n, a.n, called, first, xs, order];

// case: spread snapshots precede later awaited operands
let xs = [1], obj = {a: 1};
const change = async () => { xs.push(2); obj.a = 2; return 3; };
const array = [...xs, await change()];
obj.a = 1;
const object = {...obj, b: await change()};
return [array, object];

// case: awaited control flow crosses loops labels and cleanup
const seen = [];
outer: for (let i = 0; i < 4; i++) {
  try {
    if (await Promise.resolve(i === 0)) continue outer;
    for (const j of [0, 1]) {
      seen.push(await Promise.resolve(i * 10 + j));
      if (i === 2) break outer;
    }
  } finally { seen.push(await Promise.resolve('f' + i)); }
}
return seen;

// case: awaited finally replaces pending return or throw
const f = async () => { try { return 1; } finally { return await Promise.resolve(2); } };
const g = async () => { try { throw 'first'; } finally { await 0; throw 'last'; } };
const h = async () => { try { return 3; } finally { await 0; } };
let reason; try { await g(); } catch(e) { reason = e; }
return [await f(), reason, await h()];

// case: optional chains skip awaited keys arguments and preserve the receiver
let n = 0;
const arg = async () => { n++; return 4; };
const o = {n: 2, f(x) { return this.n + x; }};
const missing = null;
return [missing?.f(await arg()), missing?.[await arg()], o.f?.(await arg()), n];

// case: nested default patterns suspend only for missing values
const seen = [];
const get = async n => { seen.push(n); return n; };
const [a = await get(1), b = await get(2)] = [undefined, 4];
const {[await get('x')]: c = await get(3), y: [d = await get(5)]} = {y: []};
return [a, b, c, d, seen];

// case: superclass method resolution precedes awaited arguments
class A { f(x) { return this.n + x; } }
class B extends A { async f(x) { return super.f(await Promise.resolve(x)); } }
const b = new B(); b.n = 7;
return await b.f(2);

// case: reactions and async returns share FIFO ordering
const order = [];
const p = Promise.resolve(0);
p.then(() => order.push('then'));
(async () => { await p; order.push('await'); })();
p.finally(() => order.push('finally')).then(() => order.push('after finally'));
await (async () => p)();
order.push('adopted');
await 0; await 0; await 0;
return order;

// case: constructors and mutable method references precede awaited arguments
class A { constructor(n) { this.n = n; } }
class B { constructor(n) { this.n = n * 10; } }
let C = A;
const object = new C(await (async () => { C = B; return 2; })());
const target = {push: x => 'first' + x};
const method = target.push(await (async () => { target.push = x => 'last' + x; return 3; })());
return [object.n, method];

// case: dense array construction and ordinary fill calls retain awaited operands
const make = n => [n];
return [Array(await Promise.resolve(2)).fill(await Promise.resolve(4)),
        make(await Promise.resolve(3)).fill(await Promise.resolve(9))];

// case: parameter default helpers keep their own function entry
function one() { return 1; }
async function positional(x = one()) { await host('work'); return x + 1; }
const arrow = async (x = one()) => { await 0; return x + 2; };
const props = async ({x = one()}) => { await 0; return x + 3; };
const object = {async method(x = one()) { await 0; return x + 4; }};
return [await positional(), await arrow(), await props({}), await object.method.bind(object)()];

// case: rejected parameter defaults reject the call before entering its body
const reason = {code: 7};
const fail = () => { throw reason; };
let entered = 0;
async function f(x = fail()) { entered++; return 0; }
const g = async ({x = fail()}) => { entered++; return 0; };
const results = await Promise.allSettled([f(), g({})]);
return [entered, results.map(result => [result.status, result.reason === reason])];

// case: unawaited host work releases its owner after completion
host('work');
return 'returned';

// Evaluation order of an expression's parts around an `await`.

// case: order: binary operands run left to right
const log = []; const a = () => { log.push("a"); return 1; }; const b = async () => { log.push("b"); return 2; }; const r = a() + await b() + a(); return [r, log];

// case: order: list spread is copied before later items
let xs = [1]; const grow = async () => { xs.push(9); return 3; }; const out = [...xs, await grow(), ...xs]; return out;

// case: order: object spread is copied before later fields
const o = {a: 1}; const change = async () => { o.a = 5; o.b = 6; return 2; }; const out = {...o, c: await change(), ...o}; return out;

// case: order: computed key becomes text before the value runs
let n = 1; const key = () => "k" + n; const value = async () => { n = 2; return 1; }; const out = {[key()]: await value(), [key()]: 0}; return out;

// case: order: template holes become text in order
const xs = [1]; const second = async () => { xs.push(2); return "s"; }; const out = `${xs}-${await second()}-${xs}`; return out;

// case: order: a method is read before its arguments run
const o = {m() { return "old"; }}; const swap = async () => { o.m = () => "new"; return 1; }; return o.m(await swap());

// case: order: a function is read before its arguments run
let f = () => "old"; const swap = async () => { f = () => "new"; return 1; }; return f(await swap());

// case: order: index base and key run before the read
const log = []; const base = () => { log.push("base"); return {k: 7}; }; const key = async () => { log.push("key"); return "k"; }; return [base()[await key()], log];

// case: order: optional index with an awaited key
const o = {k: 4}; const none = null; const key = async () => "k"; return [o?.[await key()], none?.[await key()], o?.missing?.[await key()]];

// case: order: optional call with awaited arguments
const o = {f: (v) => v + 1}; const one = async () => 1; return [o.f?.(await one()), o.g?.(await one()), o?.f(await one())];

// case: order: constructor arguments run in order
const log = []; class P { constructor(a, b) { this.v = [a, b]; } } const n = async (v) => { log.push(v); return v; }; const p = new P(await n(1), await n(2)); return [p.v, log];

// case: order: unary and typeof of an awaited value
const v = async () => 3; return [-(await v()), !(await v()), typeof (await v()), [...[await v()]]];

// case: order: comma runs both and gives the last
const log = []; const n = async (v) => { log.push(v); return v; }; const r = (await n(1), await n(2)); return [r, log];

// case: order: compound assignment reads the target first
const o = {n: 1}; const bump = async () => { o.n = 10; return 5; }; o.n += await bump(); return o.n;

// case: order: delete with an awaited key
const o = {a: 1, b: 2}; const key = async () => "a"; delete o[await key()]; return Object.keys(o);

// case: order: spread arguments are copied before later ones
const xs = [1]; const grow = async () => { xs.push(9); return 3; }; const all = (...v) => v; return all(...xs, await grow());

// case: order: a computed key becomes text when it is evaluated
const log = []; const key = {toString() { log.push("key"); return "k"; }}; const value = async () => { log.push("value"); return 1; }; const out = {[key]: await value()}; return [out, log];

// case: order: template holes become text in order with toString
const log = []; const first = {toString() { log.push("first"); return "1"; }}; const second = async () => { log.push("second"); return "2"; }; const out = `${first}-${await second()}-${first}`; return [out, log];

// case: an awaited assignment to a key with toString
const o = {}; const key = {toString() { return "k"; }}; const v = async () => 5; o[key] = await v(); return o;
