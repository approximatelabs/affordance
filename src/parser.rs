//! Hand-written recursive-descent parser for the view language.

use crate::ast::*;
use crate::value::Value;
use std::collections::HashMap;
use std::rc::Rc;

pub const BUILTIN_TAGS: &[&str] = &[
    "sp", "tok", "box", "span", "aff", "img", "audio", "video", "text", "mount", "turn", "history",
];

#[derive(Default, Debug)]
pub struct Module {
    pub funcs: HashMap<Sym, Rc<Func>>,
    pub consts: Vec<(Sym, Expr)>,
    pub imports: Vec<Import>,
    /// Public name -> local binding. A default export is named `default`.
    pub exports: HashMap<Sym, Sym>,
    pub pragmas: Vec<String>,
    /// `type` and `interface` declarations, by name.
    pub types: HashMap<String, crate::tsx::TsType>,
}

#[derive(Debug)]
pub struct Import {
    pub path: String,
    /// (exported name, local name); empty for a side-effect import.
    pub bindings: Vec<(Sym, Sym)>,
}

/// A JSDoc comment's summary: its lines without the `*` margin, up to the first `@tag`, joined
/// with single spaces.
fn jsdoc_summary(body: &str) -> String {
    let mut out: Vec<&str> = vec![];
    for line in body.lines() {
        let l = line.trim().trim_start_matches('*').trim();
        if l.starts_with('@') {
            break;
        }
        if !l.is_empty() {
            out.push(l);
        }
    }
    out.join(" ")
}

pub struct Parser<'a> {
    pub(crate) src: &'a [u8],
    pub(crate) private_scopes: Vec<(Vec<Sym>, Vec<Sym>)>,
    nesting: usize,
    pub(crate) pos: usize,
    pub(crate) file: String,
    /// Per enclosing function: its `var` names (declared at the top of its body).
    pub(crate) var_scopes: Vec<Vec<Sym>>,
    /// Per enclosing block: its function declarations (initialized at the top of the block).
    pub(crate) block_funcs: Vec<Vec<(Sym, Expr)>>,
    /// The names declared in each enclosing scope, and whether each is a `const`.
    pub(crate) decls: Vec<Vec<(Sym, bool)>>,
    /// The span of the last parenthesized expression (`(a || b) ?? c` is allowed).
    pub(crate) last_paren: (usize, usize),
    /// Inside an async function's own body: the only place `await` is an operator.
    pub(crate) in_async: bool,
    /// TSX `"use strict_tokens"`: special-token strings in source text are load errors.
    pub(crate) strict_tokens: bool,
    pub(crate) strict_error: Option<String>,
    /// The type of the last props parameter parsed (`({q}: {q: string})`), for its function.
    pub(crate) props_type: Option<crate::tsx::TsType>,
    /// The depth of the deepest expression tree parsed since the enclosing operand began, the
    /// depths of the chains in progress (one entry each, from `chain_base`), and where the
    /// innermost chain's entry is: see `chain`.
    pub(crate) deepest: usize,
    pub(crate) lefts: Vec<usize>,
    pub(crate) chain_base: usize,
    /// The last `/** .. */` comment: where it ends and its text (a declaration right after it
    /// takes it as its doc).
    pub(crate) last_doc: Option<(usize, std::rc::Rc<str>)>,
    /// `type` and `interface` declarations (aliases for `schemaOf`).
    pub(crate) types: HashMap<String, crate::tsx::TsType>,
    /// Added-token strings of the bound tokenizer (longest first). Literal markup text
    /// is split on these at parse time, so constant special tokens cost nothing at render.
    pub specials: &'a [(String, u32)],
    pub special_ids: Option<&'a std::collections::HashMap<String, u32>>,
    /// Byte offsets where lines start, for error locations recorded in the tree.
    line_starts: Vec<usize>,
    pub(crate) file_sym: Sym,
}

pub type PResult<T> = Result<T, String>;

pub(crate) fn is_ident_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_' || c == b'$'
}
pub(crate) fn is_ident(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'$'
}

/// How a binary operator joins its operands: a node of its own (`||`, `&&`), or a `Bin` with
/// its operator. `Exact` is a `Bin` whose token is matched as written (`===`), where the others
/// are matched as whole operators (`==` not followed by `=`).
#[derive(Clone, Copy)]
enum Join {
    Or,
    And,
    Bin(BinOp),
    Exact(BinOp),
}

/// The deepest tree of operators one expression may build. Hand-written code stays far below it.
const MAX_EXPRESSION_DEPTH: usize = 1024;

impl<'a> Parser<'a> {
    pub fn new(src: &'a str, file: &str) -> Self {
        Parser {
            src: src.as_bytes(),
            private_scopes: vec![],
            nesting: 0,
            pos: 0,
            file: file.to_string(),
            specials: &[],
            special_ids: None,
            var_scopes: vec![],
            block_funcs: vec![],
            decls: vec![],
            last_paren: (usize::MAX, 0),
            in_async: false,
            strict_tokens: false,
            strict_error: None,
            props_type: None,
            deepest: 0,
            lefts: vec![],
            chain_base: 0,
            last_doc: None,
            types: HashMap::new(),
            line_starts: std::iter::once(0)
                .chain(src.bytes().enumerate().filter(|(_, c)| *c == b'\n').map(|(i, _)| i + 1))
                .collect(),
            file_sym: Sym::from(file),
        }
    }

    /// Bound recursive grammar productions and grow on the same thread before allocating
    /// their frames. Malformed input must be a load error on an ordinary worker stack.
    // The operator levels that are only "operands of the next level, joined left to right by one
    // of these operators" are one function (`level`) and a line each. An operator is listed
    // before any it is a prefix of (`===` before `==`, `>>>` before `>>`).
    fn or(&mut self) -> PResult<Expr> {
        self.level(Self::and, &[("||", Join::Or)])
    }
    fn and(&mut self) -> PResult<Expr> {
        self.level(Self::bitor, &[("&&", Join::And)])
    }
    fn bitor(&mut self) -> PResult<Expr> {
        self.level(Self::bitxor, &[("|", Join::Bin(BinOp::BitOr))])
    }
    fn bitxor(&mut self) -> PResult<Expr> {
        self.level(Self::bitand, &[("^", Join::Bin(BinOp::BitXor))])
    }
    fn bitand(&mut self) -> PResult<Expr> {
        self.level(Self::equality, &[("&", Join::Bin(BinOp::BitAnd))])
    }
    fn equality(&mut self) -> PResult<Expr> {
        self.level(
            Self::relational,
            &[
                ("===", Join::Exact(BinOp::StrictEq)),
                ("!==", Join::Exact(BinOp::StrictNe)),
                ("==", Join::Bin(BinOp::Eq)),
                ("!=", Join::Bin(BinOp::Ne)),
            ],
        )
    }
    fn shift(&mut self) -> PResult<Expr> {
        self.level(
            Self::add,
            &[
                ("<<", Join::Bin(BinOp::Shl)),
                (">>>", Join::Bin(BinOp::UShr)),
                (">>", Join::Bin(BinOp::Shr)),
            ],
        )
    }

