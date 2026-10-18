use crate::hir;

// separating language design choices again explicitly in a separate file
pub(super) fn const_contexts(expression: &hir::Expr) -> &[hir::Expr] {
    match &expression.kind {
        hir::ExprKind::RawSyscall { arguments } => arguments.get(..2).unwrap_or_default(),
        _ => &[],
    }
}
