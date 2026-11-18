//! Function control flow and ordered evaluations around suspension points.
use crate::{ast::*, value::Value};
use std::rc::Rc;
pub(crate) type Reg = usize;

#[derive(Debug)]
pub(crate) struct Code {
    pub ops: Vec<Op>,
    pub registers: usize,
}
#[derive(Debug)]
pub(crate) enum Op {
    Eval(Expr, Reg),
    /// `Eval` inside an optional chain: a link in the expression that finds null or undefined
    /// jumps to the chain's end (the third field), as a link compiled on its own does.
    EvalLink(Expr, Reg, usize),
    Statement(Stmt),
    Copy(Reg, Reg),
    Clear(Reg),
    Await(Reg, Reg),
    Return(Reg),
    Break(Option<Sym>),
    Continue(Option<Sym>),
    Enter {
        start: Reg,
        end: Reg,
    },
    Leave,
    Jump(usize),
    Branch(Reg, Test, usize),
    Target {
        end: usize,
        next: Option<usize>,
        label: Option<Sym>,
        /// A labelled block: only a `break` that names it ends it. A loop or a `switch` also ends
        /// on a plain `break`.
        block: bool,
    },
    EndTarget,
    Try {
        catch: Option<usize>,
        finally: Option<usize>,
        end: usize,
        name: Option<Sym>,
    },
    EndTry,
    Fork(Vec<Sym>),
    Call {
        callee: Expr,
        function: Reg,
        receiver: Reg,
    },
    ArrayFill {
        constructor: Reg,
        count: Reg,
        function: Reg,
        receiver: Reg,
        construct: bool,
    },
    Invoke {
        function: Reg,
        receiver: Reg,
        args: Vec<(Reg, bool)>,
        result: Reg,
        construct: bool,
    },
    Reference {
        target: Target,
        slot: usize,
    },
    Read {
        slot: usize,
        result: Reg,
    },
    Write {
        slot: usize,
        value: Reg,
    },
    Forget(usize),
    Iterate {
        source: Reg,
        slot: usize,
    },
    Next {
        slot: usize,
        value: Reg,
        end: usize,
    },
    EndIterator(usize),
    OpenElement {
        node: Node,
        slot: usize,
        result: Reg,
        end: usize,
    },
    CloseElement {
        slot: usize,
        children: Option<Reg>,
        result: Reg,
    },
    Rest {
        source: Reg,
        keys: Vec<Reg>,
        result: Reg,
    },
}
#[derive(Debug, Clone, Copy)]
pub(crate) enum Test {
    Falsy,
    Truthy,
    Nullish,
    Present,
}

