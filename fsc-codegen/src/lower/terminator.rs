use super::{block::BlockLowerer, operand::ScalarKind};
use crate::CodegenResult;
use fsb_ir::{BranchKind, BranchMode};
use fsc_mir::{Terminator, TerminatorKind};

impl BlockLowerer<'_, '_, '_> {
    pub(super) fn lower_terminator(&mut self, terminator: &Terminator) -> CodegenResult<()> {
        let terminator = match &terminator.kind {
            TerminatorKind::Goto { target } => {
                fsb_ir::Terminator::Jump(self.blocks.target(*target))
            }
            TerminatorKind::Branch {
                condition,
                then_target,
                else_target,
            } => {
                let condition = self.lower_scalar_operand(condition)?;
                debug_assert_eq!(
                    condition,
                    ScalarKind::Bool,
                    "MIR branch condition must have boolean type"
                );

                fsb_ir::Terminator::Branch {
                    kind: BranchKind::IfNonZero(BranchMode::Normal),
                    taken: self.blocks.target(*then_target),
                    not_taken: self.blocks.target(*else_target),
                }
            }
            TerminatorKind::Return | TerminatorKind::Unreachable => {
                todo!("return and unreachable terminator lowering")
            }
        };

        self.output.set_terminator(terminator);
        Ok(())
    }
}
