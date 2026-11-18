// case: cyclic array string and join
const a=[];a.push(a);return [String(a),a.join(','),String([1,a,2])];

// case: private fields have lexical brands
class A{#x=1;read(o){return o.#x;}}class B{#x=2;}
try{return new A().read(new B());}catch(e){return e.name;}

// case: inherited private slots are separate
class A{#x=1;readA(){return this.#x;}}class B extends A{#x=2;readB(){return this.#x;}}
const b=new B();return [b.readA(),b.readB()];

// case: public hash keys and private slots are separate
class A{#x=42;read(){return this.#x;}}const a=new A();a['#x']=99;return [a['#x'],a.read(),Object.keys(a)];

// case: quoted class names stay public
class A{#x=1;'#x'=2;read(){return this.#x;}}const a=new A();return [a['#x'],a.read()];

// case: private getter setter and method
class A{#x=2;#twice(){return this.#x*2;}get #value(){return this.#twice();}set #value(v){this.#x=v;}run(){this.#value=7;return this.#value;}}
return new A().run();

// case: nested lexical private context
class Outer{#value=17;make(){return class Inner{read(o){function nested(x){return x.#value;}return nested(o);}}}}
const o=new Outer();const I=o.make();return new I().read(o);

// case: static private home and constructor brand
class A{static #x=1;static read(){return this.#x;}}class B extends A{}
try{return B.read();}catch(e){return e.name;}

// case: detached super methods retain home
class A{method(){return 'A';}}class B extends A{method(){return super.method()+'B';}}
const b=new B(), f=b.method;return [f.call(b),f.apply(b,[]),f.bind(b)(),f===b.method,new Map([[f,9]]).get(b.method)];

// case: computed class methods keep receiver
class A{#x=3;read(){return this.#x;}}const a=new A();return a['read']();

// case: ordinary member reference precedes argument evaluation
const o={f(){return 1;}};return o.f(o.f=()=>2);

// case: prototype member reference precedes argument evaluation
const o={};return o.hasOwnProperty((o.hasOwnProperty=()=>false,'hasOwnProperty'));

// case: assignment cannot initialize lexical slots
try{x=3;let x;return x;}catch(e){return e.name;}

// case: const declaration temporal dead zone write
try{x=3;const x=7;return x;}catch(e){return e.name;}

// case: inherited toJSON with receiver
class A{a=2;toJSON(){return this.a+1;}}return JSON.stringify(new A());

// case: own toJSON with receiver
return JSON.stringify({a:1,toJSON(){return this.a+1;}});

// case: replacer holder receiver
return JSON.stringify({a:1},function(k,v){return k==='a'?this.a+1:v;});

// case: reviver holder receiver
return JSON.parse('{"a":1}',function(k,v){return k==='a'?this.a+1:v;});

// case: reviver observes mutation to later properties
return JSON.parse('{"a":1,"b":2}',function(k,v){if(k==='a')this.b=7;return v;});

// case: frozen ordinary objects still have mutable private slots
class A{#x=1;set(v){this.#x=v;}get(){return this.#x;}}const a=Object.freeze(new A());a.set(4);return a.get();

// case: optional class calls retain their receiver
class A{#x=3;read(){return this.#x;}}const a=new A();return [a.read?.(),a?.read(),a['read']?.(),a?.['read'](),null?.read()];

// case: undefined callee stays undefined across argument effects
const o={};try{return o.f(o.f=()=>2);}catch(e){return e.name;}

// case: private names must be declared lexically
class A{read(o){return o.#missing;}}return new A();

// case: private access is invalid outside class scope
const o={};return o.#x;

// case: class constructors cannot be cloned
try{class A{}return structuredClone(A);}catch(e){return e.name;}

// case: class constructors serialize as functions
class A{}return [JSON.stringify(A),JSON.stringify({a:A}),JSON.stringify([A])];

// case: reviver can restore later array properties after truncation
return JSON.parse('[1,2]',function(k,v){if(k==='0')this.pop();return k==='1'?9:v;});

// case: duplicate private fields are syntax errors
class A{#x;#x;}return new A();

// case: private constructor names are syntax errors
class A{#constructor;}return new A();

// case: identifier reference precedes argument effects
let f=()=>1;return f((f=()=>2));

// case: error toString uses the error prototype method
return new TypeError('bad').toString();

// case: error toString can be borrowed
const f=new Error('a').toString;return [f.call({name:'Custom',message:'m',other:1}),f.call({}),f===new TypeError().toString];

// case: intrinsic method reference identity is stable
const a={};return a.hasOwnProperty===a.hasOwnProperty;
