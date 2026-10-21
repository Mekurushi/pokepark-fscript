use super::{block::BlockLowerer, operand::ScalarKind};
use crate::CodegenResult;
use fsb_ir::{BranchKind, BranchMode, ReturnKind};
use fsc_mir::{Place, Terminator, TerminatorKind, Ty};

impl BlockLowerer<'_, '_> {
    pub(super) fn lower_terminator(&mut self, terminator: &Terminator) -> CodegenResult<()> {
        let terminator = match &terminator.kind {
            TerminatorKind::Goto { target } => {
                fsb_ir::Terminator::Jump(self.function.block(*target))
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
                    taken: self.function.block(*then_target),
                    not_taken: self.function.block(*else_target),
                }
            }
            TerminatorKind::Return => self.lower_return()?,
            TerminatorKind::Unreachable => todo!("unreachable terminator lowering"),
        };

        self.output.set_terminator(terminator);
        Ok(())
    }

    fn lower_return(&mut self) -> CodegenResult<fsb_ir::Terminator> {
        let kind = match self.function.return_type() {
            Ty::Void => ReturnKind::None,
            Ty::Int | Ty::Float | Ty::Bool | Ty::Str => ReturnKind::Value,
            Ty::Vec3 | Ty::Array { .. } | Ty::Reference(_) => {
                unreachable!("MIR function must have a supported return type")
            }
        };

        if kind == ReturnKind::Value {
            let return_place = self.function.resolve_place(&Place::return_place())?;
            self.lower_load_word(&return_place)?;
        }

        Ok(fsb_ir::Terminator::Return {
            kind,
            frame_slots: self.function.frame_slots(),
        })
    }
}
