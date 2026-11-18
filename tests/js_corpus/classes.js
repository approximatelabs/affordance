// case: class-basic
class Counter { constructor(n) { this.n = n; } inc() { this.n += 1; return this; } get double() { return this.n * 2; } } const c = new Counter(1); c.inc().inc(); return [c.n, c.double, c instanceof Counter];
// case: class-fields-and-static
class A { x = 1; static count = 0; static make() { A.count += 1; return new A(); } } A.make(); const a = A.make(); return [a.x, A.count];
// case: class-extends-super
class Animal { constructor(name) { this.name = name; } speak() { return `${this.name} makes a sound`; } } class Dog extends Animal { speak() { return super.speak() + " (woof)"; } } const d = new Dog("rex"); return [d.speak(), d instanceof Animal, d instanceof Dog, Object.keys(d)];
// case: class-private
class Box { #v = 0; set(v) { this.#v = v; return this; } get() { return this.#v; } } const b = new Box().set(4); return [b.get(), Object.keys(b)];
// case: class-getter-setter
class T { #c = 0; get c() { return this.#c; } set c(v) { this.#c = Math.max(0, v); } } const t = new T(); t.c = -5; const a = t.c; t.c = 7; return [a, t.c];
// case: class-methods-on-prototype
class P { m() { return 1; } } const p = new P(); return ["m" in p, p.hasOwnProperty("m"), Object.keys(p), typeof P];
// case: class-detached-this
class Q { v = 3; get() { return this.v; } } const f = new Q().get; try { return f(); } catch (e) { return e.name; }
// case: class-arrow-this
class R { v = 2; make() { return () => this.v * 10; } } return new R().make()();
// case: class-error-subclass
class NotFound extends Error { constructor(what) { super(`no ${what}`); this.name = "NotFound"; } } try { throw new NotFound("file"); } catch (e) { return [e.name, e.message, e instanceof NotFound, e instanceof Error, String(e)]; }
// case: class-new-without-new
class S {} try { S(); return "no"; } catch (e) { return e.name; }
// case: class-call-apply-bind
function f(a, b) { return this.k + a + b; } const o = {k: 1}; return [f.call(o, 2, 3), f.apply(o, [2, 3]), f.bind(o, 2)(3)];
// case: object-method-this
const o = {n: 2, twice() { return this.n * 2; }}; return o.twice();
// case: error-own-props
const e = new Error("x"); return [Object.keys(e), JSON.stringify(e), e.hasOwnProperty("message"), e.hasOwnProperty("name"), "name" in e];
// case: inherited-in
return ["toString" in {}, "hasOwnProperty" in {}, typeof {}.hasOwnProperty, "push" in [], "length" in []];
// case: this-plain-call-undefined
function f() { return this; } return [f() === undefined, typeof f];
// case: this-arrow-lexical
const o = {n: 3, m() { const g = () => this.n * 2; return g(); }}; return o.m();
// case: this-method-detached
const o = {n: 3, m() { return this === undefined ? "none" : this.n; }}; const d = o.m; return [o.m(), d()];
// case: this-bind-partial
function add(a, b) { return this.base + a + b; } const g = add.bind({base: 100}, 1); return [g(2), g(3), typeof g];
// case: this-nested-function-own
const o = {n: 1, m() { function inner() { return this; } return inner() === undefined; }}; return o.m();
// case: this-callback-member
const counter = {count: 0, inc() { this.count += 1; return this; }}; counter.inc().inc(); return counter.count;
// case: class-init-order
const log = []; class A { a = (log.push("A.field"), 1); constructor() { log.push("A.ctor:" + this.a); } } class B extends A { b = (log.push("B.field"), this.a + 1); constructor() { log.push("B.before"); super(); log.push("B.after:" + this.b); } } new B(); return log;
// case: class-static-inheritance
class A { static k = 5; static make() { return new this(); } who() { return "A"; } } class B extends A { who() { return "B"; } } return [B.k, B.make().who(), A.make().who(), B.make() instanceof A];
// case: class-ctor-returns-object
class A { constructor() { return {custom: true}; } } const a = new A(); return [a.custom, a instanceof A];
// case: class-private-brand
class A { #x = 1; static peek(o) { try { return o.#x; } catch (e) { return e.name; } } } return [A.peek(new A()), A.peek({})];
// case: class-json-and-spread
class P { #secret = 1; constructor() { this.x = 1; this.y = [2]; } get sum() { return this.x + 1; } m() {} } const p = new P(); return [JSON.stringify(p), {...p}, Object.keys(p), Object.entries(p), p.sum];
// case: class-getter-inherited-override
class A { get v() { return 1; } get w() { return this.v * 10; } } class B extends A { get v() { return 2; } } return [new A().w, new B().w];
// case: class-method-override-super-chain
class A { f() { return "a"; } } class B extends A { f() { return super.f() + "b"; } } class C extends B { f() { return super.f() + "c"; } } return new C().f();
// case: class-expression
const K = class { constructor(n) { this.n = n; } double() { return this.n * 2; } }; const named = class Inner { name() { return "x"; } }; return [new K(4).double(), new named().name(), typeof K];
// case: class-instance-as-map-key
class Key {} const k1 = new Key(), k2 = new Key(); const m = new Map([[k1, "one"], [k2, "two"]]); return [m.get(k1), m.get(k2), m.size];
// case: class-this-in-callback
class Acc { total = 0; add(xs) { xs.forEach(x => { this.total += x; }); return this.total; } } return new Acc().add([1, 2, 3]);
// case: class-setter-validates
class Temp { #c = 0; set c(v) { if (typeof v !== "number") throw new TypeError("number please"); this.#c = v; } get c() { return this.#c; } } const t = new Temp(); t.c = 21; let err = ""; try { t.c = "x"; } catch (e) { err = e.name + ":" + e.message; } return [t.c, err];
// case: class-error-subclass-default-name
class Oops extends Error {} const e = new Oops("bad"); return [e.name, e.message, String(e), e instanceof Oops, e instanceof Error, Object.keys(e)];
// case: class-typeerror-subclass
class Bad extends TypeError { constructor(m) { super(m); this.name = "Bad"; } } try { throw new Bad("nope"); } catch (e) { return [e.name, e.message, e instanceof TypeError, e instanceof Error, e instanceof Bad]; }
// case: class-private-method
class A { #twice(n) { return n * 2; } run(n) { return this.#twice(n) + 1; } } return new A().run(5);
// case: class-static-private-and-getter
class Id { static #next = 1; static get peek() { return Id.#next; } static take() { return Id.#next++; } } const a = Id.take(), b = Id.take(); return [a, b, Id.peek];
// case: class-method-detached-then-bound
class C { v = 7; get() { return this.v; } } const c = new C(); const g = c.get.bind(c); return [g(), c.get.call({v: 9})];
// case: getter_only_instance_and_static_properties_are_read_only
class ReadOnly {
  get value() { return 3; }
  static get value() { return 4; }
  change() { this.value = 9; }
  static change() { this.value = 10; }
}
const value = new ReadOnly();
const errors = [];
try { value.change(); } catch (e) { errors.push(e.name); }
try { ReadOnly.change(); } catch (e) { errors.push(e.name); }
return [value.value, ReadOnly.value, errors];
// case: derived_data_method_shadows_parent_readonly_getter
class Base {get value(){return 1;} static get value(){return 2;}}
class Derived extends Base {value(){return 3;} static value(){return 4;}}
const object = new Derived();
object.value = 7;
Derived.value = 8;
return [object.value, Derived.value];
