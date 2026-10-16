use crate::hir;
use crate::types::{FieldIdx, Ty};
use cranelift_entity::SecondaryMap;
use fsc_diagnostics::ErrorGuaranteed;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ExprType {
    Value(Ty),
    Function(hir::ItemId),
}

pub(crate) type ExprTypeResult = Result<ExprType, ErrorGuaranteed>;

#[derive(Debug, Default)]
pub(crate) struct TypeckResults {
    expression_types: SecondaryMap<hir::ExprId, Option<ExprType>>,
    field_indices: SecondaryMap<hir::ExprId, Option<FieldIdx>>,
}

impl TypeckResults {
    pub(crate) fn expression_type(&self, expression: hir::ExprId) -> Option<&ExprType> {
        self.expression_types
            .get(expression)
            .and_then(Option::as_ref)
    }

    pub(crate) fn field_index(&self, expression: hir::ExprId) -> Option<FieldIdx> {
        self.field_indices.get(expression).and_then(|field| *field)
    }

    pub(super) fn record_expression(&mut self, id: hir::ExprId, expression_type: ExprType) {
        let previous = self.expression_types.insert(id, Some(expression_type));
        debug_assert!(
            previous.flatten().is_none(),
            "type-check result for {id:?} was recorded more than once"
        );
    }

    pub(super) fn record_field(&mut self, expression: hir::ExprId, field: FieldIdx) {
        let previous = self.field_indices.insert(expression, Some(field));
        debug_assert!(
            previous.flatten().is_none(),
            "field for {expression:?} was recorded more than once"
        );
    }
}