    /// One left-associative level of binary operators: operands parsed by `next`, joined by
    /// whichever of `operators` comes between them.
    fn level(&mut self, next: fn(&mut Self) -> PResult<Expr>, operators: &[(&str, Join)]) -> PResult<Expr> {
        self.chain(|p| {
            let mut left = next(p)?;
            let mut links = 0;
            loop {
                p.next_link(&mut links)?;
                let found = operators.iter().find(|(token, join)| match join {
                    Join::Exact(_) => p.eat(token),
                    _ => p.eat_op(token),
                });
                let Some((_, join)) = found else { break };
                let right = next(p)?;
                left = match *join {
                    Join::Or => Expr::Or(Box::new(left), Box::new(right)),
                    Join::And => Expr::And(Box::new(left), Box::new(right)),
                    Join::Bin(op) | Join::Exact(op) => Expr::Bin(op, Box::new(left), Box::new(right)),
                };
            }
            Ok(left)
        })
    }

    /// Parse one operator chain (`a + b + c`, `a.b.c`), keeping count of how deep the tree it builds
    /// is. Every other tree walk in the engine recurses to that depth (and a derived `Drop` cannot
    /// be given more stack), so the depth is bounded here, where the tree is made: per-chain and
    /// per-nesting limits alone still allow their product.
    fn chain(&mut self, f: impl FnOnce(&mut Self) -> PResult<Expr>) -> PResult<Expr> {
        let before = std::mem::take(&mut self.deepest);
        let base = std::mem::replace(&mut self.chain_base, self.lefts.len());
        let parsed = f(self);
        // The last operand, folded into the chain's depth.
        if self.lefts.len() > self.chain_base {
            let left = self.lefts[self.chain_base];
            self.deepest = left.max(self.deepest) + 1;
        }
        self.lefts.truncate(self.chain_base);
        self.chain_base = base;
        let depth = self.deepest;
        // What was parsed before this chain, beside it (an earlier argument), counts as it did.
        self.deepest = depth.max(before);
        if parsed.is_ok() && depth > MAX_EXPRESSION_DEPTH {
            return self.err(format!("expression nests deeper than {MAX_EXPRESSION_DEPTH} operators"));
        }
        parsed
    }

    /// The top of a chain loop that looks for its operator after this call: its turn `n` follows
    /// `n - 1` links, each of which sits above everything parsed so far in the chain.
    pub(crate) fn next_link(&mut self, links: &mut usize) -> PResult<()> {
        *links += 1;
        if *links > 257 {
            return self.err("expression chain exceeds 256 links");
        }
        if *links >= 2 {
            self.deepest += 1;
        }
        Ok(())
    }

    /// One more link of the chain being parsed: at most 256 a chain, and the operands so far
    /// (left-associative: each link sits above them) folded into its depth.
    pub(crate) fn link(&mut self, links: &mut usize) -> PResult<()> {
        *links += 1;
        if *links > 256 {
            return self.err("expression chain exceeds 256 links");
        }
        let operand = std::mem::take(&mut self.deepest);
        if self.lefts.len() > self.chain_base {
            let left = self.lefts[self.chain_base];
            self.lefts[self.chain_base] = left.max(operand) + 1;
        } else {
            self.lefts.push(operand);
        }
        Ok(())
    }

    pub(crate) fn nested<T>(&mut self, f: impl FnOnce(&mut Self) -> PResult<T>) -> PResult<T> {
        if self.nesting >= 256 {
            return self.err("source nesting exceeds 256 levels");
        }
        self.nesting += 1;
        let result = stacker::maybe_grow(128 * 1024, 2 * 1024 * 1024, || f(self));
        self.nesting -= 1;
        result
    }

    /// 1-based line of the current position.
    pub(crate) fn line_no(&self) -> u32 {
        self.line_starts.partition_point(|&s| s <= self.pos) as u32
    }

    pub(crate) fn loc(&self) -> Loc {
        Loc {
            file: self.file_sym.clone(),
            line: self.line_no(),
        }
    }

    pub(crate) fn err<T>(&self, msg: impl AsRef<str>) -> PResult<T> {
        let line = self.src[..self.pos.min(self.src.len())]
            .iter()
            .filter(|&&c| c == b'\n')
            .count()
            + 1;
        let ctx: String =
            String::from_utf8_lossy(&self.src[self.pos.min(self.src.len())..(self.pos + 40).min(self.src.len())])
                .into();
        Err(format!("{}:{}: {} (near {:?})", self.file, line, msg.as_ref(), ctx))
    }

    pub(crate) fn peek(&self) -> u8 {
        *self.src.get(self.pos).unwrap_or(&0)
    }
    pub(crate) fn peek_at(&self, k: usize) -> u8 {
        *self.src.get(self.pos + k).unwrap_or(&0)
    }
    pub(crate) fn starts(&self, s: &str) -> bool {
        self.src
            .get(self.pos..)
            .is_some_and(|rest| rest.starts_with(s.as_bytes()))
    }
    pub(crate) fn eof(&self) -> bool {
        self.pos >= self.src.len()
    }

    /// Skip whitespace and comments (code mode).
    pub(crate) fn ws(&mut self) {
        loop {
            while !self.eof() && self.peek().is_ascii_whitespace() {
                self.pos += 1;
            }
            if self.starts("//") {
                while !self.eof() && self.peek() != b'\n' {
                    self.pos += 1;
                }
            } else if self.starts("/*") {
                let doc = self.starts("/**") && !self.starts("/**/");
                let start = self.pos;
                match find(&self.src[self.pos..], b"*/") {
                    Some(i) => self.pos += i + 2,
                    None => self.pos = self.src.len(),
                }
                if doc {
                    let body = String::from_utf8_lossy(&self.src[start + 3..self.pos.saturating_sub(2).max(start + 3)])
                        .into_owned();
                    self.last_doc = Some((self.pos, std::rc::Rc::from(jsdoc_summary(&body))));
                }
            } else {
                break;
            }
        }
    }

