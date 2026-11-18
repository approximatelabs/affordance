"""Convert a Jinja chat template to an Affordance view (TSX).

    .venv/bin/python tools/jinja2tsx.py TEMPLATE.jinja "source name" > view.tsx

The view is a function from the template's inputs to the same text, statement for statement.
Jinja's and Python's meaning of each operation comes from views/common/jinja.tsx (`J`). The
template is parsed as Hugging Face parses it (trim_blocks, lstrip_blocks, loop controls, the
`generation` tag). A converted view is checked like every other one: `affordance check`
compares its token ids with the official template's on every case.
"""
import json, sys
import jinja2
from jinja2 import nodes
from jinja2.ext import Extension
from jinja2.sandbox import ImmutableSandboxedEnvironment


class Generation(Extension):
    """`{% generation %}..{% endgeneration %}` marks the assistant's text; it prints its body."""
    tags = {"generation"}

    def parse(self, parser):
        lineno = next(parser.stream).lineno
        body = parser.parse_statements(["name:endgeneration"], drop_needle=True)
        return nodes.Scope(body, lineno=lineno)


ENV = ImmutableSandboxedEnvironment(trim_blocks=True, lstrip_blocks=True, extensions=[Generation, "jinja2.ext.loopcontrols"])
GLOBALS = {"range", "namespace", "dict", "raise_exception", "strftime_now", "cycler", "joiner"}
BIN = {nodes.Add: "add", nodes.Sub: "sub", nodes.Mul: "mul", nodes.Div: "div", nodes.FloorDiv: "floordiv", nodes.Mod: "mod", nodes.Pow: "pow"}
CMP = {"eq": "J.eq({a}, {b})", "ne": "!J.eq({a}, {b})", "lt": "J.lt({a}, {b})", "lteq": "J.le({a}, {b})", "gt": "J.gt({a}, {b})",
       "gteq": "J.ge({a}, {b})", "in": "J.has({a}, {b})", "notin": "!J.has({a}, {b})"}


class Unsupported(Exception):
    pass


def lit(value):
    if value is None: return "null"
    if value is True: return "true"
    if value is False: return "false"
    if isinstance(value, str): return json.dumps(value)
    if isinstance(value, (int, float)): return repr(value)
    raise Unsupported(f"constant {value!r}")


def stored(body):
    """The names a scope's own statements assign (not those of the scopes inside it)."""
    out = []
    def target(t):
        if isinstance(t, nodes.Name): out.append(t.name)
        elif isinstance(t, nodes.Tuple): [target(x) for x in t.items]
    def walk(stmts):
        for s in stmts:
            if isinstance(s, nodes.Assign): target(s.target)
            elif isinstance(s, nodes.AssignBlock): target(s.target)
            elif isinstance(s, nodes.Macro): out.append(s.name)
            elif isinstance(s, nodes.If):
                walk(s.body); [walk(e.body) for e in s.elif_]; walk(s.else_)
    walk(body)
    return list(dict.fromkeys(out))


