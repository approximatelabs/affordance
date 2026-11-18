//! Runtime source loading into the same lexical environments and executor as startup files.
use super::*;
use crate::{
    modules::{Cell, Instance, Scope},
    parser::{Module, Parser},
};

pub(crate) fn install(program: &mut Program) -> RResult<()> {
    let name: Sym = Rc::from("loadModule");
    let module = Module {
        consts: vec![(
            name.clone(),
            Expr::Lit(Value::Native(Rc::<str>::from("\0compiler.loadModule").into())),
        )],
        exports: HashMap::from([(name.clone(), name)]),
        ..Default::default()
    };
    crate::modules::link(program, "affordance:compiler".into(), module, Scope::default())
}

impl<'p> Interp<'p> {
    /// Module initialization gets its own lexical frame, not the caller's locals or effects.
    /// It shares fuel/depth accounting and ordinary evaluation; the environment survives it.
    pub(super) fn initialize_value(&mut self, instance: Rc<Instance>, name: Sym, expression: &Expr) -> RResult<()> {
        let mark = self.document_mark();
        let old_scope = std::mem::replace(&mut self.scope, instance.clone());
        let old_acting = std::mem::replace(&mut self.acting, false);
        // Initializer ASTs are transient; their results are already retained as constants.
        let old_memo = self.memo.take();
        let base = self.scopes.len();
        self.frames.push(base);
        self.initializing += 1;
        let result = match expression {
            Expr::Class(def) => self.eval_class(def, Some(def.clone())),
            _ => self.eval(expression),
        };
        self.scopes.truncate(base);
        self.frames.pop();
        self.initializing -= 1;
        self.scope = old_scope;
        self.acting = old_acting;
        self.memo = old_memo;
        self.abort_document(mark);
        match result {
            Ok(value) => instance.initialize(name, value),
            Err(error) if error.starts_with("this tokenizer has no special token") => {
                instance.bindings.borrow_mut().insert(name, Cell::Unavailable(error));
            }
            Err(error) => return Err(error),
        }
        Ok(())
    }

    pub(super) fn load_module(&mut self, args: Vec<Value>) -> RResult<Value> {
        let source = args
            .first()
            .and_then(Value::as_str)
            .ok_or("loadModule(source, options?): source must be a string")?;
        self.steps = self.steps.saturating_add(source.len() as u64);
        self.spend()?;
        if self.depth >= 400 {
            return err("RangeError: module loading exceeded the maximum call depth");
        }
        self.depth += 1;
        let result = self.load_module_inner(source, args.get(1).unwrap_or(&Value::Undef));
        self.depth -= 1;
        result
    }
    fn load_module_inner(&mut self, source: &str, options: &Value) -> RResult<Value> {
        let imports = match options {
            Value::Undef | Value::None => Value::Undef,
            Value::Map(map) => {
                if let Some(key) = map.read().keys().find(|key| &***key != "imports") {
                    return err(format!("loadModule: unknown option `{key}`"));
                }
                map.read().get("imports").cloned().unwrap_or(Value::Undef)
            }
            _ => return err("loadModule: options must be an object"),
        };
        let imports = match imports {
            Value::Undef | Value::None => Map::default(),
            Value::Map(map) => map.read().clone(),
            _ => return err("loadModule: imports must be an export-namespace map"),
        };
        let encoder = self.encoder.ok_or("loadModule requires a bound tokenizer")?;
        let id: Sym = Rc::from(format!("source:{}", crate::sha256_hex(source.as_bytes())));
        let mut parser = Parser::new(source, &id);
        parser.specials = &encoder.special_strings;
        parser.special_ids = Some(&encoder.added);
        let module = parser.module_tsx().map_err(|e| format!("SyntaxError: {e}"))?;
        for directive in &module.pragmas {
            match directive.as_str() {
                "strict_tokens" => {}
                "safe_text" if self.prog.pragmas.iter().any(|p| p == "safe_text") => {}
                "safe_text" => {
                    return err("loadModule: safe_text must be selected by the parent app");
                }
                _ => return err(format!("loadModule: unsupported directive `{directive}`")),
            }
        }
        let mut linked = Scope::default();
        let mut values = HashMap::new();
        for import in &module.imports {
            if import.path.ends_with(".tss") {
                return err(
                    "loadModule: style-file imports are unsupported; use inline styles or the parent's stylesheet",
                );
            }
            let builtin = if import.path.starts_with("affordance:") {
                Some(
                    self.prog
                        .modules
                        .get(import.path.as_str())
                        .ok_or_else(|| format!("loadModule: unknown built-in module `{}`", import.path))?,
                )
            } else {
                None
            };
            let namespace = if builtin.is_none() {
                match imports.get(import.path.as_str()) {
                    Some(Value::Map(namespace)) => Some(namespace.read()),
                    _ => {
                        return err(format!("loadModule: missing export namespace for `{}`", import.path));
                    }
                }
            } else {
                None
            };
            for (public, local) in &import.bindings {
                let value = if let Some(builtin) = builtin {
                    let target = builtin
                        .exports
                        .get(public)
                        .ok_or_else(|| format!("loadModule: `{}` has no export `{public}`", import.path))?;
                    self.binding_value(target)
                } else {
                    namespace
                        .as_ref()
                        .unwrap()
                        .get(public)
                        .cloned()
                        .ok_or_else(|| format!("loadModule: `{}` has no export `{public}`", import.path))?
                };
                if linked
                    .bindings
                    .insert(local.clone(), crate::modules::key(&id, local))
                    .is_some()
                {
                    return err(format!("loadModule: duplicate import `{local}`"));
                }
                values.insert(local.clone(), value);
            }
        }
        let linked = crate::modules::declarations(&id, &module, linked)?;
        let instance = Instance::new(linked.types);
        for (name, definition) in module.funcs {
            instance.bindings.borrow_mut().insert(name, Cell::Function(definition));
        }
        for (name, _) in &module.consts {
            instance.bindings.borrow_mut().insert(name.clone(), Cell::Uninitialized);
        }
        for (name, value) in values {
            instance.initialize(name, value);
        }
        for (name, expression) in &module.consts {
            self.initialize_value(instance.clone(), name.clone(), expression)?;
        }
        let mut exports = Map::default();
        let mut names = module.exports.into_iter().collect::<Vec<_>>();
        names.sort_by(|a, b| a.0.cmp(&b.0));
        for (public, local) in names {
            if let Some(error) = instance.unavailable(&local) {
                return err(error);
            }
            let value = instance.get(&local);
            if is_uninitialized(&value) {
                return tdz_error(&local);
            }
            exports.insert(public, value);
        }
        let namespace = Value::map(exports);
        crate::value::freeze(&namespace);
        Ok(namespace)
    }
}
