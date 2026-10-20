use super::block::BlockLowerer;
use crate::{abi::pass_mode, abi::PassMode, CodegenError, CodegenResult};
use fsb_ir::Instruction;
use fsc_mir::{Operand, Place};

impl BlockLowerer<'_, '_> {
    pub(super) fn lower_raw_syscall(
        &mut self,
        destination: Option<&Place>,
        page: u8,
        function: u16,
        arguments: &[Operand],
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

    fn lower_raw_syscall_argument(&mut self, argument: &Operand) -> CodegenResult<()> {
        match argument {
            Operand::Place(place) => {
                let place = self.function.resolve_place(place)?;
                match pass_mode(&place.ty) {
                    Some(PassMode::Direct) => self.lower_load_word(&place),
                    Some(PassMode::Indirect) => {
                        self.lower_place_address(&place);
                        Ok(())
                    }
                    None => unreachable!("void raw syscall argument reached codegen"),
                }
            }
            Operand::Constant(constant) => match pass_mode(&constant.ty) {
                Some(PassMode::Direct) => {
                    self.lower_scalar_constant_value(&constant.value);
                    Ok(())
                }
                Some(PassMode::Indirect) => Err(CodegenError::UnsupportedType(constant.ty.clone())),
                None => unreachable!("void raw syscall argument reached codegen"),
            },
        }
    }
}
