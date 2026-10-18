use super::context::EvaluationContext;
use super::expression::LocalValues;
use super::results::EvaluatedBodyConstants;
use super::rules;
use crate::hir;
use cranelift_entity::SecondaryMap;
use fsc_diagnostics::ErrorGuaranteed;

#[derive(Debug, Default)]
pub(super) struct BodyEvaluation {
    pub(super) constants: EvaluatedBodyConstants,
    pub(super) failures: SecondaryMap<hir::LocalId, Option<ErrorGuaranteed>>,
}

pub(super) struct BodyEvaluator<'context, 'analysis, 'diagnostics> {
    context: &'context mut EvaluationContext<'analysis, 'diagnostics>,
    body: &'analysis hir::Body,
    evaluation: BodyEvaluation,
}

impl<'context, 'analysis, 'diagnostics> BodyEvaluator<'context, 'analysis, 'diagnostics> {
    pub(super) fn new(
        context: &'context mut EvaluationContext<'analysis, 'diagnostics>,
        body: &'analysis hir::Body,
    ) -> Self {
        Self {
            context,
            body,
            evaluation: BodyEvaluation::default(),
        }
    }

    pub(super) fn evaluate(mut self) -> EvaluatedBodyConstants {
        self.evaluate_block(&self.body.block);
        self.evaluate_required_block(&self.body.block);
        self.evaluation.constants
    }

    // found out through this that you could make constants forward reference due the nature of
    // constants being constant. The additional complexity is it in my eyes not worth, so I make
    // it myself easy and keep it out. Still trying my best to not block it out.
    fn evaluate_block(&mut self, block: &hir::Block) {
        for statement in &block.statements {
            match &statement.kind {
                hir::StmtKind::ConstantDeclaration { local, initializer } => {
                    let locals = LocalValues::Body {
                        body: self.body,
                        evaluation: &self.evaluation,
                    };
                    match self.context.evaluate_expression(locals, initializer) {
                        Ok(value) => self.evaluation.constants.record_local(*local, value),
                        Err(error) => {
                            self.context.mark_failed();
                            self.evaluation.failures.insert(*local, Some(error));
                        }
                    }
                }
                hir::StmtKind::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    self.evaluate_block(then_block);
                    if let Some(else_block) = else_block {
                        self.evaluate_block(else_block);
                    }
                }
                hir::StmtKind::While {
                    body: loop_body, ..
                } => self.evaluate_block(loop_body),
                hir::StmtKind::Return(_)
                | hir::StmtKind::Break
                | hir::StmtKind::VariableDeclaration { .. }
                | hir::StmtKind::Assign { .. }
                | hir::StmtKind::Expression(_)
                | hir::StmtKind::Pause(_) => {}
            }
        }
    }

    fn evaluate_required_block(&mut self, block: &hir::Block) {
        for statement in &block.statements {
            match &statement.kind {
                hir::StmtKind::Return(value) => {
                    if let Some(value) = value {
                        self.evaluate_required_expression(value);
                    }
                }
                hir::StmtKind::VariableDeclaration { initializer, .. } => {
                    if let Some(initializer) = initializer {
                        self.evaluate_required_expression(initializer);
                    }
                }
                hir::StmtKind::ConstantDeclaration { initializer, .. }
                | hir::StmtKind::Expression(initializer)
                | hir::StmtKind::Pause(initializer) => {
                    self.evaluate_required_expression(initializer);
                }
                hir::StmtKind::Assign { target, value } => {
                    self.evaluate_required_expression(target);
                    self.evaluate_required_expression(value);
                }
                hir::StmtKind::If {
                    condition,
                    then_block,
                    else_block,
                } => {
                    self.evaluate_required_expression(condition);
                    self.evaluate_required_block(then_block);
                    if let Some(else_block) = else_block {
                        self.evaluate_required_block(else_block);
                    }
                }
                hir::StmtKind::While {
                    condition,
                    body: loop_body,
                } => {
                    self.evaluate_required_expression(condition);
                    self.evaluate_required_block(loop_body);
                }
                hir::StmtKind::Break => {}
            }
        }
    }

    fn evaluate_required_expression(&mut self, expression: &hir::Expr) {
        let locals = LocalValues::Body {
            body: self.body,
            evaluation: &self.evaluation,
        };
        for required in rules::const_contexts(expression) {
            if let Ok(value) = self.context.evaluate_expression(locals, required) {
                self.context.record_expression(required.id, value);
            }
        }

        match &expression.kind {
            hir::ExprKind::Member { base, .. } | hir::ExprKind::Unary { expr: base, .. } => {
                self.evaluate_required_expression(base);
            }
            hir::ExprKind::Binary { lhs, rhs, .. } => {
                self.evaluate_required_expression(lhs);
                self.evaluate_required_expression(rhs);
            }
            hir::ExprKind::Call { callee, arguments } => {
                self.evaluate_required_expression(callee);
                for argument in arguments {
                    self.evaluate_required_expression(argument);
                }
            }
            hir::ExprKind::RawSyscall { arguments } => {
                for argument in arguments {
                    self.evaluate_required_expression(argument);
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
}
