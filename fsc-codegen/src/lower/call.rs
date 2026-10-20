use super::block::BlockLowerer;
use crate::{CodegenError, CodegenResult, abi::PassMode};
use fsb_ir::Instruction;
use fsc_mir::{FunctionId, Operand, Place};

impl BlockLowerer<'_, '_> {
    pub(super) fn lower_call(
        &mut self,
        destination: Option<&Place>,
        callee: FunctionId,
        arguments: &[Operand],
    ) -> CodegenResult<()> {
        for (index, argument) in arguments.iter().enumerate().rev() {
            let mode = self
                .function
                .function_declaration(callee)?
                .parameter_modes()[index];

            match mode {
                PassMode::Direct => {
                    self.lower_scalar_operand(argument)?;
                }
                PassMode::Indirect => match argument {
                    Operand::Place(place) => {
                        let place = self.function.resolve_place(place)?;
                        self.lower_place_address(&place);
                    }
                    Operand::Constant(constant) => {
                        return Err(CodegenError::UnsupportedType(constant.ty.clone()));
                    }
                },
            }
        }

        let callee = self.function.function(callee)?;
        self.output.push_instruction(Instruction::Call(callee));

        if let Some(destination) = destination {
            let destination = self.function.resolve_place(destination)?;
            self.output.push_instruction(Instruction::PushResult);
            self.lower_store_word(&destination)?;
        }

        Ok(())
    }
}
