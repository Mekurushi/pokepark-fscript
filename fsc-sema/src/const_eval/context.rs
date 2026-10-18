use super::body::BodyEvaluator;
use super::error::ConstEvalError;
use super::expression::{ExpressionEvaluator, LocalValues};
use super::{ConstEvalResults, ConstValue};
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
pub(super) struct EvaluationContext<'analysis, 'diagnostics> {
    script: &'analysis hir::Script,
    configs: &'analysis BoundConfigs,
    states: SecondaryMap<hir::ItemId, Option<EvaluationState>>,
    stack: Vec<hir::ItemId>,
    results: ConstEvalResults,
    diagnostics: &'diagnostics mut Diagnostics,
    failed: bool,
}

impl<'analysis, 'diagnostics> EvaluationContext<'analysis, 'diagnostics> {
    fn new(
        script: &'analysis hir::Script,
        configs: &'analysis BoundConfigs,
        diagnostics: &'diagnostics mut Diagnostics,
    ) -> Self {
        Self {
            script,
            configs,
            states: SecondaryMap::new(),
            stack: Vec::new(),
            results: ConstEvalResults::default(),
            diagnostics,
            failed: false,
        }
    }
}

pub(crate) fn evaluate(
    script: &hir::Script,
    configs: &BoundConfigs,
    diagnostics: &mut Diagnostics,
) -> Option<ConstEvalResults> {
    let mut context = EvaluationContext::new(script, configs, diagnostics);
    for (item, declaration) in &script.items {
        if matches!(declaration.kind, hir::ItemKind::Constant(_))
            && context.evaluate_item(item, declaration.name_span).is_err()
        {
            context.mark_failed();
        }
    }
    context.evaluate_bodies();

    if context.failed {
        None
    } else {
        Some(context.results)
    }
}

impl EvaluationContext<'_, '_> {
    pub(super) fn evaluate_item(
        &mut self,
        item: hir::ItemId,
        reference_span: Span,
    ) -> Result<ConstValue, ErrorGuaranteed> {
        if let Some(value) = self.results.item(item) {
            return Ok(value.clone());
        }
        match self.states.get(item).and_then(|state| *state) {
            Some(EvaluationState::Evaluating) => {
                return Err(self.emit_cycle(item, reference_span));
            }
            Some(EvaluationState::Failed(error)) => return Err(error),
            None => {}
        }

        let hir::ItemKind::Constant(constant) = &self.script.items[item].kind else {
            return self.evaluate_non_constant_item(item, reference_span);
        };

        self.states.insert(item, Some(EvaluationState::Evaluating));
        self.stack.push(item);
        let result = self.evaluate_expression(LocalValues::Unavailable, &constant.initializer);
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

    pub(super) fn evaluate_expression(
        &mut self,
        locals: LocalValues<'_>,
        expression: &hir::Expr,
    ) -> Result<ConstValue, ErrorGuaranteed> {
        ExpressionEvaluator::new(self, locals).evaluate(expression)
    }

    fn evaluate_bodies(&mut self) {
        let script = self.script;
        for (owner, item) in &script.items {
            let hir::ItemKind::Function(hir::Function {
                kind: hir::FunctionKind::Defined { body, .. },
                ..
            }) = &item.kind
            else {
                continue;
            };

            let constants = BodyEvaluator::new(self, body).evaluate();
            self.results.record_body(owner, constants);
        }
    }

    pub(super) fn record_expression(&mut self, expression: hir::ExprId, value: ConstValue) {
        self.results.record_expression(expression, value);
    }

    pub(super) fn mark_failed(&mut self) {
        self.failed = true;
    }

    fn evaluate_non_constant_item(
        &mut self,
        item: hir::ItemId,
        reference_span: Span,
    ) -> Result<ConstValue, ErrorGuaranteed> {
        match &self.script.items[item].kind {
            hir::ItemKind::Config(_) => {
                let Some(value) = self.configs.value(item) else {
                    debug_assert!(false, "unbound config reached constant evaluation");
                    let error = ConstEvalError::MissingConfigValue {
                        name: self.script.items[item].name.clone(),
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

    fn emit_cycle(&mut self, repeated: hir::ItemId, reference_span: Span) -> ErrorGuaranteed {
        let cycle_start = self
            .stack
            .iter()
            .position(|item| *item == repeated)
            .unwrap_or(0);
        let constants = self.stack[cycle_start..]
            .iter()
            .map(|item| {
                let declaration = &self.script.items[*item];
                (declaration.name.clone(), declaration.name_span)
            })
            .collect();
        self.emit_error(ConstEvalError::DependencyCycle {
            reference_span,
            constants,
        })
    }
    pub(super) fn emit_error(&mut self, error: ConstEvalError) -> ErrorGuaranteed {
        self.diagnostics.emit_error(Diagnostic::from(error))
    }
}
