// case: stringify-basic
return JSON.stringify({a: [1, "x", true, null], b: {c: 1.5}});
// case: stringify-undefined-members
return JSON.stringify({a: undefined, b: () => 1, c: 1});
// case: stringify-undefined-in-array
return JSON.stringify([undefined, () => 1, NaN, Infinity, -Infinity]);
// case: stringify-top-level
return [JSON.stringify(undefined), JSON.stringify(null), JSON.stringify("s"), JSON.stringify(5), JSON.stringify(NaN), JSON.stringify(() => 1)];
// case: stringify-indent-number
return JSON.stringify({a: [1, {b: 2}], c: []}, null, 2);
// case: stringify-indent-string
return JSON.stringify({a: [1, 2]}, null, "--");
// case: stringify-indent-tab
return JSON.stringify({a: 1}, null, "\t");
// case: stringify-indent-big
return JSON.stringify({a: 1}, null, 20);
// case: stringify-empty-containers-indent
return JSON.stringify({a: {}, b: []}, null, 2);
// case: stringify-numbers
return JSON.stringify([1e21, 1e-7, 0.1, 1.5, -0, 100, 1e16, 123456789.123]);
// case: stringify-escapes
return JSON.stringify("a\"b\\c\nd\te\u0001f\u001fg\u007fh i");
// case: stringify-unicode
return JSON.stringify("héllo 日本 😀");
// case: stringify-replacer-array
return JSON.stringify({a: 1, b: 2, c: 3}, ["a", "c"]);
// case: stringify-replacer-fn
return JSON.stringify({a: 1, b: 2}, (k, v) => typeof v === "number" ? v * 10 : v);
// case: stringify-nested-key-order
return JSON.stringify({z: {b: 1, a: 2}, a: [{y: 1, x: 2}]});
// case: stringify-regex
return JSON.stringify({r: /x/});
// case: stringify-indent-zero
return JSON.stringify([1, 2], null, 0);
// case: stringify-indent-negative
return JSON.stringify([1, 2], null, -1);
// case: stringify-indent-empty-string
return JSON.stringify([1, 2], null, "");
// case: parse-basic
return JSON.parse('{"a": [1, 2.5, "x", null, true], "b": {"c": {}}}');
// case: parse-numbers
return JSON.parse('[1.0, 1e2, -0, 0.1, -0.0, 1E-7, 9007199254740991]');
// case: parse-numbers-beyond-2-53
// expect: divergent JSON integers beyond 2^53 keep their digits (Python templates print them exactly)
return JSON.parse('[12345678901234567890, 9007199254740993]');
// case: parse-numbers-beyond-2-53-arithmetic
return JSON.parse('[12345678901234567890, 9007199254740993]').map(x => x + 0);
// case: parse-errors
return JSON.parse("{a: 1}");
// case: parse-trailing-comma
return JSON.parse("[1,]");
// case: parse-string
return [JSON.parse('"x\\u0041"'), JSON.parse("5"), JSON.parse("null"), JSON.parse(" true ")];
// case: parse-duplicate-keys
return JSON.parse('{"a": 1, "b": 2, "a": 3}');
// case: roundtrip
const o = {list: [1, {x: "y"}], n: -5.25}; return JSON.stringify(JSON.parse(JSON.stringify(o))) === JSON.stringify(o);
// case: parse-reviver
return JSON.parse('{"a": 1}', (k, v) => v);
// case: stringify-toJSON
return JSON.stringify({toJSON: () => "custom"});
// case: stringify-nested-undefined
return JSON.stringify({a: {b: undefined}, c: [undefined]});
// case: stringify-number-keys
return JSON.stringify({10: "a", 2: "b", x: "c"});
// case: stringify-bool-string-array
return JSON.stringify([true, "true", [false]]);
// case: stringify-indent-nested
return JSON.stringify([[1, [2]], {a: {b: [3]}}], null, 1);
// case: stringify-large-int
return JSON.stringify(2 ** 60);
