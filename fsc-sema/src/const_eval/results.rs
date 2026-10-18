use super::ConstValue;
use crate::hir;
use cranelift_entity::SecondaryMap;

#[derive(Debug, Default)]
pub(crate) struct ConstEvalResults {
    items: SecondaryMap<hir::ItemId, Option<ConstValue>>,
    bodies: SecondaryMap<hir::ItemId, Option<EvaluatedBodyConstants>>,
    expressions: SecondaryMap<hir::ExprId, Option<ConstValue>>,
}

impl ConstEvalResults {
    pub(super) fn record_item(&mut self, item: hir::ItemId, value: ConstValue) {
        let previous = self.items.insert(item, Some(value));
        debug_assert!(
            previous.flatten().is_none(),
            "constant value for {item:?} was recorded more than once"
        );
    }

    pub(crate) fn item(&self, item: hir::ItemId) -> Option<&ConstValue> {
        self.items.get(item).and_then(Option::as_ref)
    }

    pub(crate) fn local(&self, owner: hir::ItemId, local: hir::LocalId) -> Option<&ConstValue> {
        self.body(owner).and_then(|body| body.local(local))
    }

    pub(super) fn record_expression(&mut self, expression: hir::ExprId, value: ConstValue) {
        let previous = self.expressions.insert(expression, Some(value));
        debug_assert!(
            previous.flatten().is_none(),
            "constant value for expression {expression:?} was recorded more than once"
        );
    }

    pub(crate) fn expression(&self, expression: hir::ExprId) -> Option<&ConstValue> {
        self.expressions.get(expression).and_then(Option::as_ref)
    }

    pub(super) fn record_body(&mut self, owner: hir::ItemId, body: EvaluatedBodyConstants) {
        let previous = self.bodies.insert(owner, Some(body));
        debug_assert!(
            previous.flatten().is_none(),
            "constant values for body {owner:?} were recorded more than once"
        );
    }

    pub(super) fn body(&self, owner: hir::ItemId) -> Option<&EvaluatedBodyConstants> {
        self.bodies.get(owner).and_then(Option::as_ref)
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct EvaluatedBodyConstants {
    locals: SecondaryMap<hir::LocalId, Option<ConstValue>>,
}

impl EvaluatedBodyConstants {
    pub(super) fn record_local(&mut self, local: hir::LocalId, value: ConstValue) {
        let previous = self.locals.insert(local, Some(value));
        debug_assert!(
            previous.flatten().is_none(),
            "constant value for {local:?} was recorded more than once"
        );
    }

    pub(super) fn local(&self, local: hir::LocalId) -> Option<&ConstValue> {
        self.locals.get(local).and_then(Option::as_ref)
    }
}