    pub(crate) fn eat(&mut self, s: &str) -> bool {
        self.ws();
        if self.starts(s) {
            // keyword boundary check
            if s.as_bytes().iter().all(|&c| is_ident(c)) && is_ident(self.peek_at(s.len())) {
                return false;
            }
            self.pos += s.len();
            true
        } else {
            false
        }
    }

    pub(crate) fn expect(&mut self, s: &str) -> PResult<()> {
        if self.eat(s) {
            Ok(())
        } else {
            self.err(format!("expected `{}`", s))
        }
    }

    pub(crate) fn ident(&mut self) -> PResult<Sym> {
        self.ws();
        if !is_ident_start(self.peek()) {
            return self.err("expected identifier");
        }
        let st = self.pos;
        while is_ident(self.peek()) {
            self.pos += 1;
        }
        Ok(Rc::from(std::str::from_utf8(&self.src[st..self.pos]).unwrap()))
    }

    pub(crate) fn peek_ident(&mut self) -> Option<String> {
        self.ws();
        if !is_ident_start(self.peek()) {
            return None;
        }
        let mut e = self.pos;
        while is_ident(*self.src.get(e).unwrap_or(&0)) {
            e += 1;
        }
        Some(String::from_utf8_lossy(&self.src[self.pos..e]).into())
    }

    pub(crate) fn peek_after_ws(&mut self) -> u8 {
        self.ws();
        self.peek()
    }

    pub(crate) fn paren_expr(&mut self) -> PResult<Expr> {
        self.expect("(")?;
        let e = self.expr()?;
        self.expect(")")?;
        Ok(e)
    }

    // ---------------- expressions ----------------

    pub fn expr(&mut self) -> PResult<Expr> {
        self.nested(|p| p.expr_inner())
    }

    fn expr_inner(&mut self) -> PResult<Expr> {
        // lambda: `x => e` or `(a, b) => e`
        self.ws();
        let before_async = self.pos;
        let is_async = self.starts("async") && !is_ident(self.peek_at(5));
        if is_async {
            self.pos += 5;
        }
        let save = self.pos;
        if let Some(l) = self.try_tsx_lambda(is_async)? {
            return Ok(Expr::Lambda(Rc::new(l)));
        }
        self.pos = save;
        // `async` not followed by an arrow function is an ordinary identifier.
        self.pos = if is_async { before_async } else { save };
        let e = self.ternary()?;
        // Assignment (right-associative, the lowest precedence but `,`).
        self.ws();
        let op = if self.peek() == b'=' && !matches!(self.peek_at(1), b'=' | b'>') {
            self.pos += 1;
            Some(AssignOp::Set)
        } else {
            const OPS: [(&str, AssignOp); 15] = [
                (">>>=", AssignOp::Bin(BinOp::UShr)),
                ("**=", AssignOp::Bin(BinOp::Pow)),
                ("<<=", AssignOp::Bin(BinOp::Shl)),
                (">>=", AssignOp::Bin(BinOp::Shr)),
                ("&&=", AssignOp::And),
                ("||=", AssignOp::Or),
                ("??=", AssignOp::Nullish),
                ("+=", AssignOp::Bin(BinOp::Add)),
                ("-=", AssignOp::Bin(BinOp::Sub)),
                ("*=", AssignOp::Bin(BinOp::Mul)),
                ("/=", AssignOp::Bin(BinOp::Div)),
                ("%=", AssignOp::Bin(BinOp::Mod)),
                ("&=", AssignOp::Bin(BinOp::BitAnd)),
                ("|=", AssignOp::Bin(BinOp::BitOr)),
                ("^=", AssignOp::Bin(BinOp::BitXor)),
            ];
            OPS.iter().find(|(t, _)| self.starts(t)).map(|(t, op)| {
                self.pos += t.len();
                *op
            })
        };
        let Some(op) = op else { return Ok(e) };
        let t = match to_target(e) {
            Ok(t) => t,
            Err(m) => return self.err(format!("{m} (the left side of an assignment)")),
        };
        let v = self.expr()?;
        if let Some(e) = self.const_assignment(&t, Expr::Lit(Value::Undef)) {
            return Ok(Expr::Comma(Box::new(v), Box::new(e)));
        }
        Ok(Expr::Assign(Box::new(t), op, Box::new(v)))
    }

    fn ternary(&mut self) -> PResult<Expr> {
        let c = self.nullish()?;
        self.ws();
        if self.peek() == b'?' && self.peek_at(1) != b'?' {
            self.pos += 1;
            let a = self.expr()?;
            self.expect(":")?;
            let b = self.expr()?;
            return Ok(Expr::Cond(Box::new(self.cond(c)), Box::new(a), Box::new(b)));
        }
        Ok(c)
    }

    pub(crate) fn nullish(&mut self) -> PResult<Expr> {
        self.chain(Self::nullish_links)
    }

    fn nullish_links(&mut self) -> PResult<Expr> {
        // `a || b ?? c` is a SyntaxError in JavaScript: `??` does not mix with an unparenthesized `||`/`&&`.
        let bare_logical = |p: &mut Self, start: usize, e: &Expr| {
            matches!(e, Expr::Or(..) | Expr::And(..)) && p.last_paren != (start, p.pos)
        };
        self.ws();
        let start = self.pos;
        let mut l = self.or()?;
        if self.starts_ws("??") && !self.starts_ws("??=") && bare_logical(self, start, &l) {
            return self.err("`??` cannot be mixed with `||` or `&&` without parentheses");
        }
        let mut links = 0;
        while self.eat_op("??") {
            self.link(&mut links)?;

            self.ws();
            let start = self.pos;
            let r = self.or()?;
            if bare_logical(self, start, &r) {
                return self.err("`??` cannot be mixed with `||` or `&&` without parentheses");
            }
            l = Expr::Nullish(Box::new(l), Box::new(r));
        }
        Ok(l)
    }

    /// `starts`, after whitespace and comments.
    pub(crate) fn starts_ws(&mut self, s: &str) -> bool {
        let save = self.pos;
        self.ws();
        let r = self.starts(s);
        self.pos = save;
        r
    }

