use super::block::BlockLowerer;
use crate::CodegenResult;
use fsb_ir::Instruction;
use fsc_mir::{CallArgument, Place};

impl BlockLowerer<'_, '_> {
    pub(super) fn lower_raw_syscall(
        &mut self,
        destination: Option<&Place>,
        page: u8,
        function: u16,
        arguments: &[CallArgument],
    ) -> CodegenResult<()> {
        // TODO: theoretically invariant, but sema is still not validating (who would even ever use
        // 256 parameters)
        let argument_count = arguments.len() as u8;

        for argument in arguments.iter().rev() {
            self.lower_raw_syscall_argument(argument)?;
        }

        self.output.push_instruction(Instruction::Syscall {
            argument_count,
            page,
            function,
        });

        if let Some(destination) = destination {
            let destination = self.function.resolve_place(destination)?;
            self.output.push_instruction(Instruction::PushResult);
            self.lower_store_word(&destination)?;
        }

        Ok(())
    }

    fn lower_raw_syscall_argument(&mut self, argument: &CallArgument) -> CodegenResult<()> {
        match argument {
            CallArgument::Reference(place) => {
                let place = self.function.resolve_place(place)?;
                self.lower_place_address(&place);
                Ok(())
            }
            CallArgument::Value(argument) => self.lower_scalar_operand(argument).map(|_| ()),
        }
    }
}
