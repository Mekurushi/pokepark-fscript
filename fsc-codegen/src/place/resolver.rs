use crate::{
    CodegenError, CodegenResult,
    data::StaticData,
    frame::{AllocationKind, FrameLayout, FrameSlot, slot_width},
};
use fsb_ir::DataRef;
use fsc_mir::{Body, FieldIdx, Place, PlaceBase, ProjectionElem, Ty, aggregate_fields};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IndexedOffset {
    pub(crate) index: FrameSlot,
    pub(crate) stride: u16,
    pub(crate) length: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StorageBase {
    Frame(FrameSlot),
    FramePointer(FrameSlot),
    Data(DataRef),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedPlace {
    pub(crate) base: StorageBase,
    pub(crate) static_offset: u16,
    pub(crate) indexed_offsets: Box<[IndexedOffset]>,
    pub(crate) ty: Ty,
}

impl ResolvedPlace {
    pub(crate) fn project_deref(&self) -> CodegenResult<Self> {
        let Ty::Reference(referent) = &self.ty else {
            return Err(CodegenError::UnsupportedType(self.ty.clone()));
        };
        let StorageBase::Frame(base) = self.base else {
            return Err(CodegenError::UnsupportedType(self.ty.clone()));
        };
        if self.static_offset != 0 || !self.indexed_offsets.is_empty() {
            return Err(CodegenError::UnsupportedType(self.ty.clone()));
        }

        Ok(Self {
            base: StorageBase::FramePointer(base),
            static_offset: 0,
            indexed_offsets: Box::default(),
            ty: (**referent).clone(),
        })
    }

    pub(crate) fn project_field(&self, field: FieldIdx) -> CodegenResult<Self> {
        let fields =
            aggregate_fields(&self.ty).ok_or_else(|| CodegenError::InvalidFieldProjection {
                ty: self.ty.clone(),
                field,
            })?;

        let mut field_offset = 0_u16;
        for definition in fields {
            if definition.index() == field {
                return Ok(Self {
                    base: self.base.clone(),
                    static_offset: self
                        .static_offset
                        .checked_add(field_offset)
                        .ok_or(CodegenError::FrameTooLarge)?,
                    indexed_offsets: self.indexed_offsets.clone(),
                    ty: definition.ty().clone(),
                });
            }

            field_offset = field_offset
                .checked_add(
                    slot_width(definition.ty())
                        .ok_or_else(|| CodegenError::UnsupportedType(definition.ty().clone()))?,
                )
                .ok_or(CodegenError::FrameTooLarge)?;
        }

        Err(CodegenError::InvalidFieldProjection {
            ty: self.ty.clone(),
            field,
        })
    }

    pub(crate) fn project_index(&self, index: FrameSlot) -> CodegenResult<Self> {
        let Ty::Array { element, length } = &self.ty else {
            return Err(CodegenError::InvalidIndexProjection {
                ty: self.ty.clone(),
            });
        };
        let stride = slot_width(element)
            .ok_or_else(|| CodegenError::UnsupportedType((**element).clone()))?;

        let mut indexed_offsets = Vec::with_capacity(self.indexed_offsets.len() + 1);
        indexed_offsets.extend_from_slice(&self.indexed_offsets);
        indexed_offsets.push(IndexedOffset {
            index,
            stride,
            length: *length,
        });

        Ok(Self {
            base: self.base.clone(),
            static_offset: self.static_offset,
            indexed_offsets: indexed_offsets.into_boxed_slice(),
            ty: (**element).clone(),
        })
    }

    pub(crate) fn project_constant_index(&self, index: usize) -> CodegenResult<Self> {
        let Ty::Array { element, length } = &self.ty else {
            return Err(CodegenError::InvalidIndexProjection {
                ty: self.ty.clone(),
            });
        };
        if index >= *length {
            return Err(CodegenError::IndexOutOfBounds {
                index,
                length: *length,
            });
        }

        let stride = slot_width(element)
            .ok_or_else(|| CodegenError::UnsupportedType((**element).clone()))?;
        let index = u16::try_from(index).map_err(|_| CodegenError::FrameTooLarge)?;
        let element_offset = index
            .checked_mul(stride)
            .ok_or(CodegenError::FrameTooLarge)?;

        Ok(Self {
            base: self.base.clone(),
            static_offset: self
                .static_offset
                .checked_add(element_offset)
                .ok_or(CodegenError::FrameTooLarge)?,
            indexed_offsets: self.indexed_offsets.clone(),
            ty: (**element).clone(),
        })
    }
}

pub(crate) struct PlaceResolver<'layout, 'mir> {
    frame: &'layout FrameLayout,
    statics: &'layout StaticData,
    body: &'mir Body,
}

impl<'layout, 'mir> PlaceResolver<'layout, 'mir> {
    pub(crate) const fn new(
        frame: &'layout FrameLayout,
        statics: &'layout StaticData,
        body: &'mir Body,
    ) -> Self {
        Self {
            frame,
            statics,
            body,
        }
    }

    pub(crate) fn resolve(&self, place: &Place) -> CodegenResult<ResolvedPlace> {
        place
            .projection
            .iter()
            .try_fold(
                self.resolve_base(place.base)?,
                |resolved, projection| match projection {
                    ProjectionElem::Deref => resolved.project_deref(),
                    ProjectionElem::Field(field) => resolved.project_field(*field),
                    ProjectionElem::Index(index) => {
                        resolved.project_index(self.frame.base(*index)?)
                    }
                },
            )
    }

    fn resolve_base(&self, base: PlaceBase) -> CodegenResult<ResolvedPlace> {
        let (base, ty) = match base {
            PlaceBase::Local(local) => {
                let allocation = self.frame.allocation(local)?;
                let base = match allocation.kind {
                    AllocationKind::Inline => StorageBase::Frame(allocation.base),
                    AllocationKind::Indirect => StorageBase::FramePointer(allocation.base),
                };
                let ty = self
                    .body
                    .locals
                    .get(local)
                    .ok_or(CodegenError::MissingLocalAllocation(local))?
                    .ty
                    .clone();
                (base, ty)
            }
            PlaceBase::Static(static_id) => {
                let declaration = self.statics.declaration(static_id)?;
                (
                    StorageBase::Data(declaration.base.clone()),
                    declaration.ty.clone(),
                )
            }
        };

        Ok(ResolvedPlace {
            base,
            static_offset: 0,
            indexed_offsets: Box::default(),
            ty,
        })
    }
}