    /// `s` as an operator: not the start of a longer operator (`|` is not `||` or `|=`).
    pub(crate) fn eat_op(&mut self, s: &str) -> bool {
        self.ws();
        if !self.starts(s) {
            return false;
        }
        let next = self.peek_at(s.len());
        let longer = match s {
            "??" | "||" | "&&" | "**" | "<<" | ">>>" | "==" | "!=" | "<=" | ">=" => next == b'=',
            "|" => next == b'|' || next == b'=',
            "&" => next == b'&' || next == b'=',
            "^" => next == b'=',
            "<" => next == b'<' || next == b'=',
            ">" | ">>" => next == b'>' || next == b'=',
            _ => false,
        };
        if longer {
            return false;
        }
        self.pos += s.len();
        true
    }

    fn relational(&mut self) -> PResult<Expr> {
        self.chain(Self::relational_links)
    }

    fn relational_links(&mut self) -> PResult<Expr> {
        let mut l = self.shift()?;
        let mut links = 0;
        loop {
            self.next_link(&mut links)?;

            self.ws();
            let op = if self.eat_op("<=") {
                BinOp::Le
            } else if self.eat_op(">=") {
                BinOp::Ge
            } else if self.eat_op("<") {
                BinOp::Lt
            } else if self.eat_op(">") {
                BinOp::Gt
            } else if self.starts("instanceof") && !is_ident(self.peek_at(10)) {
                self.pos += 10;
                let constructor = self.shift()?;
                l = Expr::Bin(BinOp::InstanceOf, Box::new(l), Box::new(constructor));
                continue;
            } else if self.starts("in") && !is_ident(self.peek_at(2)) {
                self.pos += 2;
                BinOp::In
            } else {
                // Python's word operators, on the same line as the operand they follow.
                match self.peek_ident().as_deref() {
                    Some("not") if !self.newline_before() => return self.err(crate::migrate::NOT_IN),
                    Some("and" | "or") if !self.newline_before() => return self.err(crate::migrate::AND_OR),
                    _ => break,
                }
            };
            let r = self.shift()?;
            l = Expr::Bin(op, Box::new(l), Box::new(r));
        }
        Ok(l)
    }

    /// At an argument: `name=` (not `==` or `=>`), a Python keyword argument.
    fn kwarg_ahead(&mut self) -> bool {
        self.ws();
        let mut i = self.pos;
        if !self.src.get(i).is_some_and(|c| is_ident_start(*c)) {
            return false;
        }
        while self.src.get(i).is_some_and(|c| is_ident(*c)) {
            i += 1;
        }
        while self.src.get(i).is_some_and(|c| *c == b' ' || *c == b'\t') {
            i += 1;
        }
        self.src.get(i) == Some(&b'=') && !matches!(self.src.get(i + 1), Some(b'=' | b'>'))
    }

    /// True if a newline separates the current position from the previous token.
    fn newline_before(&self) -> bool {
        self.src[..self.pos]
            .iter()
            .rev()
            .take_while(|c| c.is_ascii_whitespace())
            .any(|c| *c == b'\n')
    }

    /// A condition: ToBoolean.
    pub(crate) fn cond(&self, e: Expr) -> Expr {
        Expr::Truthy(Box::new(e))
    }

    fn add(&mut self) -> PResult<Expr> {
        self.chain(Self::add_links)
    }

    fn add_links(&mut self) -> PResult<Expr> {
        let mut l = self.mul()?;
        let mut links = 0;
        loop {
            self.next_link(&mut links)?;

            self.ws();
            if self.peek_at(1) == b'=' || self.peek_at(1) == self.peek() {
                break; // `+=` / `-=` assignment, `++` / `--` update
            }
            let op = match self.peek() {
                b'+' => BinOp::Add,
                b'-' => BinOp::Sub,
                _ => break,
            };
            self.pos += 1;
            let r = self.mul()?;
            l = Expr::Bin(op, Box::new(l), Box::new(r));
        }
        Ok(l)
    }

    fn mul(&mut self) -> PResult<Expr> {
        self.chain(Self::mul_links)
    }

    fn mul_links(&mut self) -> PResult<Expr> {
        let mut l = self.exp()?;
        let mut links = 0;
        loop {
            self.next_link(&mut links)?;

            self.ws();
            if self.peek_at(1) == b'=' && matches!(self.peek(), b'*' | b'/' | b'%') || self.starts("**=") {
                break; // `*=` / `/=` / `%=` / `**=` assignment
            }
            let op = match self.peek() {
                b'*' => BinOp::Mul,
                b'/' if self.peek_at(1) != b'/' && self.peek_at(1) != b'*' => BinOp::Div,
                b'%' => BinOp::Mod,
                _ => break,
            };
            self.pos += 1;
            let r = self.exp()?;
            l = Expr::Bin(op, Box::new(l), Box::new(r));
        }
        Ok(l)
    }

    /// `a ** b`: right-associative; a unary expression on its left is a SyntaxError (`-2 ** 2`).
    fn exp(&mut self) -> PResult<Expr> {
        self.nested(|p| p.exp_inner())
    }

    fn exp_inner(&mut self) -> PResult<Expr> {
        self.ws();
        let unary = matches!(self.peek(), b'-' | b'+' | b'!' | b'~') && !self.starts("--") && !self.starts("++")
            || ["typeof", "void", "delete", "await"]
                .iter()
                .any(|k| self.starts(k) && !is_ident(self.peek_at(k.len())));
        let l = self.unary()?;
        if self.eat_op("**") {
            if unary {
                return self.err("a unary expression cannot be the base of `**` (parenthesize it)");
            }
            let r = self.exp()?;
            return Ok(Expr::Bin(BinOp::Pow, Box::new(l), Box::new(r)));
        }
        Ok(l)
    }

    fn unary(&mut self) -> PResult<Expr> {
        self.nested(|p| p.unary_inner())
    }

