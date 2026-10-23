use super::function::FunctionLowerer;
use crate::{CodegenResult, stackify::StatementLocation};
use fsc_mir::{BasicBlock, BasicBlockId};

pub(super) struct BlockLowerer<'function, 'output> {
    pub(super) function: &'function FunctionLowerer<'function>,
    block: BasicBlockId,
    pub(super) output: &'output mut fsb_ir::BasicBlock,
}

impl<'function, 'output> BlockLowerer<'function, 'output> {
    pub(super) const fn new(
        function: &'function FunctionLowerer<'function>,
        block: BasicBlockId,
        output: &'output mut fsb_ir::BasicBlock,
    ) -> Self {
        Self {
            function,
            block,
            output,
        }
    }

    pub(super) fn lower(&mut self, block: &BasicBlock) -> CodegenResult<()> {
        for (statement_index, statement) in block.statements.iter().enumerate() {
            let location = StatementLocation {
                block: self.block,
                statement: statement_index,
            };
            if self.function.selects_statement(location) {
                continue;
            }
            self.lower_statement(statement)?;
        }

        self.lower_terminator(&block.terminator)
    }
}
