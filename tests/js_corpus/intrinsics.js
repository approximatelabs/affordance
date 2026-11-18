// case: lexical namespace calls constants and method aliases
const Math = {max: () => 77, PI: 12};
const f = Math.max;
return [Math.max(1, 2), f(), Math.PI];

// case: builtin namespace parameter
function use(Math) { return Math.max(1, 2); }
return use({max: () => 91});

// case: lexical JSON and Object assign
const JSON = {stringify: () => 'own'};
const Object = {assign: () => ({x: 8})};
return [JSON.stringify({}), Object.assign({}, {x: 1}).x];

// case: intrinsic functions and constructor aliases
const A = Array, N = Number, M = Map, f = Math.max;
const m = new M(); m.set('x', 3);
return [A.of(N('4'))[0], m.get('x'), f(2, 5), f === Math.max, m instanceof M];

// case: global constants respect lexical bindings
function use(undefined, NaN, Infinity) { return [undefined, NaN, Infinity]; }
return use(3, 4, 5);

// case: intrinsic namespace temporal dead zone
try { Math.max(1, 2); const Math = {}; } catch (e) { return e.name; }

// case: strings are not instanceof constructors
try { return [] instanceof 'Array'; } catch (e) { return e.name; }

// case: JSON replacer mutates its live input
const a = [1, 2, 3];
return JSON.stringify(a, (k, v) => { if (k === '0') a.pop(); return v; });
