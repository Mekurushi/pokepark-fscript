use crate::error::{CodegenError, CodegenResult};
use crate::frame::{FrameLayout, StackAllocation, StackSlot};
use fsc_sema::hir::{FuncDef, LocalKind};
use std::collections::HashMap;

pub(crate) fn plan_frame(function: &FuncDef) -> CodegenResult<FrameLayout> {
    let mut allocations = HashMap::new();
    let mut parameter_slots = 0_u16;
    let mut local_slots = 0_u16;

    for local in &function.locals {
        if allocations.contains_key(&local.id) {
            return Err(CodegenError::DuplicateLocal(local.id));
        }

        let width = local.ty.slot_width();
        if width == 0 {
            return Err(CodegenError::InvalidLocalType(local.id));
        }

        let base = match local.kind {
            LocalKind::Parameter => {
                let base = StackSlot(
                    i16::try_from(parameter_slots).map_err(|_| CodegenError::FrameTooLarge)?,
                );
                parameter_slots = parameter_slots
                    .checked_add(width)
                    .ok_or(CodegenError::FrameTooLarge)?;
                base
            }
            LocalKind::Variable => {
                local_slots = local_slots
                    .checked_add(width)
                    .ok_or(CodegenError::FrameTooLarge)?;
                StackSlot(-i16::try_from(local_slots).map_err(|_| CodegenError::FrameTooLarge)?)
            }
        };

        allocations.insert(local.id, StackAllocation { base, width });
    }

    let frame_slots = parameter_slots
        .checked_add(local_slots)
        .ok_or(CodegenError::FrameTooLarge)?;
    let parameter_slots =
        i16::try_from(parameter_slots).map_err(|_| CodegenError::FrameTooLarge)?;
    let local_slots = i16::try_from(local_slots).map_err(|_| CodegenError::FrameTooLarge)?;
    i16::try_from(frame_slots).map_err(|_| CodegenError::FrameTooLarge)?;

    Ok(FrameLayout::new(allocations, parameter_slots, local_slots))
}
