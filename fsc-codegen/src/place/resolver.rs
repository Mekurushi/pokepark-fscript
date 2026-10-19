use crate::{
    CodegenError, CodegenResult,
    data::StaticData,
    frame::{FrameLayout, FrameSlot, slot_width},
};
use fsb_ir::DataRef;
use fsc_mir::{Body, Place, PlaceBase, ProjectionElem, Ty, aggregate_fields};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IndexedOffset {
    pub(crate) index: FrameSlot,
    pub(crate) stride: u16,
    pub(crate) length: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StorageBase {
    Frame(FrameSlot),
    Data(DataRef),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedPlace {
    pub(crate) base: StorageBase,
    pub(crate) static_offset: u16,
    pub(crate) indexed_offsets: Box<[IndexedOffset]>,
    pub(crate) ty: Ty,
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
        let (base, mut ty) = self.resolve_base(place.base)?;
        let mut static_offset = 0_u16;
        let mut indexed_offsets = Vec::new();

        for projection in &place.projection {
            match projection {
                ProjectionElem::Field(field) => {
                    let fields = aggregate_fields(&ty).ok_or_else(|| {
                        CodegenError::InvalidFieldProjection {
                            ty: ty.clone(),
                            field: *field,
                        }
                    })?;
                    let mut offset = 0_u16;
                    let mut field_ty = None;
                    for definition in fields {
                        if definition.index() == *field {
                            field_ty = Some(definition.ty().clone());
                            break;
                        }
                        let width = slot_width(definition.ty()).ok_or_else(|| {
                            CodegenError::UnsupportedType(definition.ty().clone())
                        })?;
                        offset = offset
                            .checked_add(width)
                            .ok_or(CodegenError::FrameTooLarge)?;
                    }
                    let Some(field_ty) = field_ty else {
                        return Err(CodegenError::InvalidFieldProjection { ty, field: *field });
                    };
                    static_offset = static_offset
                        .checked_add(offset)
                        .ok_or(CodegenError::FrameTooLarge)?;
                    ty = field_ty;
                }
                ProjectionElem::Index(index) => {
                    let Ty::Array { element, length } = &ty else {
                        return Err(CodegenError::InvalidIndexProjection { ty });
                    };
                    let stride = slot_width(element)
                        .ok_or_else(|| CodegenError::UnsupportedType((**element).clone()))?;
                    indexed_offsets.push(IndexedOffset {
                        index: self.frame.base(*index)?,
                        stride,
                        length: *length,
                    });
                    ty = (**element).clone();
                }
            }
        }

        Ok(ResolvedPlace {
            base,
            static_offset,
            indexed_offsets: indexed_offsets.into_boxed_slice(),
            ty,
        })
    }

    fn resolve_base(&self, base: PlaceBase) -> CodegenResult<(StorageBase, Ty)> {
        match base {
            PlaceBase::Local(local) => {
                let base = StorageBase::Frame(self.frame.base(local)?);
                let ty = self
                    .body
                    .locals
                    .get(local)
                    .ok_or(CodegenError::MissingLocalAllocation(local))?
                    .ty
                    .clone();
                Ok((base, ty))
            }
            PlaceBase::Static(static_id) => {
                let declaration = self.statics.declaration(static_id)?;
                Ok((
                    StorageBase::Data(declaration.base.clone()),
                    declaration.ty.clone(),
                ))
            }
        }
    }
}
