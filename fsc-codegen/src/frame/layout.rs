use crate::{CodegenError, CodegenResult};
use cranelift_entity::{EntityRef, SecondaryMap};
use fsc_mir::{Body, LocalId, RETURN_PLACE, Ty, aggregate_fields};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FrameSlot(i16);

impl FrameSlot {
    pub(crate) const fn offset(self) -> i16 {
        self.0
    }

    pub(crate) fn checked_add(self, offset: i16) -> CodegenResult<Self> {
        self.0
            .checked_add(offset)
            .map(Self)
            .ok_or(CodegenError::FrameTooLarge)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StackAllocation {
    base: FrameSlot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SlotCount(u16);

impl SlotCount {
    const ZERO: Self = Self(0);

    fn checked_add(self, additional: u16) -> CodegenResult<Self> {
        self.0
            .checked_add(additional)
            .filter(|&slots| i16::try_from(slots).is_ok())
            .map(Self)
            .ok_or(CodegenError::FrameTooLarge)
    }

    const fn as_offset(self) -> i16 {
        self.0.cast_signed()
    }

    const fn get(self) -> u16 {
        self.0
    }
}

#[derive(Debug)]
pub(crate) struct FrameLayout {
    allocations: SecondaryMap<LocalId, Option<StackAllocation>>,
    parameter_slots: SlotCount,
    local_slots: SlotCount,
}

impl FrameLayout {
    pub(crate) fn plan(body: &Body) -> CodegenResult<Self> {
        let mut allocations = SecondaryMap::new();
        let mut parameter_slots = SlotCount::ZERO;
        let mut local_slots = SlotCount::ZERO;

        for (local, declaration) in &body.locals {
            let width =
                slot_width(&declaration.ty).ok_or_else(|| CodegenError::UnsupportedLocalType {
                    local,
                    ty: declaration.ty.clone(),
                })?;

            if local == RETURN_PLACE && declaration.ty == Ty::Void {
                continue;
            }
            if width == 0 {
                return Err(CodegenError::UnsupportedLocalType {
                    local,
                    ty: declaration.ty.clone(),
                });
            }

            let base = if is_parameter(local, body.argument_count) {
                let base = FrameSlot(parameter_slots.as_offset());
                parameter_slots = parameter_slots.checked_add(width)?;
                base
            } else {
                local_slots = local_slots.checked_add(width)?;
                FrameSlot(-local_slots.as_offset())
            };
            allocations.insert(local, Some(StackAllocation { base }));
        }

        let _ = parameter_slots.checked_add(local_slots.get())?;
        Ok(Self {
            allocations,
            parameter_slots,
            local_slots,
        })
    }

    pub(crate) const fn parameter_slots(&self) -> u16 {
        self.parameter_slots.get()
    }

    pub(crate) const fn local_slots(&self) -> u16 {
        self.local_slots.get()
    }

    pub(crate) fn frame_slots(&self) -> u16 {
        self.parameter_slots.get() + self.local_slots.get()
    }

    pub(crate) fn base(&self, local: LocalId) -> CodegenResult<FrameSlot> {
        self.allocations
            .get(local)
            .and_then(|allocation| *allocation)
            .map(|allocation| allocation.base)
            .ok_or(CodegenError::MissingLocalAllocation(local))
    }
}

// using the invariant regarding argument order from the MIR
fn is_parameter(local: LocalId, argument_count: usize) -> bool {
    let index = local.index();
    index > RETURN_PLACE.index() && index <= argument_count
}

pub(crate) fn slot_width(ty: &Ty) -> Option<u16> {
    if let Some(fields) = aggregate_fields(ty) {
        return fields.iter().try_fold(0_u16, |width, field| {
            width.checked_add(slot_width(field.ty())?)
        });
    }

    match ty {
        Ty::Int | Ty::Float | Ty::Bool | Ty::Str => Some(1),
        Ty::Void => Some(0),
        Ty::Array { element, length } => {
            let length = u16::try_from(*length).ok()?;
            slot_width(element)?.checked_mul(length)
        }
        _ => None,
    }
}
