use super::{ConstValue, SourceInfo};
use fsc_sema::types::Ty;

// TODO: mutability when we have readonly arrays, etc.
#[derive(Debug)]
pub struct StaticDecl {
    pub name: String,
    pub ty: Ty,
    pub kind: StaticKind,
    pub source: SourceInfo,
}

#[derive(Debug)]
pub enum StaticKind {
    Defined { initializer: ConstValue },
    External,
}