    fn unary_inner(&mut self) -> PResult<Expr> {
        self.ws();
        if self.starts("await") && !is_ident(self.peek_at(5)) {
            // A frame that is not async cannot suspend: `await` there would never settle.
            if !self.in_async {
                return self.err("`await` is only valid in an async function");
            }
            self.pos += 5;
            return Ok(Expr::Await(Box::new(self.unary()?)));
        }
        if self.peek() == b'!' && self.peek_at(1) != b'=' {
            self.pos += 1;
            let e = self.unary()?;
            return Ok(Expr::Not(Box::new(self.cond(e))));
        }
        if self.starts("typeof") && !is_ident(self.peek_at(6)) {
            self.pos += 6;
            return Ok(Expr::Typeof(Box::new(self.unary()?)));
        }
        if self.starts("void") && !is_ident(self.peek_at(4)) {
            self.pos += 4;
            return Ok(Expr::Unary(UnaryOp::Void, Box::new(self.unary()?)));
        }
        if self.starts("delete") && !is_ident(self.peek_at(6)) {
            self.pos += 6;
            let e = self.unary()?;
            return match to_target(e) {
                Ok(Target::Attr(b, k)) => Ok(Expr::Mutate(
                    b,
                    Rc::from("__delete"),
                    vec![Expr::Lit(Value::Str(k.into()))],
                )),
                Ok(Target::Index(b, k)) => Ok(Expr::Mutate(b, Rc::from("__delete"), vec![k])),
                _ => self.err("`delete` takes a property (`delete o.k`, `delete o[k]`)"),
            };
        }
        if self.starts("++") || self.starts("--") {
            let delta = if self.starts("++") { 1 } else { -1 };
            self.pos += 2;
            let e = self.unary()?;
            let t = to_target(e).map_err(|m| {
                self.err::<()>(format!("{m} (the operand of a prefix ++/--)"))
                    .unwrap_err()
            })?;
            return Ok(self
                .const_assignment(&t, Expr::Lit(Value::Undef))
                .unwrap_or(Expr::Update(Box::new(t), delta, true)));
        }
        if self.peek() == b'-' {
            self.pos += 1;
            return Ok(Expr::Neg(Box::new(self.unary()?)));
        }
        if self.peek() == b'+' {
            self.pos += 1;
            return Ok(Expr::Unary(UnaryOp::Plus, Box::new(self.unary()?)));
        }
        if self.peek() == b'~' {
            self.pos += 1;
            return Ok(Expr::Unary(UnaryOp::BitNot, Box::new(self.unary()?)));
        }
        let e = self.postfix()?;
        // `x++` / `x--`: no line break before the operator (else it starts the next statement).
        let save = self.pos;
        self.ws();
        if (self.starts("++") || self.starts("--")) && !self.src[save..self.pos].contains(&b'\n') {
            let delta = if self.starts("++") { 1 } else { -1 };
            self.pos += 2;
            let t = to_target(e).map_err(|m| self.err::<()>(format!("{m} (the operand of ++/--)")).unwrap_err())?;
            return Ok(self
                .const_assignment(&t, Expr::Lit(Value::Undef))
                .unwrap_or(Expr::Update(Box::new(t), delta, false)));
        }
        self.pos = save;
        Ok(e)
    }

    pub(crate) fn args(&mut self) -> PResult<Vec<Expr>> {
        // after '('
        let mut args = vec![];
        if self.eat(")") {
            return Ok(args);
        }
        loop {
            if self.kwarg_ahead() {
                return self.err(crate::migrate::KWARG);
            }
            args.push(if self.eat("...") {
                Expr::Spread(Box::new(self.expr()?))
            } else {
                self.expr()?
            });
            if self.eat(")") {
                break;
            }
            self.expect(",")?;
            if self.eat(")") {
                break;
            }
        }
        Ok(args)
    }

    pub(crate) fn postfix(&mut self) -> PResult<Expr> {
        self.chain(Self::postfix_links)
    }

    fn postfix_links(&mut self) -> PResult<Expr> {
        let mut e = self.primary()?;
        // An optional link (`?.`) makes the rest of the chain short-circuit (`Expr::Chain`).
        let mut optional = false;
        let mut links = 0;
        loop {
            self.next_link(&mut links)?;

            // no whitespace skipping before postfix ops except '.'
            let save = self.pos;
            // TypeScript's non-null `x!` (no space before it) and `x as T` / `x satisfies T`: erased.
            if self.peek() == b'!' && self.peek_at(1) != b'=' {
                self.pos += 1;
                continue;
            }
            if self.eat("as") || self.eat("satisfies") {
                if !self.eat("const") {
                    self.ts_type()?;
                }
                continue;
            }
            self.pos = save;
            self.ws();
            match self.peek() {
                b'?' if self.peek_at(1) == b'.' && !self.peek_at(2).is_ascii_digit() => {
                    self.pos += 2;
                    optional = true;
                    match self.peek() {
                        b'(' => {
                            self.pos += 1;
                            let args = self.args()?;
                            e = Expr::OptApply(Box::new(e), args);
                        }
                        b'[' => {
                            self.pos += 1;
                            let i = self.expr()?;
                            self.expect("]")?;
                            e = Expr::OptIndex(Box::new(e), Box::new(i));
                        }
                        _ => {
                            let name = self.ident()?;
                            if self.peek() == b'(' {
                                self.pos += 1;
                                let args = self.args()?;
                                e = Expr::OptMethod(Box::new(e), name, args);
                            } else {
                                e = Expr::OptAttr(Box::new(e), name);
                            }
                        }
                    }
                }
                b'.' if is_ident_start(self.peek_at(1))
                    || (self.peek_at(1) == b'#' && is_ident_start(self.peek_at(2))) =>
                {
                    self.pos += 1;
                    // `this.#x`: a private member.
                    let private = self.peek() == b'#';
                    let name = if private {
                        self.pos += 1;
                        let name: Sym = Rc::from(format!("#{}", self.ident()?));
                        let Some((_, used)) = self.private_scopes.last_mut() else {
                            return self.err("private member access outside a class");
                        };
                        used.push(name.clone());
                        name
                    } else {
                        self.ident()?
                    };
                    self.try_type_args();
                    if self.peek() == b'(' {
                        self.pos += 1;
                        let args = self.args()?;
                        if private {
                            e = Expr::PrivateMethod(Box::new(e), name, args);
                            continue;
                        }
                        e = Expr::Method(Box::new(e), name, args);
                    } else {
                        e = if private {
                            Expr::Private(Box::new(e), name)
                        } else {
                            Expr::Attr(Box::new(e), name)
                        };
                    }
                }
                // `f(a)(b)`, `(x => x)(1)`, `fs[0]()`: a call of a computed function.
                b'(' => {
                    self.pos += 1;
                    let args = self.args()?;
                    e = Expr::Apply(Box::new(e), args);
                }
                b'[' => {
                    self.pos += 1;
                    if self.peek_after_ws() == b':' {
                        return self.err(crate::migrate::SLICE);
                    }
                    let i = self.expr()?;
                    if self.peek_after_ws() == b':' {
                        return self.err(crate::migrate::SLICE);
                    }
                    self.expect("]")?;
                    e = Expr::Index(Box::new(e), Box::new(i));
                }
                _ => {
                    self.pos = save;
                    break;
                }
            }
        }
        Ok(if optional { Expr::Chain(Box::new(e)) } else { e })
    }

