use crate::{CodegenResult, data::StaticData, frame::FrameLayout, place::resolver::PlaceResolver};
use fsc_mir::{BasicBlock, Body};

pub(super) struct BlockLowerer<'layout, 'mir, 'output> {
    pub(super) places: PlaceResolver<'layout, 'mir>,
    pub(super) output: &'output mut fsb_ir::BasicBlock,
}

impl<'layout, 'mir, 'output> BlockLowerer<'layout, 'mir, 'output> {
    pub(super) const fn new(
        body: &'mir Body,
        frame: &'layout FrameLayout,
        statics: &'layout StaticData,
        output: &'output mut fsb_ir::BasicBlock,
    ) -> Self {
        Self {
            places: PlaceResolver::new(frame, statics, body),
            output,
        }
    }

    pub(super) fn lower(&mut self, block: &BasicBlock) -> CodegenResult<()> {
        for statement in &block.statements {
            self.lower_statement(statement)?;
        }

        todo!("MIR terminator lowering")
    }
}
