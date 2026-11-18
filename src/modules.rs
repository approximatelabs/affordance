//! File-local lexical bindings. Runtime values retain source names; only loader keys are private.
use crate::{ast::Sym, emit::Encoder, interp::Program, parser::Module};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    rc::Rc,
};

#[derive(Default)]
pub struct Scope {
    pub bindings: HashMap<Sym, Binding>,
    pub exports: HashMap<Sym, Binding>,
    pub types: Rc<HashMap<String, crate::tsx::TsType>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Binding {
    pub module: Sym,
    pub name: Sym,
}

pub(crate) fn key(module: &Sym, name: &Sym) -> Binding {
    Binding {
        module: module.clone(),
        name: name.clone(),
    }
}

/// Install one parsed module after its dependencies have been linked.
pub(crate) fn link(program: &mut Program, id: Sym, module: Module, mut scope: Scope) -> Result<(), String> {
    scope = declarations(&id, &module, scope)?;
    program
        .funcs
        .extend(module.funcs.into_iter().map(|(name, f)| (key(&id, &name), f)));
    program
        .consts
        .extend(module.consts.into_iter().map(|(name, e)| (key(&id, &name), e)));
    program.pragmas.extend(module.pragmas);
    program.modules.insert(id, scope);
    Ok(())
}

/// One declaration/export validation path for startup files and source modules.
pub(crate) fn declarations(id: &Sym, module: &Module, mut scope: Scope) -> Result<Scope, String> {
    for name in module.funcs.keys().chain(module.consts.iter().map(|(name, _)| name)) {
        if scope.bindings.insert(name.clone(), key(id, name)).is_some() {
            return Err(format!("{id}: duplicate module binding `{name}`"));
        }
    }
    for (public, local) in &module.exports {
        let target = scope
            .bindings
            .get(local)
            .ok_or_else(|| format!("{id}: export `{public}` refers to undefined binding `{local}`"))?;
        scope.exports.insert(public.clone(), target.clone());
    }
    scope.types = Rc::new(module.types.clone());
    Ok(scope)
}

pub(crate) fn load(
    program: &mut Program,
    path: &Path,
    enc: &Encoder,
    seen: &mut HashSet<PathBuf>,
    active: &mut Vec<PathBuf>,
) -> Result<Sym, String> {
    let canon = path.canonicalize().map_err(|e| format!("{}: {e}", path.display()))?;
    let id: Sym = Rc::from(canon.to_string_lossy().as_ref());
    if !program.roots.is_empty() && !program.roots.iter().any(|r| canon.starts_with(r)) {
        let from = active
            .last()
            .map_or_else(|| "the app".to_string(), |p| p.display().to_string());
        // A library the file belongs to (a views/ checkout) is one the deployment can name.
        let library = canon.ancestors().skip(1).find(|d| d.join("index.json").is_file());
        let fix = library.map_or_else(String::new, |l| {
            format!(
                "; to import from it, set \"library\": \"{}\" in the deployment",
                l.display()
            )
        });
        return Err(format!(
            "{from}: {} is outside the app: modules come from the app's directory or the library ({}){fix}",
            path.display(),
            program
                .roots
                .iter()
                .map(|r| r.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if active.contains(&canon) {
        let chain = active
            .iter()
            .chain(std::iter::once(&canon))
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(" -> ");
        return Err(format!("cyclic module imports are not supported: {chain}"));
    }
    if !seen.insert(canon.clone()) {
        return Ok(id);
    }
    let source = std::fs::read_to_string(&canon).map_err(|e| format!("{id}: {e}"))?;
    install(program, id, &canon, &source, enc, seen, active)
}

/// The id of `affordance:model`, the module made from the deployment's model templates
/// (`crate::library::model_module`).
const MODEL: &str = "affordance:model";

/// Parse and link one module's source. `canon` is its file (for a made module, the file its
/// relative imports are read from).
fn install(
    program: &mut Program,
    id: Sym,
    canon: &Path,
    source: &str,
    enc: &Encoder,
    seen: &mut HashSet<PathBuf>,
    active: &mut Vec<PathBuf>,
) -> Result<Sym, String> {
    let canon = canon.to_path_buf();
    if canon.extension().is_some_and(|e| e == "tss") {
        for mut rule in crate::style::parse(source, &id)? {
            rule.order = program.styles.len();
            program.styles.push(rule);
        }
        return Ok(id);
    }
    if canon.extension().is_none_or(|e| e != "tsx") {
        return Err(format!("{id}: views are .tsx files (styles .tss)"));
    }
    let mut parser = crate::parser::Parser::new(source, &id);
    parser.specials = &enc.special_strings;
    parser.special_ids = Some(&enc.added);
    let module = parser.module_tsx()?;
    if active.is_empty() {
        program.root = id.clone();
        if !module.funcs.contains_key(&program.entry)
            && let Some(default) = module.exports.get("default")
        {
            program.entry = default.clone();
        }
    }
    active.push(canon.clone());
    let mut scope = Scope::default();
    for import in &module.imports {
        let dependency = if import.path == MODEL {
            if !program.modules.contains_key(MODEL) {
                // The library: the one the importing file belongs to (`<library>/lib/model.tsx`
                // asks for this module), else the one the root app lives in, or the bundled one.
                let is_library = |d: &Path| d.join("template_apps/index.json").is_file();
                let library = match canon.ancestors().skip(1).find(|d| is_library(d)) {
                    Some(library) => library.to_path_buf(),
                    None => crate::library::for_app(Path::new(program.root.as_ref()))?,
                };
                let library = library
                    .canonicalize()
                    .map_err(|e| format!("library {}: {e}", library.display()))?;
                let made = crate::library::model_module(&library, &program.templates)?;
                // Its imports are absolute paths into the library.
                install(
                    program,
                    Rc::from(MODEL),
                    &library.join("lib/model.tsx"),
                    &made,
                    enc,
                    seen,
                    active,
                )?;
            }
            Rc::from(MODEL)
        } else if import.path.starts_with("affordance:") {
            if !program.modules.contains_key(import.path.as_str()) {
                return Err(format!("{id}: unknown built-in module `{}`", import.path));
            }
            Rc::from(import.path.as_str())
        } else if let Some(rest) = import.path.strip_prefix("@affordance/") {
            // The library, as an import map names it: the deployment's `library` or the bundled one.
            let library = program.roots.get(1).cloned().ok_or_else(|| {
                format!(
                    "{id}: {} imports from the library, and none is configured (affordance run provides it)",
                    import.path
                )
            })?;
            load(program, &library.join(rest), enc, seen, active)?
        } else {
            load(program, &canon.parent().unwrap().join(&import.path), enc, seen, active)?
        };
        for (public, local) in &import.bindings {
            let target = program
                .modules
                .get(&dependency)
                .and_then(|s| s.exports.get(public))
                .ok_or_else(|| format!("{id}: module `{}` has no export `{public}`", import.path))?;
            if scope.bindings.insert(local.clone(), target.clone()).is_some() {
                return Err(format!("{id}: duplicate import binding `{local}`"));
            }
        }
    }
    active.pop();
    link(program, id.clone(), module, scope)?;
    Ok(id)
}

/// A lexical module instance. Runtime functions own this environment, not a lookup into
/// whichever startup Program happens to be executing them.
#[derive(Debug)]
pub struct Instance {
    pub(crate) types: Rc<HashMap<String, crate::tsx::TsType>>,
    pub(crate) bindings: std::cell::RefCell<HashMap<Sym, Cell>>,
    pub(crate) frozen: std::cell::Cell<bool>,
}
#[derive(Debug)]
pub(crate) enum Cell {
    Uninitialized,
    Ready(crate::value::Value),
    Function(Rc<crate::ast::Func>),
    Alias(Rc<Instance>, Sym),
    Unavailable(String),
}
impl Instance {
    pub(crate) fn new(types: Rc<HashMap<String, crate::tsx::TsType>>) -> Rc<Self> {
        Rc::new(Self {
            types,
            bindings: Default::default(),
            frozen: std::cell::Cell::new(false),
        })
    }
    pub(crate) fn contains(&self, name: &str) -> bool {
        self.bindings.borrow().contains_key(name)
    }
    pub(crate) fn get(self: &Rc<Self>, name: &str) -> crate::value::Value {
        use crate::value::{Function, Value};
        let (module, name) = {
            let bindings = self.bindings.borrow();
            match bindings.get(name) {
                Some(Cell::Ready(v)) => return v.clone(),
                Some(Cell::Function(definition)) => {
                    return Value::Func(Rc::new(Function {
                        definition: definition.clone(),
                        module: self.clone(),
                    }));
                }
                Some(Cell::Alias(module, name)) => (module.clone(), name.clone()),
                Some(Cell::Uninitialized) => return crate::interp::uninitialized(),
                _ => return Value::Undef,
            }
        };
        module.get(&name)
    }
    pub(crate) fn unavailable(&self, name: &str) -> Option<String> {
        match self.bindings.borrow().get(name) {
            Some(Cell::Unavailable(error)) => Some(error.clone()),
            Some(Cell::Alias(module, name)) => module.unavailable(name),
            _ => None,
        }
    }
    pub(crate) fn initialize(self: &Rc<Self>, name: Sym, value: crate::value::Value) {
        crate::value::freeze(&value);
        crate::heap::note_module_write(self, &value);
        self.bindings.borrow_mut().insert(name, Cell::Ready(value));
    }
}
