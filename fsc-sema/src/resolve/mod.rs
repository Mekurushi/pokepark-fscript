mod declarations;
mod error;
mod lower;

use crate::hir;
use fsc_diagnostics::Diagnostics;
use fsc_parse::ast;

pub(crate) struct ResolveOutput {
    pub(crate) hir: hir::Script,
    pub(crate) had_errors: bool,
}

pub(crate) fn resolve(script: &ast::Script, diagnostics: &mut Diagnostics) -> ResolveOutput {
    let errors_before = diagnostics.error_count();
    let declarations = declarations::collect(script, diagnostics);
    let hir = lower::lower_script(script, &declarations, diagnostics);
    ResolveOutput {
        hir,
        had_errors: diagnostics.error_count() > errors_before,
    }
}
