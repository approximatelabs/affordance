//! Embedded Web API fallbacks. Ordinary TSX over the same journaled executor boundary.
use crate::{interp::Program, parser::Parser};
use std::cell::OnceCell;

thread_local! {
    static STANDARD: OnceCell<crate::parser::Module> = const { OnceCell::new() };
}

pub(crate) fn install(program: &mut Program) -> Result<(), String> {
    // The embedded module owns its private bindings. Share its immutable ASTs while each
    // render evaluates fresh classes; public globals remain shadowable in app modules.
    let module = STANDARD.with(|cell| {
        if cell.get().is_none() {
            let module = Parser::new(include_str!("web.tsx"), "affordance:web").module_tsx()?;
            let _ = cell.set(module);
        }
        let module = cell.get().unwrap();
        // Only the embedded globals are copied here: class definitions and their public
        // aliases. Keep the shared class ASTs, rather than cloning the language's AST.
        let constants = module
            .consts
            .iter()
            .map(|(name, value)| {
                let value = match value {
                    crate::ast::Expr::Class(class) => crate::ast::Expr::Class(class.clone()),
                    crate::ast::Expr::Var(alias) => crate::ast::Expr::Var(alias.clone()),
                    _ => return Err("Web platform globals must be classes or aliases".to_string()),
                };
                Ok((name.clone(), value))
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok::<_, String>(crate::parser::Module {
            funcs: module.funcs.clone(),
            consts: constants,
            exports: module.exports.clone(),
            ..Default::default()
        })
    })?;
    crate::modules::link(program, "affordance:web".into(), module, Default::default())?;
    for name in ["fetch", "Headers", "Response", "AbortController", "AbortSignal"] {
        program
            .globals
            .insert(name.into(), crate::modules::key(&"affordance:web".into(), &name.into()));
    }
    Ok(())
}
