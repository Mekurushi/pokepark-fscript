use super::{ExprId, FieldIdx, LocalId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    pub local: LocalId,
    pub projections: Box<[ProjectionElem]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectionElem {
    Field(FieldIdx),
    Index(ExprId),
}
