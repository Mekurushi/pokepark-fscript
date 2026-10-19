use super::block::BlockLowerer;
use crate::{place::resolver::ResolvedPlace, CodegenError, CodegenResult};
use fsc_mir::{aggregate_fields, ConstValue, Constant, FieldOperand, Operand, Rvalue, Ty};

impl BlockLowerer<'_, '_, '_> {
    pub(super) fn lower_rvalue_into(
        &mut self,
        value: &Rvalue,
        target: &ResolvedPlace,
    ) -> CodegenResult<()> {
        match value {
            Rvalue::Use(operand) => self.lower_operand_into(operand, target),
            Rvalue::Unary { .. } | Rvalue::Binary { .. } => todo!("operator rvalue lowering"),
            Rvalue::Array { elements } => self.lower_array_into(elements, target),
            Rvalue::Aggregate { ty: _ty, fields } => self.lower_aggregate_into(fields, target),
        }
    }

    fn lower_array_into(
        &mut self,
        elements: &[Operand],
        target: &ResolvedPlace,
    ) -> CodegenResult<()> {
        let Ty::Array { length, .. } = &target.ty else {
            return Err(CodegenError::UnsupportedType(target.ty.clone()));
        };
        debug_assert_eq!(
            elements.len(),
            *length,
            "MIR array rvalue must initialize every element"
        );

        for (index, element) in elements.iter().enumerate() {
            let target = target.project_constant_index(index)?;
            self.lower_operand_into(element, &target)?;
        }
        Ok(())
    }

    fn lower_aggregate_into(
        &mut self,
        fields: &[FieldOperand],
        target: &ResolvedPlace,
    ) -> CodegenResult<()> {
        for field in fields {
            let target = target.project_field(field.field)?;
            self.lower_operand_into(&field.value, &target)?;
        }
        Ok(())
    }

    fn lower_operand_into(
        &mut self,
        operand: &Operand,
        target: &ResolvedPlace,
    ) -> CodegenResult<()> {
        match operand {
            Operand::Place(source) => {
                let source = self.places.resolve(source)?;
                self.copy_place(&source, target)
            }
            Operand::Constant(constant) => self.lower_constant_into(constant, target),
        }
    }

    fn copy_place(&mut self, source: &ResolvedPlace, target: &ResolvedPlace) -> CodegenResult<()> {
        match &source.ty {
            Ty::Int | Ty::Float | Ty::Bool | Ty::Str => {
                self.lower_load_word(source)?;
                self.lower_store_word(target)
            }
            Ty::Vec3 => {
                let fields = aggregate_fields(&source.ty)
                    .ok_or_else(|| CodegenError::UnsupportedType(source.ty.clone()))?;
                for field in fields {
                    let source = source.project_field(field.index())?;
                    let target = target.project_field(field.index())?;
                    self.copy_place(&source, &target)?;
                }
                Ok(())
            }
            Ty::Array { length, .. } => {
                for index in 0..*length {
                    let source = source.project_constant_index(index)?;
                    let target = target.project_constant_index(index)?;
                    self.copy_place(&source, &target)?;
                }
                Ok(())
            }
            Ty::Void => todo!("void place reached code generation"),
        }
    }

    fn lower_constant_into(
        &mut self,
        constant: &Constant,
        target: &ResolvedPlace,
    ) -> CodegenResult<()> {
        match constant.value {
            ConstValue::Int(_)
            | ConstValue::Float(_)
            | ConstValue::Bool(_)
            | ConstValue::String(_) => {
                self.lower_constant(constant);
                self.lower_store_word(target)
            }
            ConstValue::Array(_) | ConstValue::Aggregate(_) => {
                todo!("aggregate constant lowering")
            }
        }
    }
}
