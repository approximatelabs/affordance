// The official chat templates are Python (Jinja). Where one fails on a value of the wrong type (a
// TypeError), a view fails at the same point through these explicit checks.

// The Python type name of a value, for error messages.
function pythonTypeName(x) {
  if (x === undefined) return "Undefined";
  if (x === null) return "NoneType";
  if (typeof x === "string") return "str";
  if (typeof x === "boolean") return "bool";
  if (typeof x === "number") return Number.isInteger(x) ? "int" : "float";
  if (Array.isArray(x)) return "list";
  return "dict";
}

// Python `str + x`: x must be text (a string, or a token fragment).
function textOperand(x) {
  if (typeof x === "string" || isFragment(x)) return x;
  throw new TypeError(`can only concatenate str (not "${pythonTypeName(x)}") to str`);
}

// Python `d.items()` / Jinja `d|items`: only a mapping has item pairs.
function mappingEntries(x) {
  if (isPlainObject(x)) return Object.entries(x);
  throw new TypeError(`'${pythonTypeName(x)}' object has no attribute 'items'`);
}

// Jinja `x|length`: undefined is 0; strings, lists and mappings have a length; others fail.
function templateLength(x) {
  if (x === undefined) return 0;
  if (typeof x === "string" || Array.isArray(x)) return x.length;
  if (isPlainObject(x)) return Object.keys(x).length;
  throw new TypeError(`object of type '${pythonTypeName(x)}' has no len()`);
}

// Jinja `for x in xs`: a list's items, a string's characters, a mapping's keys; undefined is empty.
function templateItems(x) {
  if (x === undefined) return [];
  if (Array.isArray(x)) return x;
  if (typeof x === "string") return [...x];
  if (isPlainObject(x)) return Object.keys(x);
  throw new TypeError(`'${pythonTypeName(x)}' object is not iterable`);
}

// Jinja truthiness: empty lists and mappings are false too.
function templateTruthy(x) {
  if (Array.isArray(x)) return x.length > 0;
  if (isPlainObject(x)) return Object.keys(x).length > 0;
  return !!x;
}

export { mappingEntries, templateItems, templateLength, templateTruthy, textOperand };
