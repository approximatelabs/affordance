// case: slice
const s = "hello world"; return [s.slice(1, 4), s.slice(-5), s.slice(3, 1), s.slice(-3, -1), s.slice(100), s.slice(), s.slice(2.7)];
// case: substring
const s = "hello"; return [s.substring(1, 3), s.substring(3, 1), s.substring(-2), s.substring(2), s.substring(NaN, 2), s.substring(1.9, 3.1)];
// case: indexOf
const s = "abcabc"; return [s.indexOf("b"), s.indexOf("b", 2), s.indexOf("z"), s.indexOf(""), s.indexOf("", 10), s.indexOf("c", -5)];
// case: lastIndexOf
const s = "abcabc"; return [s.lastIndexOf("b"), s.lastIndexOf("b", 3), s.lastIndexOf("z"), s.lastIndexOf(""), s.lastIndexOf("c", 0)];
// case: includes-startsWith
const s = "abcdef"; return [s.includes("cd"), s.includes("cd", 3), s.startsWith("cd", 2), s.endsWith("cd", 4), s.startsWith(""), s.endsWith("f")];
// case: split-basic
return ["a,b,,c".split(","), "abc".split(""), "abc".split(), "a b c".split(" ", 2), "".split(","), "".split(""), "a,b".split(",", 0)];
// case: split-edge
return ["aXbXc".split("X", -1), ",a,".split(","), "abc".split("abc"), "aaa".split("a")];
// case: replace-string
return ["a-b-c".replace("-", "+"), "a-b-c".replaceAll("-", "+"), "aaa".replace("", "x"), "abc".replace("b", "$&$&"), "abc".replace("b", "$'"), "abc".replace("b", "$`"), "abc".replace("b", "$$")];
// case: replace-fn
return "a-b-c".replace("-", (m, off, str) => `[${m}${off}${str.length}]`);
// case: replaceAll-fn
return "a-b-c".replaceAll("-", (m, off) => off);
// case: replaceAll-empty
return "abc".replaceAll("", "-");
// case: replace-regex
return ["a1b22c333".replace(/\d+/g, "#"), "John Smith".replace(/(\w+)\s(\w+)/, "$2, $1"), "aaa".replace(/a/, "b")];
// case: replace-regex-fn
return "x1y2".replace(/(\d)/g, (m, d, off) => `<${d * 2}@${off}>`);
// case: regex-match
return ["a1b22c333".match(/\d+/g), "abc".match(/z/), "abc".match(/(b)(c)/).length];
// case: regex-test-search
return [/^\d+$/.test("123"), /^\d+$/.test("12a"), "hello".search(/l+/), "hello".search(/z/)];
// case: regex-split
return ["a1b2c3".split(/\d/), "a, b,c".split(/\s*,\s*/), "abc".split(/(b)/)];
// case: trim
return ["  a b  ".trim(), "\t\nx\n".trim(), "  x ".trimStart(), " x  ".trimEnd(), " x ".trim(), "﻿x".trim()];
// case: case-convert
return ["Hello".toUpperCase(), "ÄÖÜ".toLowerCase(), "ß".toUpperCase(), "İ".toLowerCase().length];
// case: pad
return ["5".padStart(3, "0"), "abc".padEnd(6, "12"), "abc".padStart(2), "x".padStart(4), "x".padEnd(3, ""), "ab".padStart(7, "xyz")];
// case: repeat
return ["ab".repeat(3), "x".repeat(0)];
// case: repeat-negative
return "x".repeat(-1);
// case: at-charAt
const s = "abc"; return [s.at(0), s.at(-1), s.at(5), s.charAt(1), s.charAt(10), s.charAt(-1), s[1], s[10]];
// case: charCodeAt
return ["A".charCodeAt(0), "abc".charCodeAt(5), "é".charCodeAt(0), "abc".charCodeAt()];
// case: fromCharCode
return String.fromCharCode(72, 105);
// case: codePointAt
return "A".codePointAt(0);
// case: concat
return ["a".concat("b", 1, null, [2, 3]), "".concat()];
// case: template-literal
const a = 1, b = [1, 2], c = { x: 1 }; return `a=${a} b=${b} c=${c} n=${null} u=${undefined} t=${true} f=${1.50}`;
// case: template-nested
const xs = [1, 2]; return `${xs.map(x => `<${x}>`).join("")}`;
// case: template-escapes
return `a\tb\nc\\d\`e\${f}`;
// case: string-escapes
return ["\x41", "B", "\u{43}", "a\
b", "\v".length, "\b".length, "\f".length, "\0".length, "\'", "\""];
// case: string-compare
return ["a" < "b", "B" < "a", "10" < "9", "abc" < "abd", "ab" < "abc", "" < "a", "Z" > "a"];
// case: localeCompare
return ["a".localeCompare("b"), "b".localeCompare("a"), "a".localeCompare("a")];
// case: string-length-unicode
return ["héllo".length, "日本語".length, "日本語".slice(1)];
// case: string-iteration
const out = []; for (const ch of "abc") out.push(ch); return out;
// case: string-spread
return [..."héllo"];
// case: string-for-in
const out = []; for (const k in "ab") out.push(k); return out;
// case: string-methods-on-number-fail
const x = 5; return x.toUpperCase();
// case: string-lower-upper-chain
return " Mixed Case ".trim().toLowerCase().split(" ").map(w => w[0].toUpperCase() + w.slice(1)).join("");
// case: string-plus-number
return ["1" + 2 + 3, 1 + 2 + "3", "3" * "4", "10" / 2, "5" - 2, "a" * 1, "3" + -1];
// case: string-index-of-from-negative
return "aaa".indexOf("a", -10);
// case: string-to-string
return ["abc".toString(), String("x"), String(123), String([1, [2, 3]]), String({}), String(null), String(undefined), String(true)];
// case: matchAll
return [..."a1b2".matchAll(/\d/g)].map(m => m[0]);
// case: regex-exec
const m = /(\d+)-(\d+)/.exec("x 12-34 y"); return [m[0], m[1], m[2], m.length];
// case: regex-props
const r = /ab+/gi; return [r.source, r.flags, r.global, r.ignoreCase];
// case: regex-lastIndex
const r = /a/g; r.test("aaa"); return r.lastIndex;
// case: string-split-limit-regex
return "a1b2c3".split(/\d/, 2);
// case: trailing-whitespace-trim
return "x ".trim().length;
// case: regex-exec-loop-global
const re = /(\d)/g; const s = "a1b2c3"; const out = []; let m; while ((m = re.exec(s)) !== null) out.push([m[0], re.lastIndex]); return [out, re.lastIndex];
// case: regex-test-global-advances
const re = /o/g; return [re.test("foo"), re.lastIndex, re.test("foo"), re.lastIndex, re.test("foo"), re.lastIndex];
// case: regex-literal-fresh-each-evaluation
const f = () => /x/g; const a = f(); a.test("x"); const b = f(); return [a.lastIndex, b.lastIndex, a === b];
// case: regex-lastindex-assign
const re = /a/g; re.lastIndex = 2; return [re.exec("aaaa") && re.lastIndex, (re.lastIndex = 10, re.exec("aaaa")), re.lastIndex];
// case: regex-match-global-resets
const re = /a/g; re.lastIndex = 3; const m = "banana".match(re); return [m, re.lastIndex];
