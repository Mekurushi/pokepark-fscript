use super::{ItemId, LocalId};
use crate::types::Ty;
use cranelift_entity::PrimaryMap;
use fsc_diagnostics::{ErrorGuaranteed, Span};

#[derive(Debug)]
pub struct Body {
    pub parameter_locals: Vec<LocalId>,
    pub locals: PrimaryMap<LocalId, Local>,
    pub block: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub statements: Vec<Stmt>,
}

#[derive(Debug)]
pub struct Local {
    pub name: String,
    pub name_span: Span,
    pub ty: Ty,
    pub type_span: Span,
    pub kind: LocalKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalKind {
    Parameter { index: usize },
    Variable,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Error(ErrorGuaranteed),
    Int(i32),
    Float(f32),
    Bool(bool),
    String(String),
    Local(LocalId),
    ItemRef(ItemId),
    Member {
        base: Box<Expr>,
        member: String,
        member_span: Span,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        arguments: Vec<Expr>,
    },
    RawSyscall {
        arguments: Vec<Expr>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    Neg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    NotEq,
    Less,
    Greater,
    LessEq,
    GreaterEq,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    Return(Option<Expr>),
    Break,
    LocalDeclaration {
        local: LocalId,
        initializer: Option<Expr>,
    },
    Assign {
        target: Expr,
        value: Expr,
    },
    Expression(Expr),
    Pause(Expr),
    If {
        condition: Expr,
        then_block: Block,
        else_block: Option<Block>,
    },
    While {
        condition: Expr,
        body: Block,
    },
}
