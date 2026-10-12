mod body;
mod entity;

pub use body::{
    BinaryOp, Block, Body, Expr, ExprKind, Local, LocalKind, Stmt, StmtKind, UnaryOp,
};
pub use entity::{ExprId, ItemId, LocalId};

use crate::types::Ty;
use cranelift_entity::PrimaryMap;
use fsc_diagnostics::Span;

#[derive(Debug, Default)]
pub struct Script {
    pub items: PrimaryMap<ItemId, Item>,
}

#[derive(Debug)]
pub struct Item {
    pub name: String,
    pub name_span: Span,
    pub kind: ItemKind,
}

#[derive(Debug)]
pub enum ItemKind {
    Function(Function),
    Constant(Constant),
    Config(Config),
}

#[derive(Debug)]
pub struct Function {
    pub signature: FunctionSignature,
    pub kind: FunctionKind,
}

#[derive(Debug)]
pub struct FunctionSignature {
    pub parameters: Vec<Parameter>,
    pub return_type: Ty,
    pub return_type_span: Span,
}

#[derive(Debug)]
pub struct Parameter {
    pub name: String,
    pub name_span: Span,
    pub ty: Ty,
    pub type_span: Span,
}

#[derive(Debug)]
pub enum FunctionKind {
    Defined { exported: bool, body: Body },
    External,
}

#[derive(Debug)]
pub struct Constant {
    pub ty: Ty,
    pub type_span: Span,
    pub initializer: Expr,
}

#[derive(Debug)]
pub struct Config {
    pub ty: Ty,
    pub type_span: Span,
}
