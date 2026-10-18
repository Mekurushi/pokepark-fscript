use super::{Constant, Place, SourceInfo};

#[derive(Debug, Clone, PartialEq)]
pub struct VariableDebugInfo {
    pub name: String,
    pub value: DebugValue,
    pub source: SourceInfo,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DebugValue {
    Place(Place),
    Constant(Constant),
}
