mod body;
mod entity;
mod lower;
mod value;

pub(crate) use lower::lower;

pub use crate::types::FieldIdx;
pub use body::{
    BinaryOp, Block, Body, Expr, ExprKind, FieldExpr, Local, LocalKind, RawSyscallFunction,
    RawSyscallPage, Stmt, StmtKind, UnaryOp,
};
pub use entity::{ExprId, FunctionId, LocalId};
pub use value::ConstValue;

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