struct Builder {
    ops: Vec<Op>,
    registers: usize,
    chain: Vec<Vec<usize>>,
}
impl Code {
    pub(crate) fn block(body: &[Stmt]) -> Rc<Self> {
        let mut b = Builder {
            ops: vec![],
            registers: 0,
            chain: vec![],
        };
        b.block(body);
        let end = b.eval(Expr::Lit(Value::Undef));
        b.ops.push(Op::Return(end));
        Rc::new(Self {
            ops: b.ops,
            registers: b.registers,
        })
    }
    pub(crate) fn expression(expr: &Expr) -> Rc<Self> {
        Self::block(&[Stmt::Return(expr.clone())])
    }
}
impl Builder {
    fn reg(&mut self) -> Reg {
        let n = self.registers;
        self.registers += 1;
        n
    }
    fn eval(&mut self, expr: Expr) -> Reg {
        let r = self.reg();
        self.ops.push(Op::Eval(expr, r));
        r
    }
    fn temp(r: Reg) -> Expr {
        Expr::Temp(r)
    }
    fn clear(&mut self, r: Reg) {
        self.ops.push(Op::Clear(r));
    }
    fn patch(&mut self, at: usize, target: usize) {
        match &mut self.ops[at] {
            Op::Jump(t) | Op::Branch(_, _, t) | Op::EvalLink(_, _, t) => *t = target,
            _ => unreachable!(),
        }
    }
    fn jump(&mut self) -> usize {
        let at = self.ops.len();
        self.ops.push(Op::Jump(0));
        at
    }
    fn branch(&mut self, r: Reg, test: Test) -> usize {
        let at = self.ops.len();
        self.ops.push(Op::Branch(r, test, 0));
        at
    }
    fn enter(&mut self) -> usize {
        let at = self.ops.len();
        self.ops.push(Op::Enter {
            start: self.registers,
            end: 0,
        });
        at
    }
    fn leave(&mut self, at: usize) {
        if let Op::Enter { end, .. } = &mut self.ops[at] {
            *end = self.registers;
        }
        self.ops.push(Op::Leave);
    }
    fn block(&mut self, body: &[Stmt]) {
        let at = self.enter();
        for s in body {
            self.stmt(s, None);
        }
        self.leave(at);
    }
    fn bind(&mut self, stmt: Stmt, r: Reg) {
        self.ops.push(Op::Statement(stmt));
        self.clear(r);
    }
    fn stmt(&mut self, stmt: &Stmt, label: Option<Sym>) {
        match stmt {
            Stmt::Return(e) => {
                let r = self.expr(e);
                self.ops.push(Op::Return(r));
            }
            Stmt::Break(label) => self.ops.push(Op::Break(label.clone())),
            Stmt::Continue(label) => self.ops.push(Op::Continue(label.clone())),
            Stmt::Block(body) => {
                if let Some(label) = label {
                    let at = self.ops.len();
                    self.ops.push(Op::Target {
                        end: 0,
                        block: true,
                        next: None,
                        label: Some(label),
                    });
                    self.block(body);
                    self.ops.push(Op::EndTarget);
                    let end = self.ops.len();
                    if let Op::Target { end: t, .. } = &mut self.ops[at] {
                        *t = end;
                    }
                } else {
                    self.block(body);
                }
            }
            Stmt::Labeled(label, body) => self.stmt(body, Some(label.clone())),
            Stmt::If(arms, otherwise) => {
                let mut ends = vec![];
                for (condition, body) in arms {
                    let r = self.expr(condition);
                    let next = self.branch(r, Test::Falsy);
                    self.clear(r);
                    self.block(body);
                    ends.push(self.jump());
                    let at = self.ops.len();
                    self.patch(next, at);
                    self.clear(r);
                }
                if let Some(body) = otherwise {
                    self.block(body);
                }
                let end = self.ops.len();
                for at in ends {
                    self.patch(at, end);
                }
            }
            Stmt::While(condition, body) | Stmt::DoWhile(body, condition) => {
                let target = self.ops.len();
                self.ops.push(Op::Target {
                    end: 0,
                    block: false,
                    next: None,
                    label,
                });
                let start = self.ops.len();
                let mut exit = None;
                if matches!(stmt, Stmt::While(..)) {
                    let r = self.expr(condition);
                    exit = Some(self.branch(r, Test::Falsy));
                    self.clear(r);
                }
                self.block(body);
                let next = self.ops.len();
                if matches!(stmt, Stmt::DoWhile(..)) {
                    let r = self.expr(condition);
                    exit = Some(self.branch(r, Test::Falsy));
                    self.clear(r);
                }
                self.ops.push(Op::Jump(start));
                let clean = self.ops.len();
                if let Some(exit) = exit {
                    self.patch(exit, clean);
                }
                self.ops.push(Op::EndTarget);
                let end = self.ops.len();
                if let Op::Target { end: t, next: n, .. } = &mut self.ops[target] {
                    *t = end;
                    *n = Some(if matches!(stmt, Stmt::DoWhile(..)) { next } else { start });
                }
            }
            Stmt::For(pattern, iterable, body) => {
                let scope = self.enter();
                let iterable = self.expr(iterable);
                let slot = self.reg();
                let value = self.reg();
                self.ops.push(Op::Iterate { source: iterable, slot });
                self.clear(iterable);
                let target = self.ops.len();
                self.ops.push(Op::Target {
                    end: 0,
                    block: false,
                    next: None,
                    label,
                });
                let start = self.ops.len();
                let exit = self.ops.len();
                self.ops.push(Op::Next { slot, value, end: 0 });
                let iteration = self.enter();
                self.pattern(pattern, value, false);
                self.clear(value);
                self.block(body);
                self.leave(iteration);
                self.ops.push(Op::Jump(start));
                let clean = self.ops.len();
                if let Op::Next { end, .. } = &mut self.ops[exit] {
                    *end = clean;
                }
                self.ops.push(Op::EndTarget);
                let end = self.ops.len();
                if let Op::Target { end: t, next, .. } = &mut self.ops[target] {
                    *t = end;
                    *next = Some(start);
                }
                self.ops.push(Op::EndIterator(slot));
                self.clear(value);
                self.leave(scope);
            }
            Stmt::ForC(init, condition, update, body) => {
                let scope = self.enter();
                let mut names = vec![];
                for s in init {
                    match s {
                        Stmt::Let(n, _) | Stmt::Init(n, _) => names.push(n.clone()),
                        Stmt::Declare(ns) => names.extend(ns.clone()),
                        _ => {}
                    }
                    self.stmt(s, None);
                }
                let target = self.ops.len();
                self.ops.push(Op::Target {
                    end: 0,
                    block: false,
                    next: None,
                    label,
                });
                let start = self.ops.len();
                let r = self.expr(condition);
                let exit = self.branch(r, Test::Falsy);
                self.clear(r);
                self.block(body);
                let next = self.ops.len();
                self.ops.push(Op::Fork(names));
                for s in update {
                    self.stmt(s, None);
                }
                self.ops.push(Op::Jump(start));
                let clean = self.ops.len();
                self.patch(exit, clean);
                self.clear(r);
                self.ops.push(Op::EndTarget);
                let end = self.ops.len();
                if let Op::Target { end: t, next: n, .. } = &mut self.ops[target] {
                    *t = end;
                    *n = Some(next);
                }
                self.leave(scope);
            }
            Stmt::Try(body, name, handler, finally) => {
                let at = self.ops.len();
                self.ops.push(Op::Try {
                    catch: None,
                    finally: None,
                    end: 0,
                    name: name.clone(),
                });
                self.block(body);
                self.ops.push(Op::EndTry);
                let catch = handler.as_ref().map(|body| {
                    let pc = self.ops.len();
                    self.block(body);
                    self.ops.push(Op::EndTry);
                    pc
                });
                let finally = finally.as_ref().map(|body| {
                    let pc = self.ops.len();
                    self.block(body);
                    self.ops.push(Op::EndTry);
                    pc
                });
                let end = self.ops.len();
                self.ops[at] = Op::Try {
                    catch,
                    finally,
                    end,
                    name: name.clone(),
                };
            }
            Stmt::Switch(value, arms, default) => {
                let value = self.expr(value);
                let target = self.ops.len();
                self.ops.push(Op::Target {
                    end: 0,
                    block: false,
                    next: None,
                    label,
                });
                // One scope for the whole switch, entered before any case is chosen: a `let` in
                // one case is still there in the next when control falls through.
                let scope = self.enter();
                let mut jumps = vec![];
                for (i, (cases, _)) in arms.iter().enumerate() {
                    for case in cases {
                        let c = self.expr(case);
                        let eq = self.eval(Expr::Bin(
                            BinOp::StrictEq,
                            Box::new(Self::temp(value)),
                            Box::new(Self::temp(c)),
                        ));
                        self.clear(c);
                        let at = self.branch(eq, Test::Truthy);
                        self.clear(eq);
                        jumps.push((at, i));
                    }
                }
                let fallback = self.jump();
                let mut starts = vec![];
                for (_, body) in arms {
                    starts.push(self.ops.len());
                    for s in body {
                        self.stmt(s, None);
                    }
                }
                let clean = self.ops.len();
                self.leave(scope);
                self.ops.push(Op::EndTarget);
                let end = self.ops.len();
                if let Op::Target { end: t, .. } = &mut self.ops[target] {
                    *t = end;
                }
                for (at, i) in jumps {
                    self.patch(at, starts[i]);
                }
                self.patch(fallback, default.map_or(clean, |i| starts[i]));
                self.clear(value);
            }
            Stmt::Let(n, e) => {
                let r = self.expr(e);
                self.bind(Stmt::Let(n.clone(), Self::temp(r)), r);
            }
            Stmt::Init(n, e) => {
                let r = self.expr(e);
                self.bind(Stmt::Init(n.clone(), Self::temp(r)), r);
            }
            Stmt::LetPattern(p, e, init) => {
                let r = self.expr(e);
                self.pattern(p, r, *init);
                self.clear(r);
            }
            Stmt::Expr(e) => {
                let r = self.expr(e);
                self.clear(r);
            }
            Stmt::Throw(e) => {
                let r = self.expr(e);
                self.bind(Stmt::Throw(Self::temp(r)), r);
            }
            Stmt::Assign(t, e) if !awaits(e) && !target_awaits(t) => self.ops.push(Op::Statement(stmt.clone())),
            Stmt::Assign(t, e) => {
                let r = self.expr(&Expr::Assign(Box::new(t.clone()), AssignOp::Set, Box::new(e.clone())));
                self.clear(r);
            }
            Stmt::AssignPattern(ts, e) => {
                let r = self.expr(e);
                let list = self.eval(Expr::List(vec![Expr::Spread(Box::new(Self::temp(r)))]));
                self.clear(r);
                for (i, t) in ts.iter().enumerate() {
                    let value = self.eval(Expr::Index(
                        Box::new(Self::temp(list)),
                        Box::new(Expr::Lit(Value::Int(i as i64))),
                    ));
                    let target = self.target(t);
                    let slot = self.reg();
                    self.ops.push(Op::Reference { target, slot });
                    self.ops.push(Op::Write { slot, value });
                    self.ops.push(Op::Forget(slot));
                    self.clear(value);
                }
                self.clear(list);
            }
            Stmt::Line(_) | Stmt::Declare(_) => self.ops.push(Op::Statement(stmt.clone())),
        }
    }
    fn default(&mut self, value: Reg, default: &Option<Expr>) {
        if let Some(default) = default {
            let absent = self.eval(Expr::Bin(
                BinOp::StrictEq,
                Box::new(Self::temp(value)),
                Box::new(Expr::Lit(Value::Undef)),
            ));
            let skip = self.branch(absent, Test::Falsy);
            self.clear(absent);
            let replacement = self.expr(default);
            self.ops.push(Op::Copy(replacement, value));
            self.clear(replacement);
            let end = self.ops.len();
            self.patch(skip, end);
            self.clear(absent);
        }
    }
    fn pattern(&mut self, p: &Pattern, value: Reg, init: bool) {
        if !pattern_awaits(p) {
            self.ops
                .push(Op::Statement(Stmt::LetPattern(p.clone(), Self::temp(value), init)));
            return;
        }
        match p {
            Pattern::Name(_) => unreachable!(),
            Pattern::Array(items, rest) => {
                let list = self.eval(Expr::List(vec![Expr::Spread(Box::new(Self::temp(value)))]));
                for (i, item) in items.iter().enumerate() {
                    if let Some((p, d)) = item {
                        let v = self.eval(Expr::Index(
                            Box::new(Self::temp(list)),
                            Box::new(Expr::Lit(Value::Int(i as i64))),
                        ));
                        self.default(v, d);
                        self.pattern(p, v, init);
                        self.clear(v);
                    }
                }
                if let Some(rest) = rest {
                    let v = self.eval(Expr::Method(
                        Box::new(Self::temp(list)),
                        Rc::from("slice"),
                        vec![Expr::Lit(Value::Int(items.len() as i64))],
                    ));
                    self.pattern(rest, v, init);
                    self.clear(v);
                }
                self.clear(list);
            }
            Pattern::Object(items, rest) => {
                self.ops.push(Op::Statement(Stmt::LetPattern(
                    Pattern::Object(vec![], None),
                    Self::temp(value),
                    init,
                )));
                let mut keys = vec![];
                for (k, p, d) in items {
                    let k = self.expr(k);
                    let key = self.eval(Expr::ToString(Box::new(Self::temp(k))));
                    self.clear(k);
                    let v = self.eval(Expr::Index(Box::new(Self::temp(value)), Box::new(Self::temp(key))));
                    self.default(v, d);
                    self.pattern(p, v, init);
                    self.clear(v);
                    keys.push(key);
                }
                if let Some(rest) = rest {
                    let result = self.reg();
                    self.ops.push(Op::Rest {
                        source: value,
                        keys: keys.clone(),
                        result,
                    });
                    self.ops.push(Op::Statement(if init {
                        Stmt::Init(rest.clone(), Self::temp(result))
                    } else {
                        Stmt::Let(rest.clone(), Self::temp(result))
                    }));
                    self.clear(result);
                }
                for k in keys {
                    self.clear(k);
                }
            }
        }
    }
    /// The register that holds a part evaluated into `r`, as its `capture` says (`ast::Capture`).
    fn capture(&mut self, capture: Capture, r: Reg) -> Reg {
        let held = match capture {
            Capture::Value => return r,
            Capture::Text => Expr::ToString(Box::new(Self::temp(r))),
            Capture::Items => Expr::List(vec![Expr::Spread(Box::new(Self::temp(r)))]),
            Capture::Fields => Expr::Map(vec![(Expr::Spread(Box::new(Self::temp(r))), Expr::Lit(Value::Undef))]),
        };
        let held = self.eval(held);
        self.clear(r);
        held
    }
    /// A plain expression with each part evaluated, in the order `Expr::plain_parts` gives, and
    /// a temporary in its place; and those temporaries. `None` if `e` is not plain.
    fn lift(&mut self, e: &Expr) -> Option<(Expr, Vec<Reg>)> {
        let mut form = e.clone();
        let mut temps = vec![];
        for (capture, part) in form.plain_parts_mut()? {
            let r = self.expr(part);
            let held = self.capture(capture, r);
            *part = Self::temp(held);
            temps.push(held);
        }
        Some((form, temps))
    }
    fn clear_all(&mut self, temps: Vec<Reg>) {
        for r in temps {
            self.clear(r);
        }
    }
    /// Arguments, each evaluated in order; a spread one is held as a copy of its items.
    fn args(&mut self, args: &[Expr]) -> Vec<(Reg, bool)> {
        args.iter()
            .map(|arg| match arg {
                Expr::Spread(e) => {
                    let value = self.expr(e);
                    (self.capture(Capture::Items, value), true)
                }
                e => (self.expr(e), false),
            })
            .collect()
    }
    /// Call what `callee` names with `args`: the function and its receiver are read first, then
    /// the arguments are evaluated, then it is called. `optional`: a missing function ends the
    /// optional chain (`f?.()`).
    fn invoke(&mut self, callee: Expr, args: &[Expr], construct: bool, optional: bool) -> Reg {
        let function = self.reg();
        let receiver = self.reg();
        self.ops.push(Op::Call {
            callee,
            function,
            receiver,
        });
        if optional {
            self.optional(function);
        }
        self.call(function, receiver, args, construct)
    }
    fn call(&mut self, function: Reg, receiver: Reg, args: &[Expr], construct: bool) -> Reg {
        let args = self.args(args);
        let result = self.reg();
        self.ops.push(Op::Invoke {
            function,
            receiver,
            args: args.clone(),
            result,
            construct,
        });
        self.clear(function);
        self.clear(receiver);
        self.clear_all(args.into_iter().map(|(r, _)| r).collect());
        result
    }
    fn expr(&mut self, e: &Expr) -> Reg {
        if !awaits(e) {
            // Evaluated whole. Inside an optional chain it may hold a link (`a?.b` of
            // `a?.b.c(await g())`), whose short circuit ends the chain being compiled here.
            if !self.chain.is_empty() {
                let r = self.reg();
                let at = self.ops.len();
                self.chain.last_mut().unwrap().push(at);
                self.ops.push(Op::EvalLink(e.clone(), r, 0));
                return r;
            }
            return self.eval(e.clone());
        }
        // A plain expression: its parts in order, then itself (`ast::Expr::plain_parts`).
        if let Some((form, temps)) = self.lift(e) {
            let out = self.eval(form);
            self.clear_all(temps);
            return out;
        }
        // The rest decide whether a part runs, call, assign or await: each is control flow.
        match e {
            Expr::Await(value) => {
                let value = self.expr(value);
                let result = self.reg();
                self.ops.push(Op::Await(value, result));
                self.clear(value);
                result
            }
            Expr::And(a, b) | Expr::Or(a, b) | Expr::Nullish(a, b) => {
                let a = self.expr(a);
                let result = self.reg();
                self.ops.push(Op::Copy(a, result));
                let skip = self.branch(
                    a,
                    match e {
                        Expr::And(..) => Test::Falsy,
                        Expr::Or(..) => Test::Truthy,
                        _ => Test::Present,
                    },
                );
                self.clear(a);
                let b = self.expr(b);
                self.ops.push(Op::Copy(b, result));
                self.clear(b);
                let end = self.ops.len();
                self.patch(skip, end);
                self.clear(a);
                result
            }
            Expr::Cond(c, a, b) => {
                let c = self.expr(c);
                let branch = self.branch(c, Test::Falsy);
                self.clear(c);
                let a = self.expr(a);
                let out = self.reg();
                self.ops.push(Op::Copy(a, out));
                self.clear(a);
                let end = self.jump();
                let other = self.ops.len();
                self.patch(branch, other);
                self.clear(c);
                let b = self.expr(b);
                self.ops.push(Op::Copy(b, out));
                self.clear(b);
                let target = self.ops.len();
                self.patch(end, target);
                out
            }
            Expr::Call(name, args) => self.invoke(Expr::Var(name.clone()), args, false, false),
            Expr::New(name, args) => self.invoke(Expr::Var(name.clone()), args, true, false),
            Expr::Apply(callee, args) | Expr::OptApply(callee, args) => {
                let callee = self.callee(callee);
                self.invoke(callee, args, false, matches!(e, Expr::OptApply(..)))
            }
            // `Array(n).fill(await f())`: the list is made when the call is, after its argument.
            Expr::Method(base, name, args)
                if &**name == "fill"
                    && matches!(&**base,Expr::Call(_,values)|Expr::New(_,values) if values.len()==1) =>
            {
                let (Expr::Call(name, values) | Expr::New(name, values)) = &**base else {
                    unreachable!()
                };
                let constructor = self.reg();
                let receiver = self.reg();
                self.ops.push(Op::Call {
                    callee: Expr::Var(name.clone()),
                    function: constructor,
                    receiver,
                });
                let count = self.expr(&values[0]);
                let function = self.reg();
                self.ops.push(Op::ArrayFill {
                    constructor,
                    count,
                    function,
                    receiver,
                    construct: matches!(&**base, Expr::New(..)),
                });
                self.clear(constructor);
                self.clear(count);
                self.call(function, receiver, args, false)
            }
            Expr::Method(base, name, args) | Expr::PrivateMethod(base, name, args) => {
                let callee = if matches!(e, Expr::PrivateMethod(..)) {
                    Expr::Private(base.clone(), name.clone())
                } else {
                    Expr::Attr(base.clone(), name.clone())
                };
                let callee = self.callee(&callee);
                self.invoke(callee, args, false, false)
            }
            Expr::Assign(target, op, rhs) => {
                let target = self.target(target);
                let slot = self.reg();
                self.ops.push(Op::Reference { target, slot });
                let result = self.reg();
                let mut skip = None;
                if *op != AssignOp::Set {
                    self.ops.push(Op::Read { slot, result });
                    if matches!(op, AssignOp::And | AssignOp::Or | AssignOp::Nullish) {
                        skip = Some(self.branch(
                            result,
                            match op {
                                AssignOp::And => Test::Falsy,
                                AssignOp::Or => Test::Truthy,
                                _ => Test::Present,
                            },
                        ));
                    }
                }
                let value = self.expr(rhs);
                let value = if let AssignOp::Bin(op) = op {
                    let r = self.eval(Expr::Bin(
                        *op,
                        Box::new(Self::temp(result)),
                        Box::new(Self::temp(value)),
                    ));
                    self.clear(value);
                    r
                } else {
                    value
                };
                self.ops.push(Op::Write { slot, value });
                self.ops.push(Op::Copy(value, result));
                self.clear(value);
                let end = self.ops.len();
                if let Some(skip) = skip {
                    self.patch(skip, end);
                }
                self.ops.push(Op::Forget(slot));
                result
            }
            Expr::Chain(inner) => {
                self.chain.push(vec![]);
                let value = self.expr(inner);
                let out = self.reg();
                self.ops.push(Op::Copy(value, out));
                self.clear(value);
                let end = self.jump();
                let fail = self.ops.len();
                self.ops.push(Op::Eval(Expr::Lit(Value::Undef), out));
                for at in self.chain.pop().unwrap() {
                    self.patch(at, fail);
                }
                let done = self.ops.len();
                self.patch(end, done);
                out
            }
            // A link of an optional chain: its base, the check that ends the chain, then the
            // same access or call on the base's value.
            Expr::OptAttr(base, _) | Expr::OptIndex(base, _) | Expr::OptMethod(base, _, _) => {
                let base = self.expr(base);
                self.optional(base);
                let held = Box::new(Self::temp(base));
                let out = match e {
                    Expr::OptAttr(_, name) => self.eval(Expr::Attr(held, name.clone())),
                    Expr::OptIndex(_, key) => self.expr(&Expr::Index(held, key.clone())),
                    Expr::OptMethod(_, name, args) => self.invoke(Expr::Attr(held, name.clone()), args, false, false),
                    _ => unreachable!(),
                };
                self.clear(base);
                out
            }
            Expr::Markup(nodes) => self.markup(nodes),
            Expr::Class(class) => {
                let mut class = (**class).clone();
                if let Some(parent) = &class.parent {
                    let parent = self.expr(parent);
                    class.parent = Some(Self::temp(parent));
                }
                self.eval(Expr::Class(Rc::new(class)))
            }
            Expr::Mutate(target, name, args) => {
                let value = self.expr(&target_expr(target));
                if !name.starts_with("__") {
                    let out = self.invoke(
                        Expr::Attr(Box::new(Self::temp(value)), name.clone()),
                        args,
                        false,
                        false,
                    );
                    self.clear(value);
                    return out;
                }
                let (Expr::List(args), temps) = self.lift(&Expr::List(args.clone())).expect("a list is plain") else {
                    unreachable!()
                };
                let out = self.eval(Expr::Mutate(Box::new(Target::Temp(value)), name.clone(), args));
                self.clear_all(temps);
                out
            }
            Expr::Update(target, delta, prefix) => {
                let target = self.target(target);
                self.eval(Expr::Update(Box::new(target), *delta, *prefix))
            }
            _ => unreachable!("a plain expression, or one with nothing inside it to await"),
        }
    }
    fn optional(&mut self, r: Reg) {
        let at = self.branch(r, Test::Nullish);
        self.chain.last_mut().expect("optional chain boundary").push(at);
    }
    fn callee(&mut self, e: &Expr) -> Expr {
        // Inside an optional chain the callee's receiver is evaluated as a link of the chain
        // (below, through `expr`), so its short circuit ends the chain.
        if !awaits(e) && self.chain.is_empty() {
            return e.clone();
        }
        // `o.m` and `o[k]` stay an access, with their parts evaluated, so the call gets its
        // receiver; anything else is evaluated to the function.
        match e {
            Expr::Attr(..) | Expr::Private(..) | Expr::Index(..) => self.lift(e).expect("an access is plain").0,
            _ => {
                let r = self.expr(e);
                Self::temp(r)
            }
        }
    }
    fn target(&mut self, t: &Target) -> Target {
        match t {
            Target::Var(n) => Target::Var(n.clone()),
            Target::Temp(r) => Target::Temp(*r),
            Target::Expr(e) => Target::Temp(self.expr(e)),
            Target::Attr(b, n) | Target::Private(b, n) => {
                let b = self.expr(&target_expr(b));
                if matches!(t, Target::Private(..)) {
                    Target::Private(Box::new(Target::Temp(b)), n.clone())
                } else {
                    Target::Attr(Box::new(Target::Temp(b)), n.clone())
                }
            }
            Target::Index(b, i) => {
                let b = self.expr(&target_expr(b));
                let i = self.expr(i);
                Target::Index(Box::new(Target::Temp(b)), Self::temp(i))
            }
        }
    }
    fn markup(&mut self, nodes: &[Node]) -> Reg {
        let mut parts = vec![];
        for node in nodes {
            if !nodes_await(std::slice::from_ref(node)) {
                parts.push(self.eval(Expr::Markup(Rc::new(vec![node.clone()]))));
                continue;
            }
            let part = match node {
                Node::Call { attrs, children, .. } | Node::Builtin { attrs, children, .. } => {
                    let attrs = self.attrs(attrs);
                    let mount = matches!(node,Node::Builtin{tag,..} if &**tag=="mount");
                    let children_node = if mount {
                        self.mount_children(children)
                    } else {
                        children.clone()
                    };
                    let header = match node {
                        Node::Call { name, .. } => Node::Call {
                            name: name.clone(),
                            attrs,
                            children: children_node,
                        },
                        Node::Builtin { tag, .. } => Node::Builtin {
                            tag: tag.clone(),
                            attrs,
                            children: children_node,
                        },
                        _ => unreachable!(),
                    };
                    let result = self.reg();
                    let slot = self.reg();
                    let at = self.ops.len();
                    self.ops.push(Op::OpenElement {
                        node: header,
                        slot,
                        result,
                        end: 0,
                    });
                    let children = if children.is_empty() || mount {
                        None
                    } else {
                        Some(self.markup(children))
                    };
                    self.ops.push(Op::CloseElement { slot, children, result });
                    if let Some(child) = children {
                        self.clear(child);
                    }
                    let end = self.ops.len();
                    if let Op::OpenElement { end: t, .. } = &mut self.ops[at] {
                        *t = end;
                    }
                    result
                }
                Node::Child(e) | Node::DynSpecial(e) | Node::TokId(e) => {
                    let r = self.expr(e);
                    let node = match node {
                        Node::Child(_) => Node::Child(Self::temp(r)),
                        Node::DynSpecial(_) => Node::DynSpecial(Self::temp(r)),
                        _ => Node::TokId(Self::temp(r)),
                    };
                    let out = self.eval(Expr::Markup(Rc::new(vec![node])));
                    self.clear(r);
                    out
                }
                _ => unreachable!(),
            };
            parts.push(part);
        }
        let out = self.eval(Expr::Markup(Rc::new(
            parts.iter().map(|r| Node::Child(Self::temp(*r))).collect(),
        )));
        for r in parts {
            self.clear(r);
        }
        out
    }
    fn mount_children(&mut self, nodes: &[Node]) -> Vec<Node> {
        nodes
            .iter()
            .map(|node| match node {
                Node::Call { name, attrs, children } => Node::Call {
                    name: name.clone(),
                    attrs: self.attrs(attrs),
                    children: children.clone(),
                },
                node => node.clone(),
            })
            .collect()
    }
    fn attrs(&mut self, attrs: &[Attr]) -> Vec<Attr> {
        attrs
            .iter()
            .map(|attr| Attr {
                name: attr.name.clone(),
                value: match &attr.value {
                    AttrVal::Expr(e) => {
                        let r = self.expr(e);
                        AttrVal::Expr(Self::temp(r))
                    }
                    AttrVal::Spread(e) => {
                        let r = self.expr(e);
                        AttrVal::Spread(Self::temp(self.capture(Capture::Fields, r)))
                    }
                    v => v.clone(),
                },
            })
            .collect()
    }
}

