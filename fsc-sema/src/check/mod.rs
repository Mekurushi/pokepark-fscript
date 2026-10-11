mod model;
pub(crate) mod resolution;
pub(crate) mod symbols;
pub(crate) mod type_check;

use crate::error::SemaResult;
use fsc_parse::ast;
use resolution::{ResolvedFunction, ScopeStack, declare_items};
use symbols::SymbolTable;

pub use model::{CheckedFunction, CheckedScript};

pub fn check(script: ast::Script) -> SemaResult<CheckedScript> {
    let mut scope = ScopeStack::new();
    let mut symbols = SymbolTable::new();
    declare_items(&script, &mut scope, &mut symbols)?;

    let mut resolved_functions: Vec<(ast::FuncDef, ResolvedFunction)> = Vec::new();
    for item in &script.items {
        if let ast::Item::FuncDef(function) = item {
            let resolved = resolution::resolve_fn(function, &mut symbols, &mut scope)?;
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
