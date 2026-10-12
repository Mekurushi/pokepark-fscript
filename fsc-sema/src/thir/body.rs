use super::{ConstValue, ExprId, FieldIdx, FunctionId, LocalId, Place, SourceInfo, SyscallNumber};
use crate::types::Ty;
use cranelift_entity::PrimaryMap;

#[derive(Debug)]
pub struct Body {
    pub parameter_locals: Box<[LocalId]>,
    pub locals: PrimaryMap<LocalId, Local>,
    pub expressions: PrimaryMap<ExprId, Expr>,
    pub block: Block,
}

#[derive(Debug)]
pub struct Local {
    pub name: String,
    pub ty: Ty,
    pub kind: LocalKind,
    pub source: SourceInfo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalKind {
    Parameter { index: usize },
    Variable,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub statements: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub source: SourceInfo,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    Return {
        value: Option<ExprId>,
    },
    Break,
    LocalDeclaration {
        local: LocalId,
        initializer: Option<ExprId>,
    },
    Assign {
        target: Place,
        value: ExprId,
    },
    Expression {
        expression: ExprId,
    },
    Pause {
        duration: ExprId,
    },
    If {
        condition: ExprId,
        then_block: Block,
        else_block: Option<Block>,
    },
    While {
        condition: ExprId,
        body: Block,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub ty: Ty,
    pub source: SourceInfo,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Constant(ConstValue),
    Load {
        place: Place,
    },
    Array {
        elements: Box<[ExprId]>,
    },
    Aggregate {
        fields: Box<[FieldExpr]>,
    },
    Unary {
        op: UnaryOp,
        operand: ExprId,
    },
    Binary {
        op: BinaryOp,
        lhs: ExprId,
        rhs: ExprId,
    },
    Call {
        callee: FunctionId,
        arguments: Box<[ExprId]>,
    },
    RawSyscall {
        number: SyscallNumber,
        arguments: Box<[ExprId]>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldExpr {
    pub field: FieldIdx,
    pub value: ExprId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    Negate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    LogicalAnd,
    LogicalOr,
}
