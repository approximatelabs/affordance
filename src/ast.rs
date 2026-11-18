//! Syntax tree for the view language (TSX with JavaScript semantics).

use crate::value::Value;
use std::rc::Rc;

pub type Sym = Rc<str>;

/// Binary operators, with JavaScript semantics (`jsvalue.rs`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    /// `==` / `!=`: loose equality.
    Eq,
    Ne,
    /// `===` / `!==`.
    StrictEq,
    StrictNe,
    Lt,
    Le,
    Gt,
    Ge,
    /// `k in obj`: obj has the key (an array: the index).
    In,
    /// `**`.
    Pow,
    /// `& | ^ << >> >>>` on 32-bit integers.
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    UShr,
    /// `x instanceof C` for the built-in constructors (`Error`s, `Array`, `Object`).
    InstanceOf,
}

/// Prefix operators besides `-`, `!` and `typeof`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOp {
    /// `+x`: ToNumber.
    Plus,
    /// `~x`.
    BitNot,
    /// `void x`.
    Void,
}

/// What an assignment does with the target's old value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AssignOp {
    /// `=`.
    Set,
    /// `+=`, `-=`, `**=`, `|=` ...
    Bin(BinOp),
    /// `&&=`, `||=`, `??=`: assign only when the old value is truthy / falsy / nullish.
    And,
    Or,
    Nullish,
}

/// A binding pattern (`const`/`let` declarations, parameters, `for` variables).
#[derive(Debug, Clone)]
pub enum Pattern {
    Name(Sym),
    /// `[a, , b = 1, ...rest]`: holes are `None`.
    Array(Vec<Option<(Pattern, Option<Expr>)>>, Option<Box<Pattern>>),
    /// `{a, b: c, [k]: d = 1, ...rest}`: (key, pattern, default).
    Object(Vec<(Expr, Pattern, Option<Expr>)>, Option<Sym>),
}