    pub(crate) fn primary(&mut self) -> PResult<Expr> {
        self.ws();
        let c = self.peek();
        if c == b'<' {
            return self.jsx();
        }
        if c == b'`' {
            return self.template(false);
        }
        // `new C(..)`: a class in scope, or a built-in constructor (`Error`, `Map`, ...).
        if self.starts("new") && !is_ident(self.peek_at(3)) {
            self.pos += 3;
            let name = self.ident()?;
            self.try_type_args();
            let args = if self.eat("(") { self.args()? } else { vec![] };
            return Ok(Expr::New(name, args));
        }
        if c == b'p' && self.peek_at(1) == b'`' {
            self.pos += 1;
            return self.template(true);
        }
        if c == b'(' {
            let start = self.pos;
            self.pos += 1;
            let mut e = self.expr()?;
            while self.eat(",") {
                let r = self.expr()?;
                e = Expr::Comma(Box::new(e), Box::new(r));
            }
            self.expect(")")?;
            self.last_paren = (start, self.pos);
            return Ok(e);
        }
        if c == b'/' {
            return self.regex_literal();
        }
        if c == b'"' || c == b'\'' || c == b'`' {
            let st = self.string_lit()?;
            if self.strict_tokens
                && let Some((sp, _)) = self.specials.iter().find(|(sp, _)| st.contains(sp.as_str()))
            {
                return self.err(format!(
                    "\"use strict_tokens\": special token {:?} written in a string; use tok.{}",
                    sp,
                    crate::emit::token_alias(sp).unwrap_or_default()
                ));
            }
            return Ok(Expr::Lit(Value::string(st)));
        }
        if c.is_ascii_digit() || (c == b'.' && self.peek_at(1).is_ascii_digit()) {
            return self.number_literal().map(Expr::Lit);
        }
        if c == b'[' {
            self.pos += 1;
            let mut items = vec![];
            if !self.eat("]") {
                loop {
                    if self.peek_after_ws() == b',' {
                        return self.err("array holes (`[a, , b]`) are not supported (sparse arrays): write undefined");
                    }
                    if self.eat("...") {
                        items.push(Expr::Spread(Box::new(self.expr()?)));
                    } else {
                        items.push(self.expr()?);
                    }
                    if self.eat("]") {
                        break;
                    }
                    self.expect(",")?;
                    if self.eat("]") {
                        break;
                    }
                }
            }
            return Ok(Expr::List(items));
        }
        if c == b'{' {
            self.pos += 1;
            let mut items = vec![];
            if !self.eat("}") {
                loop {
                    self.ws();
                    if self.eat("...") {
                        items.push((Expr::Spread(Box::new(self.expr()?)), Expr::Lit(Value::None)));
                        if self.eat("}") {
                            break;
                        }
                        self.expect(",")?;
                        if self.eat("}") {
                            break;
                        }
                        continue;
                    }
                    let k = if is_ident_start(self.peek()) {
                        let name = self.ident()?;
                        if self.peek_after_ws() == b',' || self.peek() == b'}' {
                            items.push((Expr::Lit(Value::Str(name.clone().into())), Expr::Var(name)));
                            if self.eat("}") {
                                break;
                            }
                            self.expect(",")?;
                            if self.eat("}") {
                                break;
                            }
                            continue;
                        }
                        // `{f(x) { .. }}` / `{async f() { .. }}`: a method (a function value;
                        // there is no `this`).
                        let is_async = &*name == "async" && is_ident_start(self.peek_after_ws());
                        let name = if is_async { self.ident()? } else { name };
                        if matches!(&*name, "get" | "set") && is_ident_start(self.peek_after_ws()) {
                            return self.err("getters and setters are not supported");
                        }
                        if self.peek_after_ws() == b'(' {
                            let (params, props, patterns) = self.tsx_params()?;
                            self.skip_type_annotation()?;
                            let body = self.tsx_function_body(&params, &props, patterns, is_async)?;
                            let f = Expr::Lambda(Rc::new(Self::non_arrow(self.lambda(
                                params,
                                props,
                                LambdaBody::Block(body),
                                is_async,
                            ))));
                            items.push((Expr::Lit(Value::Str(name.into())), f));
                            if self.eat("}") {
                                break;
                            }
                            self.expect(",")?;
                            if self.eat("}") {
                                break;
                            }
                            continue;
                        }
                        Expr::Lit(Value::Str(name.into()))
                    } else if self.peek() == b'[' {
                        self.pos += 1;
                        let k = self.expr()?;
                        self.expect("]")?;
                        k
                    } else if self.peek().is_ascii_digit() || self.peek() == b'.' {
                        // `{1: "a", 2.5: "b"}`: the number's string form is the key.
                        let n = self.number_literal()?;
                        Expr::Lit(Value::string(crate::jsvalue::to_string(&n)))
                    } else {
                        Expr::Lit(Value::string(self.string_lit()?))
                    };
                    self.expect(":")?;
                    let v = self.expr()?;
                    items.push((k, v));
                    if self.eat("}") {
                        break;
                    }
                    self.expect(",")?;
                    if self.eat("}") {
                        break;
                    }
                }
            }
            return Ok(Expr::Map(items));
        }
        if is_ident_start(c) {
            let id = self.ident()?;
            match &*id {
                "true" => return Ok(Expr::Lit(Value::Bool(true))),
                "false" => return Ok(Expr::Lit(Value::Bool(false))),
                "null" => return Ok(Expr::Lit(Value::None)),
                // `function (a) { .. }` / `function name(a) { .. }`: a function value.
                "function" => {
                    if self.eat("*") {
                        return self.err("generator functions are not supported");
                    }
                    if is_ident_start(self.peek_after_ws()) {
                        self.ident()?;
                    }
                    let (params, props, patterns) = self.tsx_params()?;
                    self.skip_type_annotation()?;
                    let body = self.tsx_function_body(&params, &props, patterns, false)?;
                    return Ok(Expr::Lambda(Rc::new(Self::non_arrow(self.lambda(
                        params,
                        props,
                        LambdaBody::Block(body),
                        false,
                    )))));
                }
                // `this`: a variable a `function` or a method binds when called; an arrow
                // captures the enclosing one.
                "this" => return Ok(Expr::Var(id)),
                // `super(..)` in a constructor; `super.m(..)` in a method.
                "super" => {
                    if self.peek_after_ws() == b'(' {
                        self.pos += 1;
                        let args = self.args()?;
                        return Ok(Expr::Call(id, args));
                    }
                    return Ok(Expr::Var(id));
                }
                "class" => return Ok(Expr::Class(Rc::new(self.class_def(false)?))),
                "yield" | "arguments" | "import" | "with" | "eval" => {
                    return self.err(format!("`{id}` is not supported in views"));
                }
                _ => {}
            }
            self.try_type_args();
            if self.peek() == b'(' {
                self.pos += 1;
                // textOrToken("<think>"): the tokenizer decides, so its string may name a special token.
                let strict = self.strict_tokens;
                if &*id == "textOrToken" {
                    self.strict_tokens = false;
                }
                let args = self.args();
                self.strict_tokens = strict;
                return Ok(Expr::Call(id, args?));
            }
            return Ok(Expr::Var(id));
        }
        self.err("expected expression")
    }

