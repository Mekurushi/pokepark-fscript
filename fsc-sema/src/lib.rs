use crate::error::SemaResult;
use crate::resolve::{ResolvedFunction, ScopeStack, declare_items};
use crate::symbol::SymbolTable;
use fsc_parse::ast;

mod bind;
mod checked;
mod error;
pub mod hir;
mod resolve;
mod symbol;
mod type_check;
pub mod types;

pub use bind::{
    BindError, BindErrors, BoundScript, ConfigRequirement, ConfigType, ConfigValue, ConfigValues,
    bind_configs,
};
pub use checked::{CheckedFunction, CheckedScript};

pub fn check(script: ast::Script) -> SemaResult<CheckedScript> {
    let mut scope = ScopeStack::new();
    let mut symbols = SymbolTable::new();
    declare_items(&script, &mut scope, &mut symbols)?;

    let mut resolved_functions: Vec<(ast::FuncDef, ResolvedFunction)> = Vec::new();
    for item in &script.items {
        if let ast::Item::FuncDef(function) = item {
            let resolved = resolve::resolve_fn(function, &mut symbols, &mut scope)?;
            resolved_functions.push((function.clone(), resolved));
        }
    }
    let functions = type_check::check(resolved_functions, &symbols)?;

    Ok(CheckedScript {
        script,
        symbols,
        functions,
    })
}

pub fn lower(bound: BoundScript) -> SemaResult<hir::Script> {
    hir::lower_script(bound)
}
