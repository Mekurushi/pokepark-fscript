mod declarations;
mod error;
mod lower;

use crate::hir;
use fsc_diagnostics::Diagnostics;
use fsc_parse::ast;

#[allow(dead_code)]
pub fn resolve(script: &ast::Script) -> Result<hir::Script, Diagnostics> {
    let mut diagnostics = Diagnostics::default();
    let declarations = declarations::collect(script, &mut diagnostics);
    let hir = lower::lower(script, &declarations, &mut diagnostics);

    if diagnostics.has_errors() {
        Err(diagnostics)
    } else {
        Ok(hir)
    }
}
