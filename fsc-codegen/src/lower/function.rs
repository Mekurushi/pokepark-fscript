use super::{
    block::BlockLowerer, block_mapping::BlockMapping, context::CodegenContext,
    function_mapping::FunctionDeclaration,
};
use crate::{
    CodegenResult,
    frame::FrameLayout,
    place::resolver::PlaceResolver,
    place::resolver::ResolvedPlace,
    stackify::{StackificationPlan, StatementLocation},
};
use fsc_mir::{
    BasicBlockId, Body, FunctionId, Place, RETURN_PLACE, Rvalue, START_BLOCK, StatementKind, Ty,
};

pub(super) fn lower<'function>(
    body: &'function Body,
    frame: FrameLayout,
    stackification: StackificationPlan,
    codegen: &'function CodegenContext<'function>,
    output: &mut fsb_ir::Function,
) -> CodegenResult<()> {
    let blocks = BlockMapping::allocate(body, output);
    FunctionLowerer::new(body, frame, stackification, codegen, blocks).lower_blocks(output)
}

pub(super) struct FunctionLowerer<'function> {
    body: &'function Body,
    frame: FrameLayout,
    stackification: StackificationPlan,
    codegen: &'function CodegenContext<'function>,
    blocks: BlockMapping,
}

impl<'function> FunctionLowerer<'function> {
    const fn new(
        body: &'function Body,
        frame: FrameLayout,
        stackification: StackificationPlan,
        codegen: &'function CodegenContext<'function>,
        blocks: BlockMapping,
    ) -> Self {
        Self {
            body,
            frame,
            stackification,
            codegen,
            blocks,
        }
    }

    fn lower_blocks(&self, output: &mut fsb_ir::Function) -> CodegenResult<()> {
        for (source, block) in &self.body.blocks {
            let target = self.block(source);
            let output = output.block_mut(target);
            if source == START_BLOCK && self.frame.local_slots() != 0 {
                output.push_instruction(fsb_ir::Instruction::GrowStack(self.frame.local_slots()));
            }
            BlockLowerer::new(self, source, output).lower(block)?;
        }

        Ok(())
    }

    pub(super) fn resolve_place(&self, place: &Place) -> CodegenResult<ResolvedPlace> {
        PlaceResolver::new(&self.frame, self.codegen.statics(), self.body).resolve(place)
    }

    pub(super) fn stackified_rvalue(&self, place: &Place) -> Option<&Rvalue> {
        let value = self.stackification.value(place)?;
        let StatementKind::Assign { value, .. } =
            &self.body.blocks[value.producer.block].statements[value.producer.statement].kind
        else {
            unreachable!("stackified producer must be an assignment");
        };
        Some(value)
    }

    pub(super) fn selects_statement(&self, location: StatementLocation) -> bool {
        self.stackification.selects_statement(location)
    }

    pub(super) fn block(&self, source: BasicBlockId) -> fsb_ir::BlockId {
        self.blocks.target(source)
    }

    pub(super) fn function(&self, source: FunctionId) -> CodegenResult<fsb_ir::FunctionRef> {
        self.codegen.function(source)
    }

    pub(super) fn function_declaration(
        &self,
        source: FunctionId,
    ) -> CodegenResult<&FunctionDeclaration> {
        self.codegen.function_declaration(source)
    }

    pub(super) fn return_type(&self) -> &Ty {
        &self.body.locals[RETURN_PLACE].ty
    }

    pub(super) fn frame_slots(&self) -> u16 {
        self.frame.frame_slots()
    }
}
