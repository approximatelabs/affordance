// case: int-arith
return [1 + 2, 7 - 10, 6 * 7, 7 / 2, 6 / 3, -7 % 3, 7 % -3, 5.5 % 2];
// case: float-sum
return 0.1 + 0.2;
// case: neg-zero-literal
return -0;
// case: neg-zero-div
return 1 / -0;
// case: neg-zero-mul
return 0 * -1;
// case: neg-zero-from-var
const z = 0; return -z;
// case: neg-zero-mod
return -4 % 2;
// case: neg-zero-round
return Math.round(-0.4);
// case: neg-zero-string
return String(-0) + "|" + (-0).toString() + "|" + `${-0}`;
// case: neg-zero-json
return JSON.stringify([-0, 0]);
// case: neg-zero-eq
return [-0 === 0, 0 == -0];
// case: infinity
return [1 / 0, -1 / 0, Infinity, -Infinity, Infinity - Infinity, 0 / 0];
// case: nan-compare
const n = NaN; return [n === n, n == n, n < 1, n > 1, NaN !== NaN];
// case: large-int-mul
return 123456789 * 123456789;
// case: large-int-add
return 9007199254740992 + 1;
// case: large-int-literal
return 9007199254740993;
// case: huge-literal
return 99999999999999999999;
// case: e21
return [1e21, 1e20, 123456789012345680000, 1e300, 2e-7, 1e-7, 0.000001, 1.5e-10];
// case: exp-literals
return [1e3, 2.5e2, 1E2, 5e-1];
// case: float-int-mul-precision
return 0.1 * 3;
// case: int-overflow-mul
return 4294967296 * 4294967296;
// case: exponent-operator
return 2 ** 3;
// case: hex-literal
return 0x1F;
// case: octal-binary-literal
return [0o17, 0b101];
// case: numeric-separator
return 1_000_000;
// case: leading-dot-literal
return .5 + 1;
// case: toFixed-basic
return [(3.14159).toFixed(2), (1).toFixed(2), (0).toFixed(0), (2.5).toFixed(0), (1.005).toFixed(2), (-1.5).toFixed(0)];
// case: toFixed-ties
return [(0.5).toFixed(0), (1.5).toFixed(0), (2.5).toFixed(0), (-2.5).toFixed(0), (1.45).toFixed(1), (8.345).toFixed(2)];
// case: toFixed-large
return [(1e21).toFixed(2), (123.456).toFixed(10), (1.23e-10).toFixed(5)];
// case: toFixed-neg-zero
return [(-0.0001).toFixed(2), (-0).toFixed(2)];
// case: toString-radix
return [(255).toString(16), (255).toString(2), (-255).toString(36), (0.5).toString(2)];
// case: toString-basic
return [(12).toString(), (1.5).toString(), (1e21).toString()];
// case: parseInt-basic
return [parseInt("42px"), parseInt("  -17 "), parseInt("abc"), parseInt(""), parseInt("3.9"), parseInt("+5")];
// case: parseInt-hex
return [parseInt("0x1f"), parseInt("0X10"), parseInt("-0x10"), parseInt("0x")];
// case: parseInt-radix
return [parseInt("ff", 16), parseInt("101", 2), parseInt("z", 36), parseInt("12", 1), parseInt("12", 37), parseInt("7", 0)];
// case: parseInt-numbers
return [parseInt(0.0000005), parseInt(1e21), parseInt(-0), parseInt(15.99)];
// case: parseInt-double-sign
return [parseInt("++1"), parseInt("-+1"), parseInt("--1")];
// case: parseInt-big
return parseInt("123456789012345678901234567890");
// case: parseInt-neg-zero
return parseInt("-0");
// case: parseFloat-basic
return [parseFloat("3.5kg"), parseFloat(".5"), parseFloat("-.5e2x"), parseFloat("1e"), parseFloat("abc"), parseFloat("  2  ")];
// case: parseFloat-infinity
return [parseFloat("Infinity"), parseFloat("-Infinityx"), parseFloat("inf")];
// case: parseFloat-misc
return [parseFloat("1.2.3"), parseFloat("0x10"), parseFloat("1e1000"), parseFloat("-0")];
// case: Number-conv
return [Number(""), Number("  12  "), Number("1e3"), Number("0x10"), Number("0X10"), Number("0b11"), Number("0o7"), Number("1_0"), Number("inf"), Number("Infinity"), Number("-Infinity")];
// case: Number-conv-objects
return [Number(null), Number(undefined), Number(true), Number([]), Number([5]), Number([1, 2]), Number({}), Number([[3]]), Number([null]), Number([undefined]), Number([true])];
// case: Number-conv-misc
return [Number("5."), Number(".5"), Number("+.5"), Number("-"), Number("1e"), Number("\n\t 7 \n")];
// case: Number-isInteger
return [Number.isInteger(5), Number.isInteger(5.0), Number.isInteger(5.5), Number.isInteger("5"), Number.isInteger(Infinity)];
// case: Number-isFinite-isNaN
return [Number.isFinite("5"), isFinite("5"), Number.isNaN("x"), isNaN("x"), Number.isNaN(NaN), isNaN(undefined), isNaN(null)];
// case: unary-plus
return +"3" + +true;
// case: number-tostring-floats
return [1/3, 2/3, 100/3, 1e15/3, 123.456, -0.000001, 0.1 * 0.1];
// case: integral-float-index
const xs = [10, 20, 30]; return [xs[1e0], xs[4 / 2], xs[1.5], "abc"[2.0]];
// case: string-index-key
const xs = [10, 20, 30]; return [xs["1"], xs["01"], "abc"["1"]];
// case: max-safe
return [9007199254740991 + 2, 2 * 9007199254740991];
// case: compound-assign
let x = 10; x += 5; x -= 3; x *= 2; x /= 4; return x;
// case: compound-mod
let x = 17; x %= 5; return x;
// case: increments
let i = 0; i++; i++; i--; ++i; return i;
// case: division-results
return [10 / 4, -10 / 4, 1 / 3 * 3, 9 / 3];
// case: math-basics
return [Math.max(), Math.min(), Math.max(1, "5", 3), Math.min(1, NaN), Math.abs(-5.5), Math.sign(-3), Math.trunc(-4.7), Math.floor(-4.5), Math.ceil(-4.5)];
// case: math-round
return [Math.round(2.5), Math.round(-2.5), Math.round(-2.6), Math.round(0.49999999999999994), Math.round(1.4999999999999998)];
// case: math-pow-sqrt
return [Math.pow(2, 10), Math.pow(2, 0.5), Math.sqrt(16), Math.sqrt(-1), Math.pow(10, 21), Math.pow(2, 53) + 1];
// case: math-logs
return [Math.log(1), Math.log10(1000), Math.log2(8), Math.exp(0), Math.log(-1)];
// case: math-constants
return [Math.PI, Math.E];
// case: math-trig
return [Math.sin(0), Math.cos(0), Math.atan2(1, 1), Math.hypot(3, 4), Math.cbrt(27)];
// case: math-int-results
return [Math.floor(5), Math.max(2, 3) / 2, Math.min(-0, 0)];
// case: pow-integer-exponents
// expect: divergent Math.pow and ** may differ from V8 in the last bit (V8 pow is not fdlibm)
const out = []; for (let k = -25; k <= 25; k++) out.push(10 ** k, 2 ** k, 1.1 ** k, (-3) ** k, 0.7 ** k, 123.456 ** (k % 7)); return out;
// case: pow-squares-cubes
// expect: divergent Math.pow and ** may differ from V8 in the last bit (V8 pow is not fdlibm)
const xs = [0.1, 0.3, 1.7, 2.9, 12.34, 99.9, 1e-5, 314.159, 7.77]; return xs.map(x => [x ** 2, x ** 3, x ** 0.5, x ** -1, x ** -2, Math.pow(x, 4)]);
// case: pow-fractional
// expect: divergent Math.pow and ** may differ from V8 in the last bit (V8 pow is not fdlibm)
const xs = [0.5, 2, 3, 10, 0.037, 239.08352316357195]; return xs.map(x => [x ** 0.25, x ** 1.5, x ** (1 / 3), x ** 0.2096011109339694, x ** -0.75]);
// case: math-logs-exact
const xs = [0.1, 0.5, 1, 2, 3, 7, 10, 100, 1000, 1e-10, 12345.678, 0.6359655421692878]; return xs.map(x => [Math.log(x), Math.log10(x), Math.log2(x), Math.log1p(x), Math.exp(x), Math.cbrt(x)]);
// case: math-hyperbolic
const xs = [0.1, 0.5, 1, 2, 3, 7, 10, 25, -0.3, -4]; return xs.map(x => [Math.sinh(x), Math.cosh(x), Math.tanh(x), Math.asinh(x), Math.acosh(x), Math.atanh(x / 30)]);
// case: math-trig-inverse
const xs = [0.1, 0.5, 0.9, -0.2, 1, 0]; return xs.map(x => [Math.asin(x), Math.acos(x), Math.atan(x), Math.atan2(x, 0.3), Math.tan(x * 3)]);
// case: parseInt-large-radix
return [parseInt("zzzzzzzzzzzzzzzzzzzz", 36), parseInt("123456701234567012345670", 8), parseInt("1".repeat(70), 2), parseInt("ffffffffffffffffffff", 16)];
// case: parseInt-large-radix-rounded
// oracle: x86_64 V8 folds parts in as `result * multiplier + part`; on arm64 it is compiled to a fused multiply-add, which rounds once, so Node itself differs by CPU. Ours is the unfused result everywhere.
return [parseInt("9007199254740992", 36), parseInt("777777777777777777777777", 9)];
// case: pad-huge
const r = []; for (const w of [Infinity, 1e9, 2 ** 31]) { try { r.push("ab".padStart(w, "x").length); } catch (e) { r.push(e.name); } } r.push("ab".padEnd(Infinity, "")); return r;
// case: number-tostring-ties
return [2 ** -25, 2 ** -26, 2 ** 60 + 2 ** 8, 5e-324, 1.7976931348623157e308, 0.1 + 0.7, 2 ** -1074 * 3];
