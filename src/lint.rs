//! `affordance lint`: flag Python/Jinja idioms in view sources before they fail a render.
//!
//! Every `.tsx` file is parsed (a parse error is a finding: a view that does not load is the worst
//! case) and its syntax tree is checked against `migrate.rs`: calls to removed builtins
//! (`enumerate(..)`), certain Python methods (`.strip()`), Python names (`True`), and `+` with an
//! array literal. Markdown files contribute their ```tsx code blocks; a block that is a fragment
//! of a program only reports the idioms (`x[a:b]`, `f(k=v)`, ...) the parser names.
//! A Markdown file marked `<!-- affordance-lint: historical -->` (a design history) is skipped.

use crate::ast::*;
use crate::migrate;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// What `affordance lint` and `tests/lint.rs` check by default: the views, test fixtures, the spec, the
/// docs and README, and the out-of-tree consumers listed in `tools/view_consumers.txt` (the
/// directories that exist on this machine).
pub fn default_paths(root: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = ["views", "tests/fixtures", "spec", "docs", "README.md"]
        .iter()
        .map(|p| root.join(p))
        .collect();
    v.extend(consumers(root));
    v
}

/// Out-of-tree directories whose views depend on this language (`tools/view_consumers.txt`, one
/// path per line, relative to the repository root or absolute; `#` comments). Missing ones are
/// skipped.
pub fn consumers(root: &Path) -> Vec<PathBuf> {
    let Ok(text) = std::fs::read_to_string(root.join("tools/view_consumers.txt")) else {
        return vec![];
    };
    text.lines()
        .map(|l| l.split('#').next().unwrap_or("").trim())
        .filter(|l| !l.is_empty())
        .map(|l| root.join(l))
        .filter(|p| p.exists())
        .collect()
}

#[derive(Debug, Clone)]
pub struct Finding {
    pub file: String,
    pub line: u32,
    pub message: String,
}

impl std::fmt::Display for Finding {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.line > 0 {
            write!(f, "{}:{}: {}", self.file, self.line, self.message)
        } else {
            write!(f, "{}: {}", self.file, self.message)
        }
    }
}

/// Lint files and directories: `.tsx` files, and the ```tsx blocks of `.md` files.
pub fn lint_paths(paths: &[PathBuf]) -> Vec<Finding> {
    let mut files = vec![];
    for p in paths {
        collect(p, &mut files);
    }
    files.sort();
    files.dedup();
    let mut out = vec![];
    for f in &files {
        match f.extension().and_then(|e| e.to_str()) {
            Some("tsx") => out.extend(lint_file(f)),
            Some("md") => out.extend(lint_markdown(f)),
            _ => {}
        }
    }
    out
}

fn collect(p: &Path, out: &mut Vec<PathBuf>) {
    if p.is_dir() {
        let Ok(rd) = std::fs::read_dir(p) else { return };
        for e in rd.flatten() {
            let q = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            if q.is_dir() && (name.starts_with('.') || name == "target" || name == "node_modules") {
                continue;
            }
            collect(&q, out);
        }
    } else if p.extension().is_some_and(|e| e == "tsx" || e == "md") {
        out.push(p.to_path_buf());
    }
}

fn parse(src: &str, file: &str) -> Result<crate::parser::Module, String> {
    crate::parser::Parser::new(src, file).module_tsx()
}

/// One `.tsx` file: it must parse, and its tree must be free of Python idioms.
pub fn lint_file(path: &Path) -> Vec<Finding> {
    let file = path.display().to_string();
    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            return vec![Finding {
                file,
                line: 0,
                message: e.to_string(),
            }];
        }
    };
    let m = match parse(&src, &file) {
        Ok(m) => m,
        Err(e) => return vec![parse_finding(&file, &e, 0)],
    };
    let mut defined = HashSet::new();
    defined_names(&m, &mut defined);
    let mut l = Linter {
        file,
        line: 0,
        defined,
        out: vec![],
    };
    l.module(&m);
    l.out
}

