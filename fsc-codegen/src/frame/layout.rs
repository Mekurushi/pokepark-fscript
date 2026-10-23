use crate::{
    CodegenError, CodegenResult,
    abi::{PassMode, ValueAbi},
    stackify::StackificationPlan,
};
use cranelift_entity::SecondaryMap;
use fsc_mir::{Body, LocalId, LocalKind, RETURN_PLACE, Ty, aggregate_fields};

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
pub(crate) enum AllocationKind {
    Inline,
    Indirect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FrameAllocation {
    pub(crate) base: FrameSlot,
    pub(crate) kind: AllocationKind,
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
    allocations: SecondaryMap<LocalId, Option<FrameAllocation>>,
    parameter_slots: SlotCount,
    local_slots: SlotCount,
}

impl FrameLayout {
    pub(crate) fn plan(
        body: &Body,
        parameters: &[ValueAbi],
        stackification: &StackificationPlan,
    ) -> CodegenResult<Self> {
        debug_assert_eq!(body.argument_count, parameters.len());
        let mut allocations = SecondaryMap::new();
        let mut parameter_slots = SlotCount::ZERO;
        let mut local_slots = SlotCount::ZERO;

        for (local, declaration) in &body.locals {
            if stackification.is_stackified(local) {
                continue;
            }

            if local == RETURN_PLACE && declaration.ty == Ty::Void {
                continue;
            }

            let local_kind = body.local_kind(local);
            let (width, kind) = if let LocalKind::Parameter { index } = local_kind
                && parameters[index].mode == PassMode::Indirect
            {
                (1, AllocationKind::Indirect)
            } else {
                let width = slot_width(&declaration.ty).ok_or_else(|| {
                    CodegenError::UnsupportedLocalType {
                        local,
                        ty: declaration.ty.clone(),
                    }
                })?;
                (width, AllocationKind::Inline)
            };

            if width == 0 {
                return Err(CodegenError::UnsupportedLocalType {
                    local,
                    ty: declaration.ty.clone(),
                });
            }

            let base = match local_kind {
                LocalKind::Parameter { .. } => {
                    let base = FrameSlot(parameter_slots.as_offset());
                    parameter_slots = parameter_slots.checked_add(width)?;
                    base
                }
                LocalKind::Return | LocalKind::Local => {
                    local_slots = local_slots.checked_add(width)?;
                    FrameSlot(-local_slots.as_offset())
                }
            };
            allocations.insert(local, Some(FrameAllocation { base, kind }));
        }

        let _ = parameter_slots.checked_add(local_slots.get())?;
        Ok(Self {
            allocations,
            parameter_slots,
            local_slots,
        })
    }

    pub(crate) const fn local_slots(&self) -> u16 {
        self.local_slots.get()
    }

    pub(crate) fn frame_slots(&self) -> u16 {
        self.parameter_slots.get() + self.local_slots.get()
    }

    pub(crate) fn base(&self, local: LocalId) -> CodegenResult<FrameSlot> {
        self.allocation(local).map(|allocation| allocation.base)
    }

    pub(crate) fn allocation(&self, local: LocalId) -> CodegenResult<FrameAllocation> {
        self.allocations
            .get(local)
            .and_then(|allocation| *allocation)
            .ok_or(CodegenError::MissingLocalAllocation(local))
    }
}

pub(crate) fn slot_width(ty: &Ty) -> Option<u16> {
    if let Some(fields) = aggregate_fields(ty) {
        return fields.iter().try_fold(0_u16, |width, field| {
            width.checked_add(slot_width(field.ty())?)
        });
    }

    match ty {
        Ty::Int | Ty::Float | Ty::Bool | Ty::Str | Ty::Reference(_) => Some(1),
        Ty::Void => Some(0),
        Ty::Array { element, length } => {
            let length = u16::try_from(*length).ok()?;
            slot_width(element)?.checked_mul(length)
        }
        _ => None,
    }
}
