use super::block::BlockLowerer;
use crate::CodegenResult;
use fsb_ir::Instruction;
use fsc_mir::{FunctionId, Operand, Place};

impl BlockLowerer<'_, '_> {
    pub(super) fn lower_call(
        &mut self,
        destination: Option<&Place>,
        callee: FunctionId,
        arguments: &[Operand],
    ) -> CodegenResult<()> {
        //
        for argument in arguments.iter().rev() {
            self.lower_scalar_operand(argument)?;
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
