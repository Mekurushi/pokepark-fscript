use super::error::ConstEvalError;
use super::operation;
use super::results::EvaluatedBodyConstants;
use super::{ConstValue, EvaluatedConstants};
use crate::config::BoundConfigs;
use crate::hir;
use cranelift_entity::SecondaryMap;
use fsc_diagnostics::{Diagnostic, Diagnostics, ErrorGuaranteed, Span};

#[derive(Debug, Clone, Copy)]
enum EvaluationState {
    Evaluating,
    Failed(ErrorGuaranteed),
}

#[derive(Debug)]
struct EvaluationContext<'diagnostics> {
    states: SecondaryMap<hir::ItemId, Option<EvaluationState>>,
    stack: Vec<hir::ItemId>,
    results: EvaluatedConstants,
    diagnostics: &'diagnostics mut Diagnostics,
    failed: bool,
}

impl<'diagnostics> EvaluationContext<'diagnostics> {
    fn new(diagnostics: &'diagnostics mut Diagnostics) -> Self {
        Self {
            states: SecondaryMap::new(),
            stack: Vec::new(),
            results: EvaluatedConstants::default(),
            diagnostics,
            failed: false,
        }
    }
}

#[derive(Clone, Copy)]
enum LocalValues<'a> {
    Unavailable,
    Body {
        body: &'a hir::Body,
        evaluation: &'a BodyEvaluation,
    },
}

#[derive(Debug, Default)]
struct BodyEvaluation {
    constants: EvaluatedBodyConstants,
    failures: SecondaryMap<hir::LocalId, Option<ErrorGuaranteed>>,
}

pub(crate) fn evaluate_items(
    script: &hir::Script,
    configs: &BoundConfigs,
    diagnostics: &mut Diagnostics,
) -> Option<EvaluatedConstants> {
    let mut context = EvaluationContext::new(diagnostics);
    for (item, declaration) in &script.items {
        if matches!(declaration.kind, hir::ItemKind::Constant(_)) {
            let _ = context.evaluate_item(script, configs, item, declaration.name_span);
        }
    }
    context.evaluate_bodies(script, configs);

    if context.failed {
        None
    } else {
        Some(context.results)
    }
}

