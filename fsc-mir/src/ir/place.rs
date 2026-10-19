use super::{LocalId, StaticId};
use fsc_sema::types::FieldIdx;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    pub base: PlaceBase,
    pub projection: Box<[ProjectionElem]>,
}

impl Place {
    pub fn from_local(local: LocalId) -> Self {
        Self {
            base: PlaceBase::Local(local),
            projection: Box::default(),
        }
    }

    pub fn from_static(static_id: StaticId) -> Self {
        Self {
            base: PlaceBase::Static(static_id),
            projection: Box::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaceBase {
    Local(LocalId),
    Static(StaticId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionElem {
    Field(FieldIdx),
    Index(LocalId),
}
