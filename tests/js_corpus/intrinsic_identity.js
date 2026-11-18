// case: borrowed Object methods retain their defining prototype
const f=({}).toString;return [f.call([1,2]),f.call(new Error('x')),f.call(new Map()),f.call(new Set()),f.call(/x/),f.call(null),f.call(undefined),f.call(2),f.call('a'),f.call(true),f.call(Math),f.call(JSON)];

// case: native identities distinguish families and share inherited methods
return [({}).toString===[].toString,({}).hasOwnProperty===[].hasOwnProperty,[].forEach===new Map().forEach,new Map().has===new Set().has,new Error().toString===new TypeError().toString];

// case: borrowed array traversal and callbacks retain receiver and thisArg
const o={0:2,1:3,length:2};const f=[].map;return f.call(o,function(v,i,r){return v+this.offset+i+(r===o?1:0);},{offset:4});

// case: borrowed array mutations share the array algorithms
const o={0:'a',1:'b',length:2};const n=[].push.call(o,'c');const removed=[].splice.call(o,1,1,'x','y');[].reverse.call(o);return [n,removed,[].slice.call(o),o.length];

// case: borrowed array join reads each property when visited
class A{length=2;get '0'(){this['1']='b';return 'a';}}return [].join.call(new A(),'-');

// case: borrowed methods keep their identity through apply and bind
const o={0:'a',1:'b',length:2};const f=[].join;return [f.apply(o,['-']),f.bind(o,':')(),({}).toString.bind(o)()];

// case: array toString uses the receivers join property
return [[].toString.call({}),[].toString.call({join(){return 'custom';}})];

// case: object own property methods work on arrays and errors
const own=({}).hasOwnProperty, enumerable=({}).propertyIsEnumerable;return [own.call([7],'0'),own.call([7],'length'),enumerable.call([7],'length'),own.call(new Error('x'),'message'),enumerable.call(new Error('x'),'message')];

// case: branded collection methods reject unrelated receivers
try{return new Map().get.call(new Set(),'x');}catch(e){return e.name;}

// case: branded number and regexp methods reject unrelated receivers
let a,b;try{(1).toFixed.call('x');}catch(e){a=e.name;}try{/x/.test.call({});}catch(e){b=e.name;}return [a,b];

// case: borrowed string methods coerce ordinary receivers
return [''.trim.call([' x ']),''.toUpperCase.call(true)];

// case: array callbacks validate callable arguments even when empty
let a,b;try{[].map(0);}catch(e){a=e.name;}try{[].reduce(0,1);}catch(e){b=e.name;}return [a,b];

// case: sort callbacks see the live receiver and preserve newly appended items
const xs=[3,1,2];let before='';xs.sort((a,b)=>{if(!before){before=xs.join(',');xs.push(9);}return a-b;});return [before,xs];

// case: mutating method reference is resolved before argument evaluation
const o={push(){return 1;}};return o.push((o.push=()=>2));

// case: error string conversion joins converted field values
const e=new Error('m');e.name=[];const a=String(e);e.name=undefined;e.message=undefined;return [a,String(e),e.toString()];

// case: apply takes object arraylikes and rejects primitive argument lists
const f=(a,b)=>[a,b];let error;try{f.apply(null,'ab');}catch(e){error=e.name;}return [f.apply(null,{0:2,1:3,length:2}),error];

// case: borrowed native methods are not constructors
const F=[].join;try{return new F();}catch(e){return e.name;}

// case: flatten skips missing properties in arraylike objects
return [].flat.call({1:[2],length:3});

// case: sparse results remain explicitly unsupported
// expect: unsupported
return [].map.call({1:2,length:3},x=>x);

// case: flatten consumes nested arrays before reading later getters
const xs=[1];class A{length=2;get '0'(){return xs;}get '1'(){xs[0]=9;return 2;}}return [].flat.call(new A());

// case: unshift with no items does not rewrite indexed properties
let n=0;class A{length=1;get '0'(){n++;return 1;}set '0'(v){n++;}}const a=new A();[].unshift.call(a);return n;

// case: function source reflection remains explicitly unsupported
// expect: unsupported
return (()=>1).toString();

// case: regexp string formatting is generic while execution is branded
const f=/x/.toString;return f.call({source:'a',flags:'i'});

// case: intrinsic members agree between property access and in
return ['call' in Math.max,'max' in Math,'PI' in Math,'get' in new Map(),'get' in new Set()];

// case: native aliases retain one function identity
return [''.trimLeft===''.trimStart,''.trimRight===''.trimEnd,new Set().keys===new Set().values,new Map().keys===new Map().values];

// case: failed calls do not leak a receiver into the next call
class A{}const o={A};try{o.A();}catch(e){}function f(){return this;}return f()===undefined;