    /// TSX: `/pattern/flags` in operand position, compiled at load time.
    /// A numeric literal: decimal (`12`, `1.5`, `.5`, `5.`, `1e3`, `1_000`), `0x`/`0o`/`0b`.
    /// An integer literal beyond 2^53 is the double JavaScript reads it as.
    fn number_literal(&mut self) -> PResult<Value> {
        let st = self.pos;
        let digits = |p: &mut Self, ok: &dyn Fn(u8) -> bool| -> PResult<String> {
            let mut s = String::new();
            loop {
                let c = p.peek();
                if ok(c) {
                    s.push(c as char);
                    p.pos += 1;
                } else if c == b'_' && !s.is_empty() && ok(p.peek_at(1)) {
                    p.pos += 1;
                } else {
                    break;
                }
            }
            Ok(s)
        };
        let radix = match (self.peek(), self.peek_at(1)) {
            (b'0', b'x' | b'X') => Some(16),
            (b'0', b'o' | b'O') => Some(8),
            (b'0', b'b' | b'B') => Some(2),
            _ => None,
        };
        let v = if let Some(radix) = radix {
            self.pos += 2;
            let s = digits(self, &|c: u8| (c as char).is_digit(radix))?;
            if s.is_empty() {
                self.pos = st;
                return self.err("invalid numeric literal");
            }
            let f = s
                .chars()
                .fold(0f64, |acc, d| acc * radix as f64 + d.to_digit(radix).unwrap() as f64);
            crate::jsvalue::number(f)
        } else {
            if self.peek() == b'0' && self.peek_at(1).is_ascii_digit() {
                return self.err("legacy octal literals (a leading 0) are not allowed");
            }
            let int = digits(self, &|c: u8| c.is_ascii_digit())?;
            let mut text = int.clone();
            let mut float = false;
            if self.peek() == b'.' && self.peek_at(1) != b'.' {
                float = true;
                self.pos += 1;
                text.push('.');
                text.push_str(&digits(self, &|c: u8| c.is_ascii_digit())?);
            }
            if (self.peek() == b'e' || self.peek() == b'E')
                && (self.peek_at(1).is_ascii_digit()
                    || ((self.peek_at(1) == b'-' || self.peek_at(1) == b'+') && self.peek_at(2).is_ascii_digit()))
            {
                float = true;
                text.push('e');
                if self.peek_at(1) == b'-' {
                    text.push('-');
                }
                self.pos += if self.peek_at(1).is_ascii_digit() { 1 } else { 2 };
                text.push_str(&digits(self, &|c: u8| c.is_ascii_digit())?);
            }
            if text.starts_with('.') {
                text.insert(0, '0');
            }
            if text.ends_with('.') {
                text.push('0');
            }
            let f: f64 = text
                .parse()
                .map_err(|_| self.err::<()>("invalid numeric literal").unwrap_err())?;
            match (float, int.parse::<i64>()) {
                // `1.0` stays a float (it prints `1`; Python formatting helpers print `1.0`).
                (false, Ok(i)) if i.unsigned_abs() <= 1 << 53 => Value::Int(i),
                _ => Value::Float(f),
            }
        };
        if self.peek() == b'n' && !is_ident(self.peek_at(1)) {
            return self.err("BigInt literals (`10n`) are not supported");
        }
        if is_ident(self.peek()) {
            return self.err("an identifier starts immediately after a numeric literal");
        }
        Ok(v)
    }

    fn regex_literal(&mut self) -> PResult<Expr> {
        let start = self.pos;
        self.pos += 1;
        let mut in_class = false;
        loop {
            let c = self.peek();
            if self.eof() || c == b'\n' {
                self.pos = start;
                return self.err("unterminated regular expression literal");
            }
            self.pos += 1;
            match c {
                b'\\' => self.pos += 1,
                b'[' => in_class = true,
                b']' => in_class = false,
                b'/' if !in_class => break,
                _ => {}
            }
        }
        let source = std::str::from_utf8(&self.src[start + 1..self.pos - 1])
            .map_err(|_| "bad utf8")?
            .to_string();
        let fstart = self.pos;
        while is_ident(self.peek()) {
            self.pos += 1;
        }
        let flags = std::str::from_utf8(&self.src[fstart..self.pos]).unwrap().to_string();
        match crate::jsregex::compile(&source, &flags) {
            Ok(re) => Ok(Expr::Lit(Value::Regex(Rc::new(re)))),
            Err(e) => {
                self.pos = start;
                self.err(e)
            }
        }
    }

    pub(crate) fn string_lit(&mut self) -> PResult<String> {
        self.ws();
        let q = self.peek();
        if q != b'"' && q != b'\'' && q != b'`' {
            return self.err("expected string");
        }
        self.pos += 1;
        let mut out: Vec<u8> = vec![];
        loop {
            if self.eof() {
                return self.err("unterminated string");
            }
            let c = self.peek();
            self.pos += 1;
            if c == q {
                break;
            }
            if c == b'\\' {
                let e = self.peek();
                self.pos += 1;
                self.push_escape(e, &mut out)?;
            } else {
                out.push(c);
            }
        }
        String::from_utf8(out).or_else(|_| self.err("bad utf8"))
    }

