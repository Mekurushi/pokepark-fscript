use super::LocalId;
use fsc_sema::types::FieldIdx;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    pub local: LocalId,
    pub projection: Box<[ProjectionElem]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionElem {
    Field(FieldIdx),
    Index(LocalId),
}