/// A parse error as a finding (`file:line: message (near ...)` becomes line + message).
fn parse_finding(file: &str, e: &str, line_offset: u32) -> Finding {
    let rest = e.strip_prefix(file).and_then(|r| r.strip_prefix(':')).unwrap_or(e);
    let (line, msg) = match rest.split_once(':') {
        Some((n, m)) if n.trim().parse::<u32>().is_ok() => {
            (n.trim().parse::<u32>().unwrap() + line_offset, m.trim().to_string())
        }
        _ => (line_offset, rest.trim().to_string()),
    };
    Finding {
        file: file.to_string(),
        line,
        message: format!("does not parse: {msg}"),
    }
}

/// Only this file's declarations and explicit imports are in its lexical scope.
fn defined_names(m: &crate::parser::Module, out: &mut HashSet<String>) {
    out.extend(m.funcs.keys().map(|k| k.to_string()));
    out.extend(m.consts.iter().map(|(k, _)| k.to_string()));
    out.extend(
        m.imports
            .iter()
            .flat_map(|i| i.bindings.iter().map(|(_, n)| n.to_string())),
    );
}

/// The ```tsx / ```ts / ```jsx blocks of a Markdown file. A block is linted as a module, else as
/// a function body, else as an expression; if none parses, only a migration error is reported.
pub fn lint_markdown(path: &Path) -> Vec<Finding> {
    let file = path.display().to_string();
    let Ok(text) = std::fs::read_to_string(path) else {
        return vec![];
    };
    // A record of how things were (a design history): its examples are not current code.
    if text.contains("<!-- affordance-lint: historical -->") {
        return vec![];
    }
    let mut out = vec![];
    let mut block: Option<(u32, String)> = None;
    for (i, line) in text.lines().enumerate() {
        let n = i as u32 + 1;
        let t = line.trim_start();
        match &mut block {
            None if t.starts_with("```tsx") || t.starts_with("```ts") || t.starts_with("```jsx") => {
                block = Some((n, String::new()))
            }
            Some((start, body)) if t.starts_with("```") => {
                out.extend(lint_snippet(&file, *start, body));
                block = None;
            }
            Some((_, body)) => {
                body.push_str(line);
                body.push('\n');
            }
            None => {}
        }
    }
    out
}

fn lint_snippet(file: &str, start: u32, body: &str) -> Vec<Finding> {
    let attempts = [
        body.to_string(),
        format!("function __snippet() {{\n{body}\n}}"),
        format!("const __snippet = (\n{body}\n);"),
    ];
    let mut migration_error = None;
    for (k, src) in attempts.iter().enumerate() {
        match parse(src, file) {
            Ok(m) => {
                let mut l = Linter {
                    file: file.to_string(),
                    line: 0,
                    defined: m.funcs.keys().map(|k| k.to_string()).collect(),
                    out: vec![],
                };
                l.module(&m);
                // Lines inside the block, from the block's first line.
                for f in &mut l.out {
                    f.line = if f.line == 0 {
                        start
                    } else {
                        start + f.line - (k > 0) as u32
                    };
                }
                return l.out;
            }
            Err(e) if is_migration_error(&e) && migration_error.is_none() => {
                migration_error = Some(parse_finding(file, &e, start - (k > 0) as u32))
            }
            Err(_) => {}
        }
    }
    migration_error.into_iter().collect()
}

fn is_migration_error(e: &str) -> bool {
    [migrate::SLICE, migrate::KWARG, migrate::NOT_IN, migrate::AND_OR]
        .iter()
        .any(|m| e.contains(m))
}

struct Linter {
    file: String,
    line: u32,
    defined: HashSet<String>,
    out: Vec<Finding>,
}

impl Linter {
    fn flag(&mut self, message: String) {
        self.out.push(Finding {
            file: self.file.clone(),
            line: self.line,
            message,
        });
    }

    fn module(&mut self, m: &crate::parser::Module) {
        for (_, e) in &m.consts {
            self.line = 0;
            self.expr(e);
        }
        let mut funcs: Vec<_> = m.funcs.values().collect();
        funcs.sort_by_key(|f| f.loc.line);
        for f in funcs {
            self.line = f.loc.line;
            self.params(&f.params);
            if let Some(pp) = &f.props {
                self.params(pp);
            }
            self.stmts(&f.body);
        }
    }

