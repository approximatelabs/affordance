//! The view language: TypeScript/JSX with JavaScript semantics (`jsvalue.rs`).
//!
//! - Components are functions of state: `function Name({a, b = 1}) { ... return <>...</>; }`
//!   or `const Name = ({a}) => p`...`;`. The root function receives the whole state as props.
//! - JSX elements are values; children follow React rules (arrays splice, booleans/null vanish).
//! - Prompt text is written in tagged template literals ``p`<|im_start|>${role}\n...`` whose
//!   literal parts are split on the tokenizer's special tokens at load time.
//! - Statements: const/let/var (with destructuring patterns), blocks, function declarations,
//!   if/else, for..of, for..in, C-style for, while, do..while, labels, switch,
//!   try/catch/finally, throw, return, break/continue; expressions with assignments. A
//!   statement ends at `;`, a line break or `}`. Changing array methods mutate their shared
//!   receiver within a run; retained values are frozen at runtime boundaries. Blocks declare
//!   lexical names on entry, including their temporal dead zone, and initialize functions first.

use crate::ast::*;
use crate::parser::{
    BUILTIN_TAGS, Import, Module, PResult, Parser, is_ident, is_ident_start, split_specials, to_target,
};
use crate::value::Value;
use std::rc::Rc;

/// A TypeScript type, as written (types are erased; the shape is kept for tool schemas).
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum TsType {
    Ref(String, Vec<TsType>),
    Lit(serde_json::Value),
    Object(Vec<TsMember>),
    Array(Box<TsType>),
    Tuple(Vec<TsType>),
    Union(Vec<TsType>),
    Intersection(Vec<TsType>),
    Func,
    Other,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct TsMember {
    pub name: String,
    pub optional: bool,
    pub ty: TsType,
}

impl<'a> Parser<'a> {
    pub fn module_tsx(&mut self) -> PResult<Module> {
        let mut m = Module::default();
        loop {
            self.ws();
            if self.eof() {
                break;
            }
            if self.eat("import") {
                if let Some(import) = self.module_import()? {
                    for (_, local) in &import.bindings {
                        self.module_declaration(&m, local)?;
                    }
                    m.imports.push(import);
                }
                continue;
            }
            if self.peek() == b'"' || self.peek() == b'\'' {
                // directive: "use safe_text";
                let d = self.string_lit()?;
                self.eat(";");
                if let Some(p) = d.strip_prefix("use ") {
                    if p == "strict_tokens" {
                        self.strict_tokens = true;
                    }
                    m.pragmas.push(p.to_string());
                }
                continue;
            }
            if self.ts_declaration()? {
                continue;
            }
            let decl_start = self.pos;
            let exported = self.eat("export");
            if exported && self.ts_declaration()? {
                continue;
            }
            let is_default = exported && self.eat("default");
            if exported && !is_default && self.eat("{") {
                while !self.eat("}") {
                    let local = self.ident()?;
                    let public = if self.eat("as") { self.ident()? } else { local.clone() };
                    self.module_export(&mut m, public, local)?;
                    if !self.eat(",") {
                        self.expect("}")?;
                        break;
                    }
                }
                if self.eat("from") {
                    return self.err("re-exports with `from` are not supported; import the binding, then export it");
                }
                self.eat(";");
                continue;
            }
            let save = self.pos;
            let mut is_async = self.eat("async");
            if is_async && {
                self.ws();
                !self.starts("function") || is_ident(self.peek_at(8))
            } {
                self.pos = save;
                is_async = false;
            }
            if self.eat("function") {
                let loc = self.loc();
                let name = self.ident()?;
                self.module_declaration(&m, &name)?;
                if exported {
                    self.module_export(
                        &mut m,
                        if is_default { Rc::from("default") } else { name.clone() },
                        name.clone(),
                    )?;
                }
                let doc = self.doc_before(decl_start);
                self.skip_type_params()?;
                let (params, props, patterns) = self.tsx_params()?;
                let props_type = self.props_type.take().map(Rc::new);
                self.skip_type_annotation()?;
                let body = self.tsx_function_body(&params, &props, patterns, is_async)?;
                let uses_this = {
                    let mut n = crate::ast::Names(vec![]);
                    n.stmts(&body);
                    n.0.iter().any(|c| &**c == "this")
                };
                m.funcs.insert(
                    name.clone(),
                    Rc::new(Func {
                        code: Default::default(),
                        name,
                        params,
                        props,
                        body,
                        is_async,
                        loc,
                        doc,
                        props_type,
                        arrow: false,
                        uses_this,
                    }),
                );
                continue;
            }
            if self.eat("class") {
                let def = self.class_def(true)?;
                let name = def.name.clone().unwrap();
                self.module_declaration(&m, &name)?;
                if exported {
                    self.module_export(
                        &mut m,
                        if is_default { Rc::from("default") } else { name.clone() },
                        name.clone(),
                    )?;
                }
                m.consts.push((name, Expr::Class(Rc::new(def))));
                continue;
            }
            if self.eat("const") || self.eat("let") {
                let loc = self.loc();
                let name = self.ident()?;
                self.module_declaration(&m, &name)?;
                if exported {
                    self.module_export(
                        &mut m,
                        if is_default { Rc::from("default") } else { name.clone() },
                        name.clone(),
                    )?;
                }
                self.skip_type_annotation()?;
                self.expect("=")?;
                let e = self.expr()?;
                self.eat(";");
                match e {
                    Expr::Lambda(l) => {
                        let l = Rc::try_unwrap(l).map_err(|_| "internal: shared lambda")?;
                        let body = match l.body {
                            LambdaBody::Block(b) => b,
                            LambdaBody::Expr(e) => vec![Stmt::Return(e)],
                        };
                        let params = l.params;
                        let doc = self.doc_before(decl_start);
                        m.funcs.insert(
                            name.clone(),
                            Rc::new(Func {
                                code: Default::default(),
                                name,
                                params,
                                props: l.props,
                                body,
                                is_async: l.is_async,
                                loc,
                                doc,
                                props_type: l.props_type,
                                arrow: l.arrow,
                                uses_this: l.uses_this,
                            }),
                        );
                    }
                    other => m.consts.push((name, other)),
                }
                continue;
            }
            return self.err("expected import, function, const, type or a \"use ...\" directive");
        }
        if let Some(e) = self.strict_error.take() {
            return Err(format!("{}: {}", self.file, e));
        }
        m.types = std::mem::take(&mut self.types);
        Ok(m)
    }

    fn module_declaration(&self, module: &Module, name: &Sym) -> PResult<()> {
        if module.funcs.contains_key(name)
            || module.consts.iter().any(|(n, _)| n == name)
            || module.imports.iter().flat_map(|i| &i.bindings).any(|(_, n)| n == name)
        {
            return self.err(format!("duplicate module binding `{name}`"));
        }
        Ok(())
    }

    fn module_export(&self, module: &mut Module, public: Sym, local: Sym) -> PResult<()> {
        if module.exports.insert(public.clone(), local).is_some() {
            return self.err(format!("duplicate export `{public}`"));
        }
        Ok(())
    }

    fn module_import(&mut self) -> PResult<Option<Import>> {
        let type_only = self.eat("type");
        self.ws();
        let mut bindings = vec![];
        if self.peek() != b'"' && self.peek() != b'\'' {
            let named = if self.peek() != b'{' {
                if self.peek() == b'*' {
                    return self.err("namespace imports are not supported; use named imports");
                }
                bindings.push((Rc::from("default"), self.ident()?));
                if self.eat(",") {
                    self.expect("{")?;
                    true
                } else {
                    false
                }
            } else {
                self.expect("{")?;
                true
            };
            if named {
                while !self.eat("}") {
                    let public = self.ident()?;
                    let local = if self.eat("as") { self.ident()? } else { public.clone() };
                    if bindings.iter().any(|(_, n)| n == &local) {
                        return self.err(format!("duplicate import binding `{local}`"));
                    }
                    bindings.push((public, local));
                    if !self.eat(",") {
                        self.expect("}")?;
                        break;
                    }
                }
            }
            self.expect("from")?;
        }
        let path = self.string_lit()?;
        self.eat(";");
        if !path.starts_with("./")
            && !path.starts_with("../")
            && !path.starts_with('/')
            && !path.starts_with("affordance:")
            && !path.starts_with("@affordance/")
        {
            return self.err(format!(
                "cannot import \"{path}\": use a relative or absolute file path, or @affordance/ for the library; other packages are not supported"
            ));
        }
        if path.ends_with(".tss") && !bindings.is_empty() {
            return self.err("style sheets support only side-effect imports");
        }
        Ok((!type_only).then_some(Import { path, bindings }))
    }

    /// `type X<T> = ...;`, `interface X<T> extends Y { ... }` and `declare ...`: parsed, erased.
    fn ts_declaration(&mut self) -> PResult<bool> {
        let save = self.pos;
        if self.eat("type") {
            if self.peek_after_ws() == b'(' || self.peek_after_ws() == b'=' {
                self.pos = save;
                return Ok(false);
            }
            let name = self.ident()?;
            self.skip_type_params()?;
            self.expect("=")?;
            let t = self.ts_type()?;
            self.eat(";");
            self.types.insert(name.to_string(), t);
            return Ok(true);
        }
        if self.eat("interface") {
            let name = self.ident()?;
            self.skip_type_params()?;
            let extends = self.peek_after_ws() == b'e' && self.starts("extends");
            if self.eat("extends") {
                loop {
                    self.ts_type_ref()?;
                    if !self.eat(",") {
                        break;
                    }
                }
            }
            if self.peek_after_ws() != b'{' {
                return self.err("expected `{` after an interface name");
            }
            let t = self.ts_type()?;
            self.eat(";");
            // An interface that extends another is not mapped to a schema (its members are not all here).
            self.types
                .insert(name.to_string(), if extends { TsType::Other } else { t });
            return Ok(true);
        }
        Ok(false)
    }

    /// `Name<Args>` (an `extends` clause entry).
    fn ts_type_ref(&mut self) -> PResult<()> {
        self.ident()?;
        while self.peek() == b'.' {
            self.pos += 1;
            self.ident()?;
        }
        if self.peek_after_ws() == b'<' {
            self.pos += 1;
            loop {
                self.ts_type()?;
                if !self.eat(",") {
                    break;
                }
            }
            self.expect(">")?;
        }
        Ok(())
    }

    /// The doc comment a declaration starting at `start` takes: the last `/** */` if only
    /// whitespace separates them.
    pub(crate) fn doc_before(&mut self, start: usize) -> Option<Rc<str>> {
        let (end, text) = self.last_doc.clone()?;
        (end <= start && self.src[end..start].iter().all(|c| c.is_ascii_whitespace()) && !text.is_empty())
            .then_some(text)
    }

    /// Skip a `: Type` annotation. Types are parsed (`ts_type`) and erased.
    pub(crate) fn skip_type_annotation(&mut self) -> PResult<()> {
        if self.peek_after_ws() != b':' {
            return Ok(());
        }
        self.pos += 1;
        self.ts_type().map(|_| ())
    }

    /// `<T, U extends X = Y>` type parameters (or call-site type arguments), skipped.
    pub(crate) fn skip_type_params(&mut self) -> PResult<()> {
        if self.peek_after_ws() != b'<' {
            return Ok(());
        }
        self.pos += 1;
        loop {
            if self.eat(">") {
                return Ok(());
            }
            self.eat("const");
            self.ident()?;
            if self.eat("extends") {
                self.ts_type()?;
            }
            if self.eat("=") {
                self.ts_type()?;
            }
            if !self.eat(",") {
                return self.expect(">");
            }
        }
    }

    /// Call-site type arguments `f<T>(x)`: skipped when a call follows (else `<` is a comparison).
    pub(crate) fn try_type_args(&mut self) {
        if self.peek() != b'<' {
            return;
        }
        let save = self.pos;
        self.pos += 1;
        let ok = (|| -> PResult<()> {
            loop {
                self.ts_type()?;
                if !self.eat(",") {
                    return self.expect(">");
                }
            }
        })();
        if ok.is_err() || self.peek() != b'(' {
            self.pos = save;
        }
    }

    /// A type. Parsing one never changes which props type the enclosing function has: a function
    /// type inside it (`cb: ({q}: {q: string}) => void`) has parameters of its own.
    ///
    /// A TypeScript type: unions, intersections, arrays, tuples, object and function types,
    /// literals, generic references, `typeof` / `keyof`, indexed and conditional types.
    pub(crate) fn ts_type(&mut self) -> PResult<TsType> {
        let own = self.props_type.take();
        let parsed = self.nested(|p| p.ts_type_inner());
        self.props_type = own;
        parsed
    }

    fn ts_type_inner(&mut self) -> PResult<TsType> {
        self.eat("|");
        self.eat("&");
        let first = self.ts_intersection()?;
        let mut alts = vec![first];
        while self.peek_after_ws() == b'|' && self.peek_at(1) != b'|' {
            self.pos += 1;
            alts.push(self.ts_intersection()?);
        }
        let t = if alts.len() == 1 {
            alts.pop().unwrap()
        } else {
            TsType::Union(alts)
        };
        if self.eat("extends") {
            self.ts_type()?;
            self.expect("?")?;
            self.ts_type()?;
            self.expect(":")?;
            self.ts_type()?;
            return Ok(TsType::Other);
        }
        Ok(t)
    }

    fn ts_intersection(&mut self) -> PResult<TsType> {
        let first = self.ts_postfix()?;
        let mut parts = vec![first];
        while self.peek_after_ws() == b'&' && self.peek_at(1) != b'&' {
            self.pos += 1;
            parts.push(self.ts_postfix()?);
        }
        Ok(if parts.len() == 1 {
            parts.pop().unwrap()
        } else {
            TsType::Intersection(parts)
        })
    }

    fn ts_postfix(&mut self) -> PResult<TsType> {
        let mut t = self.ts_primary()?;
        // `T[]`, `T[K]` (no whitespace before `[`, as TypeScript reads a new line as a new statement).
        let mut links = 0;
        while self.peek() == b'[' {
            links += 1;
            if links > 256 {
                return self.err("type chain exceeds 256 links");
            }
            self.pos += 1;
            if self.eat("]") {
                t = TsType::Array(Box::new(t));
            } else {
                self.ts_type()?;
                self.expect("]")?;
                t = TsType::Other;
            }
        }
        Ok(t)
    }

    fn ts_primary(&mut self) -> PResult<TsType> {
        let c = self.peek_after_ws();
        if c == b'(' {
            // A function type `(a: T) => U`, or a parenthesized type.
            let save = self.pos;
            if self.tsx_params().is_ok() && self.eat("=>") {
                self.ts_type()?;
                return Ok(TsType::Func);
            }
            self.pos = save + 1;
            let t = self.ts_type()?;
            self.expect(")")?;
            return Ok(t);
        }
        if c == b'{' {
            self.pos += 1;
            let mut members = vec![];
            loop {
                if self.eat("}") {
                    return Ok(TsType::Object(members));
                }
                self.eat("readonly");
                if self.peek_after_ws() == b'[' {
                    // An index signature `[key: string]: T` or a mapped type `[K in T]: U`.
                    self.pos += 1;
                    self.ident()?;
                    if self.eat(":") || self.eat("in") {
                        self.ts_type()?;
                    }
                    self.expect("]")?;
                    self.eat("?");
                    self.expect(":")?;
                    self.ts_type()?;
                } else {
                    let name: String = match self.peek_after_ws() {
                        b'"' | b'\'' => self.string_lit()?,
                        c if c.is_ascii_digit() => {
                            let s = self.pos;
                            while self.peek().is_ascii_digit() {
                                self.pos += 1
                            }
                            String::from_utf8_lossy(&self.src[s..self.pos]).into_owned()
                        }
                        _ => self.ident()?.to_string(),
                    };
                    let optional = self.eat("?");
                    if matches!(self.peek_after_ws(), b'(' | b'<') {
                        // A method `m(a: T): U`.
                        self.skip_type_params()?;
                        self.tsx_params()?;
                        self.skip_type_annotation()?;
                        members.push(TsMember {
                            name,
                            optional,
                            ty: TsType::Func,
                        });
                    } else {
                        self.expect(":")?;
                        let ty = self.ts_type()?;
                        members.push(TsMember { name, optional, ty });
                    }
                }
                if !self.eat(";") && !self.eat(",") {
                    self.expect("}")?;
                    return Ok(TsType::Object(members));
                }
            }
        }
        if c == b'[' {
            self.pos += 1;
            let mut items = vec![];
            loop {
                if self.eat("]") {
                    return Ok(TsType::Tuple(items));
                }
                self.eat("...");
                // A labelled element `name: T` or `name?: T`.
                let save = self.pos;
                if self.ident().is_ok() && (self.eat(":") || (self.eat("?") && self.eat(":"))) {
                } else {
                    self.pos = save;
                }
                items.push(self.ts_type()?);
                self.eat("?");
                if !self.eat(",") {
                    self.expect("]")?;
                    return Ok(TsType::Tuple(items));
                }
            }
        }
        if c == b'"' || c == b'\'' {
            return Ok(TsType::Lit(serde_json::Value::String(self.string_lit()?)));
        }
        if c == b'`' {
            self.pos += 1;
            while !self.eof() && self.peek() != b'`' {
                self.pos += 1;
            }
            if self.eof() {
                return self.err("a template literal type is not closed");
            }
            self.pos += 1;
            return Ok(TsType::Other);
        }
        if c == b'-' || c.is_ascii_digit() {
            let s = self.pos;
            self.pos += 1;
            while self.peek().is_ascii_digit() || self.peek() == b'.' {
                self.pos += 1;
            }
            let Ok(n) = String::from_utf8_lossy(&self.src[s..self.pos]).parse::<f64>() else {
                return self.err("a bad number in a type");
            };
            return Ok(TsType::Lit(serde_json::json!(n)));
        }
        if self.eat("typeof") {
            self.ident()?;
            while self.peek() == b'.' {
                self.pos += 1;
                self.ident()?;
            }
            return Ok(TsType::Other);
        }
        if self.eat("keyof") || self.eat("readonly") || self.eat("unique") || self.eat("infer") {
            self.ts_postfix()?;
            return Ok(TsType::Other);
        }
        if self.eat("new") {
            self.tsx_params()?;
            self.expect("=>")?;
            self.ts_type()?;
            return Ok(TsType::Func);
        }
        let mut name = self.ident()?.to_string();
        while self.peek() == b'.' {
            self.pos += 1;
            name.push('.');
            name.push_str(&self.ident()?);
        }
        let mut args = vec![];
        if self.peek() == b'<' {
            self.pos += 1;
            loop {
                args.push(self.ts_type()?);
                if !self.eat(",") {
                    break;
                }
            }
            self.expect(">")?;
        }
        Ok(TsType::Ref(name, args))
    }

    /// `(a, b = 1)` positional params, or `({a, b = 1, children})` one destructured props object.
    /// Other patterns (`([a, b])`, `({a: x, ...rest})`) are temporary parameters destructured at
    /// the top of the body (the third result).
    pub(crate) fn tsx_params(&mut self) -> PResult<(Vec<Param>, Option<Vec<Param>>, Vec<(Sym, Pattern)>)> {
        self.expect("(")?;
        // `({a, b = 1})`: a component's props object.
        self.props_type = None;
        if self.peek_after_ws() == b'{' {
            let save = self.pos;
            if let Ok(pp) = self.props_pattern() {
                let typed = self.peek_after_ws() == b':';
                if typed {
                    self.pos += 1;
                    self.props_type = Some(self.ts_type()?);
                }
                if self.eat(")") {
                    return Ok((vec![], Some(pp), vec![]));
                }
            }
            self.pos = save;
        }
        let mut ps = vec![];
        let mut patterns = vec![];
        if self.eat(")") {
            return Ok((ps, None, patterns));
        }
        loop {
            let rest = self.eat("...");
            let name = if matches!(self.peek_after_ws(), b'[' | b'{') {
                let p = self.binding_pattern()?;
                let tmp: Sym = Rc::from(format!("__arg{}", ps.len()));
                patterns.push((tmp.clone(), p));
                tmp
            } else {
                self.ident()?
            };
            self.eat("?"); // TypeScript optional parameter
            self.skip_type_annotation()?;
            let default = if self.eat("=") { Some(self.expr()?) } else { None };
            ps.push(Param { name, default, rest });
            if self.eat(")") {
                break;
            }
            if rest {
                return self.err("a rest parameter must be the last parameter");
            }
            self.expect(",")?;
            if self.eat(")") {
                break;
            }
        }
        Ok((ps, None, patterns))
    }

    /// `{a, b = 1}` with plain names and defaults only (a props object).
    fn props_pattern(&mut self) -> PResult<Vec<Param>> {
        self.expect("{")?;
        let mut pp = vec![];
        loop {
            if self.eat("}") {
                break;
            }
            let name = self.ident()?;
            let default = if self.eat("=") { Some(self.expr()?) } else { None };
            pp.push(Param {
                name,
                default,
                rest: false,
            });
            if !self.eat(",") {
                self.expect("}")?;
                break;
            }
        }
        Ok(pp)
    }

    /// A binding pattern: a name, `[a, , b = 1, ...rest]` or `{a, b: c, [k]: d = 1, ...rest}`.
    pub(crate) fn binding_pattern(&mut self) -> PResult<Pattern> {
        self.nested(|p| p.binding_pattern_inner())
    }

    fn binding_pattern_inner(&mut self) -> PResult<Pattern> {
        match self.peek_after_ws() {
            b'[' => {
                self.pos += 1;
                let mut items = vec![];
                let mut rest = None;
                loop {
                    if self.eat("]") {
                        break;
                    }
                    if self.peek_after_ws() == b',' {
                        self.pos += 1;
                        items.push(None);
                        continue;
                    }
                    if self.eat("...") {
                        rest = Some(Box::new(self.binding_pattern()?));
                        self.expect("]")?;
                        break;
                    }
                    let p = self.binding_pattern()?;
                    let d = if self.eat("=") { Some(self.expr()?) } else { None };
                    items.push(Some((p, d)));
                    if !self.eat(",") {
                        self.expect("]")?;
                        break;
                    }
                }
                Ok(Pattern::Array(items, rest))
            }
            b'{' => {
                self.pos += 1;
                let mut items = vec![];
                let mut rest = None;
                loop {
                    if self.eat("}") {
                        break;
                    }
                    if self.eat("...") {
                        rest = Some(self.ident()?);
                        self.eat(",");
                        self.expect("}")?;
                        break;
                    }
                    self.ws();
                    let (key, shorthand) = if self.peek() == b'[' {
                        self.pos += 1;
                        let k = self.expr()?;
                        self.expect("]")?;
                        (k, None)
                    } else if self.peek() == b'"' || self.peek() == b'\'' {
                        (Expr::Lit(Value::string(self.string_lit()?)), None)
                    } else if self.peek().is_ascii_digit() {
                        let Expr::Lit(v) = self.primary()? else { unreachable!() };
                        (Expr::Lit(Value::string(crate::jsvalue::to_string(&v))), None)
                    } else {
                        let n = self.ident()?;
                        (Expr::Lit(Value::Str(n.clone().into())), Some(n))
                    };
                    let p = if self.eat(":") {
                        self.binding_pattern()?
                    } else {
                        match shorthand {
                            Some(n) => Pattern::Name(n),
                            None => return self.err("expected `:` after a computed or quoted key in a pattern"),
                        }
                    };
                    let d = if self.eat("=") { Some(self.expr()?) } else { None };
                    items.push((key, p, d));
                    if !self.eat(",") {
                        self.expect("}")?;
                        break;
                    }
                }
                Ok(Pattern::Object(items, rest))
            }
            _ => Ok(Pattern::Name(self.ident()?)),
        }
    }

    /// A function's body block, with its destructured parameters bound first and its `var`s
    /// declared at the top.
    pub(crate) fn tsx_function_body(
        &mut self,
        params: &[Param],
        props: &Option<Vec<Param>>,
        patterns: Vec<(Sym, Pattern)>,
        is_async: bool,
    ) -> PResult<Vec<Stmt>> {
        self.var_scopes.push(vec![]);
        self.decls.push(param_names(params, props, &patterns));
        let block = self.async_body(is_async, |p| p.tsx_block());
        self.decls.pop();
        let vars = self.var_scopes.pop().unwrap();
        let block = block?;
        let mut body: Vec<Stmt> = patterns
            .into_iter()
            .map(|(tmp, p)| Stmt::LetPattern(p, Expr::Var(tmp), false))
            .collect();
        // `var`s are function-scoped and read as undefined before their declaration runs.
        body.extend(vars.into_iter().map(|n| Stmt::Let(n, Expr::Lit(Value::Undef))));
        body.extend(block);
        Ok(body)
    }

    /// `f` parsing a function's own body: async or not, whatever encloses it.
    pub(crate) fn async_body<T>(&mut self, is_async: bool, f: impl FnOnce(&mut Self) -> PResult<T>) -> PResult<T> {
        let outer = std::mem::replace(&mut self.in_async, is_async);
        let result = f(self);
        self.in_async = outer;
        result
    }

    /// At a `(`: whether the text after its matching `)` starts with `=>` or `:` (a return type).
    /// Brackets are matched and string literals skipped; nothing is parsed.
    fn arrow_follows_parenthesis(&self) -> bool {
        let src = &self.src;
        let (mut i, mut depth) = (self.pos, 0usize);
        while i < src.len() {
            match src[i] {
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        let mut j = i + 1;
                        while j < src.len() && src[j].is_ascii_whitespace() {
                            j += 1;
                        }
                        return src[j..].starts_with(b"=>") || src.get(j) == Some(&b':');
                    }
                }
                quote @ (b'"' | b'\'' | b'`') => {
                    i += 1;
                    while i < src.len() && src[i] != quote {
                        i += if src[i] == b'\\' { 2 } else { 1 };
                    }
                }
                _ => {}
            }
            i += 1;
        }
        false
    }

    /// Arrow function: `x => e`, `(a, b) => e`, `({a}) => e`, with expression or block body.
    pub(crate) fn try_tsx_lambda(&mut self, is_async: bool) -> PResult<Option<Lambda>> {
        let save = self.pos;
        self.props_type = None;
        self.ws();
        // `<T,>(x: T) => ..`: a generic arrow (TSX writes `<T,>` so it is not an element).
        if self.peek() == b'<' && (self.skip_type_params().is_err() || self.peek_after_ws() != b'(') {
            self.pos = save;
            return Ok(None);
        }
        // A parenthesis is a parameter list only if `=>` (or a return type) follows its close.
        // Checked by scanning, before parsing it as one: parsing first and backing out parsed
        // every nested parenthesis twice, which doubled the time for each level of `(a = (a = ..))`.
        if self.peek() == b'(' && !self.arrow_follows_parenthesis() {
            self.pos = save;
            return Ok(None);
        }
        let parenthesized = self.peek() == b'(';
        let (params, props, patterns) = if self.peek() == b'(' {
            match self.tsx_params() {
                Ok(p) => p,
                Err(_) => {
                    self.pos = save;
                    return Ok(None);
                }
            }
        } else if is_ident_start(self.peek()) {
            (
                vec![Param {
                    name: self.ident()?,
                    default: None,
                    rest: false,
                }],
                None,
                vec![],
            )
        } else {
            return Ok(None);
        };
        let props_type = self.props_type.take();
        // A return type follows a parameter list only: after a bare `x`, a `:` belongs to what
        // encloses it (`flag ? id : x => x * 2`).
        let typed = if parenthesized {
            self.skip_type_annotation()
        } else {
            Ok(())
        };
        self.ws();
        if typed.is_err() || !self.starts("=>") {
            self.pos = save;
            return Ok(None);
        }
        self.pos += 2;
        let body = if self.peek_after_ws() == b'{' {
            LambdaBody::Block(self.tsx_function_body(&params, &props, patterns, is_async)?)
        } else {
            self.decls.push(param_names(&params, &props, &patterns));
            let e = self.async_body(is_async, |p| p.expr());
            self.decls.pop();
            let e = e?;
            if patterns.is_empty() {
                LambdaBody::Expr(e)
            } else {
                let mut stmts: Vec<Stmt> = patterns
                    .into_iter()
                    .map(|(tmp, p)| Stmt::LetPattern(p, Expr::Var(tmp), false))
                    .collect();
                stmts.push(Stmt::Return(e));
                LambdaBody::Block(stmts)
            }
        };
        let mut l = self.lambda(params, props, body, is_async);
        l.props_type = props_type.map(Rc::new);
        Ok(Some(l))
    }

    /// `class [Name] [extends Base] { members }`, after the `class` keyword. TypeScript's member
    /// modifiers (`public`, `readonly`, ..), type parameters, `implements` and annotations are
    /// skipped.
    pub(crate) fn class_def(&mut self, need_name: bool) -> PResult<crate::ast::ClassDef> {
        self.private_scopes.push((vec![], vec![]));
        let result = self.nested(|p| p.class_def_inner(need_name));
        let (declared, used) = self.private_scopes.pop().unwrap();
        let result = result?;
        for name in used {
            if declared.contains(&name) {
                continue;
            }
            if let Some((_, outer)) = self.private_scopes.last_mut() {
                outer.push(name);
            } else {
                return self.err(format!("private member {name} is not declared"));
            }
        }
        Ok(result)
    }

    fn class_def_inner(&mut self, need_name: bool) -> PResult<crate::ast::ClassDef> {
        use crate::ast::{Member, MemberKind};
        self.ws();
        let name = if is_ident_start(self.peek())
            && (!self.starts("extends") || is_ident(self.peek_at(7)))
            && (!self.starts("implements") || is_ident(self.peek_at(10)))
        {
            Some(self.ident()?)
        } else {
            None
        };
        if need_name && name.is_none() {
            return self.err("a class declaration needs a name");
        }
        self.skip_type_params()?;
        let parent = if self.eat("extends") {
            let p = self.ident()?;
            self.skip_type_params()?;
            Some(Expr::Var(p))
        } else {
            None
        };
        if self.eat("implements") {
            loop {
                self.ts_type()?;
                if !self.eat(",") {
                    break;
                }
            }
        }
        self.expect("{")?;
        let mut ctor = None;
        let mut members: Vec<Member> = vec![];
        loop {
            if self.eat("}") {
                break;
            }
            if self.eat(";") {
                continue;
            }
            if self.eof() {
                return self.err("expected `}` to end the class");
            }
            let mut is_static = false;
            loop {
                let save = self.pos;
                let Some(w) = self.peek_ident() else { break };
                if !matches!(
                    w.as_str(),
                    "static"
                        | "public"
                        | "private"
                        | "protected"
                        | "readonly"
                        | "declare"
                        | "abstract"
                        | "override"
                        | "accessor"
                ) {
                    break;
                }
                self.pos += w.len();
                // `static() {}` / `readonly = 1`: the word is the member's name, not a modifier.
                if matches!(self.peek_after_ws(), b'(' | b'=' | b';' | b':' | b'?' | b'!' | b'<') {
                    self.pos = save;
                    break;
                }
                if w == "static" {
                    if self.peek_after_ws() == b'{' {
                        return self.err("static blocks are not supported");
                    }
                    is_static = true;
                }
            }
            let save = self.pos;
            let mut is_async = false;
            if self.eat("async") && !matches!(self.peek_after_ws(), b'(' | b'=' | b';') {
                is_async = true
            } else if is_async {
            } else {
                self.pos = save
            }
            if self.eat("*") {
                return self.err("generator methods are not supported");
            }
            let mut kind = MemberKind::Method;
            let save = self.pos;
            if let Some(w) = self.peek_ident()
                && matches!(w.as_str(), "get" | "set")
            {
                self.pos += w.len();
                if matches!(self.peek_after_ws(), b'(' | b'=' | b';' | b':') {
                    self.pos = save
                } else {
                    kind = if w == "get" { MemberKind::Get } else { MemberKind::Set }
                }
            }
            self.ws();
            let private = self.peek() == b'#';
            let key: Sym = if self.peek() == b'#' {
                self.pos += 1;
                Rc::from(format!("#{}", self.ident()?))
            } else if self.peek() == b'"' || self.peek() == b'\'' {
                Rc::from(self.string_lit()?.as_str())
            } else if self.peek() == b'[' {
                return self.err("computed class member names are not supported");
            } else {
                self.ident()?
            };
            if private {
                self.private_scopes.last_mut().unwrap().0.push(key.clone());
            }
            self.eat("?");
            self.eat("!");
            if private {
                if key.as_ref() == "#constructor" {
                    return self.err("a private member cannot be named #constructor");
                }
                let actual = if matches!(self.peek_after_ws(), b'(' | b'<') {
                    kind
                } else {
                    MemberKind::Field
                };
                for previous in &members {
                    if previous.private
                        && previous.key == key
                        && !(previous.is_static == is_static
                            && matches!(
                                (previous.kind, actual),
                                (MemberKind::Get, MemberKind::Set) | (MemberKind::Set, MemberKind::Get)
                            ))
                    {
                        return self.err(format!("duplicate private member {key}"));
                    }
                }
            }
            if matches!(self.peek_after_ws(), b'(' | b'<') {
                self.skip_type_params()?;
                let (params, props, patterns) = self.tsx_params()?;
                self.skip_type_annotation()?;
                let body = self.tsx_function_body(&params, &props, patterns, is_async)?;
                let mut l = Self::non_arrow(self.lambda(params, props, LambdaBody::Block(body), is_async));
                l.name = Some(key.clone());
                if &*key == "constructor" && !is_static && kind == MemberKind::Method {
                    ctor = Some(Rc::new(l));
                } else {
                    members.push(Member {
                        kind,
                        is_static,
                        private,
                        key,
                        value: Some(Rc::new(l)),
                    });
                }
                continue;
            }
            // A field: `x = expr;`, `x;`, `x: T = expr;`.
            self.skip_type_annotation()?;
            let value = if self.eat("=") {
                let e = self.async_body(false, |p| p.expr())?;
                Some(Rc::new(Self::non_arrow(self.lambda(
                    vec![],
                    None,
                    LambdaBody::Expr(e),
                    false,
                ))))
            } else {
                None
            };
            self.eat(";");
            members.push(Member {
                kind: MemberKind::Field,
                is_static,
                private,
                key,
                value,
            });
        }
        Ok(crate::ast::ClassDef {
            name,
            parent,
            ctor,
            members,
        })
    }

    /// `l` as a `function` (or a method): it binds `this` itself, so `this` is not captured.
    pub(crate) fn non_arrow(mut l: Lambda) -> Lambda {
        l.arrow = false;
        l.uses_super = l.captures.iter().any(|c| &**c == "super");
        // `super.m()` runs on `this`, so a method that names `super` binds `this` too.
        l.uses_this = l.uses_super || l.captures.iter().any(|c| &**c == "this");
        l.captures.retain(|c| &**c != "this" && &**c != "super");
        l
    }

    /// A function value and the lexical names its closure captures.
    pub(crate) fn lambda(
        &mut self,
        params: Vec<Param>,
        props: Option<Vec<Param>>,
        body: LambdaBody,
        is_async: bool,
    ) -> Lambda {
        // What it captures: the names it mentions that are not its own. Its parameters are its own
        // everywhere in it, and so are the declarations at the top of its body (from the start
        // of the body: before its declaration a name is in its dead zone, not the outer one).
        let mut own: Vec<Sym> = params
            .iter()
            .chain(props.iter().flatten())
            .map(|p| p.name.clone())
            .collect();
        let mut names = Names(vec![]);
        names.params(&params);
        if let Some(pp) = &props {
            names.params(pp);
        }
        let mut captures: Vec<Sym> = names.0.into_iter().filter(|n| !own.contains(n)).collect();
        let mut names = Names(vec![]);
        match &body {
            LambdaBody::Expr(e) => names.expr(e),
            LambdaBody::Block(b) => {
                names.stmts(b);
                for s in b {
                    match s {
                        Stmt::Let(n, _) | Stmt::Init(n, _) => own.push(n.clone()),
                        Stmt::Declare(ns) => own.extend(ns.iter().cloned()),
                        Stmt::LetPattern(p, _, _) => p.names(&mut own),
                        _ => {}
                    }
                }
            }
        }
        for n in names.0 {
            if !own.contains(&n) && !captures.contains(&n) {
                captures.push(n)
            }
        }
        Lambda {
            code: Default::default(),
            module: self.file_sym.clone(),
            params,
            props,
            body,
            is_async,
            captures,
            name: None,
            doc: None,
            props_type: None,
            self_name: None,
            arrow: true,
            uses_this: false,
            uses_super: false,
        }
    }

    // ---------------- statements ----------------

    /// `{ statements }`. Declare lexical names on entry, including their temporal dead zone,
    /// and initialize function declarations first, as JavaScript hoists them.
    pub(crate) fn tsx_block(&mut self) -> PResult<Vec<Stmt>> {
        self.expect("{")?;
        self.block_funcs.push(vec![]);
        self.decls.push(vec![]);
        let mut v = vec![];
        let r = loop {
            if self.eat("}") {
                break Ok(());
            }
            if self.eof() {
                break self.err("expected `}`");
            }
            self.ws();
            v.push(Stmt::Line(self.line_no()));
            match self.tsx_stmt() {
                Ok(s) => v.extend(s),
                Err(e) => break Err(e),
            }
        };
        let funcs = self.block_funcs.pop().unwrap();
        self.decls.pop();
        r?;
        Ok(hoist(v, funcs))
    }

    fn tsx_body(&mut self) -> PResult<Vec<Stmt>> {
        if self.peek_after_ws() == b'{' {
            self.tsx_block()
        } else {
            self.tsx_stmt()
        }
    }

    /// The end of a statement: a `;`, or (automatic semicolon insertion) a line break, a `}` or
    /// the end of the input before the next token.
    fn end_stmt(&mut self) -> PResult<()> {
        let save = self.pos;
        self.ws();
        if self.peek() == b';' {
            self.pos += 1;
            return Ok(());
        }
        if self.eof() || self.peek() == b'}' || self.src[save..self.pos].contains(&b'\n') {
            return Ok(());
        }
        self.err("expected `;` or a line break after the statement")
    }

    /// True if a line break comes before the next token (`return` / `break` end there).
    fn line_break_ahead(&mut self) -> bool {
        let save = self.pos;
        self.ws();
        let nl = self.src[save..self.pos].contains(&b'\n');
        self.pos = save;
        nl
    }

    /// `const`/`let`/`var` declarators after the keyword (no terminator).
    fn declarations(&mut self, kind: &str) -> PResult<Vec<Stmt>> {
        let mut out = vec![];
        let var = kind == "var" && !self.var_scopes.is_empty();
        loop {
            let p = self.binding_pattern()?;
            self.skip_type_annotation()?;
            let e = if self.eat("=") { Some(self.expr()?) } else { None };
            let mut names = vec![];
            p.names(&mut names);
            self.declare(names, kind == "const");
            match (p, e) {
                (Pattern::Name(n), e) if var => {
                    self.var_scopes.last_mut().unwrap().push(n.clone());
                    if let Some(e) = e {
                        out.push(Stmt::Init(n, e));
                    }
                }
                (Pattern::Name(n), None) if kind == "const" => {
                    return self.err(format!("missing initializer in const declaration of `{n}`"));
                }
                (Pattern::Name(n), e) => out.push(Stmt::Let(n, e.unwrap_or(Expr::Lit(Value::Undef)))),
                (_, None) => return self.err("a destructuring declaration needs an initializer"),
                (p, Some(e)) if var => {
                    let mut names = vec![];
                    p.names(&mut names);
                    self.var_scopes.last_mut().unwrap().extend(names);
                    out.push(Stmt::LetPattern(p, e, true));
                }
                (p, Some(e)) => out.push(Stmt::LetPattern(p, e, false)),
            }
            if !self.eat(",") {
                break;
            }
        }
        Ok(out)
    }

    fn tsx_stmt(&mut self) -> PResult<Vec<Stmt>> {
        self.nested(|p| p.tsx_stmt_inner())
    }

    fn tsx_stmt_inner(&mut self) -> PResult<Vec<Stmt>> {
        self.ws();
        if self.peek() == b';' {
            self.pos += 1;
            return Ok(vec![]);
        }
        if self.peek() == b'{' {
            return Ok(vec![Stmt::Block(self.tsx_block()?)]);
        }
        // `label: statement`
        if let Some(id) = self.peek_ident() {
            let mut i = self.pos + id.len();
            while matches!(self.src.get(i), Some(b' ' | b'\t')) {
                i += 1;
            }
            if self.src.get(i) == Some(&b':')
                && self.src.get(i + 1) != Some(&b':')
                && !matches!(id.as_str(), "case" | "default")
            {
                self.pos = i + 1;
                let mut body = self.tsx_stmt()?;
                if !matches!(
                    body.first(),
                    Some(Stmt::While(..) | Stmt::DoWhile(..) | Stmt::For(..) | Stmt::ForC(..) | Stmt::Block(_))
                ) || body.len() != 1
                {
                    return self.err(format!("a label (`{id}:`) must be on a loop or a block"));
                }
                return Ok(vec![Stmt::Labeled(Rc::from(id.as_str()), Box::new(body.remove(0)))]);
            }
        }
        let stmt_start = self.pos;
        if self.starts("class") && !is_ident(self.peek_at(5)) {
            self.pos += 5;
            let def = self.class_def(true)?;
            let name = def.name.clone().unwrap();
            self.declare([name.clone()], false);
            return Ok(vec![Stmt::Let(name, Expr::Class(Rc::new(def)))]);
        }
        for kind in ["const", "let", "var"] {
            if self.eat(kind) {
                let mut out = self.declarations(kind)?;
                self.end_stmt()?;
                // `const name = (..) => ..`: the function is named for its binding, and takes the doc.
                if let [Stmt::Let(n, Expr::Lambda(l))] = out.as_mut_slice()
                    && let Some(l) = Rc::get_mut(l)
                {
                    if l.name.is_none() {
                        l.name = Some(n.clone());
                    }
                    if l.doc.is_none() {
                        l.doc = self.doc_before(stmt_start);
                    }
                    // A const function that calls itself is bound to itself, not to its variable.
                    if kind == "const" && l.captures.contains(n) {
                        l.captures.retain(|c| c != n);
                        l.self_name = Some(n.clone());
                    }
                }
                return Ok(out);
            }
        }
        // A function declaration: initialized at the top of its block (hoisted).
        let save = self.pos;
        let is_async = self.eat("async") && self.peek_ident().as_deref() == Some("function");
        if !is_async {
            self.pos = save;
        }
        if self.eat("function") {
            if self.eat("*") {
                return self.err("generator functions are not supported");
            }
            let name = self.ident()?;
            let doc = self.doc_before(save);
            self.declare([name.clone()], false);
            self.skip_type_params()?;
            let (params, props, patterns) = self.tsx_params()?;
            let props_type = self.props_type.take().map(Rc::new);
            self.skip_type_annotation()?;
            let body = self.tsx_function_body(&params, &props, patterns, is_async)?;
            let mut l = Self::non_arrow(self.lambda(params, props, LambdaBody::Block(body), is_async));
            (l.name, l.doc, l.props_type) = (Some(name.clone()), doc, props_type);
            let f = Expr::Lambda(Rc::new(l));
            match self.block_funcs.last_mut() {
                Some(funcs) => funcs.push((name, f)),
                None => return Ok(vec![Stmt::Let(name, f)]),
            }
            return Ok(vec![]);
        }
        if self.eat("if") {
            let c = self.paren_expr()?;
            let c = self.cond(c);
            let body = self.tsx_body()?;
            let els = if self.eat("else") { Some(self.tsx_body()?) } else { None };
            return Ok(vec![Stmt::If(vec![(c, body)], els)]);
        }
        if self.eat("for") {
            return self.for_stmt();
        }
        if self.eat("while") {
            let c = self.paren_expr()?;
            let c = self.cond(c);
            return Ok(vec![Stmt::While(c, self.tsx_body()?)]);
        }
        if self.eat("do") {
            let body = self.tsx_body()?;
            if !self.eat("while") {
                return self.err("expected `while` after a `do` body");
            }
            let c = self.paren_expr()?;
            let c = self.cond(c);
            self.eat(";");
            return Ok(vec![Stmt::DoWhile(body, c)]);
        }
        if self.eat("switch") {
            let e = self.paren_expr()?;
            self.expect("{")?;
            let mut arms: Vec<(Vec<Expr>, Vec<Stmt>)> = vec![];
            let mut default = None;
            loop {
                if self.eat("}") {
                    break;
                }
                if self.eat("case") {
                    let v = self.expr()?;
                    self.expect(":")?;
                    arms.push((vec![v], vec![]));
                } else if self.eat("default") {
                    self.expect(":")?;
                    default = Some(arms.len());
                    arms.push((vec![], vec![]));
                } else {
                    let st = self.tsx_stmt()?;
                    match arms.last_mut() {
                        Some(a) => a.1.extend(st),
                        None => return self.err("statement before first case"),
                    }
                }
            }
            return Ok(vec![Stmt::Switch(e, arms, default)]);
        }
        if self.eat("try") {
            let body = self.tsx_block()?;
            let (mut name, mut handler, mut finally) = (None, None, None);
            if self.eat("catch") {
                if self.eat("(") {
                    name = Some(self.ident()?);
                    self.skip_type_annotation()?;
                    self.expect(")")?;
                }
                self.decls.push(name.iter().map(|n: &Sym| (n.clone(), false)).collect());
                let h = self.tsx_block();
                self.decls.pop();
                handler = Some(h?);
            }
            if self.eat("finally") {
                finally = Some(self.tsx_block()?);
            }
            if handler.is_none() && finally.is_none() {
                return self.err("`try` needs a `catch` or a `finally`");
            }
            return Ok(vec![Stmt::Try(body, name, handler, finally)]);
        }
        if self.eat("throw") {
            if self.line_break_ahead() {
                return self.err("a line break after `throw` is not allowed");
            }
            let e = self.expr()?;
            self.end_stmt()?;
            return Ok(vec![Stmt::Throw(e)]);
        }
        if self.eat("return") {
            // `return` then a line break returns undefined (automatic semicolon insertion).
            let e = if self.line_break_ahead() || matches!(self.peek_after_ws(), b';' | b'}') || self.eof() {
                Expr::Lit(Value::Undef)
            } else {
                self.expr()?
            };
            self.end_stmt()?;
            return Ok(vec![Stmt::Return(e)]);
        }
        for kw in ["break", "continue"] {
            if self.eat(kw) {
                let label = if !self.line_break_ahead() && self.peek_ident().is_some() {
                    Some(self.ident()?)
                } else {
                    None
                };
                self.end_stmt()?;
                return Ok(vec![if kw == "break" {
                    Stmt::Break(label)
                } else {
                    Stmt::Continue(label)
                }]);
            }
        }
        if self.peek_after_ws() == b'[' {
            // `[a, b] = e;` destructuring assignment
            let save = self.pos;
            self.pos += 1;
            let mut targets = vec![];
            let ok = loop {
                let e = match self.postfix() {
                    Ok(e) => e,
                    Err(_) => break false,
                };
                match to_target(e) {
                    Ok(t) => targets.push(t),
                    Err(_) => break false,
                }
                if self.eat("]") {
                    break self.peek_after_ws() == b'=' && self.peek_at(1) != b'=' && {
                        self.pos += 1;
                        true
                    };
                }
                if !self.eat(",") {
                    break false;
                }
            };
            if ok {
                let e = self.expr()?;
                self.end_stmt()?;
                return Ok(vec![Stmt::AssignPattern(targets, e)]);
            }
            self.pos = save;
        }
        let e = self.expr()?;
        self.end_stmt()?;
        Ok(vec![expr_stmt(e)])
    }

    /// After `for`: `(const p of xs)`, `(const k in o)` or `(init; cond; update)`.
    fn for_stmt(&mut self) -> PResult<Vec<Stmt>> {
        self.decls.push(vec![]);
        let r = self.for_stmt_inner();
        self.decls.pop();
        r
    }

    fn for_stmt_inner(&mut self) -> PResult<Vec<Stmt>> {
        self.expect("(")?;
        let save = self.pos;
        let is_const = self.starts_ws("const");
        if self.eat("const") || self.eat("let") || self.eat("var") {
            let p = self.binding_pattern()?;
            let mut names = vec![];
            p.names(&mut names);
            self.declare(names, is_const);
            self.skip_type_annotation()?;
            let keys = self.eat("in");
            if keys || self.eat("of") {
                let it = self.expr()?;
                self.expect(")")?;
                let it = if keys { Expr::Keys(Box::new(it)) } else { it };
                return Ok(vec![Stmt::For(p, it, self.tsx_body()?)]);
            }
            self.pos = save;
        }
        let init = if self.peek_after_ws() == b';' {
            vec![]
        } else if let Some(kind) = ["const", "let", "var"].into_iter().find(|k| self.eat(k)) {
            self.declarations(kind)?
        } else {
            vec![expr_stmt(self.expr()?)]
        };
        self.expect(";")?;
        let cond = if self.peek_after_ws() == b';' {
            Expr::Lit(Value::Bool(true))
        } else {
            self.expr()?
        };
        let cond = self.cond(cond);
        self.expect(";")?;
        let mut update = vec![];
        if self.peek_after_ws() != b')' {
            loop {
                update.push(expr_stmt(self.expr()?));
                if !self.eat(",") {
                    break;
                }
            }
        }
        self.expect(")")?;
        Ok(vec![Stmt::ForC(init, cond, update, self.tsx_body()?)])
    }

    // ---------------- JSX ----------------

    /// At `<`: a JSX element or fragment, as an expression.
    pub(crate) fn jsx(&mut self) -> PResult<Expr> {
        let node = if self.starts("<>") {
            self.pos += 2;
            let ch = self.jsx_children(None)?;
            return Ok(Expr::Markup(Rc::new(ch)));
        } else {
            self.jsx_element()?
        };
        Ok(Expr::Markup(Rc::new(vec![node])))
    }

    fn jsx_element(&mut self) -> PResult<Node> {
        self.nested(|p| p.jsx_element_inner())
    }

    fn jsx_element_inner(&mut self) -> PResult<Node> {
        self.pos += 1; // '<'
        let st = self.pos;
        while is_ident(self.peek()) || self.peek() == b'.' {
            self.pos += 1;
        }
        if st == self.pos {
            return self.err("expected an element name after `<`");
        }
        let tag = std::str::from_utf8(&self.src[st..self.pos]).unwrap().to_string();
        let (attrs, selfclose) = self.attrs()?;
        let children = if selfclose {
            vec![]
        } else {
            self.jsx_children(Some(&tag))?
        };
        if let Some(alias) = tag.strip_prefix("tok.") {
            // <tok.im_start/>: a special token as an element, resolved at load time
            let hit = self
                .specials
                .iter()
                .find(|(s, _)| crate::emit::token_alias(s).as_deref() == Some(alias));
            return match hit {
                Some((s, id)) => Ok(Node::Special {
                    text: Rc::from(s.as_str()),
                    id: std::cell::Cell::new(Some(*id)),
                }),
                None => self.err(format!(
                    "<{}/>: no special token named `{}` in this tokenizer",
                    tag, alias
                )),
            };
        }
        // <Mount>: a child instance (an element of the harness, not a component).
        let tag: Sym = Rc::from(if tag == "Mount" { "mount" } else { tag.as_str() });
        if tag.as_bytes()[0].is_ascii_uppercase() || tag.contains('.') {
            return Ok(Node::Call {
                name: tag,
                attrs,
                children,
            });
        }
        match &*tag {
            "sp" => {
                let a = attrs
                    .into_iter()
                    .find(|a| &*a.name == "t")
                    .ok_or_else(|| self.err::<()>("<sp> needs t=").unwrap_err())?;
                Ok(match a.value {
                    AttrVal::Str(s) => {
                        let id = self.special_ids.and_then(|m| m.get(&*s).copied());
                        if self.special_ids.is_some() && id.is_none() {
                            return self.err(format!("{:?} is not an added token of this tokenizer", s));
                        }
                        Node::Special {
                            text: s,
                            id: std::cell::Cell::new(id),
                        }
                    }
                    AttrVal::Expr(e) => Node::DynSpecial(e),
                    AttrVal::Bare | AttrVal::Spread(_) => return self.err("<sp t> needs a value"),
                })
            }
            "tok" => {
                let a = attrs
                    .into_iter()
                    .find(|a| &*a.name == "id")
                    .ok_or_else(|| self.err::<()>("<tok> needs id=").unwrap_err())?;
                match a.value {
                    AttrVal::Expr(e) => Ok(Node::TokId(e)),
                    _ => self.err("<tok id={..}/>"),
                }
            }
            t if BUILTIN_TAGS.contains(&t) => Ok(Node::Builtin { tag, attrs, children }),
            t => self.err(format!(
                "unknown element <{}>: write prompt text in p`...` or use a Component",
                t
            )),
        }
    }

    /// Children until `</tag>` (or `</>` for fragments). Text follows React's JSX whitespace rule.
    fn jsx_children(&mut self, close: Option<&str>) -> PResult<Vec<Node>> {
        let mut nodes = vec![];
        let mut text: Vec<u8> = vec![];
        loop {
            if self.eof() {
                return self.err("unterminated JSX");
            }
            let c = self.peek();
            if c == b'<' {
                let n = self.peek_at(1);
                if n == b'/' {
                    self.flush_jsx_text(&mut text, &mut nodes);
                    let want = match close {
                        Some(t) => format!("</{}>", t),
                        None => "</>".to_string(),
                    };
                    if !self.starts(&want) {
                        return self.err(format!("expected {}", want));
                    }
                    self.pos += want.len();
                    return Ok(nodes);
                }
                if is_ident_start(n) || n == b'>' {
                    self.flush_jsx_text(&mut text, &mut nodes);
                    if n == b'>' {
                        self.pos += 2;
                        nodes.extend(self.nested(|p| p.jsx_children(None))?);
                    } else {
                        nodes.push(self.jsx_element()?);
                    }
                    continue;
                }
                text.push(c); // `<|im_start|>` and other non-tag `<`
                self.pos += 1;
                continue;
            }
            if c == b'{' {
                self.flush_jsx_text(&mut text, &mut nodes);
                self.pos += 1;
                self.ws(); // also skips {/* comments */}
                if self.peek() == b'}' {
                    self.pos += 1;
                    continue;
                }
                let e = self.expr()?;
                self.expect("}")?;
                nodes.push(Node::Child(e));
                continue;
            }
            text.push(c);
            self.pos += 1;
        }
    }

    fn flush_jsx_text(&mut self, text: &mut Vec<u8>, nodes: &mut Vec<Node>) {
        if text.is_empty() {
            return;
        }
        let raw = String::from_utf8(std::mem::take(text)).unwrap();
        // React: trim each line (except the outer edges of single-line text), drop empty lines,
        // join the rest with one space.
        let t = if raw.contains('\n') {
            let lines: Vec<&str> = raw.split('\n').collect();
            let last = lines.len() - 1;
            let kept: Vec<String> = lines
                .iter()
                .enumerate()
                .map(|(i, l)| {
                    let mut s: &str = l;
                    if i > 0 {
                        s = s.trim_start();
                    }
                    if i < last {
                        s = s.trim_end();
                    }
                    s.to_string()
                })
                .filter(|s| !s.trim().is_empty())
                .collect();
            kept.join(" ")
        } else {
            raw
        };
        if !t.is_empty() {
            if self.strict_tokens
                && let Err(e) = self.check_no_specials(&t)
            {
                self.strict_error = Some(e);
            }
            split_specials(&t, self.specials, nodes);
        }
    }

    /// At '`': template literal. Tagged (``p`...` ``) parts are split on special tokens.
    pub(crate) fn template(&mut self, tagged: bool) -> PResult<Expr> {
        self.pos += 1;
        let mut parts = vec![];
        let mut cur: Vec<u8> = vec![];
        let strict = self.strict_tokens;
        let mut strict_found: Option<String> = None;
        let strict_err = &mut strict_found;
        let mut lit = |cur: &mut Vec<u8>, parts: &mut Vec<TplPart>, specials: &[(String, u32)]| {
            if cur.is_empty() {
                return;
            }
            let s = String::from_utf8(std::mem::take(cur)).unwrap();
            if strict && let Some((sp, _)) = specials.iter().find(|(sp, _)| s.contains(sp.as_str())) {
                *strict_err = Some(sp.clone());
            }
            let mut nodes = vec![];
            if tagged {
                split_specials(&s, specials, &mut nodes);
            }
            parts.push(TplPart::Lit(Rc::from(s), nodes));
        };
        loop {
            if self.eof() {
                return self.err("unterminated template literal");
            }
            let c = self.peek();
            if c == b'`' {
                self.pos += 1;
                lit(&mut cur, &mut parts, self.specials);
                if let Some(sp) = strict_found {
                    return self.err(format!(
                        "\"use strict_tokens\": special token {:?} written as text; use ${{tok.{}}}",
                        sp,
                        crate::emit::token_alias(&sp).unwrap_or_default()
                    ));
                }
                return Ok(Expr::Template { tagged, parts });
            }
            if c == b'\\' {
                self.pos += 1;
                let e = self.peek();
                self.pos += 1;
                self.push_escape(e, &mut cur)?;
                continue;
            }
            if c == b'$' && self.peek_at(1) == b'{' {
                self.pos += 2;
                lit(&mut cur, &mut parts, self.specials);
                let e = self.expr()?;
                self.expect("}")?;
                // A plain template literal stringifies its parts as JavaScript does.
                parts.push(TplPart::Expr(if !tagged { Expr::ToString(Box::new(e)) } else { e }));
                continue;
            }
            cur.push(c);
            self.pos += 1;
        }
    }
}

