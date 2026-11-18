// case: loose-eq-primitives
return [1 == "1", 0 == "", 0 == "0", "" == "0", null == 0, undefined == 0, null == false, true == 1, true == "1", false == "false", "1.0" == 1, " 1 " == 1];
// case: loose-eq-objects
const a = [1]; return [a == 1, a == "1", [1, 2] == "1,2", [] == "", [] == 0, [0] == false, {} == "[object Object]", [null] == "", [[]] == 0];
// case: loose-eq-nan
return [NaN == NaN, NaN != NaN, NaN == "NaN"];
// case: strict-eq
return [1 === 1.0, "a" === "a", null === null, undefined === undefined, null === undefined, NaN === NaN, 0 === -0];
// case: not-equals
return [1 != "1", 1 !== "1", null != undefined, null !== undefined];
// case: add-mixed
return [1 + "2", "1" + 2, 1 + true, true + true, 1 + null, 1 + undefined, null + null, "a" + null, [] + {}, [1] + 1, {} + 1, true + "x", undefined + "x"];
// case: add-array-number
return [[1, 2] + 3, [] + 1, [5] - 2, [5] * [2]];
// case: subtract-coercion
return ["5" - "2", "5" - true, null - 1, undefined - 1, [] - 1, "" - 1, " " - 1];
// case: compare-mixed
return [1 < "2", "10" < 9, "a" < 1, null < 1, undefined < 1, null >= 0, null > 0, null == 0, [2] < 10, "b" > "a", true > false, "abc" < "abd"];
// case: compare-arrays-strings
return [[2] > "10", ["a"] < "b", [1, 2] < "1,3"];
// case: equality-precedence
return [false == 1 < 0, 1 < 2 == true, 3 > 2 > 1, 1 == 1 == 1, "a" != "b" == true];
// case: relational-precedence-with-in
const o = {a: 1}; return ["a" in o == true];
// case: logical-ops-values
return [1 && 2, 0 && 2, 1 || 2, 0 || 2, "" || "d", null && 1, !"", !"a", !!NaN, !0, ![]];
// case: logical-precedence
return [true || false && false, (true || false) && false, null ?? 0 || 5];
// case: ternary-chains
const f = n => n < 0 ? "neg" : n === 0 ? "zero" : n < 10 ? "small" : "big"; return [f(-1), f(0), f(5), f(50)];
// case: ternary-nested-in-args
return [1, 2].map(x => x > 1 ? x * 10 : x - 1);
// case: unary-minus-coercion
return [-"3", -"", -null, -undefined, -[], -[5], -true, -"x"];
// case: truthiness
return [[], {}, "0", " ", 0, -0, NaN, "", null, undefined, 0.0].map(x => x ? 1 : 0);
// case: comma-free-short-circuit
let n = 0; const f = () => { n += 1; return true; }; const r = false && f(); const s = true || f(); return [r, s, n];
// case: typeof-comparisons
return [typeof 1 === "number", typeof "" == "string", typeof null === "object"];
// case: void-operator
return void 0;
// case: comma-operator
return (1, 2);
// case: instanceof
return [[] instanceof Array, ({}) instanceof Object];
// case: bitwise-or
return 5 | 2;
// case: bitwise-and
return [5 & 3, 5 ^ 1, ~5, 1 << 3, -16 >> 2, -1 >>> 28];
// case: in-operator-array
return [0 in [1], 1 in [1], "0" in [1]];
// case: string-num-equality-whitespace
return ["\n" == 0, "\t1\n" == 1];
// case: bool-coercions
return [Boolean(""), Boolean("0"), Boolean([]), Boolean(0), Boolean(NaN), Boolean({})];
// case: null-undefined-arith
return [null + 1, undefined + 1, null * 5, null / 2];
// case: string-mult-concat-mix
return "3" * 2 + "1";
// case: compare-strings-numbers-mixed
return ["2" > "12", 2 > "12", "2" > 12];