    fn params(&mut self, ps: &[Param]) {
        for p in ps {
            // A parameter or a local named `max`, `first` or `keys` is the app's own: calling it
            // is not a Python idiom.
            self.defined.insert(p.name.to_string());
            if let Some(d) = &p.default {
                self.expr(d);
            }
        }
    }

    fn stmts(&mut self, b: &[Stmt]) {
        for s in b {
            self.stmt(s);
        }
    }

    fn stmt(&mut self, s: &Stmt) {
        match s {
            Stmt::Line(n) => self.line = *n,
            Stmt::Let(n, e) | Stmt::Init(n, e) => {
                self.defined.insert(n.to_string());
                self.expr(e)
            }
            Stmt::Expr(e) | Stmt::Return(e) | Stmt::Throw(e) => self.expr(e),
            Stmt::LetPattern(p, e, _) => {
                self.pattern(p);
                self.expr(e);
            }
            Stmt::Block(b) => self.stmts(b),
            Stmt::Labeled(_, s) => self.stmt(s),
            Stmt::DoWhile(b, e) => {
                self.stmts(b);
                self.expr(e);
            }
            Stmt::Declare(_) => {}
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
            Stmt::While(e, b) => {
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
                    for v in vals {
                        self.expr(v);
                    }
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
            Stmt::Break(_) | Stmt::Continue(_) => {}
        }
    }

    fn pattern(&mut self, p: &Pattern) {
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

    fn target(&mut self, t: &Target) {
        match t {
            Target::Temp(_) => {}
            Target::Var(_) => {}
            Target::Attr(b, _) | Target::Private(b, _) => self.target(b),
            Target::Index(b, e) => {
                self.target(b);
                self.expr(e);
            }
            Target::Expr(e) => self.expr(e),
        }
    }

    fn expr(&mut self, e: &Expr) {
        // What this form itself is checked for, then everything inside it.
        match e {
            Expr::Var(n) => {
                if let Some(js) = migrate::name(n)
                    && !self.defined.contains(&**n)
                {
                    self.flag(format!("`{n}` is Python; use {js}"));
                }
            }
            Expr::Call(name, _) => {
                if let Some(hint) = migrate::function(name)
                    && !self.defined.contains(&**name)
                {
                    self.flag(format!("`{name}(..)` is Python/Jinja, not a function here; use {hint}"));
                }
            }
            Expr::PrivateMethod(_, name, _) | Expr::Method(_, name, _) | Expr::OptMethod(_, name, _) => {
                if let Some(hint) = migrate::method(name).filter(|_| migrate::certain_method(name)) {
                    self.flag(format!("`.{name}(..)` is a Python method; use {hint}"));
                }
            }
            Expr::Bin(BinOp::Add, a, b) if matches!(**a, Expr::List(_)) || matches!(**b, Expr::List(_)) => {
                self.flag(migrate::LIST_ADD.to_string());
            }
            Expr::Class(c) => {
                for l in c.ctor.iter().chain(c.members.iter().filter_map(|m| m.value.as_ref())) {
                    self.expr(&Expr::Lambda(l.clone()));
                }
            }
            Expr::Lambda(l) => {
                self.params(&l.params);
                if let Some(pp) = &l.props {
                    self.params(pp);
                }
                match &l.body {
                    LambdaBody::Expr(e) => self.expr(e),
                    LambdaBody::Block(b) => {
                        let line = self.line;
                        self.stmts(b);
                        self.line = line;
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

    fn nodes(&mut self, ns: &[Node]) {
        for n in ns {
            match n {
                Node::Text(_) | Node::Special { .. } => {}
                Node::DynSpecial(e) | Node::TokId(e) | Node::Child(e) => self.expr(e),
                Node::Builtin { attrs, children, .. } | Node::Call { attrs, children, .. } => {
                    for a in attrs {
                        if let AttrVal::Expr(e) | AttrVal::Spread(e) = &a.value {
                            self.expr(e);
                        }
                    }
                    self.nodes(children);
                }
            }
        }
    }
}
