use super::{block_mapping::BlockMapping, function_mapping::FunctionMapping};
use crate::{
    CodegenResult, data::StaticData, frame::FrameLayout, place::resolver::PlaceResolver,
    place::resolver::ResolvedPlace,
};
use fsc_mir::{BasicBlockId, Body, FunctionId, Place, RETURN_PLACE, Ty};

pub(super) struct FunctionLoweringContext<'function> {
    body: &'function Body,
    frame: &'function FrameLayout,
    statics: &'function StaticData,
    functions: &'function FunctionMapping,
    blocks: &'function BlockMapping,
}

impl<'function> FunctionLoweringContext<'function> {
    pub(super) const fn new(
        body: &'function Body,
        frame: &'function FrameLayout,
        statics: &'function StaticData,
        functions: &'function FunctionMapping,
        blocks: &'function BlockMapping,
    ) -> Self {
        Self {
            body,
            frame,
            statics,
            functions,
            blocks,
        }
    }

    pub(super) fn resolve_place(&self, place: &Place) -> CodegenResult<ResolvedPlace> {
        PlaceResolver::new(self.frame, self.statics, self.body).resolve(place)
    }

    pub(super) fn block(&self, source: BasicBlockId) -> fsb_ir::BlockId {
        self.blocks.target(source)
    }

    pub(super) fn function(&self, source: FunctionId) -> CodegenResult<fsb_ir::FunctionRef> {
        self.functions.function(source)
    }

    pub(super) fn return_type(&self) -> &Ty {
        &self.body.locals[RETURN_PLACE].ty
    }

    pub(super) fn frame_slots(&self) -> u16 {
        self.frame.frame_slots()
    }
}
