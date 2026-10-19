use super::block::BlockLowerer;
use crate::CodegenResult;
use fsc_mir::{Statement, StatementKind};

impl BlockLowerer<'_, '_, '_> {
    pub(super) fn lower_statement(&mut self, statement: &Statement) -> CodegenResult<()> {
        match &statement.kind {
            StatementKind::Assign { target, value } => {
                let target = self.places.resolve(target)?;
                todo!("Rvalue lowering")
            }
            StatementKind::Call { .. }
            | StatementKind::RawSyscall { .. }
            | StatementKind::Pause { .. } => todo!("effectful statement lowering"),
        }
    }
}