/// Whether evaluating `e` reaches an `await` (not counting the functions it defines).
fn awaits(e: &Expr) -> bool {
    let mut found = matches!(e, Expr::Await(_));
    e.children(&mut |child| {
        found = found
            || match child {
                Child::Expr(e) => awaits(e),
                Child::Target(t) => target_awaits(t),
                Child::Nodes(nodes) => nodes_await(nodes),
            }
    });
    found
}
fn target_awaits(t: &Target) -> bool {
    match t {
        Target::Var(_) | Target::Temp(_) => false,
        Target::Attr(b, _) | Target::Private(b, _) => target_awaits(b),
        Target::Index(b, e) => target_awaits(b) || awaits(e),
        Target::Expr(e) => awaits(e),
    }
}
fn nodes_await(nodes: &[Node]) -> bool {
    nodes.iter().any(|n| match n {
        Node::Text(_) | Node::Special { .. } => false,
        Node::DynSpecial(e) | Node::TokId(e) | Node::Child(e) => awaits(e),
        Node::Builtin { attrs, children, .. } | Node::Call { attrs, children, .. } => {
            attrs.iter().any(|a| match &a.value {
                AttrVal::Expr(e) | AttrVal::Spread(e) => awaits(e),
                _ => false,
            }) || nodes_await(children)
        }
    })
}

fn target_expr(t: &Target) -> Expr {
    match t {
        Target::Temp(r) => Expr::Temp(*r),
        Target::Var(n) => Expr::Var(n.clone()),
        Target::Attr(b, n) => Expr::Attr(Box::new(target_expr(b)), n.clone()),
        Target::Private(b, n) => Expr::Private(Box::new(target_expr(b)), n.clone()),
        Target::Index(b, i) => Expr::Index(Box::new(target_expr(b)), Box::new(i.clone())),
        Target::Expr(e) => (**e).clone(),
    }
}

fn pattern_awaits(p: &Pattern) -> bool {
    match p {
        Pattern::Name(_) => false,
        Pattern::Array(items, rest) => {
            items
                .iter()
                .flatten()
                .any(|(p, d)| pattern_awaits(p) || d.as_ref().is_some_and(awaits))
                || rest.as_ref().is_some_and(|p| pattern_awaits(p))
        }
        Pattern::Object(items, _) => items
            .iter()
            .any(|(k, p, d)| awaits(k) || pattern_awaits(p) || d.as_ref().is_some_and(awaits)),
    }
}
