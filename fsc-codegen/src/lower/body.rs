use super::block::BlockLowerer;
use crate::{CodegenError, CodegenResult, data::StaticData, frame::FrameLayout};
use cranelift_entity::SecondaryMap;
use fsc_mir::{BasicBlockId, Body, START_BLOCK};

pub(super) struct BodyLowerer<'layout, 'mir, 'output> {
    body: &'mir Body,
    frame: FrameLayout,
    statics: &'layout StaticData,
    output: &'output mut fsb_ir::Function,
    blocks: SecondaryMap<BasicBlockId, Option<fsb_ir::BlockId>>,
}

impl<'layout, 'mir, 'output> BodyLowerer<'layout, 'mir, 'output> {
    pub(super) fn new(
        body: &'mir Body,
        frame: FrameLayout,
        statics: &'layout StaticData,
        output: &'output mut fsb_ir::Function,
    ) -> Self {
        let mut blocks = SecondaryMap::new();
        for source in body.blocks.keys() {
            blocks.insert(source, Some(output.create_block()));
        }

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
            let target = self.block(source)?;
            let output = self.output.block_mut(target);
            if source == START_BLOCK && self.frame.local_slots() != 0 {
                output.push_instruction(fsb_ir::Instruction::GrowStack(self.frame.local_slots()));
            }
            BlockLowerer::new(self.body, &self.frame, self.statics, output).lower(block)?;
        }

        Ok(())
    }

    fn block(&self, source: BasicBlockId) -> CodegenResult<fsb_ir::BlockId> {
        self.blocks
            .get(source)
            .and_then(|block| *block)
            .ok_or(CodegenError::MissingBlockMapping(source))
    }
}
