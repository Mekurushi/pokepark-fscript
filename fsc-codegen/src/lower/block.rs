use super::function::FunctionLoweringContext;
use crate::CodegenResult;
use fsc_mir::BasicBlock;

pub(super) struct BlockLowerer<'function, 'output> {
    pub(super) function: &'function FunctionLoweringContext<'function>,
    pub(super) output: &'output mut fsb_ir::BasicBlock,
}

impl<'function, 'output> BlockLowerer<'function, 'output> {
    pub(super) const fn new(
        function: &'function FunctionLoweringContext<'function>,
        output: &'output mut fsb_ir::BasicBlock,
    ) -> Self {
        Self { function, output }
    }

    pub(super) fn lower(&mut self, block: &BasicBlock) -> CodegenResult<()> {
        for statement in &block.statements {
            self.lower_statement(statement)?;
        }

        self.lower_terminator(&block.terminator)
    }
}