impl Pattern {
    /// The names a pattern binds, in order.
    pub fn names(&self, out: &mut Vec<Sym>) {
        match self {
            Pattern::Name(n) => out.push(n.clone()),
            Pattern::Array(items, rest) => {
                for (p, _) in items.iter().flatten() {
                    p.names(out)
                }
                if let Some(r) = rest {
                    r.names(out)
                }
            }
            Pattern::Object(items, rest) => {
                for (_, p, _) in items {
                    p.names(out)
                }
                if let Some(r) = rest {
                    out.push(r.clone())
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum LambdaBody {
    Expr(Expr),
    Block(Vec<Stmt>),
}

#[derive(Debug)]
pub struct Lambda {
    pub(crate) code: std::cell::OnceCell<Rc<crate::bytecode::Code>>,
    /// Lexical file scope, also retained when a closure crosses a call or replay boundary.
    pub module: Sym,
    pub params: Vec<Param>,
    /// `({a, b = 1}) => ...` destructures one props object.
    pub props: Option<Vec<Param>>,
    pub body: LambdaBody,
    /// `async (..) => ..`: inside a handler, a call returns a promise (see `effects.rs`).
    pub is_async: bool,
    /// Every name the body (with its parameters' defaults and nested functions) mentions: the
    /// enclosing locals of these names are what the closure captures.
    pub captures: Vec<Sym>,
    /// What its declaration says (a tool reads it: `nameOf`, `docOf`, `schemaOf`): the `const`
    /// it is bound to, the JSDoc before it, the type of its props parameter.
    pub name: Option<Sym>,
    pub doc: Option<Rc<str>>,
    pub props_type: Option<Rc<crate::tsx::TsType>>,
    /// `const f = (..) => .. f(..) ..`: the name the function calls itself by, bound to itself
    /// when it is called instead of captured (a closure that captured its own variable would hold
    /// itself, and never be freed).
    pub self_name: Option<Sym>,
    /// An arrow function (`=>`): its `this` is the enclosing one. A `function` or a method binds
    /// `this` when called (the receiver of `o.f()`, else undefined).
    pub arrow: bool,
    /// A non-arrow function whose body mentions `this` (only those bind it).
    pub uses_this: bool,
    /// A method whose body mentions `super` (it is bound to the method's class when called).
    pub uses_super: bool,
}

/// A piece of a template literal: cooked literal text (pre-split on special tokens for tagged
/// prompt literals) or an interpolated expression.
#[derive(Debug, Clone)]
pub enum TplPart {
    Lit(Rc<str>, Vec<Node>),
    Expr(Expr),
}

#[derive(Debug, Clone)]
pub enum Expr {
    /// Internal saved operand; never produced by the source parser.
    Temp(usize),
    Lit(Value),
    Var(Sym),
    List(Vec<Expr>),
    Map(Vec<(Expr, Expr)>),
    Attr(Box<Expr>, Sym),
    Private(Box<Expr>, Sym),
    PrivateMethod(Box<Expr>, Sym, Vec<Expr>),
    Index(Box<Expr>, Box<Expr>),
    Call(Sym, Vec<Expr>),
    Method(Box<Expr>, Sym, Vec<Expr>),
    Neg(Box<Expr>),
    Not(Box<Expr>),
    Bin(BinOp, Box<Expr>, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Nullish(Box<Expr>, Box<Expr>),
    Cond(Box<Expr>, Box<Expr>, Box<Expr>),
    Lambda(Rc<Lambda>),
    /// `await e`: yield and resume through the instance microtask queue.
    Await(Box<Expr>),
    /// A JSX element/fragment used as a value (evaluates to a fragment).
    Markup(Rc<Vec<Node>>),
    /// `` `a${x}b` `` (string) or ``p`a${x}b` `` (prompt fragment).
    Template {
        tagged: bool,
        parts: Vec<TplPart>,
    },
    /// A condition (`if`, `while`, `?:`, `!`): ToBoolean, as a bool.
    Truthy(Box<Expr>),
    /// A template-literal part: ToString.
    ToString(Box<Expr>),
    /// `for (const k in v)`: the keys of v.
    Keys(Box<Expr>),
    Typeof(Box<Expr>),
    /// `...x` inside an array literal (its items), an object literal (its entries) or a call.
    Spread(Box<Expr>),
    /// `a?.b`, `a?.b(args)`, `a?.(args)`, `a?.[i]`: when `a` is null or undefined, the whole
    /// enclosing `Chain` is undefined.
    OptAttr(Box<Expr>, Sym),
    OptMethod(Box<Expr>, Sym, Vec<Expr>),
    OptApply(Box<Expr>, Vec<Expr>),
    OptIndex(Box<Expr>, Box<Expr>),
    /// An optional chain (`a?.b.c`): a short-circuit anywhere in it makes it undefined.
    Chain(Box<Expr>),
    /// `f(a)(b)`, `(x => x)(1)`: a call of a computed function value.
    Apply(Box<Expr>, Vec<Expr>),
    Unary(UnaryOp, Box<Expr>),
    /// `t = e`, `t += e`, `t ??= e` as an expression (its value is the value assigned).
    Assign(Box<Target>, AssignOp, Box<Expr>),
    /// `t++` / `++t` / `t--` / `--t`: (target, +1 or -1, prefix).
    Update(Box<Target>, i8, bool),
    /// A mutating method on an assignable receiver (`xs.push(v)`, `xs.sort()`, `delete o.k`,
    /// `Object.assign(o, ..)`): the receiver is rebound to the updated value (values are
    /// immutable underneath) and the method's result is the expression's value.
    Mutate(Box<Target>, Sym, Vec<Expr>),
    /// `(a, b)`: both, in order; the value is the last.
    Comma(Box<Expr>, Box<Expr>),
    /// `class A extends B { .. }`: a class value.
    Class(Rc<ClassDef>),
    /// `new C(args)`: a class (or a built-in constructor, by name).
    New(Sym, Vec<Expr>),
}

/// A class's members as written. Methods, accessors and field initializers are functions that
/// bind `this` (a field's initializer runs once per instance, with `this` the new object).
#[derive(Debug, Clone)]
pub struct ClassDef {
    pub name: Option<Sym>,
    pub parent: Option<Expr>,
    pub ctor: Option<Rc<Lambda>>,
    pub members: Vec<Member>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MemberKind {
    Method,
    Get,
    Set,
    Field,
}

#[derive(Debug, Clone)]
pub struct Member {
    pub kind: MemberKind,
    pub is_static: bool,
    /// The name; a private member's starts with `#`.
    pub key: Sym,
    pub private: bool,
    /// A method's or accessor's function, a field's initializer (none: undefined).
    pub value: Option<Rc<Lambda>>,
}

#[derive(Debug, Clone)]
pub enum Target {
    /// Internal saved receiver.
    Temp(usize),
    Var(Sym),
    Attr(Box<Target>, Sym),
    Private(Box<Target>, Sym),
    Index(Box<Target>, Expr),
    /// The base of a member assignment that is not itself a place (`f().x = v`): evaluated once.
    Expr(Box<Expr>),
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let(Sym, Expr),
    Assign(Target, Expr),
    If(Vec<(Expr, Vec<Stmt>)>, Option<Vec<Stmt>>),
    /// `for (const p of xs)` (or `in`: the iterable is wrapped in `Expr::Keys`).
    For(Pattern, Expr, Vec<Stmt>),
    Return(Expr),
    /// `break` / `break label`.
    Break(Option<Sym>),
    Continue(Option<Sym>),
    Expr(Expr),
    /// `const [a, {b}] = e;`: binds new names. `init`: the names were hoisted (`Declare`) and
    /// this initializes them.
    LetPattern(Pattern, Expr, bool),
    AssignPattern(Vec<Target>, Expr),
    /// Block entry: the block's declarations, uninitialized, so closures created before a
    /// declaration (and recursive ones) see it, as in JavaScript.
    Declare(Vec<Sym>),
    /// `const x = e` / `let x = e` of a hoisted name: initializes the declared slot.
    Init(Sym, Expr),
    /// `{ ... }`: a nested block scope.
    Block(Vec<Stmt>),
    /// `label: loop`.
    Labeled(Sym, Box<Stmt>),
    /// `do { body } while (cond)`.
    DoWhile(Vec<Stmt>, Expr),
    /// switch with fallthrough; (case values, body); default index into the arms.
    Switch(Expr, Vec<(Vec<Expr>, Vec<Stmt>)>, Option<usize>),
    /// Source line of the next statement (TSX), for error locations. No effect on rendering.
    Line(u32),
    /// `while (cond) { body }`.
    While(Expr, Vec<Stmt>),
    /// `for (init; cond; update) { body }`.
    ForC(Vec<Stmt>, Expr, Vec<Stmt>, Vec<Stmt>),
    /// `try { body } catch (e) { handler } finally { cleanup }`.
    Try(Vec<Stmt>, Option<Sym>, Option<Vec<Stmt>>, Option<Vec<Stmt>>),
    /// `throw e`.
    Throw(Expr),
}

#[derive(Debug, Clone)]
pub enum AttrVal {
    Str(Rc<str>),
    Expr(Expr),
    Bare,
    /// `{...obj}`: every entry of an object, in order; a later attribute of the same name wins.
    Spread(Expr),
}

#[derive(Debug, Clone)]
pub struct Attr {
    pub name: Sym,
    pub value: AttrVal,
}

#[derive(Debug, Clone)]
pub enum Node {
    /// Literal text.
    Text(Rc<str>),
    /// A special token, resolved to an id when the program is bound to a tokenizer.
    /// `id` is filled in at bind time for static strings.
    Special {
        text: Rc<str>,
        id: std::cell::Cell<Option<u32>>,
    },
    /// `<sp t={expr}/>` with a dynamic token string, or `<tok id={expr}/>`.
    DynSpecial(Expr),
    TokId(Expr),
    /// Built-in element: box, span, aff, img, text.
    Builtin {
        tag: Sym,
        attrs: Vec<Attr>,
        children: Vec<Node>,
    },
    /// `{expr}` child with React rules (arrays splice; true/false/null/undefined render nothing).
    Child(Expr),
    /// `<Name a={..}>children</Name>` calling a function component with a props object.
    Call {
        name: Sym,
        attrs: Vec<Attr>,
        children: Vec<Node>,
    },
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: Sym,
    pub default: Option<Expr>,
    /// `...name`: the remaining arguments, as an array (last parameter only).
    pub rest: bool,
}

/// A source location: file and 1-based line.
#[derive(Debug, Clone)]
pub struct Loc {
    pub file: Sym,
    pub line: u32,
}

#[derive(Debug)]
pub struct Func {
    pub(crate) code: std::cell::OnceCell<Rc<crate::bytecode::Code>>,
    pub name: Sym,
    pub params: Vec<Param>,
    /// the function takes one props object, destructured into these names.
    pub props: Option<Vec<Param>>,
    pub body: Vec<Stmt>,
    /// `async function f(..) {..}`: inside a handler, a call returns a promise (see `effects.rs`).
    pub is_async: bool,
    /// Declaration site, for error locations.
    pub loc: Loc,
    /// The JSDoc before its declaration, and the type of its props parameter (`schemaOf`).
    pub doc: Option<Rc<str>>,
    pub props_type: Option<Rc<crate::tsx::TsType>>,
    /// As `Lambda::arrow` / `uses_this` (a `function` declaration binds `this`; a top-level
    /// `const f = () => ..` does not).
    pub arrow: bool,
    pub uses_this: bool,
}

/// How a part of a plain expression is held once it has been evaluated, while the parts after
/// it are: as its value; as its text (a computed key, a template's hole); as a copy of the items
/// it spreads into a list; or as a copy of the fields it spreads into an object. The copy and
/// the text are taken when the part is evaluated, as in JavaScript, so later parts cannot change
/// them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Capture {
    Value,
    Text,
    Items,
    Fields,
}

/// One thing directly inside an expression.
pub(crate) enum Child<'a> {
    Expr(&'a Expr),
    Target(&'a Target),
    Nodes(&'a [Node]),
}

// The parts of a plain expression, written once for shared and for mutable access.
macro_rules! plain_parts {
    ($name:ident $(, $m:ident)?) => {
        /// The parts of a plain expression, in the order they are evaluated. A plain expression
        /// evaluates each of its parts once, left to right, and then itself (`-a`, `a + b`,
        /// `a[b]`, `[a, ...b]`, `{[k]: v, ...o}`, a template). `None` for every other form: one
        /// that decides whether a part runs (`&&`, `?:`, `?.`), calls, assigns, or awaits.
        ///
        /// This is the one statement of that order. The interpreter of an `await`ing function
        /// evaluates the parts into temporaries and then the form (`bytecode.rs`), and every
        /// walk over the tree reads an expression's children from here (`Expr::children`).
        pub(crate) fn $name(& $($m)? self) -> Option<Vec<(Capture, & $($m)? Expr)>> {
            let mut out = vec![];
            match self {
                Expr::Neg(e)
                | Expr::Not(e)
                | Expr::Truthy(e)
                | Expr::ToString(e)
                | Expr::Keys(e)
                | Expr::Typeof(e)
                | Expr::Spread(e)
                | Expr::Unary(_, e)
                | Expr::Attr(e, _)
                | Expr::Private(e, _) => out.push((Capture::Value, & $($m)? **e)),
                Expr::Bin(_, a, b) | Expr::Index(a, b) | Expr::Comma(a, b) => {
                    out.push((Capture::Value, & $($m)? **a));
                    out.push((Capture::Value, & $($m)? **b));
                }
                Expr::List(items) => {
                    for item in items {
                        out.push(match item {
                            Expr::Spread(e) => (Capture::Items, & $($m)? **e),
                            e => (Capture::Value, e),
                        });
                    }
                }
                Expr::Map(entries) => {
                    for (key, value) in entries {
                        match key {
                            Expr::Spread(e) => out.push((Capture::Fields, & $($m)? **e)),
                            key => {
                                out.push((Capture::Text, key));
                                out.push((Capture::Value, value));
                            }
                        }
                    }
                }
                Expr::Template { tagged, parts } => {
                    let capture = if *tagged { Capture::Value } else { Capture::Text };
                    for part in parts {
                        if let TplPart::Expr(e) = part {
                            out.push((capture, e));
                        }
                    }
                }
                _ => return None,
            }
            Some(out)
        }
    };
}

impl Expr {
    plain_parts!(plain_parts);
    plain_parts!(plain_parts_mut, mut);

    /// Everything directly inside this expression, in source order: what any walk over the
    /// tree visits. A function's body is its own tree and is not a child.
    pub(crate) fn children<'a>(&'a self, f: &mut dyn FnMut(Child<'a>)) {
        fn exprs<'a>(es: &'a [Expr], f: &mut dyn FnMut(Child<'a>)) {
            es.iter().for_each(|e| f(Child::Expr(e)))
        }
        match self {
            Expr::Lit(_) | Expr::Temp(_) | Expr::Var(_) | Expr::Lambda(_) => {}
            Expr::Neg(_)
            | Expr::Not(_)
            | Expr::Truthy(_)
            | Expr::ToString(_)
            | Expr::Keys(_)
            | Expr::Typeof(_)
            | Expr::Spread(_)
            | Expr::Unary(..)
            | Expr::Attr(..)
            | Expr::Private(..)
            | Expr::Bin(..)
            | Expr::Index(..)
            | Expr::Comma(..)
            | Expr::List(_)
            | Expr::Map(_)
            | Expr::Template { .. } => {
                for (_, part) in self.plain_parts().expect("a plain expression") {
                    f(Child::Expr(part));
                }
                if let Expr::Template { parts, .. } = self {
                    for part in parts {
                        if let TplPart::Lit(_, nodes) = part {
                            f(Child::Nodes(nodes));
                        }
                    }
                }
            }
            Expr::Await(e) | Expr::Chain(e) | Expr::OptAttr(e, _) => f(Child::Expr(e)),
            Expr::And(a, b) | Expr::Or(a, b) | Expr::Nullish(a, b) | Expr::OptIndex(a, b) => {
                f(Child::Expr(a));
                f(Child::Expr(b));
            }
            Expr::Cond(c, a, b) => {
                f(Child::Expr(c));
                f(Child::Expr(a));
                f(Child::Expr(b));
            }
            Expr::Call(_, args) | Expr::New(_, args) => exprs(args, f),
            Expr::Method(base, _, args)
            | Expr::PrivateMethod(base, _, args)
            | Expr::OptMethod(base, _, args)
            | Expr::Apply(base, args)
            | Expr::OptApply(base, args) => {
                f(Child::Expr(base));
                exprs(args, f);
            }
            Expr::Assign(target, _, value) => {
                f(Child::Target(target));
                f(Child::Expr(value));
            }
            Expr::Update(target, ..) => f(Child::Target(target)),
            Expr::Mutate(target, _, args) => {
                f(Child::Target(target));
                exprs(args, f);
            }
            Expr::Markup(nodes) => f(Child::Nodes(nodes)),
            Expr::Class(class) => {
                if let Some(parent) = &class.parent {
                    f(Child::Expr(parent));
                }
            }
        }
    }
}

/// Every name a piece of code mentions (variables, calls, assignment targets, component tags),
/// for a closure's captures. Over-approximate: a name that is not an enclosing local is skipped
/// when the closure is created.
pub struct Names(pub Vec<Sym>);

impl Names {
    fn add(&mut self, n: &Sym) {
        if !self.0.iter().any(|m| Rc::ptr_eq(m, n) || m == n) {
            self.0.push(n.clone());
        }
    }

    pub fn exprs(&mut self, es: &[Expr]) {
        for e in es {
            self.expr(e);
        }
    }

    pub fn expr(&mut self, e: &Expr) {
        // The names this form itself mentions, then those of everything inside it.
        match e {
            Expr::Var(n) | Expr::Call(n, _) | Expr::New(n, _) => self.add(n),
            Expr::Private(..) | Expr::PrivateMethod(..) => self.add(&Rc::from("\0private.context")),
            Expr::Lambda(l) => {
                for n in &l.captures {
                    self.add(n);
                }
            }
            Expr::Class(c) => {
                for l in c.ctor.iter().chain(c.members.iter().filter_map(|m| m.value.as_ref())) {
                    for n in &l.captures {
                        self.add(n)
                    }
                }
            }
            _ => {}
        }
        e.children(&mut |child| match child {
            Child::Expr(e) => self.expr(e),
            Child::Target(t) => self.target(t),
            Child::Nodes(nodes) => self.nodes(nodes),
        });
    }

    pub fn target(&mut self, t: &Target) {
        match t {
            Target::Var(n) => self.add(n),
            Target::Temp(_) => {}
            Target::Attr(b, _) => self.target(b),
            Target::Private(b, _) => {
                self.target(b);
                self.add(&Rc::from("\0private.context"));
            }
            Target::Index(b, i) => {
                self.target(b);
                self.expr(i);
            }
            Target::Expr(e) => self.expr(e),
        }
    }

    pub fn pattern(&mut self, p: &Pattern) {
        match p {
            Pattern::Name(_) => {}
            Pattern::Array(items, rest) => {
                for (p, d) in items.iter().flatten() {
                    self.pattern(p);
                    if let Some(d) = d {
                        self.expr(d);
                    }
                }
                if let Some(r) = rest {
                    self.pattern(r);
                }
            }
            Pattern::Object(items, _) => {
                for (k, p, d) in items {
                    self.expr(k);
                    self.pattern(p);
                    if let Some(d) = d {
                        self.expr(d);
                    }
                }
            }
        }
    }

    pub fn params(&mut self, ps: &[Param]) {
        for p in ps {
            if let Some(d) = &p.default {
                self.expr(d);
            }
        }
    }

    pub fn nodes(&mut self, nodes: &[Node]) {
        for n in nodes {
            match n {
                Node::Text(_) | Node::Special { .. } => {}
                Node::DynSpecial(e) | Node::TokId(e) | Node::Child(e) => self.expr(e),
                Node::Builtin { attrs, children, .. } => {
                    self.attrs(attrs);
                    self.nodes(children);
                }
                Node::Call { name, attrs, children } => {
                    self.add(&Rc::from(name.split('.').next().unwrap()));
                    self.attrs(attrs);
                    self.nodes(children);
                }
            }
        }
    }

    fn attrs(&mut self, attrs: &[Attr]) {
        for a in attrs {
            match &a.value {
                AttrVal::Expr(e) | AttrVal::Spread(e) => self.expr(e),
                AttrVal::Str(_) | AttrVal::Bare => {}
            }
        }
    }

    pub fn stmts(&mut self, b: &[Stmt]) {
        for s in b {
            self.stmt(s);
        }
    }

    pub fn stmt(&mut self, s: &Stmt) {
        match s {
            Stmt::Let(_, e) | Stmt::Init(_, e) | Stmt::Expr(e) | Stmt::Return(e) | Stmt::Throw(e) => self.expr(e),
            Stmt::LetPattern(p, e, _) => {
                self.pattern(p);
                self.expr(e);
            }
            Stmt::Assign(t, e) => {
                self.target(t);
                self.expr(e);
            }
            Stmt::AssignPattern(ts, e) => {
                for t in ts {
                    self.target(t);
                }
                self.expr(e);
            }
            Stmt::If(arms, els) => {
                for (c, b) in arms {
                    self.expr(c);
                    self.stmts(b);
                }
                if let Some(b) = els {
                    self.stmts(b);
                }
            }
            Stmt::For(p, e, b) => {
                self.pattern(p);
                self.expr(e);
                self.stmts(b);
            }
            Stmt::While(e, b) | Stmt::DoWhile(b, e) => {
                self.expr(e);
                self.stmts(b);
            }
            Stmt::ForC(init, c, update, body) => {
                self.stmts(init);
                self.expr(c);
                self.stmts(update);
                self.stmts(body);
            }
            Stmt::Switch(e, arms, _) => {
                self.expr(e);
                for (vals, b) in arms {
                    self.exprs(vals);
                    self.stmts(b);
                }
            }
            Stmt::Try(b, _, h, f) => {
                self.stmts(b);
                if let Some(h) = h {
                    self.stmts(h);
                }
                if let Some(f) = f {
                    self.stmts(f);
                }
            }
            Stmt::Block(b) => self.stmts(b),
            Stmt::Labeled(_, s) => self.stmt(s),
            Stmt::Break(_) | Stmt::Continue(_) | Stmt::Line(_) | Stmt::Declare(_) => {}
        }
    }
}
