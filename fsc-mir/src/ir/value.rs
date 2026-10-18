use super::Place;
use fsc_sema::types::{FieldIdx, Ty};

#[derive(Debug, Clone, PartialEq)]
pub enum ConstValue {
    Int(i32),
    Float(f32),
    Bool(bool),
    String(String),
    Array(Box<[ConstValue]>),
    Aggregate(Box<[ConstValue]>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Constant {
    pub value: ConstValue,
    pub ty: Ty,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    Place(Place),
    Constant(Constant),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Rvalue {
    Use(Operand),
    Unary {
        op: UnaryOp,
        operand: Operand,
    },
    Binary {
        op: BinaryOp,
        lhs: Operand,
        rhs: Operand,
    },
    Array {
        elements: Box<[Operand]>,
    },
    Aggregate {
        ty: Ty,
        fields: Box<[FieldOperand]>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldOperand {
    pub field: FieldIdx,
    pub value: Operand,
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
}