impl EvaluationContext<'_> {
    fn evaluate_item(
        &mut self,
        script: &hir::Script,
        configs: &BoundConfigs,
        item: hir::ItemId,
        reference_span: Span,
    ) -> Result<ConstValue, ErrorGuaranteed> {
        if let Some(value) = self.results.item(item) {
            return Ok(value.clone());
        }
        match self.states.get(item).and_then(|state| *state) {
            Some(EvaluationState::Evaluating) => {
                return Err(self.emit_cycle(script, item, reference_span));
            }
            Some(EvaluationState::Failed(error)) => return Err(error),
            None => {}
        }

        let hir::ItemKind::Constant(constant) = &script.items[item].kind else {
            return self.evaluate_non_constant_item(script, configs, item, reference_span);
        };

        self.states.insert(item, Some(EvaluationState::Evaluating));
        self.stack.push(item);
        let result = self.evaluate_expression(
            script,
            configs,
            LocalValues::Unavailable,
            &constant.initializer,
        );
        let popped = self.stack.pop();
        debug_assert_eq!(popped, Some(item));

        match result {
            Ok(value) => {
                self.states.insert(item, None);
                self.results.record_item(item, value.clone());
                Ok(value)
            }
            Err(error) => {
                self.states
                    .insert(item, Some(EvaluationState::Failed(error)));
                Err(error)
            }
        }
    }

    fn evaluate_expression(
        &mut self,
        script: &hir::Script,
        configs: &BoundConfigs,
        locals: LocalValues<'_>,
        expression: &hir::Expr,
    ) -> Result<ConstValue, ErrorGuaranteed> {
        match &expression.kind {
            hir::ExprKind::Error(error) => Err(*error),
            hir::ExprKind::Int(value) => Ok(ConstValue::Int(*value)),
            hir::ExprKind::Float(value) => Ok(ConstValue::Float(*value)),
            hir::ExprKind::Bool(value) => Ok(ConstValue::Bool(*value)),
            hir::ExprKind::String(value) => Ok(ConstValue::String(value.clone())),
            hir::ExprKind::ItemRef(item) => {
                self.evaluate_item(script, configs, *item, expression.span)
            }
            hir::ExprKind::Local(local) => self.evaluate_local(locals, *local, expression.span),
            hir::ExprKind::Member { .. } => {
                Err(self.emit_error(ConstEvalError::NonConstantExpression {
                    kind: "member access",
                    span: expression.span,
                }))
            }
            hir::ExprKind::Unary { op, expr } => {
                let operand = self.evaluate_expression(script, configs, locals, expr)?;
                operation::unary(*op, operand, expression.span)
                    .map_err(|error| self.emit_error(error))
            }
            hir::ExprKind::Binary { .. } => {
                self.evaluate_binary(script, configs, locals, expression)
            }
            hir::ExprKind::Call { .. } => {
                Err(self.emit_error(ConstEvalError::NonConstantExpression {
                    kind: "function call",
                    span: expression.span,
                }))
            }
            hir::ExprKind::RawSyscall { .. } => {
                Err(self.emit_error(ConstEvalError::NonConstantExpression {
                    kind: "raw syscall",
                    span: expression.span,
                }))
            }
        }
    }

    fn evaluate_binary(
        &mut self,
        script: &hir::Script,
        configs: &BoundConfigs,
        locals: LocalValues<'_>,
        expression: &hir::Expr,
    ) -> Result<ConstValue, ErrorGuaranteed> {
        let hir::ExprKind::Binary { op, lhs, rhs } = &expression.kind else {
            unreachable!("non-binary expression reached binary constant evaluation");
        };
        let lhs = self.evaluate_expression(script, configs, locals, lhs)?;
        // doing the short-circuiting evaluation before passing further into evaluating rhs
        match (op, &lhs) {
            (hir::BinaryOp::And, ConstValue::Bool(false)) => {
                return Ok(ConstValue::Bool(false));
            }
            (hir::BinaryOp::Or, ConstValue::Bool(true)) => {
                return Ok(ConstValue::Bool(true));
            }
            _ => {}
        }

        let rhs = self.evaluate_expression(script, configs, locals, rhs)?;
        operation::binary(*op, lhs, rhs, expression.span).map_err(|error| self.emit_error(error))
    }

    fn evaluate_local(
        &mut self,
        locals: LocalValues<'_>,
        local: hir::LocalId,
        span: Span,
    ) -> Result<ConstValue, ErrorGuaranteed> {
        let LocalValues::Body { body, evaluation } = locals else {
            return Err(self.emit_error(ConstEvalError::NonConstantExpression {
                kind: "local value",
                span,
            }));
        };

        match body.locals[local].kind {
            hir::LocalKind::Constant => {
                if let Some(value) = evaluation.constants.local(local) {
                    return Ok(value.clone());
                }
                if let Some(error) = evaluation.failures.get(local).and_then(|error| *error) {
                    return Err(error);
                }

                debug_assert!(false, "unevaluated local constant was referenced");
                Err(self.emit_error(ConstEvalError::NonConstantExpression {
                    kind: "unevaluated local constant",
                    span,
                }))
            }
            hir::LocalKind::Parameter { .. } | hir::LocalKind::Variable => {
                Err(self.emit_error(ConstEvalError::NonConstantExpression {
                    kind: "runtime local value",
                    span,
                }))
            }
        }
    }

    fn evaluate_bodies(&mut self, script: &hir::Script, configs: &BoundConfigs) {
        for (owner, item) in &script.items {
            let hir::ItemKind::Function(hir::Function {
                kind: hir::FunctionKind::Defined { body, .. },
                ..
            }) = &item.kind
            else {
                continue;
            };

            let mut evaluation = BodyEvaluation::default();
            self.evaluate_block(script, configs, body, &body.block, &mut evaluation);
            self.results.record_body(owner, evaluation.constants);
        }
    }

    // found out through this that you could make constants forward reference due the nature of
    // constants being constant. The additional complexity is it in my eyes not worth, so I make
    // it myself easy and keep it out. Still trying my best to not block it out.
    fn evaluate_block(
        &mut self,
        script: &hir::Script,
        configs: &BoundConfigs,
        body: &hir::Body,
        block: &hir::Block,
        evaluation: &mut BodyEvaluation,
    ) {
        for statement in &block.statements {
            match &statement.kind {
                hir::StmtKind::ConstantDeclaration { local, initializer } => {
                    let locals = LocalValues::Body { body, evaluation };
                    match self.evaluate_expression(script, configs, locals, initializer) {
                        Ok(value) => evaluation.constants.record_local(*local, value),
                        Err(error) => {
                            evaluation.failures.insert(*local, Some(error));
                        }
                    }
                }
                hir::StmtKind::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    self.evaluate_block(script, configs, body, then_block, evaluation);
                    if let Some(else_block) = else_block {
                        self.evaluate_block(script, configs, body, else_block, evaluation);
                    }
                }
                hir::StmtKind::While {
                    body: loop_body, ..
                } => {
                    self.evaluate_block(script, configs, body, loop_body, evaluation);
                }
                hir::StmtKind::Return(_)
                | hir::StmtKind::Break
                | hir::StmtKind::VariableDeclaration { .. }
                | hir::StmtKind::Assign { .. }
                | hir::StmtKind::Expression(_)
                | hir::StmtKind::Pause(_) => {}
            }
        }
    }

    fn evaluate_non_constant_item(
        &mut self,
        script: &hir::Script,
        configs: &BoundConfigs,
        item: hir::ItemId,
        reference_span: Span,
    ) -> Result<ConstValue, ErrorGuaranteed> {
        match &script.items[item].kind {
            hir::ItemKind::Config(_) => {
                let Some(value) = configs.value(item) else {
                    debug_assert!(false, "unbound config reached constant evaluation");
                    let error = ConstEvalError::MissingConfigValue {
                        name: script.items[item].name.clone(),
                        span: reference_span,
                    };
                    return Err(self.emit_error(error));
                };

                Ok(ConstValue::from(value))
            }
            hir::ItemKind::Function(_) => {
                Err(self.emit_error(ConstEvalError::NonConstantExpression {
                    kind: "function",
                    span: reference_span,
                }))
            }
            hir::ItemKind::Constant(_) => {
                debug_assert!(false, "constant was routed as a non-constant item");
                Err(self.emit_error(ConstEvalError::NonConstantExpression {
                    kind: "constant",
                    span: reference_span,
                }))
            }
        }
    }

    fn emit_cycle(
        &mut self,
        script: &hir::Script,
        repeated: hir::ItemId,
        reference_span: Span,
    ) -> ErrorGuaranteed {
        let cycle_start = self
            .stack
            .iter()
            .position(|item| *item == repeated)
            .unwrap_or(0);
        let constants = self.stack[cycle_start..]
            .iter()
            .map(|item| {
                let declaration = &script.items[*item];
                (declaration.name.clone(), declaration.name_span)
            })
            .collect();
        self.emit_error(ConstEvalError::DependencyCycle {
            reference_span,
            constants,
        })
    }
    fn emit_error(&mut self, error: ConstEvalError) -> ErrorGuaranteed {
        self.failed = true;
        self.diagnostics.emit_error(Diagnostic::from(error))
    }
}
