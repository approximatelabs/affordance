// case: map-filter-reduce
const xs = [1, 2, 3, 4, 5]; return [xs.map(x => x * 2), xs.filter(x => x % 2), xs.reduce((a, b) => a + b), xs.reduce((a, b) => a + b, 10)];
// case: reduce-empty
return [].reduce((a, b) => a + b);
// case: reduce-empty-init
return [].reduce((a, b) => a + b, 7);
// case: reduce-index-args
return [5, 6].reduce((acc, x, i, arr) => acc + `${x}@${i}/${arr.length};`, "");
// case: reduceRight
return [1, 2, 3].reduceRight((a, b) => a + "-" + b);
// case: sort-default
return [[3, 1, 10, 2].sort(), ["b", "a", "C"].sort(), [true, false].sort(), [undefined, 3, null, 1].sort()];
// case: sort-comparator
return [[3, 1, 10, 2].sort((a, b) => a - b), [3, 1, 10, 2].sort((a, b) => b - a)];
// case: sort-stable
const xs = [{k: 1, v: "a"}, {k: 0, v: "b"}, {k: 1, v: "c"}, {k: 0, v: "d"}]; return xs.sort((a, b) => a.k - b.k).map(x => x.v).join("");
// case: sort-by-string-key
const xs = [{n: "bob"}, {n: "al"}, {n: "cy"}]; return xs.sort((a, b) => a.n < b.n ? -1 : a.n > b.n ? 1 : 0).map(x => x.n);
// case: sort-localeCompare
return ["b", "a", "c"].sort((a, b) => a.localeCompare(b));
// case: sort-in-place
const xs = [3, 1, 2]; xs.sort(); return xs;
// case: sort-returns-same
const xs = [3, 1, 2]; const ys = xs.sort(); return [ys, xs === ys];
// case: reverse-in-place
const xs = [1, 2, 3]; xs.reverse(); return xs;
// case: sort-boolean-comparator
return [5, 1, 4, 2, 3].sort((a, b) => a > b);
// case: sort-numbers-as-strings
return [1, 2, 10, 21, 100, -1, -2].sort();
// case: toSorted-toReversed
const xs = [3, 1, 2]; return [xs.toSorted(), xs.toReversed(), xs];
// case: splice
const xs = [1, 2, 3, 4, 5]; const r = xs.splice(1, 2, "a", "b", "c"); return [xs, r];
// case: pop-shift
const xs = [1, 2, 3]; const a = xs.pop(); const b = xs.shift(); return [a, b, xs];
// case: unshift
const xs = [1]; xs.unshift(0); return xs;
// case: push-returns-length
const xs = [1]; const n = xs.push(2, 3); return [n, xs];
// case: push-statement
const xs = []; xs.push(1); xs.push(2, 3); return xs;
// case: flat
return [[1, [2, [3, [4]]]].flat(), [1, [2, [3, [4]]]].flat(2), [1, [2, [3, [4]]]].flat(Infinity), [[1], 2].flat(0)];
// case: flatMap
return [1, 2].flatMap(x => [x, [x * 10]]);
// case: includes
return [[1, 2, NaN].includes(NaN), [1, 2].includes("1"), [null].includes(undefined), [1, 2, 3].includes(1, 1), [0].includes(-0)];
// case: indexOf
return [[1, 2, NaN].indexOf(NaN), [1, 2, 1].indexOf(1, 1), [1, 2, 1].lastIndexOf(1), [1, "1"].indexOf("1"), [1, 2, 3].indexOf(3, -1)];
// case: at
const xs = [1, 2, 3]; return [xs.at(0), xs.at(-1), xs.at(-4), xs.at(3), xs.at(1.7), xs.at("1"), xs.at()];
// case: findLast
const xs = [1, 2, 3, 4]; return [xs.findLast(x => x % 2 === 1), xs.findLastIndex(x => x > 10), xs.find(x => x > 2), xs.findIndex(x => x === 3)];
// case: some-every-empty
return [[].some(x => true), [].every(x => false)];
// case: join
return [[1, null, undefined, 2].join("-"), [].join(), [[1, 2], [3]].join(";"), [1, 2].join(undefined), [1, 2].join(null)];
// case: concat
return [[1].concat(2, [3, [4]]), [].concat([], [])];
// case: slice
const xs = [1, 2, 3, 4]; return [xs.slice(1), xs.slice(-2), xs.slice(1, -1), xs.slice(3, 1), xs.slice()];
// case: entries-keys-values
const xs = ["a", "b"]; return [[...xs.entries()], [...xs.keys()], [...xs.values()]];
// case: array-from
return [Array.from("abc"), Array.from({length: 3}, (_, i) => i * 2), Array.from([1, 2], x => x + 1), Array.from({length: 2})];
// case: array-of-isArray
return [Array.of(1, 2), Array.isArray([]), Array.isArray("a"), Array.isArray({length: 0})];
// case: array-length-set
const xs = [1, 2, 3]; xs.length = 1; return xs;
// case: array-index-assign
const xs = [1, 2, 3]; xs[1] = 9; return xs;
// case: array-index-assign-grow
// expect: unsupported
const xs = [1]; xs[3] = 4; return xs.length;
// case: array-index-assign-end
const xs = [1]; xs[xs.length] = 2; return xs;
// case: array-negative-index
const xs = [1, 2]; return [xs[-1], xs[2]];
// case: fill
return new Array(3).fill(0);
// case: array-constructor
// expect: unsupported
return [Array(3).length, [...Array(2)]];
// case: nested-map
return [[1, 2], [3]].map(r => r.map(x => x * x));
// case: forEach-sum
let s = 0; [1, 2, 3].forEach(x => { s += x; }); return s;
// case: map-index
return ["a", "b"].map((x, i) => x + i);
// case: map-parseInt
return ["1", "2", "3"].map(parseInt);
// case: array-to-string
return [String([1, [2, [3]]]), [] + [], [1] + [2], [null] + "", [undefined, null] + ""];
// case: array-equality
const a = [1]; const b = a; return [a === b, [1] === [1], a == "1", [1, 2] == "1,2"];
// case: spread-array
const a = [1, 2]; return [...a, ...[3], ...""];
// case: destructure-array
const [a, b, c] = [1, 2]; return [a, b, c];
// case: destructure-array-default
const [a = 5, b = 6] = [undefined, null]; return [a, b];
// case: destructure-array-rest
const [a, ...rest] = [1, 2, 3]; return [a, rest];
// case: destructure-array-hole
const [, b] = [1, 2]; return b;
// case: destructure-nested
const [[a], {b}] = [[1], {b: 2}]; return [a, b];
// case: destructure-swap
let a = 1, b = 2; [a, b] = [b, a]; return [a, b];
// case: every-index
return [1, 2, 3].every((x, i) => x === i + 1);
// case: array-keys-for-in
const out = []; for (const k in [7, 8]) out.push(typeof k + k); return out;
// case: array-of-arrays-sort
return [[2, "b"], [1, "a"], [2, "a"]].sort((x, y) => x[0] - y[0] || (x[1] < y[1] ? -1 : 1));
// case: array-includes-string
return ["ab", "c"].includes("a");
// case: array-reverse-copy
const xs = [1, 2, 3]; const ys = [...xs].reverse(); return [xs, ys];
// case: sort-undefined-comparator-result
return [3, 1, 2].sort(() => 0);
// case: sort-comparator-nan
return [3, 1, 2].sort(() => NaN);
// case: map-holes-length
return Array.from({length: 3}).map((_, i) => i);