    pub(crate) fn push_escape(&mut self, e: u8, out: &mut Vec<u8>) -> PResult<()> {
        match e {
            b'n' => out.push(b'\n'),
            b't' => out.push(b'\t'),
            b'r' => out.push(b'\r'),
            b'v' => out.push(0x0b),
            b'b' => out.push(0x08),
            b'f' => out.push(0x0c),
            b'0' if !self.peek().is_ascii_digit() => out.push(0),
            b'0'..=b'9' => return self.err("octal escape sequences are not allowed (strict mode)"),
            // A line continuation: `\` before a line terminator adds nothing.
            b'\n' => {}
            b'\r' => {
                if self.peek() == b'\n' {
                    self.pos += 1;
                }
            }
            b'x' => {
                let hex = self
                    .src
                    .get(self.pos..self.pos + 2)
                    .and_then(|h| std::str::from_utf8(h).ok())
                    .and_then(|h| u32::from_str_radix(h, 16).ok());
                let Some(cp) = hex else {
                    return self.err("bad \\x escape");
                };
                self.pos += 2;
                let mut b = [0u8; 4];
                out.extend_from_slice(char::from_u32(cp).unwrap().encode_utf8(&mut b).as_bytes());
            }
            b'u' => {
                // \u{XXXX} or \uXXXX; a surrogate pair written as two escapes is one code point.
                let mut cp = self.unicode_escape()?;
                if (0xD800..0xDC00).contains(&cp) && self.starts("\\u") {
                    let save = self.pos;
                    self.pos += 2;
                    let lo = self.unicode_escape()?;
                    if (0xDC00..0xE000).contains(&lo) {
                        cp = 0x10000 + ((cp - 0xD800) << 10) + (lo - 0xDC00)
                    } else {
                        self.pos = save
                    }
                }
                let Some(ch) = char::from_u32(cp) else {
                    return self.err("a lone surrogate (\\uD800-\\uDFFF) is not supported: strings are Unicode text");
                };
                let mut b = [0u8; 4];
                out.extend_from_slice(ch.encode_utf8(&mut b).as_bytes());
            }
            other => out.push(other),
        }
        Ok(())
    }

    /// After `\u`: `XXXX` or `{X..}`, the code unit or code point.
    fn unicode_escape(&mut self) -> PResult<u32> {
        let hex: String;
        if self.peek() == b'{' {
            let end = find(&self.src[self.pos..], b"}").ok_or("bad \\u escape")?;
            hex = String::from_utf8_lossy(&self.src[self.pos + 1..self.pos + end]).into();
            self.pos += end + 1;
        } else {
            hex = String::from_utf8_lossy(&self.src[self.pos..(self.pos + 4).min(self.src.len())]).into();
            if hex.len() != 4 {
                return self.err("bad \\u escape");
            }
            self.pos += 4;
        }
        let cp = u32::from_str_radix(&hex, 16).map_err(|_| "bad \\u escape")?;
        if cp > 0x10FFFF {
            return self.err("bad \\u escape");
        }
        Ok(cp)
    }

    pub(crate) fn attrs(&mut self) -> PResult<(Vec<Attr>, bool)> {
        let mut attrs = vec![];
        loop {
            self.ws();
            if self.starts("/>") {
                self.pos += 2;
                return Ok((attrs, true));
            }
            if self.peek() == b'>' {
                self.pos += 1;
                return Ok((attrs, false));
            }
            if self.starts("{...") {
                self.pos += 4;
                let e = self.expr()?;
                self.expect("}")?;
                attrs.push(Attr {
                    name: Rc::from("..."),
                    value: AttrVal::Spread(e),
                });
                continue;
            }
            let st = self.pos;
            while is_ident(self.peek()) || self.peek() == b'-' || self.peek() == b':' {
                self.pos += 1;
            }
            if st == self.pos {
                return self.err("expected attribute");
            }
            let name: Sym = Rc::from(std::str::from_utf8(&self.src[st..self.pos]).unwrap());
            self.ws();
            let value = if self.peek() == b'=' {
                self.pos += 1;
                self.ws();
                if self.peek() == b'{' {
                    self.pos += 1;
                    let e = self.expr()?;
                    self.expect("}")?;
                    AttrVal::Expr(e)
                } else {
                    AttrVal::Str(Rc::from(self.string_lit()?))
                }
            } else {
                AttrVal::Bare
            };
            attrs.push(Attr { name, value });
        }
    }
}

pub(crate) fn split_specials(t: &str, specials: &[(String, u32)], nodes: &mut Vec<Node>) {
    let b = t.as_bytes();
    let mut last = 0;
    let mut i = 0;
    while i < b.len() {
        if let Some((s, id)) = specials.iter().find(|(s, _)| b[i..].starts_with(s.as_bytes())) {
            if i > last {
                nodes.push(Node::Text(Rc::from(&t[last..i])));
            }
            nodes.push(Node::Special {
                text: Rc::from(s.as_str()),
                id: std::cell::Cell::new(Some(*id)),
            });
            i += s.len();
            last = i;
        } else {
            i += 1;
        }
    }
    if last < b.len() {
        nodes.push(Node::Text(Rc::from(&t[last..])));
    }
}

impl Parser<'_> {
    /// Declare names in the innermost scope (`const`s cannot be assigned).
    pub(crate) fn declare(&mut self, names: impl IntoIterator<Item = Sym>, is_const: bool) {
        if let Some(s) = self.decls.last_mut() {
            s.extend(names.into_iter().map(|n| (n, is_const)));
        }
    }

    /// Assigning `t` assigns a `const` binding: an expression that evaluates `value` and then
    /// throws JavaScript's TypeError.
    pub(crate) fn const_assignment(&self, t: &Target, value: Expr) -> Option<Expr> {
        let Target::Var(n) = t else { return None };
        let is_const = self
            .decls
            .iter()
            .rev()
            .find_map(|s| s.iter().rev().find(|(m, _)| m == n).map(|(_, c)| *c))
            .unwrap_or(false);
        is_const.then(|| {
            Expr::Call(
                Rc::from("__constAssign"),
                vec![Expr::Lit(Value::Str(n.clone().into())), value],
            )
        })
    }
}

pub(crate) fn to_target(e: Expr) -> Result<Target, String> {
    match e {
        Expr::Var(v) => Ok(Target::Var(v)),
        Expr::Attr(b, n) => Ok(Target::Attr(Box::new(to_base(*b)), n)),
        Expr::Private(b, n) => Ok(Target::Private(Box::new(to_base(*b)), n)),
        Expr::Index(b, i) => Ok(Target::Index(Box::new(to_base(*b)), *i)),
        _ => Err("invalid assignment target".into()),
    }
}

/// The object a member assignment writes to: a place, or any other expression (`f().x = v`).
fn to_base(e: Expr) -> Target {
    match e {
        Expr::Var(_) | Expr::Attr(..) | Expr::Private(..) | Expr::Index(..) => to_target(e).unwrap(),
        e => Target::Expr(Box::new(e)),
    }
}

pub(crate) fn find(h: &[u8], n: &[u8]) -> Option<usize> {
    h.windows(n.len()).position(|w| w == n)
}
