use super::error::ControlFlowError;
use crate::hir;
use fsc_diagnostics::{Diagnostic, Diagnostics};

pub(crate) fn check(script: &hir::Script, diagnostics: &mut Diagnostics) -> bool {
    let errors_before = diagnostics.error_count();
    let mut checker = ControlFlowChecker { diagnostics };
    checker.check_script(script);
    diagnostics.error_count() > errors_before
}

struct ControlFlowChecker<'diagnostics> {
    diagnostics: &'diagnostics mut Diagnostics,
}

impl ControlFlowChecker<'_> {
    fn check_script(&mut self, script: &hir::Script) {
        for (_, item) in &script.items {
            let hir::ItemKind::Function(function) = &item.kind else {
                continue;
            };
            let hir::FunctionKind::Defined { body, .. } = &function.kind else {
                continue;
            };

            self.check_block(&body.block, 0);
        }
    }

    fn check_block(&mut self, block: &hir::Block, loop_depth: usize) {
        for statement in &block.statements {
            match &statement.kind {
                hir::StmtKind::Break if loop_depth == 0 => {
                    self.diagnostics.emit_error(Diagnostic::from(
                        ControlFlowError::BreakOutsideLoop {
                            span: statement.span,
                        },
                    ));
                }
                hir::StmtKind::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    self.check_block(then_block, loop_depth);
                    if let Some(else_block) = else_block {
                        self.check_block(else_block, loop_depth);
                    }
                }
                hir::StmtKind::While { body, .. } => {
                    self.check_block(body, loop_depth + 1);
                }
                hir::StmtKind::Return(_)
                | hir::StmtKind::Break
                | hir::StmtKind::VariableDeclaration { .. }
                | hir::StmtKind::ConstantDeclaration { .. }
                | hir::StmtKind::Assign { .. }
                | hir::StmtKind::Expression(_)
                | hir::StmtKind::Pause(_) => {}
            }
        }
    }
}
