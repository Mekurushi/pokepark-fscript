use super::layout::{slot_width, FrameLayout, FrameSlot};
use crate::{CodegenError, CodegenResult};
use fsc_mir::{aggregate_fields, LocalId, Place, ProjectionElem, Ty};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IndexedOffset {
    pub(crate) index: LocalId,
    pub(crate) stride: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedPlace {
    pub(crate) base: FrameSlot,
    pub(crate) static_offset: u16,
    pub(crate) indexed_offsets: Box<[IndexedOffset]>,
    pub(crate) ty: Ty,
}

impl FrameLayout {
    pub(crate) fn resolve(
        &self,
        body: &fsc_mir::Body,
        place: &Place,
    ) -> CodegenResult<ResolvedPlace> {
        let base = self.base(place.local)?;
        let mut static_offset = 0_u16;
        let mut indexed_offsets = Vec::new();
        let mut ty = body
            .locals
            .get(place.local)
            .ok_or(CodegenError::MissingLocalAllocation(place.local))?
            .ty
            .clone();

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
                    let Ty::Array { element, .. } = &ty else {
                        return Err(CodegenError::InvalidIndexProjection { ty });
                    };
                    let stride = slot_width(element)
                        .ok_or_else(|| CodegenError::UnsupportedType((**element).clone()))?;
                    indexed_offsets.push(IndexedOffset {
                        index: *index,
                        stride,
                    });
                    ty = (**element).clone();

                    // local arrays are possible in the stack with load_arg_address + indexed
                    // access with lwi, swi. Only using StackSlot = word to make the first step
                    // easier and later on improve smth like packed bools
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
}