impl<'a> Parser<'a> {
    pub(crate) fn check_no_specials(&self, t: &str) -> Result<(), String> {
        if let Some((sp, _)) = self.specials.iter().find(|(sp, _)| t.contains(sp.as_str())) {
            return Err(format!(
                "\"use strict_tokens\": special token {:?} written as text; use {{tok.{}}} or <tok.{}/>",
                sp,
                crate::emit::token_alias(sp).unwrap_or_default(),
                crate::emit::token_alias(sp).unwrap_or_default()
            ));
        }
        Ok(())
    }
}

/// The names a function's parameters bind (none of them `const`).
fn param_names(params: &[Param], props: &Option<Vec<Param>>, patterns: &[(Sym, Pattern)]) -> Vec<(Sym, bool)> {
    let mut names: Vec<Sym> = params.iter().map(|p| p.name.clone()).collect();
    names.extend(props.iter().flatten().map(|p| p.name.clone()));
    for (_, p) in patterns {
        p.names(&mut names);
    }
    names.into_iter().map(|n| (n, false)).collect()
}

/// An expression evaluated for its effect. `x = e` keeps the statement form (it grows a list
/// spread into itself in place).
fn expr_stmt(e: Expr) -> Stmt {
    match e {
        Expr::Assign(t, AssignOp::Set, v) => Stmt::Assign(*t, *v),
        e => Stmt::Expr(e),
    }
}

/// Declare every lexical name on block entry (including before its initializer runs),
/// initialize the function declarations first, and turn each declaration into an initialization.
fn hoist(body: Vec<Stmt>, funcs: Vec<(Sym, Expr)>) -> Vec<Stmt> {
    let mut names: Vec<Sym> = funcs.iter().map(|(n, _)| n.clone()).collect();
    let body: Vec<Stmt> = body
        .into_iter()
        .map(|s| match s {
            Stmt::Let(n, e) => {
                names.push(n.clone());
                Stmt::Init(n, e)
            }
            Stmt::LetPattern(p, e, false) => {
                p.names(&mut names);
                Stmt::LetPattern(p, e, true)
            }
            s => s,
        })
        .collect();
    if names.is_empty() {
        return body;
    }
    let mut out = Vec::with_capacity(body.len() + funcs.len() + 1);
    out.push(Stmt::Declare(names));
    out.extend(funcs.into_iter().map(|(n, f)| Stmt::Init(n, f)));
    out.extend(body);
    out
}
