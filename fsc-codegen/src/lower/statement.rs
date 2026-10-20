use super::{block::BlockLowerer, operand::ScalarKind};
use crate::CodegenResult;
use fsb_ir::Instruction;
use fsc_mir::{Operand, Statement, StatementKind};

impl BlockLowerer<'_, '_> {
    pub(super) fn lower_statement(&mut self, statement: &Statement) -> CodegenResult<()> {
        match &statement.kind {
            StatementKind::Assign { target, value } => {
                let target = self.function.resolve_place(target)?;
                self.lower_rvalue_into(value, &target)
            }
            StatementKind::Pause { duration } => self.lower_pause(duration),
            StatementKind::Call { .. } | StatementKind::RawSyscall { .. } => {
                todo!("call statement lowering")
            }
        }
    }

    fn lower_pause(&mut self, duration: &Operand) -> CodegenResult<()> {
        let duration = self.lower_scalar_operand(duration)?;
        debug_assert_eq!(
            duration,
            ScalarKind::Integer,
            "MIR pause duration must have integer type"
        );
        self.output.push_instruction(Instruction::DelayFromStack);
        Ok(())
    }
}
