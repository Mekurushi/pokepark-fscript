use super::block::BlockLowerer;
use crate::{CodegenResult, place::resolver::ResolvedPlace};
use fsc_mir::{ConstValue, Constant, Operand, Rvalue, Ty};

impl BlockLowerer<'_, '_, '_> {
    pub(super) fn lower_rvalue_into(
        &mut self,
        value: &Rvalue,
        target: &ResolvedPlace,
    ) -> CodegenResult<()> {
        match value {
            Rvalue::Use(operand) => self.lower_operand_into(operand, target),
            Rvalue::Unary { .. } | Rvalue::Binary { .. } => todo!("operator rvalue lowering"),
            Rvalue::Array { .. } | Rvalue::Aggregate { .. } => {
                todo!("aggregate rvalue lowering")
            }
        }
    }

    fn lower_operand_into(
        &mut self,
        operand: &Operand,
        target: &ResolvedPlace,
    ) -> CodegenResult<()> {
        match operand {
            Operand::Place(source) => {
                let source = self.places.resolve(source)?;
                match &source.ty {
                    Ty::Int | Ty::Float | Ty::Bool | Ty::Str => {
                        self.lower_load_word(&source)?;
                        self.lower_store_word(target)
                    }
                    Ty::Vec3 | Ty::Array { .. } => todo!("aggregate place copy lowering"),
                    Ty::Void => todo!("void place reached code generation"),
                }
            }
            Operand::Constant(constant) => self.lower_constant_into(constant, target),
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
