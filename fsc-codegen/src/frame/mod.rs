mod layout;

use crate::error::{CodegenError, CodegenResult};
use fsc_sema::hir::{LocalId, Place, ProjectionElem};
use std::collections::HashMap;

pub(crate) use layout::plan_frame;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StackSlot(pub i16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StackAllocation {
    pub base: StackSlot,
    pub width: u16, // TODO: metadata only?
}

#[derive(Debug)]
pub(crate) struct FrameLayout {
    allocations: HashMap<LocalId, StackAllocation>,
    parameter_slots: i16,
    local_slots: i16,
}

impl FrameLayout {
    pub(crate) fn new(
        allocations: HashMap<LocalId, StackAllocation>,
        parameter_slots: i16,
        local_slots: i16,
    ) -> Self {
        Self {
            allocations,
            parameter_slots,
            local_slots,
        }
    }

    pub(crate) fn resolve(&self, place: &Place) -> CodegenResult<StackSlot> {
        let allocation = self
            .allocations
            .get(&place.local)
            .ok_or(CodegenError::UnknownLocal(place.local))?;
        let offset = place
            .projection
            .iter()
            .map(|ProjectionElem::Vec3Field(field)| field.offset().cast_signed())
            .sum::<i16>();

        Ok(StackSlot(allocation.base.0 + offset))
    }

    pub(crate) const fn local_slot_count(&self) -> i16 {
        self.local_slots
    }

    pub(crate) const fn frame_size(&self) -> i16 {
        self.parameter_slots + self.local_slots
    }
}
