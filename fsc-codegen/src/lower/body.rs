use super::{block::BlockLowerer, block_mapping::BlockMapping};
use crate::{CodegenResult, data::StaticData, frame::FrameLayout};
use fsc_mir::{Body, START_BLOCK};

pub(super) struct BodyLowerer<'layout, 'mir, 'output> {
    body: &'mir Body,
    frame: FrameLayout,
    statics: &'layout StaticData,
    output: &'output mut fsb_ir::Function,
    blocks: BlockMapping,
}

impl<'layout, 'mir, 'output> BodyLowerer<'layout, 'mir, 'output> {
    pub(super) fn new(
        body: &'mir Body,
        frame: FrameLayout,
        statics: &'layout StaticData,
        output: &'output mut fsb_ir::Function,
    ) -> Self {
        let blocks = BlockMapping::allocate(body, output);

        Self {
            body,
            frame,
            statics,
            output,
            blocks,
        }
    }

    pub(super) fn lower(self) -> CodegenResult<()> {
        for (source, block) in &self.body.blocks {
            let target = self.blocks.target(source);
            let output = self.output.block_mut(target);
            if source == START_BLOCK && self.frame.local_slots() != 0 {
                output.push_instruction(fsb_ir::Instruction::GrowStack(self.frame.local_slots()));
            }
            BlockLowerer::new(self.body, &self.frame, self.statics, &self.blocks, output)
                .lower(block)?;
        }

        Ok(())
    }
}