class Converter:
    def __init__(self):
        self.scopes = [{}]      # jinja name -> JS identifier, innermost last
        self.count = 0
        self.lines = []
        self.depth = 1
        self.loops = 0

    # ---- names ----
    def fresh(self, name):
        self.count += 1
        return f"{name}_{self.count}"

    def lookup(self, name):
        for scope in reversed(self.scopes):
            if name in scope: return scope[name]
        # A name the template never assigns: one of its inputs.
        ident = self.scopes[0][name] = f"{name}_0"
        self.inputs.append(name)
        return ident

    def emit(self, text):
        self.lines.append("  " * self.depth + text)

    def open_scope(self, body, extra=()):
        """Enter a Jinja scope: its assigned names start as the enclosing values."""
        scope = {}
        decls = []
        for name in stored(body):
            if name in extra: continue
            outer = self.lookup(name)
            scope[name] = ident = self.fresh(name)
            decls.append(f"let {ident} = {outer};")
        self.scopes.append(scope)
        for d in decls: self.emit(d)
        return scope

    # ---- expressions ----
    def args(self, node):
        parts = [self.expr(a) for a in node.args]
        if node.dyn_args is not None: parts.append(f"...J.iter({self.expr(node.dyn_args)})")
        named = [f"{json.dumps(k.key)}: {self.expr(k.value)}" for k in node.kwargs]
        if node.dyn_kwargs is not None: named.append(f"...{self.expr(node.dyn_kwargs)}")
        if named: parts.append("J.kw({" + ", ".join(named) + "})")
        return parts

    def expr(self, n):
        e = self.expr
        if isinstance(n, nodes.Const): return lit(n.value)
        if isinstance(n, nodes.TemplateData): return lit(n.data)
        if isinstance(n, nodes.Name):
            if n.name in GLOBALS and not any(n.name in s for s in self.scopes): return "J." + n.name
            return self.lookup(n.name)
        if isinstance(n, nodes.NSRef): return f"J.attr({self.lookup(n.name)}, {json.dumps(n.attr)})"
        if isinstance(n, nodes.Getattr): return f"J.attr({e(n.node)}, {json.dumps(n.attr)})"
        if isinstance(n, nodes.Getitem):
            if isinstance(n.arg, nodes.Slice):
                s = n.arg
                return f"J.slice({e(n.node)}, {e(s.start) if s.start else 'undefined'}, {e(s.stop) if s.stop else 'undefined'}, {e(s.step) if s.step else 'undefined'})"
            return f"J.item({e(n.node)}, {e(n.arg)})"
        if isinstance(n, nodes.Call):
            return "J.call(" + ", ".join([e(n.node)] + self.args(n)) + ")"
        if isinstance(n, nodes.Filter):
            if n.node is None: raise Unsupported("a filter with no value")
            return "J.filter(" + ", ".join([json.dumps(n.name), e(n.node)] + self.args(n)) + ")"
        if isinstance(n, nodes.Test):
            v = e(n.node)
            if n.name == "defined": return f"({v} !== undefined)"
            if n.name == "undefined": return f"({v} === undefined)"
            if n.name == "none": return f"({v} === null)"
            return "J.test(" + ", ".join([json.dumps(n.name), v] + self.args(n)) + ")"
        if isinstance(n, nodes.Not): return f"!J.t({e(n.node)})"
        if isinstance(n, nodes.And): return f"((v) => J.t(v) ? {e(n.right)} : v)({e(n.left)})"
        if isinstance(n, nodes.Or): return f"((v) => J.t(v) ? v : {e(n.right)})({e(n.left)})"
        if isinstance(n, nodes.CondExpr):
            return f"(J.t({e(n.test)}) ? {e(n.expr1)} : {e(n.expr2) if n.expr2 is not None else 'undefined'})"
        if isinstance(n, nodes.Compare):
            if len(n.ops) == 1:
                return "(" + CMP[n.ops[0].op].format(a=e(n.expr), b=e(n.ops[0].expr)) + ")"
            # a < b < c: each middle value is evaluated once
            names = [f"c{i}" for i in range(len(n.ops) + 1)]
            tests = " && ".join(CMP[op.op].format(a=names[i], b=names[i + 1]) for i, op in enumerate(n.ops))
            return f"(({', '.join(names)}) => {tests})({', '.join([e(n.expr)] + [e(op.expr) for op in n.ops])})"
        if type(n) in BIN: return f"J.{BIN[type(n)]}({e(n.left)}, {e(n.right)})"
        if isinstance(n, nodes.Concat): return "J.concat(" + ", ".join(e(x) for x in n.nodes) + ")"
        if isinstance(n, nodes.Neg): return f"J.neg({e(n.node)})"
        if isinstance(n, nodes.Pos): return e(n.node)
        if isinstance(n, (nodes.List, nodes.Tuple)): return "[" + ", ".join(e(x) for x in n.items) + "]"
        if isinstance(n, nodes.Dict):
            if all(isinstance(p.key, nodes.Const) and isinstance(p.key.value, str) for p in n.items):
                return "({" + ", ".join(f"{lit(p.key.value)}: {e(p.value)}" for p in n.items) + "})"
            if any(isinstance(p.key, nodes.Const) and not isinstance(p.key.value, str) for p in n.items):
                # keys that are not strings: a Map, which keeps each key's own type
                return "J.pairs([" + ", ".join(f"[{e(p.key)}, {e(p.value)}]" for p in n.items) + "])"
            return "({" + ", ".join(f"[{e(p.key)}]: {e(p.value)}" for p in n.items) + "})"
        if isinstance(n, nodes.MarkSafe): return e(n.expr)
        raise Unsupported(type(n).__name__)

    # ---- statements ----
    def assign(self, target, value):
        if isinstance(target, nodes.Name): self.emit(f"{self.lookup(target.name)} = {value};")
        elif isinstance(target, nodes.NSRef): self.emit(f"J.setattr({self.lookup(target.name)}, {json.dumps(target.attr)}, {value});")
        elif isinstance(target, nodes.Tuple):
            names = []
            for t in target.items:
                if not isinstance(t, nodes.Name): raise Unsupported("a nested assignment target")
                names.append(self.lookup(t.name))
            self.emit(f"[{', '.join(names)}] = J.unpack({value}, {len(names)});")
        else: raise Unsupported("assignment to " + type(target).__name__)

    def block(self, body):
        for s in body: self.stmt(s)

    def captured(self, body, scoped=True):
        """The body's output as a string expression (a function called at once)."""
        self.emit("(() => {"); self.depth += 1
        self.emit("const out = [];")
        if scoped: self.open_scope(body)
        self.block(body)
        if scoped: self.scopes.pop()
        self.emit('return out.join("");')
        self.depth -= 1
        return "})()"

    def stmt(self, s):
        e = self.expr
        if isinstance(s, nodes.Output):
            parts = [lit(x.data) if isinstance(x, nodes.TemplateData) else f"J.str({e(x)})" for x in s.nodes]
            if parts: self.emit(f"out.push({', '.join(parts)});")
        elif isinstance(s, nodes.If):
            self.emit(f"if (J.t({e(s.test)})) {{"); self.depth += 1; self.block(s.body); self.depth -= 1
            for el in s.elif_:
                self.emit(f"}} else if (J.t({e(el.test)})) {{"); self.depth += 1; self.block(el.body); self.depth -= 1
            if s.else_:
                self.emit("} else {"); self.depth += 1; self.block(s.else_); self.depth -= 1
            self.emit("}")
        elif isinstance(s, nodes.For):
            if s.recursive: raise Unsupported("a recursive loop")
            self.loops += 1
            n = self.loops
            targets = [s.target] if isinstance(s.target, nodes.Name) else list(s.target.items)
            if not all(isinstance(t, nodes.Name) for t in targets): raise Unsupported("a nested loop target")
            self.emit("{"); self.depth += 1
            self.emit(f"let _items{n} = J.iter({e(s.iter)});")
            scope = {t.name: self.fresh(t.name) for t in targets}
            scope["loop"] = f"_loop{n}"
            idents = [scope[t.name] for t in targets]
            bind = f"let {idents[0]} = _it{n};" if isinstance(s.target, nodes.Name) else f"let [{', '.join(idents)}] = J.unpack(_it{n}, {len(idents)});"
            if s.test is not None:
                self.scopes.append(dict(scope))
                self.emit(f"_items{n} = _items{n}.filter(_it{n} => {{ {bind} return J.t({e(s.test)}); }});")
                self.scopes.pop()
            self.emit(f"const _loop{n} = J.loop(_items{n});")
            if s.else_:
                self.emit(f"if (_items{n}.length === 0) {{"); self.depth += 1
                self.open_scope(s.else_); self.block(s.else_); self.scopes.pop()
                self.depth -= 1; self.emit("}")
            self.emit(f"for (let _i{n} = 0; _i{n} < _items{n}.length; _i{n}++) {{"); self.depth += 1
            self.emit(f"_loop{n}.index0 = _i{n}; const _it{n} = _items{n}[_i{n}]; {bind}")
            self.scopes.append(scope)
            inner = self.open_scope(s.body, extra=set(scope))
            # a loop variable the body assigns again is the same variable
            for name in stored(s.body):
                if name in scope: inner[name] = scope[name]
            self.block(s.body)
            self.scopes.pop(); self.scopes.pop()
            self.depth -= 1; self.emit("}")
            self.depth -= 1; self.emit("}")
        elif isinstance(s, nodes.Assign):
            self.assign(s.target, e(s.node))
        elif isinstance(s, nodes.AssignBlock):
            start = len(self.lines)
            tail = self.captured(s.body)
            value = "__captured__"
            if s.filter is not None:
                f = s.filter
                chain = []
                while isinstance(f, nodes.Filter):
                    chain.append(f); f = f.node
                for f in reversed(chain):
                    value = "J.filter(" + ", ".join([json.dumps(f.name), value] + self.args(f)) + ")"
            body = self.lines[start:]; del self.lines[start:]
            self.emit("{"); self.depth += 1
            self.emit("const captured = " + body[0].strip())
            self.lines.extend(body[1:])
            self.emit(tail + ";")
            self.assign(s.target, value.replace("__captured__", "captured"))
            self.depth -= 1; self.emit("}")
        elif isinstance(s, nodes.Macro):
            ident = self.lookup(s.name)
            params = {a.name: self.fresh(a.name) for a in s.args}
            caller = self.fresh("caller")
            self.emit(f"{ident} = (...args) => {{"); self.depth += 1
            self.emit("const [p, k] = J.args(args);")
            self.emit("const out = [];")
            self.emit(f"const {caller} = k.caller;")
            first_default = len(s.args) - len(s.defaults)
            self.scopes.append({})
            for i, a in enumerate(s.args):
                default = e(s.defaults[i - first_default]) if i >= first_default else "undefined"
                self.emit(f"let {params[a.name]} = p.length > {i} ? p[{i}] : {json.dumps(a.name)} in k ? k[{json.dumps(a.name)}] : {default};")
                self.scopes[-1][a.name] = params[a.name]
            self.scopes[-1]["caller"] = caller
            self.open_scope(s.body, extra=set(params))
            self.block(s.body)
            self.scopes.pop(); self.scopes.pop()
            self.emit('return out.join("");')
            self.depth -= 1; self.emit("};")
        elif isinstance(s, nodes.CallBlock):
            if s.args: raise Unsupported("a call block with arguments")
            start = len(self.lines)
            tail = self.captured(s.body)
            body = self.lines[start:]; del self.lines[start:]
            self.emit("{"); self.depth += 1
            self.emit("const caller = () => " + body[0].strip())
            self.lines.extend(body[1:])
            self.emit(tail + ";")
            call = s.call
            parts = [e(call.node)] + [e(a) for a in call.args]
            kws = [f"{json.dumps(k.key)}: {e(k.value)}" for k in call.kwargs] + ["caller: caller"]
            self.emit("out.push(J.str(J.call(" + ", ".join(parts + ["J.kw({" + ", ".join(kws) + "})"]) + ")));")
            self.depth -= 1; self.emit("}")
        elif isinstance(s, nodes.FilterBlock):
            start = len(self.lines)
            tail = self.captured(s.body)
            body = self.lines[start:]; del self.lines[start:]
            self.emit("{"); self.depth += 1
            self.emit("const captured = " + body[0].strip())
            self.lines.extend(body[1:])
            self.emit(tail + ";")
            value, chain, f = "captured", [], s.filter
            while isinstance(f, nodes.Filter):
                chain.append(f); f = f.node
            for f in reversed(chain):
                value = "J.filter(" + ", ".join([json.dumps(f.name), value] + self.args(f)) + ")"
            self.emit(f"out.push(J.str({value}));")
            self.depth -= 1; self.emit("}")
        elif isinstance(s, (nodes.Scope, nodes.With)):
            self.emit("{"); self.depth += 1
            if isinstance(s, nodes.With):
                scope = {}
                values = [e(v) for v in s.values]
                for t, v in zip(s.targets, values):
                    if not isinstance(t, nodes.Name): raise Unsupported("a with target")
                    scope[t.name] = self.fresh(t.name)
                    self.emit(f"let {scope[t.name]} = {v};")
                self.scopes.append(scope)
                self.open_scope(s.body, extra=set(scope)); self.block(s.body); self.scopes.pop(); self.scopes.pop()
            else:
                # the generation tag: it prints its body in the scope around it
                self.block(s.body)
            self.depth -= 1; self.emit("}")
        elif isinstance(s, nodes.Break): self.emit("break;")
        elif isinstance(s, nodes.Continue): self.emit("continue;")
        elif isinstance(s, nodes.ExprStmt): self.emit(e(s.node) + ";")
        else: raise Unsupported(type(s).__name__)

    def function(self, source, name):
        """The template as one function `name(ctx)`."""
        ast = ENV.parse(source)
        self.inputs = []
        top = self.scopes[0]
        for var in stored(ast.body):
            top[var] = f"{var}_0"; self.inputs.append(var)
        self.block(ast.body)
        head = [f"function {name}(ctx) {{", "  const out = [];"]
        for input in dict.fromkeys(self.inputs):
            # Hugging Face always passes `tools` and `documents`: None when there are none.
            default = " ?? null" if input in ("tools", "documents") else ""
            head.append(f"  let {input}_0 = ctx[{json.dumps(input)}]{default};")
        return "\n".join(head + self.lines + ['  return out.join("");', "}", ""])


