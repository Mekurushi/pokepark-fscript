use super::error::SelectorOutOfRange;
use super::{RawSyscallResults, ResolvedSelectors, SelectorKind};
use crate::const_eval::{ConstEvalResults, ConstValue};
use crate::hir;
use fsc_diagnostics::{Diagnostic, Diagnostics, Span};

pub(crate) fn check(
    script: &hir::Script,
    const_eval: &ConstEvalResults,
    diagnostics: &mut Diagnostics,
) -> Option<RawSyscallResults> {
    let mut checker = Checker {
        script,
        const_eval,
        diagnostics,
        results: RawSyscallResults::default(),
        failed: false,
    };
    checker.check_script();
    (!checker.failed).then_some(checker.results)
}

struct Checker<'a> {
    script: &'a hir::Script,
    const_eval: &'a ConstEvalResults,
    diagnostics: &'a mut Diagnostics,
    results: RawSyscallResults,
    failed: bool,
}

impl Checker<'_> {
    fn check_script(&mut self) {
        let script = self.script;
        for (_, item) in &script.items {
            let hir::ItemKind::Function(hir::Function {
                kind: hir::FunctionKind::Defined { body, .. },
                ..
            }) = &item.kind
            else {
                continue;
            };
            self.check_block(&body.block);
        }
    }

    fn check_block(&mut self, block: &hir::Block) {
        for statement in &block.statements {
            match &statement.kind {
                hir::StmtKind::Return(value) => {
                    if let Some(value) = value {
                        self.check_expression(value);
                    }
                }
                hir::StmtKind::VariableDeclaration { initializer, .. } => {
                    if let Some(initializer) = initializer {
                        self.check_expression(initializer);
                    }
                }
                hir::StmtKind::ConstantDeclaration { initializer, .. }
                | hir::StmtKind::Expression(initializer)
                | hir::StmtKind::Pause(initializer) => self.check_expression(initializer),
                hir::StmtKind::Assign { target, value } => {
                    self.check_expression(target);
                    self.check_expression(value);
                }
                hir::StmtKind::If {
                    condition,
                    then_block,
                    else_block,
                } => {
                    self.check_expression(condition);
                    self.check_block(then_block);
                    if let Some(else_block) = else_block {
                        self.check_block(else_block);
                    }
                }
                hir::StmtKind::While {
                    condition,
                    body: loop_body,
                } => {
                    self.check_expression(condition);
                    self.check_block(loop_body);
                }
                hir::StmtKind::Break => {}
            }
        }
    }

    fn check_expression(&mut self, expression: &hir::Expr) {
        match &expression.kind {
            hir::ExprKind::Member { base, .. } | hir::ExprKind::Unary { expr: base, .. } => {
                self.check_expression(base);
            }
            hir::ExprKind::Array { elements } => {
                for element in elements {
                    self.check_expression(element);
                }
            }
            hir::ExprKind::Aggregate { fields, .. } => {
                for field in fields {
                    self.check_expression(&field.value);
                }
            }
            hir::ExprKind::Index { base, index } => {
                self.check_expression(base);
                self.check_expression(index);
            }
            hir::ExprKind::Binary { lhs, rhs, .. } => {
                self.check_expression(lhs);
                self.check_expression(rhs);
            }
            hir::ExprKind::Call { callee, arguments } => {
                self.check_expression(callee);
                for argument in arguments {
                    self.check_expression(argument);
                }
            }
            hir::ExprKind::RawSyscall { arguments } => {
                if let [page, function, remaining @ ..] = arguments.as_slice() {
                    //TODO: argument count check
                    self.check_selectors(expression.id, page, function);
                    self.check_expression(page);
                    self.check_expression(function);
                    for argument in remaining {
                        self.check_expression(argument);
                    }
                } else {
                    for argument in arguments {
                        self.check_expression(argument);
                    }
                }
            }
            hir::ExprKind::Error(_)
            | hir::ExprKind::Int(_)
            | hir::ExprKind::Float(_)
            | hir::ExprKind::Bool(_)
            | hir::ExprKind::String(_)
            | hir::ExprKind::Local(_)
            | hir::ExprKind::ItemRef(_) => {}
        }
    }

    fn check_selectors(&mut self, expression: hir::ExprId, page: &hir::Expr, function: &hir::Expr) {
        let page = self.evaluated_selector(page).map(|value| {
            u8::try_from(value)
                .map_err(|_| self.emit_out_of_range(SelectorKind::Page, value, page.span))
        });
        let function = self.evaluated_selector(function).map(|value| {
            u16::try_from(value)
                .map_err(|_| self.emit_out_of_range(SelectorKind::Function, value, function.span))
        });

        if let (Some(Ok(page)), Some(Ok(function))) = (page, function) {
            self.results
                .record(expression, ResolvedSelectors { page, function });
        }
    }

    fn evaluated_selector(&mut self, expression: &hir::Expr) -> Option<i32> {
        match self.const_eval.expression(expression.id) {
            Some(ConstValue::Int(value)) => Some(*value),
            Some(_) => unreachable!("non-integer raw syscall selector passed type checking"),
            None => {
                self.failed = true;
                None
            }
        }
    }

    fn emit_out_of_range(&mut self, selector: SelectorKind, value: i32, span: Span) {
        self.failed = true;
        self.diagnostics
            .emit_error(Diagnostic::from(SelectorOutOfRange {
                selector,
                value,
                span,
            }));
    }
}
