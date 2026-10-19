use super::{Body, FunctionId, SourceInfo, StaticDecl, StaticId};
use cranelift_entity::PrimaryMap;
use fsc_sema::types::Ty;

#[derive(Debug, Default)]
pub struct Script {
    pub statics: PrimaryMap<StaticId, StaticDecl>,
    pub functions: PrimaryMap<FunctionId, Function>,
}

#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub signature: FunctionSignature,
    pub kind: FunctionKind,
    pub source: SourceInfo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionSignature {
    pub parameters: Box<[Ty]>,
    pub return_type: Ty,
}

#[derive(Debug)]
pub enum FunctionKind {
    // argument_count == signature.parameters.len()
    // locals[RETURN_PLACE].ty == signature.return_type
    // further implicitness (should be guaranteed by typeck)
    Defined { linkage: Linkage, body: Body },
    External,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Linkage {
    Internal,
    Exported,
}