def convert(source, origin, runtime="../common/jinja.tsx", prepare=None, imports=""):
    """`source`: the template, or a model's named templates as a dict (`default`, `tool_use`):
    Hugging Face uses `tool_use` when tools are passed and `default` otherwise.
    `prepare`: a JavaScript expression over `ctx` giving the inputs the template sees, for a
    model whose own code changes them before its template runs (Kimi sorts the tools' keys);
    `imports`: import lines that expression needs."""
    head = f"// Converted from the chat template of {origin} by tools/jinja2tsx.py. Regenerate it; do not edit.\nimport {{ J }} from \"{runtime}\";\n{imports}\n"
    if isinstance(source, str):
        if prepare is None:
            return head + "export default " + Converter().function(source, "Main")
        body, pick = Converter().function(source, "Template"), "Template(ctx)"
    else:
        names = {key: "".join(part.capitalize() for part in key.replace("-", "_").split("_")) + "Template" for key in source}
        body = "\n".join(Converter().function(text, names[key]) for key, text in source.items())
        if "tool_use" in source and "default" in source:
            pick = f"ctx.tools != null ? {names['tool_use']}(ctx) : {names['default']}(ctx)"
        elif "default" in source: pick = f"{names['default']}(ctx)"
        else: raise Unsupported("named templates without a default")
    if prepare is not None:
        return head + body + f"\n// What the model's own code does to the inputs before its template runs.\nexport default function Main(given) {{\n  const ctx = {prepare};\n  return {pick};\n}}\n"
    return head + body + f"\nexport default function Main(ctx) {{\n  return {pick};\n}}\n"


if __name__ == "__main__":
    print(convert(open(sys.argv[1]).read(), sys.argv[2] if len(sys.argv) > 2 else sys.argv[1]), end="")
