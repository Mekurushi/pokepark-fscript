mod body;
mod entity;
mod place;
mod value;

pub use body::{
    BinaryOp, Block, Body, Expr, ExprKind, FieldExpr, Local, LocalKind, Stmt, StmtKind, UnaryOp,
};
pub use entity::{ExprId, FieldIdx, FunctionId, LocalId};
pub use place::{Place, ProjectionElem};
pub use value::{ConstValue, SyscallNumber};

use crate::types::Ty;
use cranelift_entity::PrimaryMap;
use fsc_diagnostics::Span;

#[derive(Debug, Default)]
pub struct Script {
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
    Defined { linkage: Linkage, body: Body },
    External,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Linkage {
    Internal,
    Exported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceInfo {
    pub span: Span,
}

// TODO: verification pass
