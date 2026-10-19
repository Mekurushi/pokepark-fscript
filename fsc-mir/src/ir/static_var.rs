use super::{ConstValue, SourceInfo};
use fsc_sema::types::Ty;

// TODO: mutability when we have readonly arrays, etc.
#[derive(Debug)]
pub struct StaticDecl {
    pub name: String,
    pub ty: Ty,
    pub initializer: ConstValue,
    pub source: SourceInfo,
}
